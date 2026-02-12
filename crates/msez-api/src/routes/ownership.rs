//! # OWNERSHIP Primitive — Investment Info API
//!
//! Routes:
//! - POST   /v1/ownership/cap-table — Initialize cap table
//! - GET    /v1/ownership/{entity_id}/cap-table — Current cap table view
//! - POST   /v1/ownership/transfers — Record ownership transfer (triggers tax event)
//! - GET    /v1/ownership/{entity_id}/share-classes — Share class definitions

/// Placeholder for ownership router.
pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
}
