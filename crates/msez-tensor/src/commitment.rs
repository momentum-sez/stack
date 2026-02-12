//! # Tensor Commitment
//!
//! Content-addressed commitment generation for compliance tensor states.
//! All commitments flow through `CanonicalBytes` → SHA-256.
//!
//! ## Security Invariant
//!
//! Commitments are computed via `msez_crypto::sha256_digest()` from
//! `CanonicalBytes`, not from raw `serde_json::to_vec()`.
//!
//! ## Implements
//!
//! Spec §12 — Tensor commitment and verification.

use msez_core::ContentDigest;

/// A content-addressed commitment to a compliance tensor state.
///
/// Placeholder — full implementation will serialize the tensor state
/// via `CanonicalBytes::new()` and compute the SHA-256 digest.
#[derive(Debug, Clone)]
pub struct TensorCommitment {
    /// The content digest of the committed tensor state.
    pub digest: ContentDigest,
}
