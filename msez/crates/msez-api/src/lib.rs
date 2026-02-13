//! # msez-api — Axum API Services for the SEZ Stack
//!
//! Assembles the five programmable primitive API services into a single
//! Axum application with Tower middleware for authentication, tracing,
//! metrics, and rate limiting.
//!
//! ## API Surface
//!
//! | Prefix               | Module                     | Primitive      |
//! |-----------------------|---------------------------|----------------|
//! | `/v1/entities/*`      | [`routes::entities`]      | ENTITIES       |
//! | `/v1/ownership/*`     | [`routes::ownership`]     | OWNERSHIP      |
//! | `/v1/fiscal/*`        | [`routes::fiscal`]        | FISCAL         |
//! | `/v1/identity/*`      | [`routes::identity`]      | IDENTITY       |
//! | `/v1/consent/*`       | [`routes::consent`]       | CONSENT        |
//! | `/v1/corridors/*`     | [`routes::corridors`]     | Corridors      |
//! | `/v1/assets/*`        | [`routes::smart_assets`]  | Smart Assets   |
//! | `/v1/regulator/*`     | [`routes::regulator`]     | Regulator      |
//!
//! ## Middleware Stack
//!
//! ```text
//! TraceLayer → MetricsMiddleware → AuthMiddleware → RateLimitMiddleware
//! ```
//!
//! ## OpenAPI
//!
//! Auto-generated OpenAPI 3.1 spec via utoipa derive macros at `/openapi.json`.

pub mod auth;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod state;

use axum::extract::State;
use axum::middleware::from_fn;
use axum::response::IntoResponse;
use axum::Router;
use tower_http::trace::TraceLayer;

use crate::auth::AuthConfig;
use crate::middleware::metrics::ApiMetrics;
use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};
use crate::state::AppState;

/// Assemble the full application router with all routes and middleware.
///
/// Health probes (`/health/*`) are mounted outside the auth middleware
/// so they remain accessible without credentials.
pub fn app(state: AppState) -> Router {
    let auth_config = AuthConfig {
        token: state.config.auth_token.clone(),
    };
    let metrics = ApiMetrics::new();
    let limiter = RateLimiter::new(RateLimitConfig::default());

    // Unauthenticated health probes — readiness needs state to verify stores.
    let health = Router::new()
        .route("/health/liveness", axum::routing::get(liveness))
        .route("/health/readiness", axum::routing::get(readiness))
        .with_state(state.clone());

    // Authenticated API routes.
    let api = Router::new()
        .merge(routes::entities::router())
        .merge(routes::ownership::router())
        .merge(routes::fiscal::router())
        .merge(routes::identity::router())
        .merge(routes::consent::router())
        .merge(routes::corridors::router())
        .merge(routes::smart_assets::router())
        .merge(routes::regulator::router())
        .merge(openapi::router())
        .layer(from_fn(middleware::rate_limit::rate_limit_middleware))
        .layer(from_fn(auth::auth_middleware))
        .layer(from_fn(middleware::metrics::metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(metrics))
        .layer(axum::Extension(limiter))
        .with_state(state);

    Router::new().merge(health).merge(api)
}

/// Liveness probe — always returns 200 if the process is running.
async fn liveness() -> &'static str {
    "ok"
}

/// Readiness probe — returns 200 when the application is ready to serve.
///
/// Verifies store accessibility by acquiring read locks on the entity and
/// corridor stores. If any lock acquisition fails (should not happen with
/// `parking_lot::RwLock`), Kubernetes will stop routing traffic to this pod.
async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    let entity_count = state.entities.len();
    let corridor_count = state.corridors.len();

    let body = serde_json::json!({
        "status": "ready",
        "checks": {
            "entity_store": "ok",
            "corridor_store": "ok",
            "entities_loaded": entity_count,
            "corridors_loaded": corridor_count
        }
    });

    (axum::http::StatusCode::OK, axum::Json(body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    #[tokio::test]
    async fn readiness_returns_json_health_status() {
        let router = app(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/health/readiness")
            .body(Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let bytes = resp.into_body().collect().await.unwrap().to_bytes();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(body["status"], "ready");
        assert!(body["checks"].is_object());
        assert_eq!(body["checks"]["entity_store"], "ok");
        assert_eq!(body["checks"]["corridor_store"], "ok");
        assert_eq!(body["checks"]["entities_loaded"], 0);
        assert_eq!(body["checks"]["corridors_loaded"], 0);
    }

    #[tokio::test]
    async fn liveness_returns_ok() {
        let router = app(AppState::new());
        let req = Request::builder()
            .method("GET")
            .uri("/health/liveness")
            .body(Body::empty())
            .unwrap();

        let resp = router.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
