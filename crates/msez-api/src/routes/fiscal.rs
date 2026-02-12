//! # FISCAL Primitive — Treasury Info API
//!
//! Critical API for FBR IRIS integration. Supports NTN (National Tax Number)
//! as a first-class identifier.
//!
//! Routes:
//! - POST   /v1/fiscal/accounts — Create treasury account
//! - POST   /v1/fiscal/payments — Initiate payment
//! - POST   /v1/fiscal/withholding/calculate — Compute withholding at source
//! - GET    /v1/fiscal/{entity_id}/tax-events — Tax event history
//! - POST   /v1/fiscal/reporting/generate — Generate tax return data

/// Placeholder for fiscal router.
pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
}
