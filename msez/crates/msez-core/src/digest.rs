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
//! ## SHA-256 Centralization
//!
//! **All SHA-256 computation in the SEZ Stack flows through this module.**
//! No other crate may import `sha2` directly. Three entry points are provided:
//!
//! - [`sha256_digest()`] — canonical JSON → [`ContentDigest`] (primary path)
//! - [`Sha256Accumulator`] — streaming SHA-256 for domain-separated / multi-part hashing
//! - [`sha256_raw()`] — single-shot raw byte hashing (returns hex string)
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

    /// Reconstruct a SHA-256 ContentDigest from a 64-character hex string.
    ///
    /// This is the inverse of [`to_hex()`](Self::to_hex). It does not compute
    /// a digest — it reconstructs one from a previously-computed hex
    /// representation (e.g., evidence digests received at the API boundary).
    ///
    /// Assumes SHA-256 algorithm (Phase 1). Returns an error if the hex
    /// string is not exactly 64 characters or contains non-hex characters.
    pub fn from_hex(hex: &str) -> Result<Self, crate::error::MsezError> {
        if hex.len() != 64 {
            return Err(crate::error::MsezError::Integrity(format!(
                "expected 64 hex chars for SHA-256 digest, got {}",
                hex.len()
            )));
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).map_err(|_| {
                crate::error::MsezError::Integrity(format!(
                    "invalid hex at position {}: '{}'",
                    i * 2,
                    &hex[i * 2..i * 2 + 2]
                ))
            })?;
        }
        Ok(Self {
            algorithm: DigestAlgorithm::Sha256,
            bytes,
        })
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

/// Incremental SHA-256 accumulator for multi-part digest computation.
///
/// Use this for hash computations that combine multiple data chunks
/// (e.g., domain-prefixed pack digests, directory content hashes, MMR nodes).
/// All SHA-256 computation in the SEZ Stack must flow through `msez-core`,
/// either via [`sha256_digest`] for canonicalized structured data or via
/// this accumulator for multi-part binary data.
///
/// # Example
///
/// ```
/// use msez_core::digest::Sha256Accumulator;
///
/// let mut acc = Sha256Accumulator::new();
/// acc.update(b"prefix\0");
/// acc.update(b"content");
/// let hex = acc.finalize_hex();
/// assert_eq!(hex.len(), 64);
/// ```
pub struct Sha256Accumulator {
    hasher: Sha256,
}

impl Sha256Accumulator {
    /// Create a new empty accumulator.
    pub fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }

    /// Feed data into the accumulator.
    pub fn update(&mut self, data: &[u8]) {
        Digest::update(&mut self.hasher, data);
    }

    /// Consume the accumulator and return a [`ContentDigest`].
    pub fn finalize(self) -> ContentDigest {
        let result = self.hasher.finalize();
        ContentDigest {
            algorithm: DigestAlgorithm::Sha256,
            bytes: result.into(),
        }
    }

    /// Consume the accumulator and return the raw 32-byte digest.
    ///
    /// Use this when the consumer needs raw bytes for binary concatenation
    /// (e.g., MMR node hashing where left || right must be `[u8; 32]` each).
    /// For most uses, prefer [`finalize`](Self::finalize) or
    /// [`finalize_hex`](Self::finalize_hex).
    pub fn finalize_bytes(self) -> [u8; 32] {
        self.hasher.finalize().into()
    }

    /// Consume the accumulator and return the hex-encoded digest string.
    pub fn finalize_hex(self) -> String {
        self.finalize().to_hex()
    }
}

impl Default for Sha256Accumulator {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute a SHA-256 hex digest of raw bytes.
///
/// This is the **Tier 2** digest function for operations that legitimately
/// hash raw byte streams (file contents, Merkle tree nodes, lockfile data)
/// rather than serializable domain objects.
///
/// ## When to use which function
///
/// | Input type | Function | Output | Example |
/// |-----------|----------|--------|---------|
/// | `impl Serialize` (structs, enums, JSON) | [`sha256_digest`] via [`CanonicalBytes`] | `ContentDigest` | Audit events, compliance tensors, VCs |
/// | Raw `&[u8]` needing hex string | `sha256_raw` | `String` | Pack file digests, lockfile hashes |
/// | Raw `&[u8]` needing binary | [`sha256_bytes`] | `[u8; 32]` | MMR nodes, Merkle tree concatenation |
///
/// ## Security Invariant
///
/// All SHA-256 in the codebase flows through `msez-core`. No other crate
/// should directly `use sha2::{Digest, Sha256}` for single-shot hashing.
/// For streaming multi-part hashes, use [`Sha256Accumulator`].
pub fn sha256_raw(data: &[u8]) -> String {
    let mut acc = Sha256Accumulator::new();
    acc.update(data);
    acc.finalize_hex()
}

/// Compute SHA-256 of raw bytes, returning the 32-byte digest.
///
/// Single-shot convenience for binary hash operations that need raw `[u8; 32]`
/// (MMR leaf hashing, binary tree concatenation). For hex output, use
/// [`sha256_raw`]. For canonical JSON digests, use [`sha256_digest`].
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut acc = Sha256Accumulator::new();
    acc.update(data);
    acc.finalize_bytes()
}

/// Compute a Poseidon2 content digest from canonical bytes.
///
/// **Phase 2 — not yet implemented.** This function is gated behind the
/// `poseidon2` feature flag and currently returns `Err(MsezError::NotImplemented)`.
///
/// Poseidon2 is a ZK-friendly hash function designed for efficient verification
/// inside arithmetic circuits. It will be used for zero-knowledge proof
/// generation in the Phase 2 ZKP layer.
#[cfg(feature = "poseidon2")]
pub fn poseidon2_digest(
    _canonical: &CanonicalBytes,
) -> Result<ContentDigest, crate::error::MsezError> {
    Err(crate::error::MsezError::NotImplemented(
        "Poseidon2 digest available in Phase 2".into(),
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

    #[test]
    fn from_hex_roundtrips_with_to_hex() {
        let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
        let original = sha256_digest(&canonical);
        let hex = original.to_hex();
        let reconstructed = ContentDigest::from_hex(&hex).unwrap();
        assert_eq!(original, reconstructed);
    }

    #[test]
    fn from_hex_rejects_short_string() {
        let result = ContentDigest::from_hex("abcd");
        assert!(result.is_err());
        let err = format!("{}", result.unwrap_err());
        assert!(err.contains("64 hex chars"));
    }

    #[test]
    fn from_hex_rejects_long_string() {
        let long = "a".repeat(128);
        assert!(ContentDigest::from_hex(&long).is_err());
    }

    #[test]
    fn from_hex_rejects_non_hex_chars() {
        let bad = "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz";
        assert_eq!(bad.len(), 64);
        assert!(ContentDigest::from_hex(bad).is_err());
    }

    #[test]
    fn from_hex_produces_sha256_algorithm() {
        let hex = "a".repeat(64);
        let digest = ContentDigest::from_hex(&hex).unwrap();
        assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
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
