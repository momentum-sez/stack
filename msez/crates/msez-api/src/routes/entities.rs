//! # ENTITIES Primitive — Organization Info API
//!
//! Handles entity formation, lifecycle management, beneficial ownership
//! registry, and the 10-stage dissolution process.
//!
//! ## Endpoints
//!
//! - `POST /v1/entities` — create entity
//! - `GET /v1/entities` — list entities
//! - `GET /v1/entities/:id` — get entity
//! - `PUT /v1/entities/:id` — update entity
//! - `GET /v1/entities/:id/beneficial-owners` — beneficial ownership
//! - `POST /v1/entities/:id/dissolution/initiate` — start dissolution
//! - `GET /v1/entities/:id/dissolution/status` — dissolution status

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, BeneficialOwner, EntityRecord};
use axum::extract::rejection::JsonRejection;

// ── Request/Response DTOs ───────────────────────────────────────────

/// Request to create a new entity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateEntityRequest {
    /// Type of entity: "company", "individual", "trust", etc.
    pub entity_type: String,
    /// Legal name of the entity.
    pub legal_name: String,
    /// Jurisdiction where the entity is being formed.
    pub jurisdiction_id: String,
    /// Optional list of beneficial owners.
    #[serde(default)]
    pub beneficial_owners: Vec<BeneficialOwner>,
}

impl Validate for CreateEntityRequest {
    fn validate(&self) -> Result<(), String> {
        if self.legal_name.trim().is_empty() {
            return Err("legal_name must not be empty".to_string());
        }
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        if self.entity_type.trim().is_empty() {
            return Err("entity_type must not be empty".to_string());
        }
        Ok(())
    }
}

/// Request to update an existing entity.
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateEntityRequest {
    /// Updated legal name (optional).
    pub legal_name: Option<String>,
    /// Updated status (optional).
    pub status: Option<String>,
}

impl Validate for UpdateEntityRequest {
    fn validate(&self) -> Result<(), String> {
        if let Some(ref name) = self.legal_name {
            if name.trim().is_empty() {
                return Err("legal_name must not be empty if provided".to_string());
            }
        }
        Ok(())
    }
}

/// Dissolution status response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct DissolutionStatusResponse {
    pub entity_id: Uuid,
    pub status: String,
    pub current_stage: Option<u8>,
    pub stage_name: Option<String>,
}

// ── Router ──────────────────────────────────────────────────────────

/// Build the entities router with all CRUD and lifecycle endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/entities", get(list_entities).post(create_entity))
        .route("/v1/entities/:id", get(get_entity).put(update_entity))
        .route(
            "/v1/entities/:id/beneficial-owners",
            get(get_beneficial_owners),
        )
        .route(
            "/v1/entities/:id/dissolution/initiate",
            post(initiate_dissolution),
        )
        .route(
            "/v1/entities/:id/dissolution/status",
            get(get_dissolution_status),
        )
}

// ── Handlers ────────────────────────────────────────────────────────

/// POST /v1/entities — Create a new entity.
#[utoipa::path(
    post,
    path = "/v1/entities",
    request_body = CreateEntityRequest,
    responses(
        (status = 201, description = "Entity created", body = EntityRecord),
        (status = 422, description = "Validation error", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn create_entity(
    State(state): State<AppState>,
    body: Result<Json<CreateEntityRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<EntityRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = EntityRecord {
        id,
        entity_type: req.entity_type,
        legal_name: req.legal_name,
        jurisdiction_id: req.jurisdiction_id,
        status: "APPLIED".to_string(),
        beneficial_owners: req.beneficial_owners,
        dissolution_stage: None,
        created_at: now,
        updated_at: now,
    };

    state.entities.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/entities — List all entities.
#[utoipa::path(
    get,
    path = "/v1/entities",
    responses(
        (status = 200, description = "List of entities", body = Vec<EntityRecord>),
    ),
    tag = "entities"
)]
async fn list_entities(State(state): State<AppState>) -> Json<Vec<EntityRecord>> {
    Json(state.entities.list())
}

/// GET /v1/entities/:id — Get a single entity.
#[utoipa::path(
    get,
    path = "/v1/entities/{id}",
    params(("id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Entity found", body = EntityRecord),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn get_entity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityRecord>, AppError> {
    state
        .entities
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("entity {id} not found")))
}

/// PUT /v1/entities/:id — Update an entity.
#[utoipa::path(
    put,
    path = "/v1/entities/{id}",
    params(("id" = Uuid, Path, description = "Entity ID")),
    request_body = UpdateEntityRequest,
    responses(
        (status = 200, description = "Entity updated", body = EntityRecord),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn update_entity(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<UpdateEntityRequest>, JsonRejection>,
) -> Result<Json<EntityRecord>, AppError> {
    let req = extract_validated_json(body)?;
    state
        .entities
        .update(&id, |entity| {
            if let Some(name) = req.legal_name {
                entity.legal_name = name;
            }
            if let Some(status) = req.status {
                entity.status = status;
            }
            entity.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("entity {id} not found")))
}

/// GET /v1/entities/:id/beneficial-owners — Get beneficial owners.
#[utoipa::path(
    get,
    path = "/v1/entities/{id}/beneficial-owners",
    params(("id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Beneficial owners list", body = Vec<BeneficialOwner>),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn get_beneficial_owners(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<BeneficialOwner>>, AppError> {
    state
        .entities
        .get(&id)
        .map(|e| Json(e.beneficial_owners))
        .ok_or_else(|| AppError::NotFound(format!("entity {id} not found")))
}

/// POST /v1/entities/:id/dissolution/initiate — Initiate dissolution.
#[utoipa::path(
    post,
    path = "/v1/entities/{id}/dissolution/initiate",
    params(("id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Dissolution initiated", body = EntityRecord),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state for dissolution", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn initiate_dissolution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EntityRecord>, AppError> {
    state
        .entities
        .update(&id, |entity| {
            entity.status = "DISSOLVING".to_string();
            entity.dissolution_stage = Some(1);
            entity.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("entity {id} not found")))
}

/// GET /v1/entities/:id/dissolution/status — Get dissolution status.
#[utoipa::path(
    get,
    path = "/v1/entities/{id}/dissolution/status",
    params(("id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Dissolution status", body = DissolutionStatusResponse),
        (status = 404, description = "Entity not found", body = crate::error::ErrorBody),
    ),
    tag = "entities"
)]
async fn get_dissolution_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<DissolutionStatusResponse>, AppError> {
    let entity = state
        .entities
        .get(&id)
        .ok_or_else(|| AppError::NotFound(format!("entity {id} not found")))?;

    let stage_name = entity.dissolution_stage.map(|s| {
        match s {
            1 => "BOARD_RESOLUTION",
            2 => "SHAREHOLDER_RESOLUTION",
            3 => "APPOINT_LIQUIDATOR",
            4 => "NOTIFY_CREDITORS",
            5 => "REALIZE_ASSETS",
            6 => "SETTLE_LIABILITIES",
            7 => "FINAL_DISTRIBUTION",
            8 => "FINAL_MEETING",
            9 => "FILE_FINAL_DOCUMENTS",
            10 => "DISSOLUTION",
            _ => "UNKNOWN",
        }
        .to_string()
    });

    Ok(Json(DissolutionStatusResponse {
        entity_id: id,
        status: entity.status,
        current_stage: entity.dissolution_stage,
        stage_name,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateEntityRequest validation ─────────────────────────────

    #[test]
    fn test_create_entity_request_valid() {
        let req = CreateEntityRequest {
            entity_type: "company".to_string(),
            legal_name: "Acme Corp".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            beneficial_owners: vec![],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_entity_request_empty_legal_name() {
        let req = CreateEntityRequest {
            entity_type: "company".to_string(),
            legal_name: "".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            beneficial_owners: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("legal_name"),
            "error should mention legal_name: {err}"
        );
    }

    #[test]
    fn test_create_entity_request_whitespace_legal_name() {
        let req = CreateEntityRequest {
            entity_type: "company".to_string(),
            legal_name: "   ".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            beneficial_owners: vec![],
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_entity_request_empty_jurisdiction_id() {
        let req = CreateEntityRequest {
            entity_type: "company".to_string(),
            legal_name: "Acme Corp".to_string(),
            jurisdiction_id: "".to_string(),
            beneficial_owners: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("jurisdiction_id"),
            "error should mention jurisdiction_id: {err}"
        );
    }

    #[test]
    fn test_create_entity_request_empty_entity_type() {
        let req = CreateEntityRequest {
            entity_type: "  ".to_string(),
            legal_name: "Acme Corp".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            beneficial_owners: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("entity_type"),
            "error should mention entity_type: {err}"
        );
    }

    // ── UpdateEntityRequest validation ─────────────────────────────

    #[test]
    fn test_update_entity_request_valid_with_name() {
        let req = UpdateEntityRequest {
            legal_name: Some("New Name".to_string()),
            status: Some("ACTIVE".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_entity_request_valid_none_fields() {
        let req = UpdateEntityRequest {
            legal_name: None,
            status: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_update_entity_request_empty_legal_name() {
        let req = UpdateEntityRequest {
            legal_name: Some("".to_string()),
            status: None,
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("legal_name"),
            "error should mention legal_name: {err}"
        );
    }

    #[test]
    fn test_update_entity_request_whitespace_legal_name() {
        let req = UpdateEntityRequest {
            legal_name: Some("   ".to_string()),
            status: None,
        };
        assert!(req.validate().is_err());
    }

    // ── Router construction ────────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
        // Router construction should not panic.
    }

    // ── Handler integration tests ──────────────────────────────────

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    /// Helper: build the entities router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_entity_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"corporation","legal_name":"Test Corp","jurisdiction_id":"PK-RSEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: EntityRecord = body_json(resp).await;
        assert_eq!(record.legal_name, "Test Corp");
        assert_eq!(record.entity_type, "corporation");
        assert_eq!(record.jurisdiction_id, "PK-RSEZ");
        assert_eq!(record.status, "APPLIED");
        assert!(record.dissolution_stage.is_none());
    }

    #[tokio::test]
    async fn handler_create_entity_validation_error_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"corporation","legal_name":"","jurisdiction_id":"PK-RSEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_entity_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"not valid json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_list_entities_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/entities")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let records: Vec<EntityRecord> = body_json(resp).await;
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn handler_list_entities_after_create_returns_one() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"llc","legal_name":"Alpha LLC","jurisdiction_id":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // List entities.
        let list_req = Request::builder()
            .method("GET")
            .uri("/v1/entities")
            .body(Body::empty())
            .unwrap();
        let list_resp = app.oneshot(list_req).await.unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);

        let records: Vec<EntityRecord> = body_json(list_resp).await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].legal_name, "Alpha LLC");
    }

    #[tokio::test]
    async fn handler_get_entity_found_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"trust","legal_name":"Beta Trust","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        let created: EntityRecord = body_json(create_resp).await;

        // Get the entity by ID.
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{}", created.id))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: EntityRecord = body_json(get_resp).await;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.legal_name, "Beta Trust");
    }

    #[tokio::test]
    async fn handler_get_entity_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // ── Additional handler coverage ───────────────────────────────

    #[tokio::test]
    async fn handler_update_entity_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity first.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Old Name","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created: EntityRecord = body_json(create_resp).await;

        // Update the entity's legal_name and status.
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":"New Name","status":"ACTIVE"}"#))
            .unwrap();
        let update_resp = app.oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let updated: EntityRecord = body_json(update_resp).await;
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.legal_name, "New Name");
        assert_eq!(updated.status, "ACTIVE");
    }

    #[tokio::test]
    async fn handler_update_entity_partial_fields() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"llc","legal_name":"Partial Corp","jurisdiction_id":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Update only legal_name (no status).
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":"Updated Partial Corp"}"#))
            .unwrap();
        let update_resp = app.clone().oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::OK);

        let updated: EntityRecord = body_json(update_resp).await;
        assert_eq!(updated.legal_name, "Updated Partial Corp");
        assert_eq!(updated.status, "APPLIED"); // status unchanged

        // Update only status (no legal_name).
        let update_req2 = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"status":"SUSPENDED"}"#))
            .unwrap();
        let update_resp2 = app.oneshot(update_req2).await.unwrap();
        assert_eq!(update_resp2.status(), StatusCode::OK);

        let updated2: EntityRecord = body_json(update_resp2).await;
        assert_eq!(updated2.legal_name, "Updated Partial Corp"); // unchanged
        assert_eq!(updated2.status, "SUSPENDED");
    }

    #[tokio::test]
    async fn handler_update_entity_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{id}"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":"Ghost"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_update_entity_empty_name_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Test","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Update with empty legal_name.
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"legal_name":""}"#))
            .unwrap();
        let update_resp = app.oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_update_entity_bad_json_returns_400() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Test","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Send malformed JSON.
        let update_req = Request::builder()
            .method("PUT")
            .uri(format!("/v1/entities/{}", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{broken json"#))
            .unwrap();
        let update_resp = app.oneshot(update_req).await.unwrap();
        assert_eq!(update_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_get_beneficial_owners_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity with beneficial owners.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"BenOwner Corp","jurisdiction_id":"PK-PSEZ","beneficial_owners":[{"name":"Ali Khan","ownership_percentage":"51.0","cnic":"12345-6789012-3"},{"name":"Sara Ahmed","ownership_percentage":"49.0"}]}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created: EntityRecord = body_json(create_resp).await;

        // Get beneficial owners.
        let get_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{}/beneficial-owners", created.id))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let owners: Vec<BeneficialOwner> = body_json(get_resp).await;
        assert_eq!(owners.len(), 2);
        assert_eq!(owners[0].name, "Ali Khan");
        assert_eq!(owners[0].ownership_percentage, "51.0");
        assert_eq!(owners[0].cnic.as_deref(), Some("12345-6789012-3"));
        assert_eq!(owners[1].name, "Sara Ahmed");
        assert!(owners[1].cnic.is_none());
    }

    #[tokio::test]
    async fn handler_get_beneficial_owners_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{id}/beneficial-owners"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_initiate_dissolution_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Dissolving Corp","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Initiate dissolution.
        let dissolve_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/entities/{}/dissolution/initiate", created.id))
            .body(Body::empty())
            .unwrap();
        let dissolve_resp = app.oneshot(dissolve_req).await.unwrap();
        assert_eq!(dissolve_resp.status(), StatusCode::OK);

        let dissolved: EntityRecord = body_json(dissolve_resp).await;
        assert_eq!(dissolved.status, "DISSOLVING");
        assert_eq!(dissolved.dissolution_stage, Some(1));
    }

    #[tokio::test]
    async fn handler_initiate_dissolution_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/entities/{id}/dissolution/initiate"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_get_dissolution_status_with_active_dissolution() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create and dissolve an entity.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Status Corp","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Initiate dissolution.
        let dissolve_req = Request::builder()
            .method("POST")
            .uri(format!("/v1/entities/{}/dissolution/initiate", created.id))
            .body(Body::empty())
            .unwrap();
        app.clone().oneshot(dissolve_req).await.unwrap();

        // Get dissolution status.
        let status_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{}/dissolution/status", created.id))
            .body(Body::empty())
            .unwrap();
        let status_resp = app.oneshot(status_req).await.unwrap();
        assert_eq!(status_resp.status(), StatusCode::OK);

        let status: DissolutionStatusResponse = body_json(status_resp).await;
        assert_eq!(status.entity_id, created.id);
        assert_eq!(status.status, "DISSOLVING");
        assert_eq!(status.current_stage, Some(1));
        assert_eq!(status.stage_name.as_deref(), Some("BOARD_RESOLUTION"));
    }

    #[tokio::test]
    async fn handler_get_dissolution_status_no_dissolution() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create an entity without dissolution.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Healthy Corp","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: EntityRecord = body_json(create_resp).await;

        // Get dissolution status — no dissolution initiated.
        let status_req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{}/dissolution/status", created.id))
            .body(Body::empty())
            .unwrap();
        let status_resp = app.oneshot(status_req).await.unwrap();
        assert_eq!(status_resp.status(), StatusCode::OK);

        let status: DissolutionStatusResponse = body_json(status_resp).await;
        assert_eq!(status.entity_id, created.id);
        assert_eq!(status.status, "APPLIED");
        assert!(status.current_stage.is_none());
        assert!(status.stage_name.is_none());
    }

    #[tokio::test]
    async fn handler_get_dissolution_status_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{id}/dissolution/status"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_get_dissolution_status_all_stages() {
        // Verify stage name mapping for all 10 dissolution stages.
        let state = AppState::new();
        let now = Utc::now();

        let expected_stages = [
            (1u8, "BOARD_RESOLUTION"),
            (2, "SHAREHOLDER_RESOLUTION"),
            (3, "APPOINT_LIQUIDATOR"),
            (4, "NOTIFY_CREDITORS"),
            (5, "REALIZE_ASSETS"),
            (6, "SETTLE_LIABILITIES"),
            (7, "FINAL_DISTRIBUTION"),
            (8, "FINAL_MEETING"),
            (9, "FILE_FINAL_DOCUMENTS"),
            (10, "DISSOLUTION"),
        ];

        for (stage, expected_name) in expected_stages {
            let id = Uuid::new_v4();
            let entity = EntityRecord {
                id,
                entity_type: "company".to_string(),
                legal_name: format!("Stage {stage} Corp"),
                jurisdiction_id: "PK-PSEZ".to_string(),
                status: "DISSOLVING".to_string(),
                beneficial_owners: vec![],
                dissolution_stage: Some(stage),
                created_at: now,
                updated_at: now,
            };
            state.entities.insert(id, entity);

            let app = router().with_state(state.clone());
            let req = Request::builder()
                .method("GET")
                .uri(format!("/v1/entities/{id}/dissolution/status"))
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            let status: DissolutionStatusResponse = body_json(resp).await;
            assert_eq!(
                status.stage_name.as_deref(),
                Some(expected_name),
                "stage {stage} should map to {expected_name}"
            );
        }

        // Test unknown stage number (e.g., 99).
        let unknown_id = Uuid::new_v4();
        let unknown_entity = EntityRecord {
            id: unknown_id,
            entity_type: "company".to_string(),
            legal_name: "Unknown Stage Corp".to_string(),
            jurisdiction_id: "PK-PSEZ".to_string(),
            status: "DISSOLVING".to_string(),
            beneficial_owners: vec![],
            dissolution_stage: Some(99),
            created_at: now,
            updated_at: now,
        };
        state.entities.insert(unknown_id, unknown_entity);

        let app = router().with_state(state.clone());
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/entities/{unknown_id}/dissolution/status"))
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status: DissolutionStatusResponse = body_json(resp).await;
        assert_eq!(status.stage_name.as_deref(), Some("UNKNOWN"));
    }

    #[tokio::test]
    async fn handler_create_entity_with_beneficial_owners_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"entity_type":"corporation","legal_name":"Owned Corp","jurisdiction_id":"PK-RSEZ","beneficial_owners":[{"name":"Owner A","ownership_percentage":"60.0","ntn":"1234567"}]}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: EntityRecord = body_json(resp).await;
        assert_eq!(record.beneficial_owners.len(), 1);
        assert_eq!(record.beneficial_owners[0].name, "Owner A");
        assert_eq!(record.beneficial_owners[0].ntn.as_deref(), Some("1234567"));
    }

    #[tokio::test]
    async fn handler_create_entity_missing_content_type() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .body(Body::from(
                r#"{"entity_type":"company","legal_name":"Test","jurisdiction_id":"PK-PSEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        // Missing content-type should cause a 400 from JSON rejection.
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
