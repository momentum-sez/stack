//! # OWNERSHIP Primitive — Investment Info API
//!
//! Handles cap table management, ownership transfers, share class
//! definitions, and capital gains event tracking.

use axum::Router;

/// Build the ownership router.
pub fn router() -> Router {
    Router::new()
}
