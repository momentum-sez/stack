//! # msez-crypto — Cryptographic Primitives for the SEZ Stack
//!
//! This crate provides the cryptographic building blocks used throughout
//! the workspace:
//!
//! - **Ed25519** signing and verification for Verifiable Credentials and
//!   corridor attestations.
//! - **Merkle Mountain Range (MMR)** for append-only receipt chains with
//!   efficient inclusion proofs.
//! - **Content-Addressed Storage (CAS)** utilities for storing and resolving
//!   artifacts by their content digest.
//! - **SHA-256 digest computation** from [`CanonicalBytes`](msez_core::CanonicalBytes),
//!   producing [`ContentDigest`](msez_core::ContentDigest) values.
//!
//! ## Phase 2 Extensions
//!
//! BBS+ selective disclosure and Poseidon2 hashing will be added behind
//! Cargo feature flags when the ZK proof system activates.

pub mod cas;
pub mod ed25519;
pub mod mmr;
pub mod sha256;

// Re-export primary types.
pub use cas::ContentAddressedStore;
pub use ed25519::{Ed25519Signature, SigningKey, VerifyingKey};
pub use mmr::MerkleMountainRange;
pub use sha256::sha256_digest;
