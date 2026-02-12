//! # msez-vc — Verifiable Credentials
//!
//! Implements W3C Verifiable Credentials for the SEZ Stack, including:
//!
//! - **Credential** (`credential.rs`): VC structure, issuance, and verification
//!   following the W3C VC Data Model v2.0.
//!
//! - **Proof** (`proof.rs`): Proof types including Ed25519 cryptographic proofs
//!   and (Phase 2) BBS+ selective disclosure proofs.
//!
//! - **Registry** (`registry.rs`): Smart Asset Registry VCs — the credential
//!   type used to assert compliance evaluation results.
//!
//! ## Security Invariant
//!
//! All VC digests are computed from `CanonicalBytes` via `msez-crypto::sha256_digest()`.
//! Proof signing uses real Ed25519 — no mocking in production paths.
//!
//! ## Crate Policy
//!
//! - Depends on `msez-core` and `msez-crypto` internally.
//! - `credentialSubject` remains extensible per W3C VC spec.
//! - `proof` array elements have rigid structure (`additionalProperties: false`).

pub mod credential;
pub mod proof;
pub mod registry;

pub use credential::VerifiableCredential;
pub use proof::{Proof, ProofType};
pub use registry::SmartAssetRegistryVc;
