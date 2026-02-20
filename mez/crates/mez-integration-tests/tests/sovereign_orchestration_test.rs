// SPDX-License-Identifier: BUSL-1.1
//! Sovereign orchestration integration test.
//!
//! Verifies the critical fix: in sovereign mode, writes through /v1/* endpoints
//! produce full OrchestrationEnvelope responses (compliance tensor evaluation,
//! signed VC, attestation record) — not raw CRUD JSON.
//!
//! This test starts an in-process mez-api server with SOVEREIGN_MASS=true and
//! exercises each orchestrated write endpoint, asserting that:
//! 1. Response contains `mass_response` (the underlying data)
//! 2. Response contains `compliance` (tensor evaluation result)
//! 3. Response contains `credential` (signed VC)
//! 4. Response contains `attestation_id` (stored attestation)
//! 5. No Mass client is needed (sovereign ops handle CRUD directly)

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

/// Build the mez-api router in sovereign mode (no Mass client, no DB).
fn build_sovereign_app() -> axum::Router {
    // Force sovereign mode via env var.
    std::env::set_var("SOVEREIGN_MASS", "true");
    let state = mez_api::state::AppState::new();
    assert!(state.sovereign_mass, "SOVEREIGN_MASS must be true");
    assert!(state.mass_client.is_none(), "No Mass client in sovereign mode");

    mez_api::app(state)
}

/// Helper to extract status and JSON body from a response.
async fn response_json(response: axum::http::Response<Body>) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("collect body")
        .to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or_else(|e| {
        let text = String::from_utf8_lossy(&bytes);
        panic!("Failed to parse JSON (status={status}): {e}\nBody: {text}");
    });
    (status, body)
}

/// Assert an OrchestrationEnvelope has all expected fields.
fn assert_envelope(body: &Value, context: &str) {
    assert!(
        body.get("mass_response").is_some(),
        "{context}: missing mass_response"
    );
    assert!(
        body.get("compliance").is_some(),
        "{context}: missing compliance"
    );
    assert!(
        body.get("credential").is_some(),
        "{context}: missing credential (signed VC)"
    );
    assert!(
        body.get("attestation_id").is_some(),
        "{context}: missing attestation_id"
    );

    // Compliance must have evaluated domains.
    let compliance = &body["compliance"];
    assert!(
        compliance.get("overall_status").is_some(),
        "{context}: compliance missing overall_status"
    );
    assert!(
        compliance.get("jurisdiction_id").is_some(),
        "{context}: compliance missing jurisdiction_id"
    );
}

// ── Entity (create + get) ───────────────────────────────────────────

#[tokio::test]
async fn sovereign_create_entity_produces_envelope() {
    let app = build_sovereign_app();

    let req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Sovereign Test Corp",
                "entity_type": "LLC",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;

    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_envelope(&body, "create_entity");

    // The mass_response should have the entity data.
    let entity = &body["mass_response"];
    assert!(entity.get("id").is_some(), "entity missing id");
    assert_eq!(
        entity.get("name").and_then(|v| v.as_str()),
        Some("Sovereign Test Corp"),
        "entity name mismatch"
    );
}

#[tokio::test]
async fn sovereign_get_entity_returns_data() {
    let app = build_sovereign_app();

    // First create.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Get Test Corp",
                "entity_type": "CORP",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();

    let (status, create_body) = response_json(app.clone().oneshot(create_req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED);
    let entity_id = create_body["mass_response"]["id"]
        .as_str()
        .expect("entity id");

    // Then get.
    let get_req = Request::builder()
        .method("GET")
        .uri(&format!("/v1/entities/{entity_id}"))
        .body(Body::empty())
        .unwrap();

    let (status, get_body) = response_json(app.oneshot(get_req).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        get_body.get("id").and_then(|v| v.as_str()),
        Some(entity_id),
        "GET returned wrong entity"
    );
}

// ── Cap Table (ownership) ───────────────────────────────────────────

#[tokio::test]
async fn sovereign_create_cap_table_produces_envelope() {
    let app = build_sovereign_app();

    // Create entity first.
    let entity_req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Cap Table Corp",
                "entity_type": "CORP",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();

    let (_, entity_body) = response_json(app.clone().oneshot(entity_req).await.unwrap()).await;
    let entity_id = entity_body["mass_response"]["id"]
        .as_str()
        .expect("entity id");

    // Create cap table.
    let req = Request::builder()
        .method("POST")
        .uri("/v1/ownership/cap-tables")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "entity_id": entity_id,
                "share_classes": [{
                    "name": "Common",
                    "authorized_shares": 10000000,
                    "issued_shares": 0,
                    "par_value": "1.00",
                    "voting_rights": true
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_envelope(&body, "create_cap_table");
}

// ── Account (fiscal) ────────────────────────────────────────────────

#[tokio::test]
async fn sovereign_create_account_produces_envelope() {
    let app = build_sovereign_app();

    // Create entity first.
    let entity_req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Account Test Corp",
                "entity_type": "LLC",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();

    let (_, entity_body) = response_json(app.clone().oneshot(entity_req).await.unwrap()).await;
    let entity_id = entity_body["mass_response"]["id"]
        .as_str()
        .expect("entity id");

    // Create account.
    let req = Request::builder()
        .method("POST")
        .uri("/v1/fiscal/accounts")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "entity_id": entity_id,
                "account_type": "OPERATING",
                "currency": "PKR"
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_envelope(&body, "create_account");
}

// ── Payment (fiscal) ────────────────────────────────────────────────

#[tokio::test]
async fn sovereign_create_payment_produces_envelope() {
    let app = build_sovereign_app();

    // Create entity → account.
    let entity_req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Payment Corp",
                "entity_type": "LLC",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();
    let (_, entity_body) = response_json(app.clone().oneshot(entity_req).await.unwrap()).await;
    let entity_id = entity_body["mass_response"]["id"]
        .as_str()
        .expect("entity id");

    let acct_req = Request::builder()
        .method("POST")
        .uri("/v1/fiscal/accounts")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "entity_id": entity_id,
                "account_type": "OPERATING",
                "currency": "PKR"
            })
            .to_string(),
        ))
        .unwrap();
    let (_, acct_body) = response_json(app.clone().oneshot(acct_req).await.unwrap()).await;
    let account_id = acct_body["mass_response"]["id"]
        .as_str()
        .expect("account id");

    // Create payment.
    let req = Request::builder()
        .method("POST")
        .uri("/v1/fiscal/payments")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "from_account_id": account_id,
                "amount": "50000.00",
                "currency": "PKR",
                "reference": "INV-2026-001"
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_envelope(&body, "create_payment");
}

// ── Consent ─────────────────────────────────────────────────────────

#[tokio::test]
async fn sovereign_create_consent_produces_envelope() {
    let app = build_sovereign_app();

    // Create entity first.
    let entity_req = Request::builder()
        .method("POST")
        .uri("/v1/entities")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "legal_name": "Consent Corp",
                "entity_type": "CORP",
                "jurisdiction_id": "pk"
            })
            .to_string(),
        ))
        .unwrap();
    let (_, entity_body) = response_json(app.clone().oneshot(entity_req).await.unwrap()).await;
    let entity_id = entity_body["mass_response"]["id"]
        .as_str()
        .expect("entity id");

    let req = Request::builder()
        .method("POST")
        .uri("/v1/consent")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "consent_type": "BOARD_APPROVAL",
                "description": "Approve quarterly dividend",
                "parties": [{
                    "entity_id": entity_id,
                    "role": "APPROVER"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    assert_envelope(&body, "create_consent");
}

// ── Identity verification ───────────────────────────────────────────

#[tokio::test]
async fn sovereign_verify_identity_produces_envelope() {
    let app = build_sovereign_app();

    let req = Request::builder()
        .method("POST")
        .uri("/v1/identity/verify")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "identity_type": "CNIC",
                "linked_ids": [{
                    "id_type": "CNIC",
                    "id_value": "1234512345671"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK, "body: {body}");
    assert_envelope(&body, "verify_identity");
}

// ── Direct Mass routes also available ───────────────────────────────

#[tokio::test]
async fn sovereign_direct_mass_routes_available() {
    let app = build_sovereign_app();

    // The direct Mass API surface should be mounted at /organization-info/...
    let req = Request::builder()
        .method("POST")
        .uri("/organization-info/api/v1/organization/create")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "name": "Direct Route Corp",
                "jurisdiction": "pk"
            })
            .to_string(),
        ))
        .unwrap();

    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::CREATED, "body: {body}");
    // Direct route returns raw entity JSON, NOT an OrchestrationEnvelope.
    assert!(
        body.get("mass_response").is_none(),
        "direct route should not produce OrchestrationEnvelope"
    );
    assert!(body.get("id").is_some(), "direct route should return entity JSON");
}

// ── List entities in sovereign mode ─────────────────────────────────

#[tokio::test]
async fn sovereign_list_entities_returns_array() {
    let app = build_sovereign_app();

    // Create two entities.
    for name in &["List Corp A", "List Corp B"] {
        let req = Request::builder()
            .method("POST")
            .uri("/v1/entities")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "legal_name": name,
                    "entity_type": "CORP",
                    "jurisdiction_id": "pk"
                })
                .to_string(),
            ))
            .unwrap();
        let (status, _) = response_json(app.clone().oneshot(req).await.unwrap()).await;
        assert_eq!(status, StatusCode::CREATED);
    }

    // List.
    let req = Request::builder()
        .method("GET")
        .uri("/v1/entities")
        .body(Body::empty())
        .unwrap();
    let (status, body) = response_json(app.oneshot(req).await.unwrap()).await;
    assert_eq!(status, StatusCode::OK);
    let arr = body.as_array().expect("list should return array");
    assert!(arr.len() >= 2, "should have at least 2 entities, got {}", arr.len());
}
