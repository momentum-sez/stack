//! # Credential Issuance and Verification
//!
//! Endpoints for issuing signed Verifiable Credentials on compliance
//! evaluation results and verifying VCs presented by counterparties.
//!
//! This module is where computation becomes evidence. The compliance
//! tensor produces an evaluation; this module signs it into a W3C
//! Verifiable Credential that can be transported, stored, and
//! independently verified.
//!
//! ## Endpoints
//!
//! - `POST /v1/assets/:id/credentials/compliance` — Evaluate and attest.
//! - `POST /v1/credentials/verify` — Verify a presented VC.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use msez_core::Timestamp;
use msez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};

use crate::compliance::{
    apply_attestations, build_evaluation_result, build_tensor, AttestationInput,
    ComplianceEvalResult,
};
use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request body for compliance credential issuance.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComplianceCredentialRequest {
    /// Entity ID for entity-specific evaluation.
    pub entity_id: Option<Uuid>,
    /// Attestation evidence per compliance domain.
    /// Keys are domain names (e.g., "aml", "kyc", "sanctions").
    #[serde(default)]
    pub attestations: HashMap<String, AttestationInput>,
}

/// Response from the compliance credential issuance endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ComplianceCredentialResponse {
    /// The compliance evaluation result.
    pub evaluation: ComplianceEvalResult,
    /// The signed VC, if the evaluation was passing. Null if not passing.
    pub credential: Option<serde_json::Value>,
    /// Reason the credential was not issued (if evaluation was not passing).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Response from the credential verification endpoint.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct VerificationResponse {
    /// Whether all proofs verified successfully.
    pub verified: bool,
    /// Number of proofs checked.
    pub proof_count: usize,
    /// Per-proof verification results.
    pub results: Vec<ProofVerificationResult>,
    /// The VC issuer.
    pub issuer: String,
    /// The credential type.
    pub credential_type: String,
}

/// Per-proof verification result.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ProofVerificationResult {
    /// The DID URL of the verification method used.
    pub verification_method: String,
    /// Whether this proof verified successfully.
    pub valid: bool,
    /// Error message if verification failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the credentials router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/assets/:id/credentials/compliance",
            post(issue_compliance_credential),
        )
        .route("/v1/credentials/verify", post(verify_credential))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/assets/:id/credentials/compliance — Evaluate and attest.
///
/// Runs the compliance tensor, and if the result is passing, constructs
/// a signed Verifiable Credential attesting to the evaluation result.
/// The VC can be independently verified by any party with the zone's
/// public key.
///
/// Returns both the evaluation result and the signed credential (if issued).
/// If the evaluation is not passing, the credential field is null and the
/// response explains which domains are blocking.
#[utoipa::path(
    post,
    path = "/v1/assets/{id}/credentials/compliance",
    params(("id" = Uuid, Path, description = "Asset ID")),
    request_body = ComplianceCredentialRequest,
    responses(
        (status = 200, description = "Compliance evaluated and credential issued (if passing)",
            body = ComplianceCredentialResponse),
        (status = 404, description = "Asset not found", body = crate::error::ErrorBody),
    ),
    tag = "credentials"
)]
async fn issue_compliance_credential(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Json<serde_json::Value>,
) -> Result<Json<ComplianceCredentialResponse>, AppError> {
    // Parse the request body manually to give better error messages.
    let req: ComplianceCredentialRequest = serde_json::from_value(body.0)
        .map_err(|e| AppError::BadRequest(format!("invalid request body: {e}")))?;

    // Validate attestation bounds.
    const MAX_ATTESTATIONS: usize = 100;
    if req.attestations.len() > MAX_ATTESTATIONS {
        return Err(AppError::Validation(format!(
            "attestations must not exceed {MAX_ATTESTATIONS} entries"
        )));
    }
    for key in req.attestations.keys() {
        if key.len() > 100 {
            return Err(AppError::Validation(
                "attestation domain name must not exceed 100 characters".to_string(),
            ));
        }
    }

    // ── Act 1: Evaluate ─────────────────────────────────────────
    let asset = state
        .smart_assets
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))?;

    let mut tensor = build_tensor(&asset.jurisdiction_id);
    apply_attestations(&mut tensor, &req.attestations);

    let evaluation = build_evaluation_result(&tensor, &asset, id);

    let slice = tensor.full_slice();
    let aggregate = slice.aggregate_state();

    // ── Act 2: Decide ───────────────────────────────────────────
    if !aggregate.is_passing() {
        let reason = format!(
            "evaluation is not passing (aggregate: {}). Blocking domains: [{}]",
            evaluation.overall_status,
            evaluation.blocking_domains.join(", ")
        );
        return Ok(Json(ComplianceCredentialResponse {
            evaluation,
            credential: None,
            reason: Some(reason),
        }));
    }

    // ── Act 3: Attest ───────────────────────────────────────────
    let now = Timestamp::now();

    // Build the credential subject.
    let subject = serde_json::json!({
        "asset_id": id.to_string(),
        "jurisdiction_id": asset.jurisdiction_id,
        "evaluation_aggregate": evaluation.overall_status,
        "tensor_commitment": evaluation.tensor_commitment,
        "evaluated_at": now.to_string(),
        "domain_count": 20,
        "passing_domains": evaluation.passing_domains,
    });

    let mut vc = VerifiableCredential {
        context: ContextValue::Array(vec![serde_json::Value::String(
            "https://www.w3.org/2018/credentials/v1".into(),
        )]),
        id: Some(format!(
            "urn:msez:vc:compliance:{}:{}",
            id,
            now.as_datetime().format("%Y%m%dT%H%M%SZ")
        )),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".into(),
            "MsezComplianceAttestationCredential".into(),
        ]),
        issuer: state.zone_did.clone(),
        issuance_date: *now.as_datetime(),
        expiration_date: None,
        credential_subject: subject,
        proof: ProofValue::default(),
    };

    // Sign with the zone key.
    vc.sign_ed25519(
        &state.zone_signing_key,
        format!("{}#key-1", state.zone_did),
        ProofType::Ed25519Signature2020,
        Some(now),
    )
    .map_err(|e| AppError::Internal(format!("VC signing failed: {e}")))?;

    let vc_value = serde_json::to_value(&vc)
        .map_err(|e| AppError::Internal(format!("VC serialization failed: {e}")))?;

    Ok(Json(ComplianceCredentialResponse {
        evaluation,
        credential: Some(vc_value),
        reason: None,
    }))
}

/// POST /v1/credentials/verify — Verify a Verifiable Credential.
///
/// Accepts a VC, resolves the verification method against known zone keys,
/// and returns the verification result. Currently resolves only the zone's
/// own key; external DID resolution is a Phase 2 feature.
#[utoipa::path(
    post,
    path = "/v1/credentials/verify",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Verification result", body = VerificationResponse),
    ),
    tag = "credentials"
)]
async fn verify_credential(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<VerificationResponse>, AppError> {
    let vc: VerifiableCredential = serde_json::from_value(body)
        .map_err(|e| AppError::BadRequest(format!("invalid VC: {e}")))?;

    // Resolve verification methods: for now, only the zone's own key.
    let zone_vk = state.zone_signing_key.verifying_key();
    let zone_did = state.zone_did.clone();

    // Collect detailed per-proof results for the response.
    let results = vc.verify(|verification_method| {
        if verification_method.starts_with(&zone_did) {
            Ok(zone_vk.clone())
        } else {
            Err(format!(
                "unknown verification method: {verification_method}"
            ))
        }
    });
    let proof_count = results.len();

    // Apply the same holistic checks that verify_all() enforces:
    // (a) reject credentials with zero proofs (vacuously-true iterator bug),
    // (b) reject expired credentials regardless of signature validity.
    let proofs_ok = !results.is_empty() && results.iter().all(|r| r.ok);
    let expired = vc
        .expiration_date
        .is_some_and(|exp| exp < chrono::Utc::now());
    let all_ok = proofs_ok && !expired;

    let credential_type = match &vc.credential_type {
        CredentialTypeValue::Single(s) => s.clone(),
        CredentialTypeValue::Array(arr) => arr.join(", "),
    };

    Ok(Json(VerificationResponse {
        verified: all_ok,
        proof_count,
        results: results
            .into_iter()
            .map(|r| ProofVerificationResult {
                verification_method: r.verification_method,
                valid: r.ok,
                error: if r.error.is_empty() {
                    None
                } else {
                    Some(r.error)
                },
            })
            .collect(),
        issuer: vc.issuer.clone(),
        credential_type,
    }))
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

    /// A zone admin identity for tests that need full access.
    fn zone_admin() -> crate::auth::CallerIdentity {
        crate::auth::CallerIdentity {
            role: crate::auth::Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    /// Build the full credentials + smart assets router for integration tests.
    fn test_app() -> Router<()> {
        let state = AppState::new();
        Router::new()
            .merge(crate::routes::smart_assets::router())
            .merge(router())
            .layer(axum::Extension(zone_admin()))
            .with_state(state)
    }

    /// Build a router with shared state for multi-step tests.
    fn test_app_with_state(state: AppState) -> Router<()> {
        Router::new()
            .merge(crate::routes::smart_assets::router())
            .merge(router())
            .layer(axum::Extension(zone_admin()))
            .with_state(state)
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Create an asset and return its ID.
    async fn create_test_asset(app: &Router<()>) -> Uuid {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/assets/genesis")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ","metadata":{}}"#,
            ))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let record: crate::state::SmartAssetRecord = body_json(resp).await;
        record.id
    }

    /// Build attestation JSON that makes all 20 domains compliant.
    fn all_compliant_attestations() -> serde_json::Value {
        let domains = [
            "aml",
            "kyc",
            "sanctions",
            "tax",
            "securities",
            "corporate",
            "custody",
            "data_privacy",
            "licensing",
            "banking",
            "payments",
            "clearing",
            "settlement",
            "digital_assets",
            "employment",
            "immigration",
            "ip",
            "consumer_protection",
            "arbitration",
            "trade",
        ];
        let mut attestations = serde_json::Map::new();
        for domain in &domains {
            attestations.insert(
                domain.to_string(),
                serde_json::json!({"status": "compliant"}),
            );
        }
        serde_json::Value::Object(attestations)
    }

    // ── Integration tests ────────────────────────────────────────

    #[tokio::test]
    async fn issue_and_verify_round_trip() {
        let state = AppState::new();
        let app = test_app_with_state(state);

        // 1. Create an asset.
        let asset_id = create_test_asset(&app).await;

        // 2. Issue a compliance credential with all domains passing.
        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "attestations": all_compliant_attestations()
                }))
                .unwrap(),
            ))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        assert_eq!(issue_resp.status(), StatusCode::OK);

        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;
        assert!(
            cred_response.credential.is_some(),
            "credential should be issued for passing evaluation"
        );
        assert!(cred_response.reason.is_none());
        assert_eq!(cred_response.evaluation.overall_status, "compliant");

        // 3. Verify the credential.
        let credential = cred_response.credential.unwrap();
        let verify_req = Request::builder()
            .method("POST")
            .uri("/v1/credentials/verify")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&credential).unwrap()))
            .unwrap();

        let verify_resp = app.clone().oneshot(verify_req).await.unwrap();
        assert_eq!(verify_resp.status(), StatusCode::OK);

        let verification: VerificationResponse = body_json(verify_resp).await;
        assert!(verification.verified, "credential should verify");
        assert_eq!(verification.proof_count, 1);
        assert!(verification.results[0].valid);
        assert!(verification.results[0].error.is_none());
    }

    #[tokio::test]
    async fn non_passing_evaluation_returns_no_credential() {
        let state = AppState::new();
        let app = test_app_with_state(state);

        let asset_id = create_test_asset(&app).await;

        // Issue with NO attestations — all domains will be Pending (not passing).
        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"attestations":{}}"#))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        assert_eq!(issue_resp.status(), StatusCode::OK);

        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;
        assert!(
            cred_response.credential.is_none(),
            "credential should NOT be issued for non-passing evaluation"
        );
        assert!(cred_response.reason.is_some());
        let reason = cred_response.reason.unwrap();
        assert!(
            reason.contains("not passing"),
            "reason should explain non-passing: {reason}"
        );
    }

    #[tokio::test]
    async fn tampered_credential_fails_verification() {
        let state = AppState::new();
        let app = test_app_with_state(state);

        let asset_id = create_test_asset(&app).await;

        // Issue a valid credential.
        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "attestations": all_compliant_attestations()
                }))
                .unwrap(),
            ))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;
        let mut credential = cred_response.credential.unwrap();

        // Tamper with the credential subject.
        credential["credentialSubject"]["evaluation_aggregate"] =
            serde_json::Value::String("tampered".into());

        // Verify the tampered credential.
        let verify_req = Request::builder()
            .method("POST")
            .uri("/v1/credentials/verify")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&credential).unwrap()))
            .unwrap();

        let verify_resp = app.clone().oneshot(verify_req).await.unwrap();
        assert_eq!(verify_resp.status(), StatusCode::OK);

        let verification: VerificationResponse = body_json(verify_resp).await;
        assert!(
            !verification.verified,
            "tampered credential should fail verification"
        );
        assert!(!verification.results[0].valid);
        assert!(verification.results[0].error.is_some());
    }

    #[tokio::test]
    async fn credential_contains_correct_issuer_and_type() {
        let state = AppState::new();
        let app = test_app_with_state(state.clone());

        let asset_id = create_test_asset(&app).await;

        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&serde_json::json!({
                    "attestations": all_compliant_attestations()
                }))
                .unwrap(),
            ))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;
        let credential = cred_response.credential.unwrap();

        // Verify issuer.
        let issuer = credential["issuer"].as_str().unwrap();
        assert!(
            issuer.starts_with("did:mass:zone:"),
            "issuer should be a zone DID: {issuer}"
        );
        assert_eq!(issuer, state.zone_did);

        // Verify credential type.
        let types = credential["type"].as_array().unwrap();
        let type_strs: Vec<&str> = types.iter().map(|t| t.as_str().unwrap()).collect();
        assert!(type_strs.contains(&"VerifiableCredential"));
        assert!(type_strs.contains(&"MsezComplianceAttestationCredential"));

        // Verify ID format.
        let vc_id = credential["id"].as_str().unwrap();
        assert!(
            vc_id.starts_with("urn:msez:vc:compliance:"),
            "id should follow URN format: {vc_id}"
        );
    }

    #[tokio::test]
    async fn asset_not_found_returns_404() {
        let app = test_app();
        let missing_id = Uuid::new_v4();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{missing_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"attestations":{}}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn verify_invalid_vc_returns_422() {
        // BUG-038: BadRequest now returns 422 (Unprocessable Entity).
        let app = test_app();

        let req = Request::builder()
            .method("POST")
            .uri("/v1/credentials/verify")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"not": "a credential"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn evaluation_result_has_20_domains() {
        let state = AppState::new();
        let app = test_app_with_state(state);

        let asset_id = create_test_asset(&app).await;

        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"attestations":{}}"#))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;

        assert_eq!(cred_response.evaluation.domain_count, 20);
        assert_eq!(cred_response.evaluation.domain_results.len(), 20);
        assert!(cred_response.evaluation.tensor_commitment.is_some());
    }

    #[tokio::test]
    async fn partial_attestations_still_non_passing() {
        let state = AppState::new();
        let app = test_app_with_state(state);

        let asset_id = create_test_asset(&app).await;

        // Only attest a few domains — the rest remain Pending.
        let issue_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/assets/{asset_id}/credentials/compliance"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"attestations":{"aml":{"status":"compliant"},"kyc":{"status":"compliant"}}}"#,
            ))
            .unwrap();

        let issue_resp = app.clone().oneshot(issue_req).await.unwrap();
        let cred_response: ComplianceCredentialResponse = body_json(issue_resp).await;

        // With only 2 of 20 domains attested, result should not be passing.
        assert!(
            cred_response.credential.is_none(),
            "should not issue credential with 18 pending domains"
        );
    }

    #[tokio::test]
    async fn router_builds_successfully() {
        let _router = router();
    }
}
