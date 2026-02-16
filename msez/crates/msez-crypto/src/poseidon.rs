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

use msez_core::CanonicalBytes;

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
/// for use inside arithmetic circuits. It currently panics.
///
/// The input must be [`CanonicalBytes`] to maintain the same
/// canonicalization invariant as SHA-256 digest computation.
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
/// in MMR construction.
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
