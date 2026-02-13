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
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, AttestationRecord};
use axum::extract::rejection::JsonRejection;

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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct QueryResultsResponse {
    pub count: usize,
    pub results: Vec<AttestationRecord>,
}

/// Compliance summary for regulator dashboard.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
async fn compliance_summary(State(state): State<AppState>) -> Json<ComplianceSummary> {
    Json(ComplianceSummary {
        total_entities: state.entities.list().len(),
        total_corridors: state.corridors.list().len(),
        total_assets: state.smart_assets.list().len(),
        total_attestations: state.attestations.list().len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── QueryAttestationsRequest validation ───────────────────────

    #[test]
    fn test_query_attestations_request_valid_empty() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: None,
            entity_id: None,
            attestation_type: None,
            status: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_attestations_request_valid_with_filters() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: Some("PK-PSEZ".to_string()),
            entity_id: Some(uuid::Uuid::new_v4()),
            attestation_type: Some("identity_verification".to_string()),
            status: Some("ACTIVE".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_query_attestations_request_valid_partial_filters() {
        let req = QueryAttestationsRequest {
            jurisdiction_id: Some("AE-DIFC".to_string()),
            entity_id: None,
            attestation_type: None,
            status: Some("PENDING".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }

    // ── Handler integration tests ──────────────────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper: build the regulator router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_query_attestations_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 0);
        assert!(result.results.is_empty());
    }

    #[tokio::test]
    async fn handler_query_attestations_with_filters_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_id":"PK-PSEZ","status":"ACTIVE"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 0);
    }

    #[tokio::test]
    async fn handler_query_attestations_filters_matching_records() {
        let state = AppState::new();

        // Seed the attestations store directly.
        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "identity_verification".to_string(),
            issuer: "NADRA".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "compliance_check".to_string(),
            issuer: "FBR".to_string(),
            status: "PENDING".to_string(),
            jurisdiction_id: "AE-DIFC".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1.clone());
        state.attestations.insert(att2.id, att2.clone());

        let app = router().with_state(state.clone());

        // Query filtering by jurisdiction_id.
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"jurisdiction_id":"PK-PSEZ"}"#))
            .unwrap();

        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].jurisdiction_id, "PK-PSEZ");

        // Query with no filters returns all.
        let req_all = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"{}"#))
            .unwrap();
        let resp_all = app.oneshot(req_all).await.unwrap();
        let result_all: QueryResultsResponse = body_json(resp_all).await;
        assert_eq!(result_all.count, 2);
    }

    #[tokio::test]
    async fn handler_compliance_summary_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_entities, 0);
        assert_eq!(summary.total_corridors, 0);
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_attestations, 0);
    }

    #[tokio::test]
    async fn handler_compliance_summary_reflects_state() {
        let state = AppState::new();

        // Add some entities and corridors to the state.
        let entity = crate::state::EntityRecord {
            id: uuid::Uuid::new_v4(),
            entity_type: "company".to_string(),
            legal_name: "Test Corp".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: "ACTIVE".to_string(),
            beneficial_owners: vec![],
            dissolution_stage: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state.entities.insert(entity.id, entity);

        let corridor = crate::state::CorridorRecord {
            id: uuid::Uuid::new_v4(),
            jurisdiction_a: "PK-PSEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
            state: "ACTIVE".to_string(),
            transition_log: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state.corridors.insert(corridor.id, corridor);

        let app = router().with_state(state.clone());

        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_entities, 1);
        assert_eq!(summary.total_corridors, 1);
        assert_eq!(summary.total_assets, 0);
        assert_eq!(summary.total_attestations, 0);
    }

    // ── Additional regulator route tests ─────────────────────────

    #[tokio::test]
    async fn handler_query_attestations_invalid_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(r#"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_query_attestations_filter_by_entity_id() {
        let state = AppState::new();
        let target_entity = uuid::Uuid::new_v4();

        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: target_entity,
            attestation_type: "kyc".to_string(),
            issuer: "NADRA".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "kyc".to_string(),
            issuer: "FBR".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1);
        state.attestations.insert(att2.id, att2);

        let app = router().with_state(state);
        let body = serde_json::json!({ "entity_id": target_entity });
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].entity_id, target_entity);
    }

    #[tokio::test]
    async fn handler_query_attestations_filter_by_attestation_type() {
        let state = AppState::new();

        let att1 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "identity_verification".to_string(),
            issuer: "NADRA".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        let att2 = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "compliance_check".to_string(),
            issuer: "FBR".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att1.id, att1);
        state.attestations.insert(att2.id, att2);

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"attestation_type":"compliance_check"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        assert_eq!(result.count, 1);
        assert_eq!(result.results[0].attestation_type, "compliance_check");
    }

    #[tokio::test]
    async fn handler_query_attestations_combined_filters() {
        let state = AppState::new();

        for i in 0..5 {
            let att = AttestationRecord {
                id: uuid::Uuid::new_v4(),
                entity_id: uuid::Uuid::new_v4(),
                attestation_type: if i % 2 == 0 { "kyc" } else { "aml" }.to_string(),
                issuer: "NADRA".to_string(),
                status: if i < 3 { "ACTIVE" } else { "PENDING" }.to_string(),
                jurisdiction_id: if i < 2 { "PK-PSEZ" } else { "AE-DIFC" }.to_string(),
                issued_at: chrono::Utc::now(),
                expires_at: None,
                details: serde_json::json!({}),
            };
            state.attestations.insert(att.id, att);
        }

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/regulator/query/attestations")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_id":"PK-PSEZ","status":"ACTIVE"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let result: QueryResultsResponse = body_json(resp).await;
        // PK-PSEZ (indices 0,1) and ACTIVE (indices 0,1,2) → intersection = indices 0,1
        assert_eq!(result.count, 2);
    }

    #[tokio::test]
    async fn handler_compliance_summary_counts_assets_and_attestations() {
        let state = AppState::new();

        // Add a smart asset
        let asset = crate::state::SmartAssetRecord {
            id: uuid::Uuid::new_v4(),
            asset_type: "CapTable".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: "ACTIVE".to_string(),
            genesis_digest: None,
            compliance_status: Some("COMPLIANT".to_string()),
            metadata: serde_json::json!({}),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        state.smart_assets.insert(asset.id, asset);

        // Add an attestation
        let att = AttestationRecord {
            id: uuid::Uuid::new_v4(),
            entity_id: uuid::Uuid::new_v4(),
            attestation_type: "kyc".to_string(),
            issuer: "NADRA".to_string(),
            status: "ACTIVE".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            issued_at: chrono::Utc::now(),
            expires_at: None,
            details: serde_json::json!({}),
        };
        state.attestations.insert(att.id, att);

        let app = router().with_state(state);
        let req = Request::builder()
            .method("GET")
            .uri("/v1/regulator/summary")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let summary: ComplianceSummary = body_json(resp).await;
        assert_eq!(summary.total_assets, 1);
        assert_eq!(summary.total_attestations, 1);
    }

    #[test]
    fn compliance_summary_serialization() {
        let summary = ComplianceSummary {
            total_entities: 10,
            total_corridors: 3,
            total_assets: 25,
            total_attestations: 100,
        };
        let json = serde_json::to_string(&summary).unwrap();
        let deserialized: ComplianceSummary = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_entities, 10);
        assert_eq!(deserialized.total_corridors, 3);
        assert_eq!(deserialized.total_assets, 25);
        assert_eq!(deserialized.total_attestations, 100);
    }

    #[test]
    fn query_results_response_serialization() {
        let resp = QueryResultsResponse {
            count: 0,
            results: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deserialized: QueryResultsResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count, 0);
        assert!(deserialized.results.is_empty());
    }
}
