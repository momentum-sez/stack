//! # Corridor Typestate Machine
//!
//! Implements the corridor lifecycle from spec §40-corridors:
//! `DRAFT → PENDING → ACTIVE` with `HALTED` and `SUSPENDED` branches.
//!
//! Each state is a distinct type. Only valid transitions exist as methods.
//! Invalid transitions are compile errors.
//!
//! ## Transitions
//!
//! ```text
//! DRAFT ─submit()──▶ PENDING ─activate()──▶ ACTIVE
//!                                              │
//!                                     ┌────────┴────────┐
//!                                     │                 │
//!                                  halt()          suspend()
//!                                     │                 │
//!                                     ▼                 ▼
//!                                  HALTED          SUSPENDED
//!                                     │                 │
//!                                deprecate()       resume()
//!                                     │                 │
//!                                     ▼                 ▼
//!                                DEPRECATED          ACTIVE
//! ```
//!
//! ## Audit Reference
//!
//! Finding §2.3: The Python implementation used string states `"PROPOSED"`
//! and `"OPERATIONAL"` that diverged from the spec's `"DRAFT"`, `"PENDING"`,
//! `"ACTIVE"`. This typestate encoding makes divergence structurally impossible.

use std::marker::PhantomData;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use msez_core::{ContentDigest, CorridorId, JurisdictionId};

// ── State Types ──────────────────────────────────────────────────────

/// The initial draft state for a new corridor.
#[derive(Debug, Clone, Copy)]
pub struct Draft;

/// Corridor has been submitted and is pending regulatory approval.
#[derive(Debug, Clone, Copy)]
pub struct Pending;

/// Corridor is active and processing cross-border transactions.
#[derive(Debug, Clone, Copy)]
pub struct Active;

/// Corridor has been halted by a jurisdiction authority.
#[derive(Debug, Clone, Copy)]
pub struct Halted;

/// Corridor is temporarily suspended with an expected resume date.
#[derive(Debug, Clone, Copy)]
pub struct Suspended;

/// Corridor has been permanently deprecated. Terminal state.
#[derive(Debug, Clone, Copy)]
pub struct Deprecated;

/// Marker trait for all valid corridor states. Sealed — only the six
/// states defined in this module implement it.
pub trait CorridorState: private::Sealed + std::fmt::Debug {
    /// The canonical state name as it appears in spec and audit trail.
    fn name() -> &'static str;
    /// Whether this is a terminal state (no further transitions).
    fn is_terminal() -> bool {
        false
    }
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Draft {}
    impl Sealed for super::Pending {}
    impl Sealed for super::Active {}
    impl Sealed for super::Halted {}
    impl Sealed for super::Suspended {}
    impl Sealed for super::Deprecated {}
}

impl CorridorState for Draft {
    fn name() -> &'static str {
        "DRAFT"
    }
}
impl CorridorState for Pending {
    fn name() -> &'static str {
        "PENDING"
    }
}
impl CorridorState for Active {
    fn name() -> &'static str {
        "ACTIVE"
    }
}
impl CorridorState for Halted {
    fn name() -> &'static str {
        "HALTED"
    }
}
impl CorridorState for Suspended {
    fn name() -> &'static str {
        "SUSPENDED"
    }
}
impl CorridorState for Deprecated {
    fn name() -> &'static str {
        "DEPRECATED"
    }
    fn is_terminal() -> bool {
        true
    }
}

// ── Evidence Types ───────────────────────────────────────────────────

/// Evidence required to submit a corridor from Draft to Pending.
#[derive(Debug, Clone)]
pub struct SubmissionEvidence {
    /// Digest of the bilateral agreement between jurisdictions.
    pub bilateral_agreement_digest: ContentDigest,
    /// Digest of the pack trilogy (lawpack + regpack + licensepack).
    pub pack_trilogy_digest: ContentDigest,
}

/// Evidence required to activate a corridor from Pending to Active.
#[derive(Debug, Clone)]
pub struct ActivationEvidence {
    /// Regulatory approval digest from jurisdiction A.
    pub regulatory_approval_a: ContentDigest,
    /// Regulatory approval digest from jurisdiction B.
    pub regulatory_approval_b: ContentDigest,
}

/// Reason for halting a corridor.
#[derive(Debug, Clone)]
pub struct HaltReason {
    /// Human-readable reason for the halt.
    pub reason: String,
    /// The jurisdiction authority that issued the halt.
    pub authority: JurisdictionId,
    /// Digest of the halt evidence.
    pub evidence: ContentDigest,
}

/// Reason for suspending a corridor.
#[derive(Debug, Clone)]
pub struct SuspendReason {
    /// Human-readable reason for the suspension.
    pub reason: String,
    /// Expected resume date, if known.
    pub expected_resume: Option<DateTime<Utc>>,
}

/// Evidence required to resume a suspended corridor.
#[derive(Debug, Clone)]
pub struct ResumeEvidence {
    /// Digest of the resolution attestation.
    pub resolution_attestation: ContentDigest,
}

/// Evidence required to deprecate a halted corridor.
#[derive(Debug, Clone)]
pub struct DeprecationEvidence {
    /// Digest of the deprecation decision (e.g., bilateral agreement to sunset).
    pub deprecation_decision_digest: ContentDigest,
    /// Human-readable reason for deprecation.
    pub reason: String,
}

// ── Transition Record ────────────────────────────────────────────────

/// A record of a single state transition in the corridor lifecycle.
///
/// Every transition is logged with the source and target states, a
/// timestamp, and the digest of the evidence that authorized it. This
/// provides a complete audit trail for regulatory review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// State before the transition.
    pub from_state: String,
    /// State after the transition.
    pub to_state: String,
    /// When the transition occurred.
    pub timestamp: DateTime<Utc>,
    /// Digest of the evidence authorizing this transition.
    pub evidence_digest: Option<ContentDigest>,
}

// ── The Corridor ─────────────────────────────────────────────────────

/// A trade corridor between two jurisdictions, parameterized by its
/// current lifecycle state.
///
/// State-specific methods ensure that only valid transitions can be
/// called. For example, `Corridor<Draft>` has `.submit()` but not
/// `.halt()`, while `Corridor<Active>` has `.halt()` and `.suspend()`
/// but not `.submit()`.
///
/// ## Compile-Time Safety
///
/// ```text
/// let draft = Corridor::<Draft>::new(id, jurisdiction_a, jurisdiction_b);
/// // draft.halt(reason); // ERROR: no method named `halt` on `Corridor<Draft>`
/// ```
#[derive(Debug)]
pub struct Corridor<S: CorridorState> {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// The first jurisdiction in the bilateral corridor.
    pub jurisdiction_a: JurisdictionId,
    /// The second jurisdiction in the bilateral corridor.
    pub jurisdiction_b: JurisdictionId,
    /// When the corridor was created.
    pub created_at: DateTime<Utc>,
    /// When the corridor was last updated.
    pub updated_at: DateTime<Utc>,
    /// Internal state data.
    inner: CorridorInner,
    /// Phantom data for the state type parameter.
    _state: PhantomData<S>,
}

#[derive(Debug, Clone)]
struct CorridorInner {
    pack_trilogy_digest: Option<ContentDigest>,
    halt_reason: Option<HaltReason>,
    suspend_reason: Option<SuspendReason>,
    transition_log: Vec<TransitionRecord>,
}

impl<S: CorridorState> Corridor<S> {
    /// Return the canonical name of the current state.
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Whether the corridor is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Access the transition log for audit purposes.
    pub fn transition_log(&self) -> &[TransitionRecord] {
        &self.inner.transition_log
    }

    /// Convert internal data to a new state type, consuming self.
    fn transmute_to<T: CorridorState>(self, evidence_digest: Option<ContentDigest>) -> Corridor<T> {
        let mut inner = self.inner;
        inner.transition_log.push(TransitionRecord {
            from_state: S::name().to_string(),
            to_state: T::name().to_string(),
            timestamp: Utc::now(),
            evidence_digest,
        });
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            updated_at: Utc::now(),
            inner,
            _state: PhantomData,
        }
    }
}

// ── State-Specific Methods ───────────────────────────────────────────

impl Corridor<Draft> {
    /// Create a new corridor in Draft state.
    pub fn new(
        id: CorridorId,
        jurisdiction_a: JurisdictionId,
        jurisdiction_b: JurisdictionId,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            jurisdiction_a,
            jurisdiction_b,
            created_at: now,
            updated_at: now,
            inner: CorridorInner {
                pack_trilogy_digest: None,
                halt_reason: None,
                suspend_reason: None,
                transition_log: Vec::new(),
            },
            _state: PhantomData,
        }
    }

    /// Submit the corridor for regulatory approval.
    ///
    /// Transitions: Draft → Pending.
    ///
    /// Requires bilateral agreement and pack trilogy evidence.
    pub fn submit(mut self, evidence: SubmissionEvidence) -> Corridor<Pending> {
        self.inner.pack_trilogy_digest = Some(evidence.pack_trilogy_digest.clone());
        self.transmute_to(Some(evidence.bilateral_agreement_digest))
    }
}

impl Corridor<Pending> {
    /// Activate the corridor after regulatory approval from both jurisdictions.
    ///
    /// Transitions: Pending → Active.
    pub fn activate(self, evidence: ActivationEvidence) -> Corridor<Active> {
        self.transmute_to(Some(evidence.regulatory_approval_a))
    }
}

impl Corridor<Active> {
    /// Halt the corridor by jurisdiction authority order.
    ///
    /// Transitions: Active → Halted.
    ///
    /// A halt is an emergency action (e.g., fork detection, safety incident).
    pub fn halt(mut self, reason: HaltReason) -> Corridor<Halted> {
        let evidence = reason.evidence.clone();
        self.inner.halt_reason = Some(reason);
        self.transmute_to(Some(evidence))
    }

    /// Suspend the corridor temporarily with an expected resume date.
    ///
    /// Transitions: Active → Suspended.
    pub fn suspend(mut self, reason: SuspendReason) -> Corridor<Suspended> {
        self.inner.suspend_reason = Some(reason);
        self.transmute_to(None)
    }
}

impl Corridor<Suspended> {
    /// Resume the corridor after the suspension condition is resolved.
    ///
    /// Transitions: Suspended → Active.
    pub fn resume(mut self, evidence: ResumeEvidence) -> Corridor<Active> {
        self.inner.suspend_reason = None;
        self.transmute_to(Some(evidence.resolution_attestation))
    }
}

impl Corridor<Halted> {
    /// Deprecate a halted corridor permanently.
    ///
    /// Transitions: Halted → Deprecated.
    ///
    /// This is a terminal transition. Once deprecated, the corridor cannot
    /// be reactivated. All pending receipts should be migrated before calling.
    pub fn deprecate(self, evidence: DeprecationEvidence) -> Corridor<Deprecated> {
        self.transmute_to(Some(evidence.deprecation_decision_digest))
    }
}

// ── DynCorridor ──────────────────────────────────────────────────────

/// Serializable corridor snapshot for persistence/deserialization use
/// cases where the state is not known at compile time.
///
/// Use the typestate `Corridor<S>` for business logic where the state
/// is known; use `DynCorridorData` for storage and API serialization
/// where the state arrives as a string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynCorridorData {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// The first jurisdiction.
    pub jurisdiction_a: JurisdictionId,
    /// The second jurisdiction.
    pub jurisdiction_b: JurisdictionId,
    /// Current state name (spec-aligned: DRAFT, PENDING, ACTIVE, HALTED, SUSPENDED, DEPRECATED).
    pub state: DynCorridorState,
    /// When the corridor was created.
    pub created_at: DateTime<Utc>,
    /// When the corridor was last updated.
    pub updated_at: DateTime<Utc>,
    /// Transition history.
    pub transition_log: Vec<TransitionRecord>,
}

/// Runtime corridor state enum for serialization/deserialization.
///
/// Uses spec-aligned names only. There is no variant for the defective
/// Python v1 names (`PROPOSED`, `OPERATIONAL`) — they are structurally
/// excluded from the type system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DynCorridorState {
    /// Initial draft state.
    #[serde(rename = "DRAFT")]
    Draft,
    /// Submitted, pending regulatory approval.
    #[serde(rename = "PENDING")]
    Pending,
    /// Active and processing transactions.
    #[serde(rename = "ACTIVE")]
    Active,
    /// Halted by jurisdiction authority.
    #[serde(rename = "HALTED")]
    Halted,
    /// Temporarily suspended.
    #[serde(rename = "SUSPENDED")]
    Suspended,
    /// Permanently deprecated. Terminal.
    #[serde(rename = "DEPRECATED")]
    Deprecated,
}

impl DynCorridorState {
    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Pending => "PENDING",
            Self::Active => "ACTIVE",
            Self::Halted => "HALTED",
            Self::Suspended => "SUSPENDED",
            Self::Deprecated => "DEPRECATED",
        }
    }

    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deprecated)
    }

    /// Return the set of valid target states from this state.
    pub fn valid_transitions(&self) -> &'static [DynCorridorState] {
        match self {
            Self::Draft => &[Self::Pending],
            Self::Pending => &[Self::Active],
            Self::Active => &[Self::Halted, Self::Suspended],
            Self::Halted => &[Self::Deprecated],
            Self::Suspended => &[Self::Active],
            Self::Deprecated => &[],
        }
    }
}

impl std::fmt::Display for DynCorridorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl<S: CorridorState> From<&Corridor<S>> for DynCorridorData {
    fn from(c: &Corridor<S>) -> Self {
        let state = match S::name() {
            "DRAFT" => DynCorridorState::Draft,
            "PENDING" => DynCorridorState::Pending,
            "ACTIVE" => DynCorridorState::Active,
            "HALTED" => DynCorridorState::Halted,
            "SUSPENDED" => DynCorridorState::Suspended,
            "DEPRECATED" => DynCorridorState::Deprecated,
            _ => unreachable!("sealed trait guarantees exhaustive state names"),
        };
        DynCorridorData {
            id: c.id.clone(),
            jurisdiction_a: c.jurisdiction_a.clone(),
            jurisdiction_b: c.jurisdiction_b.clone(),
            state,
            created_at: c.created_at,
            updated_at: c.updated_at,
            transition_log: c.inner.transition_log.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest, CanonicalBytes};
    use serde_json::json;

    fn test_digest() -> ContentDigest {
        let canonical = CanonicalBytes::new(&json!({"test": "evidence"})).unwrap();
        sha256_digest(&canonical)
    }

    fn test_corridor() -> Corridor<Draft> {
        Corridor::new(
            CorridorId::new(),
            JurisdictionId::new("PK-RSEZ").unwrap(),
            JurisdictionId::new("AE-DIFC").unwrap(),
        )
    }

    #[test]
    fn draft_state_name() {
        let corridor = test_corridor();
        assert_eq!(corridor.state_name(), "DRAFT");
        assert!(!corridor.is_terminal());
    }

    #[test]
    fn draft_to_pending() {
        let draft = test_corridor();
        let evidence = SubmissionEvidence {
            bilateral_agreement_digest: test_digest(),
            pack_trilogy_digest: test_digest(),
        };
        let pending = draft.submit(evidence);
        assert_eq!(pending.state_name(), "PENDING");
        assert_eq!(pending.transition_log().len(), 1);
        assert_eq!(pending.transition_log()[0].from_state, "DRAFT");
        assert_eq!(pending.transition_log()[0].to_state, "PENDING");
    }

    #[test]
    fn pending_to_active() {
        let draft = test_corridor();
        let pending = draft.submit(SubmissionEvidence {
            bilateral_agreement_digest: test_digest(),
            pack_trilogy_digest: test_digest(),
        });
        let active = pending.activate(ActivationEvidence {
            regulatory_approval_a: test_digest(),
            regulatory_approval_b: test_digest(),
        });
        assert_eq!(active.state_name(), "ACTIVE");
        assert_eq!(active.transition_log().len(), 2);
    }

    #[test]
    fn active_to_halted() {
        let active = test_corridor()
            .submit(SubmissionEvidence {
                bilateral_agreement_digest: test_digest(),
                pack_trilogy_digest: test_digest(),
            })
            .activate(ActivationEvidence {
                regulatory_approval_a: test_digest(),
                regulatory_approval_b: test_digest(),
            });
        let halted = active.halt(HaltReason {
            reason: "Fork detected".to_string(),
            authority: JurisdictionId::new("PK-RSEZ").unwrap(),
            evidence: test_digest(),
        });
        assert_eq!(halted.state_name(), "HALTED");
        assert!(!halted.is_terminal());
    }

    #[test]
    fn active_to_suspended_and_resume() {
        let active = test_corridor()
            .submit(SubmissionEvidence {
                bilateral_agreement_digest: test_digest(),
                pack_trilogy_digest: test_digest(),
            })
            .activate(ActivationEvidence {
                regulatory_approval_a: test_digest(),
                regulatory_approval_b: test_digest(),
            });

        let suspended = active.suspend(SuspendReason {
            reason: "Maintenance window".to_string(),
            expected_resume: None,
        });
        assert_eq!(suspended.state_name(), "SUSPENDED");

        let resumed = suspended.resume(ResumeEvidence {
            resolution_attestation: test_digest(),
        });
        assert_eq!(resumed.state_name(), "ACTIVE");
    }

    #[test]
    fn halted_to_deprecated() {
        let halted = test_corridor()
            .submit(SubmissionEvidence {
                bilateral_agreement_digest: test_digest(),
                pack_trilogy_digest: test_digest(),
            })
            .activate(ActivationEvidence {
                regulatory_approval_a: test_digest(),
                regulatory_approval_b: test_digest(),
            })
            .halt(HaltReason {
                reason: "Permanent issue".to_string(),
                authority: JurisdictionId::new("PK-RSEZ").unwrap(),
                evidence: test_digest(),
            });

        let deprecated = halted.deprecate(DeprecationEvidence {
            deprecation_decision_digest: test_digest(),
            reason: "Corridor sunset".to_string(),
        });
        assert_eq!(deprecated.state_name(), "DEPRECATED");
        assert!(deprecated.is_terminal());
    }

    #[test]
    fn transition_log_records_full_history() {
        let corridor = test_corridor()
            .submit(SubmissionEvidence {
                bilateral_agreement_digest: test_digest(),
                pack_trilogy_digest: test_digest(),
            })
            .activate(ActivationEvidence {
                regulatory_approval_a: test_digest(),
                regulatory_approval_b: test_digest(),
            })
            .halt(HaltReason {
                reason: "Issue".to_string(),
                authority: JurisdictionId::new("PK-RSEZ").unwrap(),
                evidence: test_digest(),
            })
            .deprecate(DeprecationEvidence {
                deprecation_decision_digest: test_digest(),
                reason: "Sunset".to_string(),
            });

        let log = corridor.transition_log();
        assert_eq!(log.len(), 4);
        assert_eq!(log[0].from_state, "DRAFT");
        assert_eq!(log[0].to_state, "PENDING");
        assert_eq!(log[1].from_state, "PENDING");
        assert_eq!(log[1].to_state, "ACTIVE");
        assert_eq!(log[2].from_state, "ACTIVE");
        assert_eq!(log[2].to_state, "HALTED");
        assert_eq!(log[3].from_state, "HALTED");
        assert_eq!(log[3].to_state, "DEPRECATED");
    }

    #[test]
    fn dyn_corridor_from_typed() {
        let corridor = test_corridor();
        let dyn_data = DynCorridorData::from(&corridor);
        assert_eq!(dyn_data.state, DynCorridorState::Draft);
        assert_eq!(dyn_data.state.as_str(), "DRAFT");
    }

    #[test]
    fn dyn_corridor_state_serialization() {
        let state = DynCorridorState::Active;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"ACTIVE\"");

        let deserialized: DynCorridorState = serde_json::from_str("\"HALTED\"").unwrap();
        assert_eq!(deserialized, DynCorridorState::Halted);
    }

    #[test]
    fn dyn_corridor_state_display() {
        assert_eq!(format!("{}", DynCorridorState::Draft), "DRAFT");
        assert_eq!(format!("{}", DynCorridorState::Pending), "PENDING");
        assert_eq!(format!("{}", DynCorridorState::Active), "ACTIVE");
        assert_eq!(format!("{}", DynCorridorState::Halted), "HALTED");
        assert_eq!(format!("{}", DynCorridorState::Suspended), "SUSPENDED");
        assert_eq!(format!("{}", DynCorridorState::Deprecated), "DEPRECATED");
    }

    #[test]
    fn dyn_corridor_state_terminal() {
        assert!(!DynCorridorState::Draft.is_terminal());
        assert!(!DynCorridorState::Active.is_terminal());
        assert!(DynCorridorState::Deprecated.is_terminal());
    }

    /// Verify that the Python v1 defective state names cannot be deserialized.
    /// Spec §2.3: the names PROPOSED and OPERATIONAL are wrong.
    #[test]
    fn no_defective_state_names() {
        let result: Result<DynCorridorState, _> = serde_json::from_str("\"PROPOSED\"");
        assert!(result.is_err(), "PROPOSED must not be a valid state");

        let result: Result<DynCorridorState, _> = serde_json::from_str("\"OPERATIONAL\"");
        assert!(result.is_err(), "OPERATIONAL must not be a valid state");
    }

    #[test]
    fn dyn_corridor_valid_transitions() {
        assert_eq!(
            DynCorridorState::Draft.valid_transitions(),
            &[DynCorridorState::Pending]
        );
        assert_eq!(
            DynCorridorState::Active.valid_transitions(),
            &[DynCorridorState::Halted, DynCorridorState::Suspended]
        );
        assert!(DynCorridorState::Deprecated.valid_transitions().is_empty());
    }

    #[test]
    fn corridor_preserves_jurisdiction_ids() {
        let id = CorridorId::new();
        let ja = JurisdictionId::new("PK-RSEZ").unwrap();
        let jb = JurisdictionId::new("AE-DIFC").unwrap();
        let corridor = Corridor::<Draft>::new(id.clone(), ja.clone(), jb.clone());
        assert_eq!(corridor.id, id);
        assert_eq!(corridor.jurisdiction_a, ja);
        assert_eq!(corridor.jurisdiction_b, jb);
    }
}
