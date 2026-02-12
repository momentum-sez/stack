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
#[derive(Debug, Serialize, ToSchema)]
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
