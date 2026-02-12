//! # msez-zkp — Zero-Knowledge Proof System
//!
//! Provides a trait-based proof system abstraction that supports both
//! Phase 1 (deterministic mock) and Phase 2 (real ZK) implementations.
//!
//! ## Architecture
//!
//! The [`ProofSystem`] trait defines the interface for all proof backends.
//! Phase 1 ships with [`MockProofSystem`] — transparent and deterministic,
//! identical to the current Python behavior. Phase 2 activates real
//! backends (Groth16, PLONK, STARK) via Cargo feature flags.
//!
//! ## Circuit Types
//!
//! The spec defines 12 circuit types across 4 categories:
//! - Compliance attestation circuits
//! - Migration evidence circuits
//! - Identity verification circuits
//! - Settlement proof circuits
//!
//! ## Canonical Digest Bridge (CDB)
//!
//! The CDB is defined as `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.
//! Phase 1 uses SHA256-only. Phase 2 activates Poseidon2 for ZK-friendly
//! hashing within arithmetic circuits.
//!
//! ## Audit Reference
//!
//! Finding §2.5: All proof generation in Python used `secrets.token_hex(32)`.
//! The Rust trait interface ensures any real implementation must satisfy
//! the `ProofSystem` contract at compile time.

pub mod cdb;
pub mod mock;
pub mod traits;

// Re-export primary types.
pub use mock::MockProofSystem;
pub use traits::{ProofError, ProofSystem, VerifyError};
