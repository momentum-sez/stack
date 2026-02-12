//! # Canonical Digest Bridge (CDB)
//!
//! Implements the CDB: `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.
//!
//! Phase 1 uses SHA256-only (no Poseidon2). The Poseidon2 step activates
//! in Phase 2 when the `poseidon2` feature flag is enabled.
//!
//! ## Security Invariant
//!
//! The input to the CDB is always `CanonicalBytes` — ensuring that the
//! JCS canonicalization step is enforced by the type system.
//!
//! ## Implements
//!
//! Spec §8 — Canonical Digest Bridge specification.

use msez_core::ContentDigest;

/// Compute the Canonical Digest Bridge for a content digest.
///
/// Phase 1: Returns the SHA256 digest unchanged (Poseidon2 not yet active).
/// Phase 2: Will apply `Poseidon2(Split256(digest))`.
pub fn canonical_digest_bridge(sha256_digest: &ContentDigest) -> ContentDigest {
    // Phase 1: pass-through (SHA256 only)
    // Phase 2: Apply Poseidon2(Split256(sha256_digest.bytes))
    sha256_digest.clone()
}
