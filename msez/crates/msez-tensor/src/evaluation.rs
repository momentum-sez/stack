//! # Compliance State & Domain Evaluation
//!
//! Defines [`ComplianceState`] with lattice operations, the [`DomainEvaluator`]
//! trait for domain-specific evaluation logic, and the default evaluator that
//! covers all 20 [`ComplianceDomain`] variants with exhaustive matching.
//!
//! ## Audit Reference
//!
//! Finding §2.4: The Python `tensor.py` defined 8 domains; `composition.py`
//! defined 20. This module evaluates all 20 domains from the single
//! [`ComplianceDomain`](msez_core::ComplianceDomain) enum in `msez-core`.
//! Every `match` on `ComplianceDomain` is exhaustive — adding a 21st domain
//! is a compile error until every evaluation path is updated.
//!
//! ## Spec Reference
//!
//! Implements the compliance state lattice from `tools/phoenix/tensor.py`
//! and the 20-domain coverage from `tools/msez/composition.py`.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use msez_core::ComplianceDomain;

// ---------------------------------------------------------------------------
// ComplianceState
// ---------------------------------------------------------------------------

/// The compliance evaluation result for a single domain.
///
/// States follow a strict lattice for pessimistic composition:
///
/// ```text
/// Ordering (worst → best): NonCompliant < Pending < NotApplicable < Exempt < Compliant
///
/// meet(a, b) = min(a, b)  — pessimistic (both must pass)
/// join(a, b) = max(a, b)  — optimistic  (either suffices)
/// ```
///
/// `NonCompliant` is absorbing under `meet`: any domain that is non-compliant
/// causes the aggregate to be non-compliant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComplianceState {
    /// Entity is compliant with all requirements in this domain.
    Compliant,
    /// Entity is not compliant — specific violations exist.
    NonCompliant,
    /// Compliance evaluation is in progress or awaiting attestation.
    Pending,
    /// Entity is exempt from requirements in this domain (e.g., de minimis).
    Exempt,
    /// This domain does not apply to the entity's classification.
    NotApplicable,
}

impl ComplianceState {
    /// Lattice ordering value. Lower is "worse" (more restrictive).
    fn ordering(self) -> u8 {
        match self {
            Self::NonCompliant => 0,
            Self::Pending => 1,
            Self::NotApplicable => 2,
            Self::Exempt => 3,
            Self::Compliant => 4,
        }
    }

    /// Lattice meet (greatest lower bound) — pessimistic composition.
    ///
    /// Returns the more restrictive of the two states. Used when combining
    /// compliance across multiple domains where ALL must pass.
    ///
    /// # Security Invariant
    ///
    /// `NonCompliant` is absorbing: `meet(x, NonCompliant) == NonCompliant`
    /// for all x. This ensures a single violation blocks the aggregate.
    pub fn meet(self, other: Self) -> Self {
        if self.ordering() <= other.ordering() {
            self
        } else {
            other
        }
    }

    /// Lattice join (least upper bound) — optimistic composition.
    ///
    /// Returns the less restrictive of the two states. Used when ANY
    /// of multiple attestation sources suffices.
    pub fn join(self, other: Self) -> Self {
        if self.ordering() >= other.ordering() {
            self
        } else {
            other
        }
    }

    /// Check if this is a terminal (non-transitional) state.
    ///
    /// Terminal states will not change without new external evidence.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Compliant | Self::NonCompliant | Self::Exempt)
    }

    /// Check if this state represents a passing compliance evaluation.
    ///
    /// Passing states allow operations to proceed. `NotApplicable` passes
    /// because it means the domain is irrelevant to the entity.
    pub fn is_passing(self) -> bool {
        matches!(self, Self::Compliant | Self::Exempt | Self::NotApplicable)
    }
}

impl PartialOrd for ComplianceState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ComplianceState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordering().cmp(&other.ordering())
    }
}

impl fmt::Display for ComplianceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compliant => write!(f, "compliant"),
            Self::NonCompliant => write!(f, "non_compliant"),
            Self::Pending => write!(f, "pending"),
            Self::Exempt => write!(f, "exempt"),
            Self::NotApplicable => write!(f, "not_applicable"),
        }
    }
}

// ---------------------------------------------------------------------------
// AttestationRef
// ---------------------------------------------------------------------------

/// Reference to an attestation that justifies a compliance state.
///
/// Attestations are the evidentiary basis for compliance determinations.
/// Each tensor cell links to the attestation(s) that established its state.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AttestationRef {
    /// Unique identifier (typically VC id).
    pub attestation_id: String,
    /// Type of attestation (e.g., "kyc_verification", "aml_screening").
    pub attestation_type: String,
    /// DID of the attestation issuer.
    pub issuer_did: String,
    /// ISO 8601 timestamp of issuance.
    pub issued_at: String,
    /// Optional ISO 8601 expiry timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    /// SHA-256 digest of attestation content.
    pub digest: String,
}

impl AttestationRef {
    /// Check if this attestation has expired as of the given time.
    ///
    /// Returns `false` if no expiry is set or the expiry cannot be parsed.
    pub fn is_expired(&self, as_of: &chrono::DateTime<chrono::Utc>) -> bool {
        match self.expires_at {
            Some(ref expires) => chrono::DateTime::parse_from_rfc3339(expires)
                .map(|expiry| *as_of > expiry)
                .unwrap_or(false),
            None => false,
        }
    }
}

// ---------------------------------------------------------------------------
// EvaluationContext
// ---------------------------------------------------------------------------

/// Context provided to domain evaluators during compliance evaluation.
///
/// Contains entity data, the current stored state, attestations, and
/// jurisdiction-specific metadata needed for evaluation logic.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Entity identifier being evaluated.
    pub entity_id: String,
    /// Current stored compliance state for this domain, if any.
    pub current_state: Option<ComplianceState>,
    /// Attestations available for this entity and domain.
    pub attestations: Vec<AttestationRef>,
    /// Jurisdiction-specific metadata for the evaluator.
    pub metadata: HashMap<String, serde_json::Value>,
}

// ---------------------------------------------------------------------------
// DomainEvaluator Trait
// ---------------------------------------------------------------------------

/// Trait for domain-specific compliance evaluators.
///
/// Each compliance domain can have a custom evaluator that implements
/// domain-specific evaluation logic. The evaluator receives an
/// [`EvaluationContext`] and returns a compliance state with an optional
/// reason code.
///
/// ## Extensibility
///
/// Register custom evaluators on
/// [`ComplianceTensor`](crate::tensor::ComplianceTensor) via
/// `set_evaluator()` to override the default evaluation for specific domains.
pub trait DomainEvaluator: Send + Sync + fmt::Debug {
    /// The compliance domain this evaluator handles.
    fn domain(&self) -> ComplianceDomain;

    /// Evaluate compliance for the given context.
    ///
    /// Returns `(state, optional_reason_code)`.
    fn evaluate(&self, ctx: &EvaluationContext) -> (ComplianceState, Option<String>);
}

// ---------------------------------------------------------------------------
// Default Domain Evaluation — Exhaustive Match
// ---------------------------------------------------------------------------

/// Evaluate a domain using the default logic with an **exhaustive** match
/// on all 20 [`ComplianceDomain`] variants.
///
/// ## Exhaustiveness Guarantee
///
/// Adding a 21st variant to `ComplianceDomain` in `msez-core` will cause
/// a compile error at the `match` below, forcing this function to be updated.
/// This is the primary defense against the Python domain-mismatch defect (§2.4).
///
/// ## Evaluation Logic
///
/// **Original 8 domains** (ported from `tools/phoenix/tensor.py`):
/// AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY, DATA_PRIVACY
/// — check stored state and attestation freshness.
///
/// **Extended 12 domains** (from `tools/msez/composition.py`):
/// LICENSING, BANKING, PAYMENTS, CLEARING, SETTLEMENT, DIGITAL_ASSETS,
/// EMPLOYMENT, IMMIGRATION, IP, CONSUMER_PROTECTION, ARBITRATION, TRADE
/// — return stored state if set; otherwise `NotApplicable` with a warning.
pub fn evaluate_domain_default(
    domain: ComplianceDomain,
    ctx: &EvaluationContext,
) -> (ComplianceState, Option<String>) {
    // EXHAUSTIVE match — every domain variant is listed explicitly.
    // Adding a 21st domain to ComplianceDomain is a compile error here.
    match domain {
        // ─── Original 8 domains (ported from tensor.py) ─────────────
        ComplianceDomain::Aml => evaluate_attested_domain(ctx),
        ComplianceDomain::Kyc => evaluate_attested_domain(ctx),
        ComplianceDomain::Sanctions => evaluate_attested_domain(ctx),
        ComplianceDomain::Tax => evaluate_attested_domain(ctx),
        ComplianceDomain::Securities => evaluate_attested_domain(ctx),
        ComplianceDomain::Corporate => evaluate_attested_domain(ctx),
        ComplianceDomain::Custody => evaluate_attested_domain(ctx),
        ComplianceDomain::DataPrivacy => evaluate_attested_domain(ctx),

        // ─── Extended 12 domains (from composition.py) ──────────────
        ComplianceDomain::Licensing => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Banking => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Payments => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Clearing => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Settlement => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::DigitalAssets => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Employment => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Immigration => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Ip => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::ConsumerProtection => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Arbitration => evaluate_extended_domain(domain, ctx),
        ComplianceDomain::Trade => evaluate_extended_domain(domain, ctx),
    }
}

/// Evaluation logic for the original 8 domains ported from `tools/phoenix/tensor.py`.
///
/// 1. If a state has been explicitly set, check attestation freshness.
/// 2. If attestations are stale (expired), return `NonCompliant`.
/// 3. If no state is stored but attestations exist, infer from freshness.
/// 4. If nothing is stored, return `Pending` (no attestation yet).
fn evaluate_attested_domain(ctx: &EvaluationContext) -> (ComplianceState, Option<String>) {
    let now = chrono::Utc::now();

    // If a state has been explicitly stored, validate attestation freshness.
    if let Some(state) = ctx.current_state {
        let has_stale = ctx.attestations.iter().any(|a| a.is_expired(&now));
        if has_stale && state.is_passing() {
            return (
                ComplianceState::NonCompliant,
                Some("attestation_expired".into()),
            );
        }
        return (state, None);
    }

    // No stored state — check if we have attestations at all.
    if ctx.attestations.is_empty() {
        return (ComplianceState::Pending, Some("no_attestation".into()));
    }

    // Attestations exist but no explicit state — check freshness.
    let all_fresh = ctx.attestations.iter().all(|a| !a.is_expired(&now));
    if all_fresh {
        (ComplianceState::Compliant, Some("attested".into()))
    } else {
        (
            ComplianceState::NonCompliant,
            Some("attestation_expired".into()),
        )
    }
}

/// Evaluation logic for the 12 extended domains from `tools/msez/composition.py`.
///
/// These domains are defined in the composition specification but do not
/// yet have full evaluation implementations in the Python codebase. The Rust
/// version returns stored state if set; otherwise `NotApplicable` with a
/// `tracing::warn!()` log to surface the gap.
fn evaluate_extended_domain(
    domain: ComplianceDomain,
    ctx: &EvaluationContext,
) -> (ComplianceState, Option<String>) {
    // If a state has been explicitly stored, return it.
    if let Some(state) = ctx.current_state {
        return (state, None);
    }

    tracing::warn!(
        domain = %domain,
        entity_id = %ctx.entity_id,
        "domain evaluation not yet implemented — returning NotApplicable"
    );
    (
        ComplianceState::NotApplicable,
        Some("not_implemented".into()),
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ── ComplianceState lattice tests ────────────────────────────────

    #[test]
    fn meet_returns_more_restrictive() {
        assert_eq!(
            ComplianceState::Compliant.meet(ComplianceState::Pending),
            ComplianceState::Pending
        );
        assert_eq!(
            ComplianceState::Pending.meet(ComplianceState::NonCompliant),
            ComplianceState::NonCompliant
        );
        assert_eq!(
            ComplianceState::Exempt.meet(ComplianceState::Compliant),
            ComplianceState::Exempt
        );
    }

    #[test]
    fn non_compliant_is_absorbing_under_meet() {
        for state in [
            ComplianceState::Compliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ] {
            assert_eq!(
                state.meet(ComplianceState::NonCompliant),
                ComplianceState::NonCompliant,
                "meet({state}, NonCompliant) should be NonCompliant"
            );
        }
    }

    #[test]
    fn join_returns_less_restrictive() {
        assert_eq!(
            ComplianceState::Pending.join(ComplianceState::Compliant),
            ComplianceState::Compliant
        );
        assert_eq!(
            ComplianceState::NonCompliant.join(ComplianceState::Pending),
            ComplianceState::Pending
        );
    }

    #[test]
    fn meet_is_commutative() {
        let states = [
            ComplianceState::Compliant,
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ];
        for &a in &states {
            for &b in &states {
                assert_eq!(a.meet(b), b.meet(a), "meet({a}, {b}) != meet({b}, {a})");
            }
        }
    }

    #[test]
    fn join_is_commutative() {
        let states = [
            ComplianceState::Compliant,
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ];
        for &a in &states {
            for &b in &states {
                assert_eq!(a.join(b), b.join(a), "join({a}, {b}) != join({b}, {a})");
            }
        }
    }

    #[test]
    fn meet_is_idempotent() {
        let states = [
            ComplianceState::Compliant,
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ];
        for &s in &states {
            assert_eq!(s.meet(s), s, "meet({s}, {s}) should be {s}");
        }
    }

    #[test]
    fn is_terminal_classification() {
        assert!(ComplianceState::Compliant.is_terminal());
        assert!(ComplianceState::NonCompliant.is_terminal());
        assert!(ComplianceState::Exempt.is_terminal());
        assert!(!ComplianceState::Pending.is_terminal());
        assert!(!ComplianceState::NotApplicable.is_terminal());
    }

    #[test]
    fn is_passing_classification() {
        assert!(ComplianceState::Compliant.is_passing());
        assert!(ComplianceState::Exempt.is_passing());
        assert!(ComplianceState::NotApplicable.is_passing());
        assert!(!ComplianceState::NonCompliant.is_passing());
        assert!(!ComplianceState::Pending.is_passing());
    }

    #[test]
    fn ordering_is_consistent_with_meet_and_join() {
        let states = [
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::NotApplicable,
            ComplianceState::Exempt,
            ComplianceState::Compliant,
        ];
        // States are listed in ascending order. Verify.
        for i in 0..states.len() - 1 {
            assert!(
                states[i] < states[i + 1],
                "{} should be < {}",
                states[i],
                states[i + 1]
            );
        }
    }

    #[test]
    fn serde_roundtrip() {
        let states = [
            ComplianceState::Compliant,
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ];
        for state in states {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: ComplianceState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    // ── AttestationRef tests ────────────────────────────────────────

    #[test]
    fn attestation_not_expired_when_no_expiry() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc_verification".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: None,
            digest: "abc123".into(),
        };
        let now = chrono::Utc::now();
        assert!(!att.is_expired(&now));
    }

    #[test]
    fn attestation_expired_when_past_expiry() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc_verification".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2020-01-01T00:00:00Z".into(),
            expires_at: Some("2020-06-01T00:00:00Z".into()),
            digest: "abc123".into(),
        };
        let now = chrono::Utc::now();
        assert!(att.is_expired(&now));
    }

    #[test]
    fn attestation_not_expired_when_future_expiry() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc_verification".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            digest: "abc123".into(),
        };
        let now = chrono::Utc::now();
        assert!(!att.is_expired(&now));
    }

    // ── Default domain evaluation tests ─────────────────────────────

    #[test]
    fn attested_domain_returns_stored_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: Some(ComplianceState::Compliant),
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Aml, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn attested_domain_returns_pending_when_no_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Kyc, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(reason.as_deref(), Some("no_attestation"));
    }

    #[test]
    fn extended_domain_returns_not_applicable_when_no_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::NotApplicable);
        assert_eq!(reason.as_deref(), Some("not_implemented"));
    }

    #[test]
    fn extended_domain_returns_stored_state_when_set() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: Some(ComplianceState::Compliant),
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Licensing, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn stale_attestation_overrides_passing_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: Some(ComplianceState::Compliant),
            attestations: vec![AttestationRef {
                attestation_id: "att-1".into(),
                attestation_type: "aml_screening".into(),
                issuer_did: "did:example:issuer".into(),
                issued_at: "2020-01-01T00:00:00Z".into(),
                expires_at: Some("2020-06-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Aml, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("attestation_expired"));
    }

    /// Verify that evaluate_domain_default covers all 20 domains.
    /// This is a compile-time guarantee (exhaustive match) verified at runtime.
    #[test]
    fn default_evaluator_covers_all_20_domains() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };
        for &domain in ComplianceDomain::all() {
            let (state, _) = evaluate_domain_default(domain, &ctx);
            // All domains should return a valid state.
            assert!(
                state == ComplianceState::Pending || state == ComplianceState::NotApplicable,
                "domain {domain} with no state should return Pending or NotApplicable, got {state}"
            );
        }
    }
}
