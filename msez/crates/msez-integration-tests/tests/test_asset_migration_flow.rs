//! # Asset Migration Flow Integration Tests
//!
//! Tests the complete migration saga lifecycle: building, advancing through
//! all phases to completion, deadline enforcement, cancellation, and
//! compensation. Corresponds to the Python migration saga tests.

use chrono::{TimeDelta, Utc};
use msez_core::{JurisdictionId, MigrationId};
use msez_state::migration::MigrationError;
use msez_state::{MigrationBuilder, MigrationState};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn future_deadline() -> chrono::DateTime<Utc> {
    Utc::now() + TimeDelta::try_hours(24).unwrap()
}

fn past_deadline() -> chrono::DateTime<Utc> {
    Utc::now() - TimeDelta::try_hours(1).unwrap()
}

fn pk_rsez() -> JurisdictionId {
    JurisdictionId::new("PK-RSEZ").unwrap()
}

fn ae_difc() -> JurisdictionId {
    JurisdictionId::new("AE-DIFC").unwrap()
}

// ---------------------------------------------------------------------------
// 1. Migration flow complete lifecycle
// ---------------------------------------------------------------------------

#[test]
fn migration_flow_complete_lifecycle() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .asset_description("Heavy manufacturing equipment line A-7")
        .deadline(future_deadline())
        .build();

    assert_eq!(saga.state, MigrationState::Initiated);

    // Advance through all 7 phases
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
        assert_eq!(next, *expected, "expected {expected:?}");
    }

    assert!(saga.state.is_terminal());
    assert_eq!(saga.state, MigrationState::Completed);
}

#[test]
fn migration_flow_state_display_names() {
    let states = [
        (MigrationState::Initiated, "INITIATED"),
        (MigrationState::ComplianceCheck, "COMPLIANCE_CHECK"),
        (
            MigrationState::AttestationGathering,
            "ATTESTATION_GATHERING",
        ),
        (MigrationState::SourceLocked, "SOURCE_LOCKED"),
        (MigrationState::InTransit, "IN_TRANSIT"),
        (
            MigrationState::DestinationVerification,
            "DESTINATION_VERIFICATION",
        ),
        (MigrationState::DestinationUnlock, "DESTINATION_UNLOCK"),
        (MigrationState::Completed, "COMPLETED"),
        (MigrationState::Compensated, "COMPENSATED"),
        (MigrationState::TimedOut, "TIMED_OUT"),
        (MigrationState::Cancelled, "CANCELLED"),
    ];

    for (state, expected_name) in &states {
        assert_eq!(state.as_str(), *expected_name);
    }
}

// ---------------------------------------------------------------------------
// 2. Migration flow with deadline
// ---------------------------------------------------------------------------

#[test]
fn migration_flow_with_deadline_future() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(future_deadline())
        .build();

    // Should advance normally with future deadline
    let next = saga.advance().unwrap();
    assert_eq!(next, MigrationState::ComplianceCheck);
}

#[test]
fn migration_flow_with_deadline_past() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(past_deadline())
        .build();

    // Should timeout on first advance
    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::Timeout { .. }));
    assert_eq!(saga.state, MigrationState::TimedOut);
    assert!(saga.state.is_terminal());
}

#[test]
fn migration_timed_out_cannot_advance() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .deadline(past_deadline())
        .build();

    let _ = saga.advance(); // triggers timeout
    assert_eq!(saga.state, MigrationState::TimedOut);

    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

// ---------------------------------------------------------------------------
// 3. Migration flow cancellation
// ---------------------------------------------------------------------------

#[test]
fn migration_flow_cancellation() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(future_deadline())
        .build();

    // Cancel at initial state
    saga.cancel().unwrap();
    assert_eq!(saga.state, MigrationState::Cancelled);
    assert!(saga.state.is_terminal());
}

#[test]
fn migration_cancel_allowed_before_transit() {
    // Cancel should be allowed for first 4 states (before InTransit)
    for advance_count in 0..4 {
        let mut saga = MigrationBuilder::new(MigrationId::new())
            .source(pk_rsez())
            .destination(ae_difc())
            .deadline(future_deadline())
            .build();

        for _ in 0..advance_count {
            saga.advance().unwrap();
        }

        assert!(
            saga.cancel().is_ok(),
            "cancel should be allowed at state {:?}",
            saga.state
        );
        assert_eq!(saga.state, MigrationState::Cancelled);
    }
}

#[test]
fn migration_cancel_rejected_at_and_after_transit() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(future_deadline())
        .build();

    // Advance to InTransit (4 advances)
    for _ in 0..4 {
        saga.advance().unwrap();
    }
    assert_eq!(saga.state, MigrationState::InTransit);

    let err = saga.cancel().unwrap_err();
    assert!(matches!(err, MigrationError::InvalidTransition { .. }));
}

// ---------------------------------------------------------------------------
// 4. Migration flow compensation
// ---------------------------------------------------------------------------

#[test]
fn migration_flow_compensation() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(future_deadline())
        .build();

    // BUG-037 RESOLVED: compensation only allowed from InTransit or later.
    // Pre-transit states should use cancel instead.
    saga.advance().unwrap(); // ComplianceCheck
    saga.advance().unwrap(); // AttestationGathering

    // Attempting compensation from pre-transit state should fail
    assert!(saga.compensate("too_early").is_err(), "BUG-037: compensation rejected before InTransit");

    // Advance to InTransit (2 more advances: SourceLocked → InTransit)
    saga.advance().unwrap(); // SourceLocked
    saga.advance().unwrap(); // InTransit

    // Compensate from InTransit — valid
    saga.compensate("sanctions_hit: entity appeared on OFAC list")
        .unwrap();
    assert_eq!(saga.state, MigrationState::Compensated);
    assert!(saga.state.is_terminal());

    assert_eq!(saga.compensation_log.len(), 1);
    assert!(saga.compensation_log[0].succeeded);
    assert!(saga.compensation_log[0].action.contains("sanctions_hit"));
}

#[test]
fn migration_compensation_failure_records_error_detail() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .deadline(future_deadline())
        .build();

    saga.record_compensation_failure(
        "unlock_source",
        "connection timeout to source jurisdiction node",
    );

    assert_eq!(saga.compensation_log.len(), 1);
    assert!(!saga.compensation_log[0].succeeded);
    assert_eq!(
        saga.compensation_log[0].error_detail.as_deref(),
        Some("connection timeout to source jurisdiction node")
    );
}

#[test]
fn migration_compensation_from_terminal_fails() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .deadline(future_deadline())
        .build();

    saga.cancel().unwrap();
    assert_eq!(saga.state, MigrationState::Cancelled);

    let err = saga.compensate("reason").unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

// ---------------------------------------------------------------------------
// 5. Builder pattern
// ---------------------------------------------------------------------------

#[test]
fn migration_builder_all_fields() {
    let saga = MigrationBuilder::new(MigrationId::new())
        .source(pk_rsez())
        .destination(ae_difc())
        .asset_description("Mining equipment transfer batch 42")
        .deadline(future_deadline())
        .build();

    assert!(saga.source_jurisdiction.is_some());
    assert!(saga.destination_jurisdiction.is_some());
    assert_eq!(saga.asset_description, "Mining equipment transfer batch 42");
    assert_eq!(saga.state, MigrationState::Initiated);
}

#[test]
fn migration_builder_minimal_fields() {
    let saga = MigrationBuilder::new(MigrationId::new())
        .deadline(future_deadline())
        .build();

    assert!(saga.source_jurisdiction.is_none());
    assert!(saga.destination_jurisdiction.is_none());
    assert!(saga.asset_description.is_empty());
    assert_eq!(saga.state, MigrationState::Initiated);
}

// ---------------------------------------------------------------------------
// 6. Terminal state identification
// ---------------------------------------------------------------------------

#[test]
fn migration_terminal_states() {
    assert!(MigrationState::Completed.is_terminal());
    assert!(MigrationState::Compensated.is_terminal());
    assert!(MigrationState::TimedOut.is_terminal());
    assert!(MigrationState::Cancelled.is_terminal());

    assert!(!MigrationState::Initiated.is_terminal());
    assert!(!MigrationState::ComplianceCheck.is_terminal());
    assert!(!MigrationState::AttestationGathering.is_terminal());
    assert!(!MigrationState::SourceLocked.is_terminal());
    assert!(!MigrationState::InTransit.is_terminal());
    assert!(!MigrationState::DestinationVerification.is_terminal());
    assert!(!MigrationState::DestinationUnlock.is_terminal());
}
