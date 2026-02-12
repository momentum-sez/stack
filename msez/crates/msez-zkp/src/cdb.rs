//! # Canonical Digest Bridge (CDB)
//!
//! The CDB bridges content-addressed SHA256 digests to ZK-friendly
//! Poseidon2 digests: `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.
//!
//! Phase 1: SHA256-only (Poseidon2 is a no-op identity).
//! Phase 2: Full Poseidon2 implementation via feature flag.

use msez_core::ContentDigest;

/// Compute the Canonical Digest Bridge for a content digest.
///
/// In Phase 1, this returns the input digest unchanged (SHA256-only).
/// In Phase 2, this applies Poseidon2 over the Split256 decomposition.
pub fn canonical_digest_bridge(digest: &ContentDigest) -> ContentDigest {
    // Phase 1: identity function — pass through SHA256 digest.
    // Phase 2 will apply Poseidon2(Split256(digest)).
    digest.clone()
}
