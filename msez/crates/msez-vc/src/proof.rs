//! # Proof types for Verifiable Credentials
//!
//! Defines the cryptographic proof structure attached to VCs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The type of cryptographic proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    /// Ed25519 signature (Phase 1).
    Ed25519Signature2020,
    /// BBS+ selective disclosure (Phase 2).
    BbsBlsSignature2020,
}

/// The purpose of the proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProofPurpose {
    /// The issuer asserts the credential claims.
    AssertionMethod,
    /// Authentication of the holder.
    Authentication,
}

/// A cryptographic proof on a Verifiable Credential.
///
/// The proof structure is rigid — `additionalProperties: false` at the
/// schema level prevents injection of unexpected fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proof {
    /// The proof type (Ed25519, BBS+).
    #[serde(rename = "type")]
    pub proof_type: ProofType,
    /// When the proof was created.
    pub created: DateTime<Utc>,
    /// The verification method (DID URL of the signing key).
    pub verification_method: String,
    /// The purpose of this proof.
    pub proof_purpose: ProofPurpose,
    /// The proof value (base64-encoded signature bytes).
    pub proof_value: String,
}
