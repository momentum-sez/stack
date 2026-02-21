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
//! [`ComplianceDomain`] enum in `mez-core`.
//! Every `match` on `ComplianceDomain` is exhaustive — adding a 21st domain
//! is a compile error until every evaluation path is updated.
//!
//! ## Spec Reference
//!
//! Implements the compliance state lattice from `tools/phoenix/tensor.py`
//! and the 20-domain coverage from `tools/mez/composition.py`.

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use mez_core::ComplianceDomain;

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
    /// Returns `false` if no expiry is set. Returns `true` (conservatively
    /// treats as expired) if the expiry string cannot be parsed — an
    /// unparseable date must not grant indefinite validity.
    pub fn is_expired(&self, as_of: &chrono::DateTime<chrono::Utc>) -> bool {
        match self.expires_at {
            Some(ref expires) => chrono::DateTime::parse_from_rfc3339(expires)
                .map(|expiry| *as_of > expiry)
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        expires_at = %expires,
                        "unparseable attestation expiry — treating as expired (conservative)"
                    );
                    true
                }),
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
/// Adding a 21st variant to `ComplianceDomain` in `mez-core` will cause
/// a compile error at the `match` below, forcing this function to be updated.
/// This is the primary defense against the Python domain-mismatch defect (§2.4).
///
/// ## Evaluation Logic
///
/// **Original 8 domains** (ported from `tools/phoenix/tensor.py`):
/// AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY, DATA_PRIVACY
/// — check stored state and attestation freshness.
///
/// **Extended 12 domains** (from `tools/mez/composition.py`):
/// LICENSING, BANKING, PAYMENTS, CLEARING, SETTLEMENT, DIGITAL_ASSETS,
/// EMPLOYMENT, IMMIGRATION, IP, CONSUMER_PROTECTION, ARBITRATION, TRADE
/// — return stored state if set; otherwise `Pending` (fail-closed per P0-TENSOR-001).
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

/// Evaluation logic for the 12 extended domains from `tools/mez/composition.py`.
///
/// Each domain implements attestation-driven evaluation with domain-specific
/// metadata requirements. The evaluation follows the same fail-closed principle
/// as the original 8 domains: no attestation → `Pending`, expired attestation
/// → `NonCompliant`.
///
/// ## Domain-Specific Metadata Keys
///
/// Each extended domain may check for specific metadata keys that indicate
/// the entity's relationship to that domain. If the metadata indicates the
/// domain is irrelevant to the entity (e.g., no employees → Employment is
/// not applicable), the evaluator returns `NotApplicable` with a signed
/// policy artifact reference in metadata.
///
/// ## Security Invariant
///
/// `NotApplicable` is only returned when `ctx.metadata` contains an explicit
/// `"{domain}_not_applicable"` key with a policy artifact reference. Without
/// this key, the domain defaults to `Pending` (fail-closed per P0-TENSOR-001).
fn evaluate_extended_domain(
    domain: ComplianceDomain,
    ctx: &EvaluationContext,
) -> (ComplianceState, Option<String>) {
    // If a state has been explicitly stored, validate attestation freshness
    // just as the original 8 domains do. A stale attestation on a passing
    // state must revert to NonCompliant.
    if let Some(state) = ctx.current_state {
        let now = chrono::Utc::now();
        let has_stale = ctx.attestations.iter().any(|a| a.is_expired(&now));
        if has_stale && state.is_passing() {
            return (
                ComplianceState::NonCompliant,
                Some("attestation_expired".into()),
            );
        }
        return (state, None);
    }

    // Check if the domain has been explicitly declared not applicable via
    // a signed policy artifact (the metadata key acts as the reference).
    let na_key = format!("{}_not_applicable", domain.as_str());
    if ctx.metadata.contains_key(&na_key) {
        return (
            ComplianceState::NotApplicable,
            Some("policy_artifact".into()),
        );
    }

    // No stored state — evaluate based on attestation evidence.
    if ctx.attestations.is_empty() {
        // Domain-specific metadata check: if the domain has a recognized
        // metadata key indicating active engagement, return Pending with
        // a specific reason. Otherwise, return Pending with the domain
        // indicator so the caller knows what attestation is required.
        let reason = extended_domain_pending_reason(domain, ctx);
        return (ComplianceState::Pending, Some(reason));
    }

    // Attestations exist — check freshness.
    let now = chrono::Utc::now();
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

/// Produce a domain-specific pending reason that tells the caller exactly
/// what attestation evidence is required to resolve the pending state.
fn extended_domain_pending_reason(domain: ComplianceDomain, ctx: &EvaluationContext) -> String {
    match domain {
        ComplianceDomain::Licensing => {
            if ctx.metadata.contains_key("license_type") {
                "awaiting_license_verification".into()
            } else {
                "no_license_attestation".into()
            }
        }
        ComplianceDomain::Banking => {
            if ctx.metadata.contains_key("bank_license_id") {
                "awaiting_capital_adequacy_attestation".into()
            } else {
                "no_banking_attestation".into()
            }
        }
        ComplianceDomain::Payments => {
            if ctx.metadata.contains_key("psp_license_id") {
                "awaiting_psp_compliance_attestation".into()
            } else {
                "no_payments_attestation".into()
            }
        }
        ComplianceDomain::Clearing => "no_clearing_attestation".into(),
        ComplianceDomain::Settlement => "no_settlement_attestation".into(),
        ComplianceDomain::DigitalAssets => {
            if ctx.metadata.contains_key("token_classification") {
                "awaiting_digital_asset_classification_attestation".into()
            } else {
                "no_digital_asset_attestation".into()
            }
        }
        ComplianceDomain::Employment => {
            if ctx.metadata.contains_key("employee_count") {
                "awaiting_labor_compliance_attestation".into()
            } else {
                "no_employment_attestation".into()
            }
        }
        ComplianceDomain::Immigration => "no_immigration_attestation".into(),
        ComplianceDomain::Ip => "no_ip_attestation".into(),
        ComplianceDomain::ConsumerProtection => "no_consumer_protection_attestation".into(),
        ComplianceDomain::Arbitration => {
            if ctx.metadata.contains_key("arbitration_framework") {
                "awaiting_arbitration_framework_attestation".into()
            } else {
                "no_arbitration_attestation".into()
            }
        }
        ComplianceDomain::Trade => {
            if ctx.metadata.contains_key("trade_license_id") {
                "awaiting_trade_compliance_attestation".into()
            } else {
                "no_trade_attestation".into()
            }
        }
        // Original 8 domains are handled by evaluate_attested_domain, never reach here.
        _ => "no_attestation".into(),
    }
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
    fn extended_domain_returns_pending_when_no_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        // P0-TENSOR-001: fail-closed — extended domains return Pending, not NotApplicable
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(reason.as_deref(), Some("no_banking_attestation"));
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
            // P0-TENSOR-001: all domains with no state must return Pending (fail-closed).
            assert_eq!(
                state,
                ComplianceState::Pending,
                "domain {domain} with no state should return Pending, got {state}"
            );
        }
    }

    // ── Coverage expansion tests ─────────────────────────────────────

    #[test]
    fn compliance_state_display_all_variants() {
        assert_eq!(format!("{}", ComplianceState::Compliant), "compliant");
        assert_eq!(
            format!("{}", ComplianceState::NonCompliant),
            "non_compliant"
        );
        assert_eq!(format!("{}", ComplianceState::Pending), "pending");
        assert_eq!(format!("{}", ComplianceState::Exempt), "exempt");
        assert_eq!(
            format!("{}", ComplianceState::NotApplicable),
            "not_applicable"
        );
    }

    #[test]
    fn compliance_state_ord() {
        use std::cmp::Ordering;
        assert_eq!(
            ComplianceState::NonCompliant.cmp(&ComplianceState::Pending),
            Ordering::Less
        );
        assert_eq!(
            ComplianceState::Pending.cmp(&ComplianceState::NotApplicable),
            Ordering::Less
        );
        assert_eq!(
            ComplianceState::NotApplicable.cmp(&ComplianceState::Exempt),
            Ordering::Less
        );
        assert_eq!(
            ComplianceState::Exempt.cmp(&ComplianceState::Compliant),
            Ordering::Less
        );
        assert_eq!(
            ComplianceState::Compliant.cmp(&ComplianceState::Compliant),
            Ordering::Equal
        );
    }

    #[test]
    fn compliance_state_partial_ord() {
        assert!(ComplianceState::NonCompliant < ComplianceState::Pending);
        assert!(ComplianceState::Pending < ComplianceState::NotApplicable);
        assert!(ComplianceState::NotApplicable < ComplianceState::Exempt);
        assert!(ComplianceState::Exempt < ComplianceState::Compliant);
    }

    #[test]
    fn attestation_ref_no_expiry_never_expired() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: None,
            digest: "abc".into(),
        };
        assert!(!att.is_expired(&chrono::Utc::now()));
    }

    #[test]
    fn attestation_ref_unparseable_expiry_treated_as_expired() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: Some("not-a-date".into()),
            digest: "abc".into(),
        };
        // Conservative: unparseable expiry is treated as expired. An
        // unparseable date must not grant indefinite validity.
        assert!(att.is_expired(&chrono::Utc::now()));
    }

    #[test]
    fn attestation_ref_future_expiry_not_expired() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: Some("2099-01-01T00:00:00Z".into()),
            digest: "abc".into(),
        };
        assert!(!att.is_expired(&chrono::Utc::now()));
    }

    #[test]
    fn attestation_ref_serde_roundtrip() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: Some("2027-01-01T00:00:00Z".into()),
            digest: "abc123".into(),
        };
        let json = serde_json::to_string(&att).unwrap();
        let deserialized: AttestationRef = serde_json::from_str(&json).unwrap();
        assert_eq!(att, deserialized);
    }

    #[test]
    fn attestation_ref_serde_skip_none_expires() {
        let att = AttestationRef {
            attestation_id: "att-1".into(),
            attestation_type: "kyc".into(),
            issuer_did: "did:example:issuer".into(),
            issued_at: "2026-01-01T00:00:00Z".into(),
            expires_at: None,
            digest: "abc123".into(),
        };
        let json = serde_json::to_string(&att).unwrap();
        assert!(!json.contains("expires_at"));
    }

    #[test]
    fn compliance_state_is_terminal_all_variants() {
        assert!(ComplianceState::Compliant.is_terminal());
        assert!(ComplianceState::NonCompliant.is_terminal());
        assert!(!ComplianceState::Pending.is_terminal());
        assert!(ComplianceState::Exempt.is_terminal());
        assert!(!ComplianceState::NotApplicable.is_terminal());
    }

    #[test]
    fn compliance_state_is_passing_all_variants() {
        assert!(ComplianceState::Compliant.is_passing());
        assert!(!ComplianceState::NonCompliant.is_passing());
        assert!(!ComplianceState::Pending.is_passing());
        assert!(ComplianceState::Exempt.is_passing());
        assert!(ComplianceState::NotApplicable.is_passing());
    }

    #[test]
    fn evaluate_attested_domain_fresh_attestation_returns_compliant() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-fresh".into(),
                attestation_type: "kyc_verification".into(),
                issuer_did: "did:example:issuer".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Kyc, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
        assert_eq!(reason.as_deref(), Some("attested"));
    }

    #[test]
    fn evaluate_attested_domain_stale_attestation_no_stored_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-1".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-stale".into(),
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

    #[test]
    fn compliance_state_serde_roundtrip() {
        for state in [
            ComplianceState::Compliant,
            ComplianceState::NonCompliant,
            ComplianceState::Pending,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
        ] {
            let json = serde_json::to_string(&state).unwrap();
            let deserialized: ComplianceState = serde_json::from_str(&json).unwrap();
            assert_eq!(state, deserialized);
        }
    }

    #[test]
    fn compliance_state_hash_in_collection() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ComplianceState::Compliant);
        set.insert(ComplianceState::NonCompliant);
        set.insert(ComplianceState::Compliant);
        assert_eq!(set.len(), 2);
    }

    // ── Adversarial: NotApplicable bypass (P0-TENSOR-001) ───────────

    /// Adversarial vector: Attempt to bypass compliance by having extended
    /// domains return NotApplicable when no state is stored.
    /// Extended domains must return Pending (fail-closed), NOT NotApplicable.
    #[test]
    fn adversarial_extended_domain_not_applicable_bypass() {
        // Extended domain with no stored state → must be Pending, not NotApplicable
        let extended_domains = [
            ComplianceDomain::Licensing,
            ComplianceDomain::Banking,
            ComplianceDomain::Payments,
            ComplianceDomain::Clearing,
            ComplianceDomain::Settlement,
            ComplianceDomain::DigitalAssets,
            ComplianceDomain::Employment,
            ComplianceDomain::Immigration,
            ComplianceDomain::Ip,
            ComplianceDomain::ConsumerProtection,
            ComplianceDomain::Arbitration,
            ComplianceDomain::Trade,
        ];

        for domain in extended_domains {
            let ctx = EvaluationContext {
                entity_id: "adversarial-entity".to_string(),
                current_state: None, // No stored state
                attestations: vec![],
                metadata: std::collections::HashMap::new(),
            };

            let (state, _note) = evaluate_domain_default(domain, &ctx);
            assert_eq!(
                state,
                ComplianceState::Pending,
                "extended domain {:?} with no stored state must return Pending (fail-closed), \
                 not NotApplicable. An attacker must not be able to bypass compliance checks \
                 by exploiting unimplemented domains.",
                domain
            );
            // Pending is NOT passing
            assert!(
                !state.is_passing(),
                "Pending must not pass compliance check"
            );
        }
    }

    // ── Extended domain evaluation — attestation-driven tests ────────

    #[test]
    fn extended_domain_fresh_attestation_returns_compliant() {
        let extended_domains = [
            ComplianceDomain::Licensing,
            ComplianceDomain::Banking,
            ComplianceDomain::Payments,
            ComplianceDomain::Clearing,
            ComplianceDomain::Settlement,
            ComplianceDomain::DigitalAssets,
            ComplianceDomain::Employment,
            ComplianceDomain::Immigration,
            ComplianceDomain::Ip,
            ComplianceDomain::ConsumerProtection,
            ComplianceDomain::Arbitration,
            ComplianceDomain::Trade,
        ];

        for domain in extended_domains {
            let ctx = EvaluationContext {
                entity_id: "entity-with-attestation".into(),
                current_state: None,
                attestations: vec![AttestationRef {
                    attestation_id: format!("att-{}", domain.as_str()),
                    attestation_type: format!("{}_compliance", domain.as_str()),
                    issuer_did: "did:example:regulator".into(),
                    issued_at: "2026-01-01T00:00:00Z".into(),
                    expires_at: Some("2099-01-01T00:00:00Z".into()),
                    digest: "abc123".into(),
                }],
                metadata: HashMap::new(),
            };

            let (state, reason) = evaluate_domain_default(domain, &ctx);
            assert_eq!(
                state,
                ComplianceState::Compliant,
                "extended domain {:?} with fresh attestation should be Compliant",
                domain
            );
            assert_eq!(reason.as_deref(), Some("attested"));
        }
    }

    #[test]
    fn extended_domain_stale_attestation_no_stored_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-stale".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-expired".into(),
                attestation_type: "licensing_compliance".into(),
                issuer_did: "did:example:regulator".into(),
                issued_at: "2020-01-01T00:00:00Z".into(),
                expires_at: Some("2020-06-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Licensing, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("attestation_expired"));
    }

    #[test]
    fn extended_domain_stale_attestation_overrides_passing_stored_state() {
        let ctx = EvaluationContext {
            entity_id: "entity-stale-override".into(),
            current_state: Some(ComplianceState::Compliant),
            attestations: vec![AttestationRef {
                attestation_id: "att-expired".into(),
                attestation_type: "banking_compliance".into(),
                issuer_did: "did:example:regulator".into(),
                issued_at: "2020-01-01T00:00:00Z".into(),
                expires_at: Some("2020-06-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata: HashMap::new(),
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("attestation_expired"));
    }

    #[test]
    fn extended_domain_not_applicable_requires_policy_artifact() {
        // Without the policy artifact key, must be Pending.
        let ctx_no_policy = EvaluationContext {
            entity_id: "entity-no-policy".into(),
            current_state: None,
            attestations: vec![],
            metadata: HashMap::new(),
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Employment, &ctx_no_policy);
        assert_eq!(state, ComplianceState::Pending);

        // With the policy artifact key, can return NotApplicable.
        let mut metadata = HashMap::new();
        metadata.insert(
            "employment_not_applicable".to_string(),
            serde_json::json!("policy:no-employees:signed-2026-01-15"),
        );
        let ctx_with_policy = EvaluationContext {
            entity_id: "entity-with-policy".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) =
            evaluate_domain_default(ComplianceDomain::Employment, &ctx_with_policy);
        assert_eq!(state, ComplianceState::NotApplicable);
        assert_eq!(reason.as_deref(), Some("policy_artifact"));
    }

    #[test]
    fn extended_domain_specific_pending_reasons() {
        let test_cases: Vec<(ComplianceDomain, &str, &str, &str)> = vec![
            (ComplianceDomain::Licensing, "license_type", "banking_license", "awaiting_license_verification"),
            (ComplianceDomain::Banking, "bank_license_id", "BL-001", "awaiting_capital_adequacy_attestation"),
            (ComplianceDomain::Payments, "psp_license_id", "PSP-001", "awaiting_psp_compliance_attestation"),
            (ComplianceDomain::DigitalAssets, "token_classification", "security_token", "awaiting_digital_asset_classification_attestation"),
            (ComplianceDomain::Employment, "employee_count", "50", "awaiting_labor_compliance_attestation"),
            (ComplianceDomain::Arbitration, "arbitration_framework", "UNCITRAL", "awaiting_arbitration_framework_attestation"),
            (ComplianceDomain::Trade, "trade_license_id", "TL-001", "awaiting_trade_compliance_attestation"),
        ];

        for (domain, key, value, expected_reason) in test_cases {
            let mut metadata = HashMap::new();
            metadata.insert(key.to_string(), serde_json::json!(value));
            let ctx = EvaluationContext {
                entity_id: "entity-metadata".into(),
                current_state: None,
                attestations: vec![],
                metadata,
            };
            let (state, reason) = evaluate_domain_default(domain, &ctx);
            assert_eq!(state, ComplianceState::Pending);
            assert_eq!(
                reason.as_deref(),
                Some(expected_reason),
                "domain {:?} with metadata key '{}' should give reason '{}'",
                domain,
                key,
                expected_reason
            );
        }
    }

    /// Adversarial vector: aggregate_state must not return Compliant
    /// when any domain is NonCompliant, regardless of other domain states.
    #[test]
    fn adversarial_non_compliant_cannot_be_elevated() {
        // meet(Compliant, NonCompliant) must be NonCompliant
        let result = ComplianceState::Compliant.meet(ComplianceState::NonCompliant);
        assert_eq!(result, ComplianceState::NonCompliant);

        // meet(Exempt, NonCompliant) must be NonCompliant
        let result = ComplianceState::Exempt.meet(ComplianceState::NonCompliant);
        assert_eq!(result, ComplianceState::NonCompliant);

        // meet(NotApplicable, NonCompliant) must be NonCompliant
        let result = ComplianceState::NotApplicable.meet(ComplianceState::NonCompliant);
        assert_eq!(result, ComplianceState::NonCompliant);

        // Folding any set containing NonCompliant must yield NonCompliant
        let states = [
            ComplianceState::Compliant,
            ComplianceState::Exempt,
            ComplianceState::NotApplicable,
            ComplianceState::NonCompliant,
            ComplianceState::Compliant,
        ];
        let aggregate = states
            .iter()
            .copied()
            .fold(ComplianceState::Compliant, ComplianceState::meet);
        assert_eq!(
            aggregate,
            ComplianceState::NonCompliant,
            "no path must elevate NonCompliant to passing"
        );
    }
}
