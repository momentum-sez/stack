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

use axum::middleware::from_fn;
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

    // Unauthenticated health probes.
    let health = Router::new()
        .route("/health/liveness", axum::routing::get(liveness))
        .route("/health/readiness", axum::routing::get(readiness));

    Router::new().merge(health).merge(api)
}

/// Liveness probe — always returns 200 if the process is running.
async fn liveness() -> &'static str {
    "ok"
}

/// Readiness probe — returns 200 when the application is ready to serve.
async fn readiness() -> &'static str {
    "ready"
}
