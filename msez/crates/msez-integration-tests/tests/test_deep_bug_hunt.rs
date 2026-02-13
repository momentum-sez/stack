//! # Deep Bug Hunt Regression Tests
//!
//! Python counterpart: `tests/test_deep_bug_hunt.py`
//!
//! Deep regression tests for canonicalization edge cases:
//! - Deeply nested structures maintain canonical stability
//! - Large arrays produce deterministic results
//! - Mixed-type arrays are canonical
//! - Repeated canonicalization is idempotent

use msez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, JurisdictionId};
use msez_tensor::{
    evaluation::ComplianceState,
    tensor::{ComplianceTensor, DefaultJurisdiction},
};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Deep nesting canonical stability
// ---------------------------------------------------------------------------

#[test]
fn deep_nesting_canonical_stability() {
    // Build 10 levels of nesting
    let mut data = json!({"leaf": "value"});
    for i in 0..10 {
        data = json!({format!("level_{i}"): data});
    }

    let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2, "deeply nested structure must have stable digest");

    // Verify the canonical form is valid JSON
    let canonical = CanonicalBytes::new(&data).unwrap();
    let reparsed: serde_json::Value = serde_json::from_slice(canonical.as_bytes()).unwrap();
    assert!(reparsed.is_object());
}

// ---------------------------------------------------------------------------
// 2. Large array canonical stability
// ---------------------------------------------------------------------------

#[test]
fn large_array_canonical_stability() {
    let large_array: Vec<serde_json::Value> = (0..500)
        .map(|i| {
            json!({
                "index": i,
                "name": format!("item_{i:04}"),
                "active": i % 2 == 0
            })
        })
        .collect();
    let data = serde_json::Value::Array(large_array);

    let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2, "large array digest must be stable");
}

// ---------------------------------------------------------------------------
// 3. Mixed type array is canonical
// ---------------------------------------------------------------------------

#[test]
fn mixed_type_array_canonical() {
    let data = json!([
        1,
        "hello",
        true,
        null,
        {"nested": "object"},
        [1, 2, 3]
    ]);

    let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2, "mixed-type array must have stable digest");

    // Verify canonical form
    let canonical = CanonicalBytes::new(&data).unwrap();
    let canonical_str = std::str::from_utf8(canonical.as_bytes()).unwrap();
    assert!(canonical_str.starts_with('['));
    assert!(canonical_str.ends_with(']'));
}

// ---------------------------------------------------------------------------
// 4. Repeated canonicalization is idempotent
// ---------------------------------------------------------------------------

#[test]
fn repeated_canonicalization_idempotent() {
    let original = json!({
        "z_key": 3,
        "a_key": 1,
        "m_key": 2,
        "nested": {"z": true, "a": false}
    });

    // First canonicalization
    let canonical_1 = CanonicalBytes::new(&original).unwrap();
    let bytes_1 = canonical_1.as_bytes().to_vec();

    // Parse canonical bytes back and re-canonicalize
    let reparsed: serde_json::Value = serde_json::from_slice(&bytes_1).unwrap();
    let canonical_2 = CanonicalBytes::new(&reparsed).unwrap();
    let bytes_2 = canonical_2.as_bytes().to_vec();

    // Parse and canonicalize a third time
    let reparsed_2: serde_json::Value = serde_json::from_slice(&bytes_2).unwrap();
    let canonical_3 = CanonicalBytes::new(&reparsed_2).unwrap();
    let bytes_3 = canonical_3.as_bytes().to_vec();

    assert_eq!(
        bytes_1, bytes_2,
        "first and second canonicalization must be identical"
    );
    assert_eq!(
        bytes_2, bytes_3,
        "second and third canonicalization must be identical"
    );
}

// ---------------------------------------------------------------------------
// 5. Tensor commitment stability under repeated access
// ---------------------------------------------------------------------------

#[test]
fn tensor_commitment_repeated_access_stable() {
    let mut tensor = ComplianceTensor::new(DefaultJurisdiction::new(
        JurisdictionId::new("PK-RSEZ").unwrap(),
    ));
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Pending,
        vec![],
        None,
    );

    // Commit 10 times, all must be identical
    let first_commit = tensor.commit().unwrap().to_hex();
    for i in 1..10 {
        let commit = tensor.commit().unwrap().to_hex();
        assert_eq!(first_commit, commit, "commit {i} must match first commit");
    }
}

// ---------------------------------------------------------------------------
// 6. Unicode strings in canonical data
// ---------------------------------------------------------------------------

#[test]
fn unicode_strings_canonical() {
    let data = json!({
        "name_en": "Reko Diq Mining License",
        "name_ur": "\u{0631}\u{06CC}\u{06A9}\u{0648} \u{0688}\u{06A9}",
        "name_ar": "\u{0631}\u{064A}\u{0643}\u{0648} \u{062F}\u{0643}"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2, "unicode data digest must be stable");
}

// ---------------------------------------------------------------------------
// 7. Empty string vs absent field
// ---------------------------------------------------------------------------

#[test]
fn empty_string_vs_absent_field() {
    let with_empty = json!({"key": ""});
    let with_null = json!({"key": null});
    let without_key = json!({});

    let d_empty = sha256_digest(&CanonicalBytes::new(&with_empty).unwrap());
    let d_null = sha256_digest(&CanonicalBytes::new(&with_null).unwrap());
    let d_absent = sha256_digest(&CanonicalBytes::new(&without_key).unwrap());

    assert_ne!(d_empty, d_null, "empty string must differ from null");
    assert_ne!(
        d_empty, d_absent,
        "empty string must differ from absent key"
    );
    assert_ne!(d_null, d_absent, "null must differ from absent key");
}
