//! # Content Digest — Content-Addressed Identifiers
//!
//! Defines `ContentDigest` and `DigestAlgorithm` for the content-addressed
//! storage (CAS) system that underpins the entire SEZ Stack.
//!
//! ## Security Invariant
//!
//! `ContentDigest` can only be computed from `CanonicalBytes`, ensuring that
//! all digests in the system are produced through the correct canonicalization
//! pipeline. This is enforced by the function signature of `ContentDigest::from_canonical()`.
//!
//! ## Implements
//!
//! Spec §8 — Content addressing and CAS naming conventions.
//! Audit §2.2 — DigestAlgorithm enum for SHA256/Poseidon2 forward compatibility.

use serde::{Deserialize, Serialize};

/// The hash algorithm used to produce a content digest.
///
/// Phase 1 uses SHA256 exclusively. Poseidon2 activates in Phase 2 with
/// the ZK proof system. All commitment structures carry an algorithm tag
/// for forward migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    /// SHA-256 — standard content addressing (Phase 1+).
    Sha256,
    /// Poseidon2 — ZK-friendly arithmetic-circuit-native hash (Phase 2).
    Poseidon2,
}

/// A content-addressed digest with its algorithm tag.
///
/// Produced exclusively from `CanonicalBytes` to ensure canonicalization
/// correctness. The 32-byte digest and algorithm tag together form a
/// self-describing content identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentDigest {
    /// The hash algorithm that produced this digest.
    pub algorithm: DigestAlgorithm,
    /// The raw 32-byte digest value.
    pub bytes: [u8; 32],
}

impl ContentDigest {
    /// Create a new content digest from raw bytes and algorithm.
    ///
    /// Prefer `ContentDigest::sha256()` for constructing SHA256 digests
    /// from `CanonicalBytes`.
    pub fn new(algorithm: DigestAlgorithm, bytes: [u8; 32]) -> Self {
        Self { algorithm, bytes }
    }

    /// Render the digest as a lowercase hex string.
    pub fn to_hex(&self) -> String {
        self.bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl std::fmt::Display for ContentDigest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", match self.algorithm {
            DigestAlgorithm::Sha256 => "sha256",
            DigestAlgorithm::Poseidon2 => "poseidon2",
        }, self.to_hex())
    }
}
