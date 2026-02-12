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

use msez_core::{CanonicalBytes, ContentDigest};
use sha2::{Digest, Sha256};

/// Compute a SHA-256 content digest from canonical bytes.
///
/// This is the standard digest computation path for Phase 1.
/// The input must be [`CanonicalBytes`] — raw byte slices are not accepted.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result);
    ContentDigest::sha256(bytes)
}
