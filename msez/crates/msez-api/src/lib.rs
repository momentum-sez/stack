//! # msez-api — Axum API Services for the SEZ Stack
//!
//! The SEZ Stack is the orchestration layer above the Mass APIs.
//! It provides compliance tensor evaluation, corridor lifecycle management,
//! smart asset operations, regulator console, and a proxy layer to the
//! live Mass APIs for primitive operations (entities, ownership, fiscal,
//! identity, consent).
//!
//! ## API Surface
//!
//! | Prefix               | Module                      | Domain              |
//! |-----------------------|----------------------------|---------------------|
//! | `/v1/entities/*`      | [`routes::mass_proxy`]     | Mass proxy (Entities) |
//! | `/v1/ownership/*`     | [`routes::mass_proxy`]     | Mass proxy (Ownership) |
//! | `/v1/fiscal/*`        | [`routes::mass_proxy`]     | Mass proxy (Fiscal) |
//! | `/v1/identity/*`      | [`routes::mass_proxy`]     | Mass proxy (Identity) |
//! | `/v1/consent/*`       | [`routes::mass_proxy`]     | Mass proxy (Consent) |
//! | `/v1/corridors/*`     | [`routes::corridors`]      | Corridors (SEZ)     |
//! | `/v1/assets/*`        | [`routes::smart_assets`]   | Smart Assets (SEZ)  |
//! | `/v1/regulator/*`     | [`routes::regulator`]      | Regulator (SEZ)     |
//!
//! ## Middleware Stack (execution order)
//!
//! ```text
//! TraceLayer → MetricsMiddleware → AuthMiddleware → RateLimitMiddleware → Handler
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
        // Mass API proxy (all five primitives via Mass client)
        .merge(routes::mass_proxy::router())
        // SEZ Stack native routes (genuinely this codebase's domain)
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
