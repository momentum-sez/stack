//! # Poseidon2 Hash Function — Phase 4 Stub
//!
//! Poseidon2 is a ZK-friendly hash function designed for efficient
//! verification inside arithmetic circuits. It will replace SHA-256 for
//! proof-internal hashing when the ZKP layer activates in Phase 4.
//!
//! ## Current Status
//!
//! This module is gated behind the `poseidon2` Cargo feature flag.
//! All public functions have correct type signatures but return
//! `Err(CryptoError::NotImplemented)` at runtime. This allows downstream
//! crates to write code that references Poseidon2 types and compile-check
//! it without a concrete implementation.
//!
//! ## Activation Plan
//!
//! Phase 4 will provide a concrete Poseidon2 implementation via an
//! external crate (e.g., `poseidon2-plonky2` or equivalent). At that
//! point, the error-returning stubs will be replaced with real
//! hash computations and the feature flag will be enabled by default.
//!
//! ## Spec Reference
//!
//! See `spec/` Phase 4 ZKP chapters for the Poseidon2 parameter
//! selection and domain separation conventions.

use mez_core::CanonicalBytes;

/// A Poseidon2 digest (32 bytes), analogous to SHA-256 but ZK-friendly.
///
/// ## Phase 4
///
/// This type exists for forward-compatible API design. The internal
/// representation may change when the concrete Poseidon2 implementation
/// lands.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Poseidon2Digest {
    bytes: [u8; 32],
}

impl Poseidon2Digest {
    /// Access the raw 32-byte digest value.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Return the digest as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// Compute a Poseidon2 digest from canonical bytes.
///
/// ## Phase 4 — Not Yet Implemented
///
/// This function will compute a ZK-friendly Poseidon2 hash suitable
/// for use inside arithmetic circuits.
///
/// The input must be [`CanonicalBytes`] to maintain the same
/// canonicalization invariant as SHA-256 digest computation.
///
/// ### What exists today
///
/// - `Poseidon2Digest` type: 32-byte digest with `as_bytes()`, `to_hex()`.
/// - `mez-zkp::cdb::Cdb::new()` has a `#[cfg(feature = "poseidon2")]`
///   codepath that currently falls back to identity — it will call this
///   function once Poseidon2 lands.
/// - SHA-256 canonicalization pipeline (`CanonicalBytes` → `sha256_digest`)
///   is production-grade; Poseidon2 slots in as the final step.
///
/// ### Phase 4 integration steps
///
/// 1. Add `poseidon2` crate (e.g., `poseidon2-plonky2`) to
///    `mez-crypto/Cargo.toml` behind the `poseidon2` feature.
/// 2. Implement this function: instantiate the Poseidon2 permutation
///    with the chosen domain separation, hash `CanonicalBytes`, return
///    `Poseidon2Digest`.
/// 3. Implement `poseidon2_node_hash()` for Merkle node hashing.
/// 4. Update `mez-zkp::cdb::Cdb::new()` to call `poseidon2_digest()`
///    via `Split256` instead of falling back to identity.
/// 5. Enable the `poseidon2` feature by default in workspace.
pub fn poseidon2_digest(
    _data: &CanonicalBytes,
) -> Result<Poseidon2Digest, crate::error::CryptoError> {
    Err(crate::error::CryptoError::NotImplemented(
        "Poseidon2 digest available in Phase 4".into(),
    ))
}

/// Compute a Poseidon2 hash over two 32-byte inputs (for Merkle nodes).
///
/// ## Phase 4 — Not Yet Implemented
///
/// This is the ZK-friendly equivalent of the SHA-256 node hash used
/// in MMR construction. When implemented, `mez-crypto::mmr` can offer
/// a Poseidon2 MMR variant for proofs that verify inside arithmetic
/// circuits without SHA-256 gadget overhead.
///
/// ### Phase 4 integration steps
///
/// 1. Implement: concatenate left + right, hash via Poseidon2 permutation.
/// 2. Add a `Poseidon2Mmr` type or feature-gated codepath in `mez-crypto::mmr`.
/// 3. Wire into `mez-zkp::circuits::settlement::MerkleMembershipCircuit`.
pub fn poseidon2_node_hash(
    _left: &[u8; 32],
    _right: &[u8; 32],
) -> Result<Poseidon2Digest, crate::error::CryptoError> {
    Err(crate::error::CryptoError::NotImplemented(
        "Poseidon2 node hashing available in Phase 4".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poseidon2_digest_returns_not_implemented() {
        let data = CanonicalBytes::new(&serde_json::json!({"test": true})).unwrap();
        let result = poseidon2_digest(&data);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }

    #[test]
    fn poseidon2_node_hash_returns_not_implemented() {
        let result = poseidon2_node_hash(&[0u8; 32], &[1u8; 32]);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }
}
