//! # Integration Tests for msez-api
//!
//! Tests corridor state transitions, smart asset operations, regulator console,
//! Mass API proxy behavior (503 without client), authentication middleware,
//! and OpenAPI spec generation.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use msez_api::state::{AppConfig, AppState};

/// Helper: build the test app with auth disabled and no Mass client.
fn test_app() -> axum::Router {
    let state = AppState::new();
    msez_api::app(state)
}

/// Helper: build the test app with auth enabled.
fn test_app_with_auth(token: &str) -> axum::Router {
    let config = AppConfig {
        port: 8080,
        auth_token: Some(msez_api::auth::SecretToken::new(token.to_string())),
    };
    let state = AppState::with_config(config, None);
    msez_api::app(state)
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
                        "jurisdiction_id": "PK-RSEZ"
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
                        "share_classes": []
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

// -- Update Entity returns 501 (not implemented) ------------------------------

#[tokio::test]
async fn test_update_entity_returns_501() {
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
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

// -- Corridors (SEZ Stack domain) ---------------------------------------------

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
                        "jurisdiction_a": "PK-RSEZ",
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
                        "jurisdiction_a": "PK-RSEZ",
                        "jurisdiction_b": "PK-RSEZ"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// -- Smart Assets (SEZ Stack domain) ------------------------------------------

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
                        "jurisdiction_id": "PK-RSEZ",
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

// -- Regulator (SEZ Stack domain) ---------------------------------------------

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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
            r#"{"asset_type":"bond","jurisdiction_id":"PK-PSEZ"}"#,
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
    // All five primitives are proxied to Mass APIs, plus SEZ Stack native routes.
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
