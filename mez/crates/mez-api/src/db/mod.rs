//! # Database Persistence Layer
//!
//! Provides Postgres persistence for EZ-Stack-owned state via SQLx.
//!
//! ## Architecture
//!
//! The database layer is **optional**. When `DATABASE_URL` is set, the API
//! persists corridor state, smart assets, attestations, tensor snapshots,
//! and audit events to PostgreSQL. When absent, the API operates in
//! in-memory-only mode (suitable for development and testing).
//!
//! ## What is persisted (EZ Stack owned)
//!
//! - Corridor lifecycle state and receipt chains
//! - Smart asset records
//! - Compliance attestations
//! - Tensor evaluation snapshots
//! - Audit event log (immutable hash chain)
//!
//! ## What is NOT persisted here
//!
//! Entity, ownership, fiscal, identity, and consent data lives in the
//! Mass APIs and is accessed via `mez-mass-client`. See CLAUDE.md Section II.

pub mod attestations;
pub mod audit;
pub mod corridors;
pub mod mass_primitives;
pub mod smart_assets;
pub mod tax_events;
pub mod trade;

use sqlx::postgres::{PgPool, PgPoolOptions};

/// Initialize the database connection pool and run migrations.
///
/// Returns `None` if `DATABASE_URL` is not set (in-memory-only mode).
/// Returns `Err` if the URL is set but the connection or migration fails.
pub async fn init_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let url = match std::env::var("DATABASE_URL") {
        Ok(url) => url,
        Err(_) => {
            tracing::warn!(
                "DATABASE_URL not set — running in-memory only mode. \
                 State will not survive restarts."
            );
            return Ok(None);
        }
    };

    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&url)
        .await?;

    tracing::info!("Connected to PostgreSQL");

    // Run embedded migrations.
    sqlx::migrate!("./migrations").run(&pool).await?;
    tracing::info!("Database migrations applied");

    Ok(Some(pool))
}
