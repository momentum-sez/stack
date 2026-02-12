//! # IDENTITY Primitive
//!
//! Supports NADRA CNIC cross-referencing as a verification method.
//!
//! Routes:
//! - POST   /v1/identity/verify — KYC/KYB verification request
//! - GET    /v1/identity/{identity_id} — Identity record
//! - POST   /v1/identity/link — Link external ID (CNIC, NTN, passport)
//! - POST   /v1/identity/attestation — Submit identity attestation

/// Placeholder for identity router.
pub fn router() -> axum::Router<crate::AppState> {
    axum::Router::new()
}
