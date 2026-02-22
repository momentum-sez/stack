// SPDX-License-Identifier: BUSL-1.1
//! # Trade Flow API Endpoints
//!
//! Provides REST endpoints for trade corridor instrument lifecycle management.
//! Write operations follow the orchestration pipeline: compliance evaluation →
//! execute → VC issuance → attestation → audit trail.
//!
//! | Method | Path | Handler |
//! |--------|------|---------|
//! | `POST` | `/v1/trade/flows` | `create_trade_flow` |
//! | `GET` | `/v1/trade/flows` | `list_trade_flows` |
//! | `GET` | `/v1/trade/flows/:flow_id` | `get_trade_flow` |
//! | `POST` | `/v1/trade/flows/:flow_id/transitions` | `submit_transition` |
//! | `GET` | `/v1/trade/flows/:flow_id/transitions` | `list_transitions` |

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use mez_core::ComplianceDomain;
use mez_corridor::{TradeFlowType, TradeParty, TradeTransitionPayload};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::db;
use crate::error::AppError;
use crate::orchestration::{self, ComplianceSummary};
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

/// Request to create a new trade flow.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateTradeFlowRequest {
    /// Flow type: "export", "import", "letter_of_credit", or "open_account".
    #[schema(value_type = String)]
    pub flow_type: TradeFlowType,
    /// Seller party identification.
    #[schema(value_type = Object)]
    pub seller: TradeParty,
    /// Buyer party identification.
    #[schema(value_type = Object)]
    pub buyer: TradeParty,
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
}

/// Request to submit a transition to a trade flow.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SubmitTransitionRequest {
    /// Transition payload specific to the transition type.
    #[schema(value_type = Object)]
    pub payload: TradeTransitionPayload,
    #[serde(default)]
    pub jurisdiction_id: Option<String>,
}

/// Response envelope for trade flow operations.
#[derive(Debug, Serialize, ToSchema)]
pub struct TradeFlowResponse {
    pub flow: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compliance: Option<ComplianceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attestation_id: Option<Uuid>,
}

// ---------------------------------------------------------------------------
// Compliance domains relevant to trade operations
// ---------------------------------------------------------------------------

const TRADE_DOMAINS: &[ComplianceDomain] = &[
    ComplianceDomain::Aml,
    ComplianceDomain::Kyc,
    ComplianceDomain::Sanctions,
    ComplianceDomain::Tax,
];

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

/// Build the trade flow router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/trade/flows", post(create_trade_flow).get(list_trade_flows))
        .route("/v1/trade/flows/:flow_id", get(get_trade_flow))
        .route(
            "/v1/trade/flows/:flow_id/transitions",
            post(submit_transition).get(list_transitions),
        )
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// POST /v1/trade/flows — Create a new trade flow.
#[utoipa::path(
    post,
    path = "/v1/trade/flows",
    request_body = CreateTradeFlowRequest,
    responses(
        (status = 201, description = "Trade flow created", body = TradeFlowResponse),
        (status = 403, description = "Compliance hard-block", body = crate::error::ErrorBody),
    ),
    tag = "trade"
)]
async fn create_trade_flow(
    State(state): State<AppState>,
    Json(req): Json<CreateTradeFlowRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.seller.party_id.trim().is_empty() {
        return Err(AppError::Validation("seller.party_id must not be empty".to_string()));
    }
    if req.buyer.party_id.trim().is_empty() {
        return Err(AppError::Validation("buyer.party_id must not be empty".to_string()));
    }
    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("pk-sifc");

    // Pre-flight compliance evaluation.
    let (_tensor, summary) = orchestration::evaluate_compliance(
        jurisdiction,
        &req.seller.party_id,
        TRADE_DOMAINS,
    );
    if let Some(reason) = orchestration::check_hard_blocks(&summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Create the flow.
    let record = state.trade_flow_manager.create_flow(req.flow_type, req.seller, req.buyer);
    let flow_id = record.flow_id;

    let flow_value = serde_json::to_value(&record)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Issue VC + store attestation.
    let (credential, attestation_id) = issue_trade_vc_and_attestation(
        &state,
        jurisdiction,
        &flow_id.to_string(),
        "trade_flow_creation",
        &summary,
    );

    // Persist to DB.
    if let Some(ref pool) = state.db_pool {
        if let Err(e) = db::trade::save_trade_flow(pool, &record).await {
            tracing::error!(error = %e, flow_id = %flow_id, "failed to persist trade flow to database");
            return Err(AppError::Internal(format!("failed to persist trade flow: {e}")));
        }
    }

    // Audit trail.
    if let Some(ref pool) = state.db_pool {
        if let Err(e) = db::audit::append(
            pool,
            db::audit::AuditEvent {
                event_type: "trade.flow.created".to_string(),
                actor_did: Some(state.zone_did.clone()),
                resource_type: "trade_flow".to_string(),
                resource_id: flow_id,
                action: "create".to_string(),
                metadata: serde_json::json!({
                    "flow_type": record.flow_type,
                    "state": record.state,
                }),
            },
        )
        .await
        {
            tracing::warn!(error = %e, flow_id = %flow_id, "failed to append audit event for trade flow creation");
        }
    }

    let response = TradeFlowResponse {
        flow: flow_value,
        compliance: Some(summary),
        credential,
        attestation_id,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /v1/trade/flows — List all trade flows.
#[utoipa::path(
    get,
    path = "/v1/trade/flows",
    responses(
        (status = 200, description = "List of trade flows"),
    ),
    tag = "trade"
)]
async fn list_trade_flows(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let flows = state.trade_flow_manager.list_flows();
    let values: Vec<serde_json::Value> = flows
        .iter()
        .map(|f| {
            serde_json::to_value(f).map_err(|e| {
                tracing::warn!(error = %e, "failed to serialize trade flow record");
                e
            })
        })
        .filter_map(Result::ok)
        .collect();
    Ok(Json(serde_json::json!({ "flows": values, "total": values.len() })))
}

/// GET /v1/trade/flows/:flow_id — Get a trade flow by ID.
#[utoipa::path(
    get,
    path = "/v1/trade/flows/{flow_id}",
    params(("flow_id" = Uuid, Path, description = "Trade flow UUID")),
    responses(
        (status = 200, description = "Trade flow details"),
        (status = 404, description = "Trade flow not found", body = crate::error::ErrorBody),
    ),
    tag = "trade"
)]
async fn get_trade_flow(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .trade_flow_manager
        .get_flow(&flow_id)
        .ok_or_else(|| AppError::NotFound(format!("trade flow {flow_id} not found")))?;

    let value = serde_json::to_value(&record)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    Ok(Json(value))
}

/// POST /v1/trade/flows/:flow_id/transitions — Submit a transition.
#[utoipa::path(
    post,
    path = "/v1/trade/flows/{flow_id}/transitions",
    params(("flow_id" = Uuid, Path, description = "Trade flow UUID")),
    request_body = SubmitTransitionRequest,
    responses(
        (status = 200, description = "Transition applied", body = TradeFlowResponse),
        (status = 404, description = "Trade flow not found", body = crate::error::ErrorBody),
        (status = 403, description = "Compliance hard-block", body = crate::error::ErrorBody),
        (status = 422, description = "Invalid transition", body = crate::error::ErrorBody),
    ),
    tag = "trade"
)]
async fn submit_transition(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
    Json(req): Json<SubmitTransitionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("pk-sifc");

    // Pre-flight compliance evaluation.
    let (_tensor, summary) = orchestration::evaluate_compliance(
        jurisdiction,
        "trade-transition",
        TRADE_DOMAINS,
    );
    if let Some(reason) = orchestration::check_hard_blocks(&summary) {
        return Err(AppError::Forbidden(reason));
    }

    // Execute transition.
    let record = state
        .trade_flow_manager
        .submit_transition(flow_id, req.payload)
        .map_err(|e| match &e {
            mez_corridor::TradeError::NotFound(_) => {
                AppError::NotFound(format!("trade flow {flow_id} not found"))
            }
            mez_corridor::TradeError::InvalidTransition { .. } => {
                AppError::Validation(e.to_string())
            }
            _ => AppError::Internal(e.to_string()),
        })?;

    let flow_value = serde_json::to_value(&record)
        .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?;

    // Issue VC + store attestation.
    let transition_kind = record
        .transitions
        .last()
        .map(|t| t.kind.as_str())
        .unwrap_or("unknown");
    let (credential, attestation_id) = issue_trade_vc_and_attestation(
        &state,
        jurisdiction,
        &flow_id.to_string(),
        transition_kind,
        &summary,
    );

    // Persist to DB.
    if let Some(ref pool) = state.db_pool {
        if let Err(e) = db::trade::save_trade_flow(pool, &record).await {
            tracing::error!(error = %e, flow_id = %flow_id, "failed to persist trade flow transition to database");
            return Err(AppError::Internal(format!("failed to persist trade flow transition: {e}")));
        }
    }

    // Audit trail.
    if let Some(ref pool) = state.db_pool {
        if let Err(e) = db::audit::append(
            pool,
            db::audit::AuditEvent {
                event_type: format!("trade.transition.{transition_kind}"),
                actor_did: Some(state.zone_did.clone()),
                resource_type: "trade_flow".to_string(),
                resource_id: flow_id,
                action: "transition".to_string(),
                metadata: serde_json::json!({
                    "flow_type": record.flow_type,
                    "from_state": record.transitions.last().map(|t| format!("{}", t.from_state)),
                    "to_state": record.state,
                }),
            },
        )
        .await
        {
            tracing::warn!(error = %e, flow_id = %flow_id, "failed to append audit event for trade transition");
        }
    }

    let response = TradeFlowResponse {
        flow: flow_value,
        compliance: Some(summary),
        credential,
        attestation_id,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// GET /v1/trade/flows/:flow_id/transitions — List transitions for a flow.
#[utoipa::path(
    get,
    path = "/v1/trade/flows/{flow_id}/transitions",
    params(("flow_id" = Uuid, Path, description = "Trade flow UUID")),
    responses(
        (status = 200, description = "List of transitions for the flow"),
        (status = 404, description = "Trade flow not found", body = crate::error::ErrorBody),
    ),
    tag = "trade"
)]
async fn list_transitions(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let record = state
        .trade_flow_manager
        .get_flow(&flow_id)
        .ok_or_else(|| AppError::NotFound(format!("trade flow {flow_id} not found")))?;

    let transitions: Vec<serde_json::Value> = record
        .transitions
        .iter()
        .map(|t| {
            serde_json::to_value(t).map_err(|e| {
                tracing::warn!(error = %e, "failed to serialize trade transition record");
                e
            })
        })
        .filter_map(Result::ok)
        .collect();

    Ok(Json(serde_json::json!({
        "flow_id": flow_id,
        "transitions": transitions,
        "total": transitions.len(),
    })))
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// VC type for trade compliance.
const TRADE_COMPLIANCE_VC_TYPE: &str = "MezTradeComplianceCredential";

/// Issue a trade compliance VC and store an attestation record.
fn issue_trade_vc_and_attestation(
    state: &AppState,
    jurisdiction_id: &str,
    entity_reference: &str,
    description: &str,
    summary: &ComplianceSummary,
) -> (Option<serde_json::Value>, Option<Uuid>) {
    let credential = match orchestration::issue_compliance_vc(
        state,
        TRADE_COMPLIANCE_VC_TYPE,
        jurisdiction_id,
        entity_reference,
        summary,
    ) {
        Ok(vc) => match serde_json::to_value(&vc) {
            Ok(v) => Some(v),
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialize trade compliance VC");
                None
            }
        },
        Err(e) => {
            tracing::warn!(error = %e, "failed to issue trade compliance VC");
            None
        }
    };

    let entity_uuid = Uuid::parse_str(entity_reference).unwrap_or_else(|_| {
        let fallback = Uuid::new_v4();
        tracing::warn!(
            entity_reference,
            fallback_id = %fallback,
            "entity reference is not a valid UUID — using generated fallback for trade attestation"
        );
        fallback
    });

    let attestation_id = orchestration::store_attestation(
        state,
        entity_uuid,
        &format!("{TRADE_COMPLIANCE_VC_TYPE}:{description}"),
        jurisdiction_id,
        serde_json::json!({
            "operation": TRADE_COMPLIANCE_VC_TYPE,
            "overall_status": summary.overall_status,
            "blocking_domains": summary.blocking_domains,
        }),
    );

    (credential, Some(attestation_id))
}
