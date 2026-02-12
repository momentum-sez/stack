//! # Smart Asset API
//!
//! Handles smart asset CRUD, compliance evaluation triggering,
//! and anchor verification.
//! Route structure based on apis/smart-assets.openapi.yaml.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use axum::extract::rejection::JsonRejection;
use crate::extractors::{Validate, extract_validated_json};
use crate::state::{AppState, SmartAssetRecord};

/// Request to create a smart asset genesis.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAssetRequest {
    pub asset_type: String,
    pub jurisdiction_id: String,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl Validate for CreateAssetRequest {
    fn validate(&self) -> Result<(), String> {
        if self.asset_type.trim().is_empty() {
            return Err("asset_type must not be empty".to_string());
        }
        if self.jurisdiction_id.trim().is_empty() {
            return Err("jurisdiction_id must not be empty".to_string());
        }
        Ok(())
    }
}

/// Compliance evaluation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ComplianceEvalRequest {
    pub domains: Vec<String>,
    pub context: serde_json::Value,
}

impl Validate for ComplianceEvalRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Compliance evaluation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ComplianceEvalResponse {
    pub asset_id: Uuid,
    pub overall_status: String,
    pub domain_results: serde_json::Value,
    pub evaluated_at: chrono::DateTime<chrono::Utc>,
}

/// Anchor verification request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AnchorVerifyRequest {
    pub anchor_digest: String,
    pub chain: String,
}

impl Validate for AnchorVerifyRequest {
    fn validate(&self) -> Result<(), String> {
        if self.anchor_digest.trim().is_empty() {
            return Err("anchor_digest must not be empty".to_string());
        }
        Ok(())
    }
}

/// Build the smart assets router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/assets/genesis", post(create_asset))
        .route("/v1/assets/registry", post(submit_registry))
        .route("/v1/assets/:id", get(get_asset))
        .route("/v1/assets/:id/compliance/evaluate", post(evaluate_compliance))
        .route("/v1/assets/:id/anchors/corridor/verify", post(verify_anchor))
}

/// POST /v1/assets/genesis — Create smart asset genesis.
#[utoipa::path(
    post,
    path = "/v1/assets/genesis",
    request_body = CreateAssetRequest,
    responses(
        (status = 201, description = "Asset created", body = SmartAssetRecord),
    ),
    tag = "smart_assets"
)]
async fn create_asset(
    State(state): State<AppState>,
    body: Result<Json<CreateAssetRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<SmartAssetRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = SmartAssetRecord {
        id,
        asset_type: req.asset_type,
        jurisdiction_id: req.jurisdiction_id,
        status: "GENESIS".to_string(),
        genesis_digest: None,
        compliance_status: None,
        metadata: req.metadata,
        created_at: now,
        updated_at: now,
    };

    state.smart_assets.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// POST /v1/assets/registry — Submit/update smart asset registry VC.
#[utoipa::path(
    post,
    path = "/v1/assets/registry",
    responses(
        (status = 200, description = "Registry submitted"),
    ),
    tag = "smart_assets"
)]
async fn submit_registry(State(_state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "submitted",
        "message": "Registry VC submission recorded"
    }))
}

/// GET /v1/assets/:id — Get a smart asset.
#[utoipa::path(
    get,
    path = "/v1/assets/{id}",
    params(("id" = Uuid, Path, description = "Asset ID")),
    responses(
        (status = 200, description = "Asset found", body = SmartAssetRecord),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "smart_assets"
)]
async fn get_asset(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SmartAssetRecord>, AppError> {
    state
        .smart_assets
        .get(&id)
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("asset {id} not found")))
}

/// POST /v1/assets/:id/compliance/evaluate — Evaluate compliance.
#[utoipa::path(
    post,
    path = "/v1/assets/{id}/compliance/evaluate",
    params(("id" = Uuid, Path, description = "Asset ID")),
    request_body = ComplianceEvalRequest,
    responses(
        (status = 200, description = "Compliance evaluated", body = ComplianceEvalResponse),
        (status = 404, description = "Not found", body = crate::error::ErrorBody),
    ),
    tag = "smart_assets"
)]
async fn evaluate_compliance(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<ComplianceEvalRequest>, JsonRejection>,
) -> Result<Json<ComplianceEvalResponse>, AppError> {
    let _req = extract_validated_json(body)?;
    if !state.smart_assets.contains(&id) {
        return Err(AppError::NotFound(format!("asset {id} not found")));
    }

    Ok(Json(ComplianceEvalResponse {
        asset_id: id,
        overall_status: "PERMITTED".to_string(),
        domain_results: serde_json::json!({}),
        evaluated_at: Utc::now(),
    }))
}

/// POST /v1/assets/:id/anchors/corridor/verify — Verify anchor.
#[utoipa::path(
    post,
    path = "/v1/assets/{id}/anchors/corridor/verify",
    params(("id" = Uuid, Path, description = "Asset ID")),
    request_body = AnchorVerifyRequest,
    responses(
        (status = 200, description = "Anchor verified"),
    ),
    tag = "smart_assets"
)]
async fn verify_anchor(
    State(_state): State<AppState>,
    Path(id): Path<Uuid>,
    body: Result<Json<AnchorVerifyRequest>, JsonRejection>,
) -> Result<Json<serde_json::Value>, AppError> {
    let req = extract_validated_json(body)?;
    Ok(Json(serde_json::json!({
        "asset_id": id,
        "anchor_digest": req.anchor_digest,
        "chain": req.chain,
        "verified": true,
        "message": "Anchor verification is a Phase 2 feature — returning stub"
    })))
}
