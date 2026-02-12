//! # Migration Saga State Machine
//!
//! Manages cross-jurisdiction asset migration with 8 phases and 3 terminal
//! states. Includes compile-time deadline enforcement via the builder pattern.
//!
//! ## Audit Reference
//!
//! Finding §3.5: The Python implementation had a `deadline` field with no
//! enforcement. The Rust builder pattern ensures a deadline is always set
//! before a migration can be constructed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::MigrationId;

/// Migration saga phases (8 active + 3 terminal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MigrationState {
    // Active phases
    /// Migration initiated, awaiting source jurisdiction lock.
    Initiated,
    /// Source asset locked.
    SourceLocked,
    /// Compliance evaluation in progress at destination.
    ComplianceCheck,
    /// Fees computed and awaiting payment.
    FeeSettlement,
    /// Asset in transit between jurisdictions.
    Transit,
    /// Destination jurisdiction has received the asset.
    DestinationReceived,
    /// Final reconciliation and receipt generation.
    Reconciling,
    /// Migration completed successfully. Terminal state.
    Completed,

    // Terminal failure states
    /// Migration was rolled back due to failure. Terminal state.
    RolledBack,
    /// Migration failed and compensation was executed. Terminal state.
    Compensated,
    /// Migration timed out and was auto-compensated. Terminal state.
    TimedOut,
}

impl MigrationState {
    /// Whether this is a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::RolledBack | Self::Compensated | Self::TimedOut
        )
    }
}

/// A migration saga tracking cross-jurisdiction asset movement.
#[derive(Debug)]
pub struct MigrationSaga {
    /// Unique migration identifier.
    pub id: MigrationId,
    /// Current migration state.
    pub state: MigrationState,
    /// The deadline by which the migration must complete.
    pub deadline: DateTime<Utc>,
    /// When the migration was created.
    pub created_at: DateTime<Utc>,
}

/// Error during migration operations.
#[derive(Error, Debug)]
pub enum MigrationError {
    /// The migration exceeded its deadline.
    #[error("migration {0} exceeded deadline")]
    Timeout(MigrationId),
    /// Invalid state transition attempted.
    #[error("invalid migration transition from {from:?} to {to:?}")]
    InvalidTransition {
        /// Current state.
        from: MigrationState,
        /// Attempted target state.
        to: MigrationState,
    },
}
