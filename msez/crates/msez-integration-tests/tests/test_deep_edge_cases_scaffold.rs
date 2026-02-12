//! Rust counterpart of tests/scenarios/test_deep_edge_cases_scaffold.py
//! Deep edge case tests for canonical bytes and digest computation.

use msez_core::{CanonicalBytes, sha256_digest, CanonicalizationError};
use serde_json::json;

#[test]
fn edge_case_empty_string_key() {
    let data = json!({"": "value"});
    let cb = CanonicalBytes::new(&data).unwrap();
    assert!(!cb.as_bytes().is_empty());
}

#[test]
fn edge_case_very_long_string() {
    let long_str = "x".repeat(10000);
    let data = json!({"key": long_str});
    let cb = CanonicalBytes::new(&data).unwrap();
    let d1 = sha256_digest(&cb);
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2);
}

#[test]
fn edge_case_max_integer() {
    let data = json!({"n": i64::MAX});
    let cb = CanonicalBytes::new(&data).unwrap();
    assert!(!cb.as_bytes().is_empty());
}

#[test]
fn edge_case_min_integer() {
    let data = json!({"n": i64::MIN});
    let cb = CanonicalBytes::new(&data).unwrap();
    assert!(!cb.as_bytes().is_empty());
}

#[test]
fn edge_case_nested_empty_objects() {
    let data = json!({"a": {"b": {"c": {}}}});
    let cb = CanonicalBytes::new(&data).unwrap();
    let d1 = sha256_digest(&cb);
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2);
}

#[test]
fn edge_case_array_of_nulls() {
    let data = json!([null, null, null]);
    let cb = CanonicalBytes::new(&data).unwrap();
    assert!(!cb.as_bytes().is_empty());
}

#[test]
fn edge_case_mixed_nesting() {
    let data = json!({"arr": [{"a": 1}, [2, 3], null, true, "str"]});
    let cb = CanonicalBytes::new(&data).unwrap();
    let d1 = sha256_digest(&cb);
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2);
}

#[test]
fn edge_case_float_in_deep_nested() {
    let data = json!({"a": {"b": {"c": 1.5}}});
    assert!(CanonicalBytes::new(&data).is_err());
}
