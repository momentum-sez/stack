//! # msez-api — Axum API Services for the SEZ Stack
//!
//! Assembles the five programmable primitive API services into a single
//! Axum application with Tower middleware for authentication, tracing,
//! metrics, and rate limiting.
//!
//! ## API Surface
//!
//! | Prefix             | Module               | Primitive      |
//! |--------------------|----------------------|----------------|
//! | `/v1/entities/*`   | [`routes::entities`] | ENTITIES       |
//! | `/v1/ownership/*`  | [`routes::ownership`]| OWNERSHIP      |
//! | `/v1/fiscal/*`     | [`routes::fiscal`]   | FISCAL         |
//! | `/v1/identity/*`   | [`routes::identity`] | IDENTITY       |
//! | `/v1/consent/*`    | [`routes::consent`]  | CONSENT        |
//! | `/v1/corridors/*`  | [`routes::corridors`]| Corridors      |
//! | `/v1/smart-assets/*` | [`routes::smart_assets`] | Smart Assets |
//! | `/v1/regulator/*`  | [`routes::regulator`]| Regulator      |
//!
//! ## Middleware Stack
//!
//! ```text
//! TraceLayer → MetricsLayer → RateLimitLayer → AuthLayer
//! ```
//!
//! ## Database
//!
//! PostgreSQL via SQLx with compile-time query verification.
//!
//! ## OpenAPI
//!
//! Auto-generated OpenAPI 3.1 specs via utoipa derive macros.

pub mod error;
pub mod routes;
pub mod state;
