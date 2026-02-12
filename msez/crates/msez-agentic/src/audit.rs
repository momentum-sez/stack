//! # Policy Audit Trail
//!
//! Records every trigger evaluation and action execution for
//! regulatory review.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// An entry in the policy audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// When this audit event occurred.
    pub timestamp: DateTime<Utc>,
    /// The policy that was evaluated.
    pub policy_id: String,
    /// Whether the trigger condition was met.
    pub trigger_matched: bool,
    /// The action taken (if any).
    pub action_taken: Option<String>,
}
