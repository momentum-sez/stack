//! # msez-api — Axum API Services
//!
//! The top-level API service layer for the SEZ Stack, built on Axum/Tower/Tokio.
//! Assembles the five programmable primitive routers into a single application
//! with shared middleware for authentication, tracing, metrics, and rate limiting.
//!
//! ## Five Primitive Routers
//!
//! - `/v1/entities/*` — ENTITIES primitive (Organization Info API)
//! - `/v1/ownership/*` — OWNERSHIP primitive (Investment Info API)
//! - `/v1/fiscal/*` — FISCAL primitive (Treasury Info API)
//! - `/v1/identity/*` — IDENTITY primitive (Identity verification)
//! - `/v1/consent/*` — CONSENT primitive (Multi-party consent)
//!
//! ## Additional Routes
//!
//! - `/v1/corridors/*` — Corridor operations
//! - `/v1/smart-assets/*` — Smart Asset CRUD + compliance evaluation
//! - `/v1/regulator/*` — Regulator console
//! - `/health/*` — Kubernetes health probes (unauthenticated)
//!
//! ## Middleware Stack (Tower)
//!
//! TraceLayer → MetricsLayer → RateLimitLayer → AuthLayer
//!
//! ## Architecture
//!
//! Request/response types are compile-time contracts via serde derive.
//! OpenAPI 3.1 specs are auto-generated from handler types via utoipa.
//! Database queries are compile-time verified via SQLx.
//!
//! ## Crate Policy
//!
//! - Sits at the top of the dependency DAG — depends on all other crates.
//! - No business logic in route handlers — delegates to domain crates.
//! - All errors map to structured HTTP responses via `AppError`.

pub mod auth;
pub mod error;
pub mod extractors;
pub mod middleware;
pub mod routes;
pub mod state;

pub use error::AppError;
pub use state::AppState;
