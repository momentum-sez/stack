//! # Content Digest — Content-Addressed Identifiers
//!
//! Defines `ContentDigest` and `DigestAlgorithm` for the content-addressed
//! storage (CAS) system that underpins the entire SEZ Stack.
//!
//! ## Security Invariant
//!
//! `ContentDigest` can only be computed from `CanonicalBytes`, ensuring that
//! all digests in the system are produced through the correct canonicalization
//! pipeline. This is enforced by the function signature of `sha256_digest()`.
//!
//! ## Implements
//!
//! Spec §8 — Content addressing and CAS naming conventions.
//! Audit §2.2 — DigestAlgorithm enum for SHA256/Poseidon2 forward compatibility.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical::CanonicalBytes;

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
    /// Gated behind the `poseidon2` feature flag. Digest computation is
    /// not yet implemented.
    Poseidon2,
}

impl DigestAlgorithm {
    /// Returns the algorithm identifier string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sha256 => "sha256",
            Self::Poseidon2 => "poseidon2",
        }
    }
}

impl std::fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A content-addressed digest with its algorithm tag.
///
/// Produced exclusively from `CanonicalBytes` via [`sha256_digest()`] to ensure
/// canonicalization correctness. The 32-byte digest and algorithm tag together
/// form a self-describing content identifier.
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
    /// Prefer [`sha256_digest()`] for constructing SHA256 digests
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
        write!(f, "{}:{}", self.algorithm, self.to_hex())
    }
}

/// Compute a SHA-256 content digest from canonical bytes.
///
/// This is the primary digest computation path for Phase 1. The function
/// signature enforces that only `CanonicalBytes` (produced through the
/// correct JCS coercion pipeline) can be hashed, preventing the
/// canonicalization split defect by construction.
///
/// The result carries a `DigestAlgorithm::Sha256` tag for forward
/// compatibility with Poseidon2 in Phase 2.
///
/// # Security Invariant
///
/// Accepts only `&CanonicalBytes`, not raw `&[u8]`. This compile-time
/// constraint prevents any code path from computing a digest over
/// non-canonical bytes.
///
/// Implements Spec §8 — SHA-256 content digest.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    let hash = Sha256::digest(data.as_bytes());
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&hash);
    ContentDigest::new(DigestAlgorithm::Sha256, bytes)
}

/// Compute a SHA-256 hex string from canonical bytes.
///
/// Convenience wrapper around [`sha256_digest()`] for contexts that need
/// the digest as a hex string (e.g., CAS artifact naming).
pub fn sha256_hex(data: &CanonicalBytes) -> String {
    sha256_digest(data).to_hex()
}

/// Compute a Poseidon2 content digest from canonical bytes.
///
/// # Panics
///
/// Always panics — Poseidon2 is not yet implemented. This function exists
/// behind the `poseidon2` feature flag as a forward-declaration for Phase 2
/// ZK proof system integration.
#[cfg(feature = "poseidon2")]
pub fn poseidon2_digest(_data: &CanonicalBytes) -> ContentDigest {
    unimplemented!("Poseidon2 digest is Phase 2 — ZK proof system integration pending")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_sha256_digest_deterministic() {
        let mut data = BTreeMap::new();
        data.insert("a", 1);
        data.insert("b", 2);
        let cb = CanonicalBytes::new(&data).unwrap();
        let d1 = sha256_digest(&cb);
        let d2 = sha256_digest(&cb);
        assert_eq!(d1, d2);
        assert_eq!(d1.algorithm, DigestAlgorithm::Sha256);
    }

    #[test]
    fn test_sha256_hex_format() {
        let data = serde_json::json!({"key": "value"});
        let cb = CanonicalBytes::new(&data).unwrap();
        let hex = sha256_hex(&cb);
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_content_digest_display() {
        let data = serde_json::json!({"a": 1});
        let cb = CanonicalBytes::new(&data).unwrap();
        let digest = sha256_digest(&cb);
        let s = format!("{digest}");
        assert!(s.starts_with("sha256:"));
        assert_eq!(s.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_digest_algorithm_display() {
        assert_eq!(DigestAlgorithm::Sha256.to_string(), "sha256");
        assert_eq!(DigestAlgorithm::Poseidon2.to_string(), "poseidon2");
    }

    #[test]
    fn test_different_inputs_different_digests() {
        let cb1 = CanonicalBytes::new(&serde_json::json!({"a": 1})).unwrap();
        let cb2 = CanonicalBytes::new(&serde_json::json!({"a": 2})).unwrap();
        assert_ne!(sha256_digest(&cb1), sha256_digest(&cb2));
    }

    #[test]
    fn test_known_sha256_vector() {
        // SHA256 of the empty JSON object "{}" is a known value.
        let cb = CanonicalBytes::new(&serde_json::json!({})).unwrap();
        assert_eq!(cb.as_bytes(), b"{}");
        let digest = sha256_digest(&cb);
        // SHA256("{}") — verified against Python hashlib.sha256(b"{}").hexdigest()
        assert_eq!(
            digest.to_hex(),
            "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
        );
    }
}
