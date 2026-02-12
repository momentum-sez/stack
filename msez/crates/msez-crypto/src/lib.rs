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
//! ## Phase 4 Extensions
//!
//! BBS+ selective disclosure and Poseidon2 hashing are available as stub
//! modules behind Cargo feature flags. Enable `poseidon2` or `bbs-plus`
//! to compile the type signatures; concrete implementations will land
//! when the ZK proof system activates in Phase 4.

pub mod cas;
pub mod ed25519;
pub mod error;
pub mod mmr;
pub mod sha256;

#[cfg(feature = "poseidon2")]
pub mod poseidon;

#[cfg(feature = "bbs-plus")]
pub mod bbs;

// Re-export primary types.
pub use cas::{ArtifactRef, ContentAddressedStore};
pub use ed25519::{Ed25519Signature, SigningKey, VerifyingKey};
pub use error::CryptoError;
pub use mmr::MerkleMountainRange;
pub use sha256::sha256_digest;
