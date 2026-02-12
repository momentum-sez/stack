//! # FISCAL Primitive — Treasury Info API
//!
//! Handles treasury accounts, payments, withholding calculation,
//! tax event history, and reporting generation.
//! Critical for FBR IRIS integration with NTN as first-class identifier.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;
use crate::extractors::{extract_validated_json, Validate};
use crate::state::{AppState, FiscalAccountRecord, PaymentRecord, TaxEventRecord};
use axum::extract::rejection::JsonRejection;

/// Request to create a fiscal/treasury account.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    pub entity_id: Uuid,
    pub account_type: String,
    pub currency: String,
    /// NTN (National Tax Number) for FBR IRIS integration.
    pub ntn: Option<String>,
}

impl Validate for CreateAccountRequest {
    fn validate(&self) -> Result<(), String> {
        if self.account_type.trim().is_empty() {
            return Err("account_type must not be empty".to_string());
        }
        if self.currency.trim().is_empty() {
            return Err("currency must not be empty".to_string());
        }
        if let Some(ref ntn) = self.ntn {
            if ntn.len() != 7 || !ntn.chars().all(|c| c.is_ascii_digit()) {
                return Err("NTN must be exactly 7 digits".to_string());
            }
        }
        Ok(())
    }
}

/// Request to initiate a payment.
#[derive(Debug, Deserialize, ToSchema)]
pub struct InitiatePaymentRequest {
    pub from_account_id: Uuid,
    pub to_account_id: Option<Uuid>,
    pub amount: String,
    pub currency: String,
    pub reference: String,
}

impl Validate for InitiatePaymentRequest {
    fn validate(&self) -> Result<(), String> {
        if self.amount.trim().is_empty() {
            return Err("amount must not be empty".to_string());
        }
        Ok(())
    }
}

/// Withholding calculation request.
#[derive(Debug, Deserialize, ToSchema)]
pub struct WithholdingCalculateRequest {
    pub entity_id: Uuid,
    pub amount: String,
    pub income_type: String,
}

impl Validate for WithholdingCalculateRequest {
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

/// Withholding calculation response.
#[derive(Debug, Serialize, ToSchema)]
pub struct WithholdingResponse {
    pub gross_amount: String,
    pub withholding_rate: String,
    pub withholding_amount: String,
    pub net_amount: String,
}

/// Build the fiscal router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/fiscal/accounts", post(create_account))
        .route("/v1/fiscal/payments", post(initiate_payment))
        .route(
            "/v1/fiscal/withholding/calculate",
            post(calculate_withholding),
        )
        .route("/v1/fiscal/:entity_id/tax-events", get(get_tax_events))
        .route("/v1/fiscal/reporting/generate", post(generate_report))
}

/// POST /v1/fiscal/accounts — Create a treasury account.
#[utoipa::path(
    post,
    path = "/v1/fiscal/accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Account created", body = FiscalAccountRecord),
    ),
    tag = "fiscal"
)]
async fn create_account(
    State(state): State<AppState>,
    body: Result<Json<CreateAccountRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<FiscalAccountRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let now = Utc::now();
    let id = Uuid::new_v4();

    let record = FiscalAccountRecord {
        id,
        entity_id: req.entity_id,
        account_type: req.account_type,
        currency: req.currency,
        balance: "0".to_string(),
        ntn: req.ntn,
        created_at: now,
        updated_at: now,
    };

    state.fiscal_accounts.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// POST /v1/fiscal/payments — Initiate a payment.
#[utoipa::path(
    post,
    path = "/v1/fiscal/payments",
    request_body = InitiatePaymentRequest,
    responses(
        (status = 201, description = "Payment initiated", body = PaymentRecord),
    ),
    tag = "fiscal"
)]
async fn initiate_payment(
    State(state): State<AppState>,
    body: Result<Json<InitiatePaymentRequest>, JsonRejection>,
) -> Result<(axum::http::StatusCode, Json<PaymentRecord>), AppError> {
    let req = extract_validated_json(body)?;
    let id = Uuid::new_v4();

    let record = PaymentRecord {
        id,
        from_account_id: req.from_account_id,
        to_account_id: req.to_account_id,
        amount: req.amount,
        currency: req.currency,
        reference: req.reference,
        status: "PENDING".to_string(),
        created_at: Utc::now(),
    };

    state.payments.insert(id, record.clone());
    Ok((axum::http::StatusCode::CREATED, Json(record)))
}

/// POST /v1/fiscal/withholding/calculate — Compute withholding.
#[utoipa::path(
    post,
    path = "/v1/fiscal/withholding/calculate",
    request_body = WithholdingCalculateRequest,
    responses(
        (status = 200, description = "Withholding calculated", body = WithholdingResponse),
    ),
    tag = "fiscal"
)]
async fn calculate_withholding(
    State(_state): State<AppState>,
    body: Result<Json<WithholdingCalculateRequest>, JsonRejection>,
) -> Result<Json<WithholdingResponse>, AppError> {
    let req = extract_validated_json(body)?;
    // Phase 1 stub: fixed 15% withholding rate.
    let rate = "0.15";
    Ok(Json(WithholdingResponse {
        gross_amount: req.amount.clone(),
        withholding_rate: rate.to_string(),
        withholding_amount: format!("stub:{rate}*{}", req.amount),
        net_amount: format!("stub:{}*(1-{rate})", req.amount),
    }))
}

/// GET /v1/fiscal/:entity_id/tax-events — Get tax event history.
#[utoipa::path(
    get,
    path = "/v1/fiscal/{entity_id}/tax-events",
    params(("entity_id" = Uuid, Path, description = "Entity ID")),
    responses(
        (status = 200, description = "Tax events", body = Vec<TaxEventRecord>),
    ),
    tag = "fiscal"
)]
async fn get_tax_events(
    State(state): State<AppState>,
    Path(entity_id): Path<Uuid>,
) -> Json<Vec<TaxEventRecord>> {
    let events: Vec<_> = state
        .tax_events
        .list()
        .into_iter()
        .filter(|e| e.entity_id == entity_id)
        .collect();
    Json(events)
}

/// POST /v1/fiscal/reporting/generate — Generate tax return data.
#[utoipa::path(
    post,
    path = "/v1/fiscal/reporting/generate",
    responses(
        (status = 200, description = "Report generated"),
    ),
    tag = "fiscal"
)]
async fn generate_report(State(_state): State<AppState>) -> Json<serde_json::Value> {
    // Phase 1 stub.
    Json(serde_json::json!({
        "status": "generated",
        "message": "Tax reporting generation is a Phase 2 feature"
    }))
}
