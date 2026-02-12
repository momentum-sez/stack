//! # SHA-256 Digest Computation
//!
//! Computes [`ContentDigest`] values from [`CanonicalBytes`]. This is the
//! only sanctioned path for producing content-addressed digests in Phase 1.
//!
//! ## Security Invariant
//!
//! The function signature requires `CanonicalBytes` — not raw `&[u8]`.
//! This ensures that every digest was computed from properly canonicalized
//! data, preventing the canonicalization split (audit finding §2.1).

use msez_core::{sha256_digest as core_sha256_digest, CanonicalBytes, ContentDigest};

/// Compute a SHA-256 content digest from canonical bytes.
///
/// This is the standard digest computation path for Phase 1.
/// The input must be [`CanonicalBytes`] — raw byte slices are not accepted.
///
/// Delegates to [`msez_core::sha256_digest()`] — the single implementation
/// in the workspace.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    core_sha256_digest(data)
}
