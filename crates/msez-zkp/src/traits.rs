//! # Proof System Trait
//!
//! Defines the abstract interface for zero-knowledge proof systems.
//! All implementations (mock, Groth16, PLONK, STARK) must satisfy
//! this trait.
//!
//! ## Security Invariant
//!
//! The trait requires `Send + Sync` bounds for safe concurrent access.
//! Proof generation and verification are pure functions with no side effects.
//!
//! ## Implements
//!
//! Spec §18 — Zero-knowledge proof system interface.

use thiserror::Error;

/// Error during proof generation.
#[derive(Error, Debug)]
pub enum ProofError {
    /// The circuit is malformed or unsatisfiable.
    #[error("circuit error: {0}")]
    CircuitError(String),
    /// Witness generation failed.
    #[error("witness error: {0}")]
    WitnessError(String),
    /// Internal prover error.
    #[error("prover error: {0}")]
    ProverError(String),
}

/// Error during proof verification.
#[derive(Error, Debug)]
pub enum VerifyError {
    /// The proof is invalid.
    #[error("invalid proof: {0}")]
    InvalidProof(String),
    /// The verifying key is incompatible.
    #[error("key mismatch: {0}")]
    KeyMismatch(String),
}

/// Abstract interface for a zero-knowledge proof system.
///
/// Each implementation provides its own proof, key, and circuit types.
/// The trait ensures that mock and real implementations are interchangeable
/// at compile time.
pub trait ProofSystem: Send + Sync {
    /// The proof type produced by this system.
    type Proof: Send + Sync;
    /// The verifying key type.
    type VerifyingKey: Clone + Send + Sync;
    /// The proving key type.
    type ProvingKey: Send + Sync;

    /// Generate a proof.
    fn prove(
        &self,
        pk: &Self::ProvingKey,
        public_inputs: &[u8],
        private_inputs: &[u8],
    ) -> Result<Self::Proof, ProofError>;

    /// Verify a proof.
    fn verify(
        &self,
        vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_inputs: &[u8],
    ) -> Result<bool, VerifyError>;
}
