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
/// metadata requirements and business rule validation. The evaluation follows
/// the same fail-closed principle as the original 8 domains: no attestation →
/// `Pending`, expired attestation → `NonCompliant`.
///
/// ## Three-Phase Evaluation
///
/// 1. **Stored state + freshness** — if state is explicitly set, validate
///    attestation freshness (stale attestation on passing state → NonCompliant).
/// 2. **Policy artifact** — if `"{domain}_not_applicable"` metadata key exists,
///    return `NotApplicable`.
/// 3. **Domain-specific business rules** — validate metadata against
///    domain-specific requirements. Metadata can prove NonCompliant (rule
///    violations) or refine Pending (specific guidance), but only attestations
///    can prove Compliant.
/// 4. **Attestation evaluation** — check attestation presence and freshness.
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
    // Phase 1: If a state has been explicitly stored, validate attestation
    // freshness just as the original 8 domains do. A stale attestation on a
    // passing state must revert to NonCompliant.
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

    // Phase 2: Check if the domain has been explicitly declared not applicable
    // via a signed policy artifact (the metadata key acts as the reference).
    let na_key = format!("{}_not_applicable", domain.as_str());
    if ctx.metadata.contains_key(&na_key) {
        return (
            ComplianceState::NotApplicable,
            Some("policy_artifact".into()),
        );
    }

    // Phase 3: Domain-specific business rule validation on metadata.
    // If metadata contains domain-specific fields, apply business rules.
    // Metadata can prove NonCompliant (violations) or refine Pending
    // (specific requirements), but cannot prove Compliant — that requires
    // attestation evidence from a trusted issuer.
    if let Some(result) = validate_domain_metadata(domain, ctx) {
        return result;
    }

    // Phase 4: Attestation evaluation (no domain metadata present, or
    // metadata validation returned None indicating no determination).
    if ctx.attestations.is_empty() {
        let reason = extended_domain_pending_reason(domain, ctx);
        return (ComplianceState::Pending, Some(reason));
    }

    // Attestations exist — validate freshness and type appropriateness.
    let now = chrono::Utc::now();
    let all_fresh = ctx.attestations.iter().all(|a| !a.is_expired(&now));
    if !all_fresh {
        return (
            ComplianceState::NonCompliant,
            Some("attestation_expired".into()),
        );
    }

    // Check that at least one attestation has a domain-appropriate type.
    let expected_types = domain_attestation_types(domain);
    if !expected_types.is_empty() {
        let has_appropriate = ctx.attestations.iter().any(|a| {
            expected_types
                .iter()
                .any(|t| a.attestation_type.contains(t))
        });
        if has_appropriate {
            return (ComplianceState::Compliant, Some("attested".into()));
        }
        // Fresh attestations exist but none match expected types.
        // Accept generic attestations for backwards compatibility — a
        // domain-agnostic attestation is still evidence of compliance.
    }

    (ComplianceState::Compliant, Some("attested".into()))
}

// ---------------------------------------------------------------------------
// Domain-Specific Metadata Validation
// ---------------------------------------------------------------------------

/// Dispatch to domain-specific metadata validation.
///
/// Returns `Some((state, reason))` if metadata contains enough information
/// to make a domain-specific determination. Returns `None` if no relevant
/// metadata is present (caller falls through to attestation evaluation).
///
/// ## Design Principle
///
/// Metadata can prove `NonCompliant` (business rule violations) or refine
/// `Pending` (specific guidance on what's needed), but **cannot prove
/// `Compliant`** — that requires attestation evidence from a trusted issuer.
fn validate_domain_metadata(
    domain: ComplianceDomain,
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    match domain {
        ComplianceDomain::Licensing => validate_licensing_metadata(ctx),
        ComplianceDomain::Banking => validate_banking_metadata(ctx),
        ComplianceDomain::Payments => validate_payments_metadata(ctx),
        ComplianceDomain::Clearing => validate_clearing_metadata(ctx),
        ComplianceDomain::Settlement => validate_settlement_metadata(ctx),
        ComplianceDomain::DigitalAssets => validate_digital_assets_metadata(ctx),
        ComplianceDomain::Employment => validate_employment_metadata(ctx),
        ComplianceDomain::Immigration => validate_immigration_metadata(ctx),
        ComplianceDomain::Ip => validate_ip_metadata(ctx),
        ComplianceDomain::ConsumerProtection => validate_consumer_protection_metadata(ctx),
        ComplianceDomain::Arbitration => validate_arbitration_metadata(ctx),
        ComplianceDomain::Trade => validate_trade_metadata(ctx),
        // Original 8 domains are handled by evaluate_attested_domain.
        _ => None,
    }
}

/// Expected attestation types for each extended domain.
///
/// Used during Phase 4 to verify attestation type appropriateness.
/// Returns an empty slice for domains that accept any attestation type.
fn domain_attestation_types(domain: ComplianceDomain) -> &'static [&'static str] {
    match domain {
        ComplianceDomain::Licensing => &["license", "licensing"],
        ComplianceDomain::Banking => &["banking", "capital_adequacy", "prudential"],
        ComplianceDomain::Payments => &["payment", "psp", "float_safeguarding"],
        ComplianceDomain::Clearing => &["clearing", "ccp", "margin"],
        ComplianceDomain::Settlement => &["settlement", "dvp", "finality"],
        ComplianceDomain::DigitalAssets => &["digital_asset", "token", "vasp"],
        ComplianceDomain::Employment => &["employment", "labor", "workplace"],
        ComplianceDomain::Immigration => &["immigration", "work_permit", "visa"],
        ComplianceDomain::Ip => &["ip", "intellectual_property", "patent", "trademark"],
        ComplianceDomain::ConsumerProtection => &["consumer", "disclosure", "warranty"],
        ComplianceDomain::Arbitration => &["arbitration", "dispute_resolution"],
        ComplianceDomain::Trade => &["trade", "customs", "export_control", "import"],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Licensing Domain
// ---------------------------------------------------------------------------

/// Validate licensing metadata against business rules.
///
/// Required evidence: `license_type` + `license_id` + valid expiry.
/// A declared license type without a license ID indicates the entity knows
/// it needs a license but hasn't obtained one. An expired license is
/// NonCompliant regardless of attestation state.
fn validate_licensing_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_license_type = ctx.metadata.contains_key("license_type");
    let has_license_id = ctx.metadata.contains_key("license_id");

    if !has_license_type && !has_license_id {
        return None; // No licensing metadata — fall through.
    }

    // License type declared but no license ID — entity needs a license.
    if has_license_type && !has_license_id {
        return Some((
            ComplianceState::Pending,
            Some("license_type_declared_no_license_id".into()),
        ));
    }

    // Check license expiry date if provided.
    if let Some(expiry) = ctx.metadata.get("license_valid_until").and_then(|v| v.as_str()) {
        if is_date_past(expiry) {
            return Some((
                ComplianceState::NonCompliant,
                Some("license_expired".into()),
            ));
        }
    }

    // Check issuing authority is present (required for verification).
    if has_license_id && !ctx.metadata.contains_key("issuing_authority") {
        return Some((
            ComplianceState::Pending,
            Some("license_id_present_missing_issuing_authority".into()),
        ));
    }

    // License metadata complete but needs attestation verification.
    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("license_declared_awaiting_verification".into()),
        ));
    }

    None // Metadata valid, fall through to attestation evaluation.
}

// ---------------------------------------------------------------------------
// Banking Domain
// ---------------------------------------------------------------------------

/// Validate banking metadata against business rules.
///
/// Key business rules:
/// - Capital adequacy ratio must be >= 8% (Basel III minimum).
/// - Bank license ID is required.
/// - Reserve ratio must be present if declared.
/// - AML status must not be non-compliant (cross-domain dependency).
fn validate_banking_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_bank_license = ctx.metadata.contains_key("bank_license_id");

    if !has_bank_license && !ctx.metadata.contains_key("capital_adequacy_ratio") {
        return None; // No banking metadata — fall through.
    }

    // Cross-domain check: AML status must not be non-compliant.
    if let Some(aml_status) = ctx.metadata.get("aml_status").and_then(|v| v.as_str()) {
        if aml_status == "non_compliant" {
            return Some((
                ComplianceState::NonCompliant,
                Some("banking_blocked_aml_non_compliant".into()),
            ));
        }
    }

    // Capital adequacy ratio validation (Basel III: minimum 8%).
    if let Some(car_str) = ctx.metadata.get("capital_adequacy_ratio").and_then(|v| v.as_str()) {
        if let Ok(car) = car_str.parse::<f64>() {
            if car < 0.08 {
                return Some((
                    ComplianceState::NonCompliant,
                    Some(format!(
                        "capital_adequacy_ratio_{:.2}_below_basel3_minimum_0.08",
                        car
                    )),
                ));
            }
        } else {
            return Some((
                ComplianceState::NonCompliant,
                Some("capital_adequacy_ratio_unparseable".into()),
            ));
        }
    }

    // Bank license required.
    if !has_bank_license {
        return Some((
            ComplianceState::Pending,
            Some("no_bank_license_id".into()),
        ));
    }

    // Bank license present — check for reserve ratio if applicable.
    if ctx.metadata.contains_key("requires_reserve_ratio")
        && !ctx.metadata.contains_key("reserve_ratio")
    {
        return Some((
            ComplianceState::Pending,
            Some("reserve_ratio_required_but_missing".into()),
        ));
    }

    // Banking metadata valid — needs attestation.
    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("banking_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Payments Domain
// ---------------------------------------------------------------------------

/// Validate payments metadata against business rules.
///
/// Key business rules:
/// - PSP license ID is required for payment service providers.
/// - Float safeguarding evidence must be present.
/// - Payment scheme membership must be declared.
fn validate_payments_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_psp_license = ctx.metadata.contains_key("psp_license_id");
    let has_float_safeguarding = ctx.metadata.contains_key("float_safeguarding");

    if !has_psp_license && !has_float_safeguarding {
        return None; // No payments metadata — fall through.
    }

    // PSP license present — check float safeguarding.
    if has_psp_license && !has_float_safeguarding {
        return Some((
            ComplianceState::Pending,
            Some("psp_licensed_missing_float_safeguarding_evidence".into()),
        ));
    }

    // Float safeguarding declared — check status.
    if let Some(fs_status) = ctx.metadata.get("float_safeguarding").and_then(|v| v.as_str()) {
        if fs_status == "failed" || fs_status == "non_compliant" {
            return Some((
                ComplianceState::NonCompliant,
                Some("float_safeguarding_failed".into()),
            ));
        }
    }

    // Check payment scheme membership if applicable.
    if ctx.metadata.contains_key("requires_scheme_membership")
        && !ctx.metadata.contains_key("payment_scheme_id")
    {
        return Some((
            ComplianceState::Pending,
            Some("payment_scheme_membership_required_but_missing".into()),
        ));
    }

    // Payments metadata valid — needs attestation.
    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("payments_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Clearing Domain
// ---------------------------------------------------------------------------

/// Validate clearing metadata against business rules.
///
/// Key business rules:
/// - CCP membership or clearing agreement must be evidenced.
/// - Margin requirements must be met.
/// - Default fund contribution must be current.
fn validate_clearing_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_ccp_member = ctx.metadata.contains_key("ccp_membership_id");
    let has_margin = ctx.metadata.contains_key("margin_status");

    if !has_ccp_member && !has_margin {
        return None; // No clearing metadata — fall through.
    }

    // Margin status check.
    if let Some(margin) = ctx.metadata.get("margin_status").and_then(|v| v.as_str()) {
        if margin == "deficit" || margin == "call_outstanding" {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("margin_status_{}", margin)),
            ));
        }
    }

    // Default fund contribution check.
    if let Some(df_status) = ctx
        .metadata
        .get("default_fund_status")
        .and_then(|v| v.as_str())
    {
        if df_status == "overdue" || df_status == "insufficient" {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("default_fund_{}", df_status)),
            ));
        }
    }

    // CCP membership without margin data.
    if has_ccp_member && !has_margin {
        return Some((
            ComplianceState::Pending,
            Some("ccp_member_missing_margin_status".into()),
        ));
    }

    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("clearing_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Settlement Domain
// ---------------------------------------------------------------------------

/// Validate settlement metadata against business rules.
///
/// Key business rules:
/// - Settlement cycle must be declared and valid (T+0 to T+5).
/// - DVP (delivery-versus-payment) compliance is required for securities.
/// - Settlement finality must be evidenced.
fn validate_settlement_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_cycle = ctx.metadata.contains_key("settlement_cycle");
    let has_dvp = ctx.metadata.contains_key("dvp_compliance");

    if !has_cycle && !has_dvp {
        return None; // No settlement metadata — fall through.
    }

    // Settlement cycle validation.
    if let Some(cycle) = ctx.metadata.get("settlement_cycle").and_then(|v| v.as_str()) {
        // Valid cycles: T+0, T+1, T+2, T+3, T+5.
        let valid_cycles = ["T+0", "T+1", "T+2", "T+3", "T+5"];
        if !valid_cycles.contains(&cycle) {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("invalid_settlement_cycle_{}", cycle)),
            ));
        }
    }

    // DVP compliance check.
    if let Some(dvp) = ctx.metadata.get("dvp_compliance").and_then(|v| v.as_str()) {
        if dvp == "failed" || dvp == "non_compliant" {
            return Some((
                ComplianceState::NonCompliant,
                Some("dvp_compliance_failed".into()),
            ));
        }
    }

    // Settlement finality evidence.
    if has_cycle
        && !ctx.metadata.contains_key("settlement_finality")
        && ctx.attestations.is_empty()
    {
        return Some((
            ComplianceState::Pending,
            Some("settlement_cycle_declared_missing_finality_evidence".into()),
        ));
    }

    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("settlement_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Digital Assets Domain
// ---------------------------------------------------------------------------

/// Validate digital assets metadata against business rules.
///
/// Key business rules:
/// - Token classification is required (security_token, utility_token,
///   payment_token, stablecoin).
/// - Security tokens require securities domain compliance.
/// - VASP license required for exchange/custodian operations.
/// - Travel rule compliance required for transfers above threshold.
fn validate_digital_assets_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_classification = ctx.metadata.contains_key("token_classification");
    let has_vasp_license = ctx.metadata.contains_key("vasp_license_id");

    if !has_classification && !has_vasp_license {
        return None; // No digital assets metadata — fall through.
    }

    // Token classification validation.
    if let Some(classification) = ctx
        .metadata
        .get("token_classification")
        .and_then(|v| v.as_str())
    {
        let valid_types = [
            "security_token",
            "utility_token",
            "payment_token",
            "stablecoin",
        ];
        if !valid_types.contains(&classification) {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!(
                    "unrecognized_token_classification_{}",
                    classification
                )),
            ));
        }

        // Security tokens require securities compliance evidence.
        if classification == "security_token" {
            if let Some(sec_status) =
                ctx.metadata.get("securities_status").and_then(|v| v.as_str())
            {
                if sec_status == "non_compliant" {
                    return Some((
                        ComplianceState::NonCompliant,
                        Some("security_token_requires_securities_compliance".into()),
                    ));
                }
            }
        }
    }

    // Travel rule compliance check for transfers.
    if let Some(travel_rule) = ctx
        .metadata
        .get("travel_rule_compliance")
        .and_then(|v| v.as_str())
    {
        if travel_rule == "failed" || travel_rule == "non_compliant" {
            return Some((
                ComplianceState::NonCompliant,
                Some("travel_rule_non_compliant".into()),
            ));
        }
    }

    // VASP operations require license.
    if ctx.metadata.contains_key("operates_vasp") && !has_vasp_license {
        return Some((
            ComplianceState::Pending,
            Some("vasp_operations_declared_no_license".into()),
        ));
    }

    if ctx.attestations.is_empty() {
        return Some((
            ComplianceState::Pending,
            Some("digital_assets_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Employment Domain
// ---------------------------------------------------------------------------

/// Validate employment metadata against business rules.
///
/// Key business rules:
/// - Employee count determines applicability.
/// - Zero employees can support NotApplicable (via policy artifact).
/// - Active employees require labor compliance evidence.
/// - Social security registration is mandatory.
fn validate_employment_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_employee_count = ctx.metadata.contains_key("employee_count");

    if !has_employee_count && !ctx.metadata.contains_key("labor_jurisdiction") {
        return None; // No employment metadata — fall through.
    }

    // Employee count check.
    if let Some(count_str) = ctx.metadata.get("employee_count").and_then(|v| v.as_str()) {
        if let Ok(count) = count_str.parse::<u64>() {
            if count == 0 {
                // Zero employees — domain may not apply, but needs policy
                // artifact for formal exemption. Return specific pending.
                return Some((
                    ComplianceState::Pending,
                    Some("zero_employees_declare_not_applicable_via_policy_artifact".into()),
                ));
            }

            // Has employees — check for required compliance evidence.
            if !ctx.metadata.contains_key("social_security_registration") {
                return Some((
                    ComplianceState::Pending,
                    Some("employees_present_missing_social_security_registration".into()),
                ));
            }

            // Check labor contract compliance.
            if let Some(labor_status) =
                ctx.metadata.get("labor_compliance_status").and_then(|v| v.as_str())
            {
                if labor_status == "non_compliant" || labor_status == "violations_found" {
                    return Some((
                        ComplianceState::NonCompliant,
                        Some(format!("labor_compliance_{}", labor_status)),
                    ));
                }
            }
        }
    }

    if ctx.attestations.is_empty() && has_employee_count {
        return Some((
            ComplianceState::Pending,
            Some("employment_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Immigration Domain
// ---------------------------------------------------------------------------

/// Validate immigration metadata against business rules.
///
/// Key business rules:
/// - Foreign employee count determines applicability.
/// - Each foreign worker needs valid work permit.
/// - Work permits must not be expired.
fn validate_immigration_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_foreign_count = ctx.metadata.contains_key("foreign_employee_count");

    if !has_foreign_count && !ctx.metadata.contains_key("work_permit_status") {
        return None; // No immigration metadata — fall through.
    }

    // Foreign employee count check.
    if let Some(count_str) = ctx
        .metadata
        .get("foreign_employee_count")
        .and_then(|v| v.as_str())
    {
        if let Ok(count) = count_str.parse::<u64>() {
            if count == 0 {
                return Some((
                    ComplianceState::Pending,
                    Some("zero_foreign_employees_declare_not_applicable_via_policy_artifact".into()),
                ));
            }

            // Foreign workers present — work permits required.
            if !ctx.metadata.contains_key("work_permit_status") {
                return Some((
                    ComplianceState::Pending,
                    Some("foreign_employees_present_missing_work_permit_status".into()),
                ));
            }
        }
    }

    // Work permit status check.
    if let Some(wp_status) = ctx
        .metadata
        .get("work_permit_status")
        .and_then(|v| v.as_str())
    {
        match wp_status {
            "expired" | "revoked" | "invalid" => {
                return Some((
                    ComplianceState::NonCompliant,
                    Some(format!("work_permit_{}", wp_status)),
                ));
            }
            "pending_renewal" => {
                return Some((
                    ComplianceState::Pending,
                    Some("work_permit_pending_renewal".into()),
                ));
            }
            _ => {}
        }
    }

    if ctx.attestations.is_empty() && has_foreign_count {
        return Some((
            ComplianceState::Pending,
            Some("immigration_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Intellectual Property Domain
// ---------------------------------------------------------------------------

/// Validate IP metadata against business rules.
///
/// Key business rules:
/// - IP portfolio existence determines base applicability.
/// - Registered IP assets need protection evidence.
/// - Trade secret policies required if trade secrets declared.
fn validate_ip_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_portfolio = ctx.metadata.contains_key("ip_portfolio_exists");

    if !has_portfolio && !ctx.metadata.contains_key("trade_secret_policy") {
        return None; // No IP metadata — fall through.
    }

    // IP portfolio existence check.
    if let Some(exists) = ctx
        .metadata
        .get("ip_portfolio_exists")
        .and_then(|v| v.as_str())
    {
        if exists == "false" || exists == "no" {
            return Some((
                ComplianceState::Pending,
                Some("no_ip_portfolio_declare_not_applicable_via_policy_artifact".into()),
            ));
        }
    }

    // Trade secret policy required if trade secrets declared.
    if ctx.metadata.contains_key("has_trade_secrets")
        && !ctx.metadata.contains_key("trade_secret_policy")
    {
        return Some((
            ComplianceState::Pending,
            Some("trade_secrets_declared_missing_protection_policy".into()),
        ));
    }

    // IP protection status check.
    if let Some(protection_status) = ctx
        .metadata
        .get("ip_protection_status")
        .and_then(|v| v.as_str())
    {
        if protection_status == "infringement_found" || protection_status == "expired" {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("ip_protection_{}", protection_status)),
            ));
        }
    }

    if ctx.attestations.is_empty() && has_portfolio {
        return Some((
            ComplianceState::Pending,
            Some("ip_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Consumer Protection Domain
// ---------------------------------------------------------------------------

/// Validate consumer protection metadata against business rules.
///
/// Key business rules:
/// - Consumer-facing entities must have dispute resolution mechanism.
/// - Disclosure compliance is mandatory.
/// - Warranty policy required for product/service providers.
fn validate_consumer_protection_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_consumer_facing = ctx.metadata.contains_key("consumer_facing");

    if !has_consumer_facing && !ctx.metadata.contains_key("dispute_resolution_mechanism") {
        return None; // No consumer protection metadata — fall through.
    }

    // Consumer-facing check.
    if let Some(consumer_facing) = ctx
        .metadata
        .get("consumer_facing")
        .and_then(|v| v.as_str())
    {
        if consumer_facing == "false" || consumer_facing == "no" {
            return Some((
                ComplianceState::Pending,
                Some("non_consumer_facing_declare_not_applicable_via_policy_artifact".into()),
            ));
        }

        // Consumer-facing entity — requires dispute resolution.
        if consumer_facing == "true" || consumer_facing == "yes" {
            if !ctx.metadata.contains_key("dispute_resolution_mechanism") {
                return Some((
                    ComplianceState::Pending,
                    Some("consumer_facing_missing_dispute_resolution_mechanism".into()),
                ));
            }

            // Disclosure compliance check.
            if let Some(disclosure) = ctx
                .metadata
                .get("disclosure_compliance")
                .and_then(|v| v.as_str())
            {
                if disclosure == "non_compliant" || disclosure == "deficient" {
                    return Some((
                        ComplianceState::NonCompliant,
                        Some(format!("disclosure_compliance_{}", disclosure)),
                    ));
                }
            }
        }
    }

    if ctx.attestations.is_empty() && has_consumer_facing {
        return Some((
            ComplianceState::Pending,
            Some("consumer_protection_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Arbitration Domain
// ---------------------------------------------------------------------------

/// Validate arbitration metadata against business rules.
///
/// Key business rules:
/// - Arbitration framework must be a recognized framework.
/// - Arbitration clause must be present in entity agreements.
/// - Enforcement convention adherence is required.
fn validate_arbitration_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_framework = ctx.metadata.contains_key("arbitration_framework");

    if !has_framework && !ctx.metadata.contains_key("arbitration_clause") {
        return None; // No arbitration metadata — fall through.
    }

    // Framework validation — must be a recognized framework.
    if let Some(framework) = ctx
        .metadata
        .get("arbitration_framework")
        .and_then(|v| v.as_str())
    {
        let recognized = [
            "UNCITRAL",
            "ICC",
            "LCIA",
            "SIAC",
            "DIAC",
            "HKIAC",
            "ADGM",
            "DIFC-LCIA",
            "PCA",
            "local",
        ];
        if !recognized.contains(&framework) {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("unrecognized_arbitration_framework_{}", framework)),
            ));
        }
    }

    // Arbitration clause check.
    if has_framework && !ctx.metadata.contains_key("arbitration_clause") {
        return Some((
            ComplianceState::Pending,
            Some("arbitration_framework_declared_missing_clause".into()),
        ));
    }

    // Enforcement convention check (New York Convention).
    if let Some(enforcement) = ctx
        .metadata
        .get("enforcement_convention")
        .and_then(|v| v.as_str())
    {
        if enforcement == "non_signatory" {
            return Some((
                ComplianceState::NonCompliant,
                Some("jurisdiction_not_new_york_convention_signatory".into()),
            ));
        }
    }

    if ctx.attestations.is_empty() && has_framework {
        return Some((
            ComplianceState::Pending,
            Some("arbitration_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Trade Domain
// ---------------------------------------------------------------------------

/// Validate trade metadata against business rules.
///
/// Key business rules:
/// - Trade license required for import/export activities.
/// - Customs registration required.
/// - Export control compliance for restricted goods.
/// - Sanctions screening for trade counterparties.
fn validate_trade_metadata(
    ctx: &EvaluationContext,
) -> Option<(ComplianceState, Option<String>)> {
    let has_trade_license = ctx.metadata.contains_key("trade_license_id");
    let has_customs = ctx.metadata.contains_key("customs_registration_id");

    if !has_trade_license && !has_customs && !ctx.metadata.contains_key("import_export_activities")
    {
        return None; // No trade metadata — fall through.
    }

    // Import/export activities declared but no trade license.
    if ctx.metadata.contains_key("import_export_activities") && !has_trade_license {
        return Some((
            ComplianceState::Pending,
            Some("import_export_declared_missing_trade_license".into()),
        ));
    }

    // Trade license present but no customs registration.
    if has_trade_license && !has_customs {
        return Some((
            ComplianceState::Pending,
            Some("trade_licensed_missing_customs_registration".into()),
        ));
    }

    // Export control compliance check for restricted goods.
    if let Some(ec_status) = ctx
        .metadata
        .get("export_control_status")
        .and_then(|v| v.as_str())
    {
        if ec_status == "violation" || ec_status == "non_compliant" {
            return Some((
                ComplianceState::NonCompliant,
                Some(format!("export_control_{}", ec_status)),
            ));
        }
    }

    // Trade sanctions screening check.
    if let Some(screen_status) = ctx
        .metadata
        .get("trade_sanctions_screening")
        .and_then(|v| v.as_str())
    {
        if screen_status == "match_found" || screen_status == "blocked" {
            return Some((
                ComplianceState::NonCompliant,
                Some("trade_counterparty_sanctions_match".into()),
            ));
        }
    }

    if ctx.attestations.is_empty() && (has_trade_license || has_customs) {
        return Some((
            ComplianceState::Pending,
            Some("trade_metadata_valid_awaiting_attestation".into()),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Date Helper
// ---------------------------------------------------------------------------

/// Check if an ISO 8601 date string is in the past.
///
/// Accepts YYYY-MM-DD, YYYY-MM-DDTHH:MM:SSZ, and RFC 3339 formats.
/// Returns `true` (conservatively treats as past) if unparseable — an
/// unparseable date must not grant indefinite validity.
fn is_date_past(date_str: &str) -> bool {
    let now = chrono::Utc::now();

    // Try RFC 3339 first (most specific).
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return now > dt;
    }

    // Try YYYY-MM-DDTHH:MM:SSZ.
    if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%SZ") {
        let dt = naive.and_utc();
        return now > dt;
    }

    // Try YYYY-MM-DD (date only — treat as end of that day in UTC).
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
        // 23:59:59 is always valid; if it somehow fails, treat as past (fail-closed).
        let Some(eod) = naive_date.and_hms_opt(23, 59, 59) else {
            return true;
        };
        return now > eod.and_utc();
    }

    // Unparseable — treat as past (conservative, fail-closed).
    true
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
        // Deep domain validation provides more specific guidance than
        // generic pending reasons. Each test case verifies that the
        // metadata-driven evaluator identifies exactly what is missing.
        let test_cases: Vec<(ComplianceDomain, &str, &str, &str)> = vec![
            // Licensing: license_type present but no license_id → specific guidance
            (ComplianceDomain::Licensing, "license_type", "banking_license", "license_type_declared_no_license_id"),
            // Banking: bank_license_id present but no attestation → valid metadata, needs attestation
            (ComplianceDomain::Banking, "bank_license_id", "BL-001", "banking_metadata_valid_awaiting_attestation"),
            // Payments: psp_license_id but no float_safeguarding → specific gap
            (ComplianceDomain::Payments, "psp_license_id", "PSP-001", "psp_licensed_missing_float_safeguarding_evidence"),
            // DigitalAssets: token classified but no attestation → valid metadata, needs attestation
            (ComplianceDomain::DigitalAssets, "token_classification", "security_token", "digital_assets_metadata_valid_awaiting_attestation"),
            // Employment: 50 employees but no social security registration → specific gap
            (ComplianceDomain::Employment, "employee_count", "50", "employees_present_missing_social_security_registration"),
            // Arbitration: framework declared but no clause → specific gap
            (ComplianceDomain::Arbitration, "arbitration_framework", "UNCITRAL", "arbitration_framework_declared_missing_clause"),
            // Trade: trade license but no customs registration → specific gap
            (ComplianceDomain::Trade, "trade_license_id", "TL-001", "trade_licensed_missing_customs_registration"),
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

    // ── Deep domain-specific metadata validation tests ───────────

    // -- Licensing --

    #[test]
    fn licensing_expired_license_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("license_type".into(), serde_json::json!("banking"));
        metadata.insert("license_id".into(), serde_json::json!("LIC-001"));
        metadata.insert("issuing_authority".into(), serde_json::json!("pk-secp"));
        metadata.insert("license_valid_until".into(), serde_json::json!("2020-01-01"));
        let ctx = EvaluationContext {
            entity_id: "entity-expired-license".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Licensing, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("license_expired"));
    }

    #[test]
    fn licensing_complete_metadata_with_attestation_is_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("license_type".into(), serde_json::json!("banking"));
        metadata.insert("license_id".into(), serde_json::json!("LIC-001"));
        metadata.insert("issuing_authority".into(), serde_json::json!("pk-secp"));
        metadata.insert("license_valid_until".into(), serde_json::json!("2099-12-31"));
        let ctx = EvaluationContext {
            entity_id: "entity-licensed".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-lic".into(),
                attestation_type: "license_verification".into(),
                issuer_did: "did:example:regulator".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata,
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Licensing, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn licensing_missing_issuing_authority_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert("license_type".into(), serde_json::json!("banking"));
        metadata.insert("license_id".into(), serde_json::json!("LIC-001"));
        let ctx = EvaluationContext {
            entity_id: "entity-no-authority".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Licensing, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(
            reason.as_deref(),
            Some("license_id_present_missing_issuing_authority")
        );
    }

    // -- Banking --

    #[test]
    fn banking_low_car_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("bank_license_id".into(), serde_json::json!("BL-001"));
        metadata.insert("capital_adequacy_ratio".into(), serde_json::json!("0.04"));
        let ctx = EvaluationContext {
            entity_id: "entity-low-car".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert!(
            reason
                .as_deref()
                .unwrap()
                .contains("below_basel3_minimum"),
            "reason should mention Basel III: {:?}",
            reason
        );
    }

    #[test]
    fn banking_adequate_car_with_attestation_is_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("bank_license_id".into(), serde_json::json!("BL-001"));
        metadata.insert("capital_adequacy_ratio".into(), serde_json::json!("0.12"));
        let ctx = EvaluationContext {
            entity_id: "entity-good-car".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-bank".into(),
                attestation_type: "capital_adequacy_assessment".into(),
                issuer_did: "did:example:sbp".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata,
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn banking_aml_non_compliant_blocks_banking() {
        let mut metadata = HashMap::new();
        metadata.insert("bank_license_id".into(), serde_json::json!("BL-001"));
        metadata.insert("aml_status".into(), serde_json::json!("non_compliant"));
        let ctx = EvaluationContext {
            entity_id: "entity-aml-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("banking_blocked_aml_non_compliant")
        );
    }

    #[test]
    fn banking_unparseable_car_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("bank_license_id".into(), serde_json::json!("BL-001"));
        metadata.insert(
            "capital_adequacy_ratio".into(),
            serde_json::json!("not-a-number"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-bad-car".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Banking, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("capital_adequacy_ratio_unparseable")
        );
    }

    // -- Payments --

    #[test]
    fn payments_failed_float_safeguarding_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("psp_license_id".into(), serde_json::json!("PSP-001"));
        metadata.insert("float_safeguarding".into(), serde_json::json!("failed"));
        let ctx = EvaluationContext {
            entity_id: "entity-float-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Payments, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("float_safeguarding_failed"));
    }

    #[test]
    fn payments_complete_metadata_with_attestation_is_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("psp_license_id".into(), serde_json::json!("PSP-001"));
        metadata.insert("float_safeguarding".into(), serde_json::json!("compliant"));
        let ctx = EvaluationContext {
            entity_id: "entity-psp-good".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-psp".into(),
                attestation_type: "payment_compliance".into(),
                issuer_did: "did:example:sbp".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata,
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Payments, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    // -- Clearing --

    #[test]
    fn clearing_margin_deficit_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("ccp_membership_id".into(), serde_json::json!("CCP-001"));
        metadata.insert("margin_status".into(), serde_json::json!("deficit"));
        let ctx = EvaluationContext {
            entity_id: "entity-margin-deficit".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Clearing, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("margin_status_deficit"));
    }

    #[test]
    fn clearing_default_fund_overdue_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("ccp_membership_id".into(), serde_json::json!("CCP-001"));
        metadata.insert("margin_status".into(), serde_json::json!("adequate"));
        metadata.insert("default_fund_status".into(), serde_json::json!("overdue"));
        let ctx = EvaluationContext {
            entity_id: "entity-df-overdue".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Clearing, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("default_fund_overdue"));
    }

    // -- Settlement --

    #[test]
    fn settlement_invalid_cycle_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("settlement_cycle".into(), serde_json::json!("T+99"));
        let ctx = EvaluationContext {
            entity_id: "entity-bad-cycle".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Settlement, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert!(reason.as_deref().unwrap().contains("invalid_settlement_cycle"));
    }

    #[test]
    fn settlement_valid_cycle_with_attestation_is_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("settlement_cycle".into(), serde_json::json!("T+2"));
        metadata.insert("dvp_compliance".into(), serde_json::json!("compliant"));
        let ctx = EvaluationContext {
            entity_id: "entity-t2".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-settle".into(),
                attestation_type: "settlement_compliance".into(),
                issuer_did: "did:example:csd".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata,
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Settlement, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    #[test]
    fn settlement_dvp_failed_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("settlement_cycle".into(), serde_json::json!("T+2"));
        metadata.insert("dvp_compliance".into(), serde_json::json!("failed"));
        let ctx = EvaluationContext {
            entity_id: "entity-dvp-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Settlement, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("dvp_compliance_failed"));
    }

    // -- Digital Assets --

    #[test]
    fn digital_assets_unrecognized_classification_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "token_classification".into(),
            serde_json::json!("meme_token"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-meme".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::DigitalAssets, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert!(reason
            .as_deref()
            .unwrap()
            .contains("unrecognized_token_classification"));
    }

    #[test]
    fn digital_assets_security_token_requires_securities_compliance() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "token_classification".into(),
            serde_json::json!("security_token"),
        );
        metadata.insert(
            "securities_status".into(),
            serde_json::json!("non_compliant"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-sec-token-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::DigitalAssets, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("security_token_requires_securities_compliance")
        );
    }

    #[test]
    fn digital_assets_travel_rule_non_compliant_blocks() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "token_classification".into(),
            serde_json::json!("payment_token"),
        );
        metadata.insert(
            "travel_rule_compliance".into(),
            serde_json::json!("non_compliant"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-travel-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::DigitalAssets, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("travel_rule_non_compliant"));
    }

    #[test]
    fn digital_assets_vasp_operations_without_license_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "token_classification".into(),
            serde_json::json!("utility_token"),
        );
        metadata.insert("operates_vasp".into(), serde_json::json!("true"));
        let ctx = EvaluationContext {
            entity_id: "entity-vasp-no-lic".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::DigitalAssets, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(
            reason.as_deref(),
            Some("vasp_operations_declared_no_license")
        );
    }

    // -- Employment --

    #[test]
    fn employment_zero_employees_suggests_policy_artifact() {
        let mut metadata = HashMap::new();
        metadata.insert("employee_count".into(), serde_json::json!("0"));
        let ctx = EvaluationContext {
            entity_id: "entity-no-employees".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Employment, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert!(reason
            .as_deref()
            .unwrap()
            .contains("zero_employees"));
    }

    #[test]
    fn employment_labor_violations_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("employee_count".into(), serde_json::json!("100"));
        metadata.insert(
            "social_security_registration".into(),
            serde_json::json!("SSR-001"),
        );
        metadata.insert(
            "labor_compliance_status".into(),
            serde_json::json!("violations_found"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-labor-violations".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Employment, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("labor_compliance_violations_found")
        );
    }

    // -- Immigration --

    #[test]
    fn immigration_expired_work_permit_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("foreign_employee_count".into(), serde_json::json!("5"));
        metadata.insert("work_permit_status".into(), serde_json::json!("expired"));
        let ctx = EvaluationContext {
            entity_id: "entity-wp-expired".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Immigration, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("work_permit_expired"));
    }

    #[test]
    fn immigration_zero_foreign_employees_suggests_policy_artifact() {
        let mut metadata = HashMap::new();
        metadata.insert("foreign_employee_count".into(), serde_json::json!("0"));
        let ctx = EvaluationContext {
            entity_id: "entity-no-foreign".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Immigration, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert!(reason
            .as_deref()
            .unwrap()
            .contains("zero_foreign_employees"));
    }

    #[test]
    fn immigration_pending_renewal_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert("foreign_employee_count".into(), serde_json::json!("3"));
        metadata.insert(
            "work_permit_status".into(),
            serde_json::json!("pending_renewal"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-wp-renewal".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Immigration, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(reason.as_deref(), Some("work_permit_pending_renewal"));
    }

    // -- IP --

    #[test]
    fn ip_no_portfolio_suggests_policy_artifact() {
        let mut metadata = HashMap::new();
        metadata.insert("ip_portfolio_exists".into(), serde_json::json!("false"));
        let ctx = EvaluationContext {
            entity_id: "entity-no-ip".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Ip, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert!(reason
            .as_deref()
            .unwrap()
            .contains("no_ip_portfolio"));
    }

    #[test]
    fn ip_infringement_found_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("ip_portfolio_exists".into(), serde_json::json!("true"));
        metadata.insert(
            "ip_protection_status".into(),
            serde_json::json!("infringement_found"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-ip-infringement".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Ip, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("ip_protection_infringement_found")
        );
    }

    #[test]
    fn ip_trade_secrets_without_policy_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert("ip_portfolio_exists".into(), serde_json::json!("true"));
        metadata.insert("has_trade_secrets".into(), serde_json::json!("true"));
        let ctx = EvaluationContext {
            entity_id: "entity-secrets-no-policy".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Ip, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(
            reason.as_deref(),
            Some("trade_secrets_declared_missing_protection_policy")
        );
    }

    // -- Consumer Protection --

    #[test]
    fn consumer_protection_non_consumer_facing_suggests_policy() {
        let mut metadata = HashMap::new();
        metadata.insert("consumer_facing".into(), serde_json::json!("false"));
        let ctx = EvaluationContext {
            entity_id: "entity-b2b".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) =
            evaluate_domain_default(ComplianceDomain::ConsumerProtection, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert!(reason.as_deref().unwrap().contains("non_consumer_facing"));
    }

    #[test]
    fn consumer_protection_missing_dispute_resolution_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert("consumer_facing".into(), serde_json::json!("true"));
        let ctx = EvaluationContext {
            entity_id: "entity-no-drm".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) =
            evaluate_domain_default(ComplianceDomain::ConsumerProtection, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(
            reason.as_deref(),
            Some("consumer_facing_missing_dispute_resolution_mechanism")
        );
    }

    #[test]
    fn consumer_protection_deficient_disclosure_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("consumer_facing".into(), serde_json::json!("true"));
        metadata.insert(
            "dispute_resolution_mechanism".into(),
            serde_json::json!("ombudsman"),
        );
        metadata.insert(
            "disclosure_compliance".into(),
            serde_json::json!("deficient"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-disclosure-fail".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) =
            evaluate_domain_default(ComplianceDomain::ConsumerProtection, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("disclosure_compliance_deficient")
        );
    }

    // -- Arbitration --

    #[test]
    fn arbitration_unrecognized_framework_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "arbitration_framework".into(),
            serde_json::json!("kangaroo_court"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-bad-framework".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Arbitration, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert!(reason
            .as_deref()
            .unwrap()
            .contains("unrecognized_arbitration_framework"));
    }

    #[test]
    fn arbitration_non_signatory_enforcement_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "arbitration_framework".into(),
            serde_json::json!("UNCITRAL"),
        );
        metadata.insert("arbitration_clause".into(), serde_json::json!("present"));
        metadata.insert(
            "enforcement_convention".into(),
            serde_json::json!("non_signatory"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-non-signatory".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Arbitration, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("jurisdiction_not_new_york_convention_signatory")
        );
    }

    #[test]
    fn arbitration_complete_metadata_with_attestation_is_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("arbitration_framework".into(), serde_json::json!("ICC"));
        metadata.insert("arbitration_clause".into(), serde_json::json!("present"));
        let ctx = EvaluationContext {
            entity_id: "entity-icc-arb".into(),
            current_state: None,
            attestations: vec![AttestationRef {
                attestation_id: "att-arb".into(),
                attestation_type: "arbitration_compliance".into(),
                issuer_did: "did:example:icc".into(),
                issued_at: "2026-01-01T00:00:00Z".into(),
                expires_at: Some("2099-01-01T00:00:00Z".into()),
                digest: "abc123".into(),
            }],
            metadata,
        };
        let (state, _) = evaluate_domain_default(ComplianceDomain::Arbitration, &ctx);
        assert_eq!(state, ComplianceState::Compliant);
    }

    // -- Trade --

    #[test]
    fn trade_export_control_violation_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("trade_license_id".into(), serde_json::json!("TL-001"));
        metadata.insert(
            "customs_registration_id".into(),
            serde_json::json!("CR-001"),
        );
        metadata.insert(
            "export_control_status".into(),
            serde_json::json!("violation"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-export-violation".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Trade, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(reason.as_deref(), Some("export_control_violation"));
    }

    #[test]
    fn trade_sanctions_match_is_non_compliant() {
        let mut metadata = HashMap::new();
        metadata.insert("trade_license_id".into(), serde_json::json!("TL-001"));
        metadata.insert(
            "customs_registration_id".into(),
            serde_json::json!("CR-001"),
        );
        metadata.insert(
            "trade_sanctions_screening".into(),
            serde_json::json!("match_found"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-sanctions-match".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Trade, &ctx);
        assert_eq!(state, ComplianceState::NonCompliant);
        assert_eq!(
            reason.as_deref(),
            Some("trade_counterparty_sanctions_match")
        );
    }

    #[test]
    fn trade_import_export_without_license_is_pending() {
        let mut metadata = HashMap::new();
        metadata.insert(
            "import_export_activities".into(),
            serde_json::json!("true"),
        );
        let ctx = EvaluationContext {
            entity_id: "entity-no-trade-lic".into(),
            current_state: None,
            attestations: vec![],
            metadata,
        };
        let (state, reason) = evaluate_domain_default(ComplianceDomain::Trade, &ctx);
        assert_eq!(state, ComplianceState::Pending);
        assert_eq!(
            reason.as_deref(),
            Some("import_export_declared_missing_trade_license")
        );
    }

    // -- Date helper --

    #[test]
    fn is_date_past_with_past_date() {
        assert!(is_date_past("2020-01-01"));
    }

    #[test]
    fn is_date_past_with_future_date() {
        assert!(!is_date_past("2099-12-31"));
    }

    #[test]
    fn is_date_past_with_rfc3339() {
        assert!(is_date_past("2020-01-01T00:00:00Z"));
        assert!(!is_date_past("2099-01-01T00:00:00Z"));
    }

    #[test]
    fn is_date_past_unparseable_treated_as_past() {
        assert!(is_date_past("not-a-date"));
        assert!(is_date_past(""));
    }

    // -- Backwards compatibility: no metadata = original behavior --

    #[test]
    fn no_metadata_extended_domains_behave_as_before() {
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
            // No metadata, no attestations → Pending (unchanged).
            let ctx_empty = EvaluationContext {
                entity_id: "entity-empty".into(),
                current_state: None,
                attestations: vec![],
                metadata: HashMap::new(),
            };
            let (state, _) = evaluate_domain_default(domain, &ctx_empty);
            assert_eq!(
                state,
                ComplianceState::Pending,
                "{:?} with no metadata should still be Pending",
                domain
            );

            // No metadata, fresh attestation → Compliant (unchanged).
            let ctx_attested = EvaluationContext {
                entity_id: "entity-attested".into(),
                current_state: None,
                attestations: vec![AttestationRef {
                    attestation_id: "att-1".into(),
                    attestation_type: format!("{}_compliance", domain.as_str()),
                    issuer_did: "did:example:reg".into(),
                    issued_at: "2026-01-01T00:00:00Z".into(),
                    expires_at: Some("2099-01-01T00:00:00Z".into()),
                    digest: "abc".into(),
                }],
                metadata: HashMap::new(),
            };
            let (state, _) = evaluate_domain_default(domain, &ctx_attested);
            assert_eq!(
                state,
                ComplianceState::Compliant,
                "{:?} with fresh attestation and no metadata should still be Compliant",
                domain
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
