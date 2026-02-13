//! # Lifecycle State Machine Integration Tests
//!
//! Python counterpart: `tests/test_lifecycle_state_machine.py`
//!
//! Tests all lifecycle state machines:
//! - Corridor: Draft -> Pending -> Active -> Suspended -> Active -> Halted -> Deprecated
//! - Entity: Applied -> Active -> Dissolving -> Dissolved
//! - License: Applied -> UnderReview -> Active -> Suspended -> Revoked
//! - Migration: Initiated through all 7 phases to Completed

use chrono::{TimeDelta, Utc};
use msez_core::{
    sha256_digest, CanonicalBytes, ContentDigest, CorridorId, EntityId, JurisdictionId, MigrationId,
};
use msez_state::corridor::{
    ActivationEvidence, DeprecationEvidence, HaltReason, ResumeEvidence, SubmissionEvidence,
    SuspendReason,
};
use msez_state::{
    Corridor, Draft, DynCorridorState, Entity, EntityLifecycleState, License, LicenseState,
    MigrationBuilder, MigrationState,
};
use serde_json::json;

fn test_digest(label: &str) -> ContentDigest {
    sha256_digest(&CanonicalBytes::new(&json!({"e": label})).unwrap())
}

// ---------------------------------------------------------------------------
// 1. Corridor full lifecycle
// ---------------------------------------------------------------------------

#[test]
fn corridor_full_lifecycle() {
    let id = CorridorId::new();
    let ja = JurisdictionId::new("PK-RSEZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    // Draft
    let corridor = Corridor::<Draft>::new(id, ja, jb);
    assert_eq!(corridor.state_name(), "DRAFT");
    assert_eq!(corridor.transition_log().len(), 0);

    // Pending
    let pending = corridor.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("bilateral"),
        pack_trilogy_digest: test_digest("packs"),
    });
    assert_eq!(pending.state_name(), "PENDING");
    assert_eq!(pending.transition_log().len(), 1);

    // Active
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-pk"),
        regulatory_approval_b: test_digest("approval-ae"),
    });
    assert_eq!(active.state_name(), "ACTIVE");

    // Suspended
    let suspended = active.suspend(SuspendReason {
        reason: "Scheduled maintenance".to_string(),
        expected_resume: None,
    });
    assert_eq!(suspended.state_name(), "SUSPENDED");

    // Active again
    let active_again = suspended.resume(ResumeEvidence {
        resolution_attestation: test_digest("maintenance-done"),
    });
    assert_eq!(active_again.state_name(), "ACTIVE");

    // Halted
    let halted = active_again.halt(HaltReason {
        reason: "Fork detected".to_string(),
        authority: JurisdictionId::new("PK-RSEZ").unwrap(),
        evidence: test_digest("fork-evidence"),
    });
    assert_eq!(halted.state_name(), "HALTED");
    assert!(!halted.is_terminal());

    // Deprecated (terminal)
    let deprecated = halted.deprecate(DeprecationEvidence {
        deprecation_decision_digest: test_digest("deprecation"),
        reason: "Permanently sunset".to_string(),
    });
    assert_eq!(deprecated.state_name(), "DEPRECATED");
    assert!(deprecated.is_terminal());
    assert_eq!(deprecated.transition_log().len(), 6);
}

// ---------------------------------------------------------------------------
// 2. Entity formation to dissolution
// ---------------------------------------------------------------------------

#[test]
fn entity_formation_to_dissolution() {
    let mut entity = Entity::new(EntityId::new());
    assert_eq!(entity.state, EntityLifecycleState::Applied);

    entity.approve().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Active);

    entity.initiate_dissolution().unwrap();
    assert_eq!(entity.state, EntityLifecycleState::Dissolving);

    // Advance through all 10 dissolution stages
    for _ in 0..10 {
        entity.advance_dissolution().unwrap();
    }
    assert_eq!(entity.state, EntityLifecycleState::Dissolved);
    assert!(entity.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 3. License active -> suspend -> revoke
// ---------------------------------------------------------------------------

#[test]
fn license_active_suspend_revoke() {
    let mut lic = License::new("MINING_PERMIT");
    lic.review().unwrap();
    lic.issue().unwrap();
    assert_eq!(lic.state, LicenseState::Active);

    lic.suspend("Environmental violation").unwrap();
    assert_eq!(lic.state, LicenseState::Suspended);

    lic.revoke("Violation confirmed").unwrap();
    assert_eq!(lic.state, LicenseState::Revoked);
    assert!(lic.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 4. Migration saga advance through all phases
// ---------------------------------------------------------------------------

#[test]
fn migration_saga_advance_through_phases() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .deadline(Utc::now() + TimeDelta::try_hours(24).unwrap())
        .build();

    assert_eq!(saga.state, MigrationState::Initiated);

    let expected_states = [
        MigrationState::ComplianceCheck,
        MigrationState::AttestationGathering,
        MigrationState::SourceLocked,
        MigrationState::InTransit,
        MigrationState::DestinationVerification,
        MigrationState::DestinationUnlock,
        MigrationState::Completed,
    ];

    for expected in &expected_states {
        let next = saga.advance().unwrap();
        assert_eq!(next, *expected);
    }

    assert!(saga.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 5. Corridor transition log is complete
// ---------------------------------------------------------------------------

#[test]
fn corridor_transition_log_complete() {
    let corridor = Corridor::<Draft>::new(
        CorridorId::new(),
        JurisdictionId::new("PK-RSEZ").unwrap(),
        JurisdictionId::new("AE-DIFC").unwrap(),
    );

    let pending = corridor.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("b"),
        pack_trilogy_digest: test_digest("p"),
    });

    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("a1"),
        regulatory_approval_b: test_digest("a2"),
    });

    let log = active.transition_log();
    assert_eq!(log.len(), 2);
    assert_eq!(log[0].from_state, "DRAFT");
    assert_eq!(log[0].to_state, "PENDING");
    assert_eq!(log[1].from_state, "PENDING");
    assert_eq!(log[1].to_state, "ACTIVE");
}

// ---------------------------------------------------------------------------
// 6. Terminal states are terminal
// ---------------------------------------------------------------------------

#[test]
fn terminal_states_are_terminal() {
    // Entity
    assert!(EntityLifecycleState::Dissolved.is_terminal());
    assert!(EntityLifecycleState::Rejected.is_terminal());
    assert!(!EntityLifecycleState::Applied.is_terminal());
    assert!(!EntityLifecycleState::Active.is_terminal());

    // License
    assert!(LicenseState::Revoked.is_terminal());
    assert!(LicenseState::Expired.is_terminal());
    assert!(LicenseState::Surrendered.is_terminal());
    assert!(LicenseState::Rejected.is_terminal());
    assert!(!LicenseState::Active.is_terminal());
    assert!(!LicenseState::Suspended.is_terminal());

    // Migration
    assert!(MigrationState::Completed.is_terminal());
    assert!(MigrationState::Compensated.is_terminal());
    assert!(MigrationState::TimedOut.is_terminal());
    assert!(MigrationState::Cancelled.is_terminal());
    assert!(!MigrationState::Initiated.is_terminal());
    assert!(!MigrationState::InTransit.is_terminal());

    // Corridor (via DynCorridorState)
    assert!(DynCorridorState::Deprecated.is_terminal());
    assert!(!DynCorridorState::Draft.is_terminal());
    assert!(!DynCorridorState::Active.is_terminal());
}
