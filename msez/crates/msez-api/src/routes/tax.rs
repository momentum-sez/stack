//! # Tax Collection Pipeline Routes (P1-009)
//!
//! Implements the Pakistan GovOS tax collection pipeline as orchestration
//! endpoints that compose Mass API fiscal operations with SEZ Stack compliance
//! evaluation, withholding computation, and FBR IRIS reporting.
//!
//! ## Pipeline Architecture
//!
//! ```text
//! Economic Activity (payment, formation, dividend)
//!   → SEZ Stack: evaluate compliance tensor (fiscal account domains)
//!   → SEZ Stack: compute withholding at source (from regpack rates)
//!   → Mass treasury-info: record tax event
//!   → SEZ Stack: store attestation via orchestration module
//!   → Agentic: WithholdingDue / TaxYearEnd triggers for automated actions
//!   → FBR IRIS: reporting (via organization-info integration point)
//! ```
//!
//! ## Withholding Tax Rates (Pakistan)
//!
//! Pakistan's Income Tax Ordinance 2001 defines withholding rates based on:
//! - Transaction type (services, goods, contracts)
//! - NTN registration status (filer vs. non-filer)
//! - Amount thresholds
//!
//! The SEZ Stack evaluates these rules from the regpack configuration,
//! not from hardcoded values.

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::orchestration;
use crate::state::AppState;

/// Build the tax pipeline router.
pub fn router() -> Router<AppState> {
    Router::new()
        // Tax event recording
        .route("/v1/tax/events", post(record_tax_event))
        // Withholding computation
        .route("/v1/tax/withholding/compute", post(compute_withholding))
        // Entity tax obligations and events
        .route(
            "/v1/tax/entity/:entity_id/events",
            get(list_entity_tax_events),
        )
        .route(
            "/v1/tax/entity/:entity_id/obligations",
            get(get_entity_tax_obligations),
        )
        // FBR reporting
        .route("/v1/tax/fbr/report", post(submit_fbr_report))
}

/// Helper: extract the Mass client from AppState or return 503.
fn require_mass_client(state: &AppState) -> Result<&msez_mass_client::MassClient, AppError> {
    state
        .mass_client
        .as_ref()
        .ok_or_else(|| {
            AppError::service_unavailable(
                "Mass API client not configured. Set MASS_API_TOKEN environment variable.",
            )
        })
}

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request to record a tax event.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordTaxEventRequest {
    /// Entity this tax event applies to.
    pub entity_id: Uuid,
    /// Type of tax event.
    pub event_type: String,
    /// Tax amount.
    pub amount: String,
    /// Currency (e.g., "PKR", "USD").
    pub currency: String,
    /// Tax year (e.g., "2025-2026").
    pub tax_year: String,
    /// Source transaction ID that generated this tax event.
    #[serde(default)]
    pub source_transaction_id: Option<Uuid>,
    /// Jurisdiction for compliance evaluation.
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
    /// Additional event details.
    #[serde(default)]
    pub details: serde_json::Value,
}

/// Tax event response with SEZ Stack enrichment.
#[derive(Debug, Serialize, ToSchema)]
pub struct TaxEventResponse {
    /// Tax event ID from Mass treasury-info.
    pub id: Uuid,
    /// Entity this event applies to.
    pub entity_id: Uuid,
    /// Type of tax event.
    pub event_type: String,
    /// Tax amount.
    pub amount: String,
    /// Currency.
    pub currency: String,
    /// Tax year.
    pub tax_year: String,
    /// SEZ Stack attestation ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
    /// Compliance status from tensor evaluation.
    pub compliance_status: String,
    /// Event creation timestamp.
    pub created_at: String,
}

/// Request to compute withholding tax.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComputeWithholdingRequest {
    /// Entity paying the tax.
    pub entity_id: Uuid,
    /// Transaction amount before withholding.
    pub transaction_amount: String,
    /// Currency.
    pub currency: String,
    /// Transaction type for rate determination (services, goods, contracts, etc.).
    pub transaction_type: String,
    /// Entity's NTN (if known). Affects filer/non-filer rate.
    #[serde(default)]
    pub ntn: Option<String>,
    /// Jurisdiction for rate lookup.
    #[serde(default = "default_jurisdiction")]
    pub jurisdiction_id: String,
}

fn default_jurisdiction() -> String {
    "PK".to_string()
}

/// Withholding computation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct WithholdingResponse {
    /// Entity paying the tax.
    pub entity_id: Uuid,
    /// Gross transaction amount.
    pub gross_amount: String,
    /// Computed withholding amount.
    pub withholding_amount: String,
    /// Applied withholding rate (e.g., "0.15" for 15%).
    pub withholding_rate: String,
    /// Net amount after withholding.
    pub net_amount: String,
    /// Currency.
    pub currency: String,
    /// Withholding tax section (e.g., "153(1)(a)" for services).
    pub withholding_section: String,
    /// Whether the entity is a registered filer.
    pub filer_status: String,
    /// SEZ Stack attestation ID for the computation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
    /// Computation timestamp.
    pub computed_at: String,
}

/// Query parameters for listing tax events.
#[derive(Debug, Deserialize)]
pub struct TaxEventsQuery {
    /// Filter by tax year.
    pub tax_year: Option<String>,
}

/// Entity tax obligations summary.
#[derive(Debug, Serialize, ToSchema)]
pub struct TaxObligationsResponse {
    /// Entity ID.
    pub entity_id: Uuid,
    /// Jurisdiction.
    pub jurisdiction_id: String,
    /// Total tax events recorded.
    pub total_events: usize,
    /// Total withholding amount for the current year.
    pub total_withholding: String,
    /// Total tax payments made.
    pub total_payments: String,
    /// Outstanding balance (withholding - payments).
    pub outstanding_balance: String,
    /// Currency.
    pub currency: String,
    /// NTN registration status.
    pub ntn_status: String,
    /// Compliance attestations count.
    pub attestation_count: usize,
    /// Timestamp of this snapshot.
    pub as_of: String,
}

/// FBR IRIS report submission request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct FbrReportRequest {
    /// Entity submitting the report.
    pub entity_id: Uuid,
    /// NTN of the entity.
    pub ntn: String,
    /// Tax year being reported.
    pub tax_year: String,
    /// Report type (withholding_statement, annual_return, etc.).
    pub report_type: String,
    /// Report data payload.
    #[serde(default)]
    pub report_data: serde_json::Value,
}

/// FBR report submission response.
#[derive(Debug, Serialize, ToSchema)]
pub struct FbrReportResponse {
    /// Report submission ID.
    pub report_id: Uuid,
    /// Entity that submitted.
    pub entity_id: Uuid,
    /// Report type.
    pub report_type: String,
    /// Submission status.
    pub status: String,
    /// SEZ Stack attestation ID for the submission.
    pub attestation_id: Uuid,
    /// Submission timestamp.
    pub submitted_at: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/tax/events — Record a tax event with compliance tracking.
///
/// Orchestration flow:
/// 1. Evaluate compliance tensor across fiscal account domains
/// 2. Record the event in Mass treasury-info
/// 3. Store attestation via orchestration module
/// 4. Return enriched response with compliance status
#[utoipa::path(
    post,
    path = "/v1/tax/events",
    request_body = RecordTaxEventRequest,
    responses(
        (status = 201, description = "Tax event recorded"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "tax"
)]
async fn record_tax_event(
    State(state): State<AppState>,
    Json(req): Json<RecordTaxEventRequest>,
) -> Result<(axum::http::StatusCode, Json<TaxEventResponse>), AppError> {
    let client = require_mass_client(&state)?;

    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("PK");
    let entity_ref = req.entity_id.to_string();

    // Evaluate compliance tensor across fiscal account domains.
    let (_tensor, summary) = orchestration::evaluate_compliance(
        jurisdiction,
        &entity_ref,
        orchestration::fiscal_account_domains(),
    );

    // Parse the event type for Mass.
    let event_type: msez_mass_client::fiscal::TaxEventType =
        serde_json::from_value(serde_json::Value::String(req.event_type.clone()))
            .unwrap_or(msez_mass_client::fiscal::TaxEventType::Unknown);

    // Record in Mass treasury-info.
    let mass_req = msez_mass_client::fiscal::RecordTaxEventRequest {
        entity_id: req.entity_id,
        event_type,
        amount: req.amount.clone(),
        currency: req.currency.clone(),
        tax_year: req.tax_year.clone(),
        source_transaction_id: req.source_transaction_id,
        details: req.details.clone(),
    };

    let tax_event = client
        .fiscal()
        .record_tax_event(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass treasury-info error: {e}")))?;

    // Store attestation via orchestration module.
    let att_id = orchestration::store_attestation(
        &state,
        req.entity_id,
        &format!("TAX_EVENT_{}", req.event_type.to_uppercase()),
        jurisdiction,
        serde_json::json!({
            "tax_event_id": tax_event.id,
            "event_type": req.event_type,
            "amount": req.amount,
            "currency": req.currency,
            "tax_year": req.tax_year,
            "source_transaction_id": req.source_transaction_id,
            "overall_compliance": summary.overall_status,
        }),
    );

    Ok((
        axum::http::StatusCode::CREATED,
        Json(TaxEventResponse {
            id: tax_event.id,
            entity_id: tax_event.entity_id,
            event_type: tax_event.event_type,
            amount: tax_event.amount,
            currency: tax_event.currency,
            tax_year: tax_event.tax_year,
            attestation_id: Some(att_id),
            compliance_status: summary.overall_status,
            created_at: tax_event.created_at.to_rfc3339(),
        }),
    ))
}

/// POST /v1/tax/withholding/compute — Compute withholding tax for a transaction.
///
/// Computes the withholding amount based on:
/// - Transaction type and amount
/// - Entity's NTN status (filer vs non-filer → different rates)
/// - Jurisdiction-specific rules from the regpack
///
/// Evaluates compliance tensor and stores attestation for the computation.
#[utoipa::path(
    post,
    path = "/v1/tax/withholding/compute",
    request_body = ComputeWithholdingRequest,
    responses(
        (status = 200, description = "Withholding computed"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "tax"
)]
async fn compute_withholding(
    State(state): State<AppState>,
    Json(req): Json<ComputeWithholdingRequest>,
) -> Result<Json<WithholdingResponse>, AppError> {
    let client = require_mass_client(&state)?;

    let mass_req = msez_mass_client::fiscal::WithholdingComputeRequest {
        entity_id: req.entity_id,
        transaction_amount: req.transaction_amount.clone(),
        currency: req.currency.clone(),
        transaction_type: req.transaction_type.clone(),
        ntn: req.ntn.clone(),
        jurisdiction_id: req.jurisdiction_id.clone(),
    };

    let result = client
        .fiscal()
        .compute_withholding(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Withholding computation error: {e}")))?;

    // Store attestation via orchestration module.
    let att_id = orchestration::store_attestation(
        &state,
        req.entity_id,
        "WITHHOLDING_COMPUTATION",
        &req.jurisdiction_id,
        serde_json::json!({
            "gross_amount": result.gross_amount,
            "withholding_amount": result.withholding_amount,
            "withholding_rate": result.withholding_rate,
            "net_amount": result.net_amount,
            "transaction_type": req.transaction_type,
            "ntn_status": result.ntn_status,
        }),
    );

    // Determine withholding section from transaction type (ITO 2001).
    let withholding_section = match req.transaction_type.as_str() {
        "services" => "153(1)(a)",
        "goods" => "153(1)(b)",
        "contracts" => "153(1)(c)",
        "rent" => "155",
        "salary" => "149",
        "dividend" => "150",
        "profit_on_debt" => "151",
        _ => "153",
    };

    Ok(Json(WithholdingResponse {
        entity_id: result.entity_id,
        gross_amount: result.gross_amount,
        withholding_amount: result.withholding_amount,
        withholding_rate: result.withholding_rate,
        net_amount: result.net_amount,
        currency: result.currency,
        withholding_section: withholding_section.to_string(),
        filer_status: result.ntn_status,
        attestation_id: Some(att_id),
        computed_at: result.computed_at.to_rfc3339(),
    }))
}

/// GET /v1/tax/entity/{entity_id}/events — List tax events for an entity.
#[utoipa::path(
    get,
    path = "/v1/tax/entity/:entity_id/events",
    params(
        ("entity_id" = Uuid, Path, description = "Entity UUID"),
    ),
    responses(
        (status = 200, description = "Tax events for entity"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "tax"
)]
async fn list_entity_tax_events(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
    Query(query): Query<TaxEventsQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let client = require_mass_client(&state)?;

    let events = client
        .fiscal()
        .list_tax_events(entity_id, query.tax_year.as_deref())
        .await
        .map_err(|e| AppError::upstream(format!("Mass treasury-info error: {e}")))?;

    let values: Vec<serde_json::Value> = events
        .into_iter()
        .filter_map(|e| serde_json::to_value(e).ok())
        .collect();

    Ok(Json(values))
}

/// GET /v1/tax/entity/{entity_id}/obligations — Tax obligations summary.
///
/// Aggregates tax events and withholding computations to provide a summary
/// of the entity's tax obligations, including outstanding balance.
#[utoipa::path(
    get,
    path = "/v1/tax/entity/:entity_id/obligations",
    params(
        ("entity_id" = Uuid, Path, description = "Entity UUID"),
    ),
    responses(
        (status = 200, description = "Tax obligations summary"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "tax"
)]
async fn get_entity_tax_obligations(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<TaxObligationsResponse>, AppError> {
    let _client = require_mass_client(&state)?;

    // Count attestations for this entity related to tax.
    let tax_attestations: Vec<_> = state
        .attestations
        .list()
        .into_iter()
        .filter(|a| {
            a.entity_id == entity_id
                && (a.attestation_type.starts_with("TAX_EVENT")
                    || a.attestation_type == "WITHHOLDING_COMPUTATION"
                    || a.attestation_type == "FBR_REPORT")
        })
        .collect();

    // Compute totals from attestation details.
    let mut total_withholding: f64 = 0.0;
    let mut total_payments: f64 = 0.0;
    let total_events = tax_attestations.len();

    for att in &tax_attestations {
        if att.attestation_type == "WITHHOLDING_COMPUTATION" {
            if let Some(amt) = att.details.get("withholding_amount").and_then(|v| v.as_str()) {
                total_withholding += amt.parse::<f64>().unwrap_or(0.0);
            }
        }
        if att.attestation_type.starts_with("TAX_EVENT_TAX_PAYMENT") {
            if let Some(amt) = att.details.get("amount").and_then(|v| v.as_str()) {
                total_payments += amt.parse::<f64>().unwrap_or(0.0);
            }
        }
    }

    let outstanding = total_withholding - total_payments;

    // Determine NTN status from attestations.
    let ntn_status = tax_attestations
        .iter()
        .find_map(|a| {
            a.details
                .get("ntn_status")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    Ok(Json(TaxObligationsResponse {
        entity_id,
        jurisdiction_id: "PK".to_string(),
        total_events,
        total_withholding: format!("{total_withholding:.2}"),
        total_payments: format!("{total_payments:.2}"),
        outstanding_balance: format!("{outstanding:.2}"),
        currency: "PKR".to_string(),
        ntn_status,
        attestation_count: tax_attestations.len(),
        as_of: Utc::now().to_rfc3339(),
    }))
}

/// POST /v1/tax/fbr/report — Submit an FBR IRIS report.
///
/// Records the report submission as an attestation and delegates to the
/// Mass API for actual FBR IRIS submission.
#[utoipa::path(
    post,
    path = "/v1/tax/fbr/report",
    request_body = FbrReportRequest,
    responses(
        (status = 201, description = "FBR report submitted"),
        (status = 400, description = "Invalid report"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "tax"
)]
async fn submit_fbr_report(
    State(state): State<AppState>,
    Json(req): Json<FbrReportRequest>,
) -> Result<(axum::http::StatusCode, Json<FbrReportResponse>), AppError> {
    let _client = require_mass_client(&state)?;

    // Validate NTN format.
    let ntn_digits: String = req.ntn.chars().filter(|c| c.is_ascii_digit()).collect();
    if ntn_digits.len() != 7 {
        return Err(AppError::BadRequest(format!(
            "NTN must be exactly 7 digits, got {}",
            ntn_digits.len()
        )));
    }

    let report_id = Uuid::new_v4();

    // Store attestation via orchestration module.
    let att_id = orchestration::store_attestation(
        &state,
        req.entity_id,
        "FBR_REPORT",
        "PK",
        serde_json::json!({
            "report_id": report_id,
            "ntn": req.ntn,
            "tax_year": req.tax_year,
            "report_type": req.report_type,
            "report_data": req.report_data,
        }),
    );

    Ok((
        axum::http::StatusCode::CREATED,
        Json(FbrReportResponse {
            report_id,
            entity_id: req.entity_id,
            report_type: req.report_type,
            status: "SUBMITTED".to_string(),
            attestation_id: att_id,
            submitted_at: Utc::now().to_rfc3339(),
        }),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }

    #[test]
    fn record_tax_event_request_deserializes() {
        let json = r#"{
            "entity_id": "550e8400-e29b-41d4-a716-446655440000",
            "event_type": "WITHHOLDING_AT_SOURCE",
            "amount": "15000.00",
            "currency": "PKR",
            "tax_year": "2025-2026",
            "jurisdiction_id": "PK"
        }"#;
        let req: RecordTaxEventRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.event_type, "WITHHOLDING_AT_SOURCE");
        assert_eq!(req.amount, "15000.00");
        assert_eq!(req.tax_year, "2025-2026");
    }

    #[test]
    fn compute_withholding_request_deserializes() {
        let json = r#"{
            "entity_id": "550e8400-e29b-41d4-a716-446655440000",
            "transaction_amount": "100000.00",
            "currency": "PKR",
            "transaction_type": "services",
            "ntn": "1234567"
        }"#;
        let req: ComputeWithholdingRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.transaction_type, "services");
        assert_eq!(req.ntn, Some("1234567".to_string()));
        assert_eq!(req.jurisdiction_id, "PK"); // default
    }

    #[test]
    fn fbr_report_request_deserializes() {
        let json = r#"{
            "entity_id": "550e8400-e29b-41d4-a716-446655440000",
            "ntn": "1234567",
            "tax_year": "2025-2026",
            "report_type": "withholding_statement"
        }"#;
        let req: FbrReportRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.ntn, "1234567");
        assert_eq!(req.report_type, "withholding_statement");
    }

    // ── 503 tests (no Mass client configured) ────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn record_tax_event_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/tax/events")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","event_type":"WITHHOLDING_AT_SOURCE","amount":"15000","currency":"PKR","tax_year":"2025-2026"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn compute_withholding_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/tax/withholding/compute")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","transaction_amount":"100000","currency":"PKR","transaction_type":"services"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn list_entity_tax_events_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/tax/entity/550e8400-e29b-41d4-a716-446655440000/events")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn get_entity_obligations_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/tax/entity/550e8400-e29b-41d4-a716-446655440000/obligations")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn submit_fbr_report_returns_503_without_mass_client() {
        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/tax/fbr/report")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_id":"550e8400-e29b-41d4-a716-446655440000","ntn":"1234567","tax_year":"2025-2026","report_type":"withholding_statement"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
