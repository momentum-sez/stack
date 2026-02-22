//! # Domain-Specific Evaluators
//!
//! Custom `DomainEvaluator` implementations that replace the default
//! attestation-freshness logic with domain-specific evaluation.

use std::sync::Arc;

use mez_core::ComplianceDomain;
use mez_pack::regpack::SanctionsChecker;
use mez_tensor::{ComplianceState, DomainEvaluator, EvaluationContext};

/// Evaluates the SANCTIONS domain by screening entities against a
/// `SanctionsChecker` loaded from a regpack's sanctions snapshot.
///
/// ## Evaluation Logic
///
/// 1. Extract entity name from `ctx.metadata["entity_name"]`.
/// 2. If no entity name is available, return `Pending` (cannot screen).
/// 3. Run `SanctionsChecker::check_entity()` with the configured threshold.
/// 4. If matched → `NonCompliant` with match details.
/// 5. If no match → `Compliant`.
///
/// ## Security Note
///
/// The sanctions checker uses fuzzy name matching (Jaccard similarity)
/// with a configurable threshold. The default 0.7 threshold balances
/// false-positive avoidance against evasion risk. Production deployments
/// should tune this based on the jurisdiction's regulatory guidance.
pub struct SanctionsEvaluator {
    checker: Arc<SanctionsChecker>,
    threshold: f64,
}

impl std::fmt::Debug for SanctionsEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SanctionsEvaluator")
            .field("threshold", &self.threshold)
            .finish_non_exhaustive()
    }
}

impl SanctionsEvaluator {
    /// Create a new sanctions evaluator from a loaded checker.
    ///
    /// The threshold must be in (0.0, 1.0]. Values outside this range
    /// are clamped: <= 0.0 becomes 0.01 (match nearly everything),
    /// > 1.0 becomes 1.0 (exact match only), NaN becomes default 0.7.
    pub fn new(checker: Arc<SanctionsChecker>, threshold: f64) -> Self {
        let threshold = if threshold.is_nan() {
            tracing::warn!("SanctionsEvaluator: NaN threshold, using default 0.7");
            0.7
        } else if threshold <= 0.0 {
            tracing::warn!(threshold, "SanctionsEvaluator: threshold <= 0.0, clamping to 0.01");
            0.01
        } else if threshold > 1.0 {
            tracing::warn!(threshold, "SanctionsEvaluator: threshold > 1.0, clamping to 1.0");
            1.0
        } else {
            threshold
        };
        Self { checker, threshold }
    }

    /// Create with the default threshold (0.7).
    pub fn with_default_threshold(checker: Arc<SanctionsChecker>) -> Self {
        Self::new(checker, 0.7)
    }
}

impl DomainEvaluator for SanctionsEvaluator {
    fn domain(&self) -> ComplianceDomain {
        ComplianceDomain::Sanctions
    }

    fn evaluate(&self, ctx: &EvaluationContext) -> (ComplianceState, Option<String>) {
        let entity_name = ctx.metadata.get("entity_name").and_then(|v| v.as_str());

        let entity_name = match entity_name {
            Some(name) if !name.is_empty() => name,
            _ => {
                return (
                    ComplianceState::Pending,
                    Some("no entity_name in evaluation context".into()),
                );
            }
        };

        let result = self.checker.check_entity(entity_name, None, self.threshold);

        if result.matched {
            let detail = format!(
                "sanctions match: score={:.2}, entries={}",
                result.match_score,
                result.matches.len()
            );
            (ComplianceState::NonCompliant, Some(detail))
        } else {
            (ComplianceState::Compliant, Some("sanctions_clear".into()))
        }
    }
}
