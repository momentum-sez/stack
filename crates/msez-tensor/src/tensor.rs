//! # Compliance Tensor
//!
//! The core data structure mapping (jurisdiction, domain) pairs to
//! compliance states.
//!
//! ## Security Invariant
//!
//! Uses `ComplianceDomain` from `msez-core` — the single source of truth
//! for all 20 domains. Exhaustive match on `ComplianceDomain` in evaluation
//! logic ensures no domain is silently skipped.
//!
//! ## Implements
//!
//! Spec §12 — Compliance tensor T: J × D → S.

use std::collections::HashMap;

use msez_core::{ComplianceDomain, JurisdictionId};
use serde::{Deserialize, Serialize};

/// The compliance state for a single (jurisdiction, domain) evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceState {
    /// Entity satisfies all requirements for this domain.
    Compliant,
    /// Entity does not satisfy requirements for this domain.
    NonCompliant,
    /// Compliance evaluation is in progress.
    Pending,
    /// Entity is exempt from this domain in this jurisdiction.
    Exempt,
    /// Compliance state has not been evaluated.
    Unknown,
}

/// A compliance tensor mapping (jurisdiction, domain) → state.
///
/// Generic over jurisdiction representation to support both single-zone
/// and multi-zone evaluation contexts.
///
/// Placeholder — full implementation will include tensor algebra
/// operations, slicing by domain/jurisdiction, and commitment generation.
#[derive(Debug, Clone)]
pub struct ComplianceTensor {
    /// The tensor data: jurisdiction → (domain → state).
    states: HashMap<JurisdictionId, HashMap<ComplianceDomain, ComplianceState>>,
}

impl ComplianceTensor {
    /// Create an empty compliance tensor.
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    /// Get the compliance state for a (jurisdiction, domain) pair.
    pub fn get(
        &self,
        jurisdiction: &JurisdictionId,
        domain: &ComplianceDomain,
    ) -> ComplianceState {
        self.states
            .get(jurisdiction)
            .and_then(|domains| domains.get(domain))
            .copied()
            .unwrap_or(ComplianceState::Unknown)
    }

    /// Set the compliance state for a (jurisdiction, domain) pair.
    pub fn set(
        &mut self,
        jurisdiction: JurisdictionId,
        domain: ComplianceDomain,
        state: ComplianceState,
    ) {
        self.states
            .entry(jurisdiction)
            .or_default()
            .insert(domain, state);
    }
}

impl Default for ComplianceTensor {
    fn default() -> Self {
        Self::new()
    }
}
