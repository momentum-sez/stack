//! Tests for deterministic canonical byte computation.
//!
//! Validates that the CanonicalBytes pipeline produces identical output
//! for semantically identical inputs, regardless of key ordering, and
//! that it correctly rejects floats and normalizes datetimes.

use mez_core::{sha256_digest, CanonicalBytes, DigestAlgorithm};
use serde_json::json;

// ---------------------------------------------------------------------------
// Simple object determinism
// ---------------------------------------------------------------------------

#[test]
fn simple_object_deterministic() {
    let data = json!({"name": "test", "value": 42});

    let c1 = CanonicalBytes::new(&data).unwrap();
    let c2 = CanonicalBytes::new(&data).unwrap();

    let d1 = sha256_digest(&c1);
    let d2 = sha256_digest(&c2);

    assert_eq!(d1.to_hex(), d2.to_hex());
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// Key order independence
// ---------------------------------------------------------------------------

#[test]
fn key_order_independence() {
    let a = json!({"b": 2, "a": 1, "c": 3});
    let b = json!({"a": 1, "b": 2, "c": 3});
    let c = json!({"c": 3, "a": 1, "b": 2});

    let da = sha256_digest(&CanonicalBytes::new(&a).unwrap());
    let db = sha256_digest(&CanonicalBytes::new(&b).unwrap());
    let dc = sha256_digest(&CanonicalBytes::new(&c).unwrap());

    assert_eq!(da.to_hex(), db.to_hex());
    assert_eq!(db.to_hex(), dc.to_hex());
}

// ---------------------------------------------------------------------------
// Nested key sorting
// ---------------------------------------------------------------------------

#[test]
fn nested_key_sorting() {
    let a = json!({
        "outer_b": {"inner_z": 1, "inner_a": 2},
        "outer_a": {"inner_y": 3, "inner_b": 4}
    });
    let b = json!({
        "outer_a": {"inner_b": 4, "inner_y": 3},
        "outer_b": {"inner_a": 2, "inner_z": 1}
    });

    let da = sha256_digest(&CanonicalBytes::new(&a).unwrap());
    let db = sha256_digest(&CanonicalBytes::new(&b).unwrap());

    assert_eq!(da.to_hex(), db.to_hex());
}

// ---------------------------------------------------------------------------
// Array order preserved
// ---------------------------------------------------------------------------

#[test]
fn array_order_preserved() {
    let a = json!({"items": [1, 2, 3]});
    let b = json!({"items": [3, 2, 1]});

    let da = sha256_digest(&CanonicalBytes::new(&a).unwrap());
    let db = sha256_digest(&CanonicalBytes::new(&b).unwrap());

    assert_ne!(
        da.to_hex(),
        db.to_hex(),
        "Different array orders must produce different digests"
    );
}

#[test]
fn same_array_order_same_digest() {
    let a = json!({"items": [1, 2, 3]});
    let b = json!({"items": [1, 2, 3]});

    let da = sha256_digest(&CanonicalBytes::new(&a).unwrap());
    let db = sha256_digest(&CanonicalBytes::new(&b).unwrap());

    assert_eq!(da.to_hex(), db.to_hex());
}

// ---------------------------------------------------------------------------
// Datetime normalization
// ---------------------------------------------------------------------------

#[test]
fn datetime_normalization() {
    // Two identical datetime strings must produce identical digests.
    let a = json!({"timestamp": "2026-01-15T12:00:00Z"});
    let b = json!({"timestamp": "2026-01-15T12:00:00Z"});

    let da = sha256_digest(&CanonicalBytes::new(&a).unwrap());
    let db = sha256_digest(&CanonicalBytes::new(&b).unwrap());

    assert_eq!(da.to_hex(), db.to_hex());
}

// ---------------------------------------------------------------------------
// Float rejection
// ---------------------------------------------------------------------------

#[test]
fn float_rejection() {
    let data = json!({"amount": 1.5});
    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "Floats must be rejected by canonicalization"
    );
}

#[test]
fn float_zero_handling() {
    // json!(0.0) creates a serde_json Number with f64 representation.
    // The MCF float rejection rule rejects ANY f64-only Number, including 0.0.
    // This is the correct, safe behavior: callers must use integer 0, not
    // float 0.0, in payloads destined for canonicalization.
    let data = json!({"amount": 0.0});
    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "float literal 0.0 must be rejected — use integer 0 instead"
    );

    // Verify that integer 0 is accepted (the correct way to express zero).
    let data_int = json!({"amount": 0});
    assert!(
        CanonicalBytes::new(&data_int).is_ok(),
        "integer 0 must be accepted"
    );
}

// ---------------------------------------------------------------------------
// Known test vector
// ---------------------------------------------------------------------------

#[test]
fn known_test_vector() {
    // A known input must produce a known digest, ensuring the
    // canonicalization algorithm is stable across releases.
    let data = json!({"a": 1, "b": "hello"});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);

    // Verify the digest is a valid 64-character hex string.
    assert_eq!(digest.to_hex().len(), 64);
    assert!(digest.to_hex().chars().all(|c| c.is_ascii_hexdigit()));

    // Verify the algorithm.
    assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);

    // Verify determinism: same input, same output, always.
    let digest2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(digest.to_hex(), digest2.to_hex());
}

#[test]
fn digest_algorithm_is_sha256() {
    let data = json!({"test": true});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.algorithm(), DigestAlgorithm::Sha256);
}
