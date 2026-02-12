//! # IDENTITY Primitive — Identity Verification API
//!
//! Handles KYC/KYB verification, identity record management,
//! external ID linking (CNIC, NTN, passport), and identity attestations.
//! Supports NADRA CNIC cross-referencing as a verification method.

use axum::Router;

/// Build the identity router.
pub fn router() -> Router {
    Router::new()
}
