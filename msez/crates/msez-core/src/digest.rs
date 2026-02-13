//! # Content-Addressed Digests
//!
//! Defines [`ContentDigest`] and [`DigestAlgorithm`] for the content-addressed
//! storage system. All digests carry an algorithm tag for forward migration
//! from SHA256 (Phase 1) to Poseidon2 (Phase 2).
//!
//! ## Security Invariant
//!
//! [`ContentDigest`] can only be computed via [`sha256_digest()`], which
//! accepts only `&CanonicalBytes`. This ensures every digest in the system
//! was produced from properly canonicalized data. There is no constructor
//! that accepts raw `[u8; 32]` in the public API outside of deserialization.
//!
//! ## Spec Reference
//!
//! Implements digest computation per `tools/lawpack.py:sha256_bytes()` and
//! the content-addressed storage scheme in `spec/`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::canonical::CanonicalBytes;

/// The hash algorithm used to compute a content-addressed digest.
///
/// Phase 1 uses SHA256 exclusively. Phase 2 introduces Poseidon2
/// for ZK-friendly hashing within arithmetic circuits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    /// SHA-256 — standard content addressing (Phase 1+).
    Sha256,
    /// Poseidon2 — ZK-friendly arithmetic-circuit-native hash (Phase 2).
    /// Gated behind the `poseidon2` feature flag.
    Poseidon2,
}

impl std::fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sha256 => write!(f, "sha256"),
            Self::Poseidon2 => write!(f, "poseidon2"),
        }
    }
}

/// A content-addressed digest with its algorithm tag.
///
/// The 32-byte digest and its algorithm are always stored together so that
/// verification code can select the correct hash function. This supports
/// forward migration from SHA256 to Poseidon2 without invalidating existing
/// content-addressed references.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentDigest {
    algorithm: DigestAlgorithm,
    bytes: [u8; 32],
}

impl ContentDigest {
    /// Access the digest algorithm.
    pub fn algorithm(&self) -> DigestAlgorithm {
        self.algorithm
    }

    /// Access the raw 32-byte digest value.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.bytes
    }

    /// Return the digest as a lowercase hex string.
    ///
    /// Matches Python's `hashlib.sha256(...).hexdigest()` output format.
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
/// This is the primary digest computation function for the SEZ Stack.
/// It accepts only [`CanonicalBytes`] — not raw `&[u8]` — to ensure
/// all digested data has been properly canonicalized.
///
/// # Security Invariant
///
/// The type signature `&CanonicalBytes` (not `&[u8]`) guarantees that the
/// input has passed through `CanonicalBytes::new()`, which applies float
/// rejection, datetime normalization, and key sorting. This eliminates
/// the "wrong serialization path" defect class at the type level.
///
/// # Example
///
/// ```
/// use msez_core::canonical::CanonicalBytes;
/// use msez_core::digest::sha256_digest;
/// use serde_json::json;
///
/// let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
/// let digest = sha256_digest(&canonical);
/// assert_eq!(digest.algorithm(), msez_core::digest::DigestAlgorithm::Sha256);
/// assert_eq!(digest.to_hex().len(), 64);
/// ```
pub fn sha256_digest(canonical: &CanonicalBytes) -> ContentDigest {
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    ContentDigest {
        algorithm: DigestAlgorithm::Sha256,
        bytes: result.into(),
    }
}

/// Compute a Poseidon2 content digest from canonical bytes.
///
/// **Phase 2 — not yet implemented.** This function is gated behind the
/// `poseidon2` feature flag and currently panics with `unimplemented!()`.
///
/// Poseidon2 is a ZK-friendly hash function designed for efficient verification
/// inside arithmetic circuits. It will be used for zero-knowledge proof
/// generation in the Phase 2 ZKP layer.
#[cfg(feature = "poseidon2")]
pub fn poseidon2_digest(_canonical: &CanonicalBytes) -> Result<ContentDigest, crate::error::MsezError> {
    Err(crate::error::MsezError::NotImplemented(
        "Poseidon2 digest computation available in Phase 4".into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sha256_digest_produces_64_hex_chars() {
        let canonical = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(digest.to_hex().len(), 64);
        assert!(digest.to_hex().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sha256_digest_is_deterministic() {
        let canonical = CanonicalBytes::new(&json!({"key": "value", "n": 42})).unwrap();
        let d1 = sha256_digest(&canonical);
        let d2 = sha256_digest(&canonical);
        assert_eq!(d1, d2);
    }

    #[test]
    fn sha256_digest_is_tagged_sha256() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
    }

    #[test]
    fn different_inputs_produce_different_digests() {
        let c1 = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let c2 = CanonicalBytes::new(&json!({"a": 2})).unwrap();
        assert_ne!(sha256_digest(&c1), sha256_digest(&c2));
    }

    #[test]
    fn display_format() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);
        let display = format!("{digest}");
        assert!(display.starts_with("sha256:"));
        assert_eq!(display.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    /// Verify against a known SHA-256 test vector.
    ///
    /// The canonical form of `{"a":1,"b":2}` is the UTF-8 bytes of that string.
    /// SHA-256 of those bytes is a fixed, known value.
    #[test]
    fn known_test_vector() {
        let value = json!({"b": 2, "a": 1});
        let canonical = CanonicalBytes::new(&value).unwrap();
        // Verify canonical bytes are what we expect
        assert_eq!(
            std::str::from_utf8(canonical.as_bytes()).unwrap(),
            r#"{"a":1,"b":2}"#
        );
        let digest = sha256_digest(&canonical);
        // SHA-256 of b'{"a":1,"b":2}' — computed independently
        // echo -n '{"a":1,"b":2}' | sha256sum
        let expected = "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777";
        assert_eq!(digest.to_hex(), expected);
    }

    #[test]
    fn digest_algorithm_display_sha256() {
        assert_eq!(format!("{}", DigestAlgorithm::Sha256), "sha256");
    }

    #[test]
    fn digest_algorithm_display_poseidon2() {
        assert_eq!(format!("{}", DigestAlgorithm::Poseidon2), "poseidon2");
    }

    #[test]
    fn content_digest_as_bytes_is_32() {
        let canonical = CanonicalBytes::new(&json!({"test": true})).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(digest.as_bytes().len(), 32);
    }

    #[test]
    fn content_digest_serde_roundtrip() {
        let canonical = CanonicalBytes::new(&json!({"key": "val"})).unwrap();
        let digest = sha256_digest(&canonical);
        let serialized = serde_json::to_string(&digest).unwrap();
        let deserialized: ContentDigest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(digest, deserialized);
    }

    #[test]
    fn digest_algorithm_clone_and_eq() {
        let alg = DigestAlgorithm::Sha256;
        let alg2 = alg;
        assert_eq!(alg, alg2);

        let alg3 = DigestAlgorithm::Poseidon2;
        assert_ne!(alg, alg3);
    }

    #[test]
    fn content_digest_hash_works() {
        use std::collections::HashSet;
        let c1 = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let c2 = CanonicalBytes::new(&json!({"a": 2})).unwrap();
        let d1 = sha256_digest(&c1);
        let d2 = sha256_digest(&c2);
        let mut set = HashSet::new();
        set.insert(d1.clone());
        set.insert(d2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&d1));
    }

    // ── Additional coverage expansion tests ─────────────────────────

    #[test]
    fn sha256_digest_of_empty_object() {
        let c = CanonicalBytes::new(&json!({})).unwrap();
        let d = sha256_digest(&c);
        assert_eq!(d.algorithm(), DigestAlgorithm::Sha256);
        assert_eq!(d.to_hex().len(), 64);
        // Should match: printf '{}' | sha256sum
        let expected = "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";
        assert_eq!(d.to_hex(), expected);
    }

    #[test]
    fn sha256_digest_of_null() {
        let c = CanonicalBytes::new(&json!(null)).unwrap();
        let d = sha256_digest(&c);
        assert_eq!(d.to_hex().len(), 64);
    }

    #[test]
    fn sha256_digest_of_string() {
        let c = CanonicalBytes::new(&json!("hello")).unwrap();
        let d = sha256_digest(&c);
        assert_eq!(d.to_hex().len(), 64);
    }

    #[test]
    fn content_digest_debug_format() {
        let c = CanonicalBytes::new(&json!({})).unwrap();
        let d = sha256_digest(&c);
        let debug = format!("{d:?}");
        assert!(debug.contains("ContentDigest"));
        assert!(debug.contains("Sha256"));
    }
}
