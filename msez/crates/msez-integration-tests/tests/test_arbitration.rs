//! Tests for the full arbitration dispute lifecycle.
//!
//! Validates the complete dispute lifecycle from filing through enforcement
//! and closure, including evidence chain of custody, escrow operations,
//! enforcement order preconditions, and invalid state transition rejection.

use msez_arbitration::{
    Claim, ClosureEvidence, DecisionEvidence, Dispute, DisputeState, DisputeType,
    EnforcementAction, EnforcementInitiationEvidence, EnforcementOrder, EnforcementStatus,
    EscrowAccount, EscrowStatus, EscrowType, EvidenceItem, EvidencePackage, EvidencePhaseEvidence,
    EvidenceType, FilingEvidence, HearingScheduleEvidence, Money, Party, ReleaseCondition,
    ReleaseConditionType, ReviewInitiationEvidence,
};
use msez_core::{sha256_digest, CanonicalBytes, CorridorId, Did, JurisdictionId, Timestamp};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn test_digest() -> msez_core::ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"test": "arbitration"})).unwrap();
    sha256_digest(&canonical)
}

fn make_digest(label: &str) -> msez_core::ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"label": label})).unwrap();
    sha256_digest(&canonical)
}

fn test_party(name: &str) -> Party {
    Party {
        did: Did::new(format!("did:key:z6Mk{}", name)).unwrap(),
        legal_name: name.to_string(),
        jurisdiction_id: Some(JurisdictionId::new("PK-RSEZ").unwrap()),
    }
}

fn filed_dispute() -> Dispute {
    Dispute::file(
        test_party("Claimant1"),
        test_party("Respondent1"),
        DisputeType::BreachOfContract,
        JurisdictionId::new("AE-DIFC").unwrap(),
        Some(CorridorId::new()),
        "difc-lcia".to_string(),
        vec![Claim {
            claim_id: "CLM-001".to_string(),
            claim_type: DisputeType::BreachOfContract,
            description: "Breach of delivery terms".to_string(),
            amount: Some(Money::new("250000", "USD").unwrap()),
            supporting_evidence_digests: vec![test_digest()],
        }],
        FilingEvidence {
            filing_document_digest: test_digest(),
        },
    )
}

// ---------------------------------------------------------------------------
// Full dispute lifecycle
// ---------------------------------------------------------------------------

#[test]
fn dispute_full_lifecycle() {
    let mut dispute = filed_dispute();
    assert_eq!(dispute.state, DisputeState::Filed);

    // Filed -> UnderReview
    dispute
        .begin_review(ReviewInitiationEvidence {
            case_reference: "DIFC-LCIA-2026-001".to_string(),
            institution_acknowledgment_digest: make_digest("review"),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::UnderReview);

    // UnderReview -> EvidenceCollection
    dispute
        .open_evidence_collection(EvidencePhaseEvidence {
            procedural_order_digest: make_digest("evidence_order"),
            evidence_deadline: Timestamp::now(),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::EvidenceCollection);

    // EvidenceCollection -> Hearing
    dispute
        .schedule_hearing(HearingScheduleEvidence {
            hearing_date: Timestamp::now(),
            tribunal_composition_digest: make_digest("tribunal"),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::Hearing);

    // Hearing -> Decided
    dispute
        .decide(DecisionEvidence {
            ruling_digest: make_digest("ruling"),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::Decided);

    // Decided -> Enforced
    dispute
        .enforce(EnforcementInitiationEvidence {
            enforcement_order_digest: make_digest("enforcement"),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::Enforced);

    // Enforced -> Closed
    dispute
        .close(ClosureEvidence {
            final_report_digest: make_digest("closure"),
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::Closed);
    assert!(dispute.state.is_terminal());

    // Transition log should have entries for all transitions.
    assert!(dispute.transition_log.len() >= 7);
}

// ---------------------------------------------------------------------------
// Evidence chain of custody
// ---------------------------------------------------------------------------

#[test]
fn dispute_evidence_chain_of_custody() {
    let did_submitter = Did::new("did:key:z6MkSubmitter1".to_string()).unwrap();

    let content = json!({
        "contract_id": "C-2026-001",
        "amount": "150000",
        "currency": "USD"
    });

    let item = EvidenceItem::new(
        EvidenceType::ContractDocument,
        "Purchase Agreement".to_string(),
        "Original contract between parties".to_string(),
        &content,
        did_submitter.clone(),
    )
    .unwrap();

    assert_eq!(item.chain_of_custody.len(), 1);
    assert_eq!(item.evidence_type, EvidenceType::ContractDocument);

    // Verify integrity.
    assert!(item.verify_integrity(&content).is_ok());

    // Tampered content must fail integrity check.
    let tampered = json!({"contract_id": "C-2026-001", "amount": "999999", "currency": "USD"});
    assert!(item.verify_integrity(&tampered).is_err());

    // Package the evidence.
    let dispute_id = msez_arbitration::DisputeId::new();
    let package = EvidencePackage::new(dispute_id, did_submitter, vec![item]).unwrap();

    assert_eq!(package.item_count(), 1);
    assert!(package.verify_package_integrity().is_ok());
}

// ---------------------------------------------------------------------------
// Escrow deposit and release
// ---------------------------------------------------------------------------

#[test]
fn escrow_deposit_and_release() {
    let dispute_id = msez_arbitration::DisputeId::new();

    let mut escrow =
        EscrowAccount::create(dispute_id, EscrowType::FilingFee, "USD".to_string(), None);
    assert_eq!(escrow.status, EscrowStatus::Pending);

    // Deposit.
    escrow.deposit("50000".to_string(), test_digest()).unwrap();
    assert_eq!(escrow.status, EscrowStatus::Funded);
    assert_eq!(escrow.held_amount, "50000");

    // Full release.
    escrow
        .full_release(ReleaseCondition {
            condition_type: ReleaseConditionType::RulingEnforced,
            evidence_digest: test_digest(),
            satisfied_at: Timestamp::now(),
        })
        .unwrap();
    assert_eq!(escrow.status, EscrowStatus::FullyReleased);
    assert_eq!(escrow.held_amount, "0");
    assert!(escrow.status.is_terminal());
}

#[test]
fn escrow_partial_release() {
    let dispute_id = msez_arbitration::DisputeId::new();

    let mut escrow = EscrowAccount::create(
        dispute_id,
        EscrowType::SecurityDeposit,
        "SGD".to_string(),
        None,
    );
    escrow.deposit("100000".to_string(), test_digest()).unwrap();

    // Partial release.
    escrow
        .partial_release(
            "30000".to_string(),
            ReleaseCondition {
                condition_type: ReleaseConditionType::InstitutionOrder,
                evidence_digest: test_digest(),
                satisfied_at: Timestamp::now(),
            },
        )
        .unwrap();
    assert_eq!(escrow.status, EscrowStatus::PartiallyReleased);
    assert_eq!(escrow.held_amount, "70000");
}

// ---------------------------------------------------------------------------
// Enforcement order preconditions
// ---------------------------------------------------------------------------

#[test]
fn enforcement_order_preconditions() {
    let dispute_id = msez_arbitration::DisputeId::new();

    let mut order = EnforcementOrder::new(
        dispute_id,
        test_digest(),
        vec![EnforcementAction::EscrowRelease {
            escrow_id: msez_arbitration::EscrowId::new(),
            beneficiary: Did::new("did:key:z6MkWinner".to_string()).unwrap(),
            amount: None,
        }],
        None,
    );
    assert_eq!(order.status, EnforcementStatus::Pending);

    // Add precondition.
    order
        .add_precondition("Appeal period must expire".to_string())
        .unwrap();

    // Cannot begin without satisfying preconditions.
    let result = order.begin_enforcement();
    assert!(result.is_err());

    // Satisfy precondition.
    order.satisfy_precondition(0, test_digest()).unwrap();

    // Now can begin.
    order.begin_enforcement().unwrap();
    assert_eq!(order.status, EnforcementStatus::InProgress);
}

// ---------------------------------------------------------------------------
// Invalid state transition rejected
// ---------------------------------------------------------------------------

#[test]
fn invalid_state_transition_rejected() {
    let mut dispute = filed_dispute();

    // Cannot jump from Filed to Hearing (must go through UnderReview and
    // EvidenceCollection first).
    let result = dispute.schedule_hearing(HearingScheduleEvidence {
        hearing_date: Timestamp::now(),
        tribunal_composition_digest: test_digest(),
    });
    assert!(result.is_err(), "Filed -> Hearing should be rejected");

    // Cannot decide from Filed.
    let result = dispute.decide(DecisionEvidence {
        ruling_digest: test_digest(),
    });
    assert!(result.is_err(), "Filed -> Decided should be rejected");

    // State should still be Filed.
    assert_eq!(dispute.state, DisputeState::Filed);
}

#[test]
fn terminal_state_rejects_all_transitions() {
    let mut dispute = filed_dispute();

    // Settle the dispute (terminal).
    dispute
        .settle(msez_arbitration::SettlementEvidence {
            settlement_agreement_digest: test_digest(),
            party_consent_digests: vec![test_digest(), test_digest()],
        })
        .unwrap();
    assert_eq!(dispute.state, DisputeState::Settled);
    assert!(dispute.state.is_terminal());

    // All further transitions must be rejected.
    assert!(dispute
        .begin_review(ReviewInitiationEvidence {
            case_reference: "X".to_string(),
            institution_acknowledgment_digest: test_digest(),
        })
        .is_err());
}

#[test]
fn dispute_types_exhaustive() {
    let all = DisputeType::all();
    assert_eq!(all.len(), 8, "There must be exactly 8 dispute types");
}
