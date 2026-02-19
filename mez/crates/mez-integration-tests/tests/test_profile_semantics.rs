//! # Profile Validation Semantics Integration Tests
//!
//! Python counterpart: `tests/test_profile_semantics.py`
//!
//! Tests profile descriptor validation:
//! - Profile descriptors contain required fields
//! - Profile descriptor digests are deterministic
//! - Profiles with module lists have stable canonical representations

use mez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Profile descriptor has required fields
// ---------------------------------------------------------------------------

#[test]
fn profile_descriptor_has_required_fields() {
    let profile = json!({
        "profile_id": "trade-zone",
        "profile_name": "Trade Zone Profile",
        "jurisdiction_id": "PK-REZ",
        "modules": [
            "corporate/formation",
            "tax/withholding",
            "aml/screening",
            "kyc/identity"
        ],
        "version": "1.0"
    });

    // All required fields are present and non-null
    assert!(profile["profile_id"].is_string());
    assert!(profile["profile_name"].is_string());
    assert!(profile["jurisdiction_id"].is_string());
    assert!(profile["modules"].is_array());
    assert!(profile["version"].is_string());

    // Can be canonicalized
    let canonical = CanonicalBytes::new(&profile).unwrap();
    assert!(!canonical.as_bytes().is_empty());
}

// ---------------------------------------------------------------------------
// 2. Profile descriptor digest is deterministic
// ---------------------------------------------------------------------------

#[test]
fn profile_descriptor_digest_deterministic() {
    let profile = json!({
        "profile_id": "default",
        "profile_name": "Default Profile",
        "jurisdiction_id": "PK-REZ",
        "modules": ["corporate/formation", "tax/reporting"],
        "version": "1.0"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&profile).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&profile).unwrap());
    let d3 = sha256_digest(&CanonicalBytes::new(&profile).unwrap());

    assert_eq!(d1, d2, "first and second must match");
    assert_eq!(d2, d3, "second and third must match");
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 3. Profile with modules list
// ---------------------------------------------------------------------------

#[test]
fn profile_with_modules_list() {
    let profile = json!({
        "profile_id": "financial-services",
        "modules": [
            "aml/screening",
            "aml/transaction-monitoring",
            "kyc/identity",
            "kyc/due-diligence",
            "sanctions/screening",
            "tax/withholding",
            "tax/reporting",
            "securities/disclosure",
            "custody/safekeeping"
        ]
    });

    let canonical = CanonicalBytes::new(&profile).unwrap();
    let canonical_str = std::str::from_utf8(canonical.as_bytes()).unwrap();

    // Verify modules appear in canonical form
    assert!(canonical_str.contains("aml/screening"));
    assert!(canonical_str.contains("custody/safekeeping"));

    // Digest is valid
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 4. Different profiles produce different digests
// ---------------------------------------------------------------------------

#[test]
fn different_profiles_different_digests() {
    let profile_a = json!({
        "profile_id": "minimal",
        "modules": ["corporate/formation"]
    });

    let profile_b = json!({
        "profile_id": "comprehensive",
        "modules": ["corporate/formation", "tax/withholding", "aml/screening"]
    });

    let d_a = sha256_digest(&CanonicalBytes::new(&profile_a).unwrap());
    let d_b = sha256_digest(&CanonicalBytes::new(&profile_b).unwrap());
    assert_ne!(
        d_a, d_b,
        "different profiles must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 5. Profile key ordering does not affect digest
// ---------------------------------------------------------------------------

#[test]
fn profile_key_ordering_invariant() {
    let v1 = json!({
        "version": "1.0",
        "profile_id": "test",
        "modules": ["a", "b"]
    });
    let v2 = json!({
        "modules": ["a", "b"],
        "profile_id": "test",
        "version": "1.0"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&v1).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&v2).unwrap());
    assert_eq!(d1, d2, "key insertion order must not affect profile digest");
}

// ---------------------------------------------------------------------------
// 6. Empty profile is valid
// ---------------------------------------------------------------------------

#[test]
fn empty_profile_valid() {
    let profile = json!({
        "profile_id": "empty",
        "modules": []
    });

    let canonical = CanonicalBytes::new(&profile).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 7. Profile with nested metadata
// ---------------------------------------------------------------------------

#[test]
fn profile_with_nested_metadata() {
    let profile = json!({
        "profile_id": "enriched",
        "modules": ["corporate/formation"],
        "metadata": {
            "author": "mass-ez-admin",
            "created_at": "2026-01-15T12:00:00Z",
            "tags": ["financial", "trade-zone"]
        }
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&profile).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&profile).unwrap());
    assert_eq!(d1, d2, "nested metadata digest must be deterministic");
}
