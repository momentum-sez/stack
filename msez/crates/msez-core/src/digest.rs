//! # Content-Addressed Digests
//!
//! Defines [`ContentDigest`] and [`DigestAlgorithm`] for the content-addressed
//! storage system. All digests carry an algorithm tag for forward migration
//! from SHA256 (Phase 1) to Poseidon2 (Phase 2).
//!
//! ## Security Invariant
//!
//! `ContentDigest` can only be computed from [`CanonicalBytes`][crate::CanonicalBytes].
//! This ensures every digest in the system was produced from properly
//! canonicalized data.

use serde::{Deserialize, Serialize};

/// The hash algorithm used to compute a content-addressed digest.
///
/// Phase 1 uses SHA256 exclusively. Phase 2 introduces Poseidon2
/// for ZK-friendly hashing within arithmetic circuits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    /// SHA-256 — standard content addressing (Phase 1+).
    Sha256,
    /// Poseidon2 — ZK-friendly arithmetic-circuit-native hash (Phase 2).
    Poseidon2,
}

/// A content-addressed digest with its algorithm tag.
///
/// The 32-byte digest and its algorithm are always stored together so that
/// verification code can select the correct hash function. This supports
/// forward migration from SHA256 to Poseidon2 without invalidating existing
/// content-addressed references.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentDigest {
    /// The hash algorithm that produced this digest.
    pub algorithm: DigestAlgorithm,
    /// The raw 32-byte digest value.
    pub bytes: [u8; 32],
}

impl ContentDigest {
    /// Create a new SHA256 content digest from raw bytes.
    pub fn sha256(bytes: [u8; 32]) -> Self {
        Self {
            algorithm: DigestAlgorithm::Sha256,
            bytes,
        }
    }

    /// Return the digest as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}:{}", self.algorithm, self.to_hex())
    }
}
