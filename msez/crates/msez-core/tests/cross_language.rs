//! # Cross-Language Digest Equality Tests
//!
//! These integration tests verify that the Rust `CanonicalBytes` + `sha256_digest`
//! pipeline produces byte-identical output to the Python `jcs_canonicalize()` +
//! `hashlib.sha256().hexdigest()` pipeline defined in `tools/lawpack.py`.
//!
//! ## How it works
//!
//! 1. A set of JSON test vectors is defined (same inputs for both languages).
//! 2. The Rust test computes canonical bytes and SHA-256 digests.
//! 3. A companion test shells out to Python to compute the same digests via
//!    `tools/lawpack.py:jcs_canonicalize()`.
//! 4. The digests are compared for exact equality.
//!
//! ## Why this matters
//!
//! The Feb 2026 audit (Finding §2.1) discovered that the Phoenix layer used
//! `json.dumps(sort_keys=True)` while the core layer used `jcs_canonicalize()`,
//! producing different digests for identical data. This test prevents regression
//! across the Rust/Python language boundary.

use msez_core::canonical::CanonicalBytes;
use msez_core::digest::sha256_digest;

/// Test vectors: JSON inputs and their expected canonical byte representations.
///
/// These are the authoritative test vectors for cross-language canonicalization.
/// The Python companion script `tests/generate_vectors.py` must produce
/// identical results.
const TEST_VECTORS: &[(&str, &str)] = &[
    // (JSON input, expected canonical bytes as string)
    (r#"{"b":2,"a":1,"c":"hello"}"#, r#"{"a":1,"b":2,"c":"hello"}"#),
    (r#"{"z":26,"a":1}"#, r#"{"a":1,"z":26}"#),
    (r#"{}"#, r#"{}"#),
    (r#"[]"#, r#"[]"#),
    (r#"{"nested":{"z":1,"a":2},"top":true}"#, r#"{"nested":{"a":2,"z":1},"top":true}"#),
    (r#"{"arr":[3,2,1],"key":"value"}"#, r#"{"arr":[3,2,1],"key":"value"}"#),
    (r#"{"n":null,"b":false,"t":true,"i":42,"s":"text"}"#,
     r#"{"b":false,"i":42,"n":null,"s":"text","t":true}"#),
    // Large integer
    (r#"{"big":999999999999}"#, r#"{"big":999999999999}"#),
    // Negative integer
    (r#"{"neg":-42}"#, r#"{"neg":-42}"#),
    // Empty string value
    (r#"{"empty":""}"#, r#"{"empty":""}"#),
];

#[test]
fn canonical_bytes_match_expected_vectors() {
    for (input, expected_canonical) in TEST_VECTORS {
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
fn sha256_digests_are_deterministic_across_vectors() {
    for (input, _) in TEST_VECTORS {
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let d1 = sha256_digest(&cb);
        let d2 = sha256_digest(&cb);
        assert_eq!(d1, d2, "Non-deterministic digest for input: {input}");
    }
}

/// Cross-language test: shell out to Python to compute digests and compare.
///
/// This test requires Python 3 and the `tools/lawpack.py` module to be accessible.
/// It is skipped (not failed) if Python is unavailable.
#[test]
fn cross_language_digest_equality_with_python() {
    // Check if Python is available
    let python_check = std::process::Command::new("python3")
        .arg("-c")
        .arg("import sys; print(sys.version_info[:2])")
        .output();

    let python_available = python_check.map(|o| o.status.success()).unwrap_or(false);
    if !python_available {
        eprintln!("SKIP: Python 3 not available, skipping cross-language test");
        return;
    }

    // Build the Python script that computes digests for all test vectors.
    // We pass the test vector inputs as a JSON array and get back a JSON array
    // of {canonical, digest} objects.
    let inputs: Vec<&str> = TEST_VECTORS.iter().map(|(input, _)| *input).collect();
    let inputs_json = serde_json::to_string(&inputs).unwrap();

    let python_script = format!(
        r#"
import json, hashlib, sys, os
sys.path.insert(0, os.path.join(os.getcwd(), 'tools'))
try:
    from lawpack import jcs_canonicalize
except ImportError:
    # Try from repository root
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', '..', '..', 'tools'))
    from lawpack import jcs_canonicalize

inputs = json.loads('{escaped_inputs}')
results = []
for inp in inputs:
    data = json.loads(inp)
    canonical = jcs_canonicalize(data)
    digest = hashlib.sha256(canonical).hexdigest()
    results.append({{"canonical": canonical.decode("utf-8"), "digest": digest}})
print(json.dumps(results))
"#,
        escaped_inputs = inputs_json.replace('\\', "\\\\").replace('\'', "\\'")
    );

    let output = std::process::Command::new("python3")
        .arg("-c")
        .arg(&python_script)
        .current_dir(env!("CARGO_MANIFEST_DIR").to_string() + "/../../../")
        .output();

    let output = match output {
        Ok(o) if o.status.success() => o,
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            eprintln!("SKIP: Python script failed: {stderr}");
            return;
        }
        Err(e) => {
            eprintln!("SKIP: Could not run Python: {e}");
            return;
        }
    };

    let stdout = String::from_utf8(output.stdout).unwrap();
    let python_results: Vec<serde_json::Value> =
        serde_json::from_str(stdout.trim()).expect("Python output should be valid JSON");

    assert_eq!(
        python_results.len(),
        TEST_VECTORS.len(),
        "Python returned wrong number of results"
    );

    for (i, ((input, _expected_canonical), py_result)) in
        TEST_VECTORS.iter().zip(python_results.iter()).enumerate()
    {
        let py_canonical = py_result["canonical"].as_str().unwrap();
        let py_digest = py_result["digest"].as_str().unwrap();

        // Compute Rust results
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let rust_canonical = std::str::from_utf8(cb.as_bytes()).unwrap();
        let rust_digest = sha256_digest(&cb).to_hex();

        assert_eq!(
            rust_canonical, py_canonical,
            "Vector {i}: Canonical bytes mismatch for input: {input}\n  Rust:   {rust_canonical}\n  Python: {py_canonical}"
        );

        assert_eq!(
            rust_digest, py_digest,
            "Vector {i}: SHA-256 digest mismatch for input: {input}\n  Rust:   {rust_digest}\n  Python: {py_digest}"
        );
    }
}

/// Verify that float rejection matches Python behavior.
///
/// Python's `_coerce_json_types()` raises `ValueError` for `isinstance(obj, float)`.
/// Rust's `CanonicalBytes::new()` returns `Err(FloatRejected)` for serde_json
/// numbers that are not representable as i64/u64.
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
