//! # Verifiable Credential structure, signing, and verification
//!
//! Defines the core [`VerifiableCredential`] type following the W3C VC
//! Data Model, adapted for EZ Stack conventions.
//!
//! ## Security Invariants
//!
//! - **Signing** canonicalizes the credential body (with `proof` removed)
//!   via [`CanonicalBytes::new()`], computes an Ed25519 signature, and
//!   attaches a [`Proof`] object. No raw `serde_json::to_vec()` or
//!   `serde_json::to_string()` is used in the signing path.
//!
//! - **Verification** extracts the proof, recomputes `CanonicalBytes` from
//!   the credential body (without proof), and verifies the Ed25519 signature.
//!
//! - The envelope structure is rigid, while `credential_subject` is
//!   intentionally extensible per the W3C specification.
//!
//! ## Spec Reference
//!
//! Implements the signing path from `tools/vc.py:signing_input()` →
//! `canonicalize_json()` → `jcs_canonicalize()` and the verification path
//! from `tools/vc.py:verify_credential()`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use mez_core::{CanonicalBytes, Timestamp};
use mez_crypto::{Ed25519Signature, SigningKey, VerifyingKey};

use crate::proof::{Proof, ProofPurpose, ProofType};

/// Errors from VC signing and verification operations.
#[derive(Error, Debug)]
pub enum VcError {
    /// Canonicalization of the credential body failed.
    #[error("canonicalization failed: {0}")]
    Canonicalization(#[from] mez_core::CanonicalizationError),

    /// Ed25519 signature verification failed.
    #[error("signature verification failed: {0}")]
    VerificationFailed(String),

    /// The proof has an unsupported type for verification.
    #[error("unsupported proof type: {0}")]
    UnsupportedProofType(String),

    /// The credential has no proofs to verify.
    #[error("credential has no proofs")]
    NoProofs,

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The proof value has invalid hex encoding or wrong length.
    #[error("invalid proof value: {0}")]
    InvalidProofValue(String),

    /// Schema validation failed.
    #[error("schema validation failed: {0}")]
    SchemaValidation(String),
}

/// The result of verifying a single proof on a credential.
///
/// Matches the shape of Python's `ProofResult` dataclass from
/// `tools/vc.py:208-212`.
#[derive(Debug, Clone)]
pub struct ProofResult {
    /// The verification method (DID URL) from the proof.
    pub verification_method: String,
    /// Whether the signature was valid.
    pub ok: bool,
    /// Error message if verification failed; empty string if ok.
    pub error: String,
}

/// A W3C Verifiable Credential with EZ Stack extensions.
///
/// The envelope structure is rigid, while `credential_subject` is
/// intentionally extensible per the W3C specification.
///
/// ## Field Naming
///
/// Serde rename attributes map between Rust snake_case and the W3C VC
/// JSON field names (camelCase / `@`-prefixed).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifiableCredential {
    /// JSON-LD context URIs.
    #[serde(rename = "@context")]
    pub context: ContextValue,

    /// Credential identifier (URN or DID).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Credential type(s). MUST include `"VerifiableCredential"`.
    #[serde(rename = "type")]
    pub credential_type: CredentialTypeValue,

    /// DID of the credential issuer.
    pub issuer: String,

    /// When the credential was issued (UTC).
    #[serde(rename = "issuanceDate")]
    pub issuance_date: DateTime<Utc>,

    /// Optional expiration date (UTC).
    #[serde(
        rename = "expirationDate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub expiration_date: Option<DateTime<Utc>>,

    /// The credential subject — intentionally extensible per W3C spec.
    #[serde(rename = "credentialSubject")]
    pub credential_subject: serde_json::Value,

    /// Cryptographic proofs attached to this credential.
    #[serde(default, skip_serializing_if = "ProofValue::is_empty")]
    pub proof: ProofValue,
}

/// JSON-LD `@context` value — either a single string or an array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ContextValue {
    /// Single context URI string.
    Single(String),
    /// Array of context URI strings or objects.
    Array(Vec<serde_json::Value>),
}

impl Default for ContextValue {
    fn default() -> Self {
        Self::Array(vec![serde_json::Value::String(
            "https://www.w3.org/2018/credentials/v1".to_string(),
        )])
    }
}

/// Credential `type` value — either a single string or an array.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CredentialTypeValue {
    /// Single type string.
    Single(String),
    /// Array of type strings.
    Array(Vec<String>),
}

impl CredentialTypeValue {
    /// Check whether `"VerifiableCredential"` is included in the type.
    pub fn contains_vc_type(&self) -> bool {
        match self {
            CredentialTypeValue::Single(s) => s == "VerifiableCredential",
            CredentialTypeValue::Array(arr) => arr.iter().any(|s| s == "VerifiableCredential"),
        }
    }
}

/// Proof value — supports single proof, array of proofs, or absent.
///
/// The Python implementation normalizes proofs to a list internally
/// (`_proofs_as_list`). This enum handles the JSON polymorphism at the
/// serde level.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProofValue {
    /// A single proof object.
    Single(Box<Proof>),
    /// An array of proof objects.
    Array(Vec<Proof>),
}

impl Default for ProofValue {
    fn default() -> Self {
        Self::Array(Vec::new())
    }
}

impl ProofValue {
    /// Returns `true` if there are no proofs.
    pub fn is_empty(&self) -> bool {
        match self {
            ProofValue::Single(_) => false,
            ProofValue::Array(arr) => arr.is_empty(),
        }
    }

    /// Normalize to a list of proof references (matching Python's `_proofs_as_list`).
    pub fn as_list(&self) -> Vec<&Proof> {
        match self {
            ProofValue::Single(p) => vec![p.as_ref()],
            ProofValue::Array(arr) => arr.iter().collect(),
        }
    }

    /// Consume and normalize to owned list of proofs.
    pub fn into_list(self) -> Vec<Proof> {
        match self {
            ProofValue::Single(p) => vec![*p],
            ProofValue::Array(arr) => arr,
        }
    }

    /// Add a proof, converting Single to Array if needed.
    pub fn push(&mut self, proof: Proof) {
        match self {
            ProofValue::Single(existing) => {
                let prev = existing.clone();
                *self = ProofValue::Array(vec![*prev, proof]);
            }
            ProofValue::Array(arr) => {
                arr.push(proof);
            }
        }
    }
}

impl VerifiableCredential {
    /// Compute the canonical signing input for this credential.
    ///
    /// The signing input is the JCS-canonicalized bytes of the credential
    /// with the `proof` field removed. This matches the Python implementation
    /// in `tools/vc.py:signing_input()`.
    ///
    /// # Security Invariant
    ///
    /// Uses [`CanonicalBytes::from_value()`] — never raw `serde_json::to_vec()`.
    pub fn signing_input(&self) -> Result<CanonicalBytes, VcError> {
        let mut val = serde_json::to_value(self)?;
        if let Some(obj) = val.as_object_mut() {
            obj.remove("proof");
        }
        Ok(CanonicalBytes::from_value(val)?)
    }

    /// Sign this credential with an Ed25519 key pair.
    ///
    /// Computes the canonical signing input (credential body without proof),
    /// signs it with the provided key, and attaches the proof object.
    ///
    /// # Security Invariant
    ///
    /// The signing input is computed via [`CanonicalBytes`], not raw
    /// serialization. The Ed25519 `SigningKey::sign()` method only accepts
    /// `&CanonicalBytes`, enforcing this at the type level.
    ///
    /// Implements `tools/vc.py:add_ed25519_proof()`.
    pub fn sign_ed25519(
        &mut self,
        signing_key: &SigningKey,
        verification_method: String,
        proof_type: ProofType,
        created: Option<Timestamp>,
    ) -> Result<(), VcError> {
        let canonical = self.signing_input()?;
        let signature = signing_key.sign(&canonical);

        let proof = Proof {
            proof_type,
            created: *created.unwrap_or_else(Timestamp::now).as_datetime(),
            verification_method,
            proof_purpose: ProofPurpose::AssertionMethod,
            proof_value: signature.to_hex(),
        };

        self.proof.push(proof);
        Ok(())
    }

    /// Verify all Ed25519 proofs on this credential.
    ///
    /// Returns a [`ProofResult`] for each proof. Checks expiration first:
    /// an expired credential yields all-failed results without performing
    /// signature verification. A credential with zero proofs returns an
    /// empty `Vec`, which callers must treat as verification failure (not
    /// vacuously-true success).
    ///
    /// # Arguments
    ///
    /// * `resolve_key` — A function that resolves a verification method
    ///   string to a [`VerifyingKey`].
    pub fn verify<F>(&self, resolve_key: F) -> Vec<ProofResult>
    where
        F: Fn(&str) -> Result<VerifyingKey, String>,
    {
        // Check expiration before spending CPU on signature verification.
        if let Some(expiration) = self.expiration_date {
            if expiration < Utc::now() {
                return self
                    .proof
                    .as_list()
                    .iter()
                    .map(|p| ProofResult {
                        verification_method: p.verification_method.clone(),
                        ok: false,
                        error: format!("credential expired at {expiration}"),
                    })
                    .collect();
            }
        }

        let canonical = match self.signing_input() {
            Ok(c) => c,
            Err(e) => {
                return self
                    .proof
                    .as_list()
                    .iter()
                    .map(|p| ProofResult {
                        verification_method: p.verification_method.clone(),
                        ok: false,
                        error: format!("canonicalization failed: {e}"),
                    })
                    .collect();
            }
        };

        self.proof
            .as_list()
            .iter()
            .map(|proof| {
                let vm = proof.verification_method.clone();
                match verify_single_proof(proof, &canonical, &resolve_key) {
                    Ok(()) => ProofResult {
                        verification_method: vm,
                        ok: true,
                        error: String::new(),
                    },
                    Err(e) => ProofResult {
                        verification_method: vm,
                        ok: false,
                        error: e.to_string(),
                    },
                }
            })
            .collect()
    }

    /// Verify all proofs and return `Ok(())` only if all pass.
    pub fn verify_all<F>(&self, resolve_key: F) -> Result<(), VcError>
    where
        F: Fn(&str) -> Result<VerifyingKey, String>,
    {
        // Check expiration before spending CPU on signature verification.
        // An expired credential is invalid regardless of signature validity.
        if let Some(expiration) = self.expiration_date {
            if expiration < Utc::now() {
                return Err(VcError::VerificationFailed(format!(
                    "credential expired at {expiration}"
                )));
            }
        }

        let results = self.verify(resolve_key);
        if results.is_empty() {
            return Err(VcError::NoProofs);
        }
        for r in &results {
            if !r.ok {
                return Err(VcError::VerificationFailed(format!(
                    "proof from {} failed: {}",
                    r.verification_method, r.error
                )));
            }
        }
        Ok(())
    }
}

/// Verify a single proof against the canonical signing input.
fn verify_single_proof<F>(
    proof: &Proof,
    canonical: &CanonicalBytes,
    resolve_key: &F,
) -> Result<(), VcError>
where
    F: Fn(&str) -> Result<VerifyingKey, String>,
{
    if !proof.proof_type.is_ed25519() {
        return Err(VcError::UnsupportedProofType(proof.proof_type.to_string()));
    }

    let vk = resolve_key(&proof.verification_method).map_err(VcError::VerificationFailed)?;

    let sig = Ed25519Signature::from_hex(&proof.proof_value).map_err(|e| {
        VcError::InvalidProofValue(format!("failed to decode proof value as hex: {e}"))
    })?;

    vk.verify(canonical, &sig)
        .map_err(|e| VcError::VerificationFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mez_crypto::SigningKey;
    use rand_core::OsRng;
    use serde_json::json;

    fn make_test_vc() -> VerifiableCredential {
        VerifiableCredential {
            context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
            id: Some("urn:mez:vc:test:001".to_string()),
            credential_type: CredentialTypeValue::Array(vec![
                "VerifiableCredential".to_string(),
                "MezTestCredential".to_string(),
            ]),
            issuer: "did:key:z6MkTestIssuer".to_string(),
            issuance_date: chrono::Utc::now(),
            expiration_date: None,
            credential_subject: json!({
                "asset_id": "a".repeat(64),
                "name": "Test Asset"
            }),
            proof: ProofValue::default(),
        }
    }

    fn make_key_resolver(vk: VerifyingKey) -> impl Fn(&str) -> Result<VerifyingKey, String> {
        move |_vm: &str| Ok(vk.clone())
    }

    #[test]
    fn signing_input_excludes_proof() {
        let mut vc = make_test_vc();
        let input_before = vc.signing_input().unwrap();

        vc.proof = ProofValue::Single(Box::new(Proof::new_ed25519(
            "did:key:z6MkFake#key-1".to_string(),
            "00".repeat(64),
            None,
        ).unwrap()));

        let input_after = vc.signing_input().unwrap();
        assert_eq!(input_before.as_bytes(), input_after.as_bytes());
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        assert!(!vc.proof.is_empty());

        let results = vc.verify(make_key_resolver(vk));
        assert_eq!(results.len(), 1);
        assert!(results[0].ok, "verification failed: {}", results[0].error);
    }

    #[test]
    fn sign_with_mez_proof_type_and_verify() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::MezEd25519Signature2025,
            None,
        )
        .unwrap();

        let results = vc.verify(make_key_resolver(vk));
        assert_eq!(results.len(), 1);
        assert!(results[0].ok);
    }

    #[test]
    fn verification_fails_with_wrong_key() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let vk2 = sk2.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk1,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        let results = vc.verify(make_key_resolver(vk2));
        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
    }

    #[test]
    fn verification_fails_with_tampered_subject() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        vc.credential_subject = json!({"asset_id": "b".repeat(64), "name": "Tampered"});

        let results = vc.verify(make_key_resolver(vk));
        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
    }

    #[test]
    fn multi_party_signing() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let vk1 = sk1.verifying_key();
        let vk2 = sk2.verifying_key();

        let mut vc = make_test_vc();

        vc.sign_ed25519(
            &sk1,
            "did:key:z6MkSigner1#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        vc.sign_ed25519(
            &sk2,
            "did:key:z6MkSigner2#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        assert_eq!(vc.proof.as_list().len(), 2);

        let results = vc.verify(move |vm: &str| {
            if vm.contains("Signer1") {
                Ok(vk1.clone())
            } else if vm.contains("Signer2") {
                Ok(vk2.clone())
            } else {
                Err(format!("unknown verification method: {vm}"))
            }
        });

        assert_eq!(results.len(), 2);
        assert!(results[0].ok, "signer1 failed: {}", results[0].error);
        assert!(results[1].ok, "signer2 failed: {}", results[1].error);
    }

    #[test]
    fn verify_all_returns_error_on_failure() {
        let sk1 = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let vk2 = sk2.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk1,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        let result = vc.verify_all(make_key_resolver(vk2));
        assert!(result.is_err());
    }

    #[test]
    fn verify_all_returns_error_on_no_proofs() {
        let vc = make_test_vc();
        let result = vc.verify_all(|_| Err("no key".to_string()));
        assert!(matches!(result, Err(VcError::NoProofs)));
    }

    #[test]
    fn vc_serde_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        let json_str = serde_json::to_string_pretty(&vc).unwrap();
        let vc2: VerifiableCredential = serde_json::from_str(&json_str).unwrap();

        assert_eq!(vc.issuer, vc2.issuer);
        assert_eq!(vc.proof.as_list().len(), vc2.proof.as_list().len());
    }

    #[test]
    fn vc_json_field_names_match_w3c() {
        let vc = make_test_vc();
        let val = serde_json::to_value(&vc).unwrap();

        assert!(val.get("@context").is_some());
        assert!(val.get("type").is_some());
        assert!(val.get("issuanceDate").is_some());
        assert!(val.get("credentialSubject").is_some());
        assert!(val.get("credential_type").is_none());
        assert!(val.get("issuance_date").is_none());
        assert!(val.get("credential_subject").is_none());
    }

    #[test]
    fn signing_input_is_deterministic() {
        let vc = make_test_vc();
        let input1 = vc.signing_input().unwrap();
        let input2 = vc.signing_input().unwrap();
        assert_eq!(input1.as_bytes(), input2.as_bytes());
    }

    #[test]
    fn signing_input_rejects_float_in_subject() {
        let mut vc = make_test_vc();
        vc.credential_subject = json!({"amount": 3.15});
        let result = vc.signing_input();
        assert!(result.is_err());
    }

    #[test]
    fn credential_type_contains_vc_type() {
        let single = CredentialTypeValue::Single("VerifiableCredential".to_string());
        assert!(single.contains_vc_type());

        let array = CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "Custom".to_string(),
        ]);
        assert!(array.contains_vc_type());

        let no_vc = CredentialTypeValue::Array(vec!["Custom".to_string()]);
        assert!(!no_vc.contains_vc_type());
    }

    #[test]
    fn proof_value_push_converts_single_to_array() {
        let p1 = Proof::new_ed25519("vm1".to_string(), "aa".repeat(64), None).unwrap();
        let p2 = Proof::new_ed25519("vm2".to_string(), "bb".repeat(64), None).unwrap();

        let mut pv = ProofValue::Single(Box::new(p1));
        assert_eq!(pv.as_list().len(), 1);

        pv.push(p2);
        assert_eq!(pv.as_list().len(), 2);
    }

    #[test]
    fn unsupported_proof_type_returns_error() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let mut vc = make_test_vc();
        vc.proof = ProofValue::Single(Box::new(Proof {
            proof_type: ProofType::BbsBlsSignature2020,
            created: chrono::Utc::now(),
            verification_method: "did:key:z6MkTest#key-1".to_string(),
            proof_purpose: ProofPurpose::AssertionMethod,
            proof_value: "00".repeat(64),
        }));

        let results = vc.verify(make_key_resolver(vk));
        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
        assert!(results[0].error.contains("unsupported proof type"));
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn context_value_default() {
        let ctx = ContextValue::default();
        match ctx {
            ContextValue::Array(arr) => {
                assert_eq!(arr.len(), 1);
                assert_eq!(arr[0], "https://www.w3.org/2018/credentials/v1");
            }
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn proof_value_default_is_empty() {
        let pv = ProofValue::default();
        assert!(pv.is_empty());
        assert!(pv.as_list().is_empty());
    }

    #[test]
    fn proof_value_single_not_empty() {
        let p = Proof::new_ed25519("vm1".into(), "aa".repeat(64), None).unwrap();
        let pv = ProofValue::Single(Box::new(p));
        assert!(!pv.is_empty());
    }

    #[test]
    fn proof_value_into_list_single() {
        let p = Proof::new_ed25519("vm1".into(), "aa".repeat(64), None).unwrap();
        let pv = ProofValue::Single(Box::new(p));
        let list = pv.into_list();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn proof_value_into_list_array() {
        let p1 = Proof::new_ed25519("vm1".into(), "aa".repeat(64), None).unwrap();
        let p2 = Proof::new_ed25519("vm2".into(), "bb".repeat(64), None).unwrap();
        let pv = ProofValue::Array(vec![p1, p2]);
        let list = pv.into_list();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn credential_type_single_non_vc() {
        let ct = CredentialTypeValue::Single("Custom".to_string());
        assert!(!ct.contains_vc_type());
    }

    #[test]
    fn signing_input_deterministic() {
        let vc = make_test_vc();
        let si1 = vc.signing_input().unwrap();
        let si2 = vc.signing_input().unwrap();
        assert_eq!(si1.as_bytes(), si2.as_bytes());
    }

    #[test]
    fn sign_and_verify_roundtrip_full() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        assert!(!vc.proof.is_empty());
        let results = vc.verify(make_key_resolver(vk));
        assert_eq!(results.len(), 1);
        assert!(
            results[0].ok,
            "verification should succeed: {}",
            results[0].error
        );
    }

    #[test]
    fn verify_with_no_proofs() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        let vc = make_test_vc();
        // No proofs attached
        let results = vc.verify(make_key_resolver(vk));
        assert!(results.is_empty());
    }

    #[test]
    fn verify_with_wrong_key() {
        let sk = SigningKey::generate(&mut OsRng);
        let sk2 = SigningKey::generate(&mut OsRng);
        let vk2 = sk2.verifying_key();

        let mut vc = make_test_vc();
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        let results = vc.verify(make_key_resolver(vk2));
        assert_eq!(results.len(), 1);
        assert!(!results[0].ok);
    }

    #[test]
    fn vc_serde_roundtrip_coverage() {
        let vc = make_test_vc();
        let json_str = serde_json::to_string(&vc).unwrap();
        let deserialized: VerifiableCredential = serde_json::from_str(&json_str).unwrap();
        assert_eq!(vc.issuer, deserialized.issuer);
    }

    #[test]
    fn vc_with_expiration_date() {
        let vc = VerifiableCredential {
            context: ContextValue::default(),
            id: Some("urn:test:expired".to_string()),
            credential_type: CredentialTypeValue::Array(vec![
                "VerifiableCredential".to_string(),
                "TestExpiry".to_string(),
            ]),
            issuer: "did:key:z6MkIssuer".to_string(),
            issuance_date: chrono::Utc::now(),
            expiration_date: Some(chrono::Utc::now() + chrono::Duration::days(365)),
            credential_subject: serde_json::json!({"id": "subject-1"}),
            proof: ProofValue::default(),
        };
        let json_str = serde_json::to_string(&vc).unwrap();
        assert!(json_str.contains("expirationDate"));
        let deserialized: VerifiableCredential = serde_json::from_str(&json_str).unwrap();
        assert!(deserialized.expiration_date.is_some());
    }

    #[test]
    fn vc_error_display() {
        let err = VcError::NoProofs;
        assert_eq!(format!("{err}"), "credential has no proofs");

        let err2 = VcError::UnsupportedProofType("BBS".to_string());
        assert!(format!("{err2}").contains("BBS"));
    }

    #[test]
    fn proof_result_debug() {
        let pr = ProofResult {
            verification_method: "did:key:z6Mk#key-1".to_string(),
            ok: true,
            error: String::new(),
        };
        let debug = format!("{pr:?}");
        assert!(debug.contains("ProofResult"));
    }
}
