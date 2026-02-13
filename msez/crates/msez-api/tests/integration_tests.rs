//! # Integration Tests for msez-api
//!
//! Tests each primitive's basic CRUD operations, corridor state transitions,
//! authentication middleware, and OpenAPI spec generation.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use msez_api::state::{AppConfig, AppState};

/// Helper: build the test app with auth disabled.
fn test_app() -> axum::Router {
    let state = AppState::new();
    msez_api::app(state)
}

/// Helper: build the test app with auth enabled.
fn test_app_with_auth(token: &str) -> axum::Router {
    let config = AppConfig {
        port: 8080,
        auth_token: Some(token.to_string()),
    };
    let state = AppState::with_config(config);
    msez_api::app(state)
}

/// Helper: read response body as string.
async fn body_string(response: axum::http::Response<Body>) -> String {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// ── Health Probes ───────────────────────────────────────────────────

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
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(json["status"], "ready");
    assert!(json["checks"].is_object());
    assert_eq!(json["checks"]["entity_store"], "ok");
    assert_eq!(json["checks"]["corridor_store"], "ok");
}

// ── Entities CRUD ───────────────────────────────────────────────────

#[tokio::test]
async fn test_create_entity() {
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = body_string(response).await;
    let entity: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(entity["legal_name"], "Test Corp");
    assert_eq!(entity["status"], "APPLIED");
    assert!(entity["id"].is_string());
}

#[tokio::test]
async fn test_list_entities_empty() {
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
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;
    let page: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert!(page["data"].as_array().unwrap().is_empty());
    assert_eq!(page["total"], 0);
    assert_eq!(page["offset"], 0);
    assert_eq!(page["limit"], 50);
}

#[tokio::test]
async fn test_entity_not_found() {
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
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_create_entity_validation_error() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/entities")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_type": "",
                        "legal_name": "Test",
                        "jurisdiction_id": "PK-RSEZ"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Ownership ───────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_cap_table() {
    let app = test_app();
    let entity_id = uuid::Uuid::new_v4();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/ownership/cap-table")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_id": entity_id.to_string(),
                        "share_classes": [{
                            "name": "Common",
                            "authorized_shares": 1000000,
                            "issued_shares": 0,
                            "par_value": "1.00",
                            "voting_rights": true
                        }]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

// ── Fiscal ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_create_fiscal_account() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/fiscal/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_id": uuid::Uuid::new_v4().to_string(),
                        "account_type": "treasury",
                        "currency": "PKR",
                        "ntn": "1234567"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_fiscal_ntn_validation() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/fiscal/accounts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "entity_id": uuid::Uuid::new_v4().to_string(),
                        "account_type": "treasury",
                        "currency": "PKR",
                        "ntn": "123"
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

// ── Identity ────────────────────────────────────────────────────────

#[tokio::test]
async fn test_identity_verify() {
    let app = test_app();
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/identity/verify")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "identity_type": "kyc",
                        "details": {"name": "Test Person"}
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

// ── Consent ─────────────────────────────────────────────────────────

#[tokio::test]
async fn test_consent_lifecycle() {
    let app = test_app();
    let entity_a = uuid::Uuid::new_v4();
    let entity_b = uuid::Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/consent/request")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "consent_type": "tax_assessment",
                        "description": "Annual tax assessment consent",
                        "parties": [
                            {"entity_id": entity_a.to_string(), "role": "taxpayer"},
                            {"entity_id": entity_b.to_string(), "role": "assessor"}
                        ]
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = body_string(response).await;
    let consent: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(consent["status"], "PENDING");
}

// ── Corridors ───────────────────────────────────────────────────────

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

// ── Smart Assets ────────────────────────────────────────────────────

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

// ── Regulator ───────────────────────────────────────────────────────

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

// ── Authentication ──────────────────────────────────────────────────

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
    assert_eq!(response.status(), StatusCode::OK);
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

// ── OpenAPI ─────────────────────────────────────────────────────────

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
    // Verify it's a valid OpenAPI spec.
    assert!(spec["openapi"].is_string());
    assert!(spec["info"]["title"].is_string());
    assert!(spec["paths"].is_object());
    // Verify key paths exist.
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
    let expected_paths = [
        "/v1/entities",
        "/v1/ownership/cap-table",
        "/v1/fiscal/accounts",
        "/v1/identity/verify",
        "/v1/consent/request",
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
