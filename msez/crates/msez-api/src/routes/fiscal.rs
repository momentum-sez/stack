//! # FISCAL Primitive — Treasury Info API
//!
//! Handles treasury accounts, payments, withholding calculation,
//! tax event history, and reporting generation.
//! Critical for FBR IRIS integration with NTN as first-class identifier.

use axum::Router;

/// Build the fiscal router.
pub fn router() -> Router {
    Router::new()
}
