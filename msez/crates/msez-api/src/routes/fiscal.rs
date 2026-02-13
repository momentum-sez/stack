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
#[derive(Debug, Serialize, Deserialize, ToSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extractors::Validate;

    // ── CreateAccountRequest validation ────────────────────────────

    #[test]
    fn test_create_account_request_valid() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "PKR".to_string(),
            ntn: None,
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_account_request_valid_with_ntn() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "PKR".to_string(),
            ntn: Some("1234567".to_string()),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_create_account_request_empty_account_type() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "".to_string(),
            currency: "PKR".to_string(),
            ntn: None,
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("account_type"),
            "error should mention account_type: {err}"
        );
    }

    #[test]
    fn test_create_account_request_whitespace_account_type() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "   ".to_string(),
            currency: "PKR".to_string(),
            ntn: None,
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_create_account_request_empty_currency() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "".to_string(),
            ntn: None,
        };
        let err = req.validate().unwrap_err();
        assert!(
            err.contains("currency"),
            "error should mention currency: {err}"
        );
    }

    #[test]
    fn test_create_account_request_ntn_wrong_length() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "PKR".to_string(),
            ntn: Some("123".to_string()),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("NTN"), "error should mention NTN: {err}");
    }

    #[test]
    fn test_create_account_request_ntn_non_digits() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "PKR".to_string(),
            ntn: Some("123abc7".to_string()),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("NTN"), "error should mention NTN: {err}");
    }

    #[test]
    fn test_create_account_request_ntn_too_long() {
        let req = CreateAccountRequest {
            entity_id: Uuid::new_v4(),
            account_type: "treasury".to_string(),
            currency: "PKR".to_string(),
            ntn: Some("12345678".to_string()),
        };
        assert!(req.validate().is_err());
    }

    // ── InitiatePaymentRequest validation ─────────────────────────

    #[test]
    fn test_initiate_payment_request_valid() {
        let req = InitiatePaymentRequest {
            from_account_id: Uuid::new_v4(),
            to_account_id: Some(Uuid::new_v4()),
            amount: "1000.00".to_string(),
            currency: "PKR".to_string(),
            reference: "INV-001".to_string(),
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_initiate_payment_request_empty_amount() {
        let req = InitiatePaymentRequest {
            from_account_id: Uuid::new_v4(),
            to_account_id: None,
            amount: "".to_string(),
            currency: "PKR".to_string(),
            reference: "INV-001".to_string(),
        };
        let err = req.validate().unwrap_err();
        assert!(err.contains("amount"), "error should mention amount: {err}");
    }

    #[test]
    fn test_initiate_payment_request_whitespace_amount() {
        let req = InitiatePaymentRequest {
            from_account_id: Uuid::new_v4(),
            to_account_id: None,
            amount: "   ".to_string(),
            currency: "PKR".to_string(),
            reference: "INV-001".to_string(),
        };
        assert!(req.validate().is_err());
    }

    // ── WithholdingCalculateRequest validation ────────────────────

    #[test]
    fn test_withholding_calculate_request_valid() {
        let req = WithholdingCalculateRequest {
            entity_id: Uuid::new_v4(),
            amount: "50000".to_string(),
            income_type: "dividend".to_string(),
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

    /// Helper: build the fiscal router with a fresh AppState.
    fn test_app() -> Router<()> {
        router().with_state(AppState::new())
    }

    /// Helper: read the response body as bytes and deserialize from JSON.
    async fn body_json<T: serde::de::DeserializeOwned>(resp: axum::response::Response) -> T {
        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn handler_create_account_returns_201() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","account_type":"treasury","currency":"PKR","ntn":"1234567"}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: FiscalAccountRecord = body_json(resp).await;
        assert_eq!(record.entity_id, entity_id);
        assert_eq!(record.account_type, "treasury");
        assert_eq!(record.currency, "PKR");
        assert_eq!(record.ntn.as_deref(), Some("1234567"));
        assert_eq!(record.balance, "0");
    }

    #[tokio::test]
    async fn handler_create_account_invalid_ntn_returns_422() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","account_type":"treasury","currency":"PKR","ntn":"123"}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_create_account_empty_currency_returns_422() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","account_type":"treasury","currency":""}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_initiate_payment_returns_201() {
        let app = test_app();
        let from_id = Uuid::new_v4();
        let to_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"from_account_id":"{}","to_account_id":"{}","amount":"5000.00","currency":"PKR","reference":"INV-001"}}"#,
            from_id, to_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/payments")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: PaymentRecord = body_json(resp).await;
        assert_eq!(record.from_account_id, from_id);
        assert_eq!(record.amount, "5000.00");
        assert_eq!(record.currency, "PKR");
        assert_eq!(record.status, "PENDING");
    }

    #[tokio::test]
    async fn handler_initiate_payment_empty_amount_returns_422() {
        let app = test_app();
        let from_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"from_account_id":"{}","amount":"","currency":"PKR","reference":"INV-001"}}"#,
            from_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/payments")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_initiate_payment_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/payments")
            .header("content-type", "application/json")
            .body(Body::from(r#"not json"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    // ── Additional handler coverage ───────────────────────────────

    #[tokio::test]
    async fn handler_calculate_withholding_returns_200() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","amount":"100000","income_type":"dividend"}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/withholding/calculate")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let result: WithholdingResponse = body_json(resp).await;
        assert_eq!(result.gross_amount, "100000");
        assert_eq!(result.withholding_rate, "0.15");
        assert!(result.withholding_amount.contains("0.15"));
        assert!(result.net_amount.contains("100000"));
    }

    #[tokio::test]
    async fn handler_calculate_withholding_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/withholding/calculate")
            .header("content-type", "application/json")
            .body(Body::from(r#"broken"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_get_tax_events_empty_returns_200() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/fiscal/{entity_id}/tax-events"))
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let events: Vec<TaxEventRecord> = body_json(resp).await;
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn handler_get_tax_events_filters_by_entity() {
        let state = AppState::new();
        let entity_a = Uuid::new_v4();
        let entity_b = Uuid::new_v4();

        // Seed tax events for two different entities.
        let event_a = TaxEventRecord {
            id: Uuid::new_v4(),
            entity_id: entity_a,
            event_type: "capital_gains".to_string(),
            amount: "5000".to_string(),
            currency: "PKR".to_string(),
            tax_year: "2026".to_string(),
            details: serde_json::json!({"transaction": "share_sale"}),
            created_at: Utc::now(),
        };
        let event_b = TaxEventRecord {
            id: Uuid::new_v4(),
            entity_id: entity_b,
            event_type: "withholding".to_string(),
            amount: "1500".to_string(),
            currency: "PKR".to_string(),
            tax_year: "2026".to_string(),
            details: serde_json::json!({}),
            created_at: Utc::now(),
        };
        state.tax_events.insert(event_a.id, event_a);
        state.tax_events.insert(event_b.id, event_b);

        let app = router().with_state(state.clone());

        // Query tax events for entity_a.
        let req = Request::builder()
            .method("GET")
            .uri(format!("/v1/fiscal/{entity_a}/tax-events"))
            .body(Body::empty())
            .unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let events: Vec<TaxEventRecord> = body_json(resp).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity_id, entity_a);
        assert_eq!(events[0].event_type, "capital_gains");

        // Query tax events for entity_b.
        let req_b = Request::builder()
            .method("GET")
            .uri(format!("/v1/fiscal/{entity_b}/tax-events"))
            .body(Body::empty())
            .unwrap();
        let resp_b = app.oneshot(req_b).await.unwrap();
        let events_b: Vec<TaxEventRecord> = body_json(resp_b).await;
        assert_eq!(events_b.len(), 1);
        assert_eq!(events_b[0].entity_id, entity_b);
    }

    #[tokio::test]
    async fn handler_generate_report_returns_200() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/reporting/generate")
            .body(Body::empty())
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: serde_json::Value = body_json(resp).await;
        assert_eq!(body["status"], "generated");
    }

    #[tokio::test]
    async fn handler_create_account_no_ntn_returns_201() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","account_type":"operating","currency":"USD"}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: FiscalAccountRecord = body_json(resp).await;
        assert_eq!(record.account_type, "operating");
        assert_eq!(record.currency, "USD");
        assert!(record.ntn.is_none());
    }

    #[tokio::test]
    async fn handler_create_account_bad_json_returns_400() {
        let app = test_app();
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(r#"{{invalid"#))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn handler_create_account_empty_account_type_returns_422() {
        let app = test_app();
        let entity_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"entity_id":"{}","account_type":"","currency":"PKR"}}"#,
            entity_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/accounts")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn handler_initiate_payment_without_to_account_returns_201() {
        let app = test_app();
        let from_id = Uuid::new_v4();
        let body_str = format!(
            r#"{{"from_account_id":"{}","amount":"1000.00","currency":"PKR","reference":"PAY-001"}}"#,
            from_id
        );
        let req = Request::builder()
            .method("POST")
            .uri("/v1/fiscal/payments")
            .header("content-type", "application/json")
            .body(Body::from(body_str))
            .unwrap();

        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let record: PaymentRecord = body_json(resp).await;
        assert!(record.to_account_id.is_none());
        assert_eq!(record.reference, "PAY-001");
    }
}
