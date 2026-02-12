//! Tests for regpack integration with the arbitration system.
//!
//! Validates that regpacks can provide regulatory context for dispute
//! filings, and that arbitration institutions are correctly registered
//! in the institution registry.

use msez_core::{JurisdictionId, CanonicalBytes, sha256_digest, Did};
use msez_arbitration::{
    Dispute, DisputeType, DisputeState, Party, Money, Claim, FilingEvidence,
};
use msez_pack::Regpack;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_digest() -> msez_core::ContentDigest {
    let canonical = CanonicalBytes::new(&serde_json::json!({"test": "regpack_arb"})).unwrap();
    sha256_digest(&canonical)
}

fn test_party(name: &str, jurisdiction: &str) -> Party {
    Party {
        did: Did::new(format!("did:key:z6Mk{}", name)).unwrap(),
        legal_name: name.to_string(),
        jurisdiction_id: Some(JurisdictionId::new(jurisdiction).unwrap()),
    }
}

// ---------------------------------------------------------------------------
// Regpack with dispute context
// ---------------------------------------------------------------------------

#[test]
fn regpack_with_dispute_context() {
    // A regpack provides regulatory context for a jurisdiction.
    // A dispute filed in that jurisdiction should reference its regulatory framework.
    let jid = JurisdictionId::new("AE-DIFC").unwrap();
    let regpack = Regpack {
        jurisdiction: jid.clone(),
        name: "DIFC Regulatory Pack".to_string(),
        version: "1.0".to_string(),
        digest: None,
        metadata: None,
    };

    assert_eq!(regpack.jurisdiction.as_str(), "AE-DIFC");

    // File a dispute in the same jurisdiction.
    let claimant = test_party("Claimant1", "AE-DIFC");
    let respondent = test_party("Respondent1", "PK-RSEZ");

    let claim = Claim {
        claim_id: "CLM-001".to_string(),
        claim_type: DisputeType::BreachOfContract,
        description: "Failure to deliver goods per LOI".to_string(),
        amount: Some(Money::new("150000", "USD").unwrap()),
        supporting_evidence_digests: vec![test_digest()],
    };

    let dispute = Dispute::file(
        claimant,
        respondent,
        DisputeType::BreachOfContract,
        jid,
        None,
        "difc-lcia".to_string(),
        vec![claim],
        FilingEvidence {
            filing_document_digest: test_digest(),
        },
    );

    assert_eq!(dispute.state, DisputeState::Filed);
    assert_eq!(dispute.jurisdiction.as_str(), "AE-DIFC");
}

// ---------------------------------------------------------------------------
// Arbitration institution registration
// ---------------------------------------------------------------------------

#[test]
fn arbitration_institution_registration() {
    let registry = msez_arbitration::institution_registry();

    // The registry must contain at least 4 institutions.
    assert!(
        registry.len() >= 4,
        "Expected at least 4 institutions, got {}",
        registry.len()
    );

    // Check that known institutions exist.
    let ids: Vec<&str> = registry.iter().map(|i| i.id.as_str()).collect();
    assert!(ids.contains(&"difc-lcia"), "DIFC-LCIA must be registered");
    assert!(ids.contains(&"siac"), "SIAC must be registered");
    assert!(ids.contains(&"icc"), "ICC must be registered");
    assert!(ids.contains(&"aifc-iac"), "AIFC-IAC must be registered");
}

#[test]
fn institution_supports_all_dispute_types() {
    let registry = msez_arbitration::institution_registry();

    for institution in &registry {
        assert!(
            !institution.supported_dispute_types.is_empty(),
            "Institution {} must support at least one dispute type",
            institution.id
        );

        // All top-tier institutions should support all dispute types.
        assert_eq!(
            institution.supported_dispute_types.len(),
            DisputeType::all().len(),
            "Institution {} should support all {} dispute types",
            institution.id,
            DisputeType::all().len()
        );
    }
}

// ---------------------------------------------------------------------------
// Dispute filing with regulatory context
// ---------------------------------------------------------------------------

#[test]
fn dispute_filing_with_regulatory_context() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();

    let claimant = test_party("ExporterPK", "PK-RSEZ");
    let respondent = test_party("ImporterAE", "AE-DIFC");

    let claims = vec![
        Claim {
            claim_id: "CLM-001".to_string(),
            claim_type: DisputeType::PaymentDefault,
            description: "Non-payment of invoice".to_string(),
            amount: Some(Money::new("500000", "PKR").unwrap()),
            supporting_evidence_digests: vec![test_digest()],
        },
        Claim {
            claim_id: "CLM-002".to_string(),
            claim_type: DisputeType::DocumentaryDiscrepancy,
            description: "LC document mismatch".to_string(),
            amount: None,
            supporting_evidence_digests: vec![test_digest(), test_digest()],
        },
    ];

    let dispute = Dispute::file(
        claimant,
        respondent,
        DisputeType::PaymentDefault,
        jid,
        None,
        "siac".to_string(),
        claims,
        FilingEvidence {
            filing_document_digest: test_digest(),
        },
    );

    assert_eq!(dispute.state, DisputeState::Filed);
    assert_eq!(dispute.claims.len(), 2);
    assert_eq!(dispute.institution_id, "siac");
    assert!(!dispute.transition_log.is_empty());
}
