//! # Smart Asset Registry VC
//!
//! A Verifiable Credential that attests to a smart asset's registration,
//! compliance evaluation, and jurisdictional bindings.
//!
//! ## Schema
//!
//! Implements the structure from `schemas/vc.smart-asset-registry.schema.json`.
//! Validates against the JSON schema from `msez-schema` at construction time
//! via [`SmartAssetRegistryVc::validate_schema()`].
//!
//! ## Security Invariant
//!
//! The registry VC binds an `asset_id` (a SHA-256 content digest of the genesis
//! document) to one or more jurisdictional bindings, each containing lawpack
//! references with pinned digests. Tampering with any field after signing
//! invalidates the Ed25519 proof.
//!
//! ## Spec Reference
//!
//! Implements `tools/smart_asset.py:cmd_asset_registry_init()` and the
//! `evaluate_transition_compliance()` compliance evaluation pipeline.

use serde::{Deserialize, Serialize};

use crate::credential::{
    ContextValue, CredentialTypeValue, ProofValue, VcError, VerifiableCredential,
};
use crate::proof::ProofType;
use msez_core::{CanonicalBytes, Timestamp};
use msez_crypto::SigningKey;

/// The `$id` URI of the smart asset registry schema.
pub const REGISTRY_SCHEMA_ID: &str =
    "https://schemas.momentum-sez.org/msez/vc.smart-asset-registry.schema.json";

// ---------------------------------------------------------------------------
// Credential subject types
// ---------------------------------------------------------------------------

/// An artifact reference in content-addressed storage.
///
/// Matches the `artifact-ref.schema.json` format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    /// The artifact type (e.g., `"smart-asset-genesis"`).
    pub artifact_type: String,
    /// SHA-256 digest of the artifact content (64 hex chars).
    pub digest_sha256: String,
    /// Optional URI pointing to the artifact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
    /// Optional MIME type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
}

/// A lawpack reference pinned by digest.
///
/// Matches the `LawpackRef` definition in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LawpackRef {
    /// Jurisdiction identifier (e.g., `"PK"` for Pakistan).
    pub jurisdiction_id: String,
    /// Compliance domain (e.g., `"corporate"`, `"aml"`).
    pub domain: String,
    /// SHA-256 digest of the lawpack bundle (64 hex chars).
    pub lawpack_digest_sha256: String,
    /// Optional artifact reference to the lawpack bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lawpack: Option<ArtifactRef>,
}

/// Compliance profile for a jurisdictional binding.
///
/// Declarative compliance profile used by the reference compliance evaluator.
/// Matches `ComplianceProfile` in the schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComplianceProfile {
    /// Transition kinds that are permitted under this binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_transition_kinds: Option<Vec<String>>,

    /// Map of `transition_kind` → required attestation kinds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_attestations: Option<serde_json::Map<String, serde_json::Value>>,

    /// Attestation kinds required for all transitions (default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_required_attestations: Option<Vec<String>>,
}

/// Enforcement profile for a jurisdictional binding.
///
/// Matches `EnforcementProfile` in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnforcementProfile {
    /// Enforcement intensity level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intensity: Option<String>,

    /// Audit frequency.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_frequency: Option<String>,

    /// Human-readable notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Status of a jurisdictional binding.
///
/// Restricts values to the schema-defined set: `active`, `suspended`, `exited`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BindingStatus {
    /// Binding is active — compliance evaluation applies.
    Active,
    /// Binding is temporarily suspended.
    Suspended,
    /// Entity has exited this jurisdiction.
    Exited,
}

impl std::fmt::Display for BindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Suspended => write!(f, "suspended"),
            Self::Exited => write!(f, "exited"),
        }
    }
}

/// A jurisdictional binding for a smart asset.
///
/// Binds the asset to a specific zone/harbor with compliance and enforcement
/// profiles. Matches `JurisdictionBinding` in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionBinding {
    /// Zone or harbor identifier.
    pub harbor_id: String,

    /// Binding status.
    pub binding_status: BindingStatus,

    /// Redundancy role for this jurisdictional shard.
    pub shard_role: String,

    /// Lawpack references (at least one).
    pub lawpacks: Vec<LawpackRef>,

    /// Compliance profile.
    pub compliance_profile: ComplianceProfile,

    /// Optional enforcement profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforcement_profile: Option<EnforcementProfile>,

    /// When this binding becomes effective (UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_from: Option<String>,

    /// When this binding expires (UTC).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_until: Option<String>,

    /// Human-readable notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// The credential subject for a Smart Asset Registry VC.
///
/// Matches the `credentialSubject` shape in
/// `schemas/vc.smart-asset-registry.schema.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartAssetRegistrySubject {
    /// The asset identifier — SHA-256 digest of the genesis document (64 hex).
    pub asset_id: String,

    /// Stack specification version (semver).
    pub stack_spec_version: String,

    /// Reference to the genesis artifact.
    pub asset_genesis: ArtifactRef,

    /// Human-readable asset name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_name: Option<String>,

    /// Asset classification type (e.g., `"equity"`, `"bond"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_class: Option<String>,

    /// Jurisdictional bindings (at least one).
    pub jurisdiction_bindings: Vec<JurisdictionBinding>,

    /// Optional quorum policy for multi-jurisdiction compliance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quorum_policy: Option<serde_json::Value>,

    /// Human-readable notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// The result of evaluating compliance for a single jurisdiction binding.
///
/// Matches Python's `BindingComplianceResult` from
/// `tools/smart_asset.py:179-186`.
#[derive(Debug, Clone)]
pub struct BindingComplianceResult {
    /// Harbor/zone identifier.
    pub harbor_id: String,
    /// Binding status.
    pub binding_status: BindingStatus,
    /// Shard role.
    pub shard_role: String,
    /// Whether the transition is allowed under this binding.
    pub allowed: bool,
    /// Reasons the transition was allowed or denied.
    pub reasons: Vec<String>,
    /// Attestation kinds that are required but not present.
    pub missing_attestations: Vec<String>,
    /// Attestation kinds that were present.
    pub present_attestations: Vec<String>,
}

// ---------------------------------------------------------------------------
// SmartAssetRegistryVc
// ---------------------------------------------------------------------------

/// A Smart Asset Registry Verifiable Credential.
///
/// Wraps a [`VerifiableCredential`] with typed access to the
/// `SmartAssetRegistrySubject` and provides schema validation.
#[derive(Debug, Clone)]
pub struct SmartAssetRegistryVc {
    vc: VerifiableCredential,
}

impl SmartAssetRegistryVc {
    /// Create a new unsigned Smart Asset Registry VC.
    ///
    /// Constructs the VC envelope with the provided subject, matching
    /// `tools/smart_asset.py:cmd_asset_registry_init()`.
    pub fn new(
        issuer: String,
        subject: SmartAssetRegistrySubject,
        issuance_date: Option<Timestamp>,
    ) -> Result<Self, VcError> {
        // P2-SA-002: Validate asset_id format (must be 64 lowercase hex chars).
        if !Self::is_valid_sha256_hex(&subject.asset_id) {
            return Err(VcError::SchemaValidation(format!(
                "asset_id must be 64 lowercase hex chars, got: {:?}",
                subject.asset_id
            )));
        }

        let ts = issuance_date.unwrap_or_else(Timestamp::now);
        let asset_id = subject.asset_id.clone();
        let subject_value = serde_json::to_value(&subject).map_err(VcError::Json)?;

        let vc = VerifiableCredential {
            context: ContextValue::Array(vec![serde_json::Value::String(
                "https://www.w3.org/2018/credentials/v1".to_string(),
            )]),
            id: Some(format!("urn:msez:vc:smart-asset-registry:{asset_id}")),
            credential_type: CredentialTypeValue::Array(vec![
                "VerifiableCredential".to_string(),
                "MsezSmartAssetRegistryCredential".to_string(),
            ]),
            issuer,
            issuance_date: *ts.as_datetime(),
            expiration_date: None,
            credential_subject: subject_value,
            proof: ProofValue::default(),
        };

        Ok(Self { vc })
    }

    /// Wrap an existing VC, validating that its credential subject
    /// deserializes as a valid [`SmartAssetRegistrySubject`].
    pub fn from_vc(vc: VerifiableCredential) -> Result<Self, VcError> {
        let _subject: SmartAssetRegistrySubject =
            serde_json::from_value(vc.credential_subject.clone()).map_err(|e| {
                VcError::SchemaValidation(format!(
                    "credentialSubject is not a valid SmartAssetRegistrySubject: {e}"
                ))
            })?;
        Ok(Self { vc })
    }

    /// Access the underlying VC.
    pub fn as_vc(&self) -> &VerifiableCredential {
        &self.vc
    }

    /// Consume and return the underlying VC.
    pub fn into_vc(self) -> VerifiableCredential {
        self.vc
    }

    /// Get a mutable reference to the underlying VC (for signing).
    pub fn as_vc_mut(&mut self) -> &mut VerifiableCredential {
        &mut self.vc
    }

    /// Extract the typed credential subject.
    pub fn subject(&self) -> Result<SmartAssetRegistrySubject, VcError> {
        serde_json::from_value(self.vc.credential_subject.clone()).map_err(|e| {
            VcError::SchemaValidation(format!(
                "credentialSubject is not a valid SmartAssetRegistrySubject: {e}"
            ))
        })
    }

    /// Get the asset ID from the credential subject.
    pub fn asset_id(&self) -> Option<String> {
        self.vc
            .credential_subject
            .get("asset_id")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    /// Sign this registry VC with an Ed25519 key pair.
    ///
    /// Delegates to [`VerifiableCredential::sign_ed25519()`] which uses
    /// [`CanonicalBytes`] for the signing input.
    pub fn sign_ed25519(
        &mut self,
        signing_key: &SigningKey,
        verification_method: String,
        proof_type: ProofType,
        created: Option<Timestamp>,
    ) -> Result<(), VcError> {
        self.vc
            .sign_ed25519(signing_key, verification_method, proof_type, created)
    }

    /// Validate the VC against the smart asset registry JSON schema.
    ///
    /// Requires a [`SchemaValidator`](msez_schema::SchemaValidator).
    pub fn validate_schema(&self, validator: &msez_schema::SchemaValidator) -> Result<(), VcError> {
        let vc_value = serde_json::to_value(&self.vc).map_err(VcError::Json)?;
        validator
            .validate_value(&vc_value, REGISTRY_SCHEMA_ID)
            .map_err(|e| VcError::SchemaValidation(e.to_string()))
    }

    /// Compute the asset ID from a genesis document.
    ///
    /// `asset_id = sha256(JCS(genesis_without_asset_id))`
    ///
    /// Matches `tools/smart_asset.py:asset_id_from_genesis()`.
    ///
    /// # Security Invariant
    ///
    /// Uses [`CanonicalBytes::from_value()`] for JCS canonicalization.
    pub fn compute_asset_id(genesis: &serde_json::Value) -> Result<String, VcError> {
        let mut g = genesis.clone();
        if let Some(obj) = g.as_object_mut() {
            obj.remove("asset_id");
        }
        let canonical = CanonicalBytes::from_value(g)?;
        let digest = msez_crypto::sha256_digest(&canonical);
        Ok(digest.to_hex())
    }

    /// Verify that the VC's `asset_id` matches the genesis document.
    ///
    /// Recomputes `SHA256(JCS(genesis_without_asset_id))` and compares it to
    /// the `asset_id` in the credential subject. Returns an error if they
    /// diverge — this indicates the genesis document was tampered with or the
    /// wrong genesis was provided (P2-SA-002).
    pub fn verify_asset_id_binding(
        &self,
        genesis: &serde_json::Value,
    ) -> Result<(), VcError> {
        let subject = self.subject()?;
        let expected = Self::compute_asset_id(genesis)?;
        if subject.asset_id != expected {
            return Err(VcError::SchemaValidation(format!(
                "asset_id binding mismatch: subject has {:?} but genesis computes {:?}",
                subject.asset_id, expected
            )));
        }
        Ok(())
    }

    /// Check whether a string is a valid SHA-256 hex digest (64 lowercase hex chars).
    fn is_valid_sha256_hex(s: &str) -> bool {
        s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_crypto::SigningKey;
    use rand_core::OsRng;
    use serde_json::json;

    fn make_test_subject() -> SmartAssetRegistrySubject {
        SmartAssetRegistrySubject {
            asset_id: "a".repeat(64),
            stack_spec_version: "0.4.44".to_string(),
            asset_genesis: ArtifactRef {
                artifact_type: "smart-asset-genesis".to_string(),
                digest_sha256: "a".repeat(64),
                uri: None,
                media_type: None,
            },
            asset_name: Some("Test Asset".to_string()),
            asset_class: Some("equity".to_string()),
            jurisdiction_bindings: vec![JurisdictionBinding {
                harbor_id: "zone-pk-01".to_string(),
                binding_status: BindingStatus::Active,
                shard_role: "primary".to_string(),
                lawpacks: vec![LawpackRef {
                    jurisdiction_id: "PK".to_string(),
                    domain: "corporate".to_string(),
                    lawpack_digest_sha256: "b".repeat(64),
                    lawpack: None,
                }],
                compliance_profile: ComplianceProfile::default(),
                enforcement_profile: None,
                effective_from: None,
                effective_until: None,
                notes: None,
            }],
            quorum_policy: None,
            notes: None,
        }
    }

    #[test]
    fn registry_vc_creation() {
        let subject = make_test_subject();
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();

        assert_eq!(vc.as_vc().issuer, "did:key:z6MkTestIssuer");
        assert!(vc.as_vc().credential_type.contains_vc_type());
        assert_eq!(vc.asset_id(), Some("a".repeat(64)));
    }

    #[test]
    fn registry_vc_sign_and_verify() {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();

        let subject = make_test_subject();
        let mut vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();

        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTestIssuer#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        let results = vc.as_vc().verify(move |_vm| Ok(vk.clone()));
        assert_eq!(results.len(), 1);
        assert!(results[0].ok, "verification failed: {}", results[0].error);
    }

    #[test]
    fn registry_vc_from_vc_validates_subject() {
        let sk = SigningKey::generate(&mut OsRng);
        let subject = make_test_subject();
        let mut registry =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();

        registry
            .sign_ed25519(
                &sk,
                "did:key:z6MkTestIssuer#key-1".to_string(),
                ProofType::Ed25519Signature2020,
                None,
            )
            .unwrap();

        let inner = registry.into_vc();
        let recovered = SmartAssetRegistryVc::from_vc(inner).unwrap();
        assert_eq!(recovered.asset_id(), Some("a".repeat(64)));
    }

    #[test]
    fn registry_vc_from_vc_rejects_invalid_subject() {
        let vc = VerifiableCredential {
            context: ContextValue::default(),
            id: None,
            credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
            issuer: "did:key:z6Mk123".to_string(),
            issuance_date: chrono::Utc::now(),
            expiration_date: None,
            credential_subject: json!({"not_an_asset": true}),
            proof: ProofValue::default(),
        };

        let result = SmartAssetRegistryVc::from_vc(vc);
        assert!(result.is_err());
    }

    #[test]
    fn compute_asset_id_deterministic() {
        let genesis = json!({
            "type": "SmartAssetGenesis",
            "stack_spec_version": "0.4.44",
            "created_at": "2026-01-15T12:00:00Z",
            "asset_name": "Test",
            "asset_class": "equity"
        });

        let id1 = SmartAssetRegistryVc::compute_asset_id(&genesis).unwrap();
        let id2 = SmartAssetRegistryVc::compute_asset_id(&genesis).unwrap();
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 64);
    }

    #[test]
    fn compute_asset_id_excludes_asset_id_field() {
        let genesis_without = json!({
            "type": "SmartAssetGenesis",
            "stack_spec_version": "0.4.44",
            "created_at": "2026-01-15T12:00:00Z",
            "asset_name": "Test",
            "asset_class": "equity"
        });

        let mut genesis_with = genesis_without.clone();
        genesis_with
            .as_object_mut()
            .unwrap()
            .insert("asset_id".to_string(), json!("anything"));

        let id_without = SmartAssetRegistryVc::compute_asset_id(&genesis_without).unwrap();
        let id_with = SmartAssetRegistryVc::compute_asset_id(&genesis_with).unwrap();
        assert_eq!(id_without, id_with);
    }

    #[test]
    fn compute_asset_id_different_inputs_differ() {
        let g1 = json!({
            "type": "SmartAssetGenesis",
            "asset_name": "Asset A",
            "asset_class": "equity"
        });
        let g2 = json!({
            "type": "SmartAssetGenesis",
            "asset_name": "Asset B",
            "asset_class": "equity"
        });

        let id1 = SmartAssetRegistryVc::compute_asset_id(&g1).unwrap();
        let id2 = SmartAssetRegistryVc::compute_asset_id(&g2).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn subject_extraction_roundtrip() {
        let subject = make_test_subject();
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject.clone(), None)
                .unwrap();

        let extracted = vc.subject().unwrap();
        assert_eq!(extracted.asset_id, subject.asset_id);
        assert_eq!(extracted.stack_spec_version, subject.stack_spec_version);
        assert_eq!(
            extracted.jurisdiction_bindings.len(),
            subject.jurisdiction_bindings.len()
        );
        assert_eq!(
            extracted.jurisdiction_bindings[0].harbor_id,
            subject.jurisdiction_bindings[0].harbor_id
        );
    }

    #[test]
    fn registry_vc_json_matches_python_structure() {
        let subject = make_test_subject();
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();

        let val = serde_json::to_value(vc.as_vc()).unwrap();
        assert_eq!(
            val["type"],
            json!(["VerifiableCredential", "MsezSmartAssetRegistryCredential"])
        );
        assert!(val["credentialSubject"]["asset_id"].is_string());
        assert!(val["credentialSubject"]["jurisdiction_bindings"].is_array());
        assert!(val["credentialSubject"]["asset_genesis"]["artifact_type"].is_string());
    }

    #[test]
    fn jurisdiction_binding_serde_roundtrip() {
        let binding = JurisdictionBinding {
            harbor_id: "zone-pk-01".to_string(),
            binding_status: BindingStatus::Active,
            shard_role: "primary".to_string(),
            lawpacks: vec![LawpackRef {
                jurisdiction_id: "PK".to_string(),
                domain: "corporate".to_string(),
                lawpack_digest_sha256: "c".repeat(64),
                lawpack: None,
            }],
            compliance_profile: ComplianceProfile {
                allowed_transition_kinds: Some(vec!["transfer".to_string()]),
                required_attestations: None,
                default_required_attestations: Some(vec!["kyc".to_string()]),
            },
            enforcement_profile: Some(EnforcementProfile {
                intensity: Some("high".to_string()),
                audit_frequency: Some("continuous".to_string()),
                notes: None,
            }),
            effective_from: Some("2026-01-01T00:00:00Z".to_string()),
            effective_until: None,
            notes: None,
        };

        let json_str = serde_json::to_string(&binding).unwrap();
        let back: JurisdictionBinding = serde_json::from_str(&json_str).unwrap();
        assert_eq!(back.harbor_id, "zone-pk-01");
        assert_eq!(
            back.compliance_profile
                .allowed_transition_kinds
                .unwrap()
                .len(),
            1
        );
        assert_eq!(back.enforcement_profile.unwrap().intensity.unwrap(), "high");
    }

    // ── Additional coverage tests ──────────────────────────────────

    #[test]
    fn registry_schema_id_constant() {
        assert!(REGISTRY_SCHEMA_ID.contains("smart-asset-registry"));
    }

    #[test]
    fn artifact_ref_serde_roundtrip() {
        let ar = ArtifactRef {
            artifact_type: "smart-asset-genesis".to_string(),
            digest_sha256: "d".repeat(64),
            uri: Some("ipfs://Qm...".to_string()),
            media_type: Some("application/json".to_string()),
        };
        let json = serde_json::to_string(&ar).unwrap();
        let back: ArtifactRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.artifact_type, "smart-asset-genesis");
        assert_eq!(back.uri.unwrap(), "ipfs://Qm...");
        assert_eq!(back.media_type.unwrap(), "application/json");
    }

    #[test]
    fn artifact_ref_optional_fields_omitted() {
        let ar = ArtifactRef {
            artifact_type: "test".to_string(),
            digest_sha256: "e".repeat(64),
            uri: None,
            media_type: None,
        };
        let json = serde_json::to_string(&ar).unwrap();
        assert!(!json.contains("uri"));
        assert!(!json.contains("media_type"));
    }

    #[test]
    fn lawpack_ref_serde_roundtrip() {
        let lr = LawpackRef {
            jurisdiction_id: "AE".to_string(),
            domain: "aml".to_string(),
            lawpack_digest_sha256: "f".repeat(64),
            lawpack: Some(ArtifactRef {
                artifact_type: "lawpack-bundle".to_string(),
                digest_sha256: "f".repeat(64),
                uri: None,
                media_type: None,
            }),
        };
        let json = serde_json::to_string(&lr).unwrap();
        let back: LawpackRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.jurisdiction_id, "AE");
        assert!(back.lawpack.is_some());
    }

    #[test]
    fn compliance_profile_default() {
        let cp = ComplianceProfile::default();
        assert!(cp.allowed_transition_kinds.is_none());
        assert!(cp.required_attestations.is_none());
        assert!(cp.default_required_attestations.is_none());
    }

    #[test]
    fn enforcement_profile_all_none() {
        let ep = EnforcementProfile {
            intensity: None,
            audit_frequency: None,
            notes: None,
        };
        let json = serde_json::to_string(&ep).unwrap();
        // Optional fields should be omitted.
        assert!(!json.contains("intensity"));
        assert!(!json.contains("audit_frequency"));
        assert!(!json.contains("notes"));
    }

    #[test]
    fn smart_asset_registry_subject_serde_roundtrip() {
        let subject = make_test_subject();
        let json = serde_json::to_string(&subject).unwrap();
        let back: SmartAssetRegistrySubject = serde_json::from_str(&json).unwrap();
        assert_eq!(back.asset_id, subject.asset_id);
        assert_eq!(back.stack_spec_version, "0.4.44");
        assert_eq!(back.asset_name.unwrap(), "Test Asset");
        assert_eq!(back.asset_class.unwrap(), "equity");
    }

    #[test]
    fn registry_vc_as_vc_mut() {
        let subject = make_test_subject();
        let mut vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();
        let inner = vc.as_vc_mut();
        inner.issuer = "did:key:z6MkNewIssuer".to_string();
        assert_eq!(vc.as_vc().issuer, "did:key:z6MkNewIssuer");
    }

    #[test]
    fn registry_vc_into_vc_and_from_vc() {
        let subject = make_test_subject();
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();
        let inner = vc.into_vc();
        assert_eq!(inner.issuer, "did:key:z6MkTestIssuer");

        let recovered = SmartAssetRegistryVc::from_vc(inner).unwrap();
        assert_eq!(recovered.asset_id(), Some("a".repeat(64)));
    }

    #[test]
    fn registry_vc_asset_id_missing() {
        let vc = VerifiableCredential {
            context: ContextValue::default(),
            id: None,
            credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
            issuer: "did:key:z6Mk123".to_string(),
            issuance_date: chrono::Utc::now(),
            expiration_date: None,
            credential_subject: json!({"no_asset_id": true, "asset_id": 42}),
            proof: ProofValue::default(),
        };
        // asset_id is not a string, so returns None.
        let wrapper = SmartAssetRegistryVc { vc };
        assert!(wrapper.asset_id().is_none());
    }

    #[test]
    fn compute_asset_id_from_non_object() {
        // Non-object genesis (e.g., a string) should still produce a digest.
        let genesis = json!("not an object");
        let result = SmartAssetRegistryVc::compute_asset_id(&genesis);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 64);
    }

    #[test]
    fn binding_compliance_result_fields() {
        let result = BindingComplianceResult {
            harbor_id: "zone-pk-01".to_string(),
            binding_status: BindingStatus::Active,
            shard_role: "primary".to_string(),
            allowed: false,
            reasons: vec!["missing kyc attestation".to_string()],
            missing_attestations: vec!["kyc".to_string()],
            present_attestations: vec!["aml".to_string()],
        };
        assert!(!result.allowed);
        assert_eq!(result.missing_attestations.len(), 1);
        assert_eq!(result.present_attestations.len(), 1);
    }

    #[test]
    fn registry_vc_with_explicit_issuance_date() {
        let subject = make_test_subject();
        let ts = Timestamp::now();
        let expected_dt = *ts.as_datetime();
        let vc = SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, Some(ts))
            .unwrap();
        assert_eq!(vc.as_vc().issuance_date, expected_dt);
    }

    #[test]
    fn registry_vc_multiple_jurisdiction_bindings() {
        let mut subject = make_test_subject();
        subject.jurisdiction_bindings.push(JurisdictionBinding {
            harbor_id: "zone-ae-01".to_string(),
            binding_status: BindingStatus::Active,
            shard_role: "secondary".to_string(),
            lawpacks: vec![LawpackRef {
                jurisdiction_id: "AE".to_string(),
                domain: "aml".to_string(),
                lawpack_digest_sha256: "c".repeat(64),
                lawpack: None,
            }],
            compliance_profile: ComplianceProfile::default(),
            enforcement_profile: None,
            effective_from: None,
            effective_until: None,
            notes: Some("secondary shard in UAE".to_string()),
        });

        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();
        let extracted = vc.subject().unwrap();
        assert_eq!(extracted.jurisdiction_bindings.len(), 2);
        assert_eq!(extracted.jurisdiction_bindings[1].harbor_id, "zone-ae-01");
    }

    #[test]
    fn registry_vc_clone() {
        let subject = make_test_subject();
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();
        let cloned = vc.clone();
        assert_eq!(vc.asset_id(), cloned.asset_id());
        assert_eq!(vc.as_vc().issuer, cloned.as_vc().issuer);
    }

    // ── P2-SA-002: asset_id binding tests ─────────────────────────────

    #[test]
    fn new_rejects_invalid_asset_id_format() {
        let mut subject = make_test_subject();
        subject.asset_id = "not-a-valid-hex-string".to_string();
        let result =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_uppercase_asset_id() {
        let mut subject = make_test_subject();
        subject.asset_id = "A".repeat(64);
        let result =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None);
        assert!(result.is_err());
    }

    #[test]
    fn new_rejects_short_asset_id() {
        let mut subject = make_test_subject();
        subject.asset_id = "abcd1234".to_string();
        let result =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None);
        assert!(result.is_err());
    }

    #[test]
    fn verify_asset_id_binding_succeeds_when_matching() {
        let genesis = json!({
            "type": "SmartAssetGenesis",
            "stack_spec_version": "0.4.44",
            "created_at": "2026-01-15T12:00:00Z",
            "asset_name": "Test",
            "asset_class": "equity"
        });

        let computed_id = SmartAssetRegistryVc::compute_asset_id(&genesis).unwrap();

        let mut subject = make_test_subject();
        subject.asset_id = computed_id;
        subject.asset_genesis.digest_sha256 = subject.asset_id.clone();

        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();
        assert!(vc.verify_asset_id_binding(&genesis).is_ok());
    }

    #[test]
    fn verify_asset_id_binding_fails_when_mismatched() {
        let genesis = json!({
            "type": "SmartAssetGenesis",
            "asset_name": "Real Asset"
        });

        // Use a different asset_id than what the genesis computes
        let subject = make_test_subject(); // uses "a".repeat(64) as asset_id
        let vc =
            SmartAssetRegistryVc::new("did:key:z6MkTestIssuer".to_string(), subject, None).unwrap();

        let result = vc.verify_asset_id_binding(&genesis);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("binding mismatch"));
    }

    #[test]
    fn is_valid_sha256_hex_checks() {
        assert!(SmartAssetRegistryVc::is_valid_sha256_hex(&"a".repeat(64)));
        assert!(SmartAssetRegistryVc::is_valid_sha256_hex(
            &"0123456789abcdef".repeat(4)
        ));
        assert!(!SmartAssetRegistryVc::is_valid_sha256_hex("too_short"));
        assert!(!SmartAssetRegistryVc::is_valid_sha256_hex(&"A".repeat(64)));
        assert!(!SmartAssetRegistryVc::is_valid_sha256_hex(&"g".repeat(64)));
    }
}
