//! Contract tests for FiscalClient against the Mass treasury-info Swagger spec.
//!
//! ## Endpoints Tested
//!
//! | Method | Path | Test |
//! |--------|------|------|
//! | POST   | `/api/v1/treasury/create` | `create_treasury_*` |
//! | POST   | `/api/v1/account/create` | `create_account_*` |
//! | GET    | `/api/v1/account/{id}` | `get_account_*` |
//! | POST   | `/api/v1/transaction/create/payment` | `create_payment_*` |
//! | GET    | `/api/v1/transaction/{id}` | `get_transaction_*` |

use msez_mass_client::fiscal::{
    CreatePaymentRequest, CreateTreasuryRequest, MassPaymentStatus, MassTreasuryContext,
};
use msez_mass_client::{MassApiConfig, MassClient};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn test_client(mock_server: &MockServer) -> MassClient {
    let config = MassApiConfig {
        organization_info_url: "http://127.0.0.1:19000".parse().unwrap(),
        investment_info_url: "http://127.0.0.1:19001".parse().unwrap(),
        treasury_info_url: mock_server.uri().parse().unwrap(),
        consent_info_url: "http://127.0.0.1:19003".parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:19004".parse().unwrap(),
        api_token: "test-token".into(),
        timeout_secs: 5,
    };
    MassClient::new(config).unwrap()
}

// ── POST /api/v1/treasury/create ─────────────────────────────────────

#[tokio::test]
async fn create_treasury_sends_correct_path_and_returns_treasury() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/treasury/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entityId": "entity-001",
            "name": "Test Treasury",
            "context": "MASS",
            "createdAt": "2026-01-15T12:00:00Z",
            "updatedAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateTreasuryRequest {
        entity_id: "entity-001".into(),
        entity_name: Some("Test Corp".into()),
        entity_type: Some("llc".into()),
        context: Some(MassTreasuryContext::Mass),
    };

    let treasury = client.fiscal().create_treasury(&req).await.unwrap();
    assert_eq!(treasury.entity_id, "entity-001");
    assert_eq!(treasury.name.as_deref(), Some("Test Treasury"));
    assert_eq!(treasury.context, Some(MassTreasuryContext::Mass));
}

#[tokio::test]
async fn create_treasury_handles_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/treasury/create"))
        .respond_with(ResponseTemplate::new(400).set_body_string("entity_id is required"))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateTreasuryRequest {
        entity_id: "".into(),
        entity_name: None,
        entity_type: None,
        context: None,
    };

    let result = client.fiscal().create_treasury(&req).await;
    assert!(result.is_err());
}

// ── POST /api/v1/account/create ──────────────────────────────────────

#[tokio::test]
async fn create_account_sends_correct_path_with_query_params() {
    let mock_server = MockServer::start().await;

    // The client appends query params: ?treasuryId=...&idempotencyKey=...
    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/account/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "treasuryId": "550e8400-e29b-41d4-a716-446655440000",
            "name": "PKR Operating",
            "currency": "PKR",
            "balance": "0.00",
            "available": "0.00",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let treasury_id = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    let account = client
        .fiscal()
        .create_account(treasury_id, "idem-key-001", Some("PKR Operating"))
        .await
        .unwrap();

    assert_eq!(account.name.as_deref(), Some("PKR Operating"));
    assert_eq!(account.currency.as_deref(), Some("PKR"));
    assert_eq!(account.balance.as_deref(), Some("0.00"));
}

#[tokio::test]
async fn create_account_without_name() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/account/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440001",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let treasury_id = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
    let account = client
        .fiscal()
        .create_account(treasury_id, "idem-key-002", None)
        .await
        .unwrap();

    assert!(account.name.is_none());
}

// ── GET /api/v1/account/{id} ─────────────────────────────────────────

#[tokio::test]
async fn get_account_returns_account_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440001";

    Mock::given(method("GET"))
        .and(path(format!("/treasury-info/api/v1/account/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "entityId": "entity-001",
            "treasuryId": "550e8400-e29b-41d4-a716-446655440000",
            "name": "USD Account",
            "currency": "USD",
            "balance": "100000.00",
            "available": "95000.00",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let account = client
        .fiscal()
        .get_account(id.parse().unwrap())
        .await
        .unwrap();
    assert!(account.is_some());
    let account = account.unwrap();
    assert_eq!(account.currency.as_deref(), Some("USD"));
    assert_eq!(account.balance.as_deref(), Some("100000.00"));
}

#[tokio::test]
async fn get_account_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440099";

    Mock::given(method("GET"))
        .and(path(format!("/treasury-info/api/v1/account/{id}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let account = client
        .fiscal()
        .get_account(id.parse().unwrap())
        .await
        .unwrap();
    assert!(account.is_none());
}

// ── POST /api/v1/transaction/create/payment ──────────────────────────

#[tokio::test]
async fn create_payment_sends_correct_path_and_returns_payment() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/transaction/create/payment"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440002",
            "accountId": "550e8400-e29b-41d4-a716-446655440001",
            "transactionType": "PAYMENT",
            "status": "PENDING",
            "direction": "OUTBOUND",
            "currency": "PKR",
            "amount": "50000.00",
            "reference": "INV-2026-001",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreatePaymentRequest {
        source_account_id: "550e8400-e29b-41d4-a716-446655440001".parse().unwrap(),
        amount: "50000.00".into(),
        currency: Some("PKR".into()),
        reference: Some("INV-2026-001".into()),
        description: Some("Invoice payment".into()),
        payment_type: None,
        idempotency_key: None,
        payment_entity: None,
    };

    let payment = client.fiscal().create_payment(&req).await.unwrap();
    assert_eq!(payment.amount.as_deref(), Some("50000.00"));
    assert_eq!(payment.currency.as_deref(), Some("PKR"));
    assert_eq!(payment.status, Some(MassPaymentStatus::Pending));
    assert_eq!(payment.reference.as_deref(), Some("INV-2026-001"));
}

// ── GET /api/v1/transaction/{id} ─────────────────────────────────────

#[tokio::test]
async fn get_transaction_returns_payment_when_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440002";

    Mock::given(method("GET"))
        .and(path(format!("/treasury-info/api/v1/transaction/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "status": "COMPLETED",
            "amount": "50000.00",
            "currency": "PKR",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let payment = client
        .fiscal()
        .get_transaction(id.parse().unwrap())
        .await
        .unwrap();
    assert!(payment.is_some());
    let payment = payment.unwrap();
    assert_eq!(payment.status, Some(MassPaymentStatus::Completed));
}

#[tokio::test]
async fn get_transaction_returns_none_when_not_found() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440099";

    Mock::given(method("GET"))
        .and(path(format!("/treasury-info/api/v1/transaction/{id}")))
        .respond_with(ResponseTemplate::new(404))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let payment = client
        .fiscal()
        .get_transaction(id.parse().unwrap())
        .await
        .unwrap();
    assert!(payment.is_none());
}

// ── Forward compatibility ────────────────────────────────────────────

#[tokio::test]
async fn treasury_deserializes_with_unknown_context() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/treasury-info/api/v1/treasury/create"))
        .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
            "id": "550e8400-e29b-41d4-a716-446655440000",
            "entityId": "entity-001",
            "context": "FUTURE_CONTEXT",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let req = CreateTreasuryRequest {
        entity_id: "entity-001".into(),
        entity_name: None,
        entity_type: None,
        context: None,
    };

    let treasury = client.fiscal().create_treasury(&req).await.unwrap();
    assert_eq!(treasury.context, Some(MassTreasuryContext::Unknown));
}

#[tokio::test]
async fn payment_deserializes_with_unknown_status() {
    let mock_server = MockServer::start().await;
    let id = "550e8400-e29b-41d4-a716-446655440002";

    Mock::given(method("GET"))
        .and(path(format!("/treasury-info/api/v1/transaction/{id}")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": id,
            "status": "FUTURE_STATUS",
            "createdAt": "2026-01-15T12:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = test_client(&mock_server).await;
    let payment = client
        .fiscal()
        .get_transaction(id.parse().unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(payment.status, Some(MassPaymentStatus::Unknown));
}
