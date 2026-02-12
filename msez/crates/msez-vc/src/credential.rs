//! # Verifiable Credential structure
//!
//! Defines the core [`VerifiableCredential`] type following the W3C VC
//! Data Model, adapted for SEZ Stack conventions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::proof::Proof;

/// A W3C Verifiable Credential with SEZ Stack extensions.
///
/// The envelope structure is rigid (`additionalProperties: false` at the
/// schema level), while `credential_subject` is intentionally extensible
/// per the W3C specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// The JSON-LD context URIs.
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// The credential identifier (DID or URI).
    pub id: String,
    /// The credential type(s).
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// The DID of the credential issuer.
    pub issuer: String,
    /// When the credential was issued (UTC).
    pub issuance_date: DateTime<Utc>,
    /// Optional expiration date (UTC).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<DateTime<Utc>>,
    /// The credential subject — intentionally extensible.
    pub credential_subject: serde_json::Value,
    /// Cryptographic proofs attached to this credential.
    #[serde(default)]
    pub proof: Vec<Proof>,
}
