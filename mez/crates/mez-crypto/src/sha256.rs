//! # SHA-256 Digest Computation
//!
//! Computes [`ContentDigest`] values from [`CanonicalBytes`]. This is the
//! only sanctioned path for producing content-addressed digests in Phase 1.
//!
//! ## Security Invariant
//!
//! The function signature requires `CanonicalBytes` — not raw `&[u8]`.
//! This ensures that every digest was computed from properly canonicalized
//! data, preventing the canonicalization split (audit finding §2.1).

use mez_core::{sha256_digest as core_sha256_digest, CanonicalBytes, ContentDigest};

/// Compute a SHA-256 content digest from canonical bytes.
///
/// This is the standard digest computation path for Phase 1.
/// The input must be [`CanonicalBytes`] — raw byte slices are not accepted.
///
/// Delegates to [`mez_core::sha256_digest()`] — the single implementation
/// in the workspace.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    core_sha256_digest(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sha256_digest_produces_64_hex_chars() {
        let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(digest.to_hex().len(), 64);
        assert!(digest.to_hex().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn sha256_digest_is_deterministic() {
        let canonical = CanonicalBytes::new(&json!({"a": 1, "b": 2})).unwrap();
        let d1 = sha256_digest(&canonical);
        let d2 = sha256_digest(&canonical);
        assert_eq!(d1, d2);
    }

    #[test]
    fn sha256_digest_different_input_produces_different_digest() {
        let c1 = CanonicalBytes::new(&json!({"x": 1})).unwrap();
        let c2 = CanonicalBytes::new(&json!({"x": 2})).unwrap();
        assert_ne!(sha256_digest(&c1), sha256_digest(&c2));
    }

    #[test]
    fn sha256_digest_agrees_with_core() {
        let canonical = CanonicalBytes::new(&json!({"test": "agreement"})).unwrap();
        let crypto_digest = sha256_digest(&canonical);
        let core_digest = core_sha256_digest(&canonical);
        assert_eq!(crypto_digest, core_digest);
    }

    #[test]
    fn sha256_digest_empty_object() {
        let canonical = CanonicalBytes::new(&json!({})).unwrap();
        let digest = sha256_digest(&canonical);
        assert_eq!(digest.to_hex().len(), 64);
    }
}
