//! # Smart Asset Module Directory Structure Validation Test
//!
//! Tests that module descriptors can be serialized to JSON, canonicalized
//! through the CanonicalBytes pipeline, and produce deterministic digests.
//! Verifies the structural properties of module descriptor data.

use msez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Module descriptor has required fields
// ---------------------------------------------------------------------------

#[test]
fn module_descriptor_has_required_fields() {
    let descriptor = json!({
        "family": "corporate",
        "name": "formation",
        "version": "1.0.0",
        "domain": "corporate",
        "jurisdiction_scope": ["PK-RSEZ", "AE-DIFC"],
        "dependencies": [],
        "description": "Entity formation module",
        "status": "active"
    });

    // All required fields should be present
    assert!(descriptor.get("family").is_some());
    assert!(descriptor.get("name").is_some());
    assert!(descriptor.get("version").is_some());
    assert!(descriptor.get("domain").is_some());
    assert!(descriptor.get("jurisdiction_scope").is_some());

    // Should be canonicalizable
    let canonical = CanonicalBytes::new(&descriptor);
    assert!(canonical.is_ok());
}

// ---------------------------------------------------------------------------
// 2. Module descriptor digest is deterministic
// ---------------------------------------------------------------------------

#[test]
fn module_descriptor_digest_deterministic() {
    let descriptor = json!({
        "family": "tax",
        "name": "withholding",
        "version": "2.1.0",
        "domain": "tax",
        "jurisdiction_scope": ["PK-RSEZ"],
        "dependencies": ["corporate/formation"],
        "description": "Tax withholding at source"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&descriptor).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&descriptor).unwrap());
    assert_eq!(d1, d2);
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 3. Module directory structure (multiple descriptors)
// ---------------------------------------------------------------------------

#[test]
fn module_directory_structure() {
    let modules = json!({
        "corporate/formation": {
            "family": "corporate",
            "name": "formation",
            "version": "1.0.0"
        },
        "corporate/dissolution": {
            "family": "corporate",
            "name": "dissolution",
            "version": "1.0.0"
        },
        "tax/withholding": {
            "family": "tax",
            "name": "withholding",
            "version": "2.1.0"
        },
        "aml/screening": {
            "family": "aml",
            "name": "screening",
            "version": "1.0.0"
        }
    });

    // All module paths should be present
    assert!(modules.get("corporate/formation").is_some());
    assert!(modules.get("corporate/dissolution").is_some());
    assert!(modules.get("tax/withholding").is_some());
    assert!(modules.get("aml/screening").is_some());

    // The entire directory should canonicalize deterministically
    let d1 = sha256_digest(&CanonicalBytes::new(&modules).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&modules).unwrap());
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// 4. Key order in descriptor does not affect digest
// ---------------------------------------------------------------------------

#[test]
fn descriptor_key_order_invariant() {
    let v1 = json!({"z_field": "last", "a_field": "first", "m_field": "middle"});
    let v2 = json!({"a_field": "first", "m_field": "middle", "z_field": "last"});

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2);
}

// ---------------------------------------------------------------------------
// 5. Changed field produces different digest
// ---------------------------------------------------------------------------

#[test]
fn changed_field_produces_different_digest() {
    let v1 = json!({"name": "formation", "version": "1.0.0"});
    let v2 = json!({"name": "formation", "version": "1.0.1"});

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_ne!(d1, d2);
}
