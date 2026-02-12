//! # CONSENT Primitive — Consent Info API
//!
//! Handles multi-party consent requests, consent signing,
//! and full audit trail for consent lifecycle.

use axum::Router;

/// Build the consent router.
pub fn router() -> Router {
    Router::new()
}
