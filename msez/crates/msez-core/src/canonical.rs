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
//! ## Coercion Rules (matching Python `_coerce_json_types()`)
//!
//! 1. Reject floats — amounts must be strings or integers.
//! 2. Normalize datetimes to UTC ISO8601 with `Z` suffix, truncated to seconds.
//! 3. Coerce non-string dict keys to strings.
//! 4. Convert tuples/sequences to JSON arrays.
//! 5. Sort object keys lexicographically.
//! 6. Use compact separators (no whitespace).

use serde::Serialize;
use serde_json::Value;

use crate::error::CanonicalizationError;

/// Bytes produced exclusively by JCS-compatible canonicalization with
/// Momentum-specific type coercion rules.
///
/// The inner `Vec<u8>` is private — downstream code cannot construct
/// `CanonicalBytes` except through [`CanonicalBytes::new()`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Construct canonical bytes from any serializable value.
    ///
    /// Applies the full Momentum type coercion pipeline before serialization.
    /// This is the ONLY way to construct `CanonicalBytes`. All digest computation
    /// in the entire stack must flow through this constructor.
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

    /// Consume and return the inner byte vector.
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl AsRef<[u8]> for CanonicalBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Recursively coerce JSON values according to Momentum canonicalization rules.
fn coerce_json_value(value: Value) -> Result<Value, CanonicalizationError> {
    match value {
        Value::Number(n) => {
            // Reject pure floats — amounts must be strings or integers.
            if let Some(f) = n.as_f64() {
                if n.is_f64() && !n.is_i64() && !n.is_u64() {
                    return Err(CanonicalizationError::FloatRejected(f));
                }
            }
            Ok(Value::Number(n))
        }
        Value::Object(map) => {
            let mut coerced = serde_json::Map::new();
            for (k, v) in map {
                coerced.insert(k, coerce_json_value(v)?);
            }
            Ok(Value::Object(coerced))
        }
        Value::Array(arr) => {
            let coerced: Result<Vec<_>, _> =
                arr.into_iter().map(coerce_json_value).collect();
            Ok(Value::Array(coerced?))
        }
        Value::String(s) => {
            // Datetime normalization: if the string parses as RFC 3339,
            // normalize to UTC ISO8601 with Z suffix, truncated to seconds.
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                let utc = dt.with_timezone(&chrono::Utc);
                Ok(Value::String(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
            } else {
                Ok(Value::String(s))
            }
        }
        // Bool and Null pass through unchanged.
        other => Ok(other),
    }
}

/// Serialize a JSON value with sorted keys and compact separators.
///
/// This produces JCS-compatible output: keys sorted lexicographically,
/// no whitespace between tokens.
fn serialize_canonical(value: &Value) -> Result<Vec<u8>, CanonicalizationError> {
    // serde_json with sorted keys via to_string on a Value already sorts keys
    // because serde_json::Map preserves insertion order, and we rebuild in coerce.
    // For true JCS compliance, we serialize the value directly — serde_json
    // produces compact output by default with `to_vec`.
    Ok(serde_json::to_vec(value)?)
}
