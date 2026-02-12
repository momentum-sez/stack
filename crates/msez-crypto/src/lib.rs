//! # msez-crypto — Cryptographic Primitives
//!
//! Provides the cryptographic building blocks for the SEZ Stack:
//!
//! - **Ed25519** signing and verification for Verifiable Credential proofs.
//!   Signing input MUST be `&CanonicalBytes` — you cannot sign raw bytes.
//! - **SHA-256** digest computation from `CanonicalBytes` (the only valid
//!   input type, enforcing canonicalization correctness).
//! - **Merkle Mountain Range (MMR)** for append-only corridor receipt chains.
//!   Ported from `tools/mmr.py` with cross-language compatibility verified
//!   against Python-generated fixtures.
//! - **Content-Addressed Storage (CAS)** for artifact store/resolve operations,
//!   matching the `dist/artifacts/` layout in `tools/artifacts.py`.
//!
//! ## Phase 4 Extensions (Feature-Gated)
//!
//! - `poseidon2` — Poseidon2 ZK-friendly hashing (stub, `unimplemented!()`).
//! - `bbs-plus` — BBS+ selective disclosure signatures (stub, `unimplemented!()`).
//!
//! ## Crate Policy
//!
//! - Depends only on `msez-core` internally.
//! - No mocking of cryptographic operations in tests — all tests use real
//!   `CanonicalBytes`, real SHA-256, real Ed25519.
//! - `unsafe` prohibited without `// SAFETY:` justification.
//! - No raw `serde_json` serialization for digest computation — all digest
//!   paths flow through `CanonicalBytes::new()`.

pub mod cas;
pub mod ed25519;
pub mod mmr;
pub mod sha256;

#[cfg(feature = "poseidon2")]
pub mod poseidon;

#[cfg(feature = "bbs-plus")]
pub mod bbs;

pub use cas::CasStore;
pub use ed25519::{Ed25519KeyPair, Ed25519PublicKey, Ed25519Signature};
pub use mmr::MerkleMountainRange;
pub use sha256::sha256_digest;
