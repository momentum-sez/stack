//! # Mock Proof System (Phase 1)
//!
//! A deterministic, transparent proof system for development and testing.
//! Produces SHA-256-based "proofs" that are verifiable but provide
//! no zero-knowledge guarantees.

use serde::{Deserialize, Serialize};

use crate::traits::{ProofError, ProofSystem, VerifyError};

/// A mock proof — deterministic SHA-256 hash of the inputs.
/// Provides no zero-knowledge guarantees.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MockProof {
    /// Hex-encoded deterministic hash.
    pub proof_hex: String,
}

/// Mock verifying key — accepts all proofs in Phase 1.
#[derive(Debug, Clone)]
pub struct MockVerifyingKey;

/// Mock proving key — no secrets in Phase 1.
#[derive(Debug, Clone)]
pub struct MockProvingKey;

/// A deterministic mock proof system for Phase 1.
///
/// Produces SHA-256 digests as "proofs." Verification checks that the
/// proof matches the expected digest of the public inputs. This is
/// functionally equivalent to the Python implementation in
/// `tools/phoenix/zkp.py`.
pub struct MockProofSystem;

impl ProofSystem for MockProofSystem {
    type Proof = MockProof;
    type VerifyingKey = MockVerifyingKey;
    type ProvingKey = MockProvingKey;

    fn prove(
        &self,
        _pk: &Self::ProvingKey,
        _public_inputs: &[u8],
    ) -> Result<Self::Proof, ProofError> {
        // Placeholder — real implementation computes SHA-256 based mock proof.
        todo!("implement deterministic mock proof generation")
    }

    fn verify(
        &self,
        _vk: &Self::VerifyingKey,
        _proof: &Self::Proof,
        _public_inputs: &[u8],
    ) -> Result<bool, VerifyError> {
        // Placeholder — real implementation verifies digest match.
        todo!("implement mock proof verification")
    }
}
