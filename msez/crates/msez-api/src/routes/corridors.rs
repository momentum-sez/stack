//! # Corridor Operations API
//!
//! Handles corridor lifecycle transitions, receipt queries,
//! fork resolution, anchor verification, and finality status.

use axum::Router;

/// Build the corridors router.
pub fn router() -> Router {
    Router::new()
}
