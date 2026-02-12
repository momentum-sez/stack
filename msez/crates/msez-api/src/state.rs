//! # Application State
//!
//! Shared state for the Axum application, passed to all route handlers
//! via `State` extractor.

/// Shared application state accessible to all route handlers.
///
/// Contains service references, database pool, and configuration.
/// Cloneable for Axum's State extractor pattern.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Placeholder for application configuration.
    _config: AppConfig,
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self {
            _config: AppConfig::default(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Application configuration.
#[derive(Debug, Clone, Default)]
pub struct AppConfig {
    /// The port to listen on.
    pub port: u16,
}
