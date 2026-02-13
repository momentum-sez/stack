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
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, CorridorRecord, CorridorTransitionEntry};
use axum::extract::rejection::JsonRejection;

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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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
        status: "DRAFT".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateCorridorRequest validation ───────────────────────────

    #[test]
    fn test_create_corridor_request_valid() {
        let req = CreateCorridorRequest {
            jurisdiction_a: "PK-PSEZ".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_corridor_request_empty_jurisdiction_a() {
        let req = CreateCorridorRequest {
            jurisdiction_a: "".to_string(),
            jurisdiction_b: "AE-DIFC".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("non-empty"), "error should mention non-empty: {err}");
    }

    #[test]
    fn test_create_corridor_request_empty_jurisdiction_b() {
        let req = CreateCorridorRequest {
            jurisdiction_a: "PK-PSEZ".to_string(),
            jurisdiction_b: "  ".to_string(),
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_corridor_request_same_jurisdictions() {
        let req = CreateCorridorRequest {
            jurisdiction_a: "PK-PSEZ".to_string(),
            jurisdiction_b: "PK-PSEZ".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("differ"), "error should mention differ: {err}");
    }

    // ── TransitionCorridorRequest validation ──────────────────────

    #[test]
    fn test_transition_corridor_request_valid_pending() {
        let req = TransitionCorridorRequest {
            target_state: "PENDING".to_string(),
            evidence_digest: None,
            reason: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transition_corridor_request_valid_active() {
        let req = TransitionCorridorRequest {
            target_state: "ACTIVE".to_string(),
            evidence_digest: Some("abc123".to_string()),
            reason: Some("compliance approved".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transition_corridor_request_valid_halted() {
        let req = TransitionCorridorRequest {
            target_state: "HALTED".to_string(),
            evidence_digest: None,
            reason: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transition_corridor_request_valid_suspended() {
        let req = TransitionCorridorRequest {
            target_state: "SUSPENDED".to_string(),
            evidence_digest: None,
            reason: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transition_corridor_request_valid_deprecated() {
        let req = TransitionCorridorRequest {
            target_state: "DEPRECATED".to_string(),
            evidence_digest: None,
            reason: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_transition_corridor_request_invalid_state() {
        let req = TransitionCorridorRequest {
            target_state: "INVALID_STATE".to_string(),
            evidence_digest: None,
            reason: None,
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("target_state"), "error should mention target_state: {err}");
    }

    #[test]
    fn test_transition_corridor_request_empty_state() {
        let req = TransitionCorridorRequest {
            target_state: "".to_string(),
            evidence_digest: None,
            reason: None,
        };
        assert!(req.validate().is_err());
    }

    // ── ProposeReceiptRequest validation ──────────────────────────

    #[test]
    fn test_propose_receipt_request_valid() {
        let req = ProposeReceiptRequest {
            corridor_id: Uuid::new_v4(),
            payload: serde_json::json!({"key": "value"}),
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

    /// Helper: build the corridors router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_corridor_returns_201() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: CorridorRecord = body_json(resp).await;
        assert_eq!(record.jurisdiction_a, "PK-PSEZ");
        assert_eq!(record.jurisdiction_b, "AE-DIFC");
        assert_eq!(record.state, "DRAFT");
        assert!(record.transition_log.is_empty());
    }

    #[tokio::test]
    async fn handler_create_corridor_same_jurisdictions_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"PK-PSEZ"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_corridor_empty_jurisdiction_returns_422() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_list_corridors_empty_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("GET")
            .uri("/v1/corridors")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let records: Vec<CorridorRecord> = body_json(resp).await;
        assert!(records.is_empty());
    }

    #[tokio::test]
    async fn handler_list_corridors_after_create_returns_one() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);

        // List corridors.
        let list_req = Request::builder()
            .method("GET")
            .uri("/v1/corridors")
            .body(Body::empty())
            .unwrap();
        let list_resp = app.oneshot(list_req).await.unwrap();
        assert_eq!(list_resp.status(), StatusCode::OK);

        let records: Vec<CorridorRecord> = body_json(list_resp).await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].jurisdiction_a, "PK-PSEZ");
    }

    #[tokio::test]
    async fn handler_create_corridor_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"malformed"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Additional handler coverage ───────────────────────────────

    #[tokio::test]
    async fn handler_get_corridor_found_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        assert_eq!(create_resp.status(), StatusCode::CREATED);
        let created: CorridorRecord = body_json(create_resp).await;

        // Get the corridor by ID.
        let get_req = Request::builder()
            .method("GET")
            .uri(&format!("/v1/corridors/{}", created.id))
            .body(Body::empty())
            .unwrap();
        let get_resp = app.oneshot(get_req).await.unwrap();
        assert_eq!(get_resp.status(), StatusCode::OK);

        let fetched: CorridorRecord = body_json(get_resp).await;
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.jurisdiction_a, "PK-PSEZ");
        assert_eq!(fetched.jurisdiction_b, "AE-DIFC");
        assert_eq!(fetched.state, "DRAFT");
    }

    #[tokio::test]
    async fn handler_get_corridor_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(&format!("/v1/corridors/{id}"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_transition_corridor_returns_200() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: CorridorRecord = body_json(create_resp).await;

        // Transition to PENDING.
        let transition_req = Request::builder()
            .method("PUT")
            .uri(&format!("/v1/corridors/{}/transition", created.id))
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"target_state":"PENDING","evidence_digest":"sha256:abc123","reason":"compliance approved"}"#,
            ))
            .unwrap();
        let transition_resp = app.clone().oneshot(transition_req).await.unwrap();
        assert_eq!(transition_resp.status(), StatusCode::OK);

        let transitioned: CorridorRecord = body_json(transition_resp).await;
        assert_eq!(transitioned.state, "PENDING");
        assert_eq!(transitioned.transition_log.len(), 1);
        assert_eq!(transitioned.transition_log[0].from_state, "DRAFT");
        assert_eq!(transitioned.transition_log[0].to_state, "PENDING");
        assert_eq!(
            transitioned.transition_log[0].evidence_digest.as_deref(),
            Some("sha256:abc123")
        );

        // Transition again to ACTIVE.
        let transition_req2 = Request::builder()
            .method("PUT")
            .uri(&format!("/v1/corridors/{}/transition", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"target_state":"ACTIVE"}"#))
            .unwrap();
        let transition_resp2 = app.oneshot(transition_req2).await.unwrap();
        assert_eq!(transition_resp2.status(), StatusCode::OK);

        let transitioned2: CorridorRecord = body_json(transition_resp2).await;
        assert_eq!(transitioned2.state, "ACTIVE");
        assert_eq!(transitioned2.transition_log.len(), 2);
        assert_eq!(transitioned2.transition_log[1].from_state, "PENDING");
        assert_eq!(transitioned2.transition_log[1].to_state, "ACTIVE");
        assert!(transitioned2.transition_log[1].evidence_digest.is_none());
    }

    #[tokio::test]
    async fn handler_transition_corridor_not_found_returns_404() {
        let app = test_app();
        let id = Uuid::new_v4();
        let req = Request::builder()
            .method("PUT")
            .uri(&format!("/v1/corridors/{id}/transition"))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"target_state":"PENDING"}"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn handler_transition_corridor_invalid_state_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: CorridorRecord = body_json(create_resp).await;

        // Transition to an invalid state.
        let transition_req = Request::builder()
            .method("PUT")
            .uri(&format!("/v1/corridors/{}/transition", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{"target_state":"INVALID_STATE"}"#))
            .unwrap();
        let transition_resp = app.oneshot(transition_req).await.unwrap();
        assert_eq!(transition_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_transition_corridor_bad_json_returns_400() {
        let state = AppState::new();
        let app = router().with_state(state.clone());

        // Create a corridor.
        let create_req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let create_resp = app.clone().oneshot(create_req).await.unwrap();
        let created: CorridorRecord = body_json(create_resp).await;

        let transition_req = Request::builder()
            .method("PUT")
            .uri(&format!("/v1/corridors/{}/transition", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{broken"#))
            .unwrap();
        let transition_resp = app.oneshot(transition_req).await.unwrap();
        assert_eq!(transition_resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_propose_receipt_returns_200() {
        let app = test_app();
        let corridor_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"corridor_id":"{}","payload":{{"transaction":"transfer","amount":"5000"}}}}"#,
            corridor_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/propose")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let receipt: ReceiptResponse = body_json(resp).await;
        assert_eq!(receipt.corridor_id, corridor_id);
        assert_eq!(receipt.status, "PROPOSED");
        assert_eq!(receipt.payload["transaction"], "transfer");
    }

    #[tokio::test]
    async fn handler_propose_receipt_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/propose")
            .header("content-type", "application/json")
            .body(Body::from(r#"not json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_fork_resolve_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/fork-resolve")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["status"], "resolved");
        assert_eq!(body["strategy"], "longest_chain");
    }

    #[tokio::test]
    async fn handler_anchor_commitment_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/anchor")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["status"], "anchored");
    }

    #[tokio::test]
    async fn handler_finality_status_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/finality-status")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["status"], "pending");
        assert_eq!(body["confirmations"], 0);
    }

    #[tokio::test]
    async fn handler_create_corridor_missing_content_type_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
