//! # Canonical Serialization — JCS-Compatible Canonicalization
//!
//! This module defines [`CanonicalBytes`], the sole construction path for bytes
//! used in digest computation across the entire SEZ Stack.
//!
//! ## Security Invariant
//!
//! The inner `Vec<u8>` is private. The only way to construct `CanonicalBytes` is
//! through [`CanonicalBytes::new()`], which applies the full Momentum type coercion
//! pipeline before serialization. This makes the "wrong serialization path" class
//! of defects (audit finding §2.1) structurally impossible.
//!
//! ## Coercion Rules (matching Python `_coerce_json_types()` in `tools/lawpack.py`)
//!
//! 1. **Reject floats** — amounts must be strings or integers. A `serde_json::Number`
//!    that is f64-only (not representable as i64/u64) is rejected with
//!    [`CanonicalizationError::FloatRejected`].
//! 2. **Normalize datetimes** — strings that parse as RFC 3339 timestamps are
//!    normalized to UTC ISO 8601 with `Z` suffix, truncated to seconds. This
//!    matches Python's `datetime.astimezone(utc).replace(microsecond=0).isoformat()`.
//! 3. **Non-string dict keys** — handled at the serde level (Rust's type system
//!    ensures `serde_json::Map` keys are always `String`).
//! 4. **Tuples to lists** — handled at the serde level (Rust tuples serialize as
//!    JSON arrays).
//! 5. **Sort object keys** — `serde_json::Map` uses `BTreeMap` by default, which
//!    iterates keys in lexicographic order. `serde_json::to_vec` preserves this order.
//! 6. **Compact separators** — `serde_json::to_vec` produces compact JSON (no whitespace).
//!
//! ## Spec Reference
//!
//! Implements the canonicalization defined in `tools/lawpack.py:jcs_canonicalize()`.
//! Cross-language digest equality is verified by integration tests in
//! `tests/cross_language.rs`.

use serde::Serialize;
use serde_json::Value;

use crate::error::CanonicalizationError;

/// Bytes produced exclusively by JCS-compatible canonicalization with
/// Momentum-specific type coercion rules.
///
/// The inner `Vec<u8>` is private — downstream code cannot construct
/// `CanonicalBytes` except through [`CanonicalBytes::new()`]. This single
/// construction path ensures every digest in the system is computed from
/// properly canonicalized data.
///
/// # Security Invariant
///
/// Only `CanonicalBytes::new()` can create this type. The private inner field
/// makes "wrong serialization path" defects structurally impossible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Construct canonical bytes from any serializable value.
    ///
    /// Applies the full Momentum type coercion pipeline before serialization:
    /// 1. Converts to `serde_json::Value` via serde
    /// 2. Recursively coerces types (float rejection, datetime normalization)
    /// 3. Serializes with sorted keys and compact separators
    ///
    /// This is the **ONLY** way to construct `CanonicalBytes`. All digest
    /// computation in the entire stack must flow through this constructor.
    ///
    /// # Errors
    ///
    /// Returns [`CanonicalizationError::FloatRejected`] if any numeric value
    /// is a float (not representable as i64 or u64).
    /// Returns [`CanonicalizationError::SerializationFailed`] if serde
    /// serialization fails.
    pub fn new(obj: &impl Serialize) -> Result<Self, CanonicalizationError> {
        let value = serde_json::to_value(obj)?;
        let coerced = coerce_json_value(value)?;
        let bytes = serialize_canonical(&coerced)?;
        Ok(Self(bytes))
    }

    /// Construct canonical bytes from a pre-existing `serde_json::Value`.
    ///
    /// Applies the same coercion pipeline as [`CanonicalBytes::new()`].
    /// Useful when you already hold a `Value` and want to avoid a redundant
    /// serde round-trip.
    pub fn from_value(value: Value) -> Result<Self, CanonicalizationError> {
        let coerced = coerce_json_value(value)?;
        let bytes = serialize_canonical(&coerced)?;
        Ok(Self(bytes))
    }

    /// Access the canonical bytes for digest computation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Consume and return the inner byte vector.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }

    /// Return the length of the canonical byte representation.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Return whether the canonical byte representation is empty.
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
/// Matches the behavior of Python `_coerce_json_types()` in `tools/lawpack.py:111-138`:
/// - Rejects floats (numbers that are not representable as i64 or u64)
/// - Normalizes RFC 3339 datetime strings to UTC with Z suffix, truncated to seconds
/// - Recursively processes objects and arrays
/// - Passes through strings, booleans, integers, and null unchanged
fn coerce_json_value(value: Value) -> Result<Value, CanonicalizationError> {
    match value {
        Value::Number(ref n) => {
            // Match Python: isinstance(obj, float) → reject.
            // In serde_json, a Number from f64 has is_f64() true and may or may not
            // have is_i64()/is_u64() true (integer-valued floats like 1.0 are
            // representable as i64). We reject numbers that are ONLY representable
            // as f64 — this catches 1.5, 3.14, etc.
            // Integer-valued numbers (whether from i64 or integer-valued f64) pass through.
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                return Err(CanonicalizationError::FloatRejected(
                    n.as_f64().unwrap_or(f64::NAN),
                ));
            }
            Ok(value)
        }
        Value::Object(map) => {
            // serde_json::Map is BTreeMap by default → keys already sorted.
            // We rebuild to coerce child values.
            let mut coerced = serde_json::Map::new();
            for (k, v) in map {
                coerced.insert(k, coerce_json_value(v)?);
            }
            Ok(Value::Object(coerced))
        }
        Value::Array(arr) => {
            let coerced: Result<Vec<_>, _> = arr.into_iter().map(coerce_json_value).collect();
            Ok(Value::Array(coerced?))
        }
        Value::String(ref s) => {
            // Datetime normalization: if the string parses as RFC 3339,
            // normalize to UTC ISO 8601 with Z suffix, truncated to seconds.
            //
            // This matches the Python behavior for datetime objects:
            //   obj.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
            //
            // In Rust, DateTime<Utc> serializes via chrono as an RFC 3339 string.
            // This step normalizes any offset format (e.g., +00:00) to Z and
            // truncates subsecond precision.
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                let utc = dt.with_timezone(&chrono::Utc);
                Ok(Value::String(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
            } else {
                Ok(value)
            }
        }
        // Bool and Null pass through unchanged.
        other => Ok(other),
    }
}

/// Serialize a JSON value with sorted keys and compact separators.
///
/// Produces JCS-compatible output matching Python's:
///   `json.dumps(clean, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")`
///
/// `serde_json::to_vec` on a `Value` with `BTreeMap`-backed `Map`:
/// - Keys are serialized in BTreeMap iteration order (lexicographic = sorted)
/// - No whitespace between tokens (compact format)
/// - UTF-8 encoded (non-ASCII chars are NOT escaped, matching Python's `ensure_ascii=False`)
fn serialize_canonical(value: &Value) -> Result<Vec<u8>, CanonicalizationError> {
    Ok(serde_json::to_vec(value)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn canonical_sorts_keys() {
        let value = json!({"z": 1, "a": 2, "m": 3});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"a":2,"m":3,"z":1}"#);
    }

    #[test]
    fn canonical_nested_key_sorting() {
        let value = json!({"b": {"z": 1, "a": 2}, "a": 1});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"a":1,"b":{"a":2,"z":1}}"#);
    }

    #[test]
    fn canonical_rejects_float() {
        let value = json!({"amount": 3.15});
        let result = CanonicalBytes::new(&value);
        assert!(result.is_err());
        match result.unwrap_err() {
            CanonicalizationError::FloatRejected(f) => {
                assert!((f - 3.15).abs() < f64::EPSILON);
            }
            other => panic!("expected FloatRejected, got: {other}"),
        }
    }

    #[test]
    fn canonical_accepts_integers() {
        let value = json!({"count": 42, "negative": -7, "zero": 0});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"count":42,"negative":-7,"zero":0}"#);
    }

    #[test]
    fn canonical_normalizes_datetime_string() {
        // RFC 3339 with +00:00 offset → normalized to Z suffix, truncated to seconds
        let value = json!({"ts": "2026-01-15T12:00:00.123456+00:00"});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"ts":"2026-01-15T12:00:00Z"}"#);
    }

    #[test]
    fn canonical_normalizes_non_utc_datetime() {
        // RFC 3339 with +05:00 offset → converted to UTC with Z suffix
        let value = json!({"ts": "2026-01-15T17:00:00+05:00"});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"ts":"2026-01-15T12:00:00Z"}"#);
    }

    #[test]
    fn canonical_preserves_non_datetime_strings() {
        let value = json!({"name": "hello world", "id": "abc-123"});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"id":"abc-123","name":"hello world"}"#);
    }

    #[test]
    fn canonical_handles_empty_structures() {
        let empty_obj = json!({});
        let empty_arr = json!([]);
        assert_eq!(
            std::str::from_utf8(CanonicalBytes::new(&empty_obj).unwrap().as_bytes()).unwrap(),
            "{}"
        );
        assert_eq!(
            std::str::from_utf8(CanonicalBytes::new(&empty_arr).unwrap().as_bytes()).unwrap(),
            "[]"
        );
    }

    #[test]
    fn canonical_null_bool() {
        let value = json!({"flag": true, "nothing": null, "off": false});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"flag":true,"nothing":null,"off":false}"#);
    }

    #[test]
    fn canonical_is_deterministic() {
        let value = json!({"b": [3, 2, 1], "a": {"y": "hello", "x": 42}});
        let a = CanonicalBytes::new(&value).unwrap();
        let b = CanonicalBytes::new(&value).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn from_value_matches_new() {
        let value = json!({"key": "value", "n": 42});
        let from_new = CanonicalBytes::new(&value).unwrap();
        let from_value = CanonicalBytes::from_value(value).unwrap();
        assert_eq!(from_new, from_value);
    }

    #[test]
    fn canonical_len_and_is_empty() {
        let value = json!({});
        let cb = CanonicalBytes::new(&value).unwrap();
        assert_eq!(cb.len(), 2); // "{}"
        assert!(!cb.is_empty());
    }

    #[test]
    fn canonical_into_bytes() {
        let value = json!({"key": "val"});
        let cb = CanonicalBytes::new(&value).unwrap();
        let expected = cb.as_bytes().to_vec();
        let bytes = cb.into_bytes();
        assert_eq!(bytes, expected);
    }

    #[test]
    fn canonical_as_ref() {
        let value = json!({"x": 1});
        let cb = CanonicalBytes::new(&value).unwrap();
        let as_ref_bytes: &[u8] = cb.as_ref();
        assert_eq!(as_ref_bytes, cb.as_bytes());
    }

    #[test]
    fn canonical_bool_and_null_passthrough() {
        // Bool and null pass through the `other => Ok(other)` branch
        let value_true = json!(true);
        let cb = CanonicalBytes::new(&value_true).unwrap();
        assert_eq!(std::str::from_utf8(cb.as_bytes()).unwrap(), "true");

        let value_null = json!(null);
        let cb = CanonicalBytes::new(&value_null).unwrap();
        assert_eq!(std::str::from_utf8(cb.as_bytes()).unwrap(), "null");
    }

    #[test]
    fn canonical_clone_and_eq() {
        let value = json!({"a": 1});
        let cb = CanonicalBytes::new(&value).unwrap();
        let cb2 = cb.clone();
        assert_eq!(cb, cb2);
    }

    #[test]
    fn canonical_hash_works() {
        use std::collections::HashSet;
        let cb1 = CanonicalBytes::new(&json!({"a": 1})).unwrap();
        let cb2 = CanonicalBytes::new(&json!({"a": 2})).unwrap();
        let mut set = HashSet::new();
        set.insert(cb1.clone());
        set.insert(cb2);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&cb1));
    }

    #[test]
    fn canonical_array_with_nested_values() {
        let value = json!([{"b": 2, "a": 1}, null, true, "hello"]);
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"[{"a":1,"b":2},null,true,"hello"]"#);
    }

    #[test]
    fn canonical_rejects_float_in_nested_array() {
        let value = json!({"data": [1, 2, 3.15]});
        let result = CanonicalBytes::new(&value);
        assert!(result.is_err());
    }

    #[test]
    fn canonical_rejects_float_in_nested_object() {
        let value = json!({"outer": {"inner": 1.5}});
        let result = CanonicalBytes::new(&value);
        assert!(result.is_err());
    }

    // ── Coverage expansion tests (agent-added unique tests) ─────────

    #[test]
    fn canonical_from_value_rejects_float() {
        let value = json!({"x": 3.15});
        let result = CanonicalBytes::from_value(value);
        assert!(result.is_err());
    }

    #[test]
    fn canonical_debug_format() {
        let cb = CanonicalBytes::new(&json!({"test": true})).unwrap();
        let debug_str = format!("{cb:?}");
        assert!(debug_str.contains("CanonicalBytes"));
    }

    #[test]
    fn canonical_deeply_nested_object() {
        let value = json!({"a": {"b": {"c": {"d": 42}}}});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(s, r#"{"a":{"b":{"c":{"d":42}}}}"#);
    }

    #[test]
    fn canonical_string_with_special_chars() {
        let value = json!({"msg": "hello \"world\"\nnewline"});
        let cb = CanonicalBytes::new(&value).unwrap();
        let s = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert!(s.contains("hello"));
    }

    #[test]
    fn canonical_integer_zero() {
        let value = json!(0);
        let cb = CanonicalBytes::new(&value).unwrap();
        assert_eq!(std::str::from_utf8(cb.as_bytes()).unwrap(), "0");
    }

    #[test]
    fn canonical_negative_integer() {
        let value = json!(-42);
        let cb = CanonicalBytes::new(&value).unwrap();
        assert_eq!(std::str::from_utf8(cb.as_bytes()).unwrap(), "-42");
    }

    /// CRITICAL: Verify serde_json::Map iterates keys in sorted order.
    ///
    /// If preserve_order is enabled, Map uses IndexMap (insertion order)
    /// instead of BTreeMap (sorted order), silently corrupting every
    /// content-addressed digest in the system.
    ///
    /// If this test fails, run: cargo tree -e features -i serde_json
    #[test]
    fn serde_json_map_must_use_sorted_order() {
        let mut map = serde_json::Map::new();
        map.insert("z".to_string(), serde_json::Value::Null);
        map.insert("m".to_string(), serde_json::Value::Null);
        map.insert("a".to_string(), serde_json::Value::Null);
        let keys: Vec<&String> = map.keys().collect();
        assert_eq!(
            keys,
            vec!["a", "m", "z"],
            "CRITICAL: serde_json preserve_order is active — Map uses IndexMap not BTreeMap. \
             This corrupts ALL digests. Run: cargo tree -e features -i serde_json"
        );
    }

    /// End-to-end: canonical output has sorted keys from unsorted input.
    #[test]
    fn canonical_output_sorted_keys_from_reverse_input() {
        let input = r#"{"zebra":1,"apple":2,"mango":3}"#;
        let value: serde_json::Value = serde_json::from_str(input).unwrap();
        let cb = CanonicalBytes::new(&value).unwrap();
        let output = std::str::from_utf8(cb.as_bytes()).unwrap();
        assert_eq!(
            output, r#"{"apple":2,"mango":3,"zebra":1}"#,
            "Canonical output keys not sorted — preserve_order may be active"
        );
    }
}

/// Property-based tests using proptest.
///
/// These tests verify structural properties of canonicalization that must hold
/// for ALL valid inputs, not just specific test vectors.
#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use serde_json::Value;

    /// Strategy to generate arbitrary JSON values WITHOUT floats.
    ///
    /// Floats are excluded because they are intentionally rejected by the
    /// canonicalization pipeline. This strategy generates the full space of
    /// valid inputs.
    fn arb_json_value() -> impl Strategy<Value = Value> {
        let leaf = prop_oneof![
            Just(Value::Null),
            any::<bool>().prop_map(Value::Bool),
            // Use i64 range that fits in serde_json::Number
            (-1_000_000_000i64..1_000_000_000i64)
                .prop_map(|n| Value::Number(serde_json::Number::from(n))),
            "[a-zA-Z0-9 _-]{0,30}".prop_map(Value::String),
        ];
        leaf.prop_recursive(
            3,  // max depth
            64, // max nodes
            10, // items per collection
            |inner| {
                prop_oneof![
                    // Arrays
                    prop::collection::vec(inner.clone(), 0..5).prop_map(Value::Array),
                    // Objects with string keys
                    prop::collection::btree_map("[a-z_]{1,8}", inner, 0..5)
                        .prop_map(|m| { Value::Object(m.into_iter().collect()) }),
                ]
            },
        )
    }

    proptest! {
        /// Canonicalization is deterministic: same input always produces same bytes.
        #[test]
        fn canonical_is_deterministic(value in arb_json_value()) {
            let a = CanonicalBytes::new(&value).unwrap();
            let b = CanonicalBytes::new(&value).unwrap();
            prop_assert_eq!(a.as_bytes(), b.as_bytes());
        }

        /// Canonicalization is idempotent: canonicalizing already-canonical data
        /// produces identical bytes.
        #[test]
        fn canonical_is_idempotent(value in arb_json_value()) {
            let first = CanonicalBytes::new(&value).unwrap();
            // Parse the canonical bytes back to a Value and re-canonicalize.
            let reparsed: Value = serde_json::from_slice(first.as_bytes()).unwrap();
            let second = CanonicalBytes::new(&reparsed).unwrap();
            prop_assert_eq!(first.as_bytes(), second.as_bytes());
        }

        /// Object keys in canonical output are always lexicographically sorted.
        #[test]
        fn canonical_keys_are_sorted(
            keys in prop::collection::btree_set("[a-z]{1,8}", 1..10),
            val in -100i64..100i64,
        ) {
            let obj: serde_json::Map<String, Value> = keys
                .iter()
                .map(|k| (k.clone(), Value::Number(serde_json::Number::from(val))))
                .collect();
            let value = Value::Object(obj);
            let cb = CanonicalBytes::new(&value).unwrap();
            let reparsed: serde_json::Map<String, Value> =
                serde_json::from_slice(cb.as_bytes()).unwrap();
            let result_keys: Vec<&String> = reparsed.keys().collect();
            let mut sorted_keys = result_keys.clone();
            sorted_keys.sort();
            prop_assert_eq!(result_keys, sorted_keys);
        }

        /// All floats that are not integer-representable are rejected.
        #[test]
        fn canonical_rejects_true_floats(
            f in prop::num::f64::ANY.prop_filter("non-integer finite float",
                |f| f.is_finite() && f.fract() != 0.0)
        ) {
            // Construct a Value with a float Number directly.
            if let Some(n) = serde_json::Number::from_f64(f) {
                let value = Value::Object(
                    std::iter::once(("x".to_string(), Value::Number(n))).collect()
                );
                let result = CanonicalBytes::from_value(value);
                prop_assert!(result.is_err(), "Float {f} should have been rejected");
            }
            // If from_f64 returns None (NaN, inf), serde_json already prevents creation.
        }

        /// Canonical bytes are valid UTF-8 (required for cross-language compatibility
        /// with Python's str.encode("utf-8")).
        #[test]
        fn canonical_bytes_are_valid_utf8(value in arb_json_value()) {
            let cb = CanonicalBytes::new(&value).unwrap();
            prop_assert!(std::str::from_utf8(cb.as_bytes()).is_ok());
        }

        /// Canonical bytes parse back to logically equivalent JSON.
        #[test]
        fn canonical_roundtrip_preserves_data(value in arb_json_value()) {
            let cb = CanonicalBytes::new(&value).unwrap();
            let reparsed: Value = serde_json::from_slice(cb.as_bytes()).unwrap();
            // Re-canonicalize both and compare bytes (not Value equality,
            // because insertion order may differ).
            let cb2 = CanonicalBytes::new(&reparsed).unwrap();
            prop_assert_eq!(cb.as_bytes(), cb2.as_bytes());
        }
    }
}
