//! # msez-crypto — Cryptographic Primitives
//!
//! Provides the cryptographic building blocks for the SEZ Stack:
//!
//! - **Ed25519** signing and verification for Verifiable Credential proofs.
//! - **SHA-256** digest computation from `CanonicalBytes` (the only valid
//!   input type, enforcing canonicalization correctness).
//! - **Merkle Mountain Range (MMR)** for append-only corridor receipt chains.
//! - **Content-Addressed Storage (CAS)** for artifact store/resolve operations.
//!
//! ## Phase 2 Extensions
//!
//! - BBS+ selective disclosure signatures (feature-gated).
//! - Poseidon2 ZK-friendly hashing (feature-gated).
//!
//! ## Crate Policy
//!
//! - Depends only on `msez-core` internally.
//! - No mocking of cryptographic operations in tests — all tests use real
//!   `CanonicalBytes`, real SHA-256, real Ed25519.
//! - `unsafe` prohibited without `// SAFETY:` justification.

pub mod cas;
pub mod ed25519;
pub mod mmr;
pub mod sha256;

pub use cas::CasStore;
pub use ed25519::{Ed25519KeyPair, Ed25519PublicKey, Ed25519Signature};
pub use mmr::MerkleMountainRange;
pub use sha256::sha256_digest;
