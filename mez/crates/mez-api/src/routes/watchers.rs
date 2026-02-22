// SPDX-License-Identifier: BUSL-1.1
//! # Watcher Economy API
//!
//! REST endpoints for managing watcher nodes in the corridor economy.
//! Watchers post bonds, observe corridor activity, produce attestations,
//! and can be slashed for protocol violations.
//!
//! ## Endpoints
//!
//! - `POST   /v1/watchers`                  — Register a new watcher
//! - `GET    /v1/watchers`                  — List all watchers
//! - `GET    /v1/watchers/:watcher_id`      — Get watcher details
//! - `POST   /v1/watchers/:watcher_id/bond` — Post a bond (stake collateral)
//! - `POST   /v1/watchers/:watcher_id/activate` — Activate for corridor monitoring
//! - `POST   /v1/watchers/:watcher_id/slash`    — Slash for a protocol violation
//! - `POST   /v1/watchers/:watcher_id/rebond`   — Re-bond after being slashed
//! - `POST   /v1/watchers/:watcher_id/unbond`   — Begin voluntary unbonding
//! - `POST   /v1/watchers/:watcher_id/complete-unbond` — Complete unbonding (terminal)
//! - `POST   /v1/watchers/:watcher_id/attest`   — Record a successful attestation
//!
//! These endpoints fulfill Roadmap Priority 4 (Corridor Network Scale) —
//! real bonding/slashing/reward implementation for watcher economy.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use mez_core::WatcherId;
use mez_state::watcher::{SlashingCondition, Watcher};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Response representing a watcher's current state.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WatcherResponse {
    /// Unique watcher identifier.
    pub id: Uuid,
    /// Current lifecycle state.
    pub state: String,
    /// Total bonded stake (in smallest currency unit).
    pub bonded_stake: u64,
    /// Amount slashed from the bond.
    pub slashed_amount: u64,
    /// Available (unslashed) stake.
    pub available_stake: u64,
    /// Number of slashing incidents.
    pub slash_count: u32,
    /// Number of successful attestations.
    pub attestation_count: u64,
    /// Whether the watcher is in a terminal state.
    pub is_terminal: bool,
    /// When the watcher was registered.
    pub registered_at: DateTime<Utc>,
    /// Last state change timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Request to post a bond for a watcher.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BondRequest {
    /// Amount of stake to bond (in smallest currency unit). Must be > 0.
    pub stake: u64,
}

/// Request to slash a watcher for a protocol violation.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlashRequest {
    /// The slashing condition. One of: "equivocation", "availability_failure",
    /// "false_attestation", "collusion".
    pub condition: String,
    /// Evidence hash (SHA-256 hex digest) supporting the slashing decision.
    #[serde(default)]
    pub evidence_digest: Option<String>,
}

/// Request to re-bond after being slashed.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RebondRequest {
    /// Additional stake to add (in smallest currency unit). Must be > 0.
    pub additional_stake: u64,
}

/// Response after a slashing event.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SlashResponse {
    /// The watcher that was slashed.
    pub watcher: WatcherResponse,
    /// Amount actually slashed.
    pub amount_slashed: u64,
    /// The slashing condition applied.
    pub condition: String,
    /// Slash percentage applied (0.0 to 1.0).
    pub slash_percentage: f64,
}

/// Response after unbonding completion.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnbondResponse {
    /// The watcher that was unbonded.
    pub watcher: WatcherResponse,
    /// Amount of stake returned.
    pub stake_returned: u64,
}

/// Watcher list response with pagination info.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct WatcherListResponse {
    /// List of watchers.
    pub watchers: Vec<WatcherResponse>,
    /// Total number of watchers.
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Internal record type
// ---------------------------------------------------------------------------

/// Internal watcher record stored in AppState.
#[derive(Debug, Clone)]
pub struct WatcherRecord {
    pub watcher: Watcher,
    pub registered_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl WatcherRecord {
    fn to_response(&self) -> WatcherResponse {
        WatcherResponse {
            id: *self.watcher.id.as_uuid(),
            state: self.watcher.state.as_str().to_string(),
            bonded_stake: self.watcher.bonded_stake,
            slashed_amount: self.watcher.slashed_amount,
            available_stake: self.watcher.available_stake(),
            slash_count: self.watcher.slash_count,
            attestation_count: self.watcher.attestation_count,
            is_terminal: self.watcher.state.is_terminal(),
            registered_at: self.registered_at,
            updated_at: self.updated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the watcher economy router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/watchers", post(register_watcher).get(list_watchers))
        .route("/v1/watchers/:watcher_id", get(get_watcher))
        .route("/v1/watchers/:watcher_id/bond", post(bond_watcher))
        .route("/v1/watchers/:watcher_id/activate", post(activate_watcher))
        .route("/v1/watchers/:watcher_id/slash", post(slash_watcher))
        .route("/v1/watchers/:watcher_id/rebond", post(rebond_watcher))
        .route("/v1/watchers/:watcher_id/unbond", post(unbond_watcher))
        .route(
            "/v1/watchers/:watcher_id/complete-unbond",
            post(complete_unbond_watcher),
        )
        .route("/v1/watchers/:watcher_id/attest", post(record_attestation))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/watchers — Register a new watcher node.
#[utoipa::path(
    post,
    path = "/v1/watchers",
    responses(
        (status = 201, description = "Watcher registered", body = WatcherResponse),
    ),
    tag = "watchers"
)]
async fn register_watcher(
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<WatcherResponse>), AppError> {
    let id = WatcherId::new();
    let uuid = *id.as_uuid();
    let now = Utc::now();
    let watcher = Watcher::new(id);
    let record = WatcherRecord {
        watcher,
        registered_at: now,
        updated_at: now,
    };
    let response = record.to_response();
    state.watchers.insert(uuid, record);
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /v1/watchers — List all registered watchers.
#[utoipa::path(
    get,
    path = "/v1/watchers",
    responses(
        (status = 200, description = "List of watchers", body = WatcherListResponse),
    ),
    tag = "watchers"
)]
async fn list_watchers(
    State(state): State<AppState>,
) -> Json<WatcherListResponse> {
    let records = state.watchers.list();
    let total = records.len();
    // Cap the returned list to prevent unbounded response payloads.
    const MAX_LIST: usize = 1000;
    let watchers: Vec<WatcherResponse> = records
        .iter()
        .take(MAX_LIST)
        .map(|r| r.to_response())
        .collect();
    Json(WatcherListResponse { watchers, total })
}

/// GET /v1/watchers/:watcher_id — Get watcher details.
#[utoipa::path(
    get,
    path = "/v1/watchers/{watcher_id}",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    responses(
        (status = 200, description = "Watcher details", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn get_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
) -> Result<Json<WatcherResponse>, AppError> {
    let record = state
        .watchers
        .get(&watcher_id)
        .ok_or_else(|| AppError::NotFound(format!("watcher {watcher_id} not found")))?;
    Ok(Json(record.to_response()))
}

/// POST /v1/watchers/:watcher_id/bond — Post a bond (stake collateral).
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/bond",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    request_body = BondRequest,
    responses(
        (status = 200, description = "Bond posted", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn bond_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
    Json(req): Json<BondRequest>,
) -> Result<Json<WatcherResponse>, AppError> {
    if req.stake == 0 {
        return Err(AppError::Validation("stake must be greater than 0".to_string()));
    }
    let result = state.watchers.try_update(&watcher_id, |record| {
        record.watcher.bond(req.stake).map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(record.to_response())
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/activate — Activate for corridor monitoring.
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/activate",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    responses(
        (status = 200, description = "Watcher activated", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn activate_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
) -> Result<Json<WatcherResponse>, AppError> {
    let result = state.watchers.try_update(&watcher_id, |record| {
        record
            .watcher
            .activate()
            .map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(record.to_response())
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/slash — Slash for a protocol violation.
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/slash",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    request_body = SlashRequest,
    responses(
        (status = 200, description = "Watcher slashed", body = SlashResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 400, description = "Invalid slashing condition", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn slash_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
    Json(req): Json<SlashRequest>,
) -> Result<Json<SlashResponse>, AppError> {
    let condition = parse_slashing_condition(&req.condition)?;

    let result = state.watchers.try_update(&watcher_id, |record| {
        let amount_slashed = record
            .watcher
            .slash(condition)
            .map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(SlashResponse {
            watcher: record.to_response(),
            amount_slashed,
            condition: condition.as_str().to_string(),
            slash_percentage: condition.slash_percentage(),
        })
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/rebond — Re-bond after being slashed.
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/rebond",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    request_body = RebondRequest,
    responses(
        (status = 200, description = "Watcher re-bonded", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn rebond_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
    Json(req): Json<RebondRequest>,
) -> Result<Json<WatcherResponse>, AppError> {
    if req.additional_stake == 0 {
        return Err(AppError::Validation("additional_stake must be greater than 0".to_string()));
    }
    let result = state.watchers.try_update(&watcher_id, |record| {
        record
            .watcher
            .rebond(req.additional_stake)
            .map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(record.to_response())
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/unbond — Begin voluntary unbonding.
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/unbond",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    responses(
        (status = 200, description = "Unbonding initiated", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn unbond_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
) -> Result<Json<WatcherResponse>, AppError> {
    let result = state.watchers.try_update(&watcher_id, |record| {
        record.watcher.unbond().map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(record.to_response())
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/complete-unbond — Complete unbonding (terminal).
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/complete-unbond",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    responses(
        (status = 200, description = "Unbonding complete, stake returned", body = UnbondResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state transition", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn complete_unbond_watcher(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
) -> Result<Json<UnbondResponse>, AppError> {
    let result = state.watchers.try_update(&watcher_id, |record| {
        let returned = record
            .watcher
            .complete_unbond()
            .map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(UnbondResponse {
            watcher: record.to_response(),
            stake_returned: returned,
        })
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

/// POST /v1/watchers/:watcher_id/attest — Record a successful attestation.
#[utoipa::path(
    post,
    path = "/v1/watchers/{watcher_id}/attest",
    params(("watcher_id" = Uuid, Path, description = "Watcher UUID")),
    responses(
        (status = 200, description = "Attestation recorded", body = WatcherResponse),
        (status = 404, description = "Watcher not found", body = crate::error::ErrorBody),
        (status = 409, description = "Invalid state (must be ACTIVE)", body = crate::error::ErrorBody),
    ),
    tag = "watchers"
)]
async fn record_attestation(
    State(state): State<AppState>,
    Path(watcher_id): Path<Uuid>,
) -> Result<Json<WatcherResponse>, AppError> {
    let result = state.watchers.try_update(&watcher_id, |record| {
        record
            .watcher
            .record_attestation()
            .map_err(|e| e.to_string())?;
        record.updated_at = Utc::now();
        Ok(record.to_response())
    });
    match result {
        Some(Ok(response)) => Ok(Json(response)),
        Some(Err(msg)) => Err(AppError::Conflict(msg)),
        None => Err(AppError::NotFound(format!(
            "watcher {watcher_id} not found"
        ))),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_slashing_condition(s: &str) -> Result<SlashingCondition, AppError> {
    match s.to_lowercase().as_str() {
        "equivocation" => Ok(SlashingCondition::Equivocation),
        "availability_failure" => Ok(SlashingCondition::AvailabilityFailure),
        "false_attestation" => Ok(SlashingCondition::FalseAttestation),
        "collusion" => Ok(SlashingCondition::Collusion),
        _ => Err(AppError::Validation(format!(
            "unknown slashing condition: {s}. Valid: equivocation, availability_failure, false_attestation, collusion"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn register_watcher_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.state, "REGISTERED");
        assert_eq!(watcher.bonded_stake, 0);
        assert!(!watcher.is_terminal);
    }

    #[tokio::test]
    async fn full_watcher_lifecycle() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Register
        let req = Request::builder()
            .method("POST")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let watcher: WatcherResponse = body_json(resp).await;
        let watcher_id = watcher.id;

        // Bond
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/bond"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"stake": 1000000}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.state, "BONDED");
        assert_eq!(watcher.bonded_stake, 1_000_000);

        // Activate
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/activate"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.state, "ACTIVE");

        // Record attestation
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/attest"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.attestation_count, 1);

        // Unbond
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/unbond"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.state, "UNBONDING");

        // Complete unbond
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/complete-unbond"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let unbond: UnbondResponse = body_json(resp).await;
        assert_eq!(unbond.watcher.state, "DEACTIVATED");
        assert_eq!(unbond.stake_returned, 1_000_000);
        assert!(unbond.watcher.is_terminal);
    }

    #[tokio::test]
    async fn slash_and_rebond_lifecycle() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Register → Bond → Activate
        let req = Request::builder()
            .method("POST")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let watcher: WatcherResponse = body_json(resp).await;
        let watcher_id = watcher.id;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/bond"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"stake": 1000000}"#))
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/activate"))
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        // Slash (availability failure = 1%)
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/slash"))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"condition": "availability_failure"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let slash: SlashResponse = body_json(resp).await;
        assert_eq!(slash.amount_slashed, 10_000); // 1% of 1M
        assert_eq!(slash.watcher.state, "SLASHED");
        assert_eq!(slash.watcher.available_stake, 990_000);

        // Rebond
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/rebond"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"additional_stake": 50000}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let watcher: WatcherResponse = body_json(resp).await;
        assert_eq!(watcher.state, "BONDED");
        assert_eq!(watcher.bonded_stake, 1_050_000);
    }

    #[tokio::test]
    async fn collusion_slash_bans_permanently() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Register → Bond → Activate
        let req = Request::builder()
            .method("POST")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let watcher: WatcherResponse = body_json(resp).await;
        let watcher_id = watcher.id;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/bond"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"stake": 1000000}"#))
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/activate"))
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        // Slash for collusion (100% + ban)
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/slash"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"condition": "collusion"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let slash: SlashResponse = body_json(resp).await;
        assert_eq!(slash.amount_slashed, 1_000_000);
        assert_eq!(slash.watcher.state, "BANNED");
        assert!(slash.watcher.is_terminal);
    }

    #[tokio::test]
    async fn get_watcher_not_found_returns_404() {
        let app = test_app();
        let fake_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/watchers/{fake_id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn invalid_slash_condition_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Register → Bond → Activate
        let req = Request::builder()
            .method("POST")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let watcher: WatcherResponse = body_json(resp).await;
        let watcher_id = watcher.id;

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/bond"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"stake": 1000000}"#))
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/activate"))
            .body(Body::empty())
            .unwrap();
        let _ = app.clone().oneshot(req).await.unwrap();

        // Invalid condition
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/watchers/{watcher_id}/slash"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"condition": "invalid_condition"}"#))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn list_watchers_returns_registered() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Register two watchers
        for _ in 0..2 {
            let req = Request::builder()
                .method("POST")
                .uri("/v1/watchers")
                .body(Body::empty())
                .unwrap();
            let _ = app.clone().oneshot(req).await.unwrap();
        }

        let req = Request::builder()
            .method("GET")
            .uri("/v1/watchers")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let list: WatcherListResponse = body_json(resp).await;
        assert_eq!(list.total, 2);
        assert_eq!(list.watchers.len(), 2);
    }

    #[test]
    fn parse_slashing_conditions() {
        assert!(parse_slashing_condition("equivocation").is_ok());
        assert!(parse_slashing_condition("availability_failure").is_ok());
        assert!(parse_slashing_condition("false_attestation").is_ok());
        assert!(parse_slashing_condition("collusion").is_ok());
        assert!(parse_slashing_condition("EQUIVOCATION").is_ok());
        assert!(parse_slashing_condition("unknown").is_err());
    }

    #[test]
    fn router_builds_successfully() {
        let _router = router();
    }
}
