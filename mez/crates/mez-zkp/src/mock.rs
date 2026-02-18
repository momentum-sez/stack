//! # Mock Proof System (Phase 1)
//!
//! A deterministic, transparent proof system for development and testing.
//! Produces SHA-256-based "proofs" that are verifiable but provide **no
//! zero-knowledge guarantees**.
//!
//! ## How It Works
//!
//! - `prove()` computes `SHA256(canonical_bytes(circuit_data) || public_inputs)`
//!   and returns the hex-encoded digest as the proof.
//! - `verify()` recomputes the same digest and checks equality.
//!
//! This matches the deterministic mock behavior of the Python implementation
//! in `tools/phoenix/zkp.py`, which uses `hashlib.sha256` for deterministic
//! proofs and `secrets.token_hex(32)` for random ones.
//!
//! ## Security Warning
//!
//! **NOT PRIVATE.** The mock proof system is transparent — anyone can recompute
//! the proof from the inputs. It exists solely for Phase 1 deterministic
//! compliance evaluation where ZK privacy is not required.
//!
//! ## Spec Reference
//!
//! Audit §2.5: Phase 1 mock proofs are explicitly acknowledged as non-private.
//! Real ZK backends activate in Phase 2 via feature flags.

use mez_core::digest::Sha256Accumulator;
use mez_core::CanonicalBytes;
use serde::{Deserialize, Serialize};

use crate::traits::{ProofError, ProofSystem, VerifyError};

/// A mock proof — deterministic SHA-256 digest of circuit data concatenated
/// with public inputs.
///
/// **NOT PRIVATE.** Provides no zero-knowledge guarantees. The proof is a
/// transparent hash that anyone can recompute from the same inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MockProof {
    /// Hex-encoded SHA-256 digest: `SHA256(canonical(circuit) || public_inputs)`.
    pub proof_hex: String,
}

/// Mock verifying key — stateless in Phase 1.
///
/// Verification is deterministic recomputation, so the key carries no secrets.
#[derive(Debug, Clone)]
pub struct MockVerifyingKey;

/// Mock proving key — stateless in Phase 1.
///
/// Proof generation is deterministic hashing, so the key carries no secrets.
#[derive(Debug, Clone)]
pub struct MockProvingKey;

/// Mock circuit data for Phase 1 deterministic proofs.
///
/// Contains the serializable circuit payload and the public inputs that
/// the proof will bind to. The `circuit_data` field holds the canonical
/// representation of the circuit's constraint-relevant state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockCircuit {
    /// Canonical JSON-serializable circuit data (public inputs + structure).
    /// Serialized with sorted keys and compact separators for deterministic
    /// digest computation.
    pub circuit_data: serde_json::Value,

    /// Public inputs as raw bytes. These are concatenated with the canonical
    /// circuit data bytes before hashing to produce the proof.
    #[serde(with = "hex_bytes")]
    pub public_inputs: Vec<u8>,
}

/// A deterministic mock proof system for Phase 1.
///
/// Produces SHA-256 digests as "proofs." Verification recomputes the digest
/// and checks equality. This is functionally equivalent to the Python
/// implementation in `tools/phoenix/zkp.py:MockProver`.
///
/// ## Proof Generation
///
/// ```text
/// proof = SHA256( canonical_bytes(circuit_data) || public_inputs )
/// ```
///
/// Circuit data is canonicalized and included in the proof hash to bind
/// the proof to a specific circuit. Different circuits with identical
/// public inputs produce different proofs.
///
/// ## Verification
///
/// ```text
/// expected = SHA256( canonical_circuit_bytes || public_inputs )
/// valid = (proof == expected)
/// ```
///
/// The verifier must supply the canonical circuit bytes prepended to
/// the public inputs. In practice, callers should use `prove()` to
/// generate the proof and `verify()` with the same concatenated bytes.
///
/// ## Security Invariant
///
/// This system is labeled `Mock` and is clearly non-private. It MUST NOT
/// be used in any context where zero-knowledge privacy is required.
pub struct MockProofSystem;

impl ProofSystem for MockProofSystem {
    type Proof = MockProof;
    type VerifyingKey = MockVerifyingKey;
    type ProvingKey = MockProvingKey;
    type Circuit = MockCircuit;

    /// Generate a deterministic mock proof.
    ///
    /// Canonicalizes `circuit.circuit_data`, then computes
    /// `SHA256(canonical_bytes(circuit_data) || public_inputs)`.
    /// This binds the proof to both the circuit structure and the public inputs,
    /// ensuring different circuits produce different proofs even with identical
    /// public inputs.
    ///
    /// # SHA-256 exception: mock proof generation
    ///
    /// Uses `Sha256Accumulator` instead of `sha256_digest(&CanonicalBytes)`
    /// because proof generation hashes `canonical_bytes || public_inputs` — a
    /// composite of canonicalized JSON data and raw binary public inputs. The
    /// circuit data IS canonicalized via `CanonicalBytes` before hashing. The
    /// real proof system (Phase 2) will replace this with actual ZKP circuit
    /// evaluation.
    fn prove(
        &self,
        _pk: &Self::ProvingKey,
        circuit: &Self::Circuit,
    ) -> Result<Self::Proof, ProofError> {
        // Canonicalize circuit data (rejects floats, ensures deterministic bytes).
        let canonical = CanonicalBytes::from_value(circuit.circuit_data.clone()).map_err(|e| {
            ProofError::GenerationFailed(format!("failed to canonicalize circuit data: {e}"))
        })?;

        // SHA256(canonical_bytes(circuit_data) || public_inputs)
        let mut acc = Sha256Accumulator::new();
        acc.update(canonical.as_bytes());
        acc.update(&circuit.public_inputs);
        let proof_hex = acc.finalize_hex();

        Ok(MockProof { proof_hex })
    }

    /// Verify a mock proof by recomputing the expected digest.
    ///
    /// Recomputes `SHA256(public_inputs)` and checks equality with the proof.
    ///
    /// The `public_inputs` must be the same concatenated bytes that `prove()`
    /// hashed internally: `canonical_bytes(circuit_data) || original_public_inputs`.
    /// For convenience, prefer [`MockProofSystem::verify_circuit`] which accepts
    /// a `MockCircuit` directly and handles the concatenation.
    ///
    /// # SHA-256 exception: mock proof verification
    ///
    /// Uses `Sha256Accumulator` to recompute the expected digest from raw
    /// binary public inputs, matching the accumulator path used by `prove()`.
    fn verify(
        &self,
        _vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_inputs: &[u8],
    ) -> Result<bool, VerifyError> {
        if proof.proof_hex.len() != 64 {
            return Err(VerifyError::MalformedProof(format!(
                "expected 64 hex chars, got {}",
                proof.proof_hex.len()
            )));
        }

        if !proof.proof_hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(VerifyError::MalformedProof(
                "proof_hex contains non-hex characters".to_string(),
            ));
        }

        // Recompute: SHA256(public_inputs) via Sha256Accumulator to match prove().
        let mut acc = Sha256Accumulator::new();
        acc.update(public_inputs);
        let expected_hex = acc.finalize_hex();

        Ok(proof.proof_hex == expected_hex)
    }
}

impl MockProofSystem {
    /// Verify a mock proof against a circuit directly.
    ///
    /// This is the recommended verification path for mock proofs. It
    /// internally reconstructs `canonical_bytes(circuit_data) || public_inputs`
    /// to match the hash computed by `prove()`, eliminating the asymmetry
    /// between prove and verify API surfaces.
    pub fn verify_circuit(
        &self,
        vk: &MockVerifyingKey,
        proof: &MockProof,
        circuit: &MockCircuit,
    ) -> Result<bool, VerifyError> {
        let canonical = CanonicalBytes::from_value(circuit.circuit_data.clone()).map_err(|e| {
            VerifyError::MalformedProof(format!("failed to canonicalize circuit data: {e}"))
        })?;

        let mut verify_input = canonical.as_bytes().to_vec();
        verify_input.extend_from_slice(&circuit.public_inputs);
        self.verify(vk, proof, &verify_input)
    }
}

/// Serde helper for hex-encoding `Vec<u8>` fields.
mod hex_bytes {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        serializer.serialize_str(&hex)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(serde::de::Error::custom))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_system() -> (MockProofSystem, MockProvingKey, MockVerifyingKey) {
        (MockProofSystem, MockProvingKey, MockVerifyingKey)
    }

    #[test]
    fn prove_produces_64_hex_char_proof() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"type": "balance_check", "threshold": 1000}),
            public_inputs: b"test_inputs".to_vec(),
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        assert_eq!(proof.proof_hex.len(), 64);
        assert!(proof.proof_hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prove_is_deterministic() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"asset_id": "A001", "jurisdiction": "PK"}),
            public_inputs: b"same_inputs".to_vec(),
        };
        let proof1 = sys.prove(&pk, &circuit).unwrap();
        let proof2 = sys.prove(&pk, &circuit).unwrap();
        assert_eq!(proof1, proof2);
    }

    #[test]
    fn different_circuit_data_produces_different_proofs() {
        // BUG-048 RESOLVED: circuit_data is now included in the proof hash.
        // Different circuits with same public_inputs produce different proofs.
        let (sys, pk, _vk) = make_system();
        let circuit1 = MockCircuit {
            circuit_data: json!({"type": "a"}),
            public_inputs: b"inputs".to_vec(),
        };
        let circuit2 = MockCircuit {
            circuit_data: json!({"type": "b"}),
            public_inputs: b"inputs".to_vec(),
        };
        let proof1 = sys.prove(&pk, &circuit1).unwrap();
        let proof2 = sys.prove(&pk, &circuit2).unwrap();
        assert_ne!(
            proof1, proof2,
            "different circuits must produce different proofs"
        );
    }

    #[test]
    fn different_public_inputs_produce_different_proofs() {
        let (sys, pk, _vk) = make_system();
        let circuit1 = MockCircuit {
            circuit_data: json!({"type": "x"}),
            public_inputs: b"input_a".to_vec(),
        };
        let circuit2 = MockCircuit {
            circuit_data: json!({"type": "x"}),
            public_inputs: b"input_b".to_vec(),
        };
        let proof1 = sys.prove(&pk, &circuit1).unwrap();
        let proof2 = sys.prove(&pk, &circuit2).unwrap();
        assert_ne!(proof1, proof2);
    }

    #[test]
    fn verify_rejects_malformed_proof_wrong_length() {
        let (sys, _pk, vk) = make_system();
        let bad_proof = MockProof {
            proof_hex: "abcd".to_string(),
        };
        let result = sys.verify(&vk, &bad_proof, b"inputs");
        assert!(result.is_err());
        match result.unwrap_err() {
            VerifyError::MalformedProof(msg) => assert!(msg.contains("64 hex chars")),
            other => panic!("expected MalformedProof, got: {other}"),
        }
    }

    #[test]
    fn verify_rejects_malformed_proof_invalid_hex() {
        let (sys, _pk, vk) = make_system();
        let bad_proof = MockProof {
            proof_hex: "g".repeat(64),
        };
        let result = sys.verify(&vk, &bad_proof, b"inputs");
        assert!(result.is_err());
        match result.unwrap_err() {
            VerifyError::MalformedProof(msg) => assert!(msg.contains("non-hex")),
            other => panic!("expected MalformedProof, got: {other}"),
        }
    }

    #[test]
    fn mock_proof_serialization_roundtrip() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"field": "value"}),
            public_inputs: vec![1, 2, 3, 4],
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        let serialized = serde_json::to_string(&proof).unwrap();
        let deserialized: MockProof = serde_json::from_str(&serialized).unwrap();
        assert_eq!(proof, deserialized);
    }

    #[test]
    fn mock_circuit_serialization_roundtrip() {
        let circuit = MockCircuit {
            circuit_data: json!({"test": true}),
            public_inputs: vec![0xde, 0xad, 0xbe, 0xef],
        };
        let serialized = serde_json::to_string(&circuit).unwrap();
        let deserialized: MockCircuit = serde_json::from_str(&serialized).unwrap();
        assert_eq!(circuit.public_inputs, deserialized.public_inputs);
        assert_eq!(circuit.circuit_data, deserialized.circuit_data);
    }

    #[test]
    fn empty_public_inputs_valid() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"empty": true}),
            public_inputs: vec![],
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        assert_eq!(proof.proof_hex.len(), 64);
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn prove_then_verify_roundtrip() {
        // Proofs generated by prove() must be verifiable by verify()
        // when the verifier supplies canonical(circuit_data) || public_inputs.
        let (sys, pk, vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"type": "balance_check", "threshold": 1000}),
            public_inputs: b"test_public_inputs".to_vec(),
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        // Build verification input: canonical_bytes(circuit_data) || public_inputs
        let canonical = CanonicalBytes::from_value(circuit.circuit_data.clone()).unwrap();
        let mut verify_input = canonical.as_bytes().to_vec();
        verify_input.extend_from_slice(&circuit.public_inputs);
        let result = sys.verify(&vk, &proof, &verify_input).unwrap();
        assert!(
            result,
            "verify must return true for a proof generated by prove()"
        );
    }

    #[test]
    fn prove_then_verify_rejects_wrong_public_inputs() {
        let (sys, pk, vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"type": "balance_check"}),
            public_inputs: b"correct_inputs".to_vec(),
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        let result = sys.verify(&vk, &proof, b"wrong_inputs").unwrap();
        assert!(
            !result,
            "verify must return false when public inputs don't match"
        );
    }

    #[test]
    fn verify_returns_false_when_proof_does_not_match() {
        let (sys, _pk, vk) = make_system();
        // Create a valid 64-char hex proof that doesn't match
        let proof = MockProof {
            proof_hex: "aa".repeat(32),
        };
        let result = sys.verify(&vk, &proof, b"some inputs").unwrap();
        assert!(
            !result,
            "verify should return false when proof doesn't match"
        );
    }

    #[test]
    fn prove_with_complex_circuit_data() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({
                "nested": {
                    "array": [1, 2, 3],
                    "bool": true,
                    "null": null,
                    "str": "value"
                },
                "top_level": 42
            }),
            public_inputs: vec![0xde, 0xad, 0xbe, 0xef],
        };
        let proof = sys.prove(&pk, &circuit).unwrap();
        assert_eq!(proof.proof_hex.len(), 64);
    }

    #[test]
    fn prove_rejects_float_in_circuit_data() {
        let (sys, pk, _vk) = make_system();
        let circuit = MockCircuit {
            circuit_data: json!({"amount": 3.15}),
            public_inputs: vec![],
        };
        let result = sys.prove(&pk, &circuit);
        assert!(result.is_err());
        match result.unwrap_err() {
            ProofError::GenerationFailed(msg) => {
                assert!(msg.contains("canonicalize"));
            }
            other => panic!("expected GenerationFailed, got: {other}"),
        }
    }

    #[test]
    fn mock_proof_clone() {
        let proof = MockProof {
            proof_hex: "ab".repeat(32),
        };
        let cloned = proof.clone();
        assert_eq!(proof, cloned);
    }

    #[test]
    fn mock_verifying_key_debug() {
        let vk = MockVerifyingKey;
        let debug = format!("{vk:?}");
        assert!(debug.contains("MockVerifyingKey"));
    }

    #[test]
    fn mock_proving_key_debug() {
        let pk = MockProvingKey;
        let debug = format!("{pk:?}");
        assert!(debug.contains("MockProvingKey"));
    }

    #[test]
    fn mock_circuit_debug() {
        let circuit = MockCircuit {
            circuit_data: json!({"test": true}),
            public_inputs: vec![1, 2, 3],
        };
        let debug = format!("{circuit:?}");
        assert!(debug.contains("MockCircuit"));
    }

    #[test]
    fn hex_bytes_roundtrip_empty() {
        let circuit = MockCircuit {
            circuit_data: json!({}),
            public_inputs: vec![],
        };
        let json_str = serde_json::to_string(&circuit).unwrap();
        let deserialized: MockCircuit = serde_json::from_str(&json_str).unwrap();
        assert!(deserialized.public_inputs.is_empty());
    }

    #[test]
    fn hex_bytes_roundtrip_large() {
        let large_inputs: Vec<u8> = (0..=255).collect();
        let circuit = MockCircuit {
            circuit_data: json!({"size": "large"}),
            public_inputs: large_inputs.clone(),
        };
        let json_str = serde_json::to_string(&circuit).unwrap();
        let deserialized: MockCircuit = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.public_inputs, large_inputs);
    }

    #[test]
    fn mock_proof_system_struct_clone() {
        let pk = MockProvingKey;
        let pk2 = pk.clone();
        let _ = format!("{pk2:?}");

        let vk = MockVerifyingKey;
        let vk2 = vk.clone();
        let _ = format!("{vk2:?}");
    }
}
