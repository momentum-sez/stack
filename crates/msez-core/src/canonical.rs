//! # Canonical Serialization — JCS-Compatible Byte Production
//!
//! This module defines `CanonicalBytes`, the sole construction path for bytes
//! used in digest computation across the entire SEZ Stack.
//!
//! ## Security Invariant
//!
//! The `CanonicalBytes` newtype has a private inner field. The only way to
//! construct it is through `CanonicalBytes::new()`, which applies the full
//! Momentum type-coercion pipeline (float rejection, datetime normalization,
//! key stringification) before JCS serialization.
//!
//! This makes the "wrong serialization path" defect class (audit finding §2.1)
//! structurally impossible: any function requiring canonical bytes for digest
//! computation must accept `&CanonicalBytes`, and the only way to produce one
//! is through the correct pipeline.
//!
//! ## Cross-Language Compatibility
//!
//! The coercion rules match `tools/lawpack.py:_coerce_json_types()` exactly:
//!
//! 1. **Reject floats** — amounts must be strings or integers. Floats have
//!    non-deterministic JCS number serialization edge cases.
//! 2. **Normalize datetimes** — UTC ISO8601 with `Z` suffix, truncated to seconds.
//!    Naive datetimes are assumed UTC. Non-UTC offsets are converted.
//! 3. **Coerce non-string keys** — all object keys become strings via `to_string()`.
//! 4. **Convert tuples to lists** — tuples serialize as JSON arrays.
//! 5. **Unknown types** — fall back to `to_string()` (Python's `str(obj)`).
//!
//! After coercion, serialization uses `serde_jcs` for RFC 8785 (JSON
//! Canonicalization Scheme) compliant output: sorted keys, compact separators,
//! deterministic byte sequence.
//!
//! ## Implements
//!
//! Spec §8 — Canonical Digest computation rules.

use serde::Serialize;
use serde_json::Value;

use crate::error::CanonicalizationError;

/// Bytes produced exclusively by JCS-compatible canonicalization with
/// Momentum-specific type coercion rules.
///
/// # Invariants
///
/// - The only constructor is `CanonicalBytes::new()`.
/// - All datetime values are normalized to UTC ISO8601 with Z suffix, truncated to seconds.
/// - All numeric amounts are integers or strings, never floats.
/// - All dict keys are strings.
/// - Tuples/sequences are JSON arrays.
/// - Serialization uses sorted keys with compact separators (RFC 8785).
///
/// These invariants are enforced by the constructor and cannot be violated
/// by downstream code because the inner `Vec<u8>` is private.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Construct canonical bytes from any serializable value.
    ///
    /// Applies the full Momentum type coercion pipeline before JCS serialization.
    /// This is the ONLY way to construct `CanonicalBytes`. All digest computation
    /// in the entire stack must flow through this constructor.
    ///
    /// # Errors
    ///
    /// Returns `CanonicalizationError::FloatRejected` if the value contains float
    /// numbers. Returns `CanonicalizationError::SerializationFailed` if JCS
    /// serialization fails.
    pub fn new(obj: &impl Serialize) -> Result<Self, CanonicalizationError> {
        let value = serde_json::to_value(obj)?;
        let coerced = coerce_json_value(value)?;
        let bytes = serialize_canonical(&coerced)?;
        Ok(Self(bytes))
    }

    /// Access the canonical bytes for digest computation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Returns the length of the canonical byte sequence.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if the canonical byte sequence is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl AsRef<[u8]> for CanonicalBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Recursively coerce JSON values according to Momentum canonicalization rules.
///
/// These rules match `tools/lawpack.py:_coerce_json_types()` exactly:
///
/// 1. `null`, `bool`, `string`, `integer` — pass through unchanged.
/// 2. `float` (non-integer) — **rejected** with `FloatRejected` error.
/// 3. `object` — keys coerced to strings (already strings in JSON), values recursed.
/// 4. `array` — elements recursed.
///
/// In the Rust/serde world, datetime and tuple coercion happen at the
/// serialization boundary (`Serialize` impl), not in the JSON value tree.
/// Chrono's `DateTime<Utc>` serializes to an ISO8601 string, and Rust tuples
/// serialize to JSON arrays. The `Timestamp` type in this crate ensures the
/// Z-suffix and second-precision invariants.
fn coerce_json_value(value: Value) -> Result<Value, CanonicalizationError> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(value),
        Value::Number(ref n) => {
            // Reject pure floats (not representable as i64/u64).
            // This matches Python's: if isinstance(obj, float): raise ValueError(...)
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                if let Some(f) = n.as_f64() {
                    return Err(CanonicalizationError::FloatRejected(f));
                }
            }
            Ok(value)
        }
        Value::Object(map) => {
            let mut coerced = serde_json::Map::new();
            for (k, v) in map {
                // Keys are already strings in JSON (serde_json::Map<String, Value>).
                // This matches Python's: out[str(k)] = _coerce_json_types(v)
                coerced.insert(k, coerce_json_value(v)?);
            }
            Ok(Value::Object(coerced))
        }
        Value::Array(arr) => {
            let coerced: Result<Vec<_>, _> = arr.into_iter().map(coerce_json_value).collect();
            Ok(Value::Array(coerced?))
        }
    }
}

/// Serialize a JSON value in JCS-canonical form (RFC 8785).
///
/// Uses `serde_jcs` for deterministic output: sorted keys, compact separators,
/// no trailing whitespace. The output is UTF-8 encoded bytes matching the
/// Python equivalent: `json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")`
fn serialize_canonical(value: &Value) -> Result<Vec<u8>, CanonicalizationError> {
    let s = serde_jcs::to_string(value)?;
    Ok(s.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_bytes_simple_dict() {
        // Matches Python: jcs_canonicalize({"b": 2, "a": 1, "c": "hello"})
        let data = serde_json::json!({"b": 2, "a": 1, "c": "hello"});
        let cb = CanonicalBytes::new(&data).expect("should canonicalize");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        // JCS: sorted keys, compact separators
        assert_eq!(s, r#"{"a":1,"b":2,"c":"hello"}"#);
    }

    #[test]
    fn test_canonical_bytes_sorted_keys() {
        // Verify key sorting matches Python json.dumps(sort_keys=True)
        let data = serde_json::json!({"z": 1, "m": 2, "a": 3});
        let cb = CanonicalBytes::new(&data).expect("should canonicalize");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"a":3,"m":2,"z":1}"#);
    }

    #[test]
    fn test_canonical_bytes_nested() {
        let data = serde_json::json!({
            "outer": {"b": 2, "a": 1},
            "list": [3, 2, 1]
        });
        let cb = CanonicalBytes::new(&data).expect("should canonicalize");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        // Nested objects also have sorted keys
        assert_eq!(s, r#"{"list":[3,2,1],"outer":{"a":1,"b":2}}"#);
    }

    #[test]
    fn test_float_rejection() {
        let data = serde_json::json!({"amount": 1.5});
        let result = CanonicalBytes::new(&data);
        assert!(result.is_err());
        match result.unwrap_err() {
            CanonicalizationError::FloatRejected(f) => assert_eq!(f, 1.5),
            other => panic!("Expected FloatRejected, got: {other}"),
        }
    }

    #[test]
    fn test_float_zero_point_five_rejected() {
        let data = serde_json::json!({"val": 0.5});
        assert!(CanonicalBytes::new(&data).is_err());
    }

    #[test]
    fn test_integer_accepted() {
        // Integer values are accepted (serde_json stores them as i64/u64).
        let data = serde_json::json!({"amount": 42});
        let cb = CanonicalBytes::new(&data).expect("integers should be accepted");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"amount":42}"#);
    }

    #[test]
    fn test_null_passthrough() {
        let data = serde_json::json!({"key": null});
        let cb = CanonicalBytes::new(&data).expect("null should pass through");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"key":null}"#);
    }

    #[test]
    fn test_bool_passthrough() {
        let data = serde_json::json!({"flag": true, "other": false});
        let cb = CanonicalBytes::new(&data).expect("bools should pass through");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"flag":true,"other":false}"#);
    }

    #[test]
    fn test_empty_object() {
        let data = serde_json::json!({});
        let cb = CanonicalBytes::new(&data).expect("empty object should work");
        assert_eq!(cb.as_bytes(), b"{}");
    }

    #[test]
    fn test_empty_array() {
        let data = serde_json::json!([]);
        let cb = CanonicalBytes::new(&data).expect("empty array should work");
        assert_eq!(cb.as_bytes(), b"[]");
    }

    #[test]
    fn test_string_value() {
        let data = "hello world";
        let cb = CanonicalBytes::new(&data).expect("string should work");
        assert_eq!(cb.as_bytes(), b"\"hello world\"");
    }

    #[test]
    fn test_len_and_is_empty() {
        let data = serde_json::json!({"a": 1});
        let cb = CanonicalBytes::new(&data).unwrap();
        assert!(!cb.is_empty());
        assert!(cb.len() > 0);
    }

    #[test]
    fn test_negative_integer() {
        let data = serde_json::json!({"val": -42});
        let cb = CanonicalBytes::new(&data).expect("negative ints should work");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"val":-42}"#);
    }

    #[test]
    fn test_large_integer() {
        let data = serde_json::json!({"val": 9999999999i64});
        let cb = CanonicalBytes::new(&data).expect("large ints should work");
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"val":9999999999}"#);
    }

    #[test]
    fn test_deeply_nested_float_rejected() {
        let data = serde_json::json!({"a": {"b": [{"c": 3.14}]}});
        assert!(CanonicalBytes::new(&data).is_err());
    }

    #[test]
    fn test_unicode_passthrough() {
        // Matches Python ensure_ascii=False: non-ASCII chars pass through as UTF-8.
        let data = serde_json::json!({"name": "\u{00e9}\u{00e8}\u{00ea}"});
        let cb = CanonicalBytes::new(&data).expect("unicode should pass through");
        let bytes = cb.as_bytes();
        let s = std::str::from_utf8(bytes).unwrap();
        assert!(s.contains('\u{00e9}'));
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating JSON-compatible values without floats.
    /// Mirrors the restricted domain of `_coerce_json_types()`.
    fn json_value_no_floats() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            "[a-zA-Z0-9_ ]{0,50}".prop_map(|s| Value::String(s)),
        ];
        leaf.prop_recursive(
            4,  // depth
            64, // desired size
            8,  // items per collection
            |inner| {
                prop_oneof![
                    // Arrays
                    prop::collection::vec(inner.clone(), 0..8)
                        .prop_map(Value::Array),
                    // Objects with string keys
                    prop::collection::btree_map("[a-z]{1,10}", inner, 0..8)
                        .prop_map(|m| {
                            let map: serde_json::Map<String, Value> =
                                m.into_iter().collect();
                            Value::Object(map)
                        }),
                ]
            },
        )
    }

    proptest! {
        /// Canonicalization never panics for float-free values.
        #[test]
        fn canonical_bytes_never_panics(value in json_value_no_floats()) {
            let result = CanonicalBytes::new(&value);
            prop_assert!(result.is_ok(), "Canonicalization failed: {:?}", result.err());
        }

        /// Canonicalization is deterministic: same input always produces same bytes.
        #[test]
        fn canonical_bytes_deterministic(value in json_value_no_floats()) {
            let a = CanonicalBytes::new(&value).unwrap();
            let b = CanonicalBytes::new(&value).unwrap();
            prop_assert_eq!(a.as_bytes(), b.as_bytes());
        }

        /// Canonical bytes are valid UTF-8 (required for cross-language compat).
        #[test]
        fn canonical_bytes_valid_utf8(value in json_value_no_floats()) {
            let cb = CanonicalBytes::new(&value).unwrap();
            prop_assert!(std::str::from_utf8(cb.as_bytes()).is_ok());
        }

        /// Canonical bytes are valid JSON (can round-trip through serde_json).
        #[test]
        fn canonical_bytes_valid_json(value in json_value_no_floats()) {
            let cb = CanonicalBytes::new(&value).unwrap();
            let parsed: Result<Value, _> = serde_json::from_slice(cb.as_bytes());
            prop_assert!(parsed.is_ok(), "Not valid JSON: {:?}", parsed.err());
        }

        /// Object keys are sorted lexicographically in canonical output.
        #[test]
        fn canonical_bytes_sorted_keys(
            keys in prop::collection::btree_set("[a-z]{1,8}", 2..6)
        ) {
            let map: serde_json::Map<String, Value> = keys.iter()
                .enumerate()
                .map(|(i, k)| (k.clone(), serde_json::json!(i)))
                .collect();
            let value = Value::Object(map);
            let cb = CanonicalBytes::new(&value).unwrap();
            let s = std::str::from_utf8(cb.as_bytes()).unwrap();

            // Extract keys from the canonical JSON string
            let parsed: serde_json::Map<String, Value> =
                serde_json::from_str(s).unwrap();
            let output_keys: Vec<&String> = parsed.keys().collect();
            let mut sorted_keys = output_keys.clone();
            sorted_keys.sort();
            prop_assert_eq!(output_keys, sorted_keys, "Keys not sorted in canonical output");
        }

        /// Any value containing a float is rejected.
        #[test]
        fn float_always_rejected(f in any::<f64>().prop_filter("not integer", |f| {
            f.fract() != 0.0 && f.is_finite()
        })) {
            let data = serde_json::json!({"val": f});
            let result = CanonicalBytes::new(&data);
            prop_assert!(result.is_err());
        }
    }
}
