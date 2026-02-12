//! # Tensor Commitment
//!
//! Content-addressed commitment to a compliance tensor state, computed
//! via [`CanonicalBytes`](msez_core::CanonicalBytes) → SHA-256 digest.
//!
//! ## Security Invariant
//!
//! The commitment is computed using `CanonicalBytes::new()`, never raw
//! `serde_json::to_vec()`. This ensures cross-layer digest agreement
//! (audit finding §2.1).

use msez_core::ContentDigest;

/// A cryptographic commitment to a compliance tensor state.
///
/// Computed by canonicalizing the tensor state via `CanonicalBytes::new()`
/// and then applying SHA-256. The commitment can be included in VCs
/// and corridor receipts.
#[derive(Debug, Clone)]
pub struct TensorCommitment {
    /// The content digest of the canonicalized tensor state.
    pub digest: ContentDigest,
}
