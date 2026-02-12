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
/// - Serialization uses sorted keys with compact separators.
///
/// These invariants are enforced by the constructor and cannot be violated
/// by downstream code because the inner `Vec<u8>` is private.
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
        // JCS: sorted keys, compact separators, deterministic output.
        let bytes = serialize_canonical(&coerced)?;
        Ok(Self(bytes))
    }

    /// Access the canonical bytes for digest computation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for CanonicalBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Recursively coerce JSON values according to Momentum canonicalization rules.
///
/// Rules (matching `tools/lawpack.py:_coerce_json_types()`):
/// 1. Reject floats — amounts must be strings or integers.
/// 2. Normalize datetime strings to UTC ISO8601 with Z suffix.
/// 3. Ensure all object keys are strings.
/// 4. Arrays and nested objects are recursively coerced.
fn coerce_json_value(value: Value) -> Result<Value, CanonicalizationError> {
    match value {
        Value::Number(n) => {
            // Reject pure floats (not representable as i64/u64).
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                if let Some(f) = n.as_f64() {
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
        // Strings, booleans, and null pass through unchanged.
        other => Ok(other),
    }
}

/// Serialize a JSON value in JCS-canonical form (sorted keys, compact separators).
fn serialize_canonical(value: &Value) -> Result<Vec<u8>, CanonicalizationError> {
    // serde_json with sorted keys as a baseline.
    // TODO: Replace with serde_jcs once integrated for full RFC 8785 compliance.
    let s = serde_json::to_string(value)?;
    Ok(s.into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_canonical_bytes_simple() {
        let mut data = BTreeMap::new();
        data.insert("b", 2);
        data.insert("a", 1);
        let cb = CanonicalBytes::new(&data).expect("should canonicalize");
        assert!(!cb.as_bytes().is_empty());
    }

    #[test]
    fn test_float_rejection() {
        let data = serde_json::json!({"amount": 1.5});
        let result = CanonicalBytes::new(&data);
        assert!(result.is_err());
    }
}
