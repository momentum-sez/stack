//! # Compliance Tensor
//!
//! Multi-domain compliance evaluation for an entity within a jurisdiction.
//!
//! The tensor maps each of the 20 [`ComplianceDomain`] variants to a
//! [`ComplianceState`] for a specific entity/jurisdiction pair. It supports
//! both explicit state storage and dynamic evaluation via pluggable
//! [`DomainEvaluator`] implementations.
//!
//! ## Audit Reference
//!
//! Finding §2.4: The Python `tensor.py` had only 8 domains while
//! `composition.py` had 20. This implementation uses the single
//! [`ComplianceDomain`] enum from `msez-core`
//! with all 20 variants, and every `match` is exhaustive.
//!
//! ## Generic Parameter
//!
//! `ComplianceTensor<J>` is generic over `J: JurisdictionConfig`, allowing
//! jurisdiction-specific evaluation rules while sharing the core tensor
//! operations.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use msez_core::{ComplianceDomain, JurisdictionId};

use crate::commitment::TensorCommitment;
use crate::evaluation::{
    evaluate_domain_default, AttestationRef, ComplianceState, DomainEvaluator, EvaluationContext,
};

// ---------------------------------------------------------------------------
// JurisdictionConfig Trait
// ---------------------------------------------------------------------------

/// Configuration trait for jurisdiction-specific compliance evaluation.
///
/// Implementations provide the jurisdiction identity and optionally
/// restrict which domains are applicable. The default implementation
/// considers all 20 domains applicable.
///
/// ## Exhaustiveness
///
/// `applicable_domains()` returns a subset of `ComplianceDomain::all()`.
/// The tensor still stores cells for all 20 domains (non-applicable
/// domains are initialized to `NotApplicable`).
pub trait JurisdictionConfig: Clone + std::fmt::Debug + Send + Sync + 'static {
    /// The jurisdiction this configuration applies to.
    fn jurisdiction_id(&self) -> &JurisdictionId;

    /// Which domains are applicable in this jurisdiction.
    ///
    /// Defaults to all 20 domains. Override to restrict evaluation to
    /// a subset (e.g., a trade-only zone might only need AML, KYC,
    /// SANCTIONS, TAX, TRADE).
    fn applicable_domains(&self) -> &[ComplianceDomain] {
        ComplianceDomain::all()
    }
}

/// Default jurisdiction configuration that considers all 20 domains applicable.
#[derive(Debug, Clone)]
pub struct DefaultJurisdiction {
    id: JurisdictionId,
}

impl DefaultJurisdiction {
    /// Create a default jurisdiction configuration.
    pub fn new(id: JurisdictionId) -> Self {
        Self { id }
    }
}

impl JurisdictionConfig for DefaultJurisdiction {
    fn jurisdiction_id(&self) -> &JurisdictionId {
        &self.id
    }
}

// ---------------------------------------------------------------------------
// TensorCell
// ---------------------------------------------------------------------------

/// A single cell in the compliance tensor.
///
/// Each cell stores the compliance state for one domain, along with
/// the evidentiary basis (attestations) and metadata about when and
/// why the state was determined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorCell {
    /// The compliance state for this domain.
    pub state: ComplianceState,
    /// Attestations supporting this state determination.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attestations: Vec<AttestationRef>,
    /// ISO 8601 timestamp when this state was determined.
    pub determined_at: String,
    /// Optional reason code explaining the state determination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl TensorCell {
    /// Create a new cell with a state and current timestamp.
    fn new(state: ComplianceState) -> Self {
        Self {
            state,
            attestations: Vec::new(),
            determined_at: chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            reason: None,
        }
    }
}

// ---------------------------------------------------------------------------
// TensorSlice
// ---------------------------------------------------------------------------

/// A dimensional slice of a compliance tensor over selected domains.
///
/// Slices are immutable snapshots used for reporting, comparison, and
/// aggregate compliance assessment.
#[derive(Debug, Clone)]
pub struct TensorSlice {
    /// The domain→state pairs in this slice.
    pub cells: HashMap<ComplianceDomain, ComplianceState>,
}

impl TensorSlice {
    /// Aggregate all states in the slice using the lattice `meet` operation.
    ///
    /// Returns `Compliant` for an empty slice (neutral element of meet).
    pub fn aggregate_state(&self) -> ComplianceState {
        self.cells
            .values()
            .copied()
            .fold(ComplianceState::Compliant, ComplianceState::meet)
    }

    /// Check if all domains in the slice are in a passing state.
    pub fn all_passing(&self) -> bool {
        self.cells.values().all(|s| s.is_passing())
    }

    /// Return domains that are `NonCompliant`.
    pub fn non_compliant_domains(&self) -> Vec<ComplianceDomain> {
        self.cells
            .iter()
            .filter(|(_, &s)| s == ComplianceState::NonCompliant)
            .map(|(&d, _)| d)
            .collect()
    }

    /// Return domains that are `Pending`.
    pub fn pending_domains(&self) -> Vec<ComplianceDomain> {
        self.cells
            .iter()
            .filter(|(_, &s)| s == ComplianceState::Pending)
            .map(|(&d, _)| d)
            .collect()
    }

    /// Number of domains in this slice.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    /// Whether this slice is empty.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}

// ---------------------------------------------------------------------------
// ComplianceTensor
// ---------------------------------------------------------------------------

/// A compliance tensor mapping each domain to a compliance state
/// for a specific entity and jurisdiction.
///
/// Generic over `J: JurisdictionConfig` to support jurisdiction-specific
/// evaluation rules while sharing the core tensor operations.
///
/// ## Dimensions
///
/// 20 [`ComplianceDomain`] variants × jurisdiction-specific rules.
/// Non-applicable domains (per `J::applicable_domains()`) are initialized
/// to `NotApplicable`.
///
/// ## Evaluation
///
/// Two evaluation modes:
/// 1. **Stored**: `get()` returns the explicitly set state.
/// 2. **Dynamic**: `evaluate()` uses registered evaluators or the default
///    evaluation logic (attestation freshness for original 8 domains,
///    `NotApplicable` for extended 12).
///
/// ## Security Invariant
///
/// Every `match` on `ComplianceDomain` in this module is exhaustive.
/// Adding a 21st domain forces a compile error until handled.
pub struct ComplianceTensor<J: JurisdictionConfig> {
    /// The jurisdiction context for this tensor.
    jurisdiction: J,
    /// Compliance state per domain.
    cells: HashMap<ComplianceDomain, TensorCell>,
    /// Optional custom evaluators per domain.
    evaluators: HashMap<ComplianceDomain, Box<dyn DomainEvaluator>>,
}

impl<J: JurisdictionConfig> std::fmt::Debug for ComplianceTensor<J> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComplianceTensor")
            .field("jurisdiction", &self.jurisdiction)
            .field("cell_count", &self.cells.len())
            .finish()
    }
}

impl<J: JurisdictionConfig> ComplianceTensor<J> {
    /// Create a new compliance tensor for a jurisdiction with all 20 domains
    /// initialized based on applicability.
    ///
    /// Applicable domains start as `Pending`; non-applicable domains start
    /// as `NotApplicable`.
    pub fn new(jurisdiction: J) -> Self {
        let applicable: std::collections::HashSet<ComplianceDomain> =
            jurisdiction.applicable_domains().iter().copied().collect();

        let cells = ComplianceDomain::all()
            .iter()
            .map(|&d| {
                let state = if applicable.contains(&d) {
                    ComplianceState::Pending
                } else {
                    ComplianceState::NotApplicable
                };
                (d, TensorCell::new(state))
            })
            .collect();

        Self {
            jurisdiction,
            cells,
            evaluators: HashMap::new(),
        }
    }

    /// Access the jurisdiction configuration.
    pub fn jurisdiction(&self) -> &J {
        &self.jurisdiction
    }

    /// Number of cells in the tensor (always 20 for a complete tensor).
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Set the compliance state for a domain, with optional attestations
    /// and reason code.
    pub fn set(
        &mut self,
        domain: ComplianceDomain,
        state: ComplianceState,
        attestations: Vec<AttestationRef>,
        reason: Option<String>,
    ) {
        let cell = self
            .cells
            .entry(domain)
            .or_insert_with(|| TensorCell::new(state));
        cell.state = state;
        cell.attestations = attestations;
        cell.reason = reason;
        cell.determined_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    }

    /// Get the stored compliance state for a domain.
    ///
    /// Returns `Pending` if the domain has no stored cell (should not happen
    /// for a properly initialized tensor).
    pub fn get(&self, domain: ComplianceDomain) -> ComplianceState {
        self.cells
            .get(&domain)
            .map(|c| c.state)
            .unwrap_or(ComplianceState::Pending)
    }

    /// Get the full cell for a domain, if it exists.
    pub fn get_cell(&self, domain: ComplianceDomain) -> Option<&TensorCell> {
        self.cells.get(&domain)
    }

    /// Register a custom evaluator for a domain.
    ///
    /// Custom evaluators override the default evaluation logic when
    /// [`evaluate()`](Self::evaluate) is called.
    pub fn set_evaluator(&mut self, domain: ComplianceDomain, evaluator: Box<dyn DomainEvaluator>) {
        self.evaluators.insert(domain, evaluator);
    }

    /// Evaluate compliance for a single domain.
    ///
    /// If a custom evaluator is registered, delegates to it. Otherwise uses
    /// the default evaluation logic with exhaustive domain matching.
    ///
    /// ## Exhaustive Match (inside evaluate_domain_default)
    ///
    /// The default path uses an exhaustive `match` on all 20
    /// `ComplianceDomain` variants. Adding a 21st domain is a compile error.
    pub fn evaluate(&self, entity_id: &str, domain: ComplianceDomain) -> ComplianceState {
        let ctx = self.build_context(entity_id, domain);

        // Check for custom evaluator first.
        if let Some(evaluator) = self.evaluators.get(&domain) {
            let (state, _) = evaluator.evaluate(&ctx);
            return state;
        }

        // Fall back to default evaluation with exhaustive match.
        let (state, _) = evaluate_domain_default(domain, &ctx);
        state
    }

    /// Evaluate compliance across all 20 domains.
    ///
    /// Returns a complete map from every domain to its evaluated state.
    pub fn evaluate_all(&self, entity_id: &str) -> HashMap<ComplianceDomain, ComplianceState> {
        ComplianceDomain::all()
            .iter()
            .map(|&domain| (domain, self.evaluate(entity_id, domain)))
            .collect()
    }

    /// Compute a cryptographic commitment to the current tensor state.
    ///
    /// Uses the `CanonicalBytes → ContentDigest` pipeline from `msez-core`,
    /// ensuring the commitment is computed with proper canonicalization.
    ///
    /// ## Security Invariant
    ///
    /// This is the exact location where the Python canonicalization split
    /// was most dangerous. The Rust version makes the split impossible
    /// because `sha256_digest()` only accepts `&CanonicalBytes`.
    pub fn commit(&self) -> Result<TensorCommitment, msez_core::MsezError> {
        TensorCommitment::compute(self)
    }

    /// Merge another tensor into this one using the lattice `meet` operation.
    ///
    /// For each domain, the resulting state is `meet(self, other)` —
    /// the more restrictive state wins.
    pub fn merge(&mut self, other: &ComplianceTensor<J>) {
        for &domain in ComplianceDomain::all() {
            let self_state = self.get(domain);
            let other_state = other.get(domain);
            let merged = self_state.meet(other_state);
            if merged != self_state {
                self.set(domain, merged, Vec::new(), Some("merged".into()));
            }
        }
    }

    /// Create a slice of the tensor over selected domains.
    pub fn slice(&self, domains: &[ComplianceDomain]) -> TensorSlice {
        let cells = domains.iter().map(|&d| (d, self.get(d))).collect();
        TensorSlice { cells }
    }

    /// Create a slice over all domains.
    pub fn full_slice(&self) -> TensorSlice {
        self.slice(ComplianceDomain::all())
    }

    /// Build an evaluation context for a domain.
    fn build_context(&self, entity_id: &str, domain: ComplianceDomain) -> EvaluationContext {
        let cell = self.cells.get(&domain);
        EvaluationContext {
            entity_id: entity_id.to_string(),
            current_state: cell.map(|c| c.state),
            attestations: cell.map(|c| c.attestations.clone()).unwrap_or_default(),
            metadata: HashMap::new(),
        }
    }

    /// Return a serializable representation of all cells.
    ///
    /// Cells are sorted by domain name for deterministic serialization.
    pub(crate) fn to_serializable_cells(&self) -> Vec<(String, ComplianceState)> {
        let mut cells: Vec<_> = ComplianceDomain::all()
            .iter()
            .map(|&d| (d.as_str().to_string(), self.get(d)))
            .collect();
        cells.sort_by(|a, b| a.0.cmp(&b.0));
        cells
    }
}

impl<J: JurisdictionConfig> Clone for ComplianceTensor<J> {
    fn clone(&self) -> Self {
        // Clone cells and jurisdiction; evaluators are not cloneable,
        // so the clone starts with no custom evaluators.
        Self {
            jurisdiction: self.jurisdiction.clone(),
            cells: self.cells.clone(),
            evaluators: HashMap::new(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_jurisdiction() -> DefaultJurisdiction {
        DefaultJurisdiction::new(JurisdictionId::new("PK-RSEZ").unwrap())
    }

    #[test]
    fn new_tensor_has_20_cells() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        assert_eq!(tensor.cell_count(), 20);
    }

    #[test]
    fn new_tensor_all_domains_pending() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        for &domain in ComplianceDomain::all() {
            assert_eq!(
                tensor.get(domain),
                ComplianceState::Pending,
                "domain {domain} should be Pending in new tensor"
            );
        }
    }

    #[test]
    fn set_and_get_roundtrip() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        assert_eq!(
            tensor.get(ComplianceDomain::Aml),
            ComplianceState::Compliant
        );
    }

    #[test]
    fn evaluate_all_returns_20_entries() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let results = tensor.evaluate_all("entity-1");
        assert_eq!(results.len(), 20);
        for &domain in ComplianceDomain::all() {
            assert!(
                results.contains_key(&domain),
                "evaluate_all missing domain {domain}"
            );
        }
    }

    #[test]
    fn slice_over_selected_domains() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Kyc,
            ComplianceState::NonCompliant,
            vec![],
            None,
        );

        let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
        assert_eq!(slice.len(), 2);
        assert_eq!(
            slice.cells[&ComplianceDomain::Aml],
            ComplianceState::Compliant
        );
        assert_eq!(
            slice.cells[&ComplianceDomain::Kyc],
            ComplianceState::NonCompliant
        );
    }

    #[test]
    fn slice_aggregate_uses_meet() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Kyc,
            ComplianceState::Pending,
            vec![],
            None,
        );

        let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
        assert_eq!(slice.aggregate_state(), ComplianceState::Pending);
    }

    #[test]
    fn non_compliant_domains_in_slice() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(
            ComplianceDomain::Sanctions,
            ComplianceState::NonCompliant,
            vec![],
            None,
        );

        let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Sanctions]);
        let nc = slice.non_compliant_domains();
        assert_eq!(nc.len(), 1);
        assert!(nc.contains(&ComplianceDomain::Sanctions));
    }

    #[test]
    fn merge_takes_more_restrictive() {
        let mut a = ComplianceTensor::new(test_jurisdiction());
        a.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        a.set(
            ComplianceDomain::Kyc,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let mut b = ComplianceTensor::new(test_jurisdiction());
        b.set(
            ComplianceDomain::Aml,
            ComplianceState::NonCompliant,
            vec![],
            None,
        );
        b.set(
            ComplianceDomain::Kyc,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        a.merge(&b);
        assert_eq!(
            a.get(ComplianceDomain::Aml),
            ComplianceState::NonCompliant,
            "merge should take NonCompliant (more restrictive)"
        );
        assert_eq!(
            a.get(ComplianceDomain::Kyc),
            ComplianceState::Compliant,
            "merge should keep Compliant when both agree"
        );
    }

    #[test]
    fn full_slice_has_20_cells() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let slice = tensor.full_slice();
        assert_eq!(slice.len(), 20);
    }

    #[test]
    fn serializable_cells_are_sorted() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let cells = tensor.to_serializable_cells();
        assert_eq!(cells.len(), 20);
        for i in 0..cells.len() - 1 {
            assert!(
                cells[i].0 < cells[i + 1].0,
                "cells not sorted: {} >= {}",
                cells[i].0,
                cells[i + 1].0
            );
        }
    }

    #[test]
    fn clone_preserves_state_but_not_evaluators() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let cloned = tensor.clone();
        assert_eq!(
            cloned.get(ComplianceDomain::Aml),
            ComplianceState::Compliant
        );
        assert!(cloned.evaluators.is_empty());
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn tensor_debug_format() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let debug_str = format!("{tensor:?}");
        assert!(debug_str.contains("ComplianceTensor"));
        assert!(debug_str.contains("cell_count"));
    }

    #[test]
    fn tensor_jurisdiction_accessor() {
        let jur = test_jurisdiction();
        let tensor = ComplianceTensor::new(jur);
        assert_eq!(tensor.jurisdiction().jurisdiction_id().as_str(), "PK-RSEZ");
    }

    #[test]
    fn tensor_get_cell_returns_some() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let cell = tensor.get_cell(ComplianceDomain::Aml);
        assert!(cell.is_some());
        assert_eq!(cell.unwrap().state, ComplianceState::Pending);
    }

    #[test]
    fn tensor_set_with_reason() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Tax,
            ComplianceState::Compliant,
            vec![],
            Some("Manual review passed".to_string()),
        );
        let cell = tensor.get_cell(ComplianceDomain::Tax).unwrap();
        assert_eq!(cell.state, ComplianceState::Compliant);
        assert_eq!(cell.reason, Some("Manual review passed".to_string()));
    }

    #[test]
    fn tensor_set_with_attestations() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        let attestation = AttestationRef {
            attestation_id: "att-1".to_string(),
            attestation_type: "kyc_verification".to_string(),
            issuer_did: "did:key:z6MkTest".to_string(),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: Some("2027-01-01T00:00:00Z".to_string()),
            digest: "test_digest".to_string(),
        };
        tensor.set(
            ComplianceDomain::Kyc,
            ComplianceState::Compliant,
            vec![attestation.clone()],
            None,
        );
        let cell = tensor.get_cell(ComplianceDomain::Kyc).unwrap();
        assert_eq!(cell.attestations.len(), 1);
        assert_eq!(cell.attestations[0].issuer_did, "did:key:z6MkTest");
    }

    #[test]
    fn tensor_slice_empty() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let slice = tensor.slice(&[]);
        assert!(slice.is_empty());
        assert_eq!(slice.len(), 0);
        assert_eq!(slice.aggregate_state(), ComplianceState::Compliant);
        assert!(slice.all_passing());
        assert!(slice.non_compliant_domains().is_empty());
        assert!(slice.pending_domains().is_empty());
    }

    #[test]
    fn tensor_slice_pending_domains() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
        let pending = slice.pending_domains();
        assert_eq!(pending.len(), 2);
    }

    #[test]
    fn tensor_slice_all_passing_false_with_noncompliant() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::NonCompliant,
            vec![],
            None,
        );
        let slice = tensor.slice(&[ComplianceDomain::Aml]);
        assert!(!slice.all_passing());
    }

    #[test]
    fn tensor_slice_all_passing_true_with_compliant() {
        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );
        tensor.set(ComplianceDomain::Kyc, ComplianceState::Exempt, vec![], None);
        let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
        assert!(slice.all_passing());
    }

    #[test]
    fn tensor_evaluate_single_domain() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let state = tensor.evaluate("entity-1", ComplianceDomain::Aml);
        // Default evaluation should return a valid ComplianceState
        assert!(matches!(
            state,
            ComplianceState::Pending
                | ComplianceState::NonCompliant
                | ComplianceState::Compliant
                | ComplianceState::NotApplicable
                | ComplianceState::Exempt
        ));
    }

    #[test]
    fn tensor_custom_evaluator() {
        #[derive(Debug)]
        struct AlwaysCompliant;
        impl DomainEvaluator for AlwaysCompliant {
            fn domain(&self) -> ComplianceDomain {
                ComplianceDomain::Aml
            }
            fn evaluate(&self, _ctx: &EvaluationContext) -> (ComplianceState, Option<String>) {
                (ComplianceState::Compliant, Some("Always compliant".into()))
            }
        }

        let mut tensor = ComplianceTensor::new(test_jurisdiction());
        tensor.set_evaluator(ComplianceDomain::Aml, Box::new(AlwaysCompliant));
        let state = tensor.evaluate("entity-1", ComplianceDomain::Aml);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn tensor_commit_produces_valid_commitment() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let commitment = tensor.commit();
        assert!(commitment.is_ok());
        let commitment = commitment.unwrap();
        assert_eq!(commitment.digest().to_hex().len(), 64);
    }

    #[test]
    fn tensor_merge_no_change_when_same() {
        let mut a = ComplianceTensor::new(test_jurisdiction());
        a.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        let mut b = ComplianceTensor::new(test_jurisdiction());
        b.set(
            ComplianceDomain::Aml,
            ComplianceState::Compliant,
            vec![],
            None,
        );

        a.merge(&b);
        assert_eq!(a.get(ComplianceDomain::Aml), ComplianceState::Compliant);
    }

    #[test]
    fn non_applicable_jurisdiction() {
        #[derive(Debug, Clone)]
        struct TradeOnlyJurisdiction {
            id: JurisdictionId,
        }
        impl JurisdictionConfig for TradeOnlyJurisdiction {
            fn jurisdiction_id(&self) -> &JurisdictionId {
                &self.id
            }
            fn applicable_domains(&self) -> &[ComplianceDomain] {
                &[ComplianceDomain::Aml, ComplianceDomain::Kyc]
            }
        }

        let jur = TradeOnlyJurisdiction {
            id: JurisdictionId::new("TRADE-ZONE").unwrap(),
        };
        let tensor = ComplianceTensor::new(jur);
        assert_eq!(tensor.cell_count(), 20);
        // Applicable domains should be Pending
        assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Pending);
        assert_eq!(tensor.get(ComplianceDomain::Kyc), ComplianceState::Pending);
        // Non-applicable domains should be NotApplicable
        assert_eq!(
            tensor.get(ComplianceDomain::Tax),
            ComplianceState::NotApplicable
        );
        assert_eq!(
            tensor.get(ComplianceDomain::Securities),
            ComplianceState::NotApplicable
        );
    }

    #[test]
    fn tensor_cell_determined_at_is_valid_timestamp() {
        let tensor = ComplianceTensor::new(test_jurisdiction());
        let cell = tensor.get_cell(ComplianceDomain::Aml).unwrap();
        assert!(cell.determined_at.contains("T"));
        assert!(cell.determined_at.ends_with("Z"));
    }
}
