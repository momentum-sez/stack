//! # Compliance Tensor
//!
//! Multi-domain compliance evaluation for an entity within a jurisdiction.

use std::collections::HashMap;

use msez_core::{ComplianceDomain, JurisdictionId};

use crate::evaluation::ComplianceState;

/// A compliance tensor mapping each domain to a compliance state
/// for a specific entity and jurisdiction.
///
/// Generic over the jurisdiction context to support both single-zone
/// and cross-corridor compliance evaluation.
#[derive(Debug, Clone)]
pub struct ComplianceTensor {
    /// The jurisdiction this tensor evaluates against.
    pub jurisdiction: JurisdictionId,
    /// Compliance state per domain.
    pub states: HashMap<ComplianceDomain, ComplianceState>,
}

impl ComplianceTensor {
    /// Create a new compliance tensor for a jurisdiction with all
    /// domains initialized to `Pending`.
    pub fn new(jurisdiction: JurisdictionId) -> Self {
        let states = ComplianceDomain::all()
            .iter()
            .map(|&d| (d, ComplianceState::Pending))
            .collect();
        Self {
            jurisdiction,
            states,
        }
    }
}
