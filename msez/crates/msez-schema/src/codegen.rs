//! # Schema Codegen — Security-Critical Schema Analysis
//!
//! Provides compile-time and runtime utilities for working with the MSEZ
//! security-critical JSON schemas. Phase 1 implements runtime validation
//! helpers. A future phase will add compile-time Rust type generation
//! via `build.rs`.
//!
//! ## Security-critical schemas
//!
//! Per audit finding §3.1 (Schema Security — `additionalProperties: true`),
//! the following schemas must have `additionalProperties: false` at their
//! envelope level to prevent schema injection attacks:
//!
//! - `vc.smart-asset-registry.schema.json` — VC envelope level
//! - `corridor.receipt.schema.json` — Receipt envelope level
//! - `corridor.checkpoint.schema.json` — Checkpoint envelope level
//! - `corridor.fork-resolution.schema.json` — Fork resolution envelope level
//! - `attestation.schema.json` — Attestation envelope level
//! - `vc.corridor-anchor.schema.json` — VC envelope level
//! - `vc.corridor-fork-resolution.schema.json` — VC envelope level
//! - `vc.corridor-lifecycle-transition.schema.json` — VC envelope level
//! - `vc.watcher-bond.schema.json` — VC envelope level
//! - `vc.dispute-claim.schema.json` — VC envelope level
//! - `vc.arbitration-award.schema.json` — VC envelope level
//!
//! ### Rules for `additionalProperties` per audit:
//!
//! 1. **VC envelope (top level):** `false` — VC structure is standardized.
//! 2. **`credentialSubject`:** KEEP `true` — extensible per W3C VC spec.
//! 3. **`proof` array elements:** `false` — proof structure must be rigid.
//! 4. **`metadata` / `extensions`:** KEEP `true` — forward compatibility.
//! 5. **Transition `payload`:** KEEP `true` — varies by transition type.
//!
//! ## TODO: Compile-Time Codegen (Future Phase)
//!
//! In a future phase, a `build.rs` script will:
//! 1. Read security-critical schemas at compile time.
//! 2. Generate strongly-typed Rust structs with serde derives.
//! 3. Ensure `additionalProperties: false` schemas reject extra fields
//!    at the type level (no `#[serde(flatten)] HashMap` on locked types).
//! 4. Generate `validate()` methods on each struct that call the runtime
//!    [`SchemaValidator`](crate::validate::SchemaValidator) with the
//!    correct schema ID.

use serde_json::Value;

/// Schema filenames that are security-critical per audit §3.1.
///
/// These schemas MUST have `additionalProperties: false` at their
/// envelope level to prevent schema injection attacks. A runtime
/// check is provided by [`check_additional_properties_policy`].
pub const SECURITY_CRITICAL_SCHEMAS: &[&str] = &[
    "vc.smart-asset-registry.schema.json",
    "corridor.receipt.schema.json",
    "corridor.checkpoint.schema.json",
    "corridor.fork-resolution.schema.json",
    "attestation.schema.json",
    "vc.corridor-anchor.schema.json",
    "vc.corridor-fork-resolution.schema.json",
    "vc.corridor-lifecycle-transition.schema.json",
    "vc.watcher-bond.schema.json",
    "vc.dispute-claim.schema.json",
    "vc.arbitration-award.schema.json",
];

/// Paths within security-critical schemas where `additionalProperties`
/// SHOULD remain `true` for extensibility per the W3C VC spec and
/// the audit guidelines.
pub const EXTENSIBLE_PATHS: &[&str] = &[
    "credentialSubject",
    "metadata",
    "extensions",
    "payload",
    "meta",
    "public_inputs",
];

/// A violation of the `additionalProperties` security policy.
#[derive(Debug, Clone)]
pub struct AdditionalPropertiesViolation {
    /// The schema filename.
    pub schema_filename: String,
    /// JSON Pointer path within the schema where the violation was found.
    pub path: String,
    /// The current value of `additionalProperties` (expected `false`).
    pub current_value: String,
}

impl std::fmt::Display for AdditionalPropertiesViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: additionalProperties={} at {} (expected false)",
            self.schema_filename, self.current_value, self.path,
        )
    }
}

/// Check a schema's `additionalProperties` policy against audit §3.1 rules.
///
/// Returns a list of violations where `additionalProperties` is `true`
/// (or absent, which defaults to `true` in JSON Schema) at security-critical
/// positions that should have it set to `false`.
///
/// This does NOT modify the schema — it only reports violations. The actual
/// fix should be applied to the schema files per the audit remediation plan.
pub fn check_additional_properties_policy(
    schema_filename: &str,
    schema: &Value,
) -> Vec<AdditionalPropertiesViolation> {
    let mut violations = Vec::new();
    check_object_node(
        schema_filename,
        schema,
        "",
        &mut violations,
    );
    violations
}

/// Recursively check an object schema node for `additionalProperties` policy.
fn check_object_node(
    schema_filename: &str,
    node: &Value,
    path: &str,
    violations: &mut Vec<AdditionalPropertiesViolation>,
) {
    let Some(obj) = node.as_object() else {
        return;
    };

    // Determine if this node is a JSON Schema object type.
    let is_object_type = obj
        .get("type")
        .and_then(|v| v.as_str())
        .is_some_and(|t| t == "object");

    if is_object_type {
        // Extract the last segment of the path for extensibility check.
        let last_segment = path.rsplit('/').next().unwrap_or("");
        let is_extensible = EXTENSIBLE_PATHS.contains(&last_segment);

        if !is_extensible {
            // Check additionalProperties.
            let additional = obj.get("additionalProperties");
            match additional {
                Some(Value::Bool(false)) => {
                    // Correct — locked down.
                }
                Some(Value::Bool(true)) => {
                    violations.push(AdditionalPropertiesViolation {
                        schema_filename: schema_filename.to_string(),
                        path: if path.is_empty() {
                            "/".to_string()
                        } else {
                            path.to_string()
                        },
                        current_value: "true".to_string(),
                    });
                }
                None if path.is_empty() => {
                    // Top-level missing additionalProperties — report as violation
                    // since it defaults to true in JSON Schema.
                    violations.push(AdditionalPropertiesViolation {
                        schema_filename: schema_filename.to_string(),
                        path: "/".to_string(),
                        current_value: "absent (defaults to true)".to_string(),
                    });
                }
                _ => {
                    // additionalProperties is a schema object or absent at non-root.
                    // Non-root absent is less critical; we only flag explicit `true`.
                }
            }
        }
    }

    // Recurse into properties.
    if let Some(properties) = obj.get("properties").and_then(|v| v.as_object()) {
        for (key, value) in properties {
            let child_path = format!("{path}/{key}");
            check_object_node(schema_filename, value, &child_path, violations);
        }
    }

    // Recurse into definitions / $defs.
    for defs_key in &["definitions", "$defs"] {
        if let Some(defs) = obj.get(*defs_key).and_then(|v| v.as_object()) {
            for (key, value) in defs {
                let child_path = format!("{path}/{defs_key}/{key}");
                check_object_node(schema_filename, value, &child_path, violations);
            }
        }
    }

    // Recurse into items (for array types).
    if let Some(items) = obj.get("items") {
        let child_path = format!("{path}/items");
        check_object_node(schema_filename, items, &child_path, violations);
    }

    // Recurse into oneOf / anyOf / allOf.
    for combiner in &["oneOf", "anyOf", "allOf"] {
        if let Some(variants) = obj.get(*combiner).and_then(|v| v.as_array()) {
            for (i, variant) in variants.iter().enumerate() {
                let child_path = format!("{path}/{combiner}/{i}");
                check_object_node(schema_filename, variant, &child_path, violations);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop(); // crates
        dir.pop(); // msez
        dir.pop(); // stack
        dir
    }

    fn load_schema(filename: &str) -> Value {
        let path = repo_root().join("schemas").join(filename);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {filename}: {e}"));
        serde_json::from_str(&content)
            .unwrap_or_else(|e| panic!("Failed to parse {filename}: {e}"))
    }

    #[test]
    fn test_security_critical_schemas_list() {
        // Verify all listed schemas exist on disk.
        for filename in SECURITY_CRITICAL_SCHEMAS {
            let path = repo_root().join("schemas").join(filename);
            assert!(
                path.exists(),
                "Security-critical schema not found: {filename}"
            );
        }
    }

    #[test]
    fn test_check_policy_locked_schema() {
        // A schema with additionalProperties: false should have no violations
        // at the top level.
        let locked = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": false
        });

        let violations = check_additional_properties_policy("test.schema.json", &locked);
        assert!(
            violations.is_empty(),
            "Locked schema should have no violations: {violations:?}"
        );
    }

    #[test]
    fn test_check_policy_open_schema() {
        // A schema with additionalProperties: true should report a violation.
        let open = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "additionalProperties": true
        });

        let violations = check_additional_properties_policy("test.schema.json", &open);
        assert_eq!(violations.len(), 1, "Should report one violation");
        assert_eq!(violations[0].path, "/");
    }

    #[test]
    fn test_check_policy_extensible_paths_allowed() {
        // credentialSubject with additionalProperties: true should NOT be
        // reported because it's in the EXTENSIBLE_PATHS list.
        let vc_like = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "credentialSubject": {
                    "type": "object",
                    "additionalProperties": true,
                    "properties": {
                        "id": { "type": "string" }
                    }
                }
            }
        });

        let violations = check_additional_properties_policy("test.schema.json", &vc_like);
        assert!(
            violations.is_empty(),
            "credentialSubject should be allowed to be extensible: {violations:?}"
        );
    }

    #[test]
    fn test_audit_current_schema_state() {
        // This test documents the CURRENT state of the security-critical schemas.
        // Per audit §3.1, some schemas currently have additionalProperties: true
        // at the envelope level. This test reports but does not fail — the fix
        // is tracked in the audit remediation plan.
        let mut total_violations = 0;
        for filename in SECURITY_CRITICAL_SCHEMAS {
            let path = repo_root().join("schemas").join(filename);
            if !path.exists() {
                continue;
            }

            let schema = load_schema(filename);
            let violations = check_additional_properties_policy(filename, &schema);
            if !violations.is_empty() {
                eprintln!("Schema {filename} has {} policy violations:", violations.len());
                for v in &violations {
                    eprintln!("  {v}");
                }
                total_violations += violations.len();
            }
        }

        if total_violations > 0 {
            eprintln!(
                "\nTotal additionalProperties policy violations: {total_violations}"
            );
            eprintln!("These are documented findings per audit §3.1.");
        }
    }
}
