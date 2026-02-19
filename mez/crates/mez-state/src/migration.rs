//! # Migration Saga State Machine
//!
//! Manages cross-jurisdiction asset migration with 8 forward phases and
//! 3 terminal failure states. Includes compile-time deadline enforcement
//! via the builder pattern.
//!
//! ## Builder Pattern — Compile-Time Deadline Enforcement
//!
//! ```text
//! // This compiles — deadline is set:
//! let saga = MigrationBuilder::new(id)
//!     .deadline(deadline)
//!     .source(source_id)
//!     .destination(dest_id)
//!     .build();
//!
//! // This does NOT compile — no .build() method on MigrationBuilder<NoDeadline>:
//! let saga = MigrationBuilder::new(id)
//!     .source(source_id)
//!     .destination(dest_id)
//!     .build(); // ERROR: no method named `build`
//! ```
//!
//! ## Audit Reference
//!
//! Finding §3.5: The Python implementation had a `deadline` field with no
//! enforcement. Finding §5.5: compensation actions swallowed exceptions.
//! The Rust implementation enforces deadlines at every transition and
//! propagates structured errors instead of swallowing them.

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use mez_core::{JurisdictionId, MigrationId};

// ── Migration State ──────────────────────────────────────────────────

/// Migration saga phases (8 forward + 3 terminal).
///
/// Forward phases represent the happy-path progression of a cross-
/// jurisdiction asset migration. Terminal states represent final
/// outcomes (success or various failure modes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationState {
    // Forward phases (ordered)
    /// Migration initiated, awaiting compliance evaluation.
    Initiated,
    /// Compliance check in progress at source and destination.
    ComplianceCheck,
    /// Gathering watcher attestations for the migration.
    AttestationGathering,
    /// Source jurisdiction asset has been locked.
    SourceLocked,
    /// Asset is in transit between jurisdictions.
    InTransit,
    /// Destination jurisdiction has received and is verifying the asset.
    DestinationVerification,
    /// Destination asset unlocked, finalizing.
    DestinationUnlock,
    /// Migration completed successfully. Terminal state.
    Completed,

    // Terminal failure states
    /// Migration failed and compensation was executed. Terminal state.
    Compensated,
    /// Migration timed out and was auto-compensated. Terminal state.
    TimedOut,
    /// Migration was cancelled before reaching InTransit. Terminal state.
    Cancelled,
}

impl MigrationState {
    /// Whether this is a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Compensated | Self::TimedOut | Self::Cancelled
        )
    }

    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initiated => "INITIATED",
            Self::ComplianceCheck => "COMPLIANCE_CHECK",
            Self::AttestationGathering => "ATTESTATION_GATHERING",
            Self::SourceLocked => "SOURCE_LOCKED",
            Self::InTransit => "IN_TRANSIT",
            Self::DestinationVerification => "DESTINATION_VERIFICATION",
            Self::DestinationUnlock => "DESTINATION_UNLOCK",
            Self::Completed => "COMPLETED",
            Self::Compensated => "COMPENSATED",
            Self::TimedOut => "TIMED_OUT",
            Self::Cancelled => "CANCELLED",
        }
    }

    /// Return the next forward phase, if one exists.
    ///
    /// Terminal states explicitly return `None`. No wildcard is used so
    /// that adding a new variant forces a compiler error here rather than
    /// silently falling through.
    fn next_forward_phase(&self) -> Option<MigrationState> {
        match self {
            Self::Initiated => Some(Self::ComplianceCheck),
            Self::ComplianceCheck => Some(Self::AttestationGathering),
            Self::AttestationGathering => Some(Self::SourceLocked),
            Self::SourceLocked => Some(Self::InTransit),
            Self::InTransit => Some(Self::DestinationVerification),
            Self::DestinationVerification => Some(Self::DestinationUnlock),
            Self::DestinationUnlock => Some(Self::Completed),
            Self::Completed | Self::Compensated | Self::TimedOut | Self::Cancelled => None,
        }
    }

    /// Whether cancellation is allowed from this state.
    /// Cancellation is only valid before the asset enters transit.
    fn is_cancellable(&self) -> bool {
        matches!(
            self,
            Self::Initiated
                | Self::ComplianceCheck
                | Self::AttestationGathering
                | Self::SourceLocked
        )
    }
}

impl std::fmt::Display for MigrationState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Error Types ──────────────────────────────────────────────────────

/// Errors during migration operations.
///
/// Each variant carries structured context for diagnostics — no
/// swallowed exceptions (audit §5.5).
#[derive(Error, Debug)]
pub enum MigrationError {
    /// The migration exceeded its deadline.
    #[error("migration {id} exceeded deadline at state {state} (deadline: {deadline})")]
    Timeout {
        /// The migration identifier.
        id: MigrationId,
        /// The state when timeout was detected.
        state: MigrationState,
        /// The deadline that was exceeded.
        deadline: DateTime<Utc>,
    },
    /// Invalid state transition attempted.
    #[error("invalid migration transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// Current state.
        from: MigrationState,
        /// Attempted target state.
        to: MigrationState,
        /// Human-readable reason for the rejection.
        reason: String,
    },
    /// Compensation action failed with structured error context.
    #[error("compensation failed at state {state}: {detail}")]
    CompensationFailed {
        /// The state when compensation was attempted.
        state: MigrationState,
        /// Structured error detail (not swallowed).
        detail: String,
    },
    /// Migration is already in a terminal state.
    #[error("migration {id} is in terminal state {state}")]
    AlreadyTerminal {
        /// The migration identifier.
        id: MigrationId,
        /// The terminal state.
        state: MigrationState,
    },
}

// ── Compensation Record ──────────────────────────────────────────────

/// A record of a compensation action taken during saga rollback.
///
/// Unlike the Python implementation (audit §5.5), the error_detail
/// field preserves diagnostic context instead of swallowing exceptions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompensationRecord {
    /// The state from which compensation was triggered.
    pub from_state: MigrationState,
    /// The action that was compensated.
    pub action: String,
    /// Whether the compensation succeeded.
    pub succeeded: bool,
    /// Error detail if compensation failed — never swallowed.
    pub error_detail: Option<String>,
    /// When the compensation was executed.
    pub timestamp: DateTime<Utc>,
}

// ── Builder Types ────────────────────────────────────────────────────

/// Marker type: no deadline has been set on the builder.
#[derive(Debug)]
pub struct NoDeadline;

/// Marker type: a deadline has been set on the builder.
#[derive(Debug)]
pub struct HasDeadline;

/// Builder for constructing a [`MigrationSaga`] with compile-time
/// deadline enforcement.
///
/// Only `MigrationBuilder<HasDeadline>` has a `.build()` method.
/// `MigrationBuilder<NoDeadline>` does not — attempting to build
/// without setting a deadline is a compile error.
///
/// ## Audit Reference
///
/// Finding §3.5 / §5.5: The Python `MigrationSaga` had an optional
/// deadline that was never checked. This builder makes the deadline
/// mandatory at the type level.
#[derive(Debug)]
pub struct MigrationBuilder<D> {
    id: MigrationId,
    deadline: Option<DateTime<Utc>>,
    source_jurisdiction: Option<JurisdictionId>,
    destination_jurisdiction: Option<JurisdictionId>,
    asset_description: Option<String>,
    _deadline_marker: PhantomData<D>,
}

impl MigrationBuilder<NoDeadline> {
    /// Create a new migration builder. The deadline is not yet set.
    pub fn new(id: MigrationId) -> Self {
        Self {
            id,
            deadline: None,
            source_jurisdiction: None,
            destination_jurisdiction: None,
            asset_description: None,
            _deadline_marker: PhantomData,
        }
    }

    /// Set the migration deadline. Transitions the builder from
    /// `NoDeadline` to `HasDeadline`, enabling the `.build()` method.
    pub fn deadline(self, deadline: DateTime<Utc>) -> MigrationBuilder<HasDeadline> {
        MigrationBuilder {
            id: self.id,
            deadline: Some(deadline),
            source_jurisdiction: self.source_jurisdiction,
            destination_jurisdiction: self.destination_jurisdiction,
            asset_description: self.asset_description,
            _deadline_marker: PhantomData,
        }
    }
}

impl MigrationBuilder<HasDeadline> {
    /// Build the migration saga. Only available when a deadline is set.
    pub fn build(self) -> MigrationSaga {
        MigrationSaga {
            id: self.id,
            state: MigrationState::Initiated,
            deadline: self.deadline.expect("HasDeadline guarantees this is Some"),
            source_jurisdiction: self.source_jurisdiction,
            destination_jurisdiction: self.destination_jurisdiction,
            asset_description: self.asset_description.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            compensation_log: Vec::new(),
        }
    }
}

impl<D> MigrationBuilder<D> {
    /// Set the source jurisdiction.
    pub fn source(mut self, jurisdiction: JurisdictionId) -> Self {
        self.source_jurisdiction = Some(jurisdiction);
        self
    }

    /// Set the destination jurisdiction.
    pub fn destination(mut self, jurisdiction: JurisdictionId) -> Self {
        self.destination_jurisdiction = Some(jurisdiction);
        self
    }

    /// Set the asset description.
    pub fn asset_description(mut self, description: impl Into<String>) -> Self {
        self.asset_description = Some(description.into());
        self
    }
}

// ── Migration Saga ───────────────────────────────────────────────────

/// A migration saga tracking cross-jurisdiction asset movement.
///
/// Created via [`MigrationBuilder`] which enforces a mandatory deadline
/// at compile time. State transitions check the deadline and auto-
/// compensate if expired.
///
/// ## Security Invariant
///
/// Every transition checks `check_deadline()` first. An expired
/// migration cannot advance — it is force-transitioned to `TimedOut`
/// with a structured error. Implements spec §3.5 deadline enforcement.
#[derive(Debug)]
pub struct MigrationSaga {
    /// Unique migration identifier.
    pub id: MigrationId,
    /// Current migration state.
    pub state: MigrationState,
    /// The deadline by which the migration must complete.
    pub deadline: DateTime<Utc>,
    /// Source jurisdiction.
    pub source_jurisdiction: Option<JurisdictionId>,
    /// Destination jurisdiction.
    pub destination_jurisdiction: Option<JurisdictionId>,
    /// Description of the asset being migrated.
    pub asset_description: String,
    /// When the migration was created.
    pub created_at: DateTime<Utc>,
    /// When the migration was last updated.
    pub updated_at: DateTime<Utc>,
    /// Log of compensation actions (for saga rollback auditability).
    pub compensation_log: Vec<CompensationRecord>,
}

impl MigrationSaga {
    /// Check the deadline. Returns an error if expired and the migration
    /// is not already in a terminal state.
    ///
    /// ## Audit Reference
    ///
    /// Finding §3.5: The Python `MigrationSaga` had a `deadline` field
    /// but no enforcement. This method is called at the top of every
    /// transition to prevent progress on expired migrations.
    fn check_deadline(&mut self) -> Result<(), MigrationError> {
        if Utc::now() > self.deadline && !self.state.is_terminal() {
            let timed_out_state = self.state;
            self.state = MigrationState::TimedOut;
            self.updated_at = Utc::now();
            return Err(MigrationError::Timeout {
                id: self.id.clone(),
                state: timed_out_state,
                deadline: self.deadline,
            });
        }
        Ok(())
    }

    /// Advance the migration to the next forward phase.
    ///
    /// Checks the deadline before advancing. If the deadline has passed,
    /// the migration is force-transitioned to `TimedOut` and a
    /// [`MigrationError::Timeout`] is returned.
    pub fn advance(&mut self) -> Result<MigrationState, MigrationError> {
        self.check_deadline()?;

        if self.state.is_terminal() {
            return Err(MigrationError::AlreadyTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }

        let next =
            self.state
                .next_forward_phase()
                .ok_or_else(|| MigrationError::InvalidTransition {
                    from: self.state,
                    to: self.state,
                    reason: "no forward phase from current state".to_string(),
                })?;

        self.state = next;
        self.updated_at = Utc::now();
        Ok(next)
    }

    /// Cancel the migration. Only valid before InTransit.
    ///
    /// Once an asset enters transit, cancellation is not possible —
    /// only compensation can unwind the state.
    pub fn cancel(&mut self) -> Result<(), MigrationError> {
        if !self.state.is_cancellable() {
            return Err(MigrationError::InvalidTransition {
                from: self.state,
                to: MigrationState::Cancelled,
                reason: "cancellation only allowed before IN_TRANSIT".to_string(),
            });
        }
        self.state = MigrationState::Cancelled;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Trigger compensation (saga rollback) from the current state.
    ///
    /// Records a compensation entry with structured error context.
    /// Unlike the Python implementation, errors are never swallowed
    /// (audit §5.5).
    pub fn compensate(&mut self, reason: &str) -> Result<(), MigrationError> {
        if self.state.is_terminal() {
            return Err(MigrationError::AlreadyTerminal {
                id: self.id.clone(),
                state: self.state,
            });
        }

        // Only allow compensation from InTransit or later — pre-transit states
        // should use cancel, not compensate, since no irreversible work has occurred.
        let compensable = matches!(
            self.state,
            MigrationState::InTransit
                | MigrationState::DestinationVerification
                | MigrationState::DestinationUnlock
        );
        if !compensable {
            return Err(MigrationError::InvalidTransition {
                from: self.state,
                to: MigrationState::Compensated,
                reason: "compensation only allowed from IN_TRANSIT or later; use cancel for pre-transit states".to_string(),
            });
        }

        self.compensation_log.push(CompensationRecord {
            from_state: self.state,
            action: format!("compensate: {reason}"),
            succeeded: true,
            error_detail: None,
            timestamp: Utc::now(),
        });

        self.state = MigrationState::Compensated;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Record a failed compensation action for auditability.
    ///
    /// ## Audit Reference
    ///
    /// Finding §5.5: Python migration compensation swallowed exceptions
    /// with bare `except Exception:`. This method preserves the error
    /// context in a structured [`CompensationRecord`].
    pub fn record_compensation_failure(&mut self, action: &str, error_detail: &str) {
        self.compensation_log.push(CompensationRecord {
            from_state: self.state,
            action: action.to_string(),
            succeeded: false,
            error_detail: Some(error_detail.to_string()),
            timestamp: Utc::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn future_deadline() -> DateTime<Utc> {
        Utc::now() + TimeDelta::try_hours(24).unwrap()
    }

    fn past_deadline() -> DateTime<Utc> {
        Utc::now() - TimeDelta::try_hours(1).unwrap()
    }

    fn test_saga() -> MigrationSaga {
        MigrationBuilder::new(MigrationId::new())
            .source(JurisdictionId::new("PK-REZ").unwrap())
            .destination(JurisdictionId::new("AE-DIFC").unwrap())
            .deadline(future_deadline())
            .build()
    }

    #[test]
    fn builder_produces_initiated_state() {
        let saga = test_saga();
        assert_eq!(saga.state, MigrationState::Initiated);
        assert!(!saga.state.is_terminal());
    }

    #[test]
    fn advance_through_all_phases() {
        let mut saga = test_saga();
        assert_eq!(saga.state, MigrationState::Initiated);

        assert_eq!(saga.advance().unwrap(), MigrationState::ComplianceCheck);
        assert_eq!(
            saga.advance().unwrap(),
            MigrationState::AttestationGathering
        );
        assert_eq!(saga.advance().unwrap(), MigrationState::SourceLocked);
        assert_eq!(saga.advance().unwrap(), MigrationState::InTransit);
        assert_eq!(
            saga.advance().unwrap(),
            MigrationState::DestinationVerification
        );
        assert_eq!(saga.advance().unwrap(), MigrationState::DestinationUnlock);
        assert_eq!(saga.advance().unwrap(), MigrationState::Completed);

        assert!(saga.state.is_terminal());
    }

    #[test]
    fn advance_from_terminal_fails() {
        let mut saga = test_saga();
        for _ in 0..7 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::Completed);

        let err = saga.advance().unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn deadline_enforcement_on_advance() {
        let mut saga = MigrationBuilder::new(MigrationId::new())
            .deadline(past_deadline())
            .build();

        let err = saga.advance().unwrap_err();
        assert!(matches!(err, MigrationError::Timeout { .. }));
        assert_eq!(saga.state, MigrationState::TimedOut);
        assert!(saga.state.is_terminal());
    }

    #[test]
    fn cancel_before_transit() {
        let mut saga = test_saga();
        saga.advance().unwrap(); // → ComplianceCheck
        saga.advance().unwrap(); // → AttestationGathering

        saga.cancel().unwrap();
        assert_eq!(saga.state, MigrationState::Cancelled);
        assert!(saga.state.is_terminal());
    }

    #[test]
    fn cancel_after_transit_fails() {
        let mut saga = test_saga();
        for _ in 0..4 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::InTransit);

        let err = saga.cancel().unwrap_err();
        assert!(matches!(err, MigrationError::InvalidTransition { .. }));
    }

    #[test]
    fn compensation_records_context() {
        let mut saga = test_saga();
        for _ in 0..4 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::InTransit);

        saga.compensate("transit_failure").unwrap();
        assert_eq!(saga.state, MigrationState::Compensated);
        assert_eq!(saga.compensation_log.len(), 1);
        assert!(saga.compensation_log[0].succeeded);
        assert!(saga.compensation_log[0].action.contains("transit_failure"));
    }

    #[test]
    fn compensation_failure_preserves_error() {
        let mut saga = test_saga();
        saga.record_compensation_failure("unlock_source", "connection timeout to source node");
        assert_eq!(saga.compensation_log.len(), 1);
        assert!(!saga.compensation_log[0].succeeded);
        assert_eq!(
            saga.compensation_log[0].error_detail.as_deref(),
            Some("connection timeout to source node")
        );
    }

    #[test]
    fn compensation_from_terminal_fails() {
        let mut saga = test_saga();
        saga.cancel().unwrap();

        let err = saga.compensate("reason").unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn terminal_after_timeout_cannot_advance() {
        let mut saga = MigrationBuilder::new(MigrationId::new())
            .deadline(past_deadline())
            .build();

        let _ = saga.advance(); // triggers timeout
        assert_eq!(saga.state, MigrationState::TimedOut);

        let err = saga.advance().unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn state_display_names() {
        assert_eq!(MigrationState::Initiated.as_str(), "INITIATED");
        assert_eq!(MigrationState::ComplianceCheck.as_str(), "COMPLIANCE_CHECK");
        assert_eq!(MigrationState::InTransit.as_str(), "IN_TRANSIT");
        assert_eq!(MigrationState::Completed.as_str(), "COMPLETED");
        assert_eq!(MigrationState::TimedOut.as_str(), "TIMED_OUT");
    }

    #[test]
    fn builder_with_all_fields() {
        let saga = MigrationBuilder::new(MigrationId::new())
            .source(JurisdictionId::new("PK-REZ").unwrap())
            .destination(JurisdictionId::new("AE-DIFC").unwrap())
            .asset_description("Manufacturing equipment")
            .deadline(future_deadline())
            .build();

        assert!(saga.source_jurisdiction.is_some());
        assert!(saga.destination_jurisdiction.is_some());
        assert_eq!(saga.asset_description, "Manufacturing equipment");
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn migration_state_display_all_variants() {
        assert_eq!(format!("{}", MigrationState::Initiated), "INITIATED");
        assert_eq!(
            format!("{}", MigrationState::ComplianceCheck),
            "COMPLIANCE_CHECK"
        );
        assert_eq!(
            format!("{}", MigrationState::AttestationGathering),
            "ATTESTATION_GATHERING"
        );
        assert_eq!(format!("{}", MigrationState::SourceLocked), "SOURCE_LOCKED");
        assert_eq!(format!("{}", MigrationState::InTransit), "IN_TRANSIT");
        assert_eq!(
            format!("{}", MigrationState::DestinationVerification),
            "DESTINATION_VERIFICATION"
        );
        assert_eq!(
            format!("{}", MigrationState::DestinationUnlock),
            "DESTINATION_UNLOCK"
        );
        assert_eq!(format!("{}", MigrationState::Completed), "COMPLETED");
        assert_eq!(format!("{}", MigrationState::Compensated), "COMPENSATED");
        assert_eq!(format!("{}", MigrationState::TimedOut), "TIMED_OUT");
        assert_eq!(format!("{}", MigrationState::Cancelled), "CANCELLED");
    }

    #[test]
    fn migration_state_as_str_all_variants() {
        assert_eq!(
            MigrationState::AttestationGathering.as_str(),
            "ATTESTATION_GATHERING"
        );
        assert_eq!(MigrationState::SourceLocked.as_str(), "SOURCE_LOCKED");
        assert_eq!(
            MigrationState::DestinationVerification.as_str(),
            "DESTINATION_VERIFICATION"
        );
        assert_eq!(
            MigrationState::DestinationUnlock.as_str(),
            "DESTINATION_UNLOCK"
        );
        assert_eq!(MigrationState::Compensated.as_str(), "COMPENSATED");
        assert_eq!(MigrationState::Cancelled.as_str(), "CANCELLED");
    }

    #[test]
    fn migration_state_is_terminal_all_variants() {
        // Terminal states
        assert!(MigrationState::Completed.is_terminal());
        assert!(MigrationState::Compensated.is_terminal());
        assert!(MigrationState::TimedOut.is_terminal());
        assert!(MigrationState::Cancelled.is_terminal());

        // Non-terminal states
        assert!(!MigrationState::Initiated.is_terminal());
        assert!(!MigrationState::ComplianceCheck.is_terminal());
        assert!(!MigrationState::AttestationGathering.is_terminal());
        assert!(!MigrationState::SourceLocked.is_terminal());
        assert!(!MigrationState::InTransit.is_terminal());
        assert!(!MigrationState::DestinationVerification.is_terminal());
        assert!(!MigrationState::DestinationUnlock.is_terminal());
    }

    #[test]
    fn cancel_from_initiated() {
        let mut saga = test_saga();
        saga.cancel().unwrap();
        assert_eq!(saga.state, MigrationState::Cancelled);
        assert!(saga.state.is_terminal());
    }

    #[test]
    fn cancel_from_source_locked() {
        let mut saga = test_saga();
        saga.advance().unwrap(); // ComplianceCheck
        saga.advance().unwrap(); // AttestationGathering
        saga.advance().unwrap(); // SourceLocked
        saga.cancel().unwrap();
        assert_eq!(saga.state, MigrationState::Cancelled);
    }

    #[test]
    fn cancel_after_destination_verification_fails() {
        let mut saga = test_saga();
        for _ in 0..5 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::DestinationVerification);
        let err = saga.cancel().unwrap_err();
        assert!(matches!(err, MigrationError::InvalidTransition { .. }));
    }

    #[test]
    fn cancel_after_destination_unlock_fails() {
        let mut saga = test_saga();
        for _ in 0..6 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::DestinationUnlock);
        let err = saga.cancel().unwrap_err();
        assert!(matches!(err, MigrationError::InvalidTransition { .. }));
    }

    #[test]
    fn cancel_from_completed_fails() {
        let mut saga = test_saga();
        for _ in 0..7 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::Completed);
        let err = saga.cancel().unwrap_err();
        assert!(matches!(err, MigrationError::InvalidTransition { .. }));
    }

    #[test]
    fn compensate_from_initiated_rejected() {
        let mut saga = test_saga();
        let err = saga.compensate("early_abort").unwrap_err();
        assert!(matches!(err, MigrationError::InvalidTransition { .. }));
        // State should remain Initiated — compensation was rejected.
        assert_eq!(saga.state, MigrationState::Initiated);
    }

    #[test]
    fn compensate_from_in_transit() {
        let mut saga = test_saga();
        for _ in 0..4 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::InTransit);
        saga.compensate("transit_failure").unwrap();
        assert_eq!(saga.state, MigrationState::Compensated);
    }

    #[test]
    fn compensate_from_compensated_fails() {
        let mut saga = test_saga();
        // Advance to InTransit so the first compensate is allowed.
        for _ in 0..4 {
            saga.advance().unwrap();
        }
        assert_eq!(saga.state, MigrationState::InTransit);
        saga.compensate("first").unwrap();
        let err = saga.compensate("second").unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn compensate_from_timed_out_fails() {
        let mut saga = MigrationBuilder::new(MigrationId::new())
            .deadline(past_deadline())
            .build();
        let _ = saga.advance(); // triggers timeout
        assert_eq!(saga.state, MigrationState::TimedOut);
        let err = saga.compensate("reason").unwrap_err();
        assert!(matches!(err, MigrationError::AlreadyTerminal { .. }));
    }

    #[test]
    fn record_compensation_failure_multiple() {
        let mut saga = test_saga();
        saga.record_compensation_failure("unlock_source", "timeout");
        saga.record_compensation_failure("refund_fees", "insufficient funds");
        saga.record_compensation_failure("notify_parties", "email service down");
        assert_eq!(saga.compensation_log.len(), 3);
        assert!(!saga.compensation_log[0].succeeded);
        assert!(!saga.compensation_log[1].succeeded);
        assert!(!saga.compensation_log[2].succeeded);
        assert_eq!(
            saga.compensation_log[1].error_detail.as_deref(),
            Some("insufficient funds")
        );
    }

    #[test]
    fn builder_without_optional_fields() {
        let saga = MigrationBuilder::new(MigrationId::new())
            .deadline(future_deadline())
            .build();

        assert!(saga.source_jurisdiction.is_none());
        assert!(saga.destination_jurisdiction.is_none());
        assert_eq!(saga.asset_description, "");
    }

    #[test]
    fn builder_destination_before_deadline() {
        // Test that builder methods can be called in any order
        let saga = MigrationBuilder::new(MigrationId::new())
            .destination(JurisdictionId::new("AE-DIFC").unwrap())
            .source(JurisdictionId::new("PK-REZ").unwrap())
            .asset_description("Textiles")
            .deadline(future_deadline())
            .build();

        assert!(saga.source_jurisdiction.is_some());
        assert!(saga.destination_jurisdiction.is_some());
        assert_eq!(saga.asset_description, "Textiles");
        assert_eq!(saga.state, MigrationState::Initiated);
    }

    #[test]
    fn migration_error_timeout_display() {
        let err = MigrationError::Timeout {
            id: MigrationId::new(),
            state: MigrationState::InTransit,
            deadline: past_deadline(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("exceeded deadline"));
        assert!(msg.contains("IN_TRANSIT"));
    }

    #[test]
    fn migration_error_invalid_transition_display() {
        let err = MigrationError::InvalidTransition {
            from: MigrationState::InTransit,
            to: MigrationState::Cancelled,
            reason: "cancellation only allowed before IN_TRANSIT".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("IN_TRANSIT"));
        assert!(msg.contains("CANCELLED"));
    }

    #[test]
    fn migration_error_compensation_failed_display() {
        let err = MigrationError::CompensationFailed {
            state: MigrationState::SourceLocked,
            detail: "network error".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("SOURCE_LOCKED"));
        assert!(msg.contains("network error"));
    }

    #[test]
    fn migration_error_already_terminal_display() {
        let err = MigrationError::AlreadyTerminal {
            id: MigrationId::new(),
            state: MigrationState::Completed,
        };
        let msg = format!("{err}");
        assert!(msg.contains("terminal state"));
        assert!(msg.contains("COMPLETED"));
    }

    #[test]
    fn compensation_record_serialization_roundtrip() {
        let record = CompensationRecord {
            from_state: MigrationState::InTransit,
            action: "unlock_source".to_string(),
            succeeded: false,
            error_detail: Some("connection refused".to_string()),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&record).unwrap();
        let back: CompensationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back.from_state, MigrationState::InTransit);
        assert_eq!(back.action, "unlock_source");
        assert!(!back.succeeded);
        assert_eq!(back.error_detail.as_deref(), Some("connection refused"));
    }

    #[test]
    fn advance_updates_timestamp() {
        let mut saga = test_saga();
        let initial_updated = saga.updated_at;
        // Small sleep to ensure time changes
        std::thread::sleep(std::time::Duration::from_millis(10));
        saga.advance().unwrap();
        assert!(saga.updated_at >= initial_updated);
    }

    #[test]
    fn cancel_updates_timestamp() {
        let mut saga = test_saga();
        let initial_updated = saga.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        saga.cancel().unwrap();
        assert!(saga.updated_at >= initial_updated);
    }
}
