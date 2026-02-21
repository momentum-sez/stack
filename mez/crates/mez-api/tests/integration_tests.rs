//! # Integration Tests for mez-api
//!
//! Tests corridor state transitions, smart asset operations, regulator console,
//! Mass API proxy behavior (503 without client), authentication middleware,
//! and OpenAPI spec generation.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use mez_api::state::{AppConfig, AppState};

/// Helper: build the test app with auth disabled and no Mass client.
fn test_app() -> axum::Router {
    let state = AppState::new();
    mez_api::app(state)
}

/// Helper: build the test app with auth enabled.
fn test_app_with_auth(token: &str) -> axum::Router {
    let config = AppConfig {
        port: 8080,
        auth_token: Some(mez_api::auth::SecretString::new(token)),
    };
    let state = AppState::with_config(config, None);
    mez_api::app(state)
}

/// Helper: read response body as string.
async fn body_string(response: axum::http::Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// -- Health Probes ------------------------------------------------------------

#[tokio::test]
async fn test_liveness_probe() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/liveness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn test_readiness_probe() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert_eq!(body, "ready");
}

// -- Entity Proxy (Mass API delegation) ---------------------------------------
//
// Without a Mass client configured, entity endpoints return 503.

#[tokio::test]
async fn test_create_entity_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/entities")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_type": "company",
                        "legal_name": "Test Corp",
                        "jurisdiction_id": "PK-REZ"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_list_entities_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/entities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_get_entity_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/entities/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Ownership Proxy (Mass API delegation) ------------------------------------
//
// Without a Mass client configured, ownership endpoints return 503.

#[tokio::test]
async fn test_create_cap_table_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/ownership/cap-tables")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_id": "00000000-0000-0000-0000-000000000000",
                        "share_classes": [{"name":"Common","authorized_shares":1000000,"issued_shares":100000,"par_value":"0.01","voting_rights":true}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_get_cap_table_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/ownership/cap-tables/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Fiscal Proxy (Mass API delegation) ---------------------------------------

#[tokio::test]
async fn test_create_account_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/fiscal/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_id": "00000000-0000-0000-0000-000000000000",
                        "account_type": "operating",
                        "currency": "PKR"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_initiate_payment_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/fiscal/payments")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "from_account_id": "00000000-0000-0000-0000-000000000000",
                        "amount": "5000.00",
                        "currency": "PKR",
                        "reference": "INV-001"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Identity Proxy (Mass API delegation) -------------------------------------

#[tokio::test]
async fn test_verify_identity_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/identity/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "identity_type": "individual",
                        "linked_ids": [{"id_type": "CNIC", "id_value": "12345-1234567-1"}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_get_identity_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/identity/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Consent Proxy (Mass API delegation) --------------------------------------

#[tokio::test]
async fn test_create_consent_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/consent")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "consent_type": "board_resolution",
                        "description": "Approve formation",
                        "parties": [{"entity_id": "00000000-0000-0000-0000-000000000000", "role": "approver"}]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_get_consent_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/consent/00000000-0000-0000-0000-000000000000")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Update Entity returns 503 without Mass client ----------------------------

#[tokio::test]
async fn test_update_entity_returns_503_without_mass_client() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/v1/entities/00000000-0000-0000-0000-000000000000")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"legal_name":"Updated Corp"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

// -- Corridors (EZ Stack domain) ---------------------------------------------

#[tokio::test]
async fn test_create_corridor() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/corridors")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "jurisdiction_a": "PK-REZ",
                        "jurisdiction_b": "AE-DIFC"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = body_string(response).await;
    let corridor: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(corridor["state"], "DRAFT");
}

#[tokio::test]
async fn test_corridor_same_jurisdiction_rejected() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/corridors")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "jurisdiction_a": "PK-REZ",
                        "jurisdiction_b": "PK-REZ"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// -- Smart Assets (EZ Stack domain) ------------------------------------------

#[tokio::test]
async fn test_create_smart_asset() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/assets/genesis")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "asset_type": "equity",
                        "jurisdiction_id": "PK-REZ",
                        "metadata": {"description": "Test asset"}
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

// -- Regulator (EZ Stack domain) ---------------------------------------------

#[tokio::test]
async fn test_regulator_summary() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/regulator/summary")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let summary: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(summary["total_entities"], 0);
    assert_eq!(summary["total_corridors"], 0);
}

// -- Authentication -----------------------------------------------------------

#[tokio::test]
async fn test_auth_rejects_unauthorized() {
    let app = test_app_with_auth("secret-token-123");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/entities")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_auth_accepts_valid_token() {
    let app = test_app_with_auth("secret-token-123");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/entities")
                .header("authorization", "Bearer secret-token-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    // 503 because no Mass client, but auth passed (not 401).
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn test_auth_rejects_wrong_token() {
    let app = test_app_with_auth("secret-token-123");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/v1/entities")
                .header("authorization", "Bearer wrong-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_health_bypasses_auth() {
    let app = test_app_with_auth("secret-token-123");
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/liveness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

// -- RBAC: Regulator Endpoint Access Control ----------------------------------

#[tokio::test]
async fn rbac_regulator_summary_rejected_for_entity_operator() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/summary")
        .header(
            "Authorization",
            "Bearer entity_operator:550e8400-e29b-41d4-a716-446655440000:my-secret",
        )
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_regulator_summary_allowed_for_regulator() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/summary")
        .header("Authorization", "Bearer regulator::my-secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn rbac_regulator_summary_allowed_for_zone_admin() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/summary")
        .header("Authorization", "Bearer zone_admin::my-secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn rbac_regulator_query_attestations_rejected_for_entity_operator() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("POST")
        .uri("/v1/regulator/query/attestations")
        .header(
            "Authorization",
            "Bearer entity_operator:550e8400-e29b-41d4-a716-446655440000:my-secret",
        )
        .header("content-type", "application/json")
        .body(Body::from(r#"{}"#))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_regulator_dashboard_rejected_for_entity_operator() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/dashboard")
        .header(
            "Authorization",
            "Bearer entity_operator:550e8400-e29b-41d4-a716-446655440000:my-secret",
        )
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn rbac_legacy_token_format_treated_as_zone_admin() {
    let app = test_app_with_auth("my-secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/summary")
        .header("Authorization", "Bearer my-secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn rbac_unknown_role_in_token_rejected() {
    let app = test_app_with_auth("secret");

    let req = Request::builder()
        .method("GET")
        .uri("/v1/regulator/summary")
        .header("Authorization", "Bearer superadmin::secret")
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn rbac_malformed_entity_id_in_token_rejected() {
    let app = test_app_with_auth("secret");

    let req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header("Authorization", "Bearer entity_operator:not-a-uuid:secret")
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

// -- IDOR: Smart Asset Ownership Protection -----------------------------------

/// Helper: parse JSON from response body.
async fn body_json(response: axum::http::Response<Body>) -> serde_json::Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn idor_entity_cannot_access_another_entitys_asset() {
    let app = test_app_with_auth("secret");

    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let entity_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    // Entity A creates an asset.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Entity B tries to read it — must get 404 (not 403).
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/v1/assets/{asset_id}"))
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_b}:secret"),
        )
        .body(Body::empty())
        .unwrap();
    let get_resp = app.clone().oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn idor_entity_can_access_own_asset() {
    let app = test_app_with_auth("secret");
    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

    // Create asset as Entity A.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Read asset as Entity A — should succeed.
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/v1/assets/{asset_id}"))
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .body(Body::empty())
        .unwrap();
    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn idor_regulator_can_read_any_entitys_asset() {
    let app = test_app_with_auth("secret");
    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

    // Entity A creates an asset.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Regulator reads it — should succeed.
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/v1/assets/{asset_id}"))
        .header("Authorization", "Bearer regulator::secret")
        .body(Body::empty())
        .unwrap();
    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn idor_on_compliance_evaluate_blocked() {
    let app = test_app_with_auth("secret");
    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let entity_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    // Entity A creates an asset.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"equity","jurisdiction_id":"AE-DIFC"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Entity B tries to evaluate compliance — must get 404.
    let eval_req = Request::builder()
        .method("POST")
        .uri(format!("/v1/assets/{asset_id}/compliance/evaluate"))
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_b}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(r#"{"domains":["aml"],"context":{}}"#))
        .unwrap();
    let eval_resp = app.oneshot(eval_req).await.unwrap();
    assert_eq!(eval_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn idor_on_anchor_verify_blocked() {
    let app = test_app_with_auth("secret");
    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
    let entity_b = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";

    // Entity A creates an asset.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Entity B tries to verify anchor — must get 404.
    let verify_req = Request::builder()
        .method("POST")
        .uri(format!("/v1/assets/{asset_id}/anchors/corridor/verify"))
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_b}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"anchor_digest":"sha256:deadbeef","chain":"ethereum"}"#,
        ))
        .unwrap();
    let verify_resp = app.oneshot(verify_req).await.unwrap();
    assert_eq!(verify_resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn idor_zone_admin_can_access_any_asset() {
    let app = test_app_with_auth("secret");
    let entity_a = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";

    // Entity A creates an asset.
    let create_req = Request::builder()
        .method("POST")
        .uri("/v1/assets/genesis")
        .header(
            "Authorization",
            format!("Bearer entity_operator:{entity_a}:secret"),
        )
        .header("content-type", "application/json")
        .body(Body::from(
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PEZ"}"#,
        ))
        .unwrap();
    let create_resp = app.clone().oneshot(create_req).await.unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created = body_json(create_resp).await;
    let asset_id = created["id"].as_str().unwrap();

    // Zone admin reads it — should succeed.
    let get_req = Request::builder()
        .method("GET")
        .uri(format!("/v1/assets/{asset_id}"))
        .header("Authorization", "Bearer zone_admin::secret")
        .body(Body::empty())
        .unwrap();
    let get_resp = app.oneshot(get_req).await.unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
}

// -- OpenAPI ------------------------------------------------------------------

#[tokio::test]
async fn test_openapi_spec_generation() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let spec: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(spec["openapi"].is_string());
    assert!(spec["info"]["title"].is_string());
    assert!(spec["paths"].is_object());
    assert!(spec["paths"]["/v1/entities"].is_object());
    assert!(spec["paths"]["/v1/corridors"].is_object());
    assert!(spec["paths"]["/v1/assets/genesis"].is_object());
}

#[tokio::test]
async fn test_openapi_contains_all_routes() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_string(response).await;
    let spec: serde_json::Value = serde_json::from_str(&body).unwrap();
    let paths = spec["paths"].as_object().unwrap();

    // Check that all expected path prefixes are present.
    // All five primitives are proxied to Mass APIs, plus EZ Stack native routes.
    let expected_paths = [
        "/v1/entities",
        "/v1/ownership/cap-tables",
        "/v1/fiscal/accounts",
        "/v1/fiscal/payments",
        "/v1/identity",
        "/v1/consent",
        "/v1/corridors",
        "/v1/assets/genesis",
        "/v1/regulator/summary",
    ];

    for expected in &expected_paths {
        assert!(
            paths
                .keys()
                .any(|k| k.starts_with(expected) || k == expected),
            "OpenAPI spec missing path: {expected}"
        );
    }
}

// -- Mass API Health Gating (Deployment Blocker #1) ---------------------------

#[tokio::test]
async fn test_readiness_probe_passes_without_mass_client() {
    // When mass_client is None, the readiness probe should pass.
    // The server already returns 503 on proxy routes in this case.
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    assert_eq!(body, "ready");
}

#[tokio::test]
async fn test_readiness_probe_fails_with_unreachable_mass_client() {
    // When mass_client is Some but Mass APIs are unreachable, readiness
    // should return 503.
    let mass_config = mez_mass_client::MassApiConfig {
        organization_info_url: "http://127.0.0.1:1".parse().unwrap(),
        investment_info_url: "http://127.0.0.1:2".parse().unwrap(),
        treasury_info_url: "http://127.0.0.1:3".parse().unwrap(),
        consent_info_url: "http://127.0.0.1:4".parse().unwrap(),
        identity_info_url: None,
        templating_engine_url: "http://127.0.0.1:5".parse().unwrap(),
        api_token: zeroize::Zeroizing::new("test-token".into()),
        timeout_secs: 1,
    };
    let mass_client = mez_mass_client::MassClient::new(mass_config).unwrap();

    let config = AppConfig {
        port: 8080,
        auth_token: None,
    };
    let state = AppState::try_with_config(config, Some(mass_client), None)
        .expect("failed to create AppState");
    let app = mez_api::app(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health/readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = body_string(response).await;
    assert!(
        body.contains("mass api unreachable"),
        "Expected 'mass api unreachable' in body, got: {body}"
    );
}

// -- End-to-End Corridor Lifecycle (Roadmap Priority 1) -----------------------
//
// Proves the full "AWS of Economic Zones" lifecycle:
// 1. Create corridor between two jurisdictions
// 2. Walk the typestate machine: DRAFT → PENDING → ACTIVE
// 3. Exchange cross-border receipts (dual-commitment: hash-chain + MMR)
// 4. Query the receipt chain and verify cryptographic integrity
// 5. Create a checkpoint snapshot
// 6. Query compliance tensor for both jurisdictions
// 7. Query bilateral corridor compliance
//
// This test exercises the API surface that a sovereign operator would use
// to deploy and operate a cross-border corridor.

#[tokio::test]
async fn e2e_corridor_lifecycle_receipts_compliance() {
    let app = test_app();

    // ── Step 1: Create a PK ↔ AE corridor ────────────────────────

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/corridors")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jurisdiction_a":"pk","jurisdiction_b":"ae"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let corridor: serde_json::Value = body_json(create_resp).await;
    let corridor_id = corridor["id"].as_str().unwrap().to_string();
    assert_eq!(corridor["state"], "DRAFT");

    // ── Step 2: DRAFT → PENDING → ACTIVE ─────────────────────────

    let evidence = "a".repeat(64);
    let pending_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/corridors/{corridor_id}/transition"))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"target_state":"PENDING","evidence_digest":"{evidence}","reason":"bilateral agreement signed"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pending_resp.status(), StatusCode::OK);
    let pending: serde_json::Value = body_json(pending_resp).await;
    assert_eq!(pending["state"], "PENDING");

    let active_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/v1/corridors/{corridor_id}/transition"))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"target_state":"ACTIVE","evidence_digest":"{evidence}","reason":"regulatory approval"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(active_resp.status(), StatusCode::OK);
    let active: serde_json::Value = body_json(active_resp).await;
    assert_eq!(active["state"], "ACTIVE");
    assert_eq!(active["transition_log"].as_array().unwrap().len(), 2);

    // ── Step 3: Exchange 5 cross-border receipts ──────────────────

    let mut receipt_next_roots = Vec::new();
    let mut last_mmr_root = String::new();

    for i in 0..5u32 {
        let payload = serde_json::json!({
            "type": "cross_border_transfer",
            "from_zone": "pk-sifc",
            "to_zone": "ae-difc",
            "amount": format!("{}.00", (i + 1) * 10_000),
            "currency": "USD",
            "reference": format!("XB-PK-AE-{:04}", i),
            "beneficiary": format!("entity-{}", i),
        });
        let body = serde_json::to_string(&serde_json::json!({
            "corridor_id": corridor_id,
            "payload": payload,
        }))
        .unwrap();

        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/corridors/state/propose")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);

        let receipt: serde_json::Value = body_json(resp).await;
        assert_eq!(receipt["sequence"], i);
        assert_eq!(receipt["chain_height"], i + 1);
        let next_root = receipt["next_root"].as_str().unwrap().to_string();
        assert_eq!(next_root.len(), 64);
        receipt_next_roots.push(next_root);

        let mmr = receipt["mmr_root"].as_str().unwrap().to_string();
        if !last_mmr_root.is_empty() {
            assert_ne!(mmr, last_mmr_root, "MMR root must change with each receipt");
        }
        last_mmr_root = mmr;
    }

    // All next_roots must be unique (different payloads → different digests).
    let unique: std::collections::HashSet<&String> = receipt_next_roots.iter().collect();
    assert_eq!(unique.len(), 5, "all receipt digests must be unique");

    // ── Step 4: Query receipt chain ───────────────────────────────

    let chain_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/corridors/{corridor_id}/receipts"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(chain_resp.status(), StatusCode::OK);
    let chain: serde_json::Value = body_json(chain_resp).await;
    assert_eq!(chain["chain_height"], 5);
    assert_eq!(chain["receipts"].as_array().unwrap().len(), 5);

    let genesis_root = chain["genesis_root"].as_str().unwrap();
    let final_root = chain["final_state_root"].as_str().unwrap();
    let chain_mmr = chain["mmr_root"].as_str().unwrap();

    // Genesis root is 64-char hex.
    assert_eq!(genesis_root.len(), 64);
    // Final root differs from genesis (chain advanced).
    assert_ne!(genesis_root, final_root);
    // MMR root is 64-char hex.
    assert_eq!(chain_mmr.len(), 64);
    // MMR root matches the last receipt's MMR root.
    assert_eq!(chain_mmr, last_mmr_root);

    // Verify chain linkage: receipt[0].prev_root != receipt[1].prev_root.
    let receipts = chain["receipts"].as_array().unwrap();
    assert_ne!(
        receipts[0]["prev_root"], receipts[1]["prev_root"],
        "consecutive receipts must have different prev_roots"
    );

    // ── Step 5: Create checkpoint ────────────────────────────────

    let cp_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/v1/corridors/{corridor_id}/checkpoint"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(cp_resp.status(), StatusCode::CREATED);
    let checkpoint: serde_json::Value = body_json(cp_resp).await;
    assert_eq!(checkpoint["receipt_count"], 5);
    assert_eq!(checkpoint["checkpoint_count"], 1);
    assert_eq!(checkpoint["checkpoint_type"], "MEZCorridorStateCheckpoint");
    // Checkpoint commits to the same state as the chain query.
    assert_eq!(
        checkpoint["final_state_root"].as_str().unwrap(),
        final_root
    );
    assert_eq!(checkpoint["mmr_root"].as_str().unwrap(), chain_mmr);
    assert_eq!(
        checkpoint["genesis_root"].as_str().unwrap(),
        genesis_root
    );

    // ── Step 6: Query compliance tensor for both jurisdictions ────

    let pk_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/compliance/pk")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(pk_resp.status(), StatusCode::OK);
    let pk_compliance: serde_json::Value = body_json(pk_resp).await;
    assert_eq!(pk_compliance["jurisdiction_id"], "pk");
    assert_eq!(pk_compliance["total_domains"], 20);
    // Pakistan has 9 applicable domains (blocking) + 11 not-applicable (passing).
    assert!(pk_compliance["passing_count"].as_u64().unwrap() > 0);
    assert!(pk_compliance["blocking_count"].as_u64().unwrap() > 0);

    let ae_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/compliance/ae")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(ae_resp.status(), StatusCode::OK);
    let ae_compliance: serde_json::Value = body_json(ae_resp).await;
    assert_eq!(ae_compliance["jurisdiction_id"], "ae");
    assert_eq!(ae_compliance["total_domains"], 20);

    // ── Step 7: Query bilateral corridor compliance ──────────────

    let bilateral_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/v1/compliance/corridor/{corridor_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(bilateral_resp.status(), StatusCode::OK);
    let bilateral: serde_json::Value = body_json(bilateral_resp).await;
    assert_eq!(bilateral["corridor_id"], corridor_id);
    assert_eq!(bilateral["jurisdiction_a"]["jurisdiction_id"], "pk");
    assert_eq!(bilateral["jurisdiction_b"]["jurisdiction_id"], "ae");
    // Both jurisdictions have applicable domains that are pending,
    // so the corridor is not fully compliant yet.
    assert_eq!(bilateral["corridor_compliant"], false);
    // Cross-blocking domains exist (the union of blocking domains across both).
    assert!(
        bilateral["cross_blocking_domains"]
            .as_array()
            .unwrap()
            .len()
            > 0
    );

    // ── Step 8: Verify corridor compliance domains ───────────────

    let domains_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/v1/compliance/domains")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(domains_resp.status(), StatusCode::OK);
    let domains: serde_json::Value = body_json(domains_resp).await;
    assert_eq!(domains.as_array().unwrap().len(), 20);
}

// -- End-to-End: Receipt Pagination -------------------------------------------

#[tokio::test]
async fn e2e_receipt_pagination_across_api() {
    let app = test_app();

    // Create corridor.
    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/corridors")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"jurisdiction_a":"sg","jurisdiction_b":"hk"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let corridor: serde_json::Value = body_json(create_resp).await;
    let corridor_id = corridor["id"].as_str().unwrap().to_string();

    // Propose 10 receipts.
    for i in 0..10u32 {
        let body = serde_json::to_string(&serde_json::json!({
            "corridor_id": corridor_id,
            "payload": {"seq": i, "data": format!("batch-{i}")},
        }))
        .unwrap();
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/corridors/state/propose")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    // Page 1: first 3 receipts.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/corridors/{corridor_id}/receipts?limit=3&offset=0"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let page1: serde_json::Value = body_json(resp).await;
    assert_eq!(page1["chain_height"], 10);
    assert_eq!(page1["receipts"].as_array().unwrap().len(), 3);
    assert_eq!(page1["receipts"][0]["sequence"], 0);
    assert_eq!(page1["receipts"][2]["sequence"], 2);

    // Page 4: receipts 9 (only 1 left).
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/v1/corridors/{corridor_id}/receipts?limit=3&offset=9"
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let page4: serde_json::Value = body_json(resp).await;
    assert_eq!(page4["receipts"].as_array().unwrap().len(), 1);
    assert_eq!(page4["receipts"][0]["sequence"], 9);
}
