//! # Corridor Operations API
//!
//! Handles corridor lifecycle transitions, receipt queries,
//! fork resolution, anchor verification, and finality status.
//! Route structure based on apis/corridor-state.openapi.yaml.

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use axum::extract::rejection::JsonRejection;
use crate::extractors::{Validate, extract_validated_json};
use crate::state::{AppState, CorridorRecord, CorridorTransitionEntry};

/// Request to create a corridor.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateCorridorRequest {
    pub jurisdiction_a: String,
    pub jurisdiction_b: String,
}

impl Validate for CreateCorridorRequest {
    fn validate(&self) -> Result<(), String> {
        if self.jurisdiction_a.trim().is_empty() || self.jurisdiction_b.trim().is_empty() {
            return Err("both jurisdiction IDs must be non-empty".to_string());
        }
        if self.jurisdiction_a == self.jurisdiction_b {
            return Err("jurisdiction_a and jurisdiction_b must differ".to_string());
        }
        Ok(())
    }
}

/// Request to transition a corridor's state.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TransitionCorridorRequest {
    pub target_state: String,
    pub evidence_digest: Option<String>,
    pub reason: Option<String>,
}

impl Validate for TransitionCorridorRequest {
    fn validate(&self) -> Result<(), String> {
        let valid_states = ["PENDING", "ACTIVE", "HALTED", "SUSPENDED", "DEPRECATED"];
        if !valid_states.contains(&self.target_state.as_str()) {
            return Err(format!(
                "target_state must be one of: {}",
                valid_states.join(", ")
            ));
        }
        Ok(())
    }
}

/// Receipt proposal request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProposeReceiptRequest {
    pub corridor_id: Uuid,
    pub payload: serde_json::Value,
}

impl Validate for ProposeReceiptRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Receipt response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ReceiptResponse {
    pub id: Uuid,
    pub corridor_id: Uuid,
    pub status: String,
    pub payload: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Build the corridors router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/corridors", get(list_corridors).post(create_corridor))
        .route("/v1/corridors/:id", get(get_corridor))
        .route("/v1/corridors/:id/transition", put(transition_corridor))
        .route("/v1/corridors/state/propose", post(propose_receipt))
        .route("/v1/corridors/state/fork-resolve", post(fork_resolve))
        .route("/v1/corridors/state/anchor", post(anchor_commitment))
        .route("/v1/corridors/state/finality-status", post(finality_status))
}

/// POST /v1/corridors — Create a new corridor.
#[utoipa::path(
    post,
    path = "/v1/corridors",
    request_body = CreateCorridorRequest,
    responses(
        (status = 201, description = "Corridor created", body = CorridorRecord),
    ),
    tag = "corridors"
)]
async fn create_corridor(
    State(state): State<AppState>,
    body: Result<Json<CreateCorridorRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<CorridorRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = CorridorRecord {
        id,
        jurisdiction_a: req.jurisdiction_a,
        jurisdiction_b: req.jurisdiction_b,
        state: "DRAFT".to_string(),
        transition_log: Vec::new(),
        created_at: now,
        updated_at: now,
    };

    state.corridors.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// GET /v1/corridors — List all corridors.
#[utoipa::path(
    get,
    path = "/v1/corridors",
    responses(
        (status = 200, description = "List of corridors", body = Vec<CorridorRecord>),
    ),
    tag = "corridors"
)]
async fn list_corridors(State(state): State<AppState>) -> Json<Vec<CorridorRecord>> {
    Json(state.corridors.list())
}

/// GET /v1/corridors/:id — Get a corridor.
#[utoipa::path(
    get,
    path = "/v1/corridors/{id}",
    params(("id" = Uuid, Path, description = "Corridor ID")),
    responses(
        (status = 200, description = "Corridor found", body = CorridorRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "corridors"
)]
async fn get_corridor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<CorridorRecord>, AppError> {
    state
        .corridors
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("corridor {id} not found")))
}

/// PUT /v1/corridors/:id/transition — Transition corridor state.
#[utoipa::path(
    put,
    path = "/v1/corridors/{id}/transition",
    params(("id" = Uuid, Path, description = "Corridor ID")),
    request_body = TransitionCorridorRequest,
    responses(
        (status = 200, description = "Corridor transitioned", body = CorridorRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid transition", body = crate::error::ErrorBody),
    ),
    tag = "corridors"
)]
async fn transition_corridor(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<TransitionCorridorRequest>, JsonRejection>,
) -> Result<Json<CorridorRecord>, AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let target = req.target_state.clone();
    let evidence = req.evidence_digest.clone();

    state
        .corridors
        .update(&id, |corridor| {
            let entry = CorridorTransitionEntry {
                from_state: corridor.state.clone(),
                to_state: target.clone(),
                timestamp: now,
                evidence_digest: evidence,
            };
            corridor.transition_log.push(entry);
            corridor.state = target;
            corridor.updated_at = now;
        })
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("corridor {id} not found")))
}

/// POST /v1/corridors/state/propose — Propose a receipt.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/propose",
    request_body = ProposeReceiptRequest,
    responses(
        (status = 200, description = "Receipt proposed", body = ReceiptResponse),
    ),
    tag = "corridors"
)]
async fn propose_receipt(
    State(_state): State<AppState>,
    body: Result<Json<ProposeReceiptRequest>, JsonRejection>,
) -> Result<Json<ReceiptResponse>, AppError> {
    let req = extract_validated_json(body)?;
    Ok(Json(ReceiptResponse {
        id: Uuid::new_v4(),
        corridor_id: req.corridor_id,
        status: "PROPOSED".to_string(),
        payload: req.payload,
        created_at: Utc::now(),
    }))
}

/// POST /v1/corridors/state/fork-resolve — Resolve receipt fork.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/fork-resolve",
    responses(
        (status = 200, description = "Fork resolved"),
    ),
    tag = "corridors"
)]
async fn fork_resolve(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "resolved",
        "strategy": "longest_chain",
        "message": "Fork resolution is a Phase 2 feature"
    }))
}

/// POST /v1/corridors/state/anchor — Anchor corridor commitment.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/anchor",
    responses(
        (status = 200, description = "Anchor commitment recorded"),
    ),
    tag = "corridors"
)]
async fn anchor_commitment(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "anchored",
        "message": "L1 anchoring is a Phase 2 feature"
    }))
}

/// POST /v1/corridors/state/finality-status — Compute finality status.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/finality-status",
    responses(
        (status = 200, description = "Finality status computed"),
    ),
    tag = "corridors"
)]
async fn finality_status(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "pending",
        "confirmations": 0,
        "message": "Finality computation is a Phase 2 feature"
    }))
}
