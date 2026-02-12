//! # SHA-256 Digest Computation
//!
//! Computes SHA-256 digests exclusively from `CanonicalBytes`, ensuring
//! that all digest paths flow through the canonicalization pipeline.
//!
//! ## Security Invariant
//!
//! The function signature `sha256_digest(data: &CanonicalBytes) -> ContentDigest`
//! makes it a compile error to pass raw bytes. This prevents the canonicalization
//! split defect by construction.

use msez_core::{CanonicalBytes, ContentDigest, DigestAlgorithm};
use sha2::{Digest, Sha256};

/// Compute a SHA-256 content digest from canonical bytes.
///
/// This is the primary digest computation path for Phase 1. The result
/// carries a `DigestAlgorithm::Sha256` tag for forward compatibility
/// with Poseidon2 in Phase 2.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    let hash = Sha256::digest(data.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash);
    ContentDigest::new(DigestAlgorithm::Sha256, bytes)
}

/// Compute a SHA-256 hex string from canonical bytes.
pub fn sha256_hex(data: &CanonicalBytes) -> String {
    sha256_digest(data).to_hex()
}
