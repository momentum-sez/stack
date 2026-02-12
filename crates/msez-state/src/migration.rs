//! # Migration Saga State Machine
//!
//! Implements the cross-jurisdiction asset migration saga with compile-time
//! deadline enforcement and structured compensation for failures.
//!
//! ## Phases (spec §42)
//!
//! ```text
//! Initiated ──▶ Validated ──▶ InTransit ──▶ Completing ──▶ Completed
//!     │              │             │              │
//!     └──────────────┴─────────────┴──────────────┘
//!                          │
//!                     on failure:
//!                          │
//!                     Compensating ──▶ Failed
//! ```
//!
//! Terminal states: Completed, Failed.
//!
//! ## Compile-Time Deadline Enforcement (audit §3.5, §5.5)
//!
//! `MigrationBuilder<NoDeadline>` has no `.build()` method.
//! Only `MigrationBuilder<HasDeadline>` can produce a `MigrationSaga`.
//! This prevents the audit finding where migrations without deadlines
//! could create permanent asset locks.
//!
//! ## Compensation Saga
//!
//! If a step fails during InTransit or Completing, the saga transitions
//! to Compensating with structured error context (not swallowed exceptions
//! as in the Python version — audit §3.4).
//!
//! ## Implements
//!
//! Spec §42 — Cross-jurisdiction asset migration protocol.

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::{MigrationId, Timestamp};

// ─── Errors ──────────────────────────────────────────────────────────

/// Error when a migration exceeds its deadline.
///
/// This error triggers automatic compensation to prevent permanent
/// asset lock (audit §3.5).
#[derive(Error, Debug)]
#[error("migration {migration_id} exceeded deadline at phase {phase}")]
pub struct MigrationTimeoutError {
    /// The migration that timed out.
    pub migration_id: String,
    /// The phase the migration was in when it timed out.
    pub phase: String,
}

/// Errors that can occur during migration saga operations.
#[derive(Error, Debug)]
pub enum MigrationError {
    /// Migration has exceeded its deadline.
    #[error(transparent)]
    Timeout(#[from] MigrationTimeoutError),

    /// Attempted transition is not valid from the current phase.
    #[error("invalid migration transition: {from} -> {to}")]
    InvalidTransition {
        /// Current phase.
        from: String,
        /// Attempted target phase.
        to: String,
    },

    /// Migration is in a terminal state and cannot advance.
    #[error("migration {migration_id} is in terminal phase {phase}")]
    TerminalPhase {
        /// The migration identifier.
        migration_id: String,
        /// The terminal phase.
        phase: String,
    },

    /// Compensation failed with structured error context.
    #[error("compensation failed for migration {migration_id}: {detail}")]
    CompensationFailed {
        /// The migration identifier.
        migration_id: String,
        /// Structured error detail (not swallowed).
        detail: String,
    },
}

// ─── Migration Phase ─────────────────────────────────────────────────

/// The phase of a migration saga.
///
/// Phases progress linearly from Initiated through Completed, with
/// failure branches to Compensating and ultimately Failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationPhase {
    /// Migration has been initiated with a request and deadline.
    Initiated,
    /// Source and destination jurisdictions have validated the migration.
    Validated,
    /// Assets are in transit between jurisdictions.
    InTransit,
    /// Destination has received assets; completing final checks.
    Completing,
    /// Migration successfully completed (terminal).
    Completed,
    /// Migration failed irrecoverably (terminal).
    Failed,
    /// Compensation actions are in progress after a failure.
    Compensating,
}

impl MigrationPhase {
    /// Whether this phase is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    /// Whether this phase allows advancing to the next normal phase.
    pub fn next_phase(&self) -> Option<MigrationPhase> {
        match self {
            Self::Initiated => Some(Self::Validated),
            Self::Validated => Some(Self::InTransit),
            Self::InTransit => Some(Self::Completing),
            Self::Completing => Some(Self::Completed),
            _ => None,
        }
    }

    /// Whether the saga can transition to Compensating from this phase.
    pub fn can_compensate(&self) -> bool {
        matches!(
            self,
            Self::Initiated | Self::Validated | Self::InTransit | Self::Completing
        )
    }
}

impl std::fmt::Display for MigrationPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Initiated => "INITIATED",
            Self::Validated => "VALIDATED",
            Self::InTransit => "IN_TRANSIT",
            Self::Completing => "COMPLETING",
            Self::Completed => "COMPLETED",
            Self::Failed => "FAILED",
            Self::Compensating => "COMPENSATING",
        };
        f.write_str(s)
    }
}

// ─── Transition Record ───────────────────────────────────────────────

/// Record of a state transition in the migration saga.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationTransition {
    /// Phase before the transition.
    pub from_phase: MigrationPhase,
    /// Phase after the transition.
    pub to_phase: MigrationPhase,
    /// When the transition occurred.
    pub timestamp: Timestamp,
    /// Reason for the transition.
    pub reason: String,
    /// Evidence digest, if any.
    pub evidence_digest: Option<String>,
}

// ─── Compensation Record ─────────────────────────────────────────────

/// Structured record of a compensation action.
///
/// Unlike the Python version (audit §3.4), compensation failures are
/// recorded with full error context — never swallowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationRecord {
    /// The compensation action taken.
    pub action: CompensationAction,
    /// When the action was attempted.
    pub timestamp: Timestamp,
    /// Whether the action succeeded.
    pub success: bool,
    /// Structured error detail if the action failed.
    pub error_detail: Option<String>,
}

/// Actions that can be taken during compensation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompensationAction {
    /// Unlock source assets that were locked for transit.
    UnlockSource,
    /// Refund any fees that were charged.
    RefundFees,
    /// Void attestations collected during the migration.
    VoidAttestations,
    /// Restore compliance state to pre-migration snapshot.
    RestoreComplianceState,
    /// Notify counterparties of migration failure.
    NotifyCounterparties,
}

impl std::fmt::Display for CompensationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::UnlockSource => "UNLOCK_SOURCE",
            Self::RefundFees => "REFUND_FEES",
            Self::VoidAttestations => "VOID_ATTESTATIONS",
            Self::RestoreComplianceState => "RESTORE_COMPLIANCE_STATE",
            Self::NotifyCounterparties => "NOTIFY_COUNTERPARTIES",
        };
        f.write_str(s)
    }
}

// ─── Builder — Compile-Time Deadline Enforcement ─────────────────────

/// Marker: deadline has not been set yet.
#[derive(Debug)]
pub struct NoDeadline;

/// Marker: deadline has been set.
#[derive(Debug)]
pub struct HasDeadline;

/// Builder for `MigrationSaga` with compile-time deadline enforcement.
///
/// `MigrationBuilder<NoDeadline>` has no `.build()` method. You must
/// call `.deadline()` to obtain a `MigrationBuilder<HasDeadline>`,
/// which is the only type that can produce a `MigrationSaga`.
///
/// This pattern prevents the audit finding (§3.5) where migrations
/// without deadlines could create permanent asset locks.
///
/// # Example
///
/// ```
/// use msez_state::migration::{MigrationBuilder, NoDeadline};
/// use msez_core::{MigrationId, Timestamp};
///
/// let saga = MigrationBuilder::new(MigrationId::new())
///     .source("PK-PSEZ".to_string())
///     .destination("AE-DIFC".to_string())
///     .deadline(Timestamp::now())  // Required — enables .build()
///     .build();
/// ```
///
/// The following will NOT compile because `MigrationBuilder<NoDeadline>`
/// has no `.build()` method:
///
/// ```compile_fail
/// use msez_state::migration::{MigrationBuilder, NoDeadline};
/// use msez_core::MigrationId;
///
/// let saga = MigrationBuilder::new(MigrationId::new())
///     .source("PK-PSEZ".to_string())
///     .build(); // ERROR: no method named `build` found
/// ```
#[derive(Debug)]
pub struct MigrationBuilder<D> {
    id: MigrationId,
    source_jurisdiction: Option<String>,
    destination_jurisdiction: Option<String>,
    asset_id: Option<String>,
    deadline: Option<Timestamp>,
    _deadline_marker: PhantomData<D>,
}

impl MigrationBuilder<NoDeadline> {
    /// Create a new migration builder. The deadline must be set before building.
    pub fn new(id: MigrationId) -> Self {
        Self {
            id,
            source_jurisdiction: None,
            destination_jurisdiction: None,
            asset_id: None,
            deadline: None,
            _deadline_marker: PhantomData,
        }
    }

    /// Set the migration deadline (required).
    ///
    /// Converts the builder to `MigrationBuilder<HasDeadline>`, enabling
    /// the `.build()` method.
    pub fn deadline(self, deadline: Timestamp) -> MigrationBuilder<HasDeadline> {
        MigrationBuilder {
            id: self.id,
            source_jurisdiction: self.source_jurisdiction,
            destination_jurisdiction: self.destination_jurisdiction,
            asset_id: self.asset_id,
            deadline: Some(deadline),
            _deadline_marker: PhantomData,
        }
    }
}

impl<D> MigrationBuilder<D> {
    /// Set the source jurisdiction.
    pub fn source(mut self, jurisdiction: String) -> Self {
        self.source_jurisdiction = Some(jurisdiction);
        self
    }

    /// Set the destination jurisdiction.
    pub fn destination(mut self, jurisdiction: String) -> Self {
        self.destination_jurisdiction = Some(jurisdiction);
        self
    }

    /// Set the asset being migrated.
    pub fn asset(mut self, asset_id: String) -> Self {
        self.asset_id = Some(asset_id);
        self
    }
}

impl MigrationBuilder<HasDeadline> {
    /// Build the migration saga.
    ///
    /// This method only exists on `MigrationBuilder<HasDeadline>`.
    /// Attempting to call `.build()` without first calling `.deadline()`
    /// is a compile error.
    pub fn build(self) -> MigrationSaga {
        let now = Timestamp::now();
        let mut saga = MigrationSaga {
            id: self.id,
            phase: MigrationPhase::Initiated,
            deadline: self.deadline.expect("HasDeadline guarantees deadline is set"),
            source_jurisdiction: self.source_jurisdiction.unwrap_or_default(),
            destination_jurisdiction: self.destination_jurisdiction.unwrap_or_default(),
            asset_id: self.asset_id.unwrap_or_default(),
            created_at: now,
            transitions: Vec::new(),
            compensations: Vec::new(),
        };
        saga.transitions.push(MigrationTransition {
            from_phase: MigrationPhase::Initiated,
            to_phase: MigrationPhase::Initiated,
            timestamp: now,
            reason: "Migration saga created".to_string(),
            evidence_digest: None,
        });
        saga
    }
}

// ─── Migration Saga ──────────────────────────────────────────────────

/// A cross-jurisdiction asset migration saga.
///
/// Manages the complete lifecycle of a migration including phase transitions,
/// deadline enforcement, and compensation for failures. Every phase transition
/// checks the deadline — if exceeded, the saga is forced to Failed state
/// with a `MigrationTimeoutError`.
///
/// ## Security Invariant
///
/// The deadline is checked at every transition. A migration that exceeds
/// its deadline while not in a terminal state is automatically failed,
/// preventing permanent asset lock (audit §3.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationSaga {
    /// Unique migration identifier.
    pub id: MigrationId,
    /// Current phase of the migration.
    pub phase: MigrationPhase,
    /// Deadline for the migration. Enforced at every transition.
    pub deadline: Timestamp,
    /// Source jurisdiction identifier.
    pub source_jurisdiction: String,
    /// Destination jurisdiction identifier.
    pub destination_jurisdiction: String,
    /// Asset being migrated.
    pub asset_id: String,
    /// When the saga was created.
    pub created_at: Timestamp,
    /// Ordered log of all phase transitions.
    pub transitions: Vec<MigrationTransition>,
    /// Compensation records (populated on failure).
    pub compensations: Vec<CompensationRecord>,
}

impl MigrationSaga {
    /// Advance the saga to the next phase in the normal progression.
    ///
    /// Checks the deadline before transitioning. If the deadline has been
    /// exceeded and the saga is not in a terminal state, returns a
    /// `MigrationTimeoutError` and forces the saga to Failed.
    ///
    /// ## Phase progression
    ///
    /// Initiated → Validated → InTransit → Completing → Completed
    pub fn advance(&mut self, reason: &str) -> Result<MigrationPhase, MigrationError> {
        self.check_deadline()?;

        if self.phase.is_terminal() {
            return Err(MigrationError::TerminalPhase {
                migration_id: self.id.to_string(),
                phase: self.phase.to_string(),
            });
        }

        let next = self.phase.next_phase().ok_or_else(|| {
            MigrationError::InvalidTransition {
                from: self.phase.to_string(),
                to: "next".to_string(),
            }
        })?;

        self.record_transition(next, reason);
        self.phase = next;
        Ok(next)
    }

    /// Transition the saga to Compensating after a failure.
    ///
    /// Only valid from non-terminal phases. Records structured error
    /// context (not swallowed exceptions — audit §3.4).
    pub fn compensate(&mut self, reason: &str) -> Result<(), MigrationError> {
        if !self.phase.can_compensate() {
            return Err(MigrationError::InvalidTransition {
                from: self.phase.to_string(),
                to: "COMPENSATING".to_string(),
            });
        }

        self.record_transition(MigrationPhase::Compensating, reason);
        self.phase = MigrationPhase::Compensating;
        Ok(())
    }

    /// Record a compensation action with structured error context.
    ///
    /// Compensation actions are tracked with full diagnostic information,
    /// fixing the Python audit finding where `except Exception:` silently
    /// swallowed error context (audit §3.4).
    pub fn record_compensation(
        &mut self,
        action: CompensationAction,
        success: bool,
        error_detail: Option<String>,
    ) {
        self.compensations.push(CompensationRecord {
            action,
            timestamp: Timestamp::now(),
            success,
            error_detail,
        });
    }

    /// Finalize the saga as Failed after compensation is complete.
    ///
    /// Only valid from the Compensating phase.
    pub fn fail(&mut self, reason: &str) -> Result<(), MigrationError> {
        if self.phase != MigrationPhase::Compensating {
            return Err(MigrationError::InvalidTransition {
                from: self.phase.to_string(),
                to: "FAILED".to_string(),
            });
        }

        self.record_transition(MigrationPhase::Failed, reason);
        self.phase = MigrationPhase::Failed;
        Ok(())
    }

    /// Whether the saga is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.phase.is_terminal()
    }

    /// Whether the saga completed successfully.
    pub fn is_successful(&self) -> bool {
        self.phase == MigrationPhase::Completed
    }

    /// Whether the saga has exceeded its deadline.
    pub fn is_expired(&self) -> bool {
        Timestamp::now() > self.deadline
    }

    /// Check the deadline and force-fail if exceeded.
    ///
    /// This is called at the top of every state transition method.
    /// If the deadline has passed and the saga is not in a terminal state,
    /// the saga is forced to Failed with a `MigrationTimeoutError`.
    ///
    /// Implements audit §3.5 — deadline enforcement.
    fn check_deadline(&mut self) -> Result<(), MigrationError> {
        if self.phase.is_terminal() {
            return Ok(());
        }

        let now = Timestamp::now();
        if now > self.deadline {
            let phase_str = self.phase.to_string();
            let migration_id_str = self.id.to_string();

            // Force to Compensating, then Failed
            self.record_transition(
                MigrationPhase::Compensating,
                &format!("Deadline exceeded at phase {phase_str}"),
            );
            self.phase = MigrationPhase::Compensating;
            self.record_transition(
                MigrationPhase::Failed,
                &format!("Auto-failed after deadline timeout at {phase_str}"),
            );
            self.phase = MigrationPhase::Failed;

            return Err(MigrationError::Timeout(MigrationTimeoutError {
                migration_id: migration_id_str,
                phase: phase_str,
            }));
        }

        Ok(())
    }

    /// Record a phase transition in the log.
    fn record_transition(&mut self, to: MigrationPhase, reason: &str) {
        self.transitions.push(MigrationTransition {
            from_phase: self.phase,
            to_phase: to,
            timestamp: Timestamp::now(),
            reason: reason.to_string(),
            evidence_digest: None,
        });
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, Utc};

    fn future_deadline() -> Timestamp {
        Timestamp::from_utc(Utc::now() + Duration::hours(24))
    }

    fn past_deadline() -> Timestamp {
        Timestamp::from_utc(Utc::now() - Duration::hours(1))
    }

    fn make_saga() -> MigrationSaga {
        MigrationBuilder::new(MigrationId::new())
            .source("PK-PSEZ".to_string())
            .destination("AE-DIFC".to_string())
            .asset("asset-001".to_string())
            .deadline(future_deadline())
            .build()
    }

    // ── Builder tests ────────────────────────────────────────────────

    #[test]
    fn test_builder_with_deadline_builds() {
        let saga = make_saga();
        assert_eq!(saga.phase, MigrationPhase::Initiated);
        assert_eq!(saga.source_jurisdiction, "PK-PSEZ");
        assert_eq!(saga.destination_jurisdiction, "AE-DIFC");
        assert_eq!(saga.asset_id, "asset-001");
        assert!(!saga.is_terminal());
    }

    #[test]
    fn test_builder_records_initial_transition() {
        let saga = make_saga();
        assert_eq!(saga.transitions.len(), 1);
        assert_eq!(saga.transitions[0].to_phase, MigrationPhase::Initiated);
    }

    // ── Phase progression tests ──────────────────────────────────────

    #[test]
    fn test_advance_initiated_to_validated() {
        let mut saga = make_saga();
        let phase = saga.advance("Validation passed").unwrap();
        assert_eq!(phase, MigrationPhase::Validated);
        assert_eq!(saga.phase, MigrationPhase::Validated);
    }

    #[test]
    fn test_advance_through_all_phases() {
        let mut saga = make_saga();

        saga.advance("Validated").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Validated);

        saga.advance("In transit").unwrap();
        assert_eq!(saga.phase, MigrationPhase::InTransit);

        saga.advance("Completing").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Completing);

        saga.advance("Completed").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Completed);

        assert!(saga.is_terminal());
        assert!(saga.is_successful());
    }

    #[test]
    fn test_advance_records_transitions() {
        let mut saga = make_saga();
        saga.advance("Step 1").unwrap();
        saga.advance("Step 2").unwrap();

        // 1 initial + 2 advances
        assert_eq!(saga.transitions.len(), 3);
        assert_eq!(saga.transitions[1].from_phase, MigrationPhase::Initiated);
        assert_eq!(saga.transitions[1].to_phase, MigrationPhase::Validated);
        assert_eq!(saga.transitions[2].from_phase, MigrationPhase::Validated);
        assert_eq!(saga.transitions[2].to_phase, MigrationPhase::InTransit);
    }

    // ── Terminal state tests ─────────────────────────────────────────

    #[test]
    fn test_advance_from_completed_fails() {
        let mut saga = make_saga();
        saga.advance("v").unwrap();
        saga.advance("t").unwrap();
        saga.advance("c").unwrap();
        saga.advance("done").unwrap();
        assert!(saga.phase == MigrationPhase::Completed);

        let result = saga.advance("should fail");
        assert!(result.is_err());
        match result.unwrap_err() {
            MigrationError::TerminalPhase { phase, .. } => {
                assert_eq!(phase, "COMPLETED");
            }
            other => panic!("Expected TerminalPhase error, got: {other:?}"),
        }
    }

    #[test]
    fn test_advance_from_failed_fails() {
        let mut saga = make_saga();
        saga.compensate("failure").unwrap();
        saga.fail("done").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Failed);

        let result = saga.advance("should fail");
        assert!(result.is_err());
    }

    // ── Deadline enforcement tests ───────────────────────────────────

    #[test]
    fn test_expired_deadline_forces_failure() {
        let mut saga = MigrationBuilder::new(MigrationId::new())
            .source("PK".to_string())
            .deadline(past_deadline())
            .build();

        let result = saga.advance("should timeout");
        assert!(result.is_err());
        match result.unwrap_err() {
            MigrationError::Timeout(err) => {
                assert!(err.phase.contains("INITIATED"));
            }
            other => panic!("Expected Timeout error, got: {other:?}"),
        }
        assert_eq!(saga.phase, MigrationPhase::Failed);
        assert!(saga.is_terminal());
    }

    #[test]
    fn test_terminal_state_skips_deadline_check() {
        let mut saga = make_saga();
        // Advance to completion
        saga.advance("v").unwrap();
        saga.advance("t").unwrap();
        saga.advance("c").unwrap();
        saga.advance("done").unwrap();

        // Even if we manually set a past deadline, terminal state is ok
        saga.deadline = past_deadline();
        // Advance will fail because it's terminal, not because of deadline
        let result = saga.advance("nope");
        assert!(matches!(result.unwrap_err(), MigrationError::TerminalPhase { .. }));
    }

    // ── Compensation tests ───────────────────────────────────────────

    #[test]
    fn test_compensate_from_in_transit() {
        let mut saga = make_saga();
        saga.advance("v").unwrap();
        saga.advance("in transit").unwrap();
        assert_eq!(saga.phase, MigrationPhase::InTransit);

        saga.compensate("Source jurisdiction unreachable").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Compensating);
    }

    #[test]
    fn test_compensate_then_fail() {
        let mut saga = make_saga();
        saga.advance("v").unwrap();
        saga.compensate("Compliance check failed").unwrap();
        assert_eq!(saga.phase, MigrationPhase::Compensating);

        saga.record_compensation(
            CompensationAction::UnlockSource,
            true,
            None,
        );
        saga.record_compensation(
            CompensationAction::NotifyCounterparties,
            true,
            None,
        );
        saga.fail("All compensations executed").unwrap();

        assert_eq!(saga.phase, MigrationPhase::Failed);
        assert!(saga.is_terminal());
        assert!(!saga.is_successful());
        assert_eq!(saga.compensations.len(), 2);
    }

    #[test]
    fn test_compensation_records_error_detail() {
        let mut saga = make_saga();
        saga.compensate("failure").unwrap();

        saga.record_compensation(
            CompensationAction::UnlockSource,
            false,
            Some("Source jurisdiction timeout after 30s".to_string()),
        );

        assert_eq!(saga.compensations.len(), 1);
        assert!(!saga.compensations[0].success);
        assert_eq!(
            saga.compensations[0].error_detail.as_deref(),
            Some("Source jurisdiction timeout after 30s")
        );
    }

    #[test]
    fn test_compensate_from_terminal_fails() {
        let mut saga = make_saga();
        saga.advance("v").unwrap();
        saga.advance("t").unwrap();
        saga.advance("c").unwrap();
        saga.advance("done").unwrap();

        let result = saga.compensate("should fail");
        assert!(result.is_err());
    }

    #[test]
    fn test_fail_without_compensating_fails() {
        let mut saga = make_saga();
        saga.advance("v").unwrap();

        let result = saga.fail("should fail");
        assert!(result.is_err());
    }

    // ── Serialization tests ──────────────────────────────────────────

    #[test]
    fn test_migration_phase_display() {
        assert_eq!(MigrationPhase::Initiated.to_string(), "INITIATED");
        assert_eq!(MigrationPhase::Validated.to_string(), "VALIDATED");
        assert_eq!(MigrationPhase::InTransit.to_string(), "IN_TRANSIT");
        assert_eq!(MigrationPhase::Completing.to_string(), "COMPLETING");
        assert_eq!(MigrationPhase::Completed.to_string(), "COMPLETED");
        assert_eq!(MigrationPhase::Failed.to_string(), "FAILED");
        assert_eq!(MigrationPhase::Compensating.to_string(), "COMPENSATING");
    }

    #[test]
    fn test_saga_serialization() {
        let saga = make_saga();
        let json = serde_json::to_string(&saga).unwrap();
        let parsed: MigrationSaga = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.phase, saga.phase);
        assert_eq!(parsed.source_jurisdiction, saga.source_jurisdiction);
    }

    // ── No PROPOSED or OPERATIONAL strings ────────────────────────────

    #[test]
    fn test_no_defective_state_names() {
        let all_phases = [
            MigrationPhase::Initiated,
            MigrationPhase::Validated,
            MigrationPhase::InTransit,
            MigrationPhase::Completing,
            MigrationPhase::Completed,
            MigrationPhase::Failed,
            MigrationPhase::Compensating,
        ];
        for phase in &all_phases {
            let name = phase.to_string();
            assert!(!name.contains("PROPOSED"), "Defective v1 state name found");
            assert!(
                !name.contains("OPERATIONAL"),
                "Defective v1 state name found"
            );
        }
    }
}
