//! # Proof System Trait
//!
//! The core abstraction for zero-knowledge proof systems. All backends
//! (mock, Groth16, PLONK, STARK) implement this trait.

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Error during proof generation.
#[derive(Error, Debug)]
pub enum ProofError {
    /// The circuit inputs are invalid.
    #[error("invalid circuit inputs: {0}")]
    InvalidInputs(String),
    /// Proof generation failed internally.
    #[error("proof generation failed: {0}")]
    GenerationFailed(String),
}

/// Error during proof verification.
#[derive(Error, Debug)]
pub enum VerifyError {
    /// The proof is malformed.
    #[error("malformed proof: {0}")]
    MalformedProof(String),
    /// The proof is cryptographically invalid.
    #[error("proof verification failed: {0}")]
    VerificationFailed(String),
}

/// Trait defining the interface for a zero-knowledge proof system.
///
/// Each implementation provides its own proof, key, and circuit types.
/// The trait is `Send + Sync` to support concurrent proof generation
/// in the API layer.
///
/// ## Phase 1
///
/// [`MockProofSystem`](crate::MockProofSystem) provides deterministic,
/// transparent proofs for development and testing.
///
/// ## Phase 2
///
/// Real implementations (Groth16, PLONK, STARK) are feature-gated and
/// provide actual zero-knowledge guarantees.
pub trait ProofSystem: Send + Sync {
    /// The proof type produced by this system.
    type Proof: Serialize + DeserializeOwned + Clone + std::fmt::Debug;
    /// The verifying key type.
    type VerifyingKey: Clone;
    /// The proving key type.
    type ProvingKey;

    /// Generate a proof for the given public inputs.
    fn prove(
        &self,
        pk: &Self::ProvingKey,
        public_inputs: &[u8],
    ) -> Result<Self::Proof, ProofError>;

    /// Verify a proof against public inputs.
    fn verify(
        &self,
        vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_inputs: &[u8],
    ) -> Result<bool, VerifyError>;
}
