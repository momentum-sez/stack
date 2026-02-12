//! # msez-zkp — Zero-Knowledge Proof System
//!
//! Defines the trait-based proof system abstraction and Phase 1 mock
//! implementation for the SEZ Stack.
//!
//! ## Architecture
//!
//! - **Traits** (`traits.rs`): The `ProofSystem` trait defines the interface
//!   that all proof system implementations must satisfy. This is the compile-time
//!   contract that ensures mock and real implementations are interchangeable.
//!
//! - **Mock** (`mock.rs`): `MockProofSystem` provides deterministic, transparent
//!   "proofs" for Phase 1. Identical behavior to the current Python implementation
//!   (`tools/phoenix/zkp.py`) but with the trait interface already defined.
//!
//! - **CDB** (`cdb.rs`): Canonical Digest Bridge — `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.
//!   Phase 1 uses SHA256-only; Poseidon2 activates in Phase 2.
//!
//! ## Phase 2 Extensions (Feature-Gated)
//!
//! - `groth16`: Real Groth16 proofs via `ark-groth16` (Zcash, Aleo).
//! - `plonk`: Real PLONK proofs via `halo2_proofs` (Zcash Orchard, Scroll).
//! - `stark`: Real STARK proofs via `plonky2` (Polygon, StarkWare).
//! - `poseidon2`: ZK-friendly hashing via `poseidon2-rs`.
//!
//! ## 12 Circuit Types (Planned)
//!
//! - Compliance attestation circuits
//! - Migration evidence circuits
//! - Identity verification circuits
//! - Settlement proof circuits
//!
//! ## Crate Policy
//!
//! - Depends on `msez-core` and `msez-crypto` internally.
//! - Phase 2 crates are optional dependencies behind feature flags.
//! - No `unsafe` in mock implementation.

pub mod cdb;
pub mod mock;
pub mod traits;

pub use mock::MockProofSystem;
pub use traits::ProofSystem;
