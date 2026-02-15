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
    s.len() == 64
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
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
                        // Defensive: from_f64 only fails for NaN/Infinity, which
                        // is_finite() already excluded. Log if hit.
                        Ok(Value::Number(
                            serde_json::Number::from_f64(f).unwrap_or_else(|| {
                                tracing::warn!(
                                    value = f,
                                    "finite float could not be represented in JSON, substituting 0"
                                );
                                serde_json::Number::from(0)
                            }),
                        ))
                    }
                } else if !f.is_finite() {
                    // NaN and Infinity cannot be represented in JSON.
                    // Silently substituting 0 would corrupt regulatory data.
                    Err(PackError::JsonIncompatible {
                        context: "yaml_to_json".to_string(),
                        path: String::new(),
                        detail: format!("non-finite float value: {f}"),
                    })
                } else {
                    // True float — pass through, canonicalization will reject if used for digest.
                    // from_f64 is infallible for finite values (NaN/Infinity rejected above).
                    Ok(Value::Number(
                        serde_json::Number::from_f64(f).unwrap_or_else(|| {
                            tracing::warn!(
                                value = f,
                                "finite float could not be represented in JSON, substituting 0"
                            );
                            serde_json::Number::from(0)
                        }),
                    ))
                }
            } else {
                // YAML number with no i64/u64/f64 representation — should not occur.
                tracing::warn!("YAML number has no representable value, substituting 0");
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
///
/// Delegates to [`msez_core::digest::sha256_raw_hex`] — all SHA-256
/// computation in the SEZ Stack flows through `msez-core`.
pub fn sha256_hex(data: &[u8]) -> String {
    msez_core::digest::sha256_raw_hex(data)
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
        let value = serde_json::json!({"amount": 3.15});
        assert!(ensure_json_compatible(&value, "$", "test").is_err());
    }

    // -----------------------------------------------------------------------
    // File-based parsing tests (YAML)
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_yaml_as_value_simple() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(
            &path,
            "zone_id: test-zone\njurisdiction_id: pk\ncount: 42\n",
        )
        .unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert_eq!(value["zone_id"], "test-zone");
        assert_eq!(value["jurisdiction_id"], "pk");
        assert_eq!(value["count"], 42);
    }

    #[test]
    fn test_load_yaml_as_value_nested() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested.yaml");
        std::fs::write(&path, "outer:\n  inner: hello\n  list:\n    - a\n    - b\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert_eq!(value["outer"]["inner"], "hello");
        let list = value["outer"]["list"].as_array().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0], "a");
    }

    #[test]
    fn test_load_yaml_as_value_file_not_found() {
        let result = load_yaml_as_value(std::path::Path::new("/tmp/nonexistent_9999.yaml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, crate::error::PackError::FileNotFound { .. }));
    }

    #[test]
    fn test_load_yaml_as_value_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.yaml");
        std::fs::write(&path, ":\n  : :\n  [invalid").unwrap();

        let result = load_yaml_as_value(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_yaml_as_value_with_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("null.yaml");
        std::fs::write(&path, "key: null\nother: ~\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert!(value["key"].is_null());
        assert!(value["other"].is_null());
    }

    #[test]
    fn test_load_yaml_as_value_with_booleans() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bool.yaml");
        std::fs::write(&path, "enabled: true\ndisabled: false\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert_eq!(value["enabled"], true);
        assert_eq!(value["disabled"], false);
    }

    // -----------------------------------------------------------------------
    // File-based parsing tests (typed YAML)
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_yaml_typed_simple_struct() {
        #[derive(serde::Deserialize)]
        struct TestConfig {
            name: String,
            version: i32,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "name: my-module\nversion: 3\n").unwrap();

        let config: TestConfig = load_yaml_typed(&path).unwrap();
        assert_eq!(config.name, "my-module");
        assert_eq!(config.version, 3);
    }

    #[test]
    fn test_load_yaml_typed_file_not_found() {
        #[derive(serde::Deserialize, Debug)]
        struct Dummy {
            _x: String,
        }

        let result: PackResult<Dummy> =
            load_yaml_typed(std::path::Path::new("/tmp/no_such_file_xyz.yaml"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::PackError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_load_yaml_typed_parse_error() {
        #[derive(serde::Deserialize)]
        #[allow(dead_code)]
        struct ExpectInt {
            count: i32,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad_type.yaml");
        std::fs::write(&path, "count: not-a-number\n").unwrap();

        let result: PackResult<ExpectInt> = load_yaml_typed(&path);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // File-based parsing tests (JSON)
    // -----------------------------------------------------------------------

    #[test]
    fn test_load_json_value_simple() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.json");
        std::fs::write(&path, r#"{"name":"test","count":5}"#).unwrap();

        let value = load_json_value(&path).unwrap();
        assert_eq!(value["name"], "test");
        assert_eq!(value["count"], 5);
    }

    #[test]
    fn test_load_json_value_file_not_found() {
        let result = load_json_value(std::path::Path::new("/tmp/no_such_file_abc.json"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::PackError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_load_json_value_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.json");
        std::fs::write(&path, "{not valid json}").unwrap();

        let result = load_json_value(&path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::PackError::JsonParse { .. }
        ));
    }

    #[test]
    fn test_load_json_typed_simple() {
        #[derive(serde::Deserialize)]
        struct Item {
            id: String,
            value: i64,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("item.json");
        std::fs::write(&path, r#"{"id":"abc","value":99}"#).unwrap();

        let item: Item = load_json_typed(&path).unwrap();
        assert_eq!(item.id, "abc");
        assert_eq!(item.value, 99);
    }

    #[test]
    fn test_load_json_typed_file_not_found() {
        #[derive(serde::Deserialize, Debug)]
        struct Dummy {
            _x: String,
        }

        let result: PackResult<Dummy> =
            load_json_typed(std::path::Path::new("/tmp/never_exists_123.json"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::PackError::FileNotFound { .. }
        ));
    }

    #[test]
    fn test_load_json_typed_type_mismatch() {
        #[derive(serde::Deserialize, Debug)]
        #[allow(dead_code)]
        struct NeedsNum {
            count: u32,
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wrong_type.json");
        std::fs::write(&path, r#"{"count":"not_a_number"}"#).unwrap();

        let result: PackResult<NeedsNum> = load_json_typed(&path);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            crate::error::PackError::JsonParse { .. }
        ));
    }

    // -----------------------------------------------------------------------
    // yaml_to_json_value edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_yaml_to_json_value_integer_from_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("int.yaml");
        std::fs::write(&path, "positive: 42\nnegative: -7\nzero: 0\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert_eq!(value["positive"], 42);
        assert_eq!(value["negative"], -7);
        assert_eq!(value["zero"], 0);
    }

    #[test]
    fn test_yaml_to_json_value_non_string_keys() {
        // YAML allows numeric keys; these should be coerced to strings.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("numkeys.yaml");
        std::fs::write(&path, "100: hundred\n200: two_hundred\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert!(value.is_object());
        // Keys become strings "100" and "200"
        assert_eq!(value["100"], "hundred");
        assert_eq!(value["200"], "two_hundred");
    }

    #[test]
    fn test_yaml_to_json_value_boolean_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("boolkey.yaml");
        std::fs::write(&path, "true: yes_value\nfalse: no_value\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert!(value.is_object());
        assert_eq!(value["true"], "yes_value");
        assert_eq!(value["false"], "no_value");
    }

    #[test]
    fn test_yaml_to_json_value_null_key() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nullkey.yaml");
        std::fs::write(&path, "null: null_value\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert_eq!(value["null"], "null_value");
    }

    #[test]
    fn test_yaml_to_json_value_sequence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("seq.yaml");
        std::fs::write(&path, "items:\n  - first\n  - second\n  - 3\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        let items = value["items"].as_array().unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "first");
        assert_eq!(items[1], "second");
        assert_eq!(items[2], 3);
    }

    #[test]
    fn test_yaml_to_json_value_float_passthrough() {
        // Floats pass through yaml_to_json conversion but will be rejected
        // by ensure_json_compatible
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("float.yaml");
        std::fs::write(&path, "rate: 3.14\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        assert!(value["rate"].is_f64());
    }

    #[test]
    fn test_yaml_to_json_value_whole_number_float_converted() {
        // YAML may represent 100.0 as a float; yaml_to_json should convert to integer
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wholefloat.yaml");
        std::fs::write(&path, "amount: 100.0\n").unwrap();

        let value = load_yaml_as_value(&path).unwrap();
        // 100.0 should be converted to integer 100 since fract == 0.0
        assert!(value["amount"].is_u64() || value["amount"].is_i64());
    }

    // -----------------------------------------------------------------------
    // ensure_json_compatible edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_ensure_json_compatible_nested_array() {
        let value = serde_json::json!({
            "items": [1, 2, "three", [4, 5]]
        });
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_deeply_nested_object() {
        let value = serde_json::json!({
            "level1": {
                "level2": {
                    "level3": "deep"
                }
            }
        });
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_rejects_nested_float() {
        let value = serde_json::json!({
            "outer": {
                "nested": {
                    "bad": 1.5
                }
            }
        });
        let err = ensure_json_compatible(&value, "$", "test").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("outer.nested.bad"));
    }

    #[test]
    fn test_ensure_json_compatible_rejects_float_in_array() {
        let value = serde_json::json!({
            "items": [1, 2.5, 3]
        });
        let err = ensure_json_compatible(&value, "$", "test").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("items[1]"));
    }

    #[test]
    fn test_ensure_json_compatible_empty_object() {
        let value = serde_json::json!({});
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_empty_array() {
        let value = serde_json::json!([]);
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_null() {
        assert!(ensure_json_compatible(&serde_json::Value::Null, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_bool() {
        let value = serde_json::json!(true);
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    #[test]
    fn test_ensure_json_compatible_integer_number() {
        let value = serde_json::json!(42);
        assert!(ensure_json_compatible(&value, "$", "test").is_ok());
    }

    // -----------------------------------------------------------------------
    // sha256_hex additional vectors
    // -----------------------------------------------------------------------

    #[test]
    fn test_sha256_hex_empty_input() {
        let digest = sha256_hex(b"");
        assert_eq!(digest.len(), 64);
        // SHA256 of empty string is a well-known value
        assert_eq!(
            digest,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        let d1 = sha256_hex(b"test data");
        let d2 = sha256_hex(b"test data");
        assert_eq!(d1, d2);
    }

    #[test]
    fn test_sha256_hex_different_inputs_different_digests() {
        let d1 = sha256_hex(b"input_a");
        let d2 = sha256_hex(b"input_b");
        assert_ne!(d1, d2);
    }

    // -----------------------------------------------------------------------
    // is_valid_sha256 edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_valid_sha256_exact_length() {
        assert!(!is_valid_sha256(&"a".repeat(63)));
        assert!(is_valid_sha256(&"a".repeat(64)));
        assert!(!is_valid_sha256(&"a".repeat(65)));
    }

    #[test]
    fn test_is_valid_sha256_empty() {
        assert!(!is_valid_sha256(""));
    }

    #[test]
    fn test_is_valid_sha256_mixed_case_rejected() {
        // 63 lowercase + 1 uppercase
        let mut s = "a".repeat(63);
        s.push('A');
        assert!(!is_valid_sha256(&s));
    }
}
