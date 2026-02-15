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
    let config = AppConfig { port, auth_token };

    // Attempt to create Mass API client from environment.
    let mass_client = match msez_mass_client::MassApiConfig::from_env() {
        Ok(mass_config) => {
            tracing::info!("Mass API client configured");
            match msez_mass_client::MassClient::new(mass_config) {
                Ok(client) => Some(client),
                Err(e) => {
                    tracing::error!("Failed to create Mass API client: {e}");
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "Mass API client not configured: {e}. Primitive proxy endpoints will return 503."
            );
            None
        }
    };

    let state = AppState::try_with_config(config, mass_client).map_err(|e| {
        tracing::error!("Failed to initialize application state: {e}");
        e
    })?;
    let app = msez_api::app(state);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("MSEZ API listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
