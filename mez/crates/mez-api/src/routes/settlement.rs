//! # Settlement Pipeline API
//!
//! Endpoints for computing settlement plans, routing multi-hop payments,
//! and generating ISO 20022 payment instructions from corridor receipt chains.
//!
//! The pipeline: accumulate (receipts) → compress (netting) → route (bridge) → instruct (SWIFT).
//!
//! Settlement computation reads from obligations but does not modify the receipt
//! chain. Settlement plans are pure functions of input: same obligations → same plan.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use mez_core::{CorridorId, JurisdictionId};
use mez_corridor::bridge::{BridgeEdge, CorridorBridge};
use mez_corridor::netting::{NettingEngine, Obligation};
use mez_corridor::swift::{SettlementInstruction, SwiftPacs008};
use mez_corridor::SettlementRail;
use mez_state::DynCorridorState;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::AppState;

// ── Settlement Computation ──────────────────────────────────────

/// Request to compute a settlement plan from obligations.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SettlementComputeRequest {
    /// The obligations to net. Each obligation is a directed payment
    /// between two parties in a specific currency.
    #[serde(default)]
    pub obligations: Vec<ObligationInput>,
}

impl Validate for SettlementComputeRequest {
    fn validate(&self) -> Result<(), String> {
        if self.obligations.is_empty() {
            return Err("at least one obligation is required".into());
        }
        if self.obligations.len() > 10_000 {
            return Err(format!(
                "too many obligations: {} (max 10,000)",
                self.obligations.len()
            ));
        }
        for (i, ob) in self.obligations.iter().enumerate() {
            if ob.from_party.trim().is_empty() || ob.to_party.trim().is_empty() {
                return Err(format!(
                    "obligation {i}: party identifiers must be non-empty"
                ));
            }
            if ob.from_party == ob.to_party {
                return Err(format!(
                    "obligation {i}: from_party and to_party must differ"
                ));
            }
            if ob.currency.trim().is_empty() {
                return Err(format!("obligation {i}: currency must be non-empty"));
            }
            // Note: non-positive amounts are handled gracefully by NettingEngine
            // (skipped and counted), so we do NOT reject them here. See the
            // `obligations_skipped` field in SettlementPlanResponse.
        }
        Ok(())
    }
}

/// A single obligation input for settlement computation.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ObligationInput {
    /// The party that owes.
    pub from_party: String,
    /// The party that is owed.
    pub to_party: String,
    /// Amount in smallest currency unit (e.g., cents for USD).
    pub amount: i64,
    /// ISO 4217 currency code.
    pub currency: String,
    /// Optional priority (higher = settled first).
    pub priority: Option<i32>,
}

/// Response from settlement computation.
#[derive(Debug, Serialize, ToSchema)]
pub struct SettlementPlanResponse {
    pub corridor_id: Uuid,
    pub obligations_processed: usize,
    pub obligations_skipped: usize,
    pub gross_total: i64,
    pub net_total: i64,
    pub reduction_bps: u32,
    pub net_positions: Vec<NetPositionResponse>,
    pub settlement_legs: Vec<SettlementLegResponse>,
}

/// A party's net position in a specific currency.
#[derive(Debug, Serialize, ToSchema)]
pub struct NetPositionResponse {
    pub party_id: String,
    pub currency: String,
    pub receivable: i64,
    pub payable: i64,
    pub net: i64,
}

/// A single settlement leg (minimal payment).
#[derive(Debug, Serialize, ToSchema)]
pub struct SettlementLegResponse {
    pub from_party: String,
    pub to_party: String,
    pub amount: i64,
    pub currency: String,
}

// ── Bridge Routing ──────────────────────────────────────────────

/// Request to find the optimal route between two jurisdictions.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RouteRequest {
    /// Source jurisdiction identifier.
    pub source: String,
    /// Target jurisdiction identifier.
    pub target: String,
    /// Default fee basis points for corridor edges (default 10).
    pub default_fee_bps: Option<u32>,
    /// Default settlement time in seconds (default 3600).
    pub default_settlement_time_secs: Option<u64>,
}

impl Validate for RouteRequest {
    fn validate(&self) -> Result<(), String> {
        if self.source.trim().is_empty() || self.target.trim().is_empty() {
            return Err("source and target must be non-empty".into());
        }
        if self.source == self.target {
            return Err("source and target must differ".into());
        }
        Ok(())
    }
}

/// Response from route computation.
#[derive(Debug, Serialize, ToSchema)]
pub struct RouteResponse {
    pub source: String,
    pub target: String,
    pub hop_count: usize,
    pub total_fee_bps: u64,
    pub total_settlement_time_secs: u64,
    pub hops: Vec<RouteHopResponse>,
}

/// A single hop in a computed route.
#[derive(Debug, Serialize, ToSchema)]
pub struct RouteHopResponse {
    pub from: String,
    pub to: String,
    pub corridor_id: String,
    pub fee_bps: u32,
    pub settlement_time_secs: u64,
}

// ── SWIFT Instructions ──────────────────────────────────────────

/// Request to generate SWIFT pacs.008 payment instructions.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstructionRequest {
    /// BIC of the instructing agent (default: "MEZUS33").
    pub instructing_agent_bic: Option<String>,
    /// Settlement legs to generate instructions for.
    pub legs: Vec<InstructionLeg>,
}

impl Validate for InstructionRequest {
    fn validate(&self) -> Result<(), String> {
        if self.legs.is_empty() {
            return Err("at least one settlement leg is required".into());
        }
        if self.legs.len() > 1_000 {
            return Err(format!("too many legs: {} (max 1,000)", self.legs.len()));
        }
        for (i, leg) in self.legs.iter().enumerate() {
            if leg.from_party.trim().is_empty() || leg.to_party.trim().is_empty() {
                return Err(format!("leg {i}: party identifiers must be non-empty"));
            }
            if leg.from_bic.len() > 11 || leg.to_bic.len() > 11 {
                return Err(format!("leg {i}: BIC code must be at most 11 characters"));
            }
            if leg.amount <= 0 {
                return Err(format!(
                    "leg {i}: amount must be positive, got {}",
                    leg.amount
                ));
            }
            if leg.currency.trim().is_empty() {
                return Err(format!("leg {i}: currency must be non-empty"));
            }
        }
        Ok(())
    }
}

/// A single settlement leg for SWIFT instruction generation.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InstructionLeg {
    pub from_party: String,
    pub from_bic: String,
    pub from_account: Option<String>,
    pub to_party: String,
    pub to_bic: String,
    pub to_account: Option<String>,
    pub amount: i64,
    pub currency: String,
}

/// Response from SWIFT instruction generation.
#[derive(Debug, Serialize, ToSchema)]
pub struct InstructionResponse {
    pub corridor_id: Uuid,
    pub instructions_generated: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    pub instructions: Vec<Pacs008Output>,
}

/// A generated pacs.008 XML message.
#[derive(Debug, Serialize, ToSchema)]
pub struct Pacs008Output {
    pub leg_index: usize,
    pub from_party: String,
    pub to_party: String,
    pub amount: i64,
    pub currency: String,
    /// ISO 20022 pacs.008 XML payload.
    pub xml: String,
}

// ── Router ──────────────────────────────────────────────────────

/// Build the settlement pipeline router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/v1/corridors/:id/settlement/compute",
            post(compute_settlement),
        )
        .route("/v1/corridors/route", post(find_route))
        .route(
            "/v1/corridors/:id/settlement/instruct",
            post(generate_instructions),
        )
}

// ── Handlers ────────────────────────────────────────────────────

/// POST /v1/corridors/:id/settlement/compute — Compute a settlement plan.
///
/// Feeds obligations to the netting engine and returns the settlement
/// plan with net positions and minimal settlement legs.
///
/// The handler accepts obligations directly in the request body.
/// This decouples settlement computation from receipt chain storage
/// format and lets clients submit obligations from any source.
///
/// ## Negative / Zero Amount Handling (BUG-024: Resolved)
///
/// Obligations with non-positive amounts are rejected by
/// `NettingEngine::add_obligation` (returns `NettingError::InvalidAmount`).
/// The handler catches this error, logs a warning, and increments the
/// `obligations_skipped` counter in the response. The settlement plan
/// is computed from valid obligations only. If ALL obligations are
/// invalid, the handler returns 422.
#[utoipa::path(
    post,
    path = "/v1/corridors/{id}/settlement/compute",
    params(("id" = Uuid, Path, description = "Corridor ID")),
    request_body = SettlementComputeRequest,
    responses(
        (status = 200, description = "Settlement plan computed", body = SettlementPlanResponse),
        (status = 404, description = "Corridor not found", body = crate::error::ErrorBody),
        (status = 422, description = "No valid obligations", body = crate::error::ErrorBody),
    ),
    tag = "settlement"
)]
async fn compute_settlement(
    State(state): State<AppState>,
    Path(corridor_id): Path<Uuid>,
    body: Result<Json<SettlementComputeRequest>, JsonRejection>,
) -> Result<Json<SettlementPlanResponse>, AppError> {
    // Verify the corridor exists.
    let _corridor = state
        .corridors
        .get(&corridor_id)
        .ok_or_else(|| AppError::NotFound(format!("corridor {corridor_id} not found")))?;

    let req = extract_validated_json(body)?;

    let mut engine = NettingEngine::new();
    let mut skipped = 0;

    for ob in &req.obligations {
        match engine.add_obligation(Obligation {
            from_party: ob.from_party.clone(),
            to_party: ob.to_party.clone(),
            amount: ob.amount,
            currency: ob.currency.clone(),
            corridor_id: Some(corridor_id.to_string()),
            priority: ob.priority.unwrap_or(0),
        }) {
            Ok(()) => {}
            Err(e) => {
                tracing::warn!(error = %e, "skipping invalid obligation");
                skipped += 1;
            }
        }
    }

    if engine.obligation_count() == 0 {
        return Err(AppError::Validation("no valid obligations provided".into()));
    }

    let plan = engine
        .compute_plan()
        .map_err(|e| AppError::Internal(format!("netting computation failed: {e}")))?;

    Ok(Json(SettlementPlanResponse {
        corridor_id,
        obligations_processed: engine.obligation_count(),
        obligations_skipped: skipped,
        gross_total: plan.gross_total,
        net_total: plan.net_total,
        reduction_bps: plan.reduction_bps,
        net_positions: plan
            .net_positions
            .iter()
            .map(|np| NetPositionResponse {
                party_id: np.party_id.clone(),
                currency: np.currency.clone(),
                receivable: np.receivable,
                payable: np.payable,
                net: np.net,
            })
            .collect(),
        settlement_legs: plan
            .settlement_legs
            .iter()
            .map(|leg| SettlementLegResponse {
                from_party: leg.from_party.clone(),
                to_party: leg.to_party.clone(),
                amount: leg.amount,
                currency: leg.currency.clone(),
            })
            .collect(),
    }))
}

/// POST /v1/corridors/route — Find the optimal route between jurisdictions.
///
/// Builds the corridor bridge graph from all ACTIVE corridors, runs
/// Dijkstra to find the cheapest path, and returns the route with
/// hop details, total fee, and total settlement time.
///
/// HALTED corridors are excluded — typestate has economic consequences.
#[utoipa::path(
    post,
    path = "/v1/corridors/route",
    request_body = RouteRequest,
    responses(
        (status = 200, description = "Route found", body = RouteResponse),
        (status = 404, description = "No route exists", body = crate::error::ErrorBody),
        (status = 422, description = "Validation error", body = crate::error::ErrorBody),
    ),
    tag = "settlement"
)]
async fn find_route(
    State(state): State<AppState>,
    body: Result<Json<RouteRequest>, JsonRejection>,
) -> Result<Json<RouteResponse>, AppError> {
    let req = extract_validated_json(body)?;

    // Build the bridge graph from all active corridors.
    let mut bridge = CorridorBridge::new();
    let corridors = state.corridors.list();

    let fee_bps = req.default_fee_bps.unwrap_or(10);
    let settlement_secs = req.default_settlement_time_secs.unwrap_or(3600);

    for corridor in &corridors {
        // Only ACTIVE corridors participate in routing.
        if corridor.state != DynCorridorState::Active {
            continue;
        }

        let ja = JurisdictionId::new(&corridor.jurisdiction_a).map_err(|e| {
            AppError::Internal(format!(
                "invalid jurisdiction_a '{}': {e}",
                corridor.jurisdiction_a
            ))
        })?;
        let jb = JurisdictionId::new(&corridor.jurisdiction_b).map_err(|e| {
            AppError::Internal(format!(
                "invalid jurisdiction_b '{}': {e}",
                corridor.jurisdiction_b
            ))
        })?;

        // Add edges in both directions (symmetric corridor).
        bridge.add_edge(BridgeEdge {
            from: ja.clone(),
            to: jb.clone(),
            corridor_id: CorridorId::from_uuid(corridor.id),
            fee_bps,
            settlement_time_secs: settlement_secs,
        });
        bridge.add_edge(BridgeEdge {
            from: jb,
            to: ja,
            corridor_id: CorridorId::from_uuid(corridor.id),
            fee_bps,
            settlement_time_secs: settlement_secs,
        });
    }

    let source = JurisdictionId::new(&req.source)
        .map_err(|e| AppError::Validation(format!("invalid source jurisdiction: {e}")))?;
    let target = JurisdictionId::new(&req.target)
        .map_err(|e| AppError::Validation(format!("invalid target jurisdiction: {e}")))?;

    let route = bridge.find_route(&source, &target).ok_or_else(|| {
        AppError::NotFound(format!(
            "no route from {} to {} — jurisdictions are not connected by active corridors",
            req.source, req.target
        ))
    })?;

    Ok(Json(RouteResponse {
        source: req.source,
        target: req.target,
        hop_count: route.hop_count(),
        total_fee_bps: route.total_fee_bps,
        total_settlement_time_secs: route.total_settlement_time_secs,
        hops: route
            .hops
            .iter()
            .map(|hop| RouteHopResponse {
                from: hop.from.to_string(),
                to: hop.to.to_string(),
                corridor_id: hop.corridor_id.to_string(),
                fee_bps: hop.fee_bps,
                settlement_time_secs: hop.settlement_time_secs,
            })
            .collect(),
    }))
}

/// POST /v1/corridors/:id/settlement/instruct — Generate SWIFT pacs.008 messages.
///
/// Takes settlement legs and generates ISO 20022 pacs.008 XML payment
/// instructions for each leg. Each instruction is a bank-executable
/// SWIFT message.
#[utoipa::path(
    post,
    path = "/v1/corridors/{id}/settlement/instruct",
    params(("id" = Uuid, Path, description = "Corridor ID")),
    request_body = InstructionRequest,
    responses(
        (status = 200, description = "Instructions generated", body = InstructionResponse),
        (status = 404, description = "Corridor not found", body = crate::error::ErrorBody),
        (status = 422, description = "Validation error", body = crate::error::ErrorBody),
    ),
    tag = "settlement"
)]
async fn generate_instructions(
    State(state): State<AppState>,
    Path(corridor_id): Path<Uuid>,
    body: Result<Json<InstructionRequest>, JsonRejection>,
) -> Result<Json<InstructionResponse>, AppError> {
    let req = extract_validated_json(body)?;

    // Verify the corridor exists.
    let _corridor = state
        .corridors
        .get(&corridor_id)
        .ok_or_else(|| AppError::NotFound(format!("corridor {corridor_id} not found")))?;

    let adapter = SwiftPacs008::new(req.instructing_agent_bic.as_deref().unwrap_or("MEZUS33"));

    let mut instructions = Vec::new();
    let mut errors = Vec::new();

    for (i, leg) in req.legs.iter().enumerate() {
        let msg_id = format!(
            "MEZ-{}-{:03}",
            corridor_id.to_string().split('-').next().unwrap_or("????"),
            i
        );

        let instruction = SettlementInstruction {
            message_id: msg_id,
            debtor_name: leg.from_party.clone(),
            debtor_bic: leg.from_bic.clone(),
            debtor_account: leg.from_account.clone().unwrap_or_default(),
            creditor_name: leg.to_party.clone(),
            creditor_bic: leg.to_bic.clone(),
            creditor_account: leg.to_account.clone().unwrap_or_default(),
            amount: leg.amount,
            currency: leg.currency.clone(),
            remittance_info: Some(format!("MEZ corridor {} settlement", corridor_id)),
        };

        match adapter.generate_instruction(&instruction) {
            Ok(xml) => {
                instructions.push(Pacs008Output {
                    leg_index: i,
                    from_party: leg.from_party.clone(),
                    to_party: leg.to_party.clone(),
                    amount: leg.amount,
                    currency: leg.currency.clone(),
                    xml,
                });
            }
            Err(e) => {
                errors.push(format!("leg {i}: {e}"));
            }
        }
    }

    Ok(Json(InstructionResponse {
        corridor_id,
        instructions_generated: instructions.len(),
        errors: if errors.is_empty() {
            None
        } else {
            Some(errors)
        },
        instructions,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, CorridorRecord};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use chrono::Utc;
    use http_body_util::BodyExt;
    use mez_corridor::ReceiptChain;
    use mez_state::DynCorridorState;
    use tower::ServiceExt;

    /// Helper: read the response body as JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    /// Helper: build a test app with the settlement router.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: create a corridor directly in AppState and return its UUID.
    fn create_corridor_in_state(
        state: &AppState,
        jurisdiction_a: &str,
        jurisdiction_b: &str,
        corridor_state: DynCorridorState,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let record = CorridorRecord {
            id,
            jurisdiction_a: jurisdiction_a.to_string(),
            jurisdiction_b: jurisdiction_b.to_string(),
            state: corridor_state,
            transition_log: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        state.corridors.insert(id, record);
        // Initialize receipt chain with genesis root derived from corridor ID.
        let genesis_payload = serde_json::json!({"corridor_genesis": id.to_string()});
        let canonical = mez_core::CanonicalBytes::new(&genesis_payload).unwrap();
        let genesis_root = mez_core::sha256_digest(&canonical);
        let chain = ReceiptChain::new(CorridorId::from_uuid(id), genesis_root);
        state.receipt_chains.write().insert(id, chain);
        id
    }

    // ── Settlement computation tests ─────────────────────────────

    #[tokio::test]
    async fn bilateral_netting_through_api() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "obligations": [
                { "from_party": "acme-corp", "to_party": "gulf-trading", "amount": 5000000, "currency": "USD" },
                { "from_party": "acme-corp", "to_party": "gulf-trading", "amount": 3000000, "currency": "USD" },
                { "from_party": "gulf-trading", "to_party": "acme-corp", "amount": 4500000, "currency": "USD" }
            ]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/compute"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let plan: serde_json::Value = body_json(resp).await;

        // Gross: 5M + 3M + 4.5M = 12.5M
        assert_eq!(plan["gross_total"], 12_500_000);
        // Net: Acme owes Gulf 3.5M (8M - 4.5M)
        assert_eq!(plan["settlement_legs"].as_array().unwrap().len(), 1);
        assert_eq!(plan["settlement_legs"][0]["amount"], 3_500_000);
        // Reduction: 1 - (3.5M / 12.5M) = 72%
        assert!(plan["reduction_bps"].as_u64().unwrap() > 7000);
        // Processed count
        assert_eq!(plan["obligations_processed"], 3);
        assert_eq!(plan["obligations_skipped"], 0);
    }

    #[tokio::test]
    async fn settlement_compute_nonexistent_corridor_returns_404() {
        let app = test_app();
        let fake_id = Uuid::new_v4();
        let body = serde_json::json!({
            "obligations": [
                { "from_party": "A", "to_party": "B", "amount": 100, "currency": "USD" }
            ]
        });

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{fake_id}/settlement/compute"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn settlement_compute_no_obligations_returns_422() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({ "obligations": [] });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/compute"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn settlement_compute_skips_invalid_obligations() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        // One valid obligation and one with negative amount (invalid).
        let body = serde_json::json!({
            "obligations": [
                { "from_party": "A", "to_party": "B", "amount": 1000, "currency": "USD" },
                { "from_party": "C", "to_party": "D", "amount": -500, "currency": "USD" }
            ]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/compute"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let plan: serde_json::Value = body_json(resp).await;
        assert_eq!(plan["obligations_processed"], 1);
        assert_eq!(plan["obligations_skipped"], 1);
    }

    #[tokio::test]
    async fn perfectly_balanced_settlement() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "obligations": [
                { "from_party": "A", "to_party": "B", "amount": 1000, "currency": "USD" },
                { "from_party": "B", "to_party": "A", "amount": 1000, "currency": "USD" }
            ]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/compute"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let plan: serde_json::Value = body_json(resp).await;
        assert_eq!(plan["net_total"], 0);
        assert_eq!(plan["settlement_legs"].as_array().unwrap().len(), 0);
        assert_eq!(plan["reduction_bps"].as_u64().unwrap(), 10_000);
    }

    // ── Bridge routing tests ────────────────────────────────────

    #[tokio::test]
    async fn route_through_active_corridors() {
        let state = AppState::new();

        // Create three corridors forming a path: PK → AE → KZ
        create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);
        create_corridor_in_state(&state, "ae-difc", "kz-aifc", DynCorridorState::Active);
        // Create a HALTED corridor that should NOT participate in routing.
        create_corridor_in_state(&state, "pk-ez-01", "kz-aifc", DynCorridorState::Halted);

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "kz-aifc"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let route: serde_json::Value = body_json(resp).await;
        // Should route through AE (2 hops) because the direct PK→KZ corridor is HALTED.
        assert_eq!(route["hop_count"], 2);
        // First hop: PK → AE
        assert_eq!(route["hops"][0]["from"], "pk-ez-01");
        assert_eq!(route["hops"][0]["to"], "ae-difc");
        // Second hop: AE → KZ
        assert_eq!(route["hops"][1]["from"], "ae-difc");
        assert_eq!(route["hops"][1]["to"], "kz-aifc");
    }

    #[tokio::test]
    async fn no_route_returns_404() {
        let state = AppState::new();
        // No corridors exist — no route is possible.

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "kz-aifc"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn route_same_source_target_returns_422() {
        let app = test_app();

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "pk-ez-01"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn route_empty_source_returns_422() {
        let app = test_app();

        let body = serde_json::json!({
            "source": "",
            "target": "kz-aifc"
        });

        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn halted_corridors_excluded_from_routing() {
        // This test proves that the typestate machine's HALTED state
        // has economic consequences: halted corridors cannot be used for routing.
        let state = AppState::new();

        // Only create HALTED corridors — no route should be possible.
        create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Halted);

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "ae-difc"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn suspended_corridors_excluded_from_routing() {
        let state = AppState::new();
        create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Suspended);

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "ae-difc"
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn route_with_custom_fee_and_time() {
        let state = AppState::new();
        create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "source": "pk-ez-01",
            "target": "ae-difc",
            "default_fee_bps": 50,
            "default_settlement_time_secs": 7200
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri("/v1/corridors/route")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let route: serde_json::Value = body_json(resp).await;
        assert_eq!(route["hop_count"], 1);
        assert_eq!(route["total_fee_bps"], 50);
        assert_eq!(route["total_settlement_time_secs"], 7200);
    }

    // ── SWIFT instruction tests ─────────────────────────────────

    #[tokio::test]
    async fn swift_instruction_generation() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "legs": [{
                "from_party": "Acme Corp",
                "from_bic": "ACMEUSXX",
                "to_party": "Gulf Trading",
                "to_bic": "GULFAEXX",
                "amount": 3500000,
                "currency": "USD"
            }]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: serde_json::Value = body_json(resp).await;
        assert_eq!(result["instructions_generated"], 1);
        assert!(result["errors"].is_null());

        // The XML should be valid pacs.008 structure.
        let xml = result["instructions"][0]["xml"].as_str().unwrap();
        assert!(xml.contains("FIToFICstmrCdtTrf"));
        assert!(xml.contains("ACMEUSXX"));
        assert!(xml.contains("GULFAEXX"));
        assert!(xml.contains("35000.00")); // 3500000 cents → 35000.00
        assert!(xml.contains("USD"));
    }

    #[tokio::test]
    async fn swift_instruction_nonexistent_corridor_returns_404() {
        let app = test_app();
        let fake_id = Uuid::new_v4();

        let body = serde_json::json!({
            "legs": [{
                "from_party": "A",
                "from_bic": "ABCDUSXX",
                "to_party": "B",
                "to_bic": "EFGHAEXX",
                "amount": 1000,
                "currency": "USD"
            }]
        });

        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{fake_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn swift_instruction_empty_legs_returns_422() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({ "legs": [] });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn swift_instruction_invalid_bic_reports_error() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "legs": [{
                "from_party": "A",
                "from_bic": "BAD",
                "to_party": "B",
                "to_bic": "GULFAEXX",
                "amount": 1000,
                "currency": "USD"
            }]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: serde_json::Value = body_json(resp).await;
        assert_eq!(result["instructions_generated"], 0);
        assert!(result["errors"].is_array());
        let errors = result["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].as_str().unwrap().contains("BIC"));
    }

    #[tokio::test]
    async fn swift_instruction_multiple_legs() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "legs": [
                {
                    "from_party": "Acme Corp",
                    "from_bic": "ACMEUSXX",
                    "to_party": "Gulf Trading",
                    "to_bic": "GULFAEXX",
                    "amount": 5000000,
                    "currency": "USD"
                },
                {
                    "from_party": "Gulf Trading",
                    "from_bic": "GULFAEXX",
                    "to_party": "KZ Holdings",
                    "to_bic": "KZHDKZXX",
                    "amount": 2000000,
                    "currency": "USD"
                }
            ]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: serde_json::Value = body_json(resp).await;
        assert_eq!(result["instructions_generated"], 2);
        assert!(result["errors"].is_null());
        assert_eq!(result["instructions"][0]["leg_index"], 0);
        assert_eq!(result["instructions"][1]["leg_index"], 1);
    }

    #[tokio::test]
    async fn swift_instruction_custom_bic() {
        let state = AppState::new();
        let corridor_id =
            create_corridor_in_state(&state, "pk-ez-01", "ae-difc", DynCorridorState::Active);

        let body = serde_json::json!({
            "instructing_agent_bic": "MEZPKXX",
            "legs": [{
                "from_party": "Acme Corp",
                "from_bic": "ACMEUSXX",
                "to_party": "Gulf Trading",
                "to_bic": "GULFAEXX",
                "amount": 1000,
                "currency": "USD"
            }]
        });

        let app = router().with_state(state);
        let req = Request::builder()
            .method("POST")
            .uri(format!("/v1/corridors/{corridor_id}/settlement/instruct"))
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: serde_json::Value = body_json(resp).await;
        let xml = result["instructions"][0]["xml"].as_str().unwrap();
        assert!(xml.contains("MEZPKXX"));
    }

    // ── Router construction ─────────────────────────────────────

    #[test]
    fn test_settlement_router_builds_successfully() {
        let _router = router();
    }

    // ── Validation unit tests ───────────────────────────────────

    #[test]
    fn route_request_validates_non_empty() {
        let req = RouteRequest {
            source: "".to_string(),
            target: "ae-difc".to_string(),
            default_fee_bps: None,
            default_settlement_time_secs: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn route_request_validates_different_source_target() {
        let req = RouteRequest {
            source: "pk-ez-01".to_string(),
            target: "pk-ez-01".to_string(),
            default_fee_bps: None,
            default_settlement_time_secs: None,
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("differ"));
    }

    #[test]
    fn instruction_request_validates_non_empty_legs() {
        let req = InstructionRequest {
            instructing_agent_bic: None,
            legs: vec![],
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("at least one"));
    }
}
