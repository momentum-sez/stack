// SPDX-License-Identifier: BUSL-1.1
//! Sovereign Mass API stub server — standalone development server.
//!
//! In-memory implementation of the Mass API endpoints that `mez-mass-client`
//! calls. Designed for per-zone deployment so each economic zone operates
//! against its own data store, achieving sovereign data residency.
//!
//! **For production sovereign deployments, use `mez-api` with
//! `SOVEREIGN_MASS=true` which provides Postgres-backed persistence
//! (ADR-007). This standalone stub is for development and testing
//! without a database.**
//!
//! Storage is in-memory (DashMap) with no persistence — data is lost on
//! restart.

mod routes;
mod store;

use std::net::SocketAddr;

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let port: u16 = std::env::var("MASS_STUB_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8090);

    let state = store::AppState::new();
    let app = routes::router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("mez-mass-stub listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind listener");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("server error");
}
