//! # ENTITIES Primitive — Organization Info API
//!
//! Handles entity formation, lifecycle management, beneficial ownership
//! registry, and the 10-stage dissolution process.

use axum::Router;

/// Build the entities router.
pub fn router() -> Router {
    Router::new()
}
