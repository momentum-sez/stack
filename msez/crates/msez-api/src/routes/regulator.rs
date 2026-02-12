//! # Regulator Console API
//!
//! Provides read-only query access for regulatory authorities
//! to monitor zone activity, compliance status, and audit trails.
//! Route structure based on apis/regulator-console.openapi.yaml.

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use axum::extract::rejection::JsonRejection;
use crate::extractors::{Validate, extract_validated_json};
use crate::state::{AppState, AttestationRecord};

/// Query attestations request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryAttestationsRequest {
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
    #[serde(default)]
    pub entity_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub attestation_type: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

impl Validate for QueryAttestationsRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Query results response.
#[derive(Debug, Serialize, ToSchema)]
pub struct QueryResultsResponse {
    pub count: usize,
    pub results: Vec<AttestationRecord>,
}

/// Compliance summary for regulator dashboard.
#[derive(Debug, Serialize, ToSchema)]
pub struct ComplianceSummary {
    pub total_entities: usize,
    pub total_corridors: usize,
    pub total_assets: usize,
    pub total_attestations: usize,
}

/// Build the regulator router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/regulator/query/attestations", post(query_attestations))
        .route("/v1/regulator/summary", get(compliance_summary))
}

/// POST /v1/regulator/query/attestations — Query attestations.
#[utoipa::path(
    post,
    path = "/v1/regulator/query/attestations",
    request_body = QueryAttestationsRequest,
    responses(
        (status = 200, description = "Query results", body = QueryResultsResponse),
    ),
    tag = "regulator"
)]
async fn query_attestations(
    State(state): State<AppState>,
    body: Result<Json<QueryAttestationsRequest>, JsonRejection>,
) -> Result<Json<QueryResultsResponse>, AppError> {
    let req = extract_validated_json(body)?;
    let all = state.attestations.list();
    let filtered: Vec<_> = all
        .into_iter()
        .filter(|a| {
            if let Some(ref jid) = req.jurisdiction_id {
                if a.jurisdiction_id != *jid {
                    return false;
                }
            }
            if let Some(ref eid) = req.entity_id {
                if a.entity_id != *eid {
                    return false;
                }
            }
            if let Some(ref at) = req.attestation_type {
                if a.attestation_type != *at {
                    return false;
                }
            }
            if let Some(ref s) = req.status {
                if a.status != *s {
                    return false;
                }
            }
            true
        })
        .collect();

    let count = filtered.len();
    Ok(Json(QueryResultsResponse {
        count,
        results: filtered,
    }))
}

/// GET /v1/regulator/summary — Compliance summary dashboard.
#[utoipa::path(
    get,
    path = "/v1/regulator/summary",
    responses(
        (status = 200, description = "Compliance summary", body = ComplianceSummary),
    ),
    tag = "regulator"
)]
async fn compliance_summary(
    State(state): State<AppState>,
) -> Json<ComplianceSummary> {
    Json(ComplianceSummary {
        total_entities: state.entities.list().len(),
        total_corridors: state.corridors.list().len(),
        total_assets: state.smart_assets.list().len(),
        total_attestations: state.attestations.list().len(),
    })
}
