//! # Groth16 Proof System (Phase 2 — Feature-Gated)
//!
//! Stub implementation of the Groth16 SNARK proof system. This module is
//! gated behind the `groth16` Cargo feature flag and will integrate with
//! `ark-groth16` (arkworks) when the dependency is available.
//!
//! ## Properties
//!
//! - **Proof size:** ~200 bytes (constant, independent of circuit size).
//! - **Verification time:** Constant (3 pairing checks).
//! - **Trusted setup:** Required (circuit-specific).
//! - **Used by:** Zcash (Sapling), Aleo, Filecoin.
//!
//! ## Phase 2 Integration Plan
//!
//! 1. Add `ark-groth16` and `ark-bn254` to workspace dependencies.
//! 2. Implement `ProofSystem` trait using arkworks types.
//! 3. Implement circuit compilation from `circuits/` data models to
//!    arkworks R1CS constraints.
//! 4. Add trusted setup ceremony tooling.
//!
//! ## Spec Reference
//!
//! Audit §2.5: Groth16 is listed as one of the 5 NIZK systems in the spec.
//! Audit §5.6: Feature-gated behind `groth16` Cargo feature.

use serde::{Deserialize, Serialize};

use crate::traits::{ProofError, ProofSystem, VerifyError};

/// Groth16 proof artifact.
///
/// In Phase 2, this will wrap `ark_groth16::Proof<Bn254>`.
/// Currently a placeholder type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Groth16Proof {
    /// Serialized proof bytes (Phase 2: BN254 curve points).
    pub proof_bytes: Vec<u8>,
}

/// Groth16 verifying key.
///
/// In Phase 2, this will wrap `ark_groth16::VerifyingKey<Bn254>`.
#[derive(Debug, Clone)]
pub struct Groth16VerifyingKey {
    /// Serialized verifying key bytes.
    pub key_bytes: Vec<u8>,
}

/// Groth16 proving key.
///
/// In Phase 2, this will wrap `ark_groth16::ProvingKey<Bn254>`.
#[derive(Debug)]
pub struct Groth16ProvingKey {
    /// Serialized proving key bytes.
    pub key_bytes: Vec<u8>,
}

/// Groth16 circuit representation.
///
/// In Phase 2, this will wrap an arkworks `ConstraintSynthesizer`.
#[derive(Debug, Clone)]
pub struct Groth16Circuit {
    /// Circuit identifier for registry lookup.
    pub circuit_id: String,
    /// Number of R1CS constraints.
    pub constraint_count: usize,
    /// Public input bytes.
    pub public_inputs: Vec<u8>,
}

/// Groth16 proof system implementation.
///
/// Phase 2: Integrates with `ark-groth16` for real SNARK proof
/// generation and verification on the BN254 curve.
pub struct Groth16ProofSystem;

impl ProofSystem for Groth16ProofSystem {
    type Proof = Groth16Proof;
    type VerifyingKey = Groth16VerifyingKey;
    type ProvingKey = Groth16ProvingKey;
    type Circuit = Groth16Circuit;

    fn prove(
        &self,
        _pk: &Self::ProvingKey,
        _circuit: &Self::Circuit,
    ) -> Result<Self::Proof, ProofError> {
        Err(ProofError::NotImplemented(
            "Groth16 proof generation available in Phase 2".into(),
        ))
    }

    fn verify(
        &self,
        _vk: &Self::VerifyingKey,
        _proof: &Self::Proof,
        _public_inputs: &[u8],
    ) -> Result<bool, VerifyError> {
        Err(VerifyError::NotImplemented(
            "Groth16 proof verification available in Phase 2".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groth16_proof_serialization_roundtrip() {
        let proof = Groth16Proof {
            proof_bytes: vec![0xde, 0xad, 0xbe, 0xef],
        };
        let json = serde_json::to_string(&proof).unwrap();
        let deser: Groth16Proof = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.proof_bytes, vec![0xde, 0xad, 0xbe, 0xef]);
    }

    #[test]
    fn groth16_verifying_key_is_cloneable() {
        let vk = Groth16VerifyingKey {
            key_bytes: vec![1, 2, 3],
        };
        let vk2 = vk.clone();
        assert_eq!(vk.key_bytes, vk2.key_bytes);
    }

    #[test]
    fn groth16_circuit_is_cloneable() {
        let circuit = Groth16Circuit {
            circuit_id: "test-circuit".to_string(),
            constraint_count: 1024,
            public_inputs: vec![0; 32],
        };
        let c2 = circuit.clone();
        assert_eq!(c2.circuit_id, "test-circuit");
        assert_eq!(c2.constraint_count, 1024);
    }

    #[test]
    fn groth16_prove_returns_not_implemented() {
        let sys = Groth16ProofSystem;
        let pk = Groth16ProvingKey { key_bytes: vec![] };
        let circuit = Groth16Circuit {
            circuit_id: "test".to_string(),
            constraint_count: 0,
            public_inputs: vec![],
        };
        let result = sys.prove(&pk, &circuit);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }

    #[test]
    fn groth16_verify_returns_not_implemented() {
        let sys = Groth16ProofSystem;
        let vk = Groth16VerifyingKey { key_bytes: vec![] };
        let proof = Groth16Proof {
            proof_bytes: vec![],
        };
        let result = sys.verify(&vk, &proof, &[]);
        assert!(result.is_err());
        assert!(format!("{}", result.unwrap_err()).contains("not implemented"));
    }

    #[test]
    fn groth16_types_are_debug() {
        let proof = Groth16Proof {
            proof_bytes: vec![],
        };
        let vk = Groth16VerifyingKey { key_bytes: vec![] };
        let pk = Groth16ProvingKey { key_bytes: vec![] };
        let circuit = Groth16Circuit {
            circuit_id: "x".to_string(),
            constraint_count: 0,
            public_inputs: vec![],
        };
        assert!(!format!("{proof:?}").is_empty());
        assert!(!format!("{vk:?}").is_empty());
        assert!(!format!("{pk:?}").is_empty());
        assert!(!format!("{circuit:?}").is_empty());
    }
}
