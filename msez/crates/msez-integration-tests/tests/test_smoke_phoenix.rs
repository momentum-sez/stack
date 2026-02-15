//! # Smoke Tests for Phoenix Layer Components
//!
//! Quick validation that the core phoenix layer components (compliance tensor,
//! tensor commitment, mock proof system, CDB bridge, and canonical bytes)
//! can be instantiated and produce valid outputs.

use msez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, JurisdictionId};
use msez_tensor::{
    commitment::{commitment_digest, merkle_root, TensorCommitment},
    evaluation::ComplianceState,
    tensor::{ComplianceTensor, DefaultJurisdiction},
};
use msez_zkp::{Cdb, MockProofSystem, ProofSystem};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_jurisdiction() -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
}

// ---------------------------------------------------------------------------
// 1. Smoke: tensor creation
// ---------------------------------------------------------------------------

#[test]
fn smoke_tensor_creation() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    assert_eq!(tensor.cell_count(), 20);

    // All cells should have a default state
    for &domain in ComplianceDomain::all() {
        let state = tensor.get(domain);
        // Default state should be one of the valid ComplianceState variants
        let _format_check = format!("{state:?}");
    }
}

#[test]
fn smoke_tensor_set_and_get() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction());
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    assert_eq!(
        tensor.get(ComplianceDomain::Aml),
        ComplianceState::Compliant
    );
}

// ---------------------------------------------------------------------------
// 2. Smoke: tensor commitment
// ---------------------------------------------------------------------------

#[test]
fn smoke_tensor_commitment() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let commitment = tensor.commit().unwrap();

    // Commitment should be a valid 64-char hex string
    assert_eq!(commitment.to_hex().len(), 64);
    assert!(commitment.to_hex().chars().all(|c| c.is_ascii_hexdigit()));

    // Cell count should match tensor
    assert_eq!(commitment.cell_count(), 20);

    // Jurisdiction ID should be correct
    assert_eq!(commitment.jurisdiction_id(), "PK-RSEZ");
}

#[test]
fn smoke_tensor_commitment_deterministic() {
    let tensor = ComplianceTensor::new(test_jurisdiction());
    let c1 = tensor.commit().unwrap();
    let c2 = tensor.commit().unwrap();
    assert_eq!(c1.to_hex(), c2.to_hex());
}

#[test]
fn smoke_tensor_empty_commitment() {
    let c = TensorCommitment::empty("PK-RSEZ").unwrap();
    assert_eq!(c.to_hex().len(), 64);
    assert_eq!(c.cell_count(), 0);
}

// ---------------------------------------------------------------------------
// 3. Smoke: mock proof system
// ---------------------------------------------------------------------------

#[test]
fn smoke_mock_proof_system() {
    use msez_zkp::mock::{MockCircuit, MockProvingKey, MockVerifyingKey};

    let system = MockProofSystem;
    let pk = MockProvingKey;
    let _vk = MockVerifyingKey;

    let circuit = MockCircuit {
        circuit_data: json!({"type": "balance_check", "threshold": 1000}),
        public_inputs: b"test_inputs".to_vec(),
    };

    // Generate proof
    let proof = system.prove(&pk, &circuit).unwrap();
    assert_eq!(proof.proof_hex.len(), 64);
    assert!(proof.proof_hex.chars().all(|c| c.is_ascii_hexdigit()));

    // Proof should be deterministic
    let proof2 = system.prove(&pk, &circuit).unwrap();
    assert_eq!(proof, proof2);
}

#[test]
fn smoke_mock_proof_binds_circuit_data() {
    // BUG-048 RESOLVED: circuit_data is now hashed into the proof.
    // Different circuit data with same public inputs → different proofs.
    use msez_zkp::mock::{MockCircuit, MockProvingKey};

    let system = MockProofSystem;
    let pk = MockProvingKey;

    let circuit1 = MockCircuit {
        circuit_data: json!({"type": "a"}),
        public_inputs: b"inputs".to_vec(),
    };
    let circuit2 = MockCircuit {
        circuit_data: json!({"type": "b"}),
        public_inputs: b"inputs".to_vec(),
    };

    let proof1 = system.prove(&pk, &circuit1).unwrap();
    let proof2 = system.prove(&pk, &circuit2).unwrap();
    assert_ne!(
        proof1, proof2,
        "BUG-048 RESOLVED: different circuit_data produces different proofs"
    );

    // Same circuit data + same public inputs → same proof (deterministic).
    let circuit1_dup = MockCircuit {
        circuit_data: json!({"type": "a"}),
        public_inputs: b"inputs".to_vec(),
    };
    let proof1_dup = system.prove(&pk, &circuit1_dup).unwrap();
    assert_eq!(
        proof1, proof1_dup,
        "identical circuit_data + public_inputs → identical proofs"
    );

    // Different public_inputs MUST produce different proofs.
    let circuit3 = MockCircuit {
        circuit_data: json!({"type": "a"}),
        public_inputs: b"other-inputs".to_vec(),
    };
    let proof3 = system.prove(&pk, &circuit3).unwrap();
    assert_ne!(
        proof1, proof3,
        "different public_inputs must produce different proofs"
    );
}

// ---------------------------------------------------------------------------
// 4. Smoke: CDB bridge
// ---------------------------------------------------------------------------

#[test]
fn smoke_cdb_bridge() {
    let canonical = CanonicalBytes::new(&json!({"key": "value"})).unwrap();
    let digest = sha256_digest(&canonical);

    // Create CDB (Phase 1: identity)
    let cdb = Cdb::new(digest.clone());
    assert_eq!(cdb.to_hex().len(), 64);

    // In Phase 1, CDB is identity
    assert_eq!(cdb.as_digest(), &digest);

    // Display format
    let display = format!("{cdb}");
    assert!(display.starts_with("CDB(sha256:"));
}

#[test]
fn smoke_cdb_deterministic() {
    let canonical = CanonicalBytes::new(&json!({"test": true})).unwrap();
    let d1 = sha256_digest(&canonical);
    let d2 = sha256_digest(&canonical);
    let cdb1 = Cdb::new(d1);
    let cdb2 = Cdb::new(d2);
    assert_eq!(cdb1, cdb2);
}

#[test]
fn smoke_cdb_into_digest() {
    let canonical = CanonicalBytes::new(&json!({"k": "v"})).unwrap();
    let digest = sha256_digest(&canonical);
    let cdb = Cdb::new(digest.clone());
    let recovered = cdb.into_digest();
    assert_eq!(recovered, digest);
}

// ---------------------------------------------------------------------------
// 5. Smoke: canonical bytes
// ---------------------------------------------------------------------------

#[test]
fn smoke_canonical_bytes() {
    // Various data types should all produce canonical bytes
    let test_cases = vec![
        json!(null),
        json!(true),
        json!(false),
        json!(42),
        json!("hello"),
        json!([1, 2, 3]),
        json!({"key": "value"}),
        json!({}),
        json!([]),
    ];

    for data in test_cases {
        let cb = CanonicalBytes::new(&data).unwrap();
        assert!(
            !cb.as_bytes().is_empty(),
            "canonical bytes for {data:?} must be non-empty"
        );
        assert!(
            std::str::from_utf8(cb.as_bytes()).is_ok(),
            "canonical bytes for {data:?} must be valid UTF-8"
        );
    }
}

#[test]
fn smoke_canonical_bytes_sorted_keys() {
    let data = json!({"z": 1, "a": 2, "m": 3});
    let cb = CanonicalBytes::new(&data).unwrap();
    let s = std::str::from_utf8(cb.as_bytes()).unwrap();

    // Keys must be sorted: a < m < z
    let a_pos = s.find("\"a\"").unwrap();
    let m_pos = s.find("\"m\"").unwrap();
    let z_pos = s.find("\"z\"").unwrap();
    assert!(a_pos < m_pos);
    assert!(m_pos < z_pos);
}

// ---------------------------------------------------------------------------
// 6. Smoke: commitment_digest standalone
// ---------------------------------------------------------------------------

#[test]
fn smoke_commitment_digest_standalone() {
    let states: Vec<_> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, ComplianceState::Pending))
        .collect();
    let digest = commitment_digest("PK-RSEZ", &states).unwrap();
    assert_eq!(digest.to_hex().len(), 64);

    // Deterministic
    let digest2 = commitment_digest("PK-RSEZ", &states).unwrap();
    assert_eq!(digest, digest2);
}

#[test]
fn smoke_merkle_root() {
    let t1 = ComplianceTensor::new(test_jurisdiction());
    let c1 = t1.commit().unwrap();

    // Single commitment merkle root
    let root = merkle_root(std::slice::from_ref(&c1));
    assert!(root.is_some());
    assert_eq!(root.unwrap(), c1.to_hex());

    // Empty merkle root
    assert_eq!(merkle_root(&[]), None);
}
