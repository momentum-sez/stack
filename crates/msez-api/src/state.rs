//! # Application State
//!
//! Shared state for the Axum application, including database pool
//! and service instances.

/// Shared application state passed to all route handlers.
///
/// Placeholder — full implementation will include:
/// - Database connection pool (`sqlx::PgPool`)
/// - Service instances for each domain
/// - Configuration
#[derive(Debug, Clone)]
pub struct AppState {
    _private: (),
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
