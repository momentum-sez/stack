//! # mez-crypto — Cryptographic Primitives for the EZ Stack
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
//! - **SHA-256 digest computation** from [`CanonicalBytes`](mez_core::CanonicalBytes),
//!   producing [`ContentDigest`](mez_core::ContentDigest) values.
//! - **HSM/KMS key management** abstraction supporting software, AWS KMS,
//!   and AWS CloudHSM backends.
//!
//! ## Phase 4 Extensions
//!
//! BBS+ selective disclosure and Poseidon2 hashing are available behind
//! Cargo feature flags. Enable `poseidon2` or `bbs-plus` to compile the
//! production implementations.

pub mod cas;
pub mod ed25519;
pub mod error;
pub mod hsm;
pub mod mmr;
pub mod sha256;

#[cfg(feature = "poseidon2")]
pub mod poseidon;

#[cfg(feature = "bbs-plus")]
pub mod bbs;

// Re-export primary types.
pub use cas::{ArtifactRef, ArtifactType, ContentAddressedStore};
pub use ed25519::{Ed25519Signature, SigningKey, VerifyingKey};
pub use error::CryptoError;
pub use hsm::{KeyAlgorithm, KeyMetadata, KeyProvider, ProviderType, SoftwareKeyProvider};
pub use mmr::MerkleMountainRange;
pub use sha256::sha256_digest;
