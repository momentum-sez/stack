//! # Canonicalization Digest Tests
//!
//! These tests verify that the Rust `CanonicalBytes` + `sha256_digest`
//! pipeline produces byte-identical output to the expected canonical
//! representations and SHA-256 digests.
//!
//! ## History
//!
//! These vectors were originally verified against the Python
//! `jcs_canonicalize()` implementation for cross-language parity.
//! The Python code has been removed (Feb 2026) and these hardcoded
//! vectors serve as the authoritative reference.
//!
//! ## Why this matters
//!
//! The Feb 2026 audit (Finding §2.1) discovered that different layers used
//! different canonicalization approaches (`json.dumps(sort_keys=True)` vs
//! `jcs_canonicalize()`), producing different digests for identical data.
//! These tests prevent regression.

use msez_core::canonical::CanonicalBytes;
use msez_core::digest::sha256_digest;

/// Test vectors: JSON inputs, expected canonical bytes, and expected SHA-256 digests.
///
/// These are the authoritative test vectors for canonicalization.
/// The SHA-256 digests were verified against the Python implementation
/// before the Python code was removed.
const TEST_VECTORS: &[(&str, &str, &str)] = &[
    // (JSON input, expected canonical bytes, expected SHA-256 hex digest)
    (
        r#"{"b":2,"a":1,"c":"hello"}"#,
        r#"{"a":1,"b":2,"c":"hello"}"#,
        "264be526dd59f5bed5c756e96e5a6a08f285ca424658f70b981f2554b4709121",
    ),
    (
        r#"{"z":26,"a":1}"#,
        r#"{"a":1,"z":26}"#,
        "b052ee0e2868b2a815003267140610c82c8d190a11506a9a8d25f626e910300b",
    ),
    (
        r#"{}"#,
        r#"{}"#,
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a",
    ),
    (
        r#"[]"#,
        r#"[]"#,
        "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945",
    ),
    (
        r#"{"nested":{"z":1,"a":2},"top":true}"#,
        r#"{"nested":{"a":2,"z":1},"top":true}"#,
        "58ccce9a512c98592bc25d09cf386bac7079a698ef0c3646f1834e25fc9e6c70",
    ),
    (
        r#"{"arr":[3,2,1],"key":"value"}"#,
        r#"{"arr":[3,2,1],"key":"value"}"#,
        "016975dad96a6a910491578b3db1e665dbc9aa8ee96f77ade19d12f53815e315",
    ),
    (
        r#"{"n":null,"b":false,"t":true,"i":42,"s":"text"}"#,
        r#"{"b":false,"i":42,"n":null,"s":"text","t":true}"#,
        "1f73be934d44ec0b088191054e1e88315d067c72465819140c4fd20e4ed7f2cf",
    ),
    // Large integer
    (
        r#"{"big":999999999999}"#,
        r#"{"big":999999999999}"#,
        "b1fa13966a70f0a715859fb0d6db77521bc9ce7b2eedce388daed2bdeed74350",
    ),
    // Negative integer
    (
        r#"{"neg":-42}"#,
        r#"{"neg":-42}"#,
        "b74f5f1531da1b040306db6ceea0a02075f1a31e54bab8c14f7bf9b02a8603f0",
    ),
    // Empty string value
    (
        r#"{"empty":""}"#,
        r#"{"empty":""}"#,
        "5ddea0bed9ab50512425b4c9fa9698e0bacfb81414a3e65f3af66c9e85a9c8f0",
    ),
];

#[test]
fn canonical_bytes_match_expected_vectors() {
    for (input, expected_canonical, _) in TEST_VECTORS {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let actual = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(
            actual, *expected_canonical,
            "Canonical mismatch for input: {input}"
        );
    }
}

#[test]
fn sha256_digests_match_expected_vectors() {
    for (input, _, expected_digest) in TEST_VECTORS {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let digest = sha256_digest(&cb).to_hex();
        assert_eq!(
            digest, *expected_digest,
            "SHA-256 digest mismatch for input: {input}"
        );
    }
}

#[test]
fn sha256_digests_are_deterministic_across_vectors() {
    for (input, _, _) in TEST_VECTORS {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let d1 = sha256_digest(&cb);
        let d2 = sha256_digest(&cb);
        assert_eq!(d1, d2, "Non-deterministic digest for input: {input}");
    }
}

/// Verify that float rejection is consistent.
///
/// Floats must be rejected to prevent non-deterministic canonicalization.
/// Integers are accepted.
#[test]
fn float_rejection_consistency() {
    // These should all be rejected (true floats)
    let float_inputs = [
        r#"{"x":1.5}"#,
        r#"{"x":3.14}"#,
        r#"{"x":0.1}"#,
        r#"{"nested":{"y":2.718}}"#,
        r#"[1.1]"#,
    ];

    for input in &float_inputs {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = CanonicalBytes::from_value(value);
        assert!(
            result.is_err(),
            "Float input should have been rejected: {input}"
        );
    }

    // These should all be accepted (integers)
    let int_inputs = [
        r#"{"x":1}"#,
        r#"{"x":0}"#,
        r#"{"x":-42}"#,
        r#"{"x":999999999999}"#,
    ];

    for input in &int_inputs {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let result = CanonicalBytes::from_value(value);
        assert!(
            result.is_ok(),
            "Integer input should have been accepted: {input}"
        );
    }
}
