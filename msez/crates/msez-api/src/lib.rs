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
//! | `/v1/assets/*/credentials/*` | [`routes::credentials`] | VC Issuance (SEZ) |
//! | `/v1/credentials/*`  | [`routes::credentials`]    | VC Verification (SEZ) |
//! | `/v1/triggers`        | [`routes::agentic`]        | Agentic Engine (SEZ)|
//! | `/v1/policies/*`      | [`routes::agentic`]        | Policy Mgmt (SEZ)   |
//! | `/v1/regulator/*`     | [`routes::regulator`]      | Regulator (SEZ)     |
//!
//! ## Middleware Stack (execution order)
//!
//! ```text
//! TraceLayer → MetricsMiddleware → RateLimitMiddleware → AuthMiddleware → Handler
//! ```
//!
//! ## OpenAPI
//!
//! Auto-generated OpenAPI 3.1 spec via utoipa derive macros at `/openapi.json`.

pub mod auth;
pub mod bootstrap;
pub mod compliance;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod openapi;
pub mod routes;
pub mod state;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
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

    // Authenticated API routes.
    //
    // Body size limit: 2 MiB. This prevents OOM from oversized request bodies.
    // Individual routes that need larger payloads (e.g., bulk import) can
    // override with a route-level DefaultBodyLimit.
    let api = Router::new()
        // Mass API proxy (all five primitives via Mass client)
        .merge(routes::mass_proxy::router())
        // SEZ Stack native routes (genuinely this codebase's domain)
        .merge(routes::corridors::router())
        .merge(routes::settlement::router())
        .merge(routes::smart_assets::router())
        .merge(routes::credentials::router())
        .merge(routes::regulator::router())
        .merge(routes::agentic::router())
        .merge(openapi::router())
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(from_fn(auth::auth_middleware))
        .layer(from_fn(middleware::rate_limit::rate_limit_middleware))
        .layer(from_fn(middleware::metrics::metrics_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(metrics))
        .layer(axum::Extension(limiter))
        .with_state(state.clone());

    // Unauthenticated health probes — readiness checks actual service health.
    let health = Router::new()
        .route("/health/liveness", axum::routing::get(liveness))
        .route("/health/readiness", axum::routing::get(readiness))
        .with_state(state);

    Router::new().merge(health).merge(api)
}

/// Liveness probe — always returns 200 if the process is running.
async fn liveness() -> &'static str {
    "ok"
}

/// Readiness probe — verifies the application is ready to serve traffic.
///
/// Checks:
/// - Zone signing key is loaded (can derive verifying key).
/// - Policy engine lock is acquirable (not deadlocked).
/// - In-memory stores are accessible.
///
/// Returns 200 "ready" or 503 with a diagnostic message.
async fn readiness(State(state): State<AppState>) -> impl IntoResponse {
    // Verify zone signing key is functional.
    let vk = state.zone_signing_key.verifying_key();
    if vk.to_hex().len() != 64 {
        return (StatusCode::SERVICE_UNAVAILABLE, "zone key degraded").into_response();
    }

    // Verify policy engine lock is acquirable (not deadlocked).
    // parking_lot::Mutex::try_lock is non-blocking.
    if state.policy_engine.try_lock().is_none() {
        return (StatusCode::SERVICE_UNAVAILABLE, "policy engine locked").into_response();
    }

    // Verify stores are accessible (read lock acquirable).
    let _ = state.corridors.len();
    let _ = state.smart_assets.len();
    let _ = state.attestations.len();

    (StatusCode::OK, "ready").into_response()
}
