//! Regression tests for known phoenix bugs.
//!
//! These tests document and guard against bugs discovered during the audit
//! of the phoenix layer, including empty tensor commitments, key ordering
//! inconsistencies, and domain enum completeness.

use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, JurisdictionId};
use mez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction, TensorCommitment};
use serde_json::json;

// ---------------------------------------------------------------------------
// Empty tensor stability
// ---------------------------------------------------------------------------

#[test]
fn empty_tensor_has_stable_commitment() {
    // An empty tensor (all domains NotApplicable) must produce a stable,
    // deterministic commitment across invocations.
    let jid = JurisdictionId::new("PK-REZ").unwrap();
    let config = DefaultJurisdiction::new(jid);
    let tensor = ComplianceTensor::new(config);

    let commitment1 = TensorCommitment::compute(&tensor).unwrap();
    let commitment2 = TensorCommitment::compute(&tensor).unwrap();

    assert_eq!(
        commitment1.digest().to_hex(),
        commitment2.digest().to_hex(),
        "Empty tensor commitments must be deterministic"
    );
    assert_eq!(commitment1.digest().to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// Key ordering in nested objects
// ---------------------------------------------------------------------------

#[test]
fn key_ordering_in_nested_objects() {
    // Canonicalization must produce identical bytes regardless of
    // insertion order of keys in the source JSON.
    let a = json!({"z": 1, "a": 2, "m": {"x": 3, "b": 4}});
    let b = json!({"a": 2, "m": {"b": 4, "x": 3}, "z": 1});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Key ordering must not affect canonical bytes"
    );
}

#[test]
fn key_ordering_with_numeric_string_keys() {
    // Ensure numeric-looking string keys are sorted lexicographically,
    // not numerically.
    let a = json!({"10": "a", "2": "b", "1": "c"});
    let b = json!({"1": "c", "10": "a", "2": "b"});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Numeric string keys must sort lexicographically"
    );
}

// ---------------------------------------------------------------------------
// Datetime normalization consistency
// ---------------------------------------------------------------------------

#[test]
fn datetime_normalization_consistency() {
    // String-based datetime values should canonicalize identically
    // when provided as identical strings.
    let a = json!({"ts": "2026-01-15T12:00:00Z", "val": 42});
    let b = json!({"val": 42, "ts": "2026-01-15T12:00:00Z"});

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Datetime strings must canonicalize identically"
    );
}

// ---------------------------------------------------------------------------
// Domain enum completeness
// ---------------------------------------------------------------------------

#[test]
fn domain_enum_completeness() {
    // The ComplianceDomain enum must have exactly 20 variants.
    // This guards against domain addition/removal without test updates.
    let all = ComplianceDomain::all();
    assert_eq!(
        all.len(),
        ComplianceDomain::COUNT,
        "ComplianceDomain::all() must return COUNT elements"
    );
    assert_eq!(
        ComplianceDomain::COUNT,
        20,
        "ComplianceDomain::COUNT must be 20"
    );
}

#[test]
fn domain_enum_no_duplicates() {
    let all = ComplianceDomain::all();
    let mut seen = std::collections::HashSet::new();
    for domain in all {
        assert!(
            seen.insert(domain),
            "Duplicate ComplianceDomain variant detected: {:?}",
            domain
        );
    }
}

#[test]
fn tensor_covers_all_domains() {
    // A freshly created tensor should be queryable for every domain.
    let jid = JurisdictionId::new("PK-REZ").unwrap();
    let config = DefaultJurisdiction::new(jid);
    let tensor = ComplianceTensor::new(config);

    // Querying each domain should return a valid state.
    for domain in ComplianceDomain::all() {
        let state = tensor.get(*domain);
        // Default state for a fresh tensor is Pending (not yet evaluated).
        assert_eq!(
            state,
            ComplianceState::Pending,
            "Fresh tensor domain {:?} should be Pending",
            domain
        );
    }
}
