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
        auth_token: Some(token.to_string()),
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
    // Note: ownership, fiscal, identity, consent routes have been removed
    // (those primitives are now proxied to Mass APIs via /v1/entities).
    let expected_paths = [
        "/v1/entities",
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
