//! # mez-api — Axum API Services for the EZ Stack
//!
//! The EZ Stack is the orchestration layer above the Mass APIs.
//! It provides compliance tensor evaluation, corridor lifecycle management,
//! smart asset operations, regulator console, and a proxy layer to the
//! live Mass APIs for primitive operations (entities, ownership, fiscal,
//! identity, consent).
//!
//! ## API Surface
//!
//! | Prefix               | Module                      | Domain              |
//! |-----------------------|----------------------------|---------------------|
//! | `/v1/entities/*`      | [`routes::mass_proxy`]     | Entities (orchestrated) |
//! | `/v1/ownership/*`     | [`routes::mass_proxy`]     | Ownership (orchestrated) |
//! | `/v1/fiscal/*`        | [`routes::mass_proxy`]     | Fiscal (orchestrated) |
//! | `/v1/identity/*`      | [`routes::mass_proxy`]     | Identity (orchestrated) |
//! | `/v1/identity/cnic/*` | [`routes::identity`]       | NADRA CNIC verification |
//! | `/v1/identity/ntn/*`  | [`routes::identity`]       | FBR IRIS NTN verification |
//! | `/v1/identity/entity/*` | [`routes::identity`]     | Consolidated identity |
//! | `/v1/consent/*`       | [`routes::mass_proxy`]     | Consent (orchestrated) |
//! | `/v1/tax/*`           | [`routes::tax`]            | Tax pipeline (EZ)  |
//! | `/v1/corridors/*`     | [`routes::corridors`]      | Corridors (EZ)     |
//! | `/v1/assets/*`        | [`routes::smart_assets`]   | Smart Assets (EZ)  |
//! | `/v1/assets/*/credentials/*` | [`routes::credentials`] | VC Issuance (EZ) |
//! | `/v1/credentials/*`  | [`routes::credentials`]    | VC Verification (EZ) |
//! | `/v1/triggers`        | [`routes::agentic`]        | Agentic Engine (EZ)|
//! | `/v1/policies/*`      | [`routes::agentic`]        | Policy Mgmt (EZ)   |
//! | `/v1/tax/*`           | [`routes::tax`]            | Tax Pipeline (EZ)  |
//! | `/v1/regulator/*`     | [`routes::regulator`]      | Regulator (EZ)     |
//! | `/v1/govos/*`         | [`routes::govos`]          | GovOS Console (EZ) |
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
pub mod bootstrap;
pub mod compliance;
pub mod db;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod openapi;
pub mod orchestration;
pub mod routes;
pub mod state;

use std::collections::HashMap;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::StatusCode;
use axum::middleware::from_fn;
use axum::response::IntoResponse;
use axum::{Extension, Router};
use tower_http::trace::TraceLayer;

use crate::auth::AuthConfig;
use crate::middleware::metrics::ApiMetrics;
use crate::middleware::rate_limit::{RateLimitConfig, RateLimiter};
use crate::state::AppState;

/// Check if metrics are enabled via the `MEZ_METRICS_ENABLED` env var.
/// Defaults to `true` when the variable is absent or set to anything other than `"false"`.
fn metrics_enabled() -> bool {
    std::env::var("MEZ_METRICS_ENABLED")
        .map(|v| v.to_lowercase() != "false")
        .unwrap_or(true)
}

/// Assemble the full application router with all routes and middleware.
///
/// Health probes (`/health/*`) and `/metrics` are mounted outside the auth
/// middleware so they remain accessible without credentials.
pub fn app(state: AppState) -> Router {
    let auth_config = AuthConfig {
        token: state.config.auth_token.clone(),
    };
    let metrics = ApiMetrics::new();
    let limiter = RateLimiter::new(RateLimitConfig::default());
    let metrics_on = metrics_enabled();

    // Authenticated API routes.
    //
    // Body size limit: 2 MiB. This prevents OOM from oversized request bodies.
    // Individual routes that need larger payloads (e.g., bulk import) can
    // override with a route-level DefaultBodyLimit.
    //
    // Middleware execution order (outermost → innermost):
    //   TraceLayer → MetricsMiddleware → AuthMiddleware → RateLimitMiddleware → Handler
    //
    // Auth runs BEFORE rate limiting so unauthenticated requests are rejected
    // without consuming rate limit quota (prevents DoS via auth bypass).
    // Sovereign Mass mode: serve Mass primitives directly from in-memory stores
    // backed by Postgres. Proxy mode: delegate to centralized Mass APIs.
    let mass_routes = if state.sovereign_mass {
        tracing::info!("Sovereign Mass mode enabled — serving Mass primitives locally");
        routes::mass_sovereign::sovereign_mass_router()
    } else {
        routes::mass_proxy::router()
    };

    let api = Router::new()
        .merge(mass_routes)
        // Identity orchestration — P1-005: CNIC/NTN verification,
        // consolidated entity identity, service status.
        .merge(routes::identity::router())
        // Tax collection pipeline — P1-009: withholding computation,
        // tax event recording, reporting.
        .merge(routes::tax::router())
        // EZ Stack native routes (genuinely this codebase's domain)
        .merge(routes::corridors::router())
        .merge(routes::settlement::router())
        .merge(routes::smart_assets::router())
        .merge(routes::credentials::router())
        .merge(routes::regulator::router())
        .merge(routes::agentic::router())
        .merge(routes::peers::router())
        .merge(openapi::router());

    // GovOS Console — M-009: Pakistan sovereign deployment dashboards
    // (GovOS Console, Tax & Revenue, Digital Free Zone, Citizen Services).
    // Gated behind `jurisdiction-pk` feature — not compiled for non-Pakistan zones.
    #[cfg(feature = "jurisdiction-pk")]
    let api = api.merge(routes::govos::router());

    let mut api = api
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .layer(from_fn(middleware::rate_limit::rate_limit_middleware))
        .layer(from_fn(auth::auth_middleware));

    // Only register the metrics middleware when metrics are enabled.
    if metrics_on {
        api = api
            .layer(from_fn(middleware::metrics::metrics_middleware))
            .layer(axum::Extension(metrics.clone()));
    }

    let api = api
        .layer(TraceLayer::new_for_http())
        .layer(axum::Extension(auth_config))
        .layer(axum::Extension(limiter))
        .with_state(state.clone());

    // Unauthenticated health probes — readiness checks actual service health.
    let mut unauthenticated = Router::new()
        .route("/health/liveness", axum::routing::get(liveness))
        .route("/health/readiness", axum::routing::get(readiness));

    // Mount /metrics endpoint when metrics are enabled (unauthenticated, like health probes).
    if metrics_on {
        unauthenticated = unauthenticated
            .route("/metrics", axum::routing::get(prometheus_metrics))
            .layer(axum::Extension(metrics));
    }

    let unauthenticated = unauthenticated.with_state(state);

    Router::new().merge(unauthenticated).merge(api)
}

/// GET /metrics — Prometheus metrics scrape endpoint.
///
/// Updates domain gauges from current `AppState` on each scrape (pull model),
/// then gathers and encodes all metrics in Prometheus text exposition format.
async fn prometheus_metrics(
    State(state): State<AppState>,
    Extension(metrics): Extension<ApiMetrics>,
) -> impl IntoResponse {
    // -- Update domain gauges from AppState --

    // Corridors by state.
    let corridors = state.corridors.list();
    let mut by_state: HashMap<String, usize> = HashMap::new();
    for c in &corridors {
        *by_state.entry(c.state.as_str().to_string()).or_default() += 1;
    }
    // Reset all corridor state labels, then set current values.
    metrics.corridors_total().reset();
    for (st, count) in &by_state {
        metrics
            .corridors_total()
            .with_label_values(&[st])
            .set(*count as f64);
    }

    // Receipt chain heights per corridor.
    metrics.corridor_receipt_chain_height().reset();
    let mut total_receipts: u64 = 0;
    {
        let chains = state.receipt_chains.read();
        for c in &corridors {
            if let Some(chain) = chains.get(&c.id) {
                let h = chain.height();
                metrics
                    .corridor_receipt_chain_height()
                    .with_label_values(&[&c.id.to_string()])
                    .set(h as f64);
                total_receipts += h;
            }
        }
    }
    metrics
        .receipt_chain_total_receipts()
        .set(total_receipts as f64);

    // Assets by compliance status.
    let assets = state.smart_assets.list();
    let mut compliant = 0usize;
    let mut non_compliant = 0usize;
    let mut pending = 0usize;
    let mut unevaluated = 0usize;
    let mut partially_compliant = 0usize;
    for a in &assets {
        match a.compliance_status {
            state::AssetComplianceStatus::Compliant => compliant += 1,
            state::AssetComplianceStatus::NonCompliant => non_compliant += 1,
            state::AssetComplianceStatus::Pending => pending += 1,
            state::AssetComplianceStatus::Unevaluated => unevaluated += 1,
            state::AssetComplianceStatus::PartiallyCompliant => partially_compliant += 1,
        }
    }
    metrics.assets_total().reset();
    metrics
        .assets_total()
        .with_label_values(&["compliant"])
        .set(compliant as f64);
    metrics
        .assets_total()
        .with_label_values(&["non_compliant"])
        .set(non_compliant as f64);
    metrics
        .assets_total()
        .with_label_values(&["pending"])
        .set(pending as f64);
    metrics
        .assets_total()
        .with_label_values(&["unevaluated"])
        .set(unevaluated as f64);
    metrics
        .assets_total()
        .with_label_values(&["partially_compliant"])
        .set(partially_compliant as f64);

    // Attestations total.
    metrics
        .attestations_total()
        .set(state.attestations.len() as f64);

    // Peers by status.
    {
        let registry = state.peer_registry.read();
        let peers = registry.list_peers();
        let mut peer_counts: HashMap<&str, usize> = HashMap::new();
        for p in &peers {
            let status_str = match p.status {
                mez_corridor::PeerStatus::Discovered => "discovered",
                mez_corridor::PeerStatus::Proposing => "proposing",
                mez_corridor::PeerStatus::Active => "active",
                mez_corridor::PeerStatus::Unreachable => "unreachable",
                mez_corridor::PeerStatus::Disconnected => "disconnected",
            };
            *peer_counts.entry(status_str).or_default() += 1;
        }
        metrics.peers_total().reset();
        for (status, count) in &peer_counts {
            metrics
                .peers_total()
                .with_label_values(&[status])
                .set(*count as f64);
        }
    }

    // Policies and audit trail.
    {
        let engine = state.policy_engine.lock();
        metrics.policies_total().set(engine.policy_count() as f64);
        metrics
            .audit_trail_entries_total()
            .set(engine.audit_trail.len() as f64);
    }

    // Zone key ephemeral.
    let ephemeral = match &state.zone {
        Some(zc) => zc.key_ephemeral,
        None => std::env::var("ZONE_SIGNING_KEY_HEX").is_err(),
    };
    metrics
        .zone_key_ephemeral()
        .set(if ephemeral { 1.0 } else { 0.0 });

    // -- Gather and encode --
    match metrics.gather_and_encode() {
        Ok(body) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            body,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("Failed to encode Prometheus metrics: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, e).into_response()
        }
    }
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
/// - Database connection is healthy (when configured).
/// - Mass API connectivity (when client is configured).
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

    // Verify database connection (when configured).
    if let Some(pool) = &state.db_pool {
        if let Err(e) = sqlx::query("SELECT 1").execute(pool).await {
            tracing::warn!("Database health check failed: {e}");
            return (StatusCode::SERVICE_UNAVAILABLE, "database unreachable").into_response();
        }
    }

    // Verify Mass API connectivity (when client is configured).
    // If mass_client is None, the server already returns 503 on proxy routes,
    // so the readiness probe passes — the zone may intentionally run without Mass.
    if let Some(mass_client) = &state.mass_client {
        let result = mass_client.health_check().await;
        if !result.all_healthy() {
            let services: Vec<String> = result
                .unreachable
                .iter()
                .map(|(name, err)| format!("{name}: {err}"))
                .collect();
            let msg = format!("mass api unreachable: {}", services.join("; "));
            tracing::warn!("{}", msg);
            return (StatusCode::SERVICE_UNAVAILABLE, msg).into_response();
        }
    }

    (StatusCode::OK, "ready").into_response()
}
