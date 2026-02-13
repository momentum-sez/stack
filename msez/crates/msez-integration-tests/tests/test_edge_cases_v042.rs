//! Edge case regression tests for v0.4.2.
//!
//! Canonicalization edge cases that were identified during the v0.4.2 release
//! cycle, including empty strings, special characters in keys, zero/negative
//! integers, and Unicode key ordering.

use msez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// Empty string
// ---------------------------------------------------------------------------

#[test]
fn empty_string_canonical() {
    let data = json!("");
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Empty string must differ from null.
    let null_data = json!(null);
    let null_canonical = CanonicalBytes::new(&null_data).unwrap();
    assert_ne!(
        digest.to_hex(),
        sha256_digest(&null_canonical).to_hex(),
        "Empty string and null must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// Special characters in keys
// ---------------------------------------------------------------------------

#[test]
fn special_characters_in_keys() {
    let data = json!({
        "key-with-dash": 1,
        "key.with.dot": 2,
        "key_with_underscore": 3,
        "key/with/slash": 4
    });
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Deterministic.
    let canonical2 = CanonicalBytes::new(&data).unwrap();
    assert_eq!(digest.to_hex(), sha256_digest(&canonical2).to_hex());
}

#[test]
fn escaped_characters_in_values() {
    let data = json!({
        "quote": "he said \"hello\"",
        "backslash": "path\\to\\file",
        "newline": "line1\nline2",
        "tab": "col1\tcol2"
    });
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// Zero integer
// ---------------------------------------------------------------------------

#[test]
fn zero_integer_canonical() {
    let data = json!({"value": 0});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Zero must differ from empty string.
    let empty_str = json!({"value": ""});
    let empty_canonical = CanonicalBytes::new(&empty_str).unwrap();
    assert_ne!(
        digest.to_hex(),
        sha256_digest(&empty_canonical).to_hex(),
        "Integer 0 and empty string must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// Negative integer
// ---------------------------------------------------------------------------

#[test]
fn negative_integer_canonical() {
    let data = json!({"value": -1});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    let positive = json!({"value": 1});
    let pos_canonical = CanonicalBytes::new(&positive).unwrap();
    assert_ne!(
        digest.to_hex(),
        sha256_digest(&pos_canonical).to_hex(),
        "-1 and 1 must produce different digests"
    );
}

#[test]
fn large_negative_integer() {
    let data = json!({"value": -9_007_199_254_740_992_i64});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// Unicode key ordering
// ---------------------------------------------------------------------------

#[test]
fn unicode_key_ordering() {
    // Keys with Unicode characters must be sorted consistently.
    let a = json!({"\u{00E9}": 1, "a": 2, "z": 3});
    let b = json!({"z": 3, "a": 2, "\u{00E9}": 1});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Unicode key ordering must be deterministic"
    );
}

#[test]
fn mixed_ascii_unicode_keys() {
    let a = json!({"abc": 1, "\u{4E2D}": 2, "def": 3, "\u{0410}": 4});
    let b = json!({"\u{0410}": 4, "def": 3, "\u{4E2D}": 2, "abc": 1});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(sha256_digest(&ca).to_hex(), sha256_digest(&cb).to_hex(),);
}

// ---------------------------------------------------------------------------
// Array order preservation
// ---------------------------------------------------------------------------

#[test]
fn array_order_preserved_v042() {
    // Arrays must preserve element order (unlike object keys).
    let a = json!({"items": [1, 2, 3]});
    let b = json!({"items": [3, 2, 1]});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_ne!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Array element order must be preserved in canonicalization"
    );
}
