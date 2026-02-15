//! # Corridor Operations API
//!
//! Handles corridor lifecycle transitions, receipt queries,
//! fork resolution, anchor verification, and finality status.
//! Route structure based on apis/corridor-state.openapi.yaml.

use axum::extract::{Path, State};
use axum::routing::{get, post, put};
use axum::{Json, Router};
use chrono::Utc;
use msez_core::{sha256_digest, CanonicalBytes, CorridorId, Timestamp};
use msez_corridor::{CorridorReceipt, ForkBranch, ForkDetector, ReceiptChain, ResolutionReason};
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
///
/// The caller provides the corridor ID and a JSON payload representing
/// the cross-border transaction event. The server computes the canonical
/// digest, validates chain integrity, and appends to the MMR.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProposeReceiptRequest {
    /// The corridor to append this receipt to.
    pub corridor_id: Uuid,
    /// Transaction payload — the content being committed to the receipt chain.
    /// Will be canonicalized and digested via CanonicalBytes → SHA-256.
    pub payload: serde_json::Value,
    /// Optional: lawpack digest set governing this receipt.
    #[serde(default)]
    pub lawpack_digest_set: Vec<String>,
    /// Optional: ruleset digest set governing this receipt.
    #[serde(default)]
    pub ruleset_digest_set: Vec<String>,
}

impl Validate for ProposeReceiptRequest {
    fn validate(&self) -> Result<(), String> {
        if self.payload.is_null() {
            return Err("payload must not be null".to_string());
        }
        Ok(())
    }
}

/// Receipt proposal response — the committed receipt with chain proof.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReceiptProposalResponse {
    /// The corridor this receipt was appended to.
    pub corridor_id: Uuid,
    /// Sequence number of this receipt in the chain (0-indexed).
    pub sequence: u64,
    /// MMR root before this receipt was appended.
    pub prev_root: String,
    /// Canonical digest of the receipt payload (SHA-256 hex, 64 chars).
    pub next_root: String,
    /// Current MMR root after appending (SHA-256 hex, 64 chars).
    pub mmr_root: String,
    /// Current chain height after appending.
    pub chain_height: u64,
    /// Receipt creation timestamp.
    pub timestamp: String,
}

/// Fork resolution request — two competing branches to resolve.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForkResolveRequest {
    /// First competing branch.
    pub branch_a: ForkBranchInput,
    /// Second competing branch.
    pub branch_b: ForkBranchInput,
}

/// Input representation of a fork branch for the API.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ForkBranchInput {
    /// Hex digest of the branch's receipt content.
    pub receipt_digest: String,
    /// ISO 8601 timestamp of the branch's receipt.
    pub timestamp: String,
    /// Number of independent watcher attestations.
    pub attestation_count: u32,
    /// The receipt's next_root digest (64 hex chars).
    pub next_root: String,
}

impl Validate for ForkResolveRequest {
    fn validate(&self) -> Result<(), String> {
        if self.branch_a.receipt_digest.is_empty() || self.branch_b.receipt_digest.is_empty() {
            return Err("receipt_digest must not be empty".to_string());
        }
        if self.branch_a.next_root.is_empty() || self.branch_b.next_root.is_empty() {
            return Err("next_root must not be empty".to_string());
        }
        Ok(())
    }
}

/// Fork resolution response.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ForkResolveResponse {
    /// Digest of the winning branch.
    pub winning_branch: String,
    /// Digest of the losing branch.
    pub losing_branch: String,
    /// Reason the winning branch was selected.
    pub resolution_reason: String,
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

    // Initialize an empty receipt chain for this corridor.
    let chain = ReceiptChain::new(CorridorId::from_uuid(id));
    state.receipt_chains.write().insert(id, chain);

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
///
/// Computes a canonical digest of the payload, validates chain integrity
/// (sequence and prev_root), appends to the MMR, and returns the
/// cryptographic proof of inclusion.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/propose",
    request_body = ProposeReceiptRequest,
    responses(
        (status = 201, description = "Receipt appended to chain", body = ReceiptProposalResponse),
        (status = 404, description = "Corridor not found", body = crate::error::ErrorBody),
        (status = 409, description = "Chain integrity violation", body = crate::error::ErrorBody),
        (status = 422, description = "Validation error", body = crate::error::ErrorBody),
    ),
    tag = "corridors"
)]
async fn propose_receipt(
    State(state): State<AppState>,
    body: Result<Json<ProposeReceiptRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<ReceiptProposalResponse>), AppError> {
    let req = extract_validated_json(body)?;

    // Acquire the receipt chain for this corridor.
    let mut chains = state.receipt_chains.write();
    let chain = chains.get_mut(&req.corridor_id).ok_or_else(|| {
        AppError::NotFound(format!("no receipt chain for corridor {}", req.corridor_id))
    })?;

    // Compute the canonical digest of the payload.
    let canonical = CanonicalBytes::new(&req.payload)
        .map_err(|e| AppError::Validation(format!("payload canonicalization failed: {e}")))?;
    let next_root = sha256_digest(&canonical).to_hex();

    // Read current chain state for the receipt.
    let prev_root = chain
        .mmr_root()
        .map_err(|e| AppError::Internal(format!("MMR root error: {e}")))?;
    let sequence = chain.height();
    let timestamp = Timestamp::now();

    // Build the receipt.
    let receipt = CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
        corridor_id: CorridorId::from_uuid(req.corridor_id),
        sequence,
        timestamp: timestamp.clone(),
        prev_root: prev_root.clone(),
        next_root: next_root.clone(),
        lawpack_digest_set: req.lawpack_digest_set,
        ruleset_digest_set: req.ruleset_digest_set,
    };

    // Append to the chain. This validates sequence and prev_root integrity.
    chain
        .append(receipt)
        .map_err(|e| AppError::Conflict(format!("receipt chain append failed: {e}")))?;

    // Read post-append state.
    let mmr_root = chain
        .mmr_root()
        .map_err(|e| AppError::Internal(format!("MMR root error: {e}")))?;
    let chain_height = chain.height();

    Ok((
        axum::http::StatusCode::CREATED,
        Json(ReceiptProposalResponse {
            corridor_id: req.corridor_id,
            sequence,
            prev_root,
            next_root,
            mmr_root,
            chain_height,
            timestamp: timestamp.to_string(),
        }),
    ))
}

/// POST /v1/corridors/state/fork-resolve — Resolve receipt fork.
///
/// Accepts two competing branches and resolves the fork using the
/// three-level ordering protocol: timestamp → attestation count →
/// lexicographic digest tiebreak.
#[utoipa::path(
    post,
    path = "/v1/corridors/state/fork-resolve",
    request_body = ForkResolveRequest,
    responses(
        (status = 200, description = "Fork resolved", body = ForkResolveResponse),
        (status = 422, description = "Validation error", body = crate::error::ErrorBody),
    ),
    tag = "corridors"
)]
async fn fork_resolve(
    State(_state): State<AppState>,
    body: Result<Json<ForkResolveRequest>, JsonRejection>,
) -> Result<Json<ForkResolveResponse>, AppError> {
    let req = extract_validated_json(body)?;

    // Parse timestamps.
    let ts_a = chrono::DateTime::parse_from_rfc3339(&req.branch_a.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            AppError::Validation(format!("branch_a.timestamp is not valid RFC 3339: {e}"))
        })?;
    let ts_b = chrono::DateTime::parse_from_rfc3339(&req.branch_b.timestamp)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            AppError::Validation(format!("branch_b.timestamp is not valid RFC 3339: {e}"))
        })?;

    // Build ForkBranch domain objects from the receipt digests.
    let digest_a = {
        let canonical = CanonicalBytes::new(&serde_json::json!({
            "digest": req.branch_a.receipt_digest
        }))
        .map_err(|e| AppError::Internal(format!("canonicalization error: {e}")))?;
        sha256_digest(&canonical)
    };
    let digest_b = {
        let canonical = CanonicalBytes::new(&serde_json::json!({
            "digest": req.branch_b.receipt_digest
        }))
        .map_err(|e| AppError::Internal(format!("canonicalization error: {e}")))?;
        sha256_digest(&canonical)
    };

    let branch_a = ForkBranch {
        receipt_digest: digest_a,
        timestamp: ts_a,
        attestation_count: req.branch_a.attestation_count,
        next_root: req.branch_a.next_root,
    };
    let branch_b = ForkBranch {
        receipt_digest: digest_b,
        timestamp: ts_b,
        attestation_count: req.branch_b.attestation_count,
        next_root: req.branch_b.next_root,
    };

    let mut detector = ForkDetector::new();
    detector.register_fork(branch_a, branch_b);
    let resolutions = detector.resolve_all();

    let resolution = resolutions
        .into_iter()
        .next()
        .ok_or_else(|| AppError::Internal("fork resolution produced no result".into()))?;

    let reason = match resolution.resolution_reason {
        ResolutionReason::EarlierTimestamp => "earlier_timestamp",
        ResolutionReason::MoreAttestations => "more_attestations",
        ResolutionReason::LexicographicTiebreak => "lexicographic_tiebreak",
    };

    Ok(Json(ForkResolveResponse {
        winning_branch: resolution.winning_branch.to_hex(),
        losing_branch: resolution.losing_branch.to_hex(),
        resolution_reason: reason.to_string(),
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
        assert!(
            err.contains("non-empty"),
            "error should mention non-empty: {err}"
        );
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
        assert!(
            err.contains("target_state"),
            "error should mention target_state: {err}"
        );
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
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_propose_receipt_request_null_payload_rejected() {
        let req = ProposeReceiptRequest {
            corridor_id: Uuid::new_v4(),
            payload: serde_json::Value::Null,
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("null"),
            "error should mention null: {err}"
        );
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
            .uri(format!("/v1/corridors/{}", created.id))
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
            .uri(format!("/v1/corridors/{id}"))
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
            .uri(format!("/v1/corridors/{}/transition", created.id))
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
            .uri(format!("/v1/corridors/{}/transition", created.id))
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
            .uri(format!("/v1/corridors/{id}/transition"))
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
            .uri(format!("/v1/corridors/{}/transition", created.id))
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
            .uri(format!("/v1/corridors/{}/transition", created.id))
            .header("content-type", "application/json")
            .body(Body::from(r#"{broken"#))
            .unwrap();
        let transition_resp = app.oneshot(transition_req).await.unwrap();
        assert_eq!(transition_resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Receipt chain integration tests ─────────────────────────

    /// Helper: create a corridor via the API and return its ID.
    async fn create_test_corridor(app: &Router<()>) -> Uuid {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"jurisdiction_a":"PK-PSEZ","jurisdiction_b":"AE-DIFC"}"#,
            ))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
        let record: CorridorRecord = body_json(resp).await;
        record.id
    }

    /// Helper: propose a receipt and return the parsed response.
    async fn propose_test_receipt(
        app: &Router<()>,
        corridor_id: Uuid,
        payload: serde_json::Value,
    ) -> (StatusCode, ReceiptProposalResponse) {
        let body_str = serde_json::to_string(&serde_json::json!({
            "corridor_id": corridor_id,
            "payload": payload,
        }))
        .unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/propose")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let receipt: ReceiptProposalResponse = body_json(resp).await;
        (status, receipt)
    }

    #[tokio::test]
    async fn propose_receipt_returns_valid_digest_and_mmr_root() {
        let state = AppState::new();
        let app = router().with_state(state);

        let corridor_id = create_test_corridor(&app).await;
        let (status, receipt) = propose_test_receipt(
            &app,
            corridor_id,
            serde_json::json!({"transaction": "transfer", "amount": "5000"}),
        )
        .await;

        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(receipt.corridor_id, corridor_id);
        assert_eq!(receipt.sequence, 0);
        assert_eq!(receipt.chain_height, 1);
        // next_root is a 64-char hex string (SHA-256).
        assert_eq!(receipt.next_root.len(), 64);
        assert!(receipt.next_root.chars().all(|c| c.is_ascii_hexdigit()));
        // mmr_root is a 64-char hex string.
        assert_eq!(receipt.mmr_root.len(), 64);
        assert!(receipt.mmr_root.chars().all(|c| c.is_ascii_hexdigit()));
        // For the first receipt, prev_root is empty (empty chain).
        assert_eq!(receipt.prev_root, "");
        // timestamp is non-empty.
        assert!(!receipt.timestamp.is_empty());
    }

    #[tokio::test]
    async fn two_receipts_form_chain() {
        let state = AppState::new();
        let app = router().with_state(state);

        let corridor_id = create_test_corridor(&app).await;

        // Receipt 0.
        let (status0, r0) = propose_test_receipt(
            &app,
            corridor_id,
            serde_json::json!({"event": "shipment_departed", "ref": "INV-001"}),
        )
        .await;
        assert_eq!(status0, StatusCode::CREATED);
        assert_eq!(r0.sequence, 0);
        assert_eq!(r0.chain_height, 1);

        // Receipt 1.
        let (status1, r1) = propose_test_receipt(
            &app,
            corridor_id,
            serde_json::json!({"event": "shipment_arrived", "ref": "INV-001"}),
        )
        .await;
        assert_eq!(status1, StatusCode::CREATED);
        assert_eq!(r1.sequence, 1);
        assert_eq!(r1.chain_height, 2);

        // Chain integrity: receipt 1's prev_root equals receipt 0's mmr_root.
        assert_eq!(
            r1.prev_root, r0.mmr_root,
            "receipt 1's prev_root must equal receipt 0's post-append mmr_root"
        );
        // MMR root changes between the two receipts.
        assert_ne!(
            r0.mmr_root, r1.mmr_root,
            "mmr_root must change after appending a second receipt"
        );
        // Different payloads produce different next_root digests.
        assert_ne!(
            r0.next_root, r1.next_root,
            "different payloads must produce different next_root digests"
        );
    }

    #[tokio::test]
    async fn propose_receipt_for_nonexistent_corridor_returns_404() {
        let app = test_app();
        let fake_corridor = Uuid::new_v4();
        let body_str = serde_json::to_string(&serde_json::json!({
            "corridor_id": fake_corridor,
            "payload": {"event": "test"},
        }))
        .unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/propose")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn propose_receipt_null_payload_returns_422() {
        let state = AppState::new();
        let app = router().with_state(state);

        let corridor_id = create_test_corridor(&app).await;
        let body_str = serde_json::to_string(&serde_json::json!({
            "corridor_id": corridor_id,
            "payload": null,
        }))
        .unwrap();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/propose")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn propose_receipt_bad_json_returns_400() {
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
    async fn propose_receipt_deterministic_digest() {
        // Two proposals with the same payload should produce the same next_root.
        let state = AppState::new();
        let app = router().with_state(state.clone());
        let app2 = router().with_state(state);

        let corridor_id_a = create_test_corridor(&app).await;
        let corridor_id_b = create_test_corridor(&app2).await;

        let payload = serde_json::json!({"event": "deterministic_test", "value": 42});

        let (_, r_a) = propose_test_receipt(&app, corridor_id_a, payload.clone()).await;
        let (_, r_b) = propose_test_receipt(&app2, corridor_id_b, payload).await;

        assert_eq!(
            r_a.next_root, r_b.next_root,
            "same payload must produce same canonical digest"
        );
    }

    // ── Fork resolution tests ────────────────────────────────────

    #[tokio::test]
    async fn handler_fork_resolve_returns_200() {
        let app = test_app();
        let now = Utc::now();
        let earlier = now - chrono::Duration::minutes(10);

        let body_str = serde_json::to_string(&serde_json::json!({
            "branch_a": {
                "receipt_digest": "aaaa",
                "timestamp": earlier.to_rfc3339(),
                "attestation_count": 3,
                "next_root": "aa".repeat(32),
            },
            "branch_b": {
                "receipt_digest": "bbbb",
                "timestamp": now.to_rfc3339(),
                "attestation_count": 5,
                "next_root": "bb".repeat(32),
            }
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/fork-resolve")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: ForkResolveResponse = body_json(resp).await;
        // 10 minutes apart > 5 minute skew → earlier timestamp wins.
        assert_eq!(body.resolution_reason, "earlier_timestamp");
        // Both digests should be 64-char hex.
        assert_eq!(body.winning_branch.len(), 64);
        assert_eq!(body.losing_branch.len(), 64);
    }

    #[tokio::test]
    async fn handler_fork_resolve_attestation_ordering() {
        let app = test_app();
        let now = Utc::now();
        // Within 5-minute skew → falls through to attestation count.
        let close = now + chrono::Duration::minutes(2);

        let body_str = serde_json::to_string(&serde_json::json!({
            "branch_a": {
                "receipt_digest": "aaaa",
                "timestamp": now.to_rfc3339(),
                "attestation_count": 2,
                "next_root": "aa".repeat(32),
            },
            "branch_b": {
                "receipt_digest": "bbbb",
                "timestamp": close.to_rfc3339(),
                "attestation_count": 7,
                "next_root": "bb".repeat(32),
            }
        }))
        .unwrap();

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/state/fork-resolve")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: ForkResolveResponse = body_json(resp).await;
        assert_eq!(body.resolution_reason, "more_attestations");
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
