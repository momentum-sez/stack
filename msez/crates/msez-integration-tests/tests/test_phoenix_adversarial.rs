//! Adversarial input tests for phoenix components (tensor, migration, ZKP).
//!
//! Validates that the phoenix layer rejects malformed and adversarial inputs
//! with structured errors rather than panicking or producing incorrect results.
//! Covers canonicalization edge cases, ZKP determinism under adversarial data,
//! and CDB pipeline consistency.

use msez_core::{sha256_digest, CanonicalBytes};
use msez_zkp::mock::{MockCircuit, MockProvingKey};
use msez_zkp::{Cdb, MockProofSystem, ProofSystem};
use serde_json::json;

// ---------------------------------------------------------------------------
// Canonicalization adversarial inputs
// ---------------------------------------------------------------------------

#[test]
fn tensor_float_input_rejected() {
    // Float values must be rejected by the canonicalization pipeline.
    // Financial amounts must be strings or integers, never floats.
    let result = CanonicalBytes::new(&json!({"amount": 1.5}));
    assert!(
        result.is_err(),
        "Expected float rejection from CanonicalBytes::new, but got Ok"
    );
}

#[test]
fn canonicalization_rejects_nested_float() {
    // Floats nested inside objects must also be rejected.
    let data = json!({
        "outer": {
            "inner": {
                "value": 3.15
            }
        }
    });
    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "Expected nested float rejection from CanonicalBytes::new"
    );
}

#[test]
fn canonicalization_rejects_float_in_array() {
    let data = json!([1, 2, 3.5, 4]);
    let result = CanonicalBytes::new(&data);
    assert!(
        result.is_err(),
        "Expected float-in-array rejection from CanonicalBytes::new"
    );
}

// ---------------------------------------------------------------------------
// ZKP adversarial inputs
// ---------------------------------------------------------------------------

#[test]
fn mock_proof_deterministic_under_adversarial_input() {
    // The mock proof system must produce identical proofs for identical
    // inputs regardless of how many times it is called.
    let system = MockProofSystem;
    let pk = MockProvingKey;

    let circuit = MockCircuit {
        circuit_data: json!({"adversarial_key": "value", "z": 0, "a": 1}),
        public_inputs: b"adversarial-public-input".to_vec(),
    };

    let proof1 = system.prove(&pk, &circuit).unwrap();
    let proof2 = system.prove(&pk, &circuit).unwrap();
    assert_eq!(
        proof1, proof2,
        "Mock proof system must be deterministic for identical inputs"
    );
}

#[test]
fn mock_proof_distinct_for_different_inputs() {
    let system = MockProofSystem;
    let pk = MockProvingKey;

    let circuit_a = MockCircuit {
        circuit_data: json!({"key": "a"}),
        public_inputs: b"same".to_vec(),
    };
    let circuit_b = MockCircuit {
        circuit_data: json!({"key": "b"}),
        public_inputs: b"same".to_vec(),
    };

    let proof_a = system.prove(&pk, &circuit_a).unwrap();
    let proof_b = system.prove(&pk, &circuit_b).unwrap();
    assert_ne!(
        proof_a, proof_b,
        "Different circuit data must produce different proofs"
    );
}

// ---------------------------------------------------------------------------
// CDB (Canonical Digest Bridge) determinism
// ---------------------------------------------------------------------------

#[test]
fn cdb_canonical_bridge_deterministic() {
    // The CDB must produce identical bridged digests for identical inputs.
    let data = json!({"bridge_test": "determinism", "count": 42});
    let canonical = CanonicalBytes::new(&data).unwrap();
    let digest = sha256_digest(&canonical);

    let cdb1 = Cdb::new(digest.clone()).unwrap();
    let cdb2 = Cdb::new(digest).unwrap();

    assert_eq!(
        cdb1.as_digest().to_hex(),
        cdb2.as_digest().to_hex(),
        "CDB must be deterministic for identical input digests"
    );
}

#[test]
fn cdb_distinct_for_different_data() {
    let data_a = json!({"x": 1});
    let data_b = json!({"x": 2});

    let ca = CanonicalBytes::new(&data_a).unwrap();
    let cb = CanonicalBytes::new(&data_b).unwrap();

    let cdb_a = Cdb::new(sha256_digest(&ca)).unwrap();
    let cdb_b = Cdb::new(sha256_digest(&cb)).unwrap();

    assert_ne!(
        cdb_a.as_digest().to_hex(),
        cdb_b.as_digest().to_hex(),
        "CDB must produce distinct digests for distinct data"
    );
}

// ---------------------------------------------------------------------------
// Malformed JSON in canonicalization
// ---------------------------------------------------------------------------

#[test]
fn malformed_json_in_canonicalization() {
    // CanonicalBytes::new expects a valid serde_json::Value.
    // An empty object should succeed.
    let result = CanonicalBytes::new(&json!({}));
    assert!(
        result.is_ok(),
        "Empty object should canonicalize successfully"
    );

    // A null value should also succeed (it's valid JSON).
    let result_null = CanonicalBytes::new(&json!(null));
    assert!(
        result_null.is_ok(),
        "JSON null should canonicalize successfully"
    );
}

#[test]
fn adversarial_deeply_nested_object() {
    // Deeply nested objects should still canonicalize deterministically.
    let mut val = json!("leaf");
    for _ in 0..50 {
        val = json!({"nested": val});
    }
    let result = CanonicalBytes::new(&val);
    assert!(result.is_ok(), "Deeply nested object should canonicalize");

    // Same nesting should produce same digest.
    let canonical1 = result.unwrap();
    let canonical2 = CanonicalBytes::new(&val).unwrap();
    assert_eq!(
        sha256_digest(&canonical1).to_hex(),
        sha256_digest(&canonical2).to_hex(),
    );
}
