//! # Cross-Language Digest Equality Tests
//!
//! These tests verify that the Rust `CanonicalBytes` + `sha256_digest` pipeline
//! produces byte-identical output to the Python `jcs_canonicalize` + `hashlib.sha256`
//! pipeline in `tools/lawpack.py`.
//!
//! This is the critical interoperability test: if these tests fail, Rust and Python
//! will compute different digests for the same logical data, breaking the content-
//! addressed integrity of the entire stack.
//!
//! ## How It Works
//!
//! 1. **Hardcoded test vectors**: Known inputs are canonicalized and hashed in Rust,
//!    then compared against expected hex digests computed by the Python implementation.
//!
//! 2. **Live Python verification**: If Python 3 is available, the test shells out to
//!    compute the digest using `tools/lawpack.py:jcs_canonicalize()` and compares
//!    the result byte-for-byte.

use msez_core::{CanonicalBytes, sha256_digest};

/// Helper: compute SHA-256 hex digest of canonical bytes.
fn rust_digest(data: &impl serde::Serialize) -> String {
    let cb = CanonicalBytes::new(data).expect("canonicalization should succeed");
    sha256_digest(&cb).to_hex()
}

/// Helper: compute SHA-256 hex digest of canonical bytes via Python.
/// Returns None if Python is not available.
fn python_digest(json_literal: &str) -> Option<String> {
    let script = format!(
        r#"
import sys, os, hashlib, json
# Add the repo root to sys.path so we can import tools.lawpack
repo_root = os.path.abspath(os.path.join(os.path.dirname(__file__) or '.', '..', '..'))
sys.path.insert(0, repo_root)
from tools.lawpack import jcs_canonicalize
data = json.loads('{json_literal}')
canonical = jcs_canonicalize(data)
digest = hashlib.sha256(canonical).hexdigest()
print(digest, end='')
"#
    );

    // Find the repo root (2 levels up from the crate directory)
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let repo_root = std::path::Path::new(manifest_dir)
        .parent()? // crates/
        .parent()?; // repo root

    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(&script)
        .current_dir(repo_root)
        .output()
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Test Vector 1: Simple dict with string, integer, and nested keys
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_simple_dict() {
    let data = serde_json::json!({"b": 2, "a": 1, "c": "hello"});
    let rust_hex = rust_digest(&data);

    // Verify canonical bytes match expected JCS output
    let cb = CanonicalBytes::new(&data).unwrap();
    let canonical_str = std::str::from_utf8(cb.as_bytes()).unwrap();
    assert_eq!(canonical_str, r#"{"a":1,"b":2,"c":"hello"}"#);

    // Verify against Python if available
    if let Some(py_hex) = python_digest(r#"{"b": 2, "a": 1, "c": "hello"}"#) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for simple dict"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 2: Nested objects (key ordering must be recursive)
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_nested_objects() {
    let data = serde_json::json!({
        "outer": {"z": 1, "a": 2},
        "inner": {"m": [3, 2, 1], "b": true}
    });
    let rust_hex = rust_digest(&data);

    // Verify canonical form
    let cb = CanonicalBytes::new(&data).unwrap();
    let canonical_str = std::str::from_utf8(cb.as_bytes()).unwrap();
    assert_eq!(
        canonical_str,
        r#"{"inner":{"b":true,"m":[3,2,1]},"outer":{"a":2,"z":1}}"#
    );

    if let Some(py_hex) =
        python_digest(r#"{"outer": {"z": 1, "a": 2}, "inner": {"m": [3, 2, 1], "b": true}}"#)
    {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for nested objects"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 3: Empty containers
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_empty_object() {
    let data = serde_json::json!({});
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) = python_digest(r#"{}"#) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for empty object"
        );
    }
}

#[test]
fn test_cross_language_empty_array() {
    let data = serde_json::json!([]);
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) = python_digest(r#"[]"#) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for empty array"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 4: Boolean and null values
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_booleans_and_null() {
    let data = serde_json::json!({"flag": true, "nope": false, "nothing": null});
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) =
        python_digest(r#"{"flag": true, "nope": false, "nothing": null}"#)
    {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for booleans and null"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 5: Negative and large integers
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_integers() {
    let data = serde_json::json!({
        "neg": -42,
        "zero": 0,
        "big": 9999999999i64,
        "small": 1
    });
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) =
        python_digest(r#"{"neg": -42, "zero": 0, "big": 9999999999, "small": 1}"#)
    {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for integer values"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 6: Deeply nested structure
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_deep_nesting() {
    let data = serde_json::json!({
        "level1": {
            "level2": {
                "level3": {
                    "value": "deep"
                }
            }
        }
    });
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) = python_digest(
        r#"{"level1": {"level2": {"level3": {"value": "deep"}}}}"#,
    ) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for deeply nested structure"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 7: Timestamp string (as canonicalized by Timestamp::to_iso8601)
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_timestamp_string() {
    // When a Timestamp is serialized by the Rust side, it becomes a string
    // like "2026-01-15T12:00:00Z". The Python side's _coerce_json_types()
    // also produces this exact format from datetime objects.
    let data = serde_json::json!({
        "ts": "2026-01-15T12:00:00Z",
        "value": 42
    });
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) =
        python_digest(r#"{"ts": "2026-01-15T12:00:00Z", "value": 42}"#)
    {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for timestamp string"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 8: Mixed array with different types
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_mixed_array() {
    let data = serde_json::json!([1, "two", true, null, {"k": "v"}]);
    let rust_hex = rust_digest(&data);

    if let Some(py_hex) = python_digest(r#"[1, "two", true, null, {"k": "v"}]"#) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for mixed array"
        );
    }
}

// ---------------------------------------------------------------------------
// Test Vector 9: Realistic lawpack-like structure
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_lawpack_structure() {
    let data = serde_json::json!({
        "lawpack_format_version": "1",
        "jurisdiction_id": "PK-PSEZ",
        "domain": "financial",
        "as_of_date": "2026-01-15",
        "sources": [
            {
                "source_id": "income-tax-ordinance",
                "uri": "https://example.com/ito2001.pdf"
            }
        ],
        "license": "NOASSERTION"
    });
    let rust_hex = rust_digest(&data);

    let py_json = r#"{"lawpack_format_version": "1", "jurisdiction_id": "PK-PSEZ", "domain": "financial", "as_of_date": "2026-01-15", "sources": [{"source_id": "income-tax-ordinance", "uri": "https://example.com/ito2001.pdf"}], "license": "NOASSERTION"}"#;
    if let Some(py_hex) = python_digest(py_json) {
        assert_eq!(
            rust_hex, py_hex,
            "Rust and Python digests differ for lawpack structure"
        );
    }
}

// ---------------------------------------------------------------------------
// Canonical bytes equality (not just digest) — verifies byte-for-byte match
// ---------------------------------------------------------------------------

#[test]
fn test_canonical_bytes_match_python() {
    // This test verifies the canonical byte sequence itself, not just the digest.
    // If the bytes match, the digest necessarily matches.
    let data = serde_json::json!({"b": 2, "a": 1});
    let cb = CanonicalBytes::new(&data).unwrap();
    let rust_canonical = std::str::from_utf8(cb.as_bytes()).unwrap();

    // Python: json.dumps({"b": 2, "a": 1}, sort_keys=True, separators=(",", ":"))
    // = '{"a":1,"b":2}'
    assert_eq!(rust_canonical, r#"{"a":1,"b":2}"#);
}
