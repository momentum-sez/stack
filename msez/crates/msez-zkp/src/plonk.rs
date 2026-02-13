//! # PLONK Proof System (Phase 2 — Feature-Gated)
//!
//! Stub implementation of the PLONK proof system. This module is gated
//! behind the `plonk` Cargo feature flag and will integrate with
//! `halo2_proofs` when the dependency is available.
//!
//! ## Properties
//!
//! - **Proof size:** ~500 bytes (larger than Groth16).
//! - **Verification time:** O(log n) in circuit size.
//! - **Trusted setup:** Universal (not circuit-specific) — one ceremony
//!   supports all circuits up to a maximum size.
//! - **Used by:** Zcash (Orchard), Scroll, Aztec.
//!
//! ## Phase 2 Integration Plan
//!
//! 1. Add `halo2_proofs` to workspace dependencies.
//! 2. Implement `ProofSystem` trait using halo2 types.
//! 3. Implement circuit compilation from `circuits/` data models to
//!    halo2 PLONKish constraint system.
//! 4. Generate universal SRS parameters.
//!
//! ## Spec Reference
//!
//! Audit §2.5: PLONK is listed as one of the 5 NIZK systems in the spec.
//! Audit §5.6: Feature-gated behind `plonk` Cargo feature.

use serde::{Deserialize, Serialize};

use crate::traits::{ProofError, ProofSystem, VerifyError};

/// PLONK proof artifact.
///
/// In Phase 2, this will wrap `halo2_proofs::plonk::Proof`.
/// Currently a placeholder type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlonkProof {
    /// Serialized proof bytes (Phase 2: polynomial commitment openings).
    pub proof_bytes: Vec<u8>,
}

/// PLONK verifying key.
///
/// In Phase 2, this will wrap `halo2_proofs::plonk::VerifyingKey`.
#[derive(Debug, Clone)]
pub struct PlonkVerifyingKey {
    /// Serialized verifying key bytes.
    pub key_bytes: Vec<u8>,
}

/// PLONK proving key.
///
/// In Phase 2, this will wrap `halo2_proofs::plonk::ProvingKey`.
#[derive(Debug)]
pub struct PlonkProvingKey {
    /// Serialized proving key bytes.
    pub key_bytes: Vec<u8>,
}

/// PLONK circuit representation.
///
/// In Phase 2, this will wrap a halo2 `Circuit` implementor.
#[derive(Debug, Clone)]
pub struct PlonkCircuit {
    /// Circuit identifier for registry lookup.
    pub circuit_id: String,
    /// Number of PLONKish gates.
    pub gate_count: usize,
    /// Public input bytes.
    pub public_inputs: Vec<u8>,
}

/// PLONK proof system implementation.
///
/// Phase 2: Integrates with `halo2_proofs` for real PLONK proof
/// generation and verification with universal trusted setup.
pub struct PlonkProofSystem;

impl ProofSystem for PlonkProofSystem {
    type Proof = PlonkProof;
    type VerifyingKey = PlonkVerifyingKey;
    type ProvingKey = PlonkProvingKey;
    type Circuit = PlonkCircuit;

    fn prove(
        &self,
        _pk: &Self::ProvingKey,
        _circuit: &Self::Circuit,
    ) -> Result<Self::Proof, ProofError> {
        unimplemented!(
            "PLONK proof generation requires the `halo2_proofs` dependency. \
             This is Phase 2 work. Use `MockProofSystem` for Phase 1."
        )
    }

    fn verify(
        &self,
        _vk: &Self::VerifyingKey,
        _proof: &Self::Proof,
        _public_inputs: &[u8],
    ) -> Result<bool, VerifyError> {
        unimplemented!(
            "PLONK proof verification requires the `halo2_proofs` dependency. \
             This is Phase 2 work. Use `MockProofSystem` for Phase 1."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plonk_proof_serialization_roundtrip() {
        let proof = PlonkProof {
            proof_bytes: vec![0xca, 0xfe, 0xba, 0xbe],
        };
        let json = serde_json::to_string(&proof).unwrap();
        let deser: PlonkProof = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.proof_bytes, vec![0xca, 0xfe, 0xba, 0xbe]);
    }

    #[test]
    fn plonk_verifying_key_is_cloneable() {
        let vk = PlonkVerifyingKey {
            key_bytes: vec![4, 5, 6],
        };
        let vk2 = vk.clone();
        assert_eq!(vk.key_bytes, vk2.key_bytes);
    }

    #[test]
    fn plonk_circuit_is_cloneable() {
        let circuit = PlonkCircuit {
            circuit_id: "plonk-test".to_string(),
            gate_count: 2048,
            public_inputs: vec![0; 16],
        };
        let c2 = circuit.clone();
        assert_eq!(c2.circuit_id, "plonk-test");
        assert_eq!(c2.gate_count, 2048);
    }

    #[test]
    #[should_panic(expected = "Phase 2")]
    fn plonk_prove_panics_with_phase2_message() {
        let sys = PlonkProofSystem;
        let pk = PlonkProvingKey { key_bytes: vec![] };
        let circuit = PlonkCircuit {
            circuit_id: "test".to_string(),
            gate_count: 0,
            public_inputs: vec![],
        };
        let _ = sys.prove(&pk, &circuit);
    }

    #[test]
    #[should_panic(expected = "Phase 2")]
    fn plonk_verify_panics_with_phase2_message() {
        let sys = PlonkProofSystem;
        let vk = PlonkVerifyingKey { key_bytes: vec![] };
        let proof = PlonkProof {
            proof_bytes: vec![],
        };
        let _ = sys.verify(&vk, &proof, &[]);
    }

    #[test]
    fn plonk_types_are_debug() {
        let proof = PlonkProof {
            proof_bytes: vec![],
        };
        let vk = PlonkVerifyingKey { key_bytes: vec![] };
        let pk = PlonkProvingKey { key_bytes: vec![] };
        let circuit = PlonkCircuit {
            circuit_id: "x".to_string(),
            gate_count: 0,
            public_inputs: vec![],
        };
        assert!(!format!("{proof:?}").is_empty());
        assert!(!format!("{vk:?}").is_empty());
        assert!(!format!("{pk:?}").is_empty());
        assert!(!format!("{circuit:?}").is_empty());
    }
}
