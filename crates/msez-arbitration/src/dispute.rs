//! # Dispute Lifecycle
//!
//! Models the lifecycle of disputes between corridor participants.
//!
//! ## States
//!
//! FILED → RESPONSE → HEARING → RESOLVED | DISMISSED
//!
//! ## Implements
//!
//! Spec §22 — Dispute lifecycle protocol.

use serde::{Deserialize, Serialize};

/// The state of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeState {
    /// Dispute has been filed.
    Filed,
    /// Respondent has been notified and may respond.
    Response,
    /// Hearing is in progress.
    Hearing,
    /// Dispute has been resolved with an award (terminal).
    Resolved,
    /// Dispute has been dismissed (terminal).
    Dismissed,
}

impl DisputeState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Resolved | Self::Dismissed)
    }
}

/// A dispute between corridor participants.
///
/// Placeholder — full implementation will include claimant/respondent
/// identifiers, evidence packages, and award details.
#[derive(Debug)]
pub struct Dispute {
    /// Current dispute state.
    pub state: DisputeState,
}
