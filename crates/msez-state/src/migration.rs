//! # Migration Saga State Machine
//!
//! Implements the 8-phase cross-jurisdiction asset migration saga with
//! 3 terminal states and compile-time deadline enforcement.
//!
//! ## Phases (spec §42)
//!
//! INITIATED → VALIDATED → LOCKED → TRANSIT → RECEIVED → VERIFIED → SETTLED → COMPLETED
//!
//! Terminal states: COMPLETED, COMPENSATED, FAILED
//!
//! ## Security Invariant
//!
//! The deadline field is checked at every phase transition. If the migration
//! has exceeded its deadline and is not in a terminal state, automatic
//! compensation is triggered. This prevents permanent asset lock (audit §3.5).
//!
//! ## Implements
//!
//! Spec §42 — Cross-jurisdiction asset migration protocol.

use msez_core::{MigrationId, Timestamp};
use thiserror::Error;

/// Error when a migration exceeds its deadline.
#[derive(Error, Debug)]
#[error("migration {migration_id} exceeded deadline at state {state}")]
pub struct MigrationTimeoutError {
    /// The migration that timed out.
    pub migration_id: String,
    /// The state the migration was in when it timed out.
    pub state: String,
}

/// The phase of a migration saga.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MigrationPhase {
    /// Migration has been initiated.
    Initiated,
    /// Source jurisdiction has validated the migration request.
    Validated,
    /// Source assets are locked pending transfer.
    Locked,
    /// Assets are in transit between jurisdictions.
    Transit,
    /// Target jurisdiction has received the assets.
    Received,
    /// Target jurisdiction has verified asset integrity.
    Verified,
    /// Settlement has been completed.
    Settled,
    /// Migration successfully completed (terminal).
    Completed,
    /// Migration was compensated after failure (terminal).
    Compensated,
    /// Migration failed irrecoverably (terminal).
    Failed,
}

impl MigrationPhase {
    /// Whether this phase is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Compensated | Self::Failed
        )
    }
}

impl std::fmt::Display for MigrationPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Initiated => "INITIATED",
            Self::Validated => "VALIDATED",
            Self::Locked => "LOCKED",
            Self::Transit => "TRANSIT",
            Self::Received => "RECEIVED",
            Self::Verified => "VERIFIED",
            Self::Settled => "SETTLED",
            Self::Completed => "COMPLETED",
            Self::Compensated => "COMPENSATED",
            Self::Failed => "FAILED",
        };
        f.write_str(s)
    }
}

/// A cross-jurisdiction asset migration saga.
///
/// Placeholder — full implementation will use typestate or enum-based
/// phase transitions with deadline enforcement at every step.
#[derive(Debug)]
pub struct MigrationSaga {
    /// Unique migration identifier.
    pub id: MigrationId,
    /// Current phase of the migration.
    pub phase: MigrationPhase,
    /// Deadline for the migration (checked at every transition).
    pub deadline: Option<Timestamp>,
}

impl MigrationSaga {
    /// Create a new migration saga in the INITIATED phase.
    pub fn new(id: MigrationId, deadline: Option<Timestamp>) -> Self {
        Self {
            id,
            phase: MigrationPhase::Initiated,
            deadline,
        }
    }
}
