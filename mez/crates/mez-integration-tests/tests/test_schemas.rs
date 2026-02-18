//! # Schema Validation Tests
//!
//! Verifies the schema infrastructure constants and policy checking functions.
//! Tests that security-critical schema lists are populated, extensible paths
//! are defined, schema IDs are non-empty, and the policy checker correctly
//! identifies violations vs. allowed extensibility points.

use mez_schema::{
    check_additional_properties_policy, EXTENSIBLE_PATHS, SECURITY_CRITICAL_SCHEMAS,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Security-critical schemas are listed
// ---------------------------------------------------------------------------

#[test]
fn security_critical_schemas_listed() {
    assert!(
        SECURITY_CRITICAL_SCHEMAS.len() >= 10,
        "at least 10 security-critical schemas must be defined (found {})",
        SECURITY_CRITICAL_SCHEMAS.len()
    );
}

#[test]
fn security_critical_schemas_have_json_extension() {
    for schema in SECURITY_CRITICAL_SCHEMAS {
        assert!(
            schema.ends_with(".schema.json"),
            "schema {schema} must end with .schema.json"
        );
    }
}

// ---------------------------------------------------------------------------
// 2. Extensible paths are defined
// ---------------------------------------------------------------------------

#[test]
fn extensible_paths_defined() {
    assert!(
        !EXTENSIBLE_PATHS.is_empty(),
        "extensible paths must be defined"
    );
}

#[test]
fn extensible_paths_contain_credential_subject() {
    assert!(
        EXTENSIBLE_PATHS.contains(&"credentialSubject"),
        "credentialSubject must be in extensible paths (W3C VC spec)"
    );
}

#[test]
fn extensible_paths_contain_metadata() {
    assert!(
        EXTENSIBLE_PATHS.contains(&"metadata"),
        "metadata must be in extensible paths (forward compatibility)"
    );
}

// ---------------------------------------------------------------------------
// 3. Schema IDs are non-empty
// ---------------------------------------------------------------------------

#[test]
fn schema_ids_are_non_empty() {
    for schema in SECURITY_CRITICAL_SCHEMAS {
        assert!(!schema.is_empty(), "schema ID must not be empty");
        assert!(schema.len() > 5, "schema ID {schema} is suspiciously short");
    }
}

// ---------------------------------------------------------------------------
// 4. Extensible paths reference valid schema positions
// ---------------------------------------------------------------------------

#[test]
fn extensible_paths_reference_valid_schemas() {
    // All extensible paths should be recognizable JSON Schema property names
    for path in EXTENSIBLE_PATHS {
        assert!(!path.is_empty(), "extensible path must not be empty");
        assert!(
            !path.contains('/'),
            "extensible path {path} should be a simple property name, not a JSON Pointer"
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Policy checker correctly identifies locked schemas
// ---------------------------------------------------------------------------

#[test]
fn policy_check_locked_schema_passes() {
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
        "locked schema should have no violations: {:?}",
        violations
    );
}

#[test]
fn policy_check_open_schema_reports_violation() {
    let open = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "additionalProperties": true
    });

    let violations = check_additional_properties_policy("test.schema.json", &open);
    assert_eq!(
        violations.len(),
        1,
        "open schema should report one violation"
    );
    assert_eq!(violations[0].path, "/");
    assert_eq!(violations[0].current_value, "true");
}

#[test]
fn policy_check_extensible_path_allowed() {
    // credentialSubject with additionalProperties: true should NOT be reported
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
        "credentialSubject should be allowed to be extensible: {:?}",
        violations
    );
}

#[test]
fn policy_check_nested_non_extensible_reported() {
    // A nested property that is NOT in EXTENSIBLE_PATHS should be flagged
    let schema = json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "proof": {
                "type": "object",
                "additionalProperties": true,
                "properties": {
                    "type": { "type": "string" }
                }
            }
        }
    });

    let violations = check_additional_properties_policy("test.schema.json", &schema);
    // "proof" is not in EXTENSIBLE_PATHS, so it should be flagged
    assert!(
        !violations.is_empty(),
        "proof with additionalProperties: true should be reported"
    );
}

// ---------------------------------------------------------------------------
// 6. Security-critical schemas are distinct
// ---------------------------------------------------------------------------

#[test]
fn security_critical_schemas_are_unique() {
    let mut seen = std::collections::HashSet::new();
    for schema in SECURITY_CRITICAL_SCHEMAS {
        assert!(
            seen.insert(*schema),
            "duplicate security-critical schema: {schema}"
        );
    }
}
