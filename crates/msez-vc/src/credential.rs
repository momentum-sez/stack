//! # Verifiable Credential Structure
//!
//! Defines the core VC envelope following W3C VC Data Model v2.0.
//!
//! ## Implements
//!
//! Spec §9 — Verifiable Credential structure and signing protocol.

use serde::{Deserialize, Serialize};

/// A W3C Verifiable Credential.
///
/// Placeholder — full implementation will include context URIs,
/// credential subject, issuer DID, issuance/expiration dates,
/// and proof array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// The VC type (e.g., "SmartAssetRegistryCredential").
    #[serde(rename = "type")]
    pub vc_type: Vec<String>,
    /// The issuer DID.
    pub issuer: String,
    /// The credential subject (extensible per W3C spec).
    #[serde(rename = "credentialSubject")]
    pub credential_subject: serde_json::Value,
}
