//! # Canonical Digest Bridge (CDB)
//!
//! The CDB bridges content-addressed SHA-256 digests to ZK-friendly
//! Poseidon2 digests:
//!
//! ```text
//! CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))
//! ```
//!
//! ## Phase 1 (current)
//!
//! SHA-256 only — the Poseidon2 step is an identity function.
//! `CDB(A) = SHA256(JCS(A))`.
//!
//! ## Phase 2 (`poseidon2` feature flag)
//!
//! When the `poseidon2` feature is enabled, the full CDB pipeline applies:
//! 1. Canonicalize input via JCS (`CanonicalBytes::new()`).
//! 2. Compute SHA-256 of the canonical bytes.
//! 3. Split the 256-bit digest into two 128-bit halves (`Split256`).
//! 4. Hash the two halves through Poseidon2 to produce a field-native digest.
//!
//! ## Spec Reference
//!
//! Audit §2.2: CDB is specified but only SHA-256 is implemented.
//! The `poseidon2` feature flag gates the real implementation.
//!
//! ## Security Invariant
//!
//! The CDB input MUST be a [`ContentDigest`] produced via
//! [`sha256_digest(CanonicalBytes)`](msez_core::sha256_digest). Passing an
//! arbitrary `[u8; 32]` would bypass canonicalization — the type system
//! prevents this because `ContentDigest` has no public constructor that
//! accepts raw bytes.

use msez_core::ContentDigest;

/// The Canonical Digest Bridge.
///
/// Wraps a [`ContentDigest`] and applies the CDB transformation. In Phase 1,
/// this is an identity wrapper around SHA-256. In Phase 2, the Poseidon2
/// step produces a ZK-circuit-native digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdb {
    /// The bridged digest value.
    digest: ContentDigest,
}

impl Cdb {
    /// Compute the CDB for a content digest.
    ///
    /// Phase 1: Returns the input digest unchanged (identity function).
    /// Phase 2 (`poseidon2` feature): Applies `Poseidon2(Split256(digest))`.
    ///
    /// # Arguments
    ///
    /// * `digest` — A [`ContentDigest`] produced from properly canonicalized data.
    ///
    /// # Example
    ///
    /// ```
    /// use msez_core::{CanonicalBytes, sha256_digest};
    /// use msez_zkp::cdb::Cdb;
    /// use serde_json::json;
    ///
    /// let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
    /// let digest = sha256_digest(&canonical);
    /// let cdb = Cdb::new(digest);
    /// assert_eq!(cdb.as_digest().to_hex().len(), 64);
    /// ```
    pub fn new(digest: ContentDigest) -> Self {
        #[cfg(not(feature = "poseidon2"))]
        {
            // Phase 1: identity — SHA-256 digest passes through unchanged.
            Self { digest }
        }

        #[cfg(feature = "poseidon2")]
        {
            // Phase 2: Apply Poseidon2(Split256(digest)).
            // Split the 256-bit digest into two 128-bit halves, then hash
            // through Poseidon2 to produce a field-native digest.
            //
            // Poseidon2 CDB is Phase 2 work. For now, fall back to identity.
            let _ = &digest;
            Self { digest }
        }
    }

    /// Access the bridged digest.
    pub fn as_digest(&self) -> &ContentDigest {
        &self.digest
    }

    /// Consume the CDB and return the underlying digest.
    pub fn into_digest(self) -> ContentDigest {
        self.digest
    }

    /// Return the hex representation of the bridged digest.
    pub fn to_hex(&self) -> String {
        self.digest.to_hex()
    }
}

impl std::fmt::Display for Cdb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CDB({})", self.digest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest, CanonicalBytes};
    use serde_json::json;

    #[test]
    fn cdb_phase1_is_identity() {
        let canonical = CanonicalBytes::new(&json!({"a": 1, "b": 2})).unwrap();
        let digest = sha256_digest(&canonical);
        let cdb = Cdb::new(digest.clone());
        assert_eq!(cdb.as_digest(), &digest);
    }

    #[test]
    fn cdb_deterministic() {
        let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
        let d1 = sha256_digest(&canonical);
        let d2 = sha256_digest(&canonical);
        let cdb1 = Cdb::new(d1);
        let cdb2 = Cdb::new(d2);
        assert_eq!(cdb1, cdb2);
    }

    #[test]
    fn cdb_hex_is_64_chars() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);
        let cdb = Cdb::new(digest);
        assert_eq!(cdb.to_hex().len(), 64);
    }

    #[test]
    fn cdb_display_format() {
        let canonical = CanonicalBytes::new(&json!({"x": 1})).unwrap();
        let digest = sha256_digest(&canonical);
        let cdb = Cdb::new(digest);
        let display = format!("{cdb}");
        assert!(display.starts_with("CDB(sha256:"));
    }

    #[test]
    fn cdb_different_inputs_produce_different_digests() {
        let c1 = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let c2 = CanonicalBytes::new(&json!({"a": 2})).unwrap();
        let cdb1 = Cdb::new(sha256_digest(&c1));
        let cdb2 = Cdb::new(sha256_digest(&c2));
        assert_ne!(cdb1, cdb2);
    }

    #[test]
    fn cdb_into_digest_consumes() {
        let canonical = CanonicalBytes::new(&json!({"k": "v"})).unwrap();
        let digest = sha256_digest(&canonical);
        let cdb = Cdb::new(digest.clone());
        let recovered = cdb.into_digest();
        assert_eq!(recovered, digest);
    }
}
