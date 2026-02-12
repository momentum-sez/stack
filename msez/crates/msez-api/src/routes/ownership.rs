//! # OWNERSHIP Primitive — Investment Info API
//!
//! Handles cap table management, ownership transfers, share class
//! definitions, and capital gains event tracking.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, CapTableRecord, OwnershipTransfer, ShareClass};
use axum::extract::rejection::JsonRejection;

/// Request to create/initialize a cap table.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCapTableRequest {
    pub entity_id: Uuid,
    #[serde(default)]
    pub share_classes: Vec<ShareClass>,
}

impl Validate for CreateCapTableRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Request to record an ownership transfer.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RecordTransferRequest {
    pub from_holder: String,
    pub to_holder: String,
    pub share_class: String,
    pub quantity: u64,
    pub price_per_share: Option<String>,
}

impl Validate for RecordTransferRequest {
    fn validate(&self) -> Result<(), String> {
        if self.from_holder.trim().is_empty() || self.to_holder.trim().is_empty() {
            return Err("from_holder and to_holder must not be empty".to_string());
        }
        if self.quantity == 0 {
            return Err("quantity must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Build the ownership router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/ownership/cap-table", post(create_cap_table))
        .route("/v1/ownership/:entity_id/cap-table", get(get_cap_table))
        .route("/v1/ownership/:entity_id/transfers", post(record_transfer))
        .route(
            "/v1/ownership/:entity_id/share-classes",
            get(get_share_classes),
        )
}

/// POST /v1/ownership/cap-table — Initialize a cap table.
#[utoipa::path(
    post,
    path = "/v1/ownership/cap-table",
    request_body = CreateCapTableRequest,
    responses(
        (status = 201, description = "Cap table created", body = CapTableRecord),
    ),
    tag = "ownership"
)]
async fn create_cap_table(
    State(state): State<AppState>,
    body: Result<Json<CreateCapTableRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<CapTableRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = CapTableRecord {
        id,
        entity_id: req.entity_id,
        share_classes: req.share_classes,
        transfers: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    state.cap_tables.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/ownership/:entity_id/cap-table — Get cap table for an entity.
#[utoipa::path(
    get,
    path = "/v1/ownership/{entity_id}/cap-table",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Cap table found", body = CapTableRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn get_cap_table(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<CapTableRecord>, AppError> {
    state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))
}

/// POST /v1/ownership/:entity_id/transfers — Record an ownership transfer.
#[utoipa::path(
    post,
    path = "/v1/ownership/{entity_id}/transfers",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    request_body = RecordTransferRequest,
    responses(
        (status = 200, description = "Transfer recorded", body = CapTableRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn record_transfer(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
    body: Result<Json<RecordTransferRequest>, JsonRejection>,
) -> Result<Json<CapTableRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let cap_table = state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))?;

    let transfer = OwnershipTransfer {
        id: Uuid::new_v4(),
        from_holder: req.from_holder,
        to_holder: req.to_holder,
        share_class: req.share_class,
        quantity: req.quantity,
        price_per_share: req.price_per_share,
        transferred_at: Utc::now(),
    };

    state
        .cap_tables
        .update(&cap_table.id, |ct| {
            ct.transfers.push(transfer);
            ct.updated_at = Utc::now();
        })
        .map(Json)
        .ok_or_else(|| AppError::Internal("failed to update cap table".to_string()))
}

/// GET /v1/ownership/:entity_id/share-classes — Get share classes.
#[utoipa::path(
    get,
    path = "/v1/ownership/{entity_id}/share-classes",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Share classes", body = Vec<ShareClass>),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "ownership"
)]
async fn get_share_classes(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Result<Json<Vec<ShareClass>>, AppError> {
    state
        .cap_tables
        .list()
        .into_iter()
        .find(|ct| ct.entity_id == entity_id)
        .map(|ct| Json(ct.share_classes))
        .ok_or_else(|| AppError::NotFound(format!("cap table for entity {entity_id} not found")))
}
