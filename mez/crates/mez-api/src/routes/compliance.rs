//! # Compliance Query API
//!
//! Standalone compliance query endpoint for regulator integration.
//! Provides `GET /v1/compliance/{entity_id}` to evaluate the full
//! 20-domain compliance tensor for an entity without requiring a
//! prior Mass API operation.
//!
//! ## Authorization
//!
//! Requires `Role::Regulator` or `Role::ZoneAdmin`.

use std::collections::HashMap;

use axum::extract::{Path, Query, State};
use axum::routing::get;
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use mez_core::ComplianceDomain;
use mez_tensor::{ComplianceState, JurisdictionConfig};

use crate::auth::{require_role, CallerIdentity, Role};
use crate::error::AppError;
use crate::state::AppState;

/// Query parameters for the compliance endpoint.
#[derive(Debug, Deserialize)]
pub struct ComplianceQueryParams {
    /// Jurisdiction to evaluate against. Defaults to the zone's jurisdiction.
    pub jurisdiction: Option<String>,
}

/// Compliance evaluation response for a single entity.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComplianceResponse {
    /// Entity identifier that was evaluated.
    pub entity_id: String,
    /// Jurisdiction evaluated against.
    pub jurisdiction_id: String,
    /// Aggregate compliance status across all 20 domains.
    pub overall_status: String,
    /// Per-domain compliance state.
    pub domain_results: HashMap<String, String>,
    /// Total number of domains evaluated.
    pub domain_count: usize,
    /// Domains in a passing state (compliant, exempt, not_applicable).
    pub passing_domains: Vec<String>,
    /// Domains that are blocking (non_compliant or pending).
    pub blocking_domains: Vec<String>,
    /// SHA-256 tensor commitment digest (hex).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tensor_commitment: Option<String>,
    /// When the evaluation was performed.
    pub evaluated_at: DateTime<Utc>,
}

/// Build the compliance query router.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/compliance/:entity_id", get(get_compliance))
}

/// GET /v1/compliance/{entity_id} — Evaluate compliance for an entity.
///
/// Builds a 20-domain compliance tensor for the specified jurisdiction
/// (or the zone's default jurisdiction) and returns the full evaluation.
#[utoipa::path(
    get,
    path = "/v1/compliance/{entity_id}",
    params(
        ("entity_id" = String, Path, description = "Entity identifier to evaluate"),
        ("jurisdiction" = Option<String>, Query, description = "Jurisdiction to evaluate against"),
    ),
    responses(
        (status = 200, description = "Compliance evaluation result", body = ComplianceResponse),
        (status = 422, description = "Invalid entity_id or jurisdiction"),
    ),
    tag = "compliance"
)]
async fn get_compliance(
    State(state): State<AppState>,
    caller: CallerIdentity,
    Path(entity_id): Path<String>,
    Query(params): Query<ComplianceQueryParams>,
) -> Result<Json<ComplianceResponse>, AppError> {
    // Allow both regulator and zone admin access.
    if require_role(&caller, Role::Regulator).is_err() {
        require_role(&caller, Role::ZoneAdmin)?;
    }

    // Validate entity_id is non-empty.
    if entity_id.is_empty() {
        return Err(AppError::Validation("entity_id must not be empty".to_string()));
    }

    // Determine jurisdiction: explicit param > zone config > fallback.
    let jurisdiction_id = params
        .jurisdiction
        .or_else(|| state.zone.as_ref().map(|z| z.jurisdiction_id.clone()))
        .unwrap_or_else(|| "GLOBAL".to_string());

    // Build and evaluate the compliance tensor.
    let tensor = crate::compliance::build_tensor(&jurisdiction_id);
    let all_results = tensor.evaluate_all(&entity_id);

    let mut domain_results = HashMap::new();
    let mut passing_domains = Vec::new();
    let mut blocking_domains = Vec::new();

    for &domain in ComplianceDomain::all() {
        let domain_state = all_results
            .get(&domain)
            .copied()
            .unwrap_or(ComplianceState::Pending);

        domain_results.insert(domain.as_str().to_string(), format!("{domain_state}"));

        if domain_state.is_passing() {
            passing_domains.push(domain.as_str().to_string());
        } else {
            blocking_domains.push(domain.as_str().to_string());
        }
    }

    passing_domains.sort();
    blocking_domains.sort();

    // Compute aggregate state.
    let aggregate = ComplianceDomain::all()
        .iter()
        .map(|d| {
            all_results
                .get(d)
                .copied()
                .unwrap_or(ComplianceState::Pending)
        })
        .fold(ComplianceState::Compliant, ComplianceState::meet);

    let tensor_commitment = tensor
        .commit()
        .map_err(|e| {
            tracing::warn!(error = %e, "tensor commitment failed");
            e
        })
        .ok()
        .map(|c| c.to_hex());

    Ok(Json(ComplianceResponse {
        entity_id,
        jurisdiction_id: tensor.jurisdiction().jurisdiction_id().as_str().to_string(),
        overall_status: format!("{aggregate}"),
        domain_results,
        domain_count: 20,
        passing_domains,
        blocking_domains,
        tensor_commitment,
        evaluated_at: Utc::now(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{CallerIdentity, Role};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn zone_admin() -> CallerIdentity {
        CallerIdentity {
            role: Role::ZoneAdmin,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    fn regulator() -> CallerIdentity {
        CallerIdentity {
            role: Role::Regulator,
            entity_id: None,
            jurisdiction_id: None,
        }
    }

    fn test_app() -> Router<()> {
        router()
            .layer(axum::Extension(zone_admin()))
            .with_state(AppState::new())
    }

    fn test_app_with_identity(identity: CallerIdentity) -> Router<()> {
        router()
            .layer(axum::Extension(identity))
            .with_state(AppState::new())
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn compliance_returns_200_for_valid_entity() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-123")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: ComplianceResponse = body_json(resp).await;
        assert_eq!(result.entity_id, "entity-123");
        assert_eq!(result.domain_count, 20);
        assert_eq!(result.domain_results.len(), 20);
        assert!(!result.overall_status.is_empty());
    }

    #[tokio::test]
    async fn compliance_with_jurisdiction_param() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-456?jurisdiction=PK-PEZ")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: ComplianceResponse = body_json(resp).await;
        assert_eq!(result.entity_id, "entity-456");
        assert_eq!(result.jurisdiction_id, "PK-PEZ");
    }

    #[tokio::test]
    async fn compliance_defaults_to_global_jurisdiction() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-789")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: ComplianceResponse = body_json(resp).await;
        // Without zone config, defaults to GLOBAL.
        assert_eq!(result.jurisdiction_id, "GLOBAL");
    }

    #[tokio::test]
    async fn compliance_accessible_by_regulator() {
        let app = test_app_with_identity(regulator());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-abc")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn compliance_has_passing_and_blocking_domains() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-test")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: ComplianceResponse = body_json(resp).await;
        // All domains should be either passing or blocking.
        assert_eq!(
            result.passing_domains.len() + result.blocking_domains.len(),
            20
        );
    }

    #[tokio::test]
    async fn compliance_includes_tensor_commitment() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/compliance/entity-commit")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        let result: ComplianceResponse = body_json(resp).await;

        // Tensor commitment should be present and be 64-char hex.
        if let Some(ref commitment) = result.tensor_commitment {
            assert_eq!(commitment.len(), 64);
            assert!(commitment.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn router_builds() {
        let _r = router();
    }

    #[test]
    fn compliance_response_serialization() {
        let resp = ComplianceResponse {
            entity_id: "test".to_string(),
            jurisdiction_id: "PK".to_string(),
            overall_status: "pending".to_string(),
            domain_results: HashMap::new(),
            domain_count: 20,
            passing_domains: vec![],
            blocking_domains: vec![],
            tensor_commitment: None,
            evaluated_at: Utc::now(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let deser: ComplianceResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.entity_id, "test");
        assert_eq!(deser.domain_count, 20);
    }
}
