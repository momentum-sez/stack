//! # ENTITIES Primitive — Organization Info API
//!
//! Routes:
//! - POST   /v1/entities — Create entity (formation)
//! - GET    /v1/entities/{entity_id} — Get entity details
//! - PUT    /v1/entities/{entity_id}/status — Update lifecycle status
//! - GET    /v1/entities/{entity_id}/beneficial-owners — Beneficial ownership registry
//! - POST   /v1/entities/{entity_id}/dissolution/initiate — Begin 10-stage dissolution
//! - GET    /v1/entities/{entity_id}/dissolution/status — Dissolution stage query

/// Placeholder for entities router.
pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
}
