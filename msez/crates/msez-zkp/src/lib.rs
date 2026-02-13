//! # msez-zkp — Zero-Knowledge Proof System
//!
//! Provides a sealed, trait-based proof system abstraction that supports both
//! Phase 1 (deterministic mock) and Phase 2 (real ZK) implementations.
//!
//! ## Architecture
//!
//! The [`ProofSystem`] trait defines the interface for all proof backends.
//! It is **sealed** — only implementations authorized within this crate can
//! exist. This prevents unauthorized proof backends from being injected
//! into sovereign infrastructure.
//!
//! Phase 1 ships with [`MockProofSystem`] — transparent and deterministic,
//! matching the current Python behavior (`tools/phoenix/zkp.py`).
//! Phase 2 activates real backends (Groth16, PLONK) via Cargo feature flags.
//!
//! ## Circuit Types
//!
//! The spec defines 12 circuit types across 4 categories in the [`circuits`]
//! module:
//!
//! - **Compliance**: Balance sufficiency, sanctions clearance, tensor inclusion.
//! - **Migration**: Migration evidence, ownership chain, compensation validity.
//! - **Identity**: KYC attestation, attestation validity, threshold signature.
//! - **Settlement**: Range proof, Merkle membership, netting validity.
//!
//! ## Canonical Digest Bridge (CDB)
//!
//! The [`cdb`] module defines the CDB transformation:
//! `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.
//!
//! Phase 1 uses SHA256-only (Poseidon2 step is identity).
//! Phase 2 activates Poseidon2 for ZK-friendly hashing within arithmetic
//! circuits via the `poseidon2` feature flag.
//!
//! ## Feature Flags
//!
//! | Feature    | Description                          | Phase |
//! |------------|--------------------------------------|-------|
//! | `mock`     | Deterministic SHA-256 mock proofs    | 1     |
//! | `groth16`  | arkworks Groth16 SNARK backend       | 2     |
//! | `plonk`    | halo2 PLONK backend                  | 2     |
//! | `poseidon2`| Poseidon2 ZK-friendly hash in CDB    | 2     |
//!
//! ## Audit References
//!
//! - §2.2: Poseidon2 hash — specified but not implemented.
//! - §2.5: ZKP system — entirely mocked in Python.
//! - §5.6: Feature flags for phase gating.

pub mod cdb;
pub mod circuits;
pub mod mock;
pub mod traits;

#[cfg(feature = "groth16")]
pub mod groth16;

#[cfg(feature = "plonk")]
pub mod plonk;

// Re-export primary types for ergonomic imports.
pub use cdb::Cdb;
pub use mock::MockProofSystem;
pub use traits::{ProofError, ProofSystem, VerifyError};
