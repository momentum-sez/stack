//! # Schema Code Generation & Security Analysis
//!
//! Identifies and documents security-critical schemas per audit finding §3.1.
//!
//! ## Phase 1: Runtime Validation
//!
//! For Phase 1, this module provides runtime validation for security-critical
//! schemas with specific `additionalProperties` enforcement rules.
//!
//! ## Phase 2 (TODO): Compile-Time Code Generation
//!
//! In a future phase, this module will be integrated into `build.rs` to
//! generate Rust types from schema definitions at compile time, ensuring
//! the API surface and data model cannot diverge. The generated types
//! will enforce `additionalProperties: false` structurally — unknown
//! fields will simply have no corresponding struct field and will be
//! rejected by `#[serde(deny_unknown_fields)]`.
//!
//! ## Implements
//!
//! Audit §3.1 — Schema security: `additionalProperties: true` on security-critical schemas.

use serde_json::Value;

/// A security-critical schema with its `additionalProperties` enforcement rules.
///
/// Per audit §3.1, the following rules apply:
///
/// 1. **Top-level VC envelope:** `additionalProperties: false` — VC structure is standardized.
/// 2. **`credentialSubject`:** KEEP `true` — subjects are intentionally extensible per W3C VC spec.
/// 3. **`proof` array elements:** `additionalProperties: false` — proof structure must be rigid.
/// 4. **`metadata` or `extensions` objects:** KEEP `true` — designed for forward compatibility.
/// 5. **Transition `payload` objects:** KEEP `true` — payload schemas vary by transition type.
#[derive(Debug, Clone)]
pub struct SecuritySchemaSpec {
    /// Schema filename (e.g., "vc.smart-asset-registry.schema.json").
    pub schema_name: &'static str,
    /// Description of what this schema protects.
    pub description: &'static str,
    /// Whether the top-level `additionalProperties` should be `false`.
    pub top_level_locked: bool,
    /// Whether proof elements should have `additionalProperties: false`.
    pub proof_locked: bool,
}

/// List of all security-critical schemas identified by the audit.
///
/// These schemas protect VCs, receipts, attestations, and proofs.
/// An attacker who can inject unexpected fields into a VC can potentially
/// cause downstream processors to misinterpret authorization signals.
pub const SECURITY_CRITICAL_SCHEMAS: &[SecuritySchemaSpec] = &[
    SecuritySchemaSpec {
        schema_name: "vc.smart-asset-registry.schema.json",
        description: "Smart Asset Registry credential — binds assets to jurisdictional compliance profiles",
        // TODO: Per audit §3.1, top-level should be changed to false.
        // Currently `additionalProperties: true` in the schema file.
        // Changing it requires updating all producers and consumers.
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "corridor.receipt.schema.json",
        description: "Corridor state receipt — cryptographic proof of state transitions",
        // TODO: Per audit §3.1, top-level should be changed to false.
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "attestation.schema.json",
        description: "Generic attestation envelope — carries signed claims",
        top_level_locked: false,
        proof_locked: false,
    },
    SecuritySchemaSpec {
        schema_name: "corridor.checkpoint.schema.json",
        description: "Corridor checkpoint — commits to MMR state and receipt accumulator",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "corridor.fork-resolution.schema.json",
        description: "Fork resolution — selects canonical receipt for forked sequence points",
        top_level_locked: false,
        proof_locked: false,
    },
    SecuritySchemaSpec {
        schema_name: "vc.corridor-anchor.schema.json",
        description: "Corridor anchor credential — L1 chain commitment",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "vc.corridor-fork-resolution.schema.json",
        description: "Fork resolution credential — signed fork resolution decision",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "vc.corridor-lifecycle-transition.schema.json",
        description: "Corridor lifecycle transition credential",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "vc.watcher-bond.schema.json",
        description: "Watcher bond credential — stake commitment for corridor watchers",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "vc.dispute-claim.schema.json",
        description: "Dispute claim credential — initiates arbitration",
        top_level_locked: false,
        proof_locked: true,
    },
    SecuritySchemaSpec {
        schema_name: "vc.arbitration-award.schema.json",
        description: "Arbitration award credential — binding dispute resolution",
        top_level_locked: false,
        proof_locked: true,
    },
];

/// Audit the `additionalProperties` setting in a parsed schema value.
///
/// Returns a list of findings describing where `additionalProperties` is
/// set to `true` (or absent, which defaults to `true` in JSON Schema) at
/// security-sensitive locations.
///
/// This function checks:
/// - Top-level `additionalProperties`
/// - `proof` sub-schema `additionalProperties`
/// - Nested proof array items' `additionalProperties`
pub fn audit_additional_properties(schema: &Value) -> Vec<AdditionalPropertiesFinding> {
    let mut findings = Vec::new();

    // Check top-level additionalProperties
    match schema.get("additionalProperties") {
        Some(Value::Bool(false)) => {}
        Some(Value::Bool(true)) | None => {
            findings.push(AdditionalPropertiesFinding {
                json_path: "/additionalProperties".to_string(),
                current_value: schema
                    .get("additionalProperties")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "(absent, defaults to true)".to_string()),
                recommendation: "Set to false for security-critical envelope schemas".to_string(),
            });
        }
        Some(Value::Object(_)) => {
            // additionalProperties is a schema — this is fine (restricts rather than allows all).
        }
        _ => {}
    }

    // Check proof sub-schema
    if let Some(proof_schema) = schema.pointer("/properties/proof") {
        audit_proof_additional_properties(proof_schema, "/properties/proof", &mut findings);
    }

    findings
}

/// Check proof schemas for additionalProperties settings.
fn audit_proof_additional_properties(
    proof_schema: &Value,
    base_path: &str,
    findings: &mut Vec<AdditionalPropertiesFinding>,
) {
    // Proof can be a direct object or oneOf [object, array]
    if let Some(one_of) = proof_schema.get("oneOf") {
        if let Some(arr) = one_of.as_array() {
            for (i, variant) in arr.iter().enumerate() {
                let variant_path = format!("{base_path}/oneOf/{i}");
                check_object_additional_properties(variant, &variant_path, findings);

                // If this variant is an array type, check items
                if let Some(items) = variant.get("items") {
                    let items_path = format!("{variant_path}/items");
                    check_object_additional_properties(items, &items_path, findings);
                }
            }
        }
    } else {
        check_object_additional_properties(proof_schema, base_path, findings);
    }
}

/// Check a single object schema for additionalProperties.
fn check_object_additional_properties(
    schema: &Value,
    path: &str,
    findings: &mut Vec<AdditionalPropertiesFinding>,
) {
    if schema.get("type") == Some(&Value::String("object".to_string()))
        || schema.get("required").is_some()
    {
        match schema.get("additionalProperties") {
            Some(Value::Bool(false)) => {} // Locked down — good.
            Some(Value::Bool(true)) => {
                findings.push(AdditionalPropertiesFinding {
                    json_path: format!("{path}/additionalProperties"),
                    current_value: "true".to_string(),
                    recommendation: "Set to false for proof schemas".to_string(),
                });
            }
            None => {
                findings.push(AdditionalPropertiesFinding {
                    json_path: format!("{path}/additionalProperties"),
                    current_value: "(absent, defaults to true)".to_string(),
                    recommendation: "Set to false for proof schemas".to_string(),
                });
            }
            _ => {}
        }
    }
}

/// A finding about `additionalProperties` configuration.
#[derive(Debug, Clone)]
pub struct AdditionalPropertiesFinding {
    /// JSON Pointer path to the `additionalProperties` field.
    pub json_path: String,
    /// Current value of `additionalProperties`.
    pub current_value: String,
    /// Recommended action.
    pub recommendation: String,
}

impl std::fmt::Display for AdditionalPropertiesFinding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "  {}: {} → {}",
            self.json_path, self.current_value, self.recommendation
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.pop();
        dir.pop();
        dir
    }

    fn schema_dir() -> PathBuf {
        repo_root().join("schemas")
    }

    #[test]
    fn test_security_critical_schemas_listed() {
        assert!(
            SECURITY_CRITICAL_SCHEMAS.len() >= 10,
            "Expected >= 10 security-critical schemas, found {}",
            SECURITY_CRITICAL_SCHEMAS.len()
        );
    }

    #[test]
    fn test_security_critical_schemas_exist_on_disk() {
        let dir = schema_dir();
        for spec in SECURITY_CRITICAL_SCHEMAS {
            let path = dir.join(spec.schema_name);
            assert!(
                path.exists(),
                "Security-critical schema not found: {} (expected at {})",
                spec.schema_name,
                path.display()
            );
        }
    }

    #[test]
    fn test_audit_locked_schema() {
        let schema = json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "proof": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["type", "jws"],
                    "properties": {
                        "type": {"type": "string"},
                        "jws": {"type": "string"}
                    }
                }
            }
        });
        let findings = audit_additional_properties(&schema);
        assert!(
            findings.is_empty(),
            "Locked schema should produce no findings, got: {findings:?}"
        );
    }

    #[test]
    fn test_audit_unlocked_schema() {
        let schema = json!({
            "type": "object",
            "additionalProperties": true,
            "properties": {
                "proof": {
                    "type": "object",
                    "required": ["type", "jws"],
                    "properties": {
                        "type": {"type": "string"},
                        "jws": {"type": "string"}
                    }
                }
            }
        });
        let findings = audit_additional_properties(&schema);
        assert!(
            findings.len() >= 2,
            "Unlocked schema should produce findings for top-level and proof, got: {findings:?}"
        );
    }

    #[test]
    fn test_audit_real_vc_smart_asset_registry() {
        let path = schema_dir().join("vc.smart-asset-registry.schema.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            let schema: Value = serde_json::from_str(&content).unwrap();
            let findings = audit_additional_properties(&schema);
            // Per audit §3.1, this schema currently has additionalProperties: true
            // at the top level. We expect at least one finding.
            assert!(
                !findings.is_empty(),
                "vc.smart-asset-registry should have additionalProperties findings"
            );
        }
    }

    #[test]
    fn test_audit_corridor_receipt() {
        let path = schema_dir().join("corridor.receipt.schema.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            let schema: Value = serde_json::from_str(&content).unwrap();
            let findings = audit_additional_properties(&schema);
            assert!(
                !findings.is_empty(),
                "corridor.receipt should have additionalProperties findings"
            );
        }
    }

    #[test]
    fn test_proof_schema_already_locked() {
        // The vc.proof.jcs-ed25519 schema already has additionalProperties: false.
        let path = schema_dir().join("vc.proof.jcs-ed25519.schema.json");
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap();
            let schema: Value = serde_json::from_str(&content).unwrap();
            assert_eq!(
                schema.get("additionalProperties"),
                Some(&Value::Bool(false)),
                "vc.proof.jcs-ed25519 should have additionalProperties: false"
            );
        }
    }
}
