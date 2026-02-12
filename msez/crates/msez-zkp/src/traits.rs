//! # Proof System Trait (Sealed)
//!
//! The core abstraction for zero-knowledge proof systems. All backends
//! (mock, Groth16, PLONK) implement this trait.
//!
//! ## Sealed Trait
//!
//! The `ProofSystem` trait is **sealed**: only implementations defined within
//! the `msez-zkp` crate can exist. External crates cannot implement it. This
//! prevents unauthorized proof backends from being injected into the system,
//! which is a security requirement for sovereign infrastructure.
//!
//! ## Spec Reference
//!
//! Audit §2.5: All proof generation in Python used `secrets.token_hex(32)`.
//! This trait defines the compile-time contract that any real or mock
//! implementation must satisfy.

use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

/// Error during proof generation.
///
/// Returned by [`ProofSystem::prove`] when proof generation cannot proceed.
#[derive(Error, Debug)]
pub enum ProofError {
    /// The circuit inputs are invalid or missing.
    #[error("invalid circuit inputs: {0}")]
    InvalidInputs(String),
    /// Proof generation failed internally.
    #[error("proof generation failed: {0}")]
    GenerationFailed(String),
}

/// Error during proof verification.
///
/// Returned by [`ProofSystem::verify`] when verification cannot proceed or
/// when the proof is cryptographically invalid.
#[derive(Error, Debug)]
pub enum VerifyError {
    /// The proof is structurally malformed.
    #[error("malformed proof: {0}")]
    MalformedProof(String),
    /// The proof is cryptographically invalid.
    #[error("proof verification failed: {0}")]
    VerificationFailed(String),
}

/// Private module that seals the [`ProofSystem`] trait.
///
/// Only types within `msez-zkp` that implement `private::Sealed` can
/// implement `ProofSystem`. This prevents external crates from creating
/// unauthorized proof backends.
mod private {
    /// Sealing marker trait. Not accessible outside `msez-zkp`.
    pub trait Sealed {}
}

/// Sealed trait defining the interface for a zero-knowledge proof system.
///
/// Each implementation provides its own proof, key, and circuit types via
/// associated types. The trait is sealed — only implementations authorized
/// within `msez-zkp` can exist.
///
/// The trait requires `Send + Sync` to support concurrent proof generation
/// and verification in the API layer.
///
/// ## Associated Types
///
/// - **`Proof`**: The proof artifact produced by `prove()`.
/// - **`VerifyingKey`**: The key used to verify proofs. Must be cloneable
///   for distribution to verifiers.
/// - **`ProvingKey`**: The key used to generate proofs. May be large and
///   expensive to clone.
/// - **`Circuit`**: The circuit definition that constrains what the proof
///   demonstrates.
///
/// ## Phase 1
///
/// [`MockProofSystem`](crate::mock::MockProofSystem) provides deterministic,
/// transparent proofs using SHA-256. No zero-knowledge guarantees.
///
/// ## Phase 2
///
/// Real implementations (Groth16, PLONK) are feature-gated and provide
/// actual zero-knowledge guarantees via arkworks and halo2 respectively.
pub trait ProofSystem: private::Sealed + Send + Sync {
    /// The proof type produced by this system.
    type Proof: Serialize + DeserializeOwned + Clone + std::fmt::Debug;
    /// The verifying key type.
    type VerifyingKey: Clone;
    /// The proving key type.
    type ProvingKey;
    /// The circuit type that defines the proof statement.
    type Circuit: Clone;

    /// Generate a proof that the prover knows a valid witness satisfying
    /// the circuit constraints.
    ///
    /// # Arguments
    ///
    /// * `pk` — The proving key for the target circuit.
    /// * `circuit` — The circuit definition with public inputs and witness data.
    ///
    /// # Errors
    ///
    /// Returns [`ProofError::InvalidInputs`] if the circuit data is malformed.
    /// Returns [`ProofError::GenerationFailed`] if proof generation fails.
    fn prove(
        &self,
        pk: &Self::ProvingKey,
        circuit: &Self::Circuit,
    ) -> Result<Self::Proof, ProofError>;

    /// Verify a proof against public inputs.
    ///
    /// # Arguments
    ///
    /// * `vk` — The verifying key for the target circuit.
    /// * `proof` — The proof to verify.
    /// * `public_inputs` — The public inputs that the proof claims to satisfy.
    ///
    /// # Returns
    ///
    /// `Ok(true)` if the proof is valid, `Ok(false)` if the proof is
    /// cryptographically invalid but structurally well-formed.
    ///
    /// # Errors
    ///
    /// Returns [`VerifyError::MalformedProof`] if the proof is structurally
    /// invalid (wrong length, corrupt encoding).
    fn verify(
        &self,
        vk: &Self::VerifyingKey,
        proof: &Self::Proof,
        public_inputs: &[u8],
    ) -> Result<bool, VerifyError>;
}

// ---- Sealed trait implementations for authorized proof systems ----

impl private::Sealed for crate::mock::MockProofSystem {}

#[cfg(feature = "groth16")]
impl private::Sealed for crate::groth16::Groth16ProofSystem {}

#[cfg(feature = "plonk")]
impl private::Sealed for crate::plonk::PlonkProofSystem {}
