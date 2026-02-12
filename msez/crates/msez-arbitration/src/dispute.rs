//! # Dispute Lifecycle
//!
//! Manages dispute initiation, claim filing, and lifecycle stages.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use msez_core::EntityId;

/// The lifecycle state of a dispute.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DisputeState {
    /// Dispute has been filed.
    Filed,
    /// Evidence collection phase.
    EvidenceCollection,
    /// Hearing in progress.
    Hearing,
    /// Deliberation by arbitrators.
    Deliberation,
    /// Award has been issued.
    Awarded,
    /// Award is being enforced.
    Enforcing,
    /// Dispute has been settled. Terminal state.
    Settled,
    /// Dispute was dismissed. Terminal state.
    Dismissed,
}

/// A dispute between two or more entities.
#[derive(Debug)]
pub struct Dispute {
    /// Current dispute state.
    pub state: DisputeState,
    /// The claimant entity.
    pub claimant: EntityId,
    /// The respondent entity.
    pub respondent: EntityId,
    /// When the dispute was filed.
    pub filed_at: DateTime<Utc>,
}
