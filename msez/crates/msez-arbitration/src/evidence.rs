//! # Evidence Package Management
//!
//! Content-addressed evidence packages with chain-of-custody tracking.
//! Each evidence item is canonicalized via [`CanonicalBytes`] and assigned
//! a [`ContentDigest`] for integrity verification.
//!
//! ## Security Invariant
//!
//! Evidence items are immutable after creation. Their content digests are
//! computed via `CanonicalBytes::new()` → `sha256_digest()`, ensuring the
//! same canonicalization pipeline used by the entire stack. Integrity
//! verification recomputes the digest and compares against the stored value.
//!
//! ## Spec Reference
//!
//! Implements Definition 26.5 (Evidence Management) from the specification.
//! Evidence types and authenticity attestation types match the Python
//! `tools/arbitration.py` evidence handling.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use msez_core::{sha256_digest, CanonicalBytes, ContentDigest, Did, Timestamp};

use crate::dispute::DisputeId;
use crate::error::ArbitrationError;

// ── Identifiers ────────────────────────────────────────────────────────

/// A unique identifier for an evidence package.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidencePackageId(Uuid);

impl EvidencePackageId {
    /// Create a new random evidence package identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EvidencePackageId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EvidencePackageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evidence-pkg:{}", self.0)
    }
}

/// A unique identifier for an individual evidence item.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvidenceItemId(Uuid);

impl EvidenceItemId {
    /// Create a new random evidence item identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID.
    pub fn from_uuid(id: Uuid) -> Self {
        Self(id)
    }

    /// Access the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for EvidenceItemId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EvidenceItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evidence-item:{}", self.0)
    }
}

// ── Evidence Types ─────────────────────────────────────────────────────

/// Categories of evidence that can be submitted in arbitration proceedings.
///
/// Matches the 10 evidence types defined in Definition 26.5 of the
/// specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceType {
    /// Corridor receipts proving smart asset states.
    SmartAssetReceipt,
    /// Cross-jurisdiction corridor transaction receipts.
    CorridorReceipt,
    /// Compliance tensor evaluations, domain attestations.
    ComplianceEvidence,
    /// Professional analysis (valuations, inspections).
    ExpertReport,
    /// Testimonial evidence from witnesses.
    WitnessStatement,
    /// Underlying contract, amendments, BOL.
    ContractDocument,
    /// Email, messaging chains.
    CommunicationRecord,
    /// Bank transfers, SWIFT docs, settlement proofs.
    PaymentRecord,
    /// Bills of lading, waybills, tracking data.
    ShippingDocument,
    /// Quality, customs, or insurance inspection reports.
    InspectionReport,
}

impl EvidenceType {
    /// All evidence types as a slice.
    pub fn all() -> &'static [EvidenceType] {
        &[
            Self::SmartAssetReceipt,
            Self::CorridorReceipt,
            Self::ComplianceEvidence,
            Self::ExpertReport,
            Self::WitnessStatement,
            Self::ContractDocument,
            Self::CommunicationRecord,
            Self::PaymentRecord,
            Self::ShippingDocument,
            Self::InspectionReport,
        ]
    }
}

impl std::fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::SmartAssetReceipt => "smart_asset_receipt",
            Self::CorridorReceipt => "corridor_receipt",
            Self::ComplianceEvidence => "compliance_evidence",
            Self::ExpertReport => "expert_report",
            Self::WitnessStatement => "witness_statement",
            Self::ContractDocument => "contract_document",
            Self::CommunicationRecord => "communication_record",
            Self::PaymentRecord => "payment_record",
            Self::ShippingDocument => "shipping_document",
            Self::InspectionReport => "inspection_report",
        };
        write!(f, "{s}")
    }
}

// ── Authenticity Attestation ───────────────────────────────────────────

/// Types of authenticity attestation for evidence items.
///
/// Matches the 5 authenticity types in Definition 26.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuthenticityType {
    /// Evidence proven to be in a corridor checkpoint (MMR inclusion proof).
    CorridorCheckpointInclusion,
    /// Evidence in a smart asset state root.
    SmartAssetCheckpointInclusion,
    /// Third-party notarization certificate.
    NotarizedDocument,
    /// Professional credentials/certification.
    ExpertCertification,
    /// Document custody trail from origin to submission.
    ChainOfCustody,
}

impl std::fmt::Display for AuthenticityType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::CorridorCheckpointInclusion => "corridor_checkpoint_inclusion",
            Self::SmartAssetCheckpointInclusion => "smart_asset_checkpoint_inclusion",
            Self::NotarizedDocument => "notarized_document",
            Self::ExpertCertification => "expert_certification",
            Self::ChainOfCustody => "chain_of_custody",
        };
        write!(f, "{s}")
    }
}

/// An attestation of authenticity for an evidence item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticityAttestation {
    /// The type of authenticity proof.
    pub attestation_type: AuthenticityType,
    /// Digest of the attestation proof document.
    pub proof_digest: ContentDigest,
    /// Who provided the attestation (DID of the attester).
    pub attester: Did,
    /// When the attestation was made.
    pub attested_at: Timestamp,
}

// ── Chain of Custody ───────────────────────────────────────────────────

/// A single entry in the chain of custody for an evidence item.
///
/// Records each custodial transfer of an evidence artifact from one
/// party to another, providing a tamper-evident provenance trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainOfCustodyEntry {
    /// DID of the party receiving custody.
    pub custodian: Did,
    /// When custody was transferred.
    pub transferred_at: Timestamp,
    /// Digest of the evidence at the point of transfer (for integrity).
    pub evidence_digest_at_transfer: ContentDigest,
    /// Description of the transfer event.
    pub description: String,
}

// ── Evidence Item ──────────────────────────────────────────────────────

/// A single piece of evidence with content-addressed integrity.
///
/// Each item is assigned a digest at creation time via canonical
/// serialization. The digest can be reverified at any point to detect
/// tampering.
///
/// ## Security Invariant
///
/// The `digest` field is computed from the item's content via
/// `CanonicalBytes::new()` → `sha256_digest()`. Any modification to the
/// content will produce a different digest, detectable via
/// [`EvidencePackage::verify_package_integrity`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Unique evidence item identifier.
    pub id: EvidenceItemId,
    /// Category of the evidence.
    pub evidence_type: EvidenceType,
    /// Human-readable title.
    pub title: String,
    /// Description of the evidence content.
    pub description: String,
    /// Content digest computed via canonical serialization.
    pub digest: ContentDigest,
    /// DID of the party that submitted this item.
    pub submitted_by: Did,
    /// When the item was submitted.
    pub submitted_at: Timestamp,
    /// Authenticity attestations for this item.
    pub authenticity: Vec<AuthenticityAttestation>,
    /// Chain of custody entries.
    pub chain_of_custody: Vec<ChainOfCustodyEntry>,
}

impl EvidenceItem {
    /// Create a new evidence item from serializable content.
    ///
    /// The content is canonicalized and its SHA-256 digest is stored for
    /// later integrity verification.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::Canonicalization`] if the content cannot
    /// be canonicalized (e.g., contains float values).
    pub fn new(
        evidence_type: EvidenceType,
        title: String,
        description: String,
        content: &impl serde::Serialize,
        submitted_by: Did,
    ) -> Result<Self, ArbitrationError> {
        let canonical = CanonicalBytes::new(content)?;
        let digest = sha256_digest(&canonical);
        let now = Timestamp::now();
        Ok(Self {
            id: EvidenceItemId::new(),
            evidence_type,
            title,
            description,
            digest: digest.clone(),
            submitted_by: submitted_by.clone(),
            submitted_at: now.clone(),
            authenticity: Vec::new(),
            chain_of_custody: vec![ChainOfCustodyEntry {
                custodian: submitted_by,
                transferred_at: now,
                evidence_digest_at_transfer: digest,
                description: "Initial submission".to_string(),
            }],
        })
    }

    /// Verify the integrity of this evidence item against provided content.
    ///
    /// Recomputes the digest from the content and compares against the
    /// stored digest.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EvidenceIntegrityViolation`] if the
    /// recomputed digest does not match.
    pub fn verify_integrity(
        &self,
        content: &impl serde::Serialize,
    ) -> Result<(), ArbitrationError> {
        let canonical = CanonicalBytes::new(content)?;
        let recomputed = sha256_digest(&canonical);
        if recomputed != self.digest {
            return Err(ArbitrationError::EvidenceIntegrityViolation {
                evidence_id: self.id.to_string(),
                expected: self.digest.to_hex(),
                actual: recomputed.to_hex(),
            });
        }
        Ok(())
    }

    /// Add an authenticity attestation to this evidence item.
    pub fn add_attestation(&mut self, attestation: AuthenticityAttestation) {
        self.authenticity.push(attestation);
    }

    /// Record a chain-of-custody transfer.
    pub fn transfer_custody(&mut self, entry: ChainOfCustodyEntry) {
        self.chain_of_custody.push(entry);
    }
}

// ── Evidence Package ───────────────────────────────────────────────────

/// A collection of evidence items submitted for a dispute proceeding.
///
/// Evidence packages are tied to a specific dispute and submitting party.
/// The package itself is content-addressed — its digest is computed from
/// the digests of all contained items.
///
/// ## Security Invariant
///
/// The package digest is the hash of the sorted concatenation of all item
/// digests. Adding, removing, or modifying any item changes the package
/// digest. This ensures the entire evidence bundle is integrity-protected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidencePackage {
    /// Unique package identifier.
    pub id: EvidencePackageId,
    /// The dispute this evidence package belongs to.
    pub dispute_id: DisputeId,
    /// DID of the party that submitted this package.
    pub submitted_by: Did,
    /// When the package was submitted.
    pub submitted_at: DateTime<Utc>,
    /// Evidence items in this package.
    pub items: Vec<EvidenceItem>,
    /// Content digest of the entire package (computed from item digests).
    pub package_digest: ContentDigest,
}

impl EvidencePackage {
    /// Create a new evidence package from a collection of evidence items.
    ///
    /// The package digest is computed from all item digests, providing
    /// integrity over the entire bundle.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::Canonicalization`] if the digest
    /// computation fails.
    pub fn new(
        dispute_id: DisputeId,
        submitted_by: Did,
        items: Vec<EvidenceItem>,
    ) -> Result<Self, ArbitrationError> {
        let package_digest = compute_package_digest(&items)?;
        Ok(Self {
            id: EvidencePackageId::new(),
            dispute_id,
            submitted_by,
            submitted_at: Utc::now(),
            items,
            package_digest,
        })
    }

    /// Add an evidence item to the package and recompute the package digest.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::Canonicalization`] if the digest
    /// recomputation fails.
    pub fn add_item(&mut self, item: EvidenceItem) -> Result<(), ArbitrationError> {
        self.items.push(item);
        self.package_digest = compute_package_digest(&self.items)?;
        Ok(())
    }

    /// Retrieve an evidence item by its identifier.
    pub fn get_item(&self, item_id: &EvidenceItemId) -> Option<&EvidenceItem> {
        self.items.iter().find(|item| &item.id == item_id)
    }

    /// Verify the integrity of the entire evidence package.
    ///
    /// Recomputes the package digest from all item digests and compares
    /// against the stored value.
    ///
    /// # Errors
    ///
    /// Returns [`ArbitrationError::EvidenceIntegrityViolation`] if the
    /// package digest does not match.
    pub fn verify_package_integrity(&self) -> Result<(), ArbitrationError> {
        let recomputed = compute_package_digest(&self.items)?;
        if recomputed != self.package_digest {
            return Err(ArbitrationError::EvidenceIntegrityViolation {
                evidence_id: self.id.to_string(),
                expected: self.package_digest.to_hex(),
                actual: recomputed.to_hex(),
            });
        }
        Ok(())
    }

    /// Return the number of evidence items in the package.
    pub fn item_count(&self) -> usize {
        self.items.len()
    }
}

/// Compute the package-level digest from individual item digests.
///
/// The digest is computed by sorting item digest hex strings lexicographically,
/// concatenating them, and hashing the result. This ensures deterministic
/// package digests regardless of item insertion order.
fn compute_package_digest(items: &[EvidenceItem]) -> Result<ContentDigest, ArbitrationError> {
    let mut digests: Vec<String> = items.iter().map(|item| item.digest.to_hex()).collect();
    digests.sort();
    let concatenated = digests.join(",");
    let canonical = CanonicalBytes::new(&concatenated)?;
    Ok(sha256_digest(&canonical))
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest as core_sha256, CanonicalBytes};
    use serde_json::json;

    fn test_did(name: &str) -> Did {
        Did::new(format!("did:key:z6Mk{name}")).unwrap()
    }

    fn test_digest() -> ContentDigest {
        let canonical = CanonicalBytes::new(&json!({"test": "evidence"})).unwrap();
        core_sha256(&canonical)
    }

    #[test]
    fn evidence_item_creation_computes_digest() {
        let content = json!({"contract_id": "C-2026-001", "amount": "150000", "currency": "USD"});
        let item = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Purchase Agreement".to_string(),
            "Original contract between parties".to_string(),
            &content,
            test_did("Submitter123"),
        )
        .unwrap();

        assert_eq!(item.evidence_type, EvidenceType::ContractDocument);
        assert_eq!(item.digest.to_hex().len(), 64);
        assert_eq!(item.chain_of_custody.len(), 1);
    }

    #[test]
    fn evidence_item_integrity_verification_passes() {
        let content = json!({"key": "value", "n": 42});
        let item = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Payment proof".to_string(),
            "Bank transfer confirmation".to_string(),
            &content,
            test_did("Submitter123"),
        )
        .unwrap();

        assert!(item.verify_integrity(&content).is_ok());
    }

    #[test]
    fn evidence_item_integrity_verification_fails_on_tamper() {
        let content = json!({"key": "value", "n": 42});
        let item = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Payment proof".to_string(),
            "Bank transfer confirmation".to_string(),
            &content,
            test_did("Submitter123"),
        )
        .unwrap();

        let tampered = json!({"key": "value", "n": 43});
        assert!(item.verify_integrity(&tampered).is_err());
    }

    #[test]
    fn evidence_package_creation() {
        let content1 = json!({"doc": "contract"});
        let content2 = json!({"doc": "receipt"});

        let item1 = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Contract".to_string(),
            "Main contract".to_string(),
            &content1,
            test_did("Party1"),
        )
        .unwrap();

        let item2 = EvidenceItem::new(
            EvidenceType::CorridorReceipt,
            "Receipt".to_string(),
            "Transaction receipt".to_string(),
            &content2,
            test_did("Party1"),
        )
        .unwrap();

        let dispute_id = crate::dispute::DisputeId::new();
        let package =
            EvidencePackage::new(dispute_id, test_did("Party1"), vec![item1, item2]).unwrap();

        assert_eq!(package.item_count(), 2);
        assert_eq!(package.package_digest.to_hex().len(), 64);
    }

    #[test]
    fn evidence_package_integrity_passes() {
        let content = json!({"evidence": "data"});
        let item = EvidenceItem::new(
            EvidenceType::ExpertReport,
            "Expert analysis".to_string(),
            "Valuation report".to_string(),
            &content,
            test_did("Expert1"),
        )
        .unwrap();

        let dispute_id = crate::dispute::DisputeId::new();
        let package = EvidencePackage::new(dispute_id, test_did("Expert1"), vec![item]).unwrap();

        assert!(package.verify_package_integrity().is_ok());
    }

    #[test]
    fn evidence_package_add_item_updates_digest() {
        let content1 = json!({"doc": "first"});
        let item1 = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "First".to_string(),
            "First doc".to_string(),
            &content1,
            test_did("Party1"),
        )
        .unwrap();

        let dispute_id = crate::dispute::DisputeId::new();
        let mut package =
            EvidencePackage::new(dispute_id, test_did("Party1"), vec![item1]).unwrap();

        let original_digest = package.package_digest.clone();

        let content2 = json!({"doc": "second"});
        let item2 = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Second".to_string(),
            "Second doc".to_string(),
            &content2,
            test_did("Party1"),
        )
        .unwrap();

        package.add_item(item2).unwrap();
        assert_ne!(package.package_digest, original_digest);
        assert_eq!(package.item_count(), 2);
        assert!(package.verify_package_integrity().is_ok());
    }

    #[test]
    fn evidence_package_get_item() {
        let content = json!({"doc": "data"});
        let item = EvidenceItem::new(
            EvidenceType::WitnessStatement,
            "Witness".to_string(),
            "Witness testimony".to_string(),
            &content,
            test_did("Witness1"),
        )
        .unwrap();
        let item_id = item.id.clone();

        let dispute_id = crate::dispute::DisputeId::new();
        let package = EvidencePackage::new(dispute_id, test_did("Party1"), vec![item]).unwrap();

        assert!(package.get_item(&item_id).is_some());
        assert!(package.get_item(&EvidenceItemId::new()).is_none());
    }

    #[test]
    fn evidence_type_all_returns_ten() {
        assert_eq!(EvidenceType::all().len(), 10);
    }

    #[test]
    fn chain_of_custody_tracking() {
        let content = json!({"doc": "original"});
        let mut item = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Contract".to_string(),
            "Original contract".to_string(),
            &content,
            test_did("Originator1"),
        )
        .unwrap();

        assert_eq!(item.chain_of_custody.len(), 1);

        item.transfer_custody(ChainOfCustodyEntry {
            custodian: test_did("Custodian2"),
            transferred_at: Timestamp::now(),
            evidence_digest_at_transfer: item.digest.clone(),
            description: "Transfer to legal counsel".to_string(),
        });

        assert_eq!(item.chain_of_custody.len(), 2);
        assert_eq!(
            item.chain_of_custody[1].description,
            "Transfer to legal counsel"
        );
    }

    #[test]
    fn authenticity_attestation() {
        let content = json!({"doc": "notarized"});
        let mut item = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Notarized contract".to_string(),
            "Notarized copy".to_string(),
            &content,
            test_did("Submitter1"),
        )
        .unwrap();

        item.add_attestation(AuthenticityAttestation {
            attestation_type: AuthenticityType::NotarizedDocument,
            proof_digest: test_digest(),
            attester: test_did("Notary1"),
            attested_at: Timestamp::now(),
        });

        assert_eq!(item.authenticity.len(), 1);
        assert_eq!(
            item.authenticity[0].attestation_type,
            AuthenticityType::NotarizedDocument
        );
    }

    #[test]
    fn evidence_item_serialization_roundtrip() {
        let content = json!({"key": "value"});
        let item = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Payment".to_string(),
            "SWIFT confirmation".to_string(),
            &content,
            test_did("Party1"),
        )
        .unwrap();

        let json_str = serde_json::to_string(&item).unwrap();
        let deserialized: EvidenceItem = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.id, item.id);
        assert_eq!(deserialized.digest, item.digest);
        assert_eq!(deserialized.evidence_type, item.evidence_type);
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn evidence_integrity_recomputation_matches_original() {
        let content = json!({"amount": "50000", "currency": "PKR", "reference": "TX-2026-001"});
        let item = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Wire transfer".to_string(),
            "Payment confirmation".to_string(),
            &content,
            test_did("Bank1"),
        )
        .unwrap();

        // Manually recompute and verify the digest matches
        let canonical = CanonicalBytes::new(&content).unwrap();
        let recomputed = core_sha256(&canonical);
        assert_eq!(item.digest, recomputed);
    }

    #[test]
    fn evidence_integrity_fails_with_different_content() {
        let original = json!({"contract": "A", "value": 100});
        let item = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Contract A".to_string(),
            "Original contract".to_string(),
            &original,
            test_did("Submitter1"),
        )
        .unwrap();

        // Verify passes with original
        assert!(item.verify_integrity(&original).is_ok());

        // Verify fails with modified content
        let modified = json!({"contract": "A", "value": 200});
        let err = item.verify_integrity(&modified).unwrap_err();
        match err {
            ArbitrationError::EvidenceIntegrityViolation {
                evidence_id,
                expected,
                actual,
            } => {
                assert!(!evidence_id.is_empty());
                assert_ne!(expected, actual);
                assert_eq!(expected.len(), 64);
                assert_eq!(actual.len(), 64);
            }
            other => panic!("Expected EvidenceIntegrityViolation, got: {other:?}"),
        }
    }

    #[test]
    fn evidence_package_integrity_fails_after_tampering() {
        let content1 = json!({"doc": "evidence_1"});
        let item1 = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Contract".to_string(),
            "Main contract".to_string(),
            &content1,
            test_did("Party1"),
        )
        .unwrap();

        let dispute_id = crate::dispute::DisputeId::new();
        let mut package =
            EvidencePackage::new(dispute_id, test_did("Party1"), vec![item1]).unwrap();

        // Package integrity should pass initially
        assert!(package.verify_package_integrity().is_ok());

        // Tamper with the stored package digest
        let tampered_canonical = CanonicalBytes::new(&json!({"fake": "digest"})).unwrap();
        package.package_digest = core_sha256(&tampered_canonical);

        // Integrity check should now fail
        let err = package.verify_package_integrity().unwrap_err();
        assert!(matches!(
            err,
            ArbitrationError::EvidenceIntegrityViolation { .. }
        ));
    }

    #[test]
    fn evidence_type_display_all_variants() {
        assert_eq!(format!("{}", EvidenceType::SmartAssetReceipt), "smart_asset_receipt");
        assert_eq!(format!("{}", EvidenceType::CorridorReceipt), "corridor_receipt");
        assert_eq!(format!("{}", EvidenceType::ComplianceEvidence), "compliance_evidence");
        assert_eq!(format!("{}", EvidenceType::ExpertReport), "expert_report");
        assert_eq!(format!("{}", EvidenceType::WitnessStatement), "witness_statement");
        assert_eq!(format!("{}", EvidenceType::ContractDocument), "contract_document");
        assert_eq!(format!("{}", EvidenceType::CommunicationRecord), "communication_record");
        assert_eq!(format!("{}", EvidenceType::PaymentRecord), "payment_record");
        assert_eq!(format!("{}", EvidenceType::ShippingDocument), "shipping_document");
        assert_eq!(format!("{}", EvidenceType::InspectionReport), "inspection_report");
    }

    #[test]
    fn authenticity_type_display_all_variants() {
        assert_eq!(
            format!("{}", AuthenticityType::CorridorCheckpointInclusion),
            "corridor_checkpoint_inclusion"
        );
        assert_eq!(
            format!("{}", AuthenticityType::SmartAssetCheckpointInclusion),
            "smart_asset_checkpoint_inclusion"
        );
        assert_eq!(
            format!("{}", AuthenticityType::NotarizedDocument),
            "notarized_document"
        );
        assert_eq!(
            format!("{}", AuthenticityType::ExpertCertification),
            "expert_certification"
        );
        assert_eq!(
            format!("{}", AuthenticityType::ChainOfCustody),
            "chain_of_custody"
        );
    }

    #[test]
    fn evidence_package_id_display() {
        let id = EvidencePackageId::new();
        let display = format!("{id}");
        assert!(display.starts_with("evidence-pkg:"));
    }

    #[test]
    fn evidence_item_id_display() {
        let id = EvidenceItemId::new();
        let display = format!("{id}");
        assert!(display.starts_with("evidence-item:"));
    }

    #[test]
    fn evidence_package_id_default() {
        let id = EvidencePackageId::default();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn evidence_item_id_default() {
        let id = EvidenceItemId::default();
        assert!(!id.as_uuid().is_nil());
    }

    #[test]
    fn evidence_package_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EvidencePackageId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn evidence_item_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = EvidenceItemId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn evidence_package_digest_is_order_independent() {
        let content1 = json!({"doc": "alpha"});
        let content2 = json!({"doc": "beta"});

        let item1 = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Alpha".to_string(),
            "First".to_string(),
            &content1,
            test_did("Party1"),
        )
        .unwrap();

        let item2 = EvidenceItem::new(
            EvidenceType::PaymentRecord,
            "Beta".to_string(),
            "Second".to_string(),
            &content2,
            test_did("Party1"),
        )
        .unwrap();

        let dispute_id = crate::dispute::DisputeId::new();
        let pkg_a = EvidencePackage::new(
            dispute_id.clone(),
            test_did("Party1"),
            vec![item1.clone(), item2.clone()],
        )
        .unwrap();
        let pkg_b =
            EvidencePackage::new(dispute_id, test_did("Party1"), vec![item2, item1]).unwrap();

        // Package digest should be the same regardless of item order,
        // since compute_package_digest sorts the item digests.
        assert_eq!(pkg_a.package_digest, pkg_b.package_digest);
    }

    #[test]
    fn evidence_empty_package() {
        let dispute_id = crate::dispute::DisputeId::new();
        let package =
            EvidencePackage::new(dispute_id, test_did("Party1"), vec![]).unwrap();
        assert_eq!(package.item_count(), 0);
        assert!(package.verify_package_integrity().is_ok());
    }

    #[test]
    fn evidence_item_all_types_creatable() {
        for et in EvidenceType::all() {
            let content = json!({"type": format!("{et}")});
            let item = EvidenceItem::new(
                *et,
                format!("Test {et}"),
                "Description".to_string(),
                &content,
                test_did("Submitter1"),
            );
            assert!(item.is_ok(), "Failed to create evidence item of type {et}");
        }
    }

    #[test]
    fn multiple_authenticity_attestations() {
        let content = json!({"doc": "certified"});
        let mut item = EvidenceItem::new(
            EvidenceType::ContractDocument,
            "Certified document".to_string(),
            "A certified document".to_string(),
            &content,
            test_did("Submitter1"),
        )
        .unwrap();

        item.add_attestation(AuthenticityAttestation {
            attestation_type: AuthenticityType::NotarizedDocument,
            proof_digest: test_digest(),
            attester: test_did("Notary1"),
            attested_at: Timestamp::now(),
        });

        item.add_attestation(AuthenticityAttestation {
            attestation_type: AuthenticityType::ExpertCertification,
            proof_digest: test_digest(),
            attester: test_did("Expert1"),
            attested_at: Timestamp::now(),
        });

        item.add_attestation(AuthenticityAttestation {
            attestation_type: AuthenticityType::ChainOfCustody,
            proof_digest: test_digest(),
            attester: test_did("Custodian1"),
            attested_at: Timestamp::now(),
        });

        assert_eq!(item.authenticity.len(), 3);
        assert_eq!(item.authenticity[0].attestation_type, AuthenticityType::NotarizedDocument);
        assert_eq!(item.authenticity[1].attestation_type, AuthenticityType::ExpertCertification);
        assert_eq!(item.authenticity[2].attestation_type, AuthenticityType::ChainOfCustody);
    }

    #[test]
    fn chain_of_custody_multiple_transfers() {
        let content = json!({"doc": "tracked"});
        let mut item = EvidenceItem::new(
            EvidenceType::ShippingDocument,
            "Bill of lading".to_string(),
            "Shipping document".to_string(),
            &content,
            test_did("Shipper1"),
        )
        .unwrap();

        // Initial custody entry exists from creation
        assert_eq!(item.chain_of_custody.len(), 1);
        assert_eq!(item.chain_of_custody[0].description, "Initial submission");

        // Transfer to customs
        item.transfer_custody(ChainOfCustodyEntry {
            custodian: test_did("Customs1"),
            transferred_at: Timestamp::now(),
            evidence_digest_at_transfer: item.digest.clone(),
            description: "Transfer to customs authority".to_string(),
        });

        // Transfer to legal
        item.transfer_custody(ChainOfCustodyEntry {
            custodian: test_did("Legal1"),
            transferred_at: Timestamp::now(),
            evidence_digest_at_transfer: item.digest.clone(),
            description: "Transfer to legal counsel".to_string(),
        });

        assert_eq!(item.chain_of_custody.len(), 3);
        assert_eq!(item.chain_of_custody[1].description, "Transfer to customs authority");
        assert_eq!(item.chain_of_custody[2].description, "Transfer to legal counsel");
    }
}
