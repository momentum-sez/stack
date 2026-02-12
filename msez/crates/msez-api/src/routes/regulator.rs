//! # Regulator Console API
//!
//! Provides read-only query access for regulatory authorities
//! to monitor zone activity, compliance status, and audit trails.

use axum::Router;

/// Build the regulator router.
pub fn router() -> Router {
    Router::new()
}
