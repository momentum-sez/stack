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
        unimplemented!(
            "Groth16 proof generation requires the `ark-groth16` dependency. \
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
            "Groth16 proof verification requires the `ark-groth16` dependency. \
             This is Phase 2 work. Use `MockProofSystem` for Phase 1."
        )
    }
}
