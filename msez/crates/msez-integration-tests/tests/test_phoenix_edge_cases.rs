//! Edge case tests for phoenix components.
//!
//! Tests boundary conditions in canonicalization including empty objects,
//! null values, large integers, deeply nested structures, and boolean values.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_zkp::mock::{MockCircuit, MockProvingKey, MockVerifyingKey};
use msez_zkp::{MockProofSystem, ProofSystem};
use serde_json::json;

// ---------------------------------------------------------------------------
// Canonicalization edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_object_canonical_bytes() {
    let data = json!({});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Must be deterministic.
    let canonical2 = CanonicalBytes::new(&data).unwrap();
    assert_eq!(digest.to_hex(), sha256_digest(&canonical2).to_hex());
}

#[test]
fn null_value_canonical_bytes() {
    let data = json!(null);
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

#[test]
fn null_field_canonical_bytes() {
    let data = json!({"key": null, "other": 1});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Null field must be different from missing field.
    let data_without_null = json!({"other": 1});
    let canonical2 = CanonicalBytes::new(&data_without_null).unwrap();
    assert_ne!(
        digest.to_hex(),
        sha256_digest(&canonical2).to_hex(),
        "Null field must produce different digest than missing field"
    );
}

#[test]
fn large_integer_canonical_bytes() {
    // Large integers (within i64 range) must canonicalize.
    let data = json!({"big": 9_007_199_254_740_992_i64});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

#[test]
fn negative_integer_canonical_bytes() {
    let data = json!({"negative": -42});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Must differ from positive.
    let data_pos = json!({"negative": 42});
    let canonical_pos = CanonicalBytes::new(&data_pos).unwrap();
    assert_ne!(digest.to_hex(), sha256_digest(&canonical_pos).to_hex());
}

#[test]
fn deeply_nested_canonical_bytes() {
    // Build a 20-level nested object.
    let mut val = json!("leaf");
    for i in 0..20 {
        val = json!({format!("level_{}", i): val});
    }
    let canonical = CanonicalBytes::new(&val).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

#[test]
fn boolean_canonical_bytes() {
    let data_true = json!({"flag": true});
    let data_false = json!({"flag": false});

    let ct = CanonicalBytes::new(&data_true).unwrap();
    let cf = CanonicalBytes::new(&data_false).unwrap();

    assert_ne!(
        sha256_digest(&ct).to_hex(),
        sha256_digest(&cf).to_hex(),
        "true and false must produce different digests"
    );
}

#[test]
fn empty_array_canonical_bytes() {
    let data = json!({"items": []});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

#[test]
fn empty_string_in_object_canonical_bytes() {
    let data = json!({"key": ""});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Empty string must differ from missing key.
    let data_no_key = json!({});
    let canonical2 = CanonicalBytes::new(&data_no_key).unwrap();
    assert_ne!(digest.to_hex(), sha256_digest(&canonical2).to_hex());
}

// ---------------------------------------------------------------------------
// Mock proof system edge cases
// ---------------------------------------------------------------------------

#[test]
fn mock_proof_with_empty_public_inputs() {
    let system = MockProofSystem;
    let pk = MockProvingKey;
    let circuit = MockCircuit {
        circuit_data: json!({"test": "empty_inputs"}),
        public_inputs: vec![],
    };
    let proof = system.prove(&pk, &circuit).unwrap();
    assert_eq!(proof.proof_hex.len(), 64);
}

#[test]
fn mock_proof_verification_with_wrong_inputs() {
    let system = MockProofSystem;
    let pk = MockProvingKey;
    let vk = MockVerifyingKey;

    let circuit = MockCircuit {
        circuit_data: json!({"key": "value"}),
        public_inputs: b"correct".to_vec(),
    };

    let proof = system.prove(&pk, &circuit).unwrap();

    // Verification with different public inputs should return false.
    let valid = system.verify(&vk, &proof, b"wrong").unwrap();
    assert!(
        !valid,
        "Verification with wrong public inputs must return false"
    );
}
