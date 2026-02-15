//! # Corridor Lifecycle End-to-End Test
//!
//! Tests the full corridor lifecycle from Draft to Deprecated:
//! Draft → Pending → Active → Suspended → Active → Halted → Deprecated.
//!
//! Also verifies receipt chain, MMR inclusion proofs, and transition log
//! completeness along the way.

use msez_core::{sha256_digest, CanonicalBytes, ContentDigest, CorridorId, JurisdictionId};
use msez_corridor::{CorridorReceipt, ReceiptChain};
use msez_state::corridor::{
    ActivationEvidence, DeprecationEvidence, HaltReason, ResumeEvidence, SubmissionEvidence,
    SuspendReason,
};
use msez_state::{Corridor, Draft, DynCorridorData, DynCorridorState};
use serde_json::json;

fn test_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"evidence": label})).unwrap();
    sha256_digest(&canonical)
}

fn make_next_root(i: u64) -> String {
    let data = json!({"payload": i, "corridor": "test"});
    let canonical = CanonicalBytes::new(&data).unwrap();
    sha256_digest(&canonical).to_hex()
}

fn make_receipt(chain: &ReceiptChain, i: u64) -> CorridorReceipt {
    CorridorReceipt {
        receipt_type: "MSEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: msez_core::Timestamp::now(),
        prev_root: chain.mmr_root().unwrap(),
        next_root: make_next_root(i),
        lawpack_digest_set: vec!["deadbeef".repeat(8)],
        ruleset_digest_set: vec!["cafebabe".repeat(8)],
    }
}

// ---------------------------------------------------------------------------
// Full lifecycle: Draft → Pending → Active → Suspended → Active → Halted → Deprecated
// ---------------------------------------------------------------------------

#[test]
fn full_corridor_lifecycle() {
    let id = CorridorId::new();
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    // 1. Create Draft
    let corridor = Corridor::<Draft>::new(id.clone(), ja, jb);
    assert_eq!(corridor.state_name(), "DRAFT");
    assert_eq!(corridor.transition_log().len(), 0);

    // 2. Submit evidence → Pending
    let pending = corridor.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("bilateral-agreement"),
        pack_trilogy_digest: test_digest("pack-trilogy"),
    });
    assert_eq!(pending.state_name(), "PENDING");
    assert_eq!(pending.transition_log().len(), 1);
    assert_eq!(
        pending.transition_log()[0].from_state,
        DynCorridorState::Draft
    );
    assert_eq!(
        pending.transition_log()[0].to_state,
        DynCorridorState::Pending
    );

    // 3. Activate → Active
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-pk"),
        regulatory_approval_b: test_digest("approval-ae"),
    });
    assert_eq!(active.state_name(), "ACTIVE");
    assert_eq!(active.transition_log().len(), 2);

    // 4. Suspend → Suspended
    let suspended = active.suspend(SuspendReason {
        reason: "Scheduled maintenance window".to_string(),
        expected_resume: None,
    });
    assert_eq!(suspended.state_name(), "SUSPENDED");
    assert_eq!(suspended.transition_log().len(), 3);

    // 5. Resume → Active
    let active_again = suspended.resume(ResumeEvidence {
        resolution_attestation: test_digest("maintenance-complete"),
    });
    assert_eq!(active_again.state_name(), "ACTIVE");
    assert_eq!(active_again.transition_log().len(), 4);

    // 6. Halt → Halted
    let halted = active_again.halt(HaltReason {
        reason: "Fork detected in receipt chain".to_string(),
        authority: JurisdictionId::new("PK-RSEZ").unwrap(),
        evidence: test_digest("fork-evidence"),
    });
    assert_eq!(halted.state_name(), "HALTED");
    assert!(!halted.is_terminal());
    assert_eq!(halted.transition_log().len(), 5);

    // 7. Deprecate → Deprecated (terminal)
    let deprecated = halted.deprecate(DeprecationEvidence {
        deprecation_decision_digest: test_digest("deprecation-decision"),
        reason: "Corridor permanently sunset by bilateral agreement".to_string(),
    });
    assert_eq!(deprecated.state_name(), "DEPRECATED");
    assert!(deprecated.is_terminal());
    assert_eq!(deprecated.transition_log().len(), 6);

    // Verify complete transition log
    let log = deprecated.transition_log();
    let expected_transitions = [
        (DynCorridorState::Draft, DynCorridorState::Pending),
        (DynCorridorState::Pending, DynCorridorState::Active),
        (DynCorridorState::Active, DynCorridorState::Suspended),
        (DynCorridorState::Suspended, DynCorridorState::Active),
        (DynCorridorState::Active, DynCorridorState::Halted),
        (DynCorridorState::Halted, DynCorridorState::Deprecated),
    ];

    for (i, (from, to)) in expected_transitions.iter().enumerate() {
        assert_eq!(log[i].from_state, *from, "transition {i} from");
        assert_eq!(log[i].to_state, *to, "transition {i} to");
    }
}

// ---------------------------------------------------------------------------
// Receipt chain with MMR inclusion proofs
// ---------------------------------------------------------------------------

#[test]
fn receipt_chain_with_mmr_proofs() {
    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id);
    assert_eq!(chain.height(), 0);

    // Append 10 receipts
    for i in 0..10 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }
    assert_eq!(chain.height(), 10);

    // Verify MMR root is non-empty and 64 hex chars
    let root = chain.mmr_root().unwrap();
    assert_eq!(root.len(), 64);
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));

    // Verify inclusion proofs for all receipts
    for idx in 0..10 {
        let proof = chain.build_inclusion_proof(idx).unwrap();
        assert!(
            chain.verify_inclusion_proof(&proof).unwrap(),
            "inclusion proof for receipt {idx} must verify"
        );
    }
}

#[test]
fn receipt_chain_reject_sequence_mismatch() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    let receipt = make_receipt(&chain, 0);
    chain.append(receipt).unwrap();

    // Create receipt with wrong sequence
    let mut bad_receipt = make_receipt(&chain, 1);
    bad_receipt.sequence = 5; // Wrong
    assert!(chain.append(bad_receipt).is_err());
}

#[test]
fn receipt_chain_reject_prev_root_mismatch() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    let receipt = make_receipt(&chain, 0);
    chain.append(receipt).unwrap();

    let mut bad_receipt = make_receipt(&chain, 1);
    bad_receipt.prev_root = "00".repeat(32); // Wrong
    assert!(chain.append(bad_receipt).is_err());
}

#[test]
fn checkpoint_captures_mmr_state() {
    let mut chain = ReceiptChain::new(CorridorId::new());

    for i in 0..5 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }

    let checkpoint = chain.create_checkpoint().unwrap();
    assert_eq!(checkpoint.height, 5);
    assert_eq!(checkpoint.mmr_root, chain.mmr_root().unwrap());
    assert_eq!(checkpoint.checkpoint_digest.to_hex().len(), 64);
}

#[test]
fn tampered_proof_fails_verification() {
    let mut chain = ReceiptChain::new(CorridorId::new());
    for i in 0..5 {
        let receipt = make_receipt(&chain, i);
        chain.append(receipt).unwrap();
    }

    let mut proof = chain.build_inclusion_proof(2).unwrap();
    if !proof.path.is_empty() {
        proof.path[0].hash = "00".repeat(32);
    }
    // Tampered proof should fail
    assert!(!msez_corridor::receipt::verify_receipt_proof(&proof));
}

// ---------------------------------------------------------------------------
// DynCorridor serialization
// ---------------------------------------------------------------------------

#[test]
fn dyn_corridor_from_typed_preserves_state() {
    let corridor = Corridor::<Draft>::new(
        CorridorId::new(),
        JurisdictionId::new("PK-RSEZ").unwrap(),
        JurisdictionId::new("AE-DIFC").unwrap(),
    );
    let dyn_data = DynCorridorData::from(&corridor);
    assert_eq!(dyn_data.state, DynCorridorState::Draft);
    assert_eq!(dyn_data.state.as_str(), "DRAFT");
    assert!(!dyn_data.state.is_terminal());
}

#[test]
fn defective_state_names_are_rejected() {
    // The Python v1 defective names must not deserialize
    let proposed: Result<DynCorridorState, _> = serde_json::from_str("\"PROPOSED\"");
    assert!(proposed.is_err(), "PROPOSED must not be a valid state");

    let operational: Result<DynCorridorState, _> = serde_json::from_str("\"OPERATIONAL\"");
    assert!(
        operational.is_err(),
        "OPERATIONAL must not be a valid state"
    );
}

#[test]
fn valid_transitions_are_exhaustive() {
    assert_eq!(
        DynCorridorState::Draft.valid_transitions(),
        &[DynCorridorState::Pending]
    );
    assert_eq!(
        DynCorridorState::Pending.valid_transitions(),
        &[DynCorridorState::Active]
    );
    assert_eq!(
        DynCorridorState::Active.valid_transitions(),
        &[DynCorridorState::Halted, DynCorridorState::Suspended]
    );
    assert_eq!(
        DynCorridorState::Halted.valid_transitions(),
        &[DynCorridorState::Deprecated]
    );
    assert_eq!(
        DynCorridorState::Suspended.valid_transitions(),
        &[DynCorridorState::Active]
    );
    assert!(DynCorridorState::Deprecated.valid_transitions().is_empty());
}

// ---------------------------------------------------------------------------
// Multiple checkpoints across receipt growth
// ---------------------------------------------------------------------------

#[test]
fn multiple_checkpoints_across_growth() {
    let mut chain = ReceiptChain::new(CorridorId::new());

    // Add 3 receipts, checkpoint
    for i in 0..3 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let cp1 = chain.create_checkpoint().unwrap();
    assert_eq!(cp1.height, 3);

    // Add 4 more receipts, checkpoint
    for i in 3..7 {
        chain.append(make_receipt(&chain, i)).unwrap();
    }
    let cp2 = chain.create_checkpoint().unwrap();
    assert_eq!(cp2.height, 7);

    // Checkpoints should differ
    assert_ne!(cp1.mmr_root, cp2.mmr_root);
    assert_ne!(cp1.checkpoint_digest, cp2.checkpoint_digest);
    assert_eq!(chain.checkpoints().len(), 2);
}

// ---------------------------------------------------------------------------
// Receipt content digest is deterministic
// ---------------------------------------------------------------------------

#[test]
fn receipt_content_digest_is_deterministic() {
    let chain = ReceiptChain::new(CorridorId::new());
    let receipt = make_receipt(&chain, 42);
    let d1 = receipt.content_digest().unwrap();
    let d2 = receipt.content_digest().unwrap();
    assert_eq!(d1, d2);
}
