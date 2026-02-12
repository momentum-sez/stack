//! # msez-api — Binary Entry Point
//!
//! Starts the Axum HTTP server for the SEZ Stack API.
//! Binds to configurable port (default 8080).

use msez_api::state::{AppConfig, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize structured tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Build configuration from environment.
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let auth_token = std::env::var("AUTH_TOKEN").ok();

    let config = AppConfig {
        port,
        auth_token,
    };

    let state = AppState::with_config(config);
    let app = msez_api::app(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("MSEZ API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
