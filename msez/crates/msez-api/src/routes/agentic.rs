//! # Agentic Policy Engine API
//!
//! HTTP surface for the autonomous policy engine. Accepts environmental
//! triggers, evaluates them against registered policies, and dispatches
//! resulting actions to domain operations.
//!
//! This is the nervous system: environmental change enters through
//! `/v1/triggers` and propagates through policy evaluation into corridor
//! transitions, compliance re-evaluations, and audit trail entries.
//!
//! ## Reactive Bridge
//!
//! When the policy engine produces a `Halt` or `Resume` action targeting
//! a corridor, the handler transitions the corridor via the typestate
//! machine (`DynCorridorState::valid_transitions`). All other actions
//! are recorded as "scheduled" — awaiting future executors.

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use chrono::Utc;
use msez_agentic::{PolicyAction, ScheduledAction, Trigger, TriggerType};
use msez_core::{sha256_digest, CanonicalBytes};
use msez_state::{DynCorridorState, TransitionRecord};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::AppState;
use axum::extract::rejection::JsonRejection;

// ---------------------------------------------------------------------------
// Request / Response types
// ---------------------------------------------------------------------------

/// Request to submit an environmental trigger for policy evaluation.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TriggerRequest {
    /// Trigger type name (e.g., "sanctions_list_update", "license_status_change").
    pub trigger_type: String,
    /// Asset or corridor ID affected by this trigger.
    pub asset_id: Option<String>,
    /// Jurisdiction scope for policy filtering.
    pub jurisdiction: Option<String>,
    /// Trigger payload — event-specific data.
    pub data: Option<serde_json::Value>,
}

impl Validate for TriggerRequest {
    fn validate(&self) -> Result<(), String> {
        if self.trigger_type.trim().is_empty() {
            return Err("trigger_type must not be empty".to_string());
        }
        if self.trigger_type.len() > 255 {
            return Err("trigger_type must not exceed 255 characters".to_string());
        }
        if let Some(ref j) = self.jurisdiction {
            if j.len() > 255 {
                return Err("jurisdiction must not exceed 255 characters".to_string());
            }
        }
        Ok(())
    }
}

/// Response from trigger evaluation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TriggerResponse {
    /// The trigger type that was evaluated.
    pub trigger_type: String,
    /// Number of actions produced by policy evaluation.
    pub actions_produced: usize,
    /// Per-action execution results.
    pub actions: Vec<ActionResult>,
    /// Number of recent audit entries (snapshot).
    pub audit_entries: usize,
}

/// Execution status of a dispatched policy action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionStatus {
    /// Action was executed successfully (e.g., corridor transitioned).
    Executed,
    /// Action was recorded for future execution by a domain executor.
    Scheduled,
    /// Action was skipped (target not found, invalid transition, etc.).
    Skipped,
    /// Action execution failed with an error.
    Failed,
}

/// Result of dispatching a single action.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ActionResult {
    /// Unique action identifier.
    pub action_id: String,
    /// Action type (e.g., "halt", "resume", "update_manifest").
    pub action_type: String,
    /// Execution status.
    pub status: ActionStatus,
    /// Human-readable detail about the action outcome.
    pub detail: Option<String>,
    /// Resource affected by the action (corridor UUID, asset ID, etc.).
    pub affected_resource: Option<String>,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the agentic policy engine router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/triggers", post(submit_trigger))
        .route("/v1/policies", get(list_policies))
        .route("/v1/policies/:id", delete(delete_policy))
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/triggers — Submit an environmental trigger for policy evaluation.
///
/// The engine evaluates the trigger against all registered policies,
/// resolves conflicts, and dispatches immediate actions. Actions that
/// cannot be executed immediately are scheduled for later execution.
///
/// ## Reactive Bridge
///
/// When the evaluation produces a `Halt` or `Resume` action targeting
/// a corridor, the handler transitions the corridor via the typestate
/// machine (`DynCorridorState::valid_transitions`). The trigger's audit
/// entry becomes the evidence for the corridor transition.
async fn submit_trigger(
    State(state): State<AppState>,
    body: Result<Json<TriggerRequest>, JsonRejection>,
) -> Result<Json<TriggerResponse>, AppError> {
    let req = extract_validated_json(body)?;

    // Parse the trigger type.
    let trigger_type: TriggerType =
        serde_json::from_value(serde_json::Value::String(req.trigger_type.clone())).map_err(
            |e| AppError::Validation(format!("unknown trigger type: '{}': {e}", req.trigger_type,)),
        )?;

    // Build the trigger.
    let trigger = Trigger::new(trigger_type, req.data.clone().unwrap_or_default());

    // Evaluate against the policy engine.
    let actions = {
        let mut engine = state.policy_engine.lock();

        engine.process_trigger(
            &trigger,
            req.asset_id.as_deref().unwrap_or("*"),
            req.jurisdiction.as_deref(),
        )
    };

    // Dispatch actions (engine lock released so corridor updates don't deadlock).
    let mut action_results = Vec::new();
    for action in &actions {
        let result = dispatch_action(&state, action).await;
        action_results.push(result);
    }

    // Snapshot audit trail size.
    let audit_entries = state.policy_engine.lock().audit_trail.last_n(10).len();

    Ok(Json(TriggerResponse {
        trigger_type: req.trigger_type,
        actions_produced: actions.len(),
        actions: action_results,
        audit_entries,
    }))
}

/// GET /v1/policies — List all registered policies.
async fn list_policies(
    State(state): State<AppState>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let engine = state.policy_engine.lock();

    let policies: Vec<serde_json::Value> = engine
        .list_policies()
        .iter()
        .map(|p| {
            serde_json::json!({
                "policy_id": p.policy_id,
                "trigger_type": p.trigger_type.as_str(),
                "action": p.action.as_str(),
                "priority": p.priority,
                "description": p.description,
            })
        })
        .collect();

    Ok(Json(policies))
}

/// DELETE /v1/policies/:id — Unregister a policy.
async fn delete_policy(
    State(state): State<AppState>,
    Path(policy_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut engine = state.policy_engine.lock();

    let removed = engine.unregister_policy(&policy_id);
    match removed {
        Some(p) => Ok(Json(serde_json::json!({
            "removed": true,
            "policy_id": p.policy_id,
        }))),
        None => Err(AppError::NotFound(format!(
            "policy '{}' not found",
            policy_id
        ))),
    }
}

// ---------------------------------------------------------------------------
// Reactive dispatch
// ---------------------------------------------------------------------------

/// Dispatch a scheduled action to the appropriate domain operation.
///
/// For this release, only `Halt` and `Resume` are wired to corridor
/// transitions. All other actions are recorded as "scheduled" pending
/// future executors.
async fn dispatch_action(state: &AppState, action: &ScheduledAction) -> ActionResult {
    match action.action {
        PolicyAction::Halt => {
            dispatch_corridor_transition(state, action, DynCorridorState::Halted).await
        }
        PolicyAction::Resume => {
            dispatch_corridor_transition(state, action, DynCorridorState::Active).await
        }
        _ => {
            // Action type has no executor yet. Record as scheduled.
            ActionResult {
                action_id: action.action_id.clone(),
                action_type: action.action.as_str().to_string(),
                status: ActionStatus::Scheduled,
                detail: Some(format!(
                    "action '{}' recorded but no executor wired yet",
                    action.action.as_str()
                )),
                affected_resource: None,
            }
        }
    }
}

/// Attempt to transition a corridor based on a policy action.
///
/// Parses the `asset_id` as a corridor UUID, looks up the corridor,
/// validates the transition via the typestate machine, and applies it.
async fn dispatch_corridor_transition(
    state: &AppState,
    action: &ScheduledAction,
    target: DynCorridorState,
) -> ActionResult {
    // Parse asset_id as corridor UUID.
    let corridor_id = match uuid::Uuid::parse_str(&action.asset_id) {
        Ok(id) => id,
        Err(_) => {
            return ActionResult {
                action_id: action.action_id.clone(),
                action_type: action.action.as_str().to_string(),
                status: ActionStatus::Skipped,
                detail: Some(format!(
                    "asset_id '{}' is not a valid corridor UUID",
                    action.asset_id
                )),
                affected_resource: None,
            };
        }
    };

    // Look up the corridor.
    let corridor = match state.corridors.get(&corridor_id) {
        Some(c) => c,
        None => {
            return ActionResult {
                action_id: action.action_id.clone(),
                action_type: action.action.as_str().to_string(),
                status: ActionStatus::Skipped,
                detail: Some(format!("corridor {} not found", corridor_id)),
                affected_resource: None,
            };
        }
    };

    // Validate via the typestate machine.
    let valid_targets = corridor.state.valid_transitions();
    if !valid_targets.contains(&target) {
        return ActionResult {
            action_id: action.action_id.clone(),
            action_type: action.action.as_str().to_string(),
            status: ActionStatus::Skipped,
            detail: Some(format!(
                "corridor {} cannot transition from {} to {} (valid: [{}])",
                corridor_id,
                corridor.state.as_str(),
                target.as_str(),
                valid_targets
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            )),
            affected_resource: None,
        };
    }

    // Compute evidence digest from the policy action metadata.
    let evidence_digest = {
        let evidence_data = serde_json::json!({
            "policy_id": action.policy_id,
            "action_id": action.action_id,
            "trigger": "agentic_policy_execution",
        });
        CanonicalBytes::new(&evidence_data)
            .ok()
            .map(|c| sha256_digest(&c))
    };

    // Build the transition record.
    let now = Utc::now();
    let record = TransitionRecord {
        from_state: corridor.state,
        to_state: target,
        timestamp: now,
        evidence_digest,
    };

    // Apply the transition.
    let updated = state.corridors.update(&corridor_id, |c| {
        c.state = target;
        c.transition_log.push(record.clone());
        c.updated_at = now;
    });

    match updated {
        Some(_) => ActionResult {
            action_id: action.action_id.clone(),
            action_type: action.action.as_str().to_string(),
            status: ActionStatus::Executed,
            detail: Some(format!(
                "corridor {} transitioned to {}",
                corridor_id,
                target.as_str()
            )),
            affected_resource: Some(corridor_id.to_string()),
        },
        None => ActionResult {
            action_id: action.action_id.clone(),
            action_type: action.action.as_str().to_string(),
            status: ActionStatus::Failed,
            detail: Some(format!(
                "corridor {} disappeared during transition",
                corridor_id
            )),
            affected_resource: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::CorridorRecord;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    use uuid::Uuid;

    /// Helper: build the agentic router only (no auth middleware) from a given AppState.
    fn agentic_app(state: AppState) -> Router<()> {
        router().with_state(state)
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Helper: create an ACTIVE corridor directly in the store.
    fn insert_active_corridor(state: &AppState) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        state.corridors.insert(
            id,
            CorridorRecord {
                id,
                jurisdiction_a: "pk-sez-01".into(),
                jurisdiction_b: "ae-difc".into(),
                state: DynCorridorState::Active,
                transition_log: vec![],
                created_at: now,
                updated_at: now,
            },
        );
        id
    }

    /// Helper: create a DRAFT corridor directly in the store.
    fn insert_draft_corridor(state: &AppState) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        state.corridors.insert(
            id,
            CorridorRecord {
                id,
                jurisdiction_a: "pk-sez-01".into(),
                jurisdiction_b: "ae-difc".into(),
                state: DynCorridorState::Draft,
                transition_log: vec![],
                created_at: now,
                updated_at: now,
            },
        );
        id
    }

    // ── Trigger ingestion ─────────────────────────────────────────

    #[tokio::test]
    async fn sanctions_trigger_halts_active_corridor() {
        let state = AppState::new();
        let corridor_id = insert_active_corridor(&state);
        let app = agentic_app(state.clone());

        let body = serde_json::json!({
            "trigger_type": "sanctions_list_update",
            "asset_id": corridor_id.to_string(),
            "data": {
                "affected_parties": ["self"],
                "source": "OFAC",
                "list_version": "2026-02-15"
            }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;

        // The engine should have produced at least one action.
        assert!(
            resp.actions_produced > 0,
            "expected actions from sanctions trigger"
        );

        // At least one action should have been executed (the halt).
        let executed: Vec<_> = resp
            .actions
            .iter()
            .filter(|a| a.status == ActionStatus::Executed)
            .collect();
        assert!(
            !executed.is_empty(),
            "expected at least one executed action"
        );

        // The corridor should now be HALTED.
        let corridor = state.corridors.get(&corridor_id).unwrap();
        assert_eq!(corridor.state, DynCorridorState::Halted);

        // The transition log should have an entry with agentic evidence.
        assert!(!corridor.transition_log.is_empty());
        let last = corridor.transition_log.last().unwrap();
        assert_eq!(last.from_state, DynCorridorState::Active);
        assert_eq!(last.to_state, DynCorridorState::Halted);
        assert!(last.evidence_digest.is_some());
    }

    #[tokio::test]
    async fn sanctions_trigger_on_draft_corridor_skips() {
        // A DRAFT corridor cannot transition to HALTED (typestate: DRAFT -> PENDING only).
        // The engine produces a Halt action, but the dispatcher validates the transition
        // and skips it — the typestate machine's rules are respected even in reactive mode.
        let state = AppState::new();
        let corridor_id = insert_draft_corridor(&state);
        let app = agentic_app(state.clone());

        let body = serde_json::json!({
            "trigger_type": "sanctions_list_update",
            "asset_id": corridor_id.to_string(),
            "data": { "affected_parties": ["self"] }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;

        // Action should be "skipped" because DRAFT cannot transition to HALTED.
        let skipped: Vec<_> = resp
            .actions
            .iter()
            .filter(|a| a.status == ActionStatus::Skipped)
            .collect();
        assert!(
            !skipped.is_empty(),
            "expected skipped action for DRAFT corridor"
        );

        // Corridor should still be DRAFT.
        let corridor = state.corridors.get(&corridor_id).unwrap();
        assert_eq!(corridor.state, DynCorridorState::Draft);
    }

    #[tokio::test]
    async fn trigger_with_nonexistent_corridor_skips() {
        let state = AppState::new();
        let app = agentic_app(state);

        let body = serde_json::json!({
            "trigger_type": "sanctions_list_update",
            "asset_id": Uuid::new_v4().to_string(),
            "data": { "affected_parties": ["self"] }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;
        // Actions should exist but be skipped (corridor not found).
        let skipped: Vec<_> = resp
            .actions
            .iter()
            .filter(|a| a.status == ActionStatus::Skipped)
            .collect();
        assert!(
            !skipped.is_empty(),
            "expected skipped action for nonexistent corridor"
        );
    }

    #[tokio::test]
    async fn trigger_with_non_uuid_asset_id_skips() {
        let state = AppState::new();
        let app = agentic_app(state);

        let body = serde_json::json!({
            "trigger_type": "sanctions_list_update",
            "asset_id": "not-a-uuid",
            "data": { "affected_parties": ["self"] }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;
        // Halt actions should be skipped (invalid UUID).
        let skipped: Vec<_> = resp
            .actions
            .iter()
            .filter(|a| a.status == ActionStatus::Skipped)
            .collect();
        assert!(
            !skipped.is_empty(),
            "expected skipped action for non-UUID asset_id"
        );
    }

    #[tokio::test]
    async fn invalid_trigger_type_returns_422() {
        let state = AppState::new();
        let app = agentic_app(state);

        let body = serde_json::json!({
            "trigger_type": "nonexistent_trigger_type",
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn empty_trigger_type_returns_422() {
        let state = AppState::new();
        let app = agentic_app(state);

        let body = serde_json::json!({
            "trigger_type": "",
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn trigger_without_asset_id_uses_wildcard() {
        let state = AppState::new();
        let app = agentic_app(state);

        let body = serde_json::json!({
            "trigger_type": "checkpoint_due",
            "data": { "receipts_since_last": 150 }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;
        assert_eq!(resp.trigger_type, "checkpoint_due");
    }

    // ── Non-corridor actions are recorded as scheduled ────────────

    #[tokio::test]
    async fn non_corridor_action_recorded_as_scheduled() {
        let state = AppState::new();
        let app = agentic_app(state);

        // dispute_filed triggers Halt (wired) but may also trigger other actions
        // depending on extended policies. Use a trigger that produces non-Halt actions.
        let body = serde_json::json!({
            "trigger_type": "ruling_received",
            "asset_id": Uuid::new_v4().to_string(),
            "data": { "ruling_id": "rul-001" }
        });

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: TriggerResponse = body_json(response).await;
        // At least some actions should exist (from extended policies).
        // If actions exist that aren't Halt/Resume, they should be "scheduled".
        for action in &resp.actions {
            if action.action_type != "halt" && action.action_type != "resume" {
                assert_eq!(
                    action.status,
                    ActionStatus::Scheduled,
                    "non-halt/resume action should be scheduled, got: {:?}",
                    action.status
                );
            }
        }
    }

    // ── Policy management ─────────────────────────────────────────

    #[tokio::test]
    async fn list_policies_returns_extended_set() {
        let state = AppState::new();
        let app = agentic_app(state);

        let request = Request::builder()
            .method("GET")
            .uri("/v1/policies")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let policies: Vec<serde_json::Value> = body_json(response).await;
        // Extended policies include 19+ policies.
        assert!(
            policies.len() >= 10,
            "expected >= 10 extended policies, got {}",
            policies.len()
        );

        // Each policy should have the expected fields.
        for p in &policies {
            assert!(p["policy_id"].is_string());
            assert!(p["trigger_type"].is_string());
            assert!(p["action"].is_string());
            assert!(p["priority"].is_number());
            assert!(p["description"].is_string());
        }
    }

    #[tokio::test]
    async fn delete_policy_removes_it() {
        let state = AppState::new();

        // Find a policy to delete.
        let policy_id = {
            let engine = state.policy_engine.lock();
            let policies = engine.list_policies();
            policies.first().unwrap().policy_id.clone()
        };

        let initial_count = {
            let engine = state.policy_engine.lock();
            engine.list_policies().len()
        };

        let app = agentic_app(state.clone());

        let request = Request::builder()
            .method("DELETE")
            .uri(format!("/v1/policies/{}", policy_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let resp: serde_json::Value = body_json(response).await;
        assert_eq!(resp["removed"], true);
        assert_eq!(resp["policy_id"], policy_id);

        // Verify the policy was actually removed.
        let final_count = {
            let engine = state.policy_engine.lock();
            engine.list_policies().len()
        };
        assert_eq!(final_count, initial_count - 1);
    }

    #[tokio::test]
    async fn delete_nonexistent_policy_returns_404() {
        let state = AppState::new();
        let app = agentic_app(state);

        let request = Request::builder()
            .method("DELETE")
            .uri("/v1/policies/nonexistent_policy_xyz")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ── Determinism ───────────────────────────────────────────────

    #[tokio::test]
    async fn evaluation_is_deterministic() {
        // Submit the same trigger twice to fresh engines.
        // Verify: both responses produce the same action set.
        // This tests Theorem 17.1 through the HTTP layer.

        let trigger_body = serde_json::json!({
            "trigger_type": "sanctions_list_update",
            "asset_id": Uuid::new_v4().to_string(),
            "data": {
                "affected_parties": ["self"],
                "source": "OFAC",
            }
        });
        let body_str = serde_json::to_string(&trigger_body).unwrap();

        let mut action_sets = Vec::new();
        for _ in 0..2 {
            let state = AppState::new();
            let app = agentic_app(state);

            let request = Request::builder()
                .method("POST")
                .uri("/v1/triggers")
                .header("content-type", "application/json")
                .body(Body::from(body_str.clone()))
                .unwrap();

            let response = app.oneshot(request).await.unwrap();
            let resp: TriggerResponse = body_json(response).await;

            // Collect (action_type, status) pairs for comparison.
            let set: Vec<(String, ActionStatus)> = resp
                .actions
                .iter()
                .map(|a| (a.action_type.clone(), a.status))
                .collect();
            action_sets.push(set);
        }

        assert_eq!(
            action_sets[0], action_sets[1],
            "identical triggers must produce identical action sets"
        );
    }

    // ── Bad request handling ──────────────────────────────────────

    #[tokio::test]
    async fn trigger_bad_json_returns_422() {
        // BUG-038: JSON parse errors now return 422 (Unprocessable Entity).
        let state = AppState::new();
        let app = agentic_app(state);

        let request = Request::builder()
            .method("POST")
            .uri("/v1/triggers")
            .header("content-type", "application/json")
            .body(Body::from(r#"not json"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    // ── Router construction ───────────────────────────────────────

    #[test]
    fn test_router_builds_successfully() {
        let _router = router();
    }
}
