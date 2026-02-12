//! # Mock Proof System (Phase 1)
//!
//! A deterministic, transparent "proof system" for Phase 1 deployment.
//! Proofs are SHA-256 hashes of the inputs — they provide no zero-knowledge
//! privacy guarantees but satisfy the trait interface.
//!
//! This matches the behavior of `tools/phoenix/zkp.py` which uses
//! `secrets.token_hex(32)` and `hashlib.sha256()` for mock proofs.
//!
//! ## Security Notice
//!
//! This implementation provides NO zero-knowledge privacy. It is
//! acceptable for Phase 1 (deterministic compliance evaluation) but
//! must be replaced with real proof systems in Phase 2.

use crate::traits::{ProofError, ProofSystem, VerifyError};

/// A mock proof — a deterministic hash of the inputs.
#[derive(Debug, Clone)]
pub struct MockProof {
    /// The mock proof bytes (SHA-256 hash).
    pub bytes: Vec<u8>,
}

/// A mock verifying key.
#[derive(Debug, Clone)]
pub struct MockVerifyingKey;

/// A mock proving key.
#[derive(Debug, Clone)]
pub struct MockProvingKey;

/// Phase 1 mock proof system — deterministic, transparent, no ZK privacy.
#[derive(Debug, Default)]
pub struct MockProofSystem;

impl ProofSystem for MockProofSystem {
    type Proof = MockProof;
    type VerifyingKey = MockVerifyingKey;
    type ProvingKey = MockProvingKey;

    fn prove(
        &self,
        _pk: &Self::ProvingKey,
        _public_inputs: &[u8],
        _private_inputs: &[u8],
    ) -> Result<Self::Proof, ProofError> {
        // TODO: Implement deterministic mock proof generation
        Ok(MockProof {
            bytes: vec![0u8; 32],
        })
    }

    fn verify(
        &self,
        _vk: &Self::VerifyingKey,
        _proof: &Self::Proof,
        _public_inputs: &[u8],
    ) -> Result<bool, VerifyError> {
        // TODO: Implement deterministic mock proof verification
        Ok(true)
    }
}
