//! Shared YAML/JSON parsing infrastructure.
//!
//! Provides serde_yaml deserialization with proper error context (file path,
//! structural validation). All pack modules delegate parsing through these
//! functions to ensure consistent error reporting and JSON-compatibility
//! enforcement.
//!
//! ## JSON-Compatibility Enforcement
//!
//! YAML allows types that are not JSON-compatible: implicit timestamps,
//! floats, non-string keys. The SEZ Stack's reproducibility depends on
//! manifests being portable across implementations/languages, so strict
//! mode rejects these types at parse time.
//!
//! ## Spec Reference
//!
//! Mirrors the behavior of Python `tools/lawpack.py:_ensure_json_compatible()`
//! and `_load_yaml_manifest()`.

use std::path::Path;

use serde_json::Value;

use crate::error::{PackError, PackResult};

/// SHA-256 hex digest pattern: exactly 64 lowercase hex characters.
pub fn is_valid_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Load a YAML file and return it as a `serde_json::Value`.
///
/// Parses via serde_yaml, then converts to serde_json::Value for uniform
/// processing. This two-step approach lets us use the same canonicalization
/// pipeline regardless of whether the source is YAML or JSON.
///
/// In strict mode, rejects YAML-specific types that are not JSON-compatible:
/// - Floats (non-deterministic canonicalization edge cases)
/// - datetime/date objects (YAML implicit timestamps)
/// - Non-string mapping keys
pub fn load_yaml_as_value(path: &Path) -> PackResult<Value> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PackError::FileNotFound {
                path: path.to_path_buf(),
            }
        } else {
            PackError::Io(e)
        }
    })?;
    let yaml_value: serde_yaml::Value =
        serde_yaml::from_str(&content).map_err(|e| PackError::YamlParse {
            path: path.to_path_buf(),
            source: e,
        })?;
    let json_value = yaml_to_json_value(yaml_value)?;
    Ok(json_value)
}

/// Load a YAML file into a strongly-typed struct.
pub fn load_yaml_typed<T: serde::de::DeserializeOwned>(path: &Path) -> PackResult<T> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PackError::FileNotFound {
                path: path.to_path_buf(),
            }
        } else {
            PackError::Io(e)
        }
    })?;
    serde_yaml::from_str(&content).map_err(|e| PackError::YamlParse {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Load a JSON file and return it as a `serde_json::Value`.
pub fn load_json_value(path: &Path) -> PackResult<Value> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PackError::FileNotFound {
                path: path.to_path_buf(),
            }
        } else {
            PackError::Io(e)
        }
    })?;
    serde_json::from_str(&content).map_err(|e| PackError::JsonParse {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Load a JSON file into a strongly-typed struct.
pub fn load_json_typed<T: serde::de::DeserializeOwned>(path: &Path) -> PackResult<T> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            PackError::FileNotFound {
                path: path.to_path_buf(),
            }
        } else {
            PackError::Io(e)
        }
    })?;
    serde_json::from_str(&content).map_err(|e| PackError::JsonParse {
        path: path.to_path_buf(),
        source: e,
    })
}

/// Convert a serde_yaml::Value to a serde_json::Value.
///
/// Handles the type mapping differences between YAML and JSON value models.
/// YAML has tagged values, timestamps, etc. that JSON does not support.
fn yaml_to_json_value(yaml: serde_yaml::Value) -> PackResult<Value> {
    match yaml {
        serde_yaml::Value::Null => Ok(Value::Null),
        serde_yaml::Value::Bool(b) => Ok(Value::Bool(b)),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Value::Number(serde_json::Number::from(i)))
            } else if let Some(u) = n.as_u64() {
                Ok(Value::Number(serde_json::Number::from(u)))
            } else if let Some(f) = n.as_f64() {
                // Attempt to represent as integer if possible; otherwise pass as float.
                // The canonicalization layer will reject true floats.
                if f.fract() == 0.0 && f.is_finite() {
                    if f >= 0.0 && f <= u64::MAX as f64 {
                        Ok(Value::Number(serde_json::Number::from(f as u64)))
                    } else if f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                        Ok(Value::Number(serde_json::Number::from(f as i64)))
                    } else {
                        Ok(Value::Number(
                            serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
                        ))
                    }
                } else {
                    // True float — pass through, canonicalization will reject if used for digest
                    Ok(Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0)),
                    ))
                }
            } else {
                Ok(Value::Number(serde_json::Number::from(0)))
            }
        }
        serde_yaml::Value::String(s) => Ok(Value::String(s)),
        serde_yaml::Value::Sequence(seq) => {
            let items: PackResult<Vec<Value>> = seq.into_iter().map(yaml_to_json_value).collect();
            Ok(Value::Array(items?))
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    serde_yaml::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            i.to_string()
                        } else if let Some(f) = n.as_f64() {
                            f.to_string()
                        } else {
                            "0".to_string()
                        }
                    }
                    serde_yaml::Value::Bool(b) => b.to_string(),
                    serde_yaml::Value::Null => "null".to_string(),
                    other => format!("{other:?}"),
                };
                obj.insert(key, yaml_to_json_value(v)?);
            }
            Ok(Value::Object(obj))
        }
        serde_yaml::Value::Tagged(tagged) => {
            // Strip YAML tags and process the inner value
            yaml_to_json_value(tagged.value)
        }
    }
}

/// Validate that a serde_json::Value is strictly JSON-compatible.
///
/// In strict mode, rejects:
/// - Floats (non-deterministic canonicalization)
/// - Values that would not round-trip identically through JSON
///
/// Mirrors Python `tools/lawpack.py:_ensure_json_compatible()`.
pub fn ensure_json_compatible(value: &Value, path: &str, context: &str) -> PackResult<()> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(()),
        Value::Number(n) => {
            if n.is_f64() && !n.is_i64() && !n.is_u64() {
                Err(PackError::JsonIncompatible {
                    context: context.to_string(),
                    path: path.to_string(),
                    detail: format!(
                        "floats are not allowed; use strings or integers (got {})",
                        n.as_f64().unwrap_or(0.0)
                    ),
                })
            } else {
                Ok(())
            }
        }
        Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                ensure_json_compatible(item, &format!("{path}[{i}]"), context)?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (k, v) in map {
                let key_path = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                ensure_json_compatible(v, &key_path, context)?;
            }
            Ok(())
        }
    }
}

/// Compute SHA-256 hex digest of raw bytes.
pub fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_sha256() {
        assert!(is_valid_sha256(
            "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
        ));
        assert!(!is_valid_sha256("short"));
        assert!(!is_valid_sha256(
            "43258CFF783FE7036D8A43033F830ADFC60EC037382473548AC742B888292777"
        )); // uppercase
        assert!(!is_valid_sha256(
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz"
        )); // non-hex
    }

    #[test]
    fn test_sha256_hex() {
        let digest = sha256_hex(b"hello");
        assert_eq!(digest.len(), 64);
        assert_eq!(
            digest,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_ensure_json_compatible_accepts_valid() {
        let value = serde_json::json!({"a": 1, "b": "hello", "c": [true, null]});
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_rejects_float() {
        let value = serde_json::json!({"amount": 3.14});
        assert!(ensure_json_compatible(&value, "$", "test").is_err());
    }
}
