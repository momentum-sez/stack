//! # Bug Hunt Regression Tests
//!
//! Python counterpart: `tests/test_bughunt_regressions.py`
//!
//! Regression tests for edge cases in canonicalization:
//! - Floats in nested objects are rejected
//! - Empty array vs empty object produce different digests
//! - Null value handling
//! - Integer zero vs boolean false are distinguishable

use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Float in nested object is rejected
// ---------------------------------------------------------------------------

#[test]
fn regression_float_in_nested_object() {
    let data = json!({
        "entity": {
            "id": "ent-001",
            "balance": {
                "amount": 1.5,
                "currency": "PKR"
            }
        }
    });

    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "float nested inside object must be rejected"
    );
}

#[test]
fn regression_float_in_array_is_rejected() {
    let data = json!([1, 2, 3.15, 4]);
    let result = CanonicalBytes::new(&data);
    assert!(result.is_err(), "float inside array must be rejected");
}

// ---------------------------------------------------------------------------
// 2. Empty array vs empty object
// ---------------------------------------------------------------------------

#[test]
fn regression_empty_array_vs_empty_object() {
    let empty_arr = json!([]);
    let empty_obj = json!({});

    let d_arr = sha256_digest(&CanonicalBytes::new(&empty_arr).unwrap());
    let d_obj = sha256_digest(&CanonicalBytes::new(&empty_obj).unwrap());

    assert_ne!(
        d_arr, d_obj,
        "empty array and empty object must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 3. Null value handling
// ---------------------------------------------------------------------------

#[test]
fn regression_null_value_handling() {
    let with_null = json!({"key": null});
    let without_null = json!({"key": "value"});
    let empty = json!({});

    let d_null = sha256_digest(&CanonicalBytes::new(&with_null).unwrap());
    let d_value = sha256_digest(&CanonicalBytes::new(&without_null).unwrap());
    let d_empty = sha256_digest(&CanonicalBytes::new(&empty).unwrap());

    assert_ne!(d_null, d_value, "null value must differ from string value");
    assert_ne!(
        d_null, d_empty,
        "object with null key must differ from empty object"
    );
}

#[test]
fn regression_null_is_stable() {
    let data = json!(null);
    let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
    assert_eq!(d1, d2, "null digest must be stable");
}

// ---------------------------------------------------------------------------
// 4. Integer zero vs boolean false
// ---------------------------------------------------------------------------

#[test]
fn regression_integer_zero_vs_boolean_false() {
    let int_zero = json!({"val": 0});
    let bool_false = json!({"val": false});

    let d_zero = sha256_digest(&CanonicalBytes::new(&int_zero).unwrap());
    let d_false = sha256_digest(&CanonicalBytes::new(&bool_false).unwrap());

    assert_ne!(
        d_zero, d_false,
        "integer 0 and boolean false must produce different digests"
    );
}

#[test]
fn regression_boolean_true_vs_integer_one() {
    let int_one = json!({"val": 1});
    let bool_true = json!({"val": true});

    let d_one = sha256_digest(&CanonicalBytes::new(&int_one).unwrap());
    let d_true = sha256_digest(&CanonicalBytes::new(&bool_true).unwrap());

    assert_ne!(
        d_one, d_true,
        "integer 1 and boolean true must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 5. String "null" vs JSON null
// ---------------------------------------------------------------------------

#[test]
fn regression_string_null_vs_json_null() {
    let string_null = json!({"val": "null"});
    let json_null = json!({"val": null});

    let d_string = sha256_digest(&CanonicalBytes::new(&string_null).unwrap());
    let d_null = sha256_digest(&CanonicalBytes::new(&json_null).unwrap());

    assert_ne!(
        d_string, d_null,
        "string 'null' and JSON null must produce different digests"
    );
}

// ---------------------------------------------------------------------------
// 6. Compliance domain names in data are stable
// ---------------------------------------------------------------------------

#[test]
fn regression_compliance_domain_data_stability() {
    for &domain in ComplianceDomain::all() {
        let data = json!({"domain": domain.as_str(), "state": "compliant"});
        let d1 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
        let d2 = sha256_digest(&CanonicalBytes::new(&data).unwrap());
        assert_eq!(
            d1,
            d2,
            "compliance domain '{}' data digest must be stable",
            domain.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Integer boundary values
// ---------------------------------------------------------------------------

#[test]
fn regression_integer_boundaries() {
    let values = [i64::MIN, -1, 0, 1, i64::MAX];
    let digests: Vec<_> = values
        .iter()
        .map(|&v| sha256_digest(&CanonicalBytes::new(&json!({"n": v})).unwrap()).to_hex())
        .collect();

    // All must be unique
    for i in 0..digests.len() {
        for j in (i + 1)..digests.len() {
            assert_ne!(
                digests[i], digests[j],
                "values {} and {} must produce different digests",
                values[i], values[j]
            );
        }
    }
}
