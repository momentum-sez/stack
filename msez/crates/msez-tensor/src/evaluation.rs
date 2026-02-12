//! # Compliance State
//!
//! The result of evaluating an entity's compliance for a specific domain.

use serde::{Deserialize, Serialize};

/// The compliance evaluation result for a single domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceState {
    /// Entity is compliant with all requirements in this domain.
    Compliant,
    /// Entity is not compliant — specific violations exist.
    NonCompliant,
    /// Compliance evaluation is in progress.
    Pending,
    /// This domain does not apply to the entity's classification.
    NotApplicable,
}
