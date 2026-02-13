//! # Mass API Proxy Routes
//!
//! Thin proxy layer that forwards primitive CRUD operations to the live
//! Mass APIs via `msez-mass-client`. These endpoints exist so that SEZ Stack
//! consumers have a single API surface rather than calling Mass directly.
//!
//! The proxy layer adds SEZ Stack value by:
//! 1. Evaluating compliance tensor BEFORE forwarding to Mass
//! 2. Issuing VCs for significant state changes
//! 3. Recording corridor-relevant events
//! 4. Enforcing zone-level policies via the agentic engine
//!
//! In the future, orchestration endpoints (e.g., "form entity + create
//! account + issue KYC VC" in a single call) will live here.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

/// Build the Mass API proxy router.
///
/// Provides primitive endpoints that delegate to the live Mass APIs.
pub fn router() -> Router<AppState> {
    Router::new()
        // Entity proxy (delegates to Mass organization-info)
        .route("/v1/entities", get(list_entities).post(create_entity))
        .route("/v1/entities/:id", get(get_entity))
}

// -- Request/Response DTOs for the proxy layer --------------------------------

/// Request to create an entity via the Mass API proxy.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEntityProxyRequest {
    pub entity_type: String,
    pub legal_name: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub beneficial_owners: Vec<BeneficialOwnerInput>,
}

/// Beneficial owner input.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BeneficialOwnerInput {
    pub name: String,
    pub ownership_percentage: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cnic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntn: Option<String>,
}

// -- Handlers -----------------------------------------------------------------

/// POST /v1/entities — Create an entity via Mass organization-info API.
#[utoipa::path(
    post,
    path = "/v1/entities",
    request_body = CreateEntityProxyRequest,
    responses(
        (status = 201, description = "Entity created in Mass"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn create_entity(
    State(state): State<AppState>,
    Json(req): Json<CreateEntityProxyRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // TODO Phase 2: Compliance tensor pre-check before forwarding to Mass.
    // TODO Phase 2: Agentic policy evaluation.

    let client = state
        .mass_client
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable(
            "Mass API client not configured. Set MASS_API_TOKEN environment variable.",
        ))?;

    let mass_req = msez_mass_client::entities::CreateEntityRequest {
        entity_type: req.entity_type,
        legal_name: req.legal_name,
        jurisdiction_id: req.jurisdiction_id,
        beneficial_owners: req
            .beneficial_owners
            .into_iter()
            .map(|bo| msez_mass_client::entities::MassBeneficialOwner {
                name: bo.name,
                ownership_percentage: bo.ownership_percentage,
                cnic: bo.cnic,
                ntn: bo.ntn,
            })
            .collect(),
    };

    let entity = client
        .entities()
        .create(&mass_req)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    // TODO Phase 2: Issue VC for entity creation, record in audit trail.
    serde_json::to_value(entity)
        .map(Json)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))
}

/// GET /v1/entities/{id} — Get an entity from Mass by ID.
#[utoipa::path(
    get,
    path = "/v1/entities/:id",
    params(("id" = uuid::Uuid, Path, description = "Entity UUID")),
    responses(
        (status = 200, description = "Entity found"),
        (status = 404, description = "Entity not found"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn get_entity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = state
        .mass_client
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("Mass API client not configured."))?;

    match client.entities().get(id).await {
        Ok(Some(entity)) => serde_json::to_value(entity)
            .map(Json)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}"))),
        Ok(None) => Err(AppError::not_found(format!("entity {id} not found"))),
        Err(e) => Err(AppError::upstream(format!("Mass API error: {e}"))),
    }
}

/// GET /v1/entities — List entities from Mass.
#[utoipa::path(
    get,
    path = "/v1/entities",
    responses(
        (status = 200, description = "List of entities"),
        (status = 502, description = "Mass API error"),
        (status = 503, description = "Mass client not configured"),
    ),
    tag = "entities"
)]
async fn list_entities(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let client = state
        .mass_client
        .as_ref()
        .ok_or_else(|| AppError::service_unavailable("Mass API client not configured."))?;

    let entities = client
        .entities()
        .list(None, None)
        .await
        .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;

    serde_json::to_value(entities)
        .map(Json)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }

    #[test]
    fn create_entity_proxy_request_deserializes() {
        let json = r#"{
            "entity_type": "llc",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01"
        }"#;
        let req: CreateEntityProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.entity_type, "llc");
        assert_eq!(req.legal_name, "Test Corp");
        assert!(req.beneficial_owners.is_empty());
    }

    #[test]
    fn create_entity_proxy_request_with_beneficial_owners() {
        let json = r#"{
            "entity_type": "llc",
            "legal_name": "Test Corp",
            "jurisdiction_id": "pk-sez-01",
            "beneficial_owners": [{
                "name": "Alice",
                "ownership_percentage": "51.0",
                "cnic": "12345-1234567-1"
            }]
        }"#;
        let req: CreateEntityProxyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.beneficial_owners.len(), 1);
        assert_eq!(req.beneficial_owners[0].name, "Alice");
    }

    #[tokio::test]
    async fn create_entity_returns_503_without_mass_client() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use http_body_util::BodyExt;
        use tower::ServiceExt;

        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"llc","legal_name":"Test","jurisdiction_id":"pk-sez-01"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], "SERVICE_UNAVAILABLE");
    }

    #[tokio::test]
    async fn get_entity_returns_503_without_mass_client() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/entities/550e8400-e29b-41d4-a716-446655440000")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn list_entities_returns_503_without_mass_client() {
        use axum::body::Body;
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let app = router().with_state(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/v1/entities")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
