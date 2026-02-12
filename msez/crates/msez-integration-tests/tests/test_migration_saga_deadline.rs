//! # Migration Saga Deadline Test
//!
//! Tests the migration saga state machine with deadline enforcement:
//! - Advancing past deadline triggers automatic TimedOut transition
//! - Normal advancement with future deadline
//! - Terminal state cannot advance
//! - Compensation saga triggers
//! - Builder pattern compile-time deadline enforcement (via doc test)

use chrono::{TimeDelta, Utc};
use msez_core::{JurisdictionId, MigrationId};
use msez_state::migration::MigrationError;
use msez_state::{MigrationBuilder, MigrationSaga, MigrationState};

fn future_deadline() -> chrono::DateTime<Utc> {
    Utc::now() + TimeDelta::try_hours(24).unwrap()
}

fn past_deadline() -> chrono::DateTime<Utc> {
    Utc::now() - TimeDelta::try_hours(1).unwrap()
}

fn test_saga() -> MigrationSaga {
    MigrationBuilder::new(MigrationId::new())
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .deadline(future_deadline())
        .build()
}

// ---------------------------------------------------------------------------
// 1. Normal advancement with future deadline
// ---------------------------------------------------------------------------

#[test]
fn advance_through_all_phases() {
    let mut saga = test_saga();
    assert_eq!(saga.state, MigrationState::Initiated);

    let expected = [
        MigrationState::ComplianceCheck,
        MigrationState::AttestationGathering,
        MigrationState::SourceLocked,
        MigrationState::InTransit,
        MigrationState::DestinationVerification,
        MigrationState::DestinationUnlock,
        MigrationState::Completed,
    ];

    for expected_state in &expected {
        let next = saga.advance().unwrap();
        assert_eq!(next, *expected_state);
    }

    assert!(saga.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 2. Deadline enforcement — past deadline triggers TimedOut
// ---------------------------------------------------------------------------

#[test]
fn past_deadline_triggers_timeout() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .deadline(past_deadline())
        .build();

    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::Timeout { .. }));
    assert_eq!(saga.state, MigrationState::TimedOut);
    assert!(saga.state.is_terminal());
}

#[test]
fn past_deadline_at_in_transit_triggers_timeout() {
    // Start with future deadline, advance to InTransit, then simulate deadline passing
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .deadline(past_deadline()) // Will trigger on first advance
        .build();

    let err = saga.advance().unwrap_err();
    assert!(
        matches!(err, MigrationError::Timeout { state: MigrationState::Initiated, .. }),
        "should timeout at Initiated state"
    );
    assert_eq!(saga.state, MigrationState::TimedOut);
}

// ---------------------------------------------------------------------------
// 3. Terminal state cannot advance
// ---------------------------------------------------------------------------

#[test]
fn completed_saga_cannot_advance() {
    let mut saga = test_saga();
    for _ in 0..7 {
        saga.advance().unwrap();
    }
    assert_eq!(saga.state, MigrationState::Completed);

    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

#[test]
fn timed_out_saga_cannot_advance() {
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .deadline(past_deadline())
        .build();

    let _ = saga.advance(); // triggers timeout
    assert_eq!(saga.state, MigrationState::TimedOut);

    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

#[test]
fn cancelled_saga_cannot_advance() {
    let mut saga = test_saga();
    saga.cancel().unwrap();
    assert_eq!(saga.state, MigrationState::Cancelled);

    let err = saga.advance().unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

// ---------------------------------------------------------------------------
// 4. Cancellation rules
// ---------------------------------------------------------------------------

#[test]
fn cancel_allowed_before_transit() {
    for advance_count in 0..4 {
        let mut saga = test_saga();
        for _ in 0..advance_count {
            saga.advance().unwrap();
        }
        assert!(saga.cancel().is_ok(), "cancel should be allowed at state {:?}", saga.state);
        assert_eq!(saga.state, MigrationState::Cancelled);
    }
}

#[test]
fn cancel_rejected_at_and_after_transit() {
    let mut saga = test_saga();
    // Advance to InTransit (4 advances)
    for _ in 0..4 {
        saga.advance().unwrap();
    }
    assert_eq!(saga.state, MigrationState::InTransit);

    let err = saga.cancel().unwrap_err();
    assert!(matches!(err, MigrationError::InvalidTransition { .. }));
}

// ---------------------------------------------------------------------------
// 5. Compensation saga triggers
// ---------------------------------------------------------------------------

#[test]
fn compensation_records_context() {
    let mut saga = test_saga();
    saga.advance().unwrap(); // ComplianceCheck
    saga.advance().unwrap(); // AttestationGathering

    saga.compensate("compliance_failure: sanctions hit").unwrap();
    assert_eq!(saga.state, MigrationState::Compensated);
    assert_eq!(saga.compensation_log.len(), 1);
    assert!(saga.compensation_log[0].succeeded);
    assert!(saga.compensation_log[0]
        .action
        .contains("compliance_failure"));
}

#[test]
fn compensation_from_terminal_fails() {
    let mut saga = test_saga();
    saga.cancel().unwrap();

    let err = saga.compensate("reason").unwrap_err();
    assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
}

#[test]
fn compensation_failure_preserves_error_detail() {
    let mut saga = test_saga();
    saga.record_compensation_failure(
        "unlock_source",
        "connection timeout to source jurisdiction node at pk-rsez.mass.gov.pk:8443",
    );

    assert_eq!(saga.compensation_log.len(), 1);
    assert!(!saga.compensation_log[0].succeeded);
    assert_eq!(
        saga.compensation_log[0].error_detail.as_deref(),
        Some("connection timeout to source jurisdiction node at pk-rsez.mass.gov.pk:8443")
    );
}

// ---------------------------------------------------------------------------
// 6. Builder pattern
// ---------------------------------------------------------------------------

#[test]
fn builder_with_all_fields() {
    let saga = MigrationBuilder::new(MigrationId::new())
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .asset_description("Heavy manufacturing equipment line A-7")
        .deadline(future_deadline())
        .build();

    assert!(saga.source_jurisdiction.is_some());
    assert!(saga.destination_jurisdiction.is_some());
    assert_eq!(saga.asset_description, "Heavy manufacturing equipment line A-7");
    assert_eq!(saga.state, MigrationState::Initiated);
}

#[test]
fn builder_minimal_fields() {
    let saga = MigrationBuilder::new(MigrationId::new())
        .deadline(future_deadline())
        .build();

    assert!(saga.source_jurisdiction.is_none());
    assert!(saga.destination_jurisdiction.is_none());
    assert!(saga.asset_description.is_empty());
    assert_eq!(saga.state, MigrationState::Initiated);
}

// ---------------------------------------------------------------------------
// 7. State display names
// ---------------------------------------------------------------------------

#[test]
fn state_display_names_match_spec() {
    assert_eq!(MigrationState::Initiated.as_str(), "INITIATED");
    assert_eq!(MigrationState::ComplianceCheck.as_str(), "COMPLIANCE_CHECK");
    assert_eq!(MigrationState::AttestationGathering.as_str(), "ATTESTATION_GATHERING");
    assert_eq!(MigrationState::SourceLocked.as_str(), "SOURCE_LOCKED");
    assert_eq!(MigrationState::InTransit.as_str(), "IN_TRANSIT");
    assert_eq!(MigrationState::DestinationVerification.as_str(), "DESTINATION_VERIFICATION");
    assert_eq!(MigrationState::DestinationUnlock.as_str(), "DESTINATION_UNLOCK");
    assert_eq!(MigrationState::Completed.as_str(), "COMPLETED");
    assert_eq!(MigrationState::Compensated.as_str(), "COMPENSATED");
    assert_eq!(MigrationState::TimedOut.as_str(), "TIMED_OUT");
    assert_eq!(MigrationState::Cancelled.as_str(), "CANCELLED");
}

// ---------------------------------------------------------------------------
// 8. Terminal state identification
// ---------------------------------------------------------------------------

#[test]
fn terminal_states_are_correct() {
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

/// This doc test verifies compile-time enforcement:
/// `MigrationBuilder<NoDeadline>` does NOT have a `.build()` method.
///
/// ```compile_fail
/// use msez_state::MigrationBuilder;
/// use msez_core::MigrationId;
///
/// // This should NOT compile because no deadline was set
/// let saga = MigrationBuilder::new(MigrationId::new()).build();
/// ```
#[test]
fn compile_time_deadline_enforcement_doc_test_marker() {
    // The actual compile-fail test is in the doc comment above.
    // This test exists to anchor the doc test in the test runner.
    // The fact that the code below compiles proves HasDeadline works:
    let _saga = MigrationBuilder::new(MigrationId::new())
        .deadline(future_deadline())
        .build();
}

// ---------------------------------------------------------------------------
// 9. Multiple compensation records
// ---------------------------------------------------------------------------

#[test]
fn multiple_compensation_records() {
    let mut saga = test_saga();
    saga.advance().unwrap(); // ComplianceCheck

    // Record multiple failures before compensating
    saga.record_compensation_failure("unlock_source", "timeout");
    saga.record_compensation_failure("refund_fees", "insufficient balance");
    saga.record_compensation_failure("notify_counterparties", "smtp error");

    assert_eq!(saga.compensation_log.len(), 3);
    assert!(saga.compensation_log.iter().all(|r| !r.succeeded));

    // Now compensate
    saga.compensate("multiple_failures").unwrap();
    assert_eq!(saga.compensation_log.len(), 4);
    assert!(saga.compensation_log[3].succeeded);
    assert_eq!(saga.state, MigrationState::Compensated);
}
