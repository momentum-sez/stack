// SPDX-License-Identifier: BUSL-1.1
//! # Arbitration API Routes
//!
//! HTTP surface for the full dispute lifecycle. Exposes endpoints to file
//! disputes, advance through the 7-phase lifecycle (Filed → UnderReview →
//! EvidenceCollection → Hearing → Decided → Enforced → Closed), settle,
//! dismiss, and query dispute state.
//!
//! ## Lifecycle Transitions
//!
//! Each transition requires typed evidence — the HTTP layer validates the
//! evidence payload and delegates to `mez-arbitration::Dispute` methods
//! which enforce the state machine rules.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use mez_arbitration::{
    Claim, ClosureEvidence, DecisionEvidence, DismissalEvidence, Dispute, DisputeType,
    EnforcementInitiationEvidence, EvidencePhaseEvidence, FilingEvidence,
    HearingScheduleEvidence, Money, Party, ReviewInitiationEvidence, SettlementEvidence,
};
use mez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, Did, JurisdictionId, Timestamp};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request to file a new dispute.
#[derive(Debug, Deserialize, ToSchema)]
pub struct FileDisputeRequest {
    /// Claimant DID string.
    pub claimant_did: String,
    /// Claimant legal name.
    pub claimant_name: String,
    /// Claimant jurisdiction (optional).
    pub claimant_jurisdiction: Option<String>,
    /// Respondent DID string.
    pub respondent_did: String,
    /// Respondent legal name.
    pub respondent_name: String,
    /// Respondent jurisdiction (optional).
    pub respondent_jurisdiction: Option<String>,
    /// Dispute type identifier.
    pub dispute_type: String,
    /// Governing jurisdiction identifier.
    pub jurisdiction: String,
    /// Optional corridor ID for cross-border disputes.
    pub corridor_id: Option<String>,
    /// Arbitration institution ID.
    pub institution_id: String,
    /// Claims with descriptions and optional amounts.
    pub claims: Vec<ClaimRequest>,
    /// SHA-256 hex digest of the filing document.
    pub filing_document_digest: String,
}

/// A claim within a file-dispute request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ClaimRequest {
    /// Claim identifier.
    pub claim_id: String,
    /// Claim type.
    pub claim_type: String,
    /// Description.
    pub description: String,
    /// Amount (decimal string).
    pub amount: Option<String>,
    /// Currency (ISO 4217).
    pub currency: Option<String>,
}

/// Generic transition request carrying an evidence digest.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TransitionRequest {
    /// SHA-256 hex digest of the evidence document.
    pub evidence_digest: String,
    /// Optional case reference (for begin_review).
    pub case_reference: Option<String>,
    /// Optional deadline (ISO 8601, for open_evidence).
    pub deadline: Option<String>,
    /// Optional hearing date (ISO 8601, for schedule_hearing).
    pub hearing_date: Option<String>,
}

/// Settlement request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SettleRequest {
    /// SHA-256 hex digest of the settlement agreement.
    pub settlement_agreement_digest: String,
    /// Consent digests from each party.
    pub party_consent_digests: Vec<String>,
}

/// Dismissal request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct DismissRequest {
    /// Reason for dismissal.
    pub reason: String,
    /// SHA-256 hex digest of the dismissal order.
    pub dismissal_order_digest: String,
}

/// Dispute summary in API responses.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DisputeResponse {
    pub dispute_id: String,
    pub state: String,
    pub dispute_type: String,
    pub claimant_did: String,
    pub respondent_did: String,
    pub jurisdiction: String,
    pub institution_id: String,
    pub corridor_id: Option<String>,
    pub claim_count: usize,
    pub transition_count: usize,
    pub filed_at: String,
    pub updated_at: String,
    pub valid_transitions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the arbitration dispute lifecycle router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/disputes", post(file_dispute).get(list_disputes))
        .route("/v1/disputes/:id", get(get_dispute))
        .route("/v1/disputes/:id/begin-review", post(begin_review))
        .route("/v1/disputes/:id/open-evidence", post(open_evidence))
        .route("/v1/disputes/:id/schedule-hearing", post(schedule_hearing))
        .route("/v1/disputes/:id/decide", post(decide))
        .route("/v1/disputes/:id/enforce", post(enforce))
        .route("/v1/disputes/:id/close", post(close))
        .route("/v1/disputes/:id/settle", post(settle))
        .route("/v1/disputes/:id/dismiss", post(dismiss))
        .route(
            "/v1/arbitration/institutions",
            get(list_institutions),
        )
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_dispute_type(s: &str) -> Result<DisputeType, AppError> {
    match s {
        "breach_of_contract" => Ok(DisputeType::BreachOfContract),
        "non_conforming_goods" => Ok(DisputeType::NonConformingGoods),
        "payment_default" => Ok(DisputeType::PaymentDefault),
        "delivery_failure" => Ok(DisputeType::DeliveryFailure),
        "quality_defect" => Ok(DisputeType::QualityDefect),
        "documentary_discrepancy" => Ok(DisputeType::DocumentaryDiscrepancy),
        "force_majeure" => Ok(DisputeType::ForceMajeure),
        "fraudulent_misrepresentation" => Ok(DisputeType::FraudulentMisrepresentation),
        other => Err(AppError::Validation(format!("unknown dispute type: '{other}'"))),
    }
}

fn digest_from_hex(hex: &str) -> Result<ContentDigest, AppError> {
    let canonical = CanonicalBytes::new(&serde_json::json!({"digest": hex}))
        .map_err(|e| AppError::Validation(format!("invalid digest: {e}")))?;
    Ok(sha256_digest(&canonical))
}

fn dispute_to_response(d: &Dispute) -> DisputeResponse {
    DisputeResponse {
        dispute_id: d.id.as_uuid().to_string(),
        state: d.state.as_str().to_string(),
        dispute_type: d.dispute_type.as_str().to_string(),
        claimant_did: d.claimant.did.as_str().to_string(),
        respondent_did: d.respondent.did.as_str().to_string(),
        jurisdiction: d.jurisdiction.as_str().to_string(),
        institution_id: d.institution_id.clone(),
        corridor_id: d.corridor_id.as_ref().map(|c| c.as_uuid().to_string()),
        claim_count: d.claims.len(),
        transition_count: d.transition_log.len(),
        filed_at: d.filed_at.to_string(),
        updated_at: d.updated_at.to_string(),
        valid_transitions: d
            .state
            .valid_transitions()
            .iter()
            .map(|s| s.as_str().to_string())
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/disputes — File a new dispute.
#[utoipa::path(
    post,
    path = "/v1/disputes",
    request_body = FileDisputeRequest,
    responses(
        (status = 201, description = "Dispute filed successfully", body = DisputeResponse),
        (status = 422, description = "Validation error"),
    ),
    tag = "arbitration"
)]
async fn file_dispute(
    State(state): State<AppState>,
    Json(req): Json<FileDisputeRequest>,
) -> Result<(axum::http::StatusCode, Json<DisputeResponse>), AppError> {
    // Validate inputs.
    let claimant_did = Did::new(&req.claimant_did)
        .map_err(|e| AppError::Validation(format!("invalid claimant DID: {e}")))?;
    let respondent_did = Did::new(&req.respondent_did)
        .map_err(|e| AppError::Validation(format!("invalid respondent DID: {e}")))?;
    let jurisdiction = JurisdictionId::new(&req.jurisdiction)
        .map_err(|e| AppError::Validation(format!("invalid jurisdiction: {e}")))?;
    let dispute_type = parse_dispute_type(&req.dispute_type)?;

    if req.claimant_name.trim().is_empty() {
        return Err(AppError::Validation("claimant_name must not be empty".to_string()));
    }
    if req.respondent_name.trim().is_empty() {
        return Err(AppError::Validation("respondent_name must not be empty".to_string()));
    }
    if req.institution_id.trim().is_empty() {
        return Err(AppError::Validation("institution_id must not be empty".to_string()));
    }
    if req.claims.is_empty() {
        return Err(AppError::Validation("at least one claim is required".to_string()));
    }

    let corridor_id = req
        .corridor_id
        .as_deref()
        .map(|s| {
            Uuid::parse_str(s)
                .map(CorridorId::from_uuid)
                .map_err(|e| AppError::Validation(format!("invalid corridor_id: {e}")))
        })
        .transpose()?;

    // Build claims.
    let mut claims = Vec::new();
    for c in &req.claims {
        let claim_type = parse_dispute_type(&c.claim_type)?;
        let amount = match (&c.amount, &c.currency) {
            (Some(amt), Some(cur)) => Some(
                Money::new(amt.as_str(), cur.as_str())
                    .map_err(|e| AppError::Validation(format!("invalid claim amount: {e}")))?,
            ),
            _ => None,
        };
        claims.push(Claim {
            claim_id: c.claim_id.clone(),
            claim_type,
            description: c.description.clone(),
            amount,
            supporting_evidence_digests: vec![],
        });
    }

    let filing_digest = digest_from_hex(&req.filing_document_digest)?;

    let claimant = Party {
        did: claimant_did,
        legal_name: req.claimant_name.clone(),
        jurisdiction_id: req
            .claimant_jurisdiction
            .as_deref()
            .and_then(|s| JurisdictionId::new(s).ok()),
    };
    let respondent = Party {
        did: respondent_did,
        legal_name: req.respondent_name.clone(),
        jurisdiction_id: req
            .respondent_jurisdiction
            .as_deref()
            .and_then(|s| JurisdictionId::new(s).ok()),
    };

    let dispute = Dispute::file(
        claimant,
        respondent,
        dispute_type,
        jurisdiction,
        corridor_id,
        req.institution_id.clone(),
        claims,
        FilingEvidence {
            filing_document_digest: filing_digest,
        },
    );

    let response = dispute_to_response(&dispute);
    let id = *dispute.id.as_uuid();
    state.disputes.insert(id, dispute);

    Ok((axum::http::StatusCode::CREATED, Json(response)))
}

/// GET /v1/disputes — List all disputes.
#[utoipa::path(
    get,
    path = "/v1/disputes",
    responses(
        (status = 200, description = "List of disputes", body = Vec<DisputeResponse>),
    ),
    tag = "arbitration"
)]
async fn list_disputes(
    State(state): State<AppState>,
) -> Result<Json<Vec<DisputeResponse>>, AppError> {
    let all = state.disputes.list();
    let responses: Vec<DisputeResponse> = all.iter().map(dispute_to_response).collect();
    Ok(Json(responses))
}

/// GET /v1/disputes/:id — Get dispute details.
#[utoipa::path(
    get,
    path = "/v1/disputes/{id}",
    params(("id" = String, Path, description = "Dispute UUID")),
    responses(
        (status = 200, description = "Dispute details", body = DisputeResponse),
        (status = 404, description = "Dispute not found"),
    ),
    tag = "arbitration"
)]
async fn get_dispute(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DisputeResponse>, AppError> {
    let dispute = state
        .disputes
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("dispute {id} not found")))?;
    Ok(Json(dispute_to_response(&dispute)))
}

/// POST /v1/disputes/:id/begin-review — Filed → UnderReview.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/begin-review",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to UnderReview", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn begin_review(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let case_ref = req
        .case_reference
        .ok_or_else(|| AppError::Validation("case_reference is required for begin-review".to_string()))?;
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .begin_review(ReviewInitiationEvidence {
                case_reference: case_ref.clone(),
                institution_acknowledgment_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/open-evidence — UnderReview → EvidenceCollection.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/open-evidence",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to EvidenceCollection", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn open_evidence(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .open_evidence_collection(EvidencePhaseEvidence {
                procedural_order_digest: digest.clone(),
                evidence_deadline: Timestamp::now(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/schedule-hearing — EvidenceCollection → Hearing.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/schedule-hearing",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to Hearing", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn schedule_hearing(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .schedule_hearing(HearingScheduleEvidence {
                hearing_date: Timestamp::now(),
                tribunal_composition_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/decide — Hearing → Decided.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/decide",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to Decided", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn decide(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .decide(DecisionEvidence {
                ruling_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/enforce — Decided → Enforced.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/enforce",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to Enforced", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn enforce(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .enforce(EnforcementInitiationEvidence {
                enforcement_order_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/close — Enforced → Closed.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/close",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = TransitionRequest,
    responses(
        (status = 200, description = "Transitioned to Closed", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn close(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransitionRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.evidence_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .close(ClosureEvidence {
                final_report_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/settle — Settle from any pre-decision state.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/settle",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = SettleRequest,
    responses(
        (status = 200, description = "Dispute settled", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn settle(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<SettleRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let agreement_digest = digest_from_hex(&req.settlement_agreement_digest)?;
    let consent_digests: Result<Vec<ContentDigest>, AppError> = req
        .party_consent_digests
        .iter()
        .map(|h| digest_from_hex(h))
        .collect();
    let consent_digests = consent_digests?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .settle(SettlementEvidence {
                settlement_agreement_digest: agreement_digest.clone(),
                party_consent_digests: consent_digests.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// POST /v1/disputes/:id/dismiss — Dismiss from Filed or UnderReview.
#[utoipa::path(
    post,
    path = "/v1/disputes/{id}/dismiss",
    params(("id" = String, Path, description = "Dispute UUID")),
    request_body = DismissRequest,
    responses(
        (status = 200, description = "Dispute dismissed", body = DisputeResponse),
        (status = 404, description = "Not found"),
        (status = 409, description = "Invalid transition"),
    ),
    tag = "arbitration"
)]
async fn dismiss(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<DismissRequest>,
) -> Result<Json<DisputeResponse>, AppError> {
    let digest = digest_from_hex(&req.dismissal_order_digest)?;

    let result = state.disputes.try_update(&id, |dispute| {
        dispute
            .dismiss(DismissalEvidence {
                reason: req.reason.clone(),
                dismissal_order_digest: digest.clone(),
            })
            .map(|()| dispute_to_response(dispute))
    });

    match result {
        Some(Ok(resp)) => Ok(Json(resp)),
        Some(Err(e)) => Err(AppError::Conflict(e.to_string())),
        None => Err(AppError::NotFound(format!("dispute {id} not found"))),
    }
}

/// GET /v1/arbitration/institutions — List supported arbitration institutions.
#[utoipa::path(
    get,
    path = "/v1/arbitration/institutions",
    responses(
        (status = 200, description = "List of arbitration institutions"),
    ),
    tag = "arbitration"
)]
async fn list_institutions() -> Json<serde_json::Value> {
    let registry = mez_arbitration::institution_registry();
    let items: Vec<serde_json::Value> = registry
        .iter()
        .map(|inst| {
            serde_json::json!({
                "id": inst.id,
                "name": inst.name,
                "jurisdiction_id": inst.jurisdiction_id,
                "supported_dispute_types": inst.supported_dispute_types.iter().map(|dt| dt.as_str()).collect::<Vec<_>>(),
                "emergency_arbitrator": inst.emergency_arbitrator,
                "expedited_procedure": inst.expedited_procedure,
                "enforcement_jurisdictions": inst.enforcement_jurisdictions,
            })
        })
        .collect();
    Json(serde_json::json!(items))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app(state: AppState) -> Router<()> {
        router().with_state(state)
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    fn file_dispute_body() -> serde_json::Value {
        serde_json::json!({
            "claimant_did": "did:key:z6MkClaimant123",
            "claimant_name": "Trade Corp ADGM Ltd",
            "claimant_jurisdiction": "uae-adgm",
            "respondent_did": "did:key:z6MkRespondent456",
            "respondent_name": "Import Corp AIFC LLP",
            "respondent_jurisdiction": "kaz-aifc",
            "dispute_type": "payment_default",
            "jurisdiction": "uae-difc",
            "institution_id": "difc-lcia",
            "claims": [{
                "claim_id": "claim-001",
                "claim_type": "payment_default",
                "description": "Outstanding payment for delivered goods",
                "amount": "150000",
                "currency": "USD"
            }],
            "filing_document_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        })
    }

    #[tokio::test]
    async fn file_dispute_creates_in_filed_state() {
        let state = AppState::new();
        let app = test_app(state.clone());

        let request = Request::builder()
            .method("POST")
            .uri("/v1/disputes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&file_dispute_body()).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);

        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "FILED");
        assert_eq!(resp.dispute_type, "payment_default");
        assert_eq!(resp.institution_id, "difc-lcia");
        assert_eq!(resp.claim_count, 1);
        assert!(!resp.valid_transitions.is_empty());
    }

    #[tokio::test]
    async fn full_lifecycle_via_api() {
        let state = AppState::new();
        let app = test_app(state.clone());

        // File dispute.
        let request = Request::builder()
            .method("POST")
            .uri("/v1/disputes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&file_dispute_body()).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let resp: DisputeResponse = body_json(response).await;
        let dispute_id = resp.dispute_id;

        // Begin review.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "case_reference": "DIFC-LCIA-2026-001"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/begin-review"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "UNDER_REVIEW");

        // Open evidence.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/open-evidence"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "EVIDENCE_COLLECTION");

        // Schedule hearing.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/schedule-hearing"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "HEARING");

        // Decide.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/decide"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "DECIDED");

        // Enforce.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/enforce"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "ENFORCED");

        // Close.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/close"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "CLOSED");
        assert!(resp.valid_transitions.is_empty());
    }

    #[tokio::test]
    async fn settle_from_filed() {
        let state = AppState::new();
        let app = test_app(state.clone());

        // File.
        let request = Request::builder()
            .method("POST")
            .uri("/v1/disputes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&file_dispute_body()).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let resp: DisputeResponse = body_json(response).await;
        let dispute_id = resp.dispute_id;

        // Settle.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "settlement_agreement_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
            "party_consent_digests": [
                "1111111111111111111111111111111111111111111111111111111111111111",
                "2222222222222222222222222222222222222222222222222222222222222222"
            ]
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/settle"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "SETTLED");
    }

    #[tokio::test]
    async fn dismiss_from_filed() {
        let state = AppState::new();
        let app = test_app(state.clone());

        // File.
        let request = Request::builder()
            .method("POST")
            .uri("/v1/disputes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&file_dispute_body()).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let resp: DisputeResponse = body_json(response).await;
        let dispute_id = resp.dispute_id;

        // Dismiss.
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "reason": "Lack of jurisdiction",
            "dismissal_order_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/dismiss"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let resp: DisputeResponse = body_json(response).await;
        assert_eq!(resp.state, "DISMISSED");
    }

    #[tokio::test]
    async fn invalid_transition_returns_409() {
        let state = AppState::new();
        let app = test_app(state.clone());

        // File.
        let request = Request::builder()
            .method("POST")
            .uri("/v1/disputes")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&file_dispute_body()).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        let resp: DisputeResponse = body_json(response).await;
        let dispute_id = resp.dispute_id;

        // Try to decide (invalid from Filed).
        let app = test_app(state.clone());
        let body = serde_json::json!({
            "evidence_digest": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
        });
        let request = Request::builder()
            .method("POST")
            .uri(format!("/v1/disputes/{dispute_id}/decide"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn get_nonexistent_dispute_returns_404() {
        let state = AppState::new();
        let app = test_app(state);

        let request = Request::builder()
            .method("GET")
            .uri(format!("/v1/disputes/{}", Uuid::new_v4()))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn list_institutions_returns_registry() {
        let state = AppState::new();
        let app = test_app(state);

        let request = Request::builder()
            .method("GET")
            .uri("/v1/arbitration/institutions")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(response).await;
        let items = body.as_array().unwrap();
        assert_eq!(items.len(), 7);
    }

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }
}
