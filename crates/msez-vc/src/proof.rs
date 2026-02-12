//! # Proof Types
//!
//! Defines the proof structures for Verifiable Credentials.
//!
//! ## Proof Types Supported
//!
//! - Ed25519Signature2020 — Phase 1 (current).
//! - BBS+ — Phase 2 (selective disclosure).
//!
//! ## Security Invariant
//!
//! Proof array elements have rigid structure — `additionalProperties: false`
//! at the schema level. No arbitrary fields may be injected into proofs.
//!
//! ## Implements
//!
//! Spec §9 — VC proof structure.

use serde::{Deserialize, Serialize};

/// The type of cryptographic proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofType {
    /// Ed25519 digital signature proof.
    Ed25519Signature2020,
    /// BBS+ selective disclosure proof (Phase 2).
    BbsBlsSignature2020,
}

/// A cryptographic proof attached to a Verifiable Credential.
///
/// Placeholder — full implementation will include verification method,
/// proof purpose, creation timestamp, and the actual proof value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// The type of proof.
    #[serde(rename = "type")]
    pub proof_type: ProofType,
    /// The DID URL of the verification method used.
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,
}
