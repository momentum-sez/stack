//! # Proof types for Verifiable Credentials
//!
//! Defines the cryptographic proof structure attached to VCs. The proof object
//! has rigid structure (`additionalProperties: false` at the schema level) to
//! prevent injection attacks via unexpected fields.
//!
//! ## Supported Proof Types
//!
//! - **Ed25519Signature2020** — Phase 1. Ed25519 digital signatures over
//!   JCS-canonicalized credential bodies.
//! - **MezEd25519Signature2025** — Phase 1. MEZ-specific Ed25519 proof type
//!   for interoperability with the Python `tools/vc.py` layer.
//! - **BbsBlsSignature2020** — Phase 2, behind the `bbs-plus` feature flag
//!   in `mez-crypto`. Enables selective disclosure of credential claims
//!   per audit §3.4.
//!
//! ## Spec Reference
//!
//! Implements the proof profile from `tools/vc.py` lines 180-260.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use mez_core::Timestamp;

/// The type of cryptographic proof attached to a VC.
///
/// Each variant corresponds to a specific signature scheme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofType {
    /// Ed25519 digital signature per W3C VC Data Integrity spec (Phase 1).
    Ed25519Signature2020,

    /// Ed25519 digital signature using the MEZ-specific proof type name.
    /// Matches the Python `tools/vc.py` `_PROOF_TYPE` constant for
    /// cross-layer compatibility.
    MezEd25519Signature2025,

    /// BBS+ selective disclosure signature (Phase 2).
    /// Requires the `bbs-plus` feature flag in `mez-crypto`.
    BbsBlsSignature2020,
}

impl ProofType {
    /// Returns `true` if this is an Ed25519-based proof type.
    pub fn is_ed25519(&self) -> bool {
        matches!(
            self,
            ProofType::Ed25519Signature2020 | ProofType::MezEd25519Signature2025
        )
    }
}

impl std::fmt::Display for ProofType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofType::Ed25519Signature2020 => write!(f, "Ed25519Signature2020"),
            ProofType::MezEd25519Signature2025 => write!(f, "MezEd25519Signature2025"),
            ProofType::BbsBlsSignature2020 => write!(f, "BbsBlsSignature2020"),
        }
    }
}

/// The purpose of a cryptographic proof.
///
/// Follows W3C VC Data Integrity specification proof purpose vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProofPurpose {
    /// The issuer asserts the credential claims are true.
    AssertionMethod,
    /// Authentication of the credential holder.
    Authentication,
}

impl std::fmt::Display for ProofPurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofPurpose::AssertionMethod => write!(f, "assertionMethod"),
            ProofPurpose::Authentication => write!(f, "authentication"),
        }
    }
}

/// A cryptographic proof on a Verifiable Credential.
///
/// The proof structure is rigid — `additionalProperties: false` at the
/// schema level prevents injection of unexpected fields.
///
/// ## Security Invariant
///
/// The `proof_value` contains hex-encoded signature bytes computed over the
/// JCS-canonicalized credential body (with `proof` field excluded). The
/// canonicalization MUST use [`CanonicalBytes::new()`](mez_core::CanonicalBytes)
/// — never raw `serde_json::to_vec()`.
///
/// ## Spec Reference
///
/// Implements the proof object from `tools/vc.py:add_ed25519_proof()` and
/// `tools/vc.py:_validate_proof_object()`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Proof {
    /// The proof type (Ed25519, BBS+).
    #[serde(rename = "type")]
    pub proof_type: ProofType,

    /// When the proof was created (UTC, truncated to seconds).
    pub created: DateTime<Utc>,

    /// The verification method — a DID URL identifying the signing key.
    #[serde(rename = "verificationMethod")]
    pub verification_method: String,

    /// The purpose of this proof.
    #[serde(rename = "proofPurpose")]
    pub proof_purpose: ProofPurpose,

    /// The proof value — hex-encoded signature bytes.
    ///
    /// For Ed25519: 64 bytes → 128 hex characters.
    #[serde(rename = "proofValue")]
    pub proof_value: String,
}

impl Proof {
    /// Create a new Ed25519Signature2020 proof.
    ///
    /// # Arguments
    ///
    /// * `verification_method` — DID URL of the signing key
    /// * `proof_value` — Hex-encoded Ed25519 signature (128 hex chars)
    /// * `created` — Optional creation timestamp; defaults to current UTC time
    pub fn new_ed25519(
        verification_method: String,
        proof_value: String,
        created: Option<Timestamp>,
    ) -> Self {
        let ts = created.unwrap_or_else(Timestamp::now);
        Self {
            proof_type: ProofType::Ed25519Signature2020,
            created: *ts.as_datetime(),
            verification_method,
            proof_purpose: ProofPurpose::AssertionMethod,
            proof_value,
        }
    }

    /// Create a new proof using the MEZ-specific Ed25519 proof type.
    ///
    /// Uses `MezEd25519Signature2025` for compatibility with the Python layer.
    pub fn new_mez_ed25519(
        verification_method: String,
        proof_value: String,
        created: Option<Timestamp>,
    ) -> Self {
        let ts = created.unwrap_or_else(Timestamp::now);
        Self {
            proof_type: ProofType::MezEd25519Signature2025,
            created: *ts.as_datetime(),
            verification_method,
            proof_purpose: ProofPurpose::AssertionMethod,
            proof_value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proof_type_serde_roundtrip() {
        let ed25519 = ProofType::Ed25519Signature2020;
        let json = serde_json::to_string(&ed25519).unwrap();
        assert_eq!(json, r#""Ed25519Signature2020""#);
        let back: ProofType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, ed25519);
    }

    #[test]
    fn mez_proof_type_serde_roundtrip() {
        let mez = ProofType::MezEd25519Signature2025;
        let json = serde_json::to_string(&mez).unwrap();
        assert_eq!(json, r#""MezEd25519Signature2025""#);
        let back: ProofType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, mez);
    }

    #[test]
    fn proof_type_is_ed25519() {
        assert!(ProofType::Ed25519Signature2020.is_ed25519());
        assert!(ProofType::MezEd25519Signature2025.is_ed25519());
        assert!(!ProofType::BbsBlsSignature2020.is_ed25519());
    }

    #[test]
    fn proof_purpose_serde_camel_case() {
        let purpose = ProofPurpose::AssertionMethod;
        let json = serde_json::to_string(&purpose).unwrap();
        assert_eq!(json, r#""assertionMethod""#);

        let auth = ProofPurpose::Authentication;
        let json = serde_json::to_string(&auth).unwrap();
        assert_eq!(json, r#""authentication""#);
    }

    #[test]
    fn proof_struct_serializes_correctly() {
        let proof = Proof {
            proof_type: ProofType::Ed25519Signature2020,
            created: chrono::Utc::now(),
            verification_method: "did:key:z6MkTest#key-1".to_string(),
            proof_purpose: ProofPurpose::AssertionMethod,
            proof_value: "ab".repeat(64),
        };

        let val = serde_json::to_value(&proof).unwrap();
        assert_eq!(val["type"], "Ed25519Signature2020");
        assert_eq!(val["verificationMethod"], "did:key:z6MkTest#key-1");
        assert_eq!(val["proofPurpose"], "assertionMethod");
        assert!(val["proofValue"].is_string());
        assert!(val["created"].is_string());
    }

    #[test]
    fn proof_json_field_names_match_w3c_spec() {
        let proof = Proof::new_ed25519("did:key:z6Mk123#key-1".to_string(), "00".repeat(64), None);

        let val = serde_json::to_value(&proof).unwrap();
        assert!(val.get("type").is_some());
        assert!(val.get("created").is_some());
        assert!(val.get("verificationMethod").is_some());
        assert!(val.get("proofPurpose").is_some());
        assert!(val.get("proofValue").is_some());
        // Must NOT have snake_case versions
        assert!(val.get("proof_type").is_none());
        assert!(val.get("verification_method").is_none());
        assert!(val.get("proof_purpose").is_none());
        assert!(val.get("proof_value").is_none());
    }

    #[test]
    fn proof_deserializes_from_w3c_json() {
        let json_str = r#"{
            "type": "Ed25519Signature2020",
            "created": "2026-01-15T12:00:00Z",
            "verificationMethod": "did:key:z6MkTest#key-1",
            "proofPurpose": "assertionMethod",
            "proofValue": "deadbeef"
        }"#;

        let proof: Proof = serde_json::from_str(json_str).unwrap();
        assert_eq!(proof.proof_type, ProofType::Ed25519Signature2020);
        assert_eq!(proof.verification_method, "did:key:z6MkTest#key-1");
        assert_eq!(proof.proof_purpose, ProofPurpose::AssertionMethod);
        assert_eq!(proof.proof_value, "deadbeef");
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn proof_purpose_display_assertion_method() {
        assert_eq!(
            format!("{}", ProofPurpose::AssertionMethod),
            "assertionMethod"
        );
    }

    #[test]
    fn proof_purpose_display_authentication() {
        assert_eq!(
            format!("{}", ProofPurpose::Authentication),
            "authentication"
        );
    }

    #[test]
    fn proof_type_display_all_variants() {
        assert_eq!(
            format!("{}", ProofType::Ed25519Signature2020),
            "Ed25519Signature2020"
        );
        assert_eq!(
            format!("{}", ProofType::MezEd25519Signature2025),
            "MezEd25519Signature2025"
        );
        assert_eq!(
            format!("{}", ProofType::BbsBlsSignature2020),
            "BbsBlsSignature2020"
        );
    }

    #[test]
    fn bbs_proof_type_serde_roundtrip() {
        let bbs = ProofType::BbsBlsSignature2020;
        let json = serde_json::to_string(&bbs).unwrap();
        assert_eq!(json, r#""BbsBlsSignature2020""#);
        let back: ProofType = serde_json::from_str(&json).unwrap();
        assert_eq!(back, bbs);
    }

    #[test]
    fn proof_full_serde_roundtrip() {
        let proof = Proof::new_ed25519(
            "did:key:z6MkRoundtrip#key-1".to_string(),
            "aa".repeat(64),
            None,
        );

        let json_str = serde_json::to_string(&proof).unwrap();
        let deserialized: Proof = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.proof_type, ProofType::Ed25519Signature2020);
        assert_eq!(
            deserialized.verification_method,
            "did:key:z6MkRoundtrip#key-1"
        );
        assert_eq!(deserialized.proof_purpose, ProofPurpose::AssertionMethod);
        assert_eq!(deserialized.proof_value, "aa".repeat(64));
        assert_eq!(deserialized.created, proof.created);
    }

    #[test]
    fn proof_mez_ed25519_serde_roundtrip() {
        let proof =
            Proof::new_mez_ed25519("did:key:z6MkMez#key-1".to_string(), "bb".repeat(64), None);

        let json_str = serde_json::to_string(&proof).unwrap();
        let deserialized: Proof = serde_json::from_str(&json_str).unwrap();

        assert_eq!(deserialized.proof_type, ProofType::MezEd25519Signature2025);
        assert_eq!(deserialized.verification_method, "did:key:z6MkMez#key-1");
        assert_eq!(deserialized.proof_purpose, ProofPurpose::AssertionMethod);
        assert_eq!(deserialized.proof_value, "bb".repeat(64));
    }

    #[test]
    fn proof_deserializes_authentication_purpose() {
        let json_str = r#"{
            "type": "Ed25519Signature2020",
            "created": "2026-06-01T00:00:00Z",
            "verificationMethod": "did:key:z6MkAuth#key-1",
            "proofPurpose": "authentication",
            "proofValue": "cafebabe"
        }"#;

        let proof: Proof = serde_json::from_str(json_str).unwrap();
        assert_eq!(proof.proof_purpose, ProofPurpose::Authentication);
    }

    #[test]
    fn proof_new_ed25519_with_explicit_timestamp() {
        let ts = Timestamp::now();
        let proof = Proof::new_ed25519(
            "did:key:z6MkTs#key-1".to_string(),
            "cc".repeat(64),
            Some(ts.clone()),
        );
        assert_eq!(proof.created, *ts.as_datetime());
    }

    #[test]
    fn proof_new_mez_ed25519_with_explicit_timestamp() {
        let ts = Timestamp::now();
        let proof = Proof::new_mez_ed25519(
            "did:key:z6MkTs#key-1".to_string(),
            "dd".repeat(64),
            Some(ts.clone()),
        );
        assert_eq!(proof.created, *ts.as_datetime());
        assert_eq!(proof.proof_type, ProofType::MezEd25519Signature2025);
    }

    #[test]
    fn proof_purpose_serde_roundtrip_authentication() {
        let purpose = ProofPurpose::Authentication;
        let json = serde_json::to_string(&purpose).unwrap();
        let back: ProofPurpose = serde_json::from_str(&json).unwrap();
        assert_eq!(back, purpose);
    }

    #[test]
    fn proof_purpose_serde_roundtrip_assertion() {
        let purpose = ProofPurpose::AssertionMethod;
        let json = serde_json::to_string(&purpose).unwrap();
        let back: ProofPurpose = serde_json::from_str(&json).unwrap();
        assert_eq!(back, purpose);
    }

    #[test]
    fn proof_bbs_is_not_ed25519() {
        assert!(!ProofType::BbsBlsSignature2020.is_ed25519());
    }

    #[test]
    fn proof_deserializes_mez_type_from_json() {
        let _json_str = r#"{
            "type": "MezEd25519Signature2025",
            "created": "2026-03-01T10:30:00Z",
            "verificationMethod": "did:mez:key:001",
            "proofPurpose": "assertionMethod",
            "proofValue": "ee".repeat(64)
        }"#
        .replace("\"ee\".repeat(64)", &format!("\"{}\"", "ee".repeat(64)));

        // Build the JSON string properly
        let json_str = format!(
            r#"{{"type":"MezEd25519Signature2025","created":"2026-03-01T10:30:00Z","verificationMethod":"did:mez:key:001","proofPurpose":"assertionMethod","proofValue":"{}"}}"#,
            "ee".repeat(64)
        );

        let proof: Proof = serde_json::from_str(&json_str).unwrap();
        assert_eq!(proof.proof_type, ProofType::MezEd25519Signature2025);
    }
}
