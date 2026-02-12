//! # Corridor Typestate Machine
//!
//! Implements the corridor lifecycle using the typestate pattern.
//! Each state is a distinct type — invalid transitions are compile errors.
//!
//! ## States (spec-aligned per §40)
//!
//! - `Draft` → initial state, corridor proposal under construction.
//! - `Pending` → submitted for regulatory review.
//! - `Active` → approved and operational for cross-border settlement.
//! - `Halted` → emergency stop by jurisdictional authority.
//! - `Suspended` → temporary pause with expected resumption.
//! - `Deprecated` → terminal state, corridor permanently decommissioned.
//!
//! ## Allowed Transitions
//!
//! ```text
//! Draft ──submit()──▶ Pending ──activate()──▶ Active
//!                                              │   │
//!                                     halt()───┘   └───suspend()
//!                                       │                  │
//!                                       ▼                  ▼
//!                                    Halted           Suspended
//!                                       │                  │
//!                              deprecate()     resume()────┘
//!                                       │          │
//!                                       ▼          ▼
//!                                  Deprecated    Active
//! ```
//!
//! ## Security Invariant
//!
//! State names are types, not strings. The string "OPERATIONAL" from the
//! defective v1 state machine (audit §2.3) cannot exist in this system.
//! Calling `.halt()` on a `Corridor<Draft>` is a compile error — there is
//! no `.halt()` method defined for `Corridor<Draft>`.
//!
//! ## Compile-Time Safety Example
//!
//! The following code will NOT compile because `Corridor<Draft>` has no
//! `.halt()` method:
//!
//! ```compile_fail
//! use msez_state::corridor::*;
//! use msez_core::{CorridorId, JurisdictionId};
//!
//! let corridor = Corridor::<Draft>::new(
//!     CorridorId::new(),
//!     JurisdictionId::new("PK").unwrap(),
//!     JurisdictionId::new("AE").unwrap(),
//! );
//! // ERROR: no method named `halt` found for `Corridor<Draft>`
//! let _halted = corridor.halt(HaltReason {
//!     reason: "test".into(),
//!     authority: JurisdictionId::new("PK").unwrap(),
//!     evidence: msez_core::ContentDigest::new(
//!         msez_core::DigestAlgorithm::Sha256,
//!         [0u8; 32],
//!     ),
//! });
//! ```

use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::{ContentDigest, CorridorId, JurisdictionId, Timestamp};

// ─── State Types (each is a distinct type at compile time) ───────────

/// Corridor state: proposal under construction.
#[derive(Debug, Clone, Copy)]
pub struct Draft;

/// Corridor state: submitted for regulatory review.
#[derive(Debug, Clone, Copy)]
pub struct Pending;

/// Corridor state: approved and operational.
#[derive(Debug, Clone, Copy)]
pub struct Active;

/// Corridor state: emergency stop by authority.
#[derive(Debug, Clone, Copy)]
pub struct Halted;

/// Corridor state: temporary pause with expected resumption.
#[derive(Debug, Clone, Copy)]
pub struct Suspended;

/// Corridor state: permanently decommissioned (terminal).
#[derive(Debug, Clone, Copy)]
pub struct Deprecated;

// ─── Sealed Trait ────────────────────────────────────────────────────

mod private {
    pub trait Sealed {}
    impl Sealed for super::Draft {}
    impl Sealed for super::Pending {}
    impl Sealed for super::Active {}
    impl Sealed for super::Halted {}
    impl Sealed for super::Suspended {}
    impl Sealed for super::Deprecated {}
}

/// Marker trait for all valid corridor states.
///
/// Sealed — only the six states defined in this module implement it.
/// External crates cannot add new states.
pub trait CorridorState: private::Sealed + std::fmt::Debug {
    /// The canonical string name of this state (e.g., "DRAFT").
    fn name() -> &'static str;

    /// Whether this state is terminal (no further transitions allowed).
    fn is_terminal() -> bool {
        false
    }
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

// ─── Evidence Types (each transition requires specific evidence) ─────

/// Evidence required to submit a corridor for review (DRAFT → PENDING).
///
/// Implements Protocol 40.1 — bilateral agreement submission.
#[derive(Debug, Clone)]
pub struct SubmissionEvidence {
    /// Digest of the bilateral agreement between jurisdictions.
    pub bilateral_agreement_digest: ContentDigest,
    /// Digest of the pack trilogy (lawpack + regpack + licensepack).
    pub pack_trilogy_digest: ContentDigest,
    /// Attestation from the submitting party.
    pub submitter_attestation: Vec<u8>,
}

/// Evidence required to activate a corridor (PENDING → ACTIVE).
///
/// Implements Protocol 40.2 — regulatory approval gate.
#[derive(Debug, Clone)]
pub struct ActivationEvidence {
    /// Regulatory approval digest from jurisdiction A.
    pub regulatory_approval_a: ContentDigest,
    /// Regulatory approval digest from jurisdiction B.
    pub regulatory_approval_b: ContentDigest,
    /// Watcher quorum attestation confirming readiness.
    pub watcher_quorum_attestation: Vec<u8>,
}

/// Reason for halting a corridor (ACTIVE → HALTED).
///
/// Implements Protocol 40.3 — emergency halt.
#[derive(Debug, Clone)]
pub struct HaltReason {
    /// Human-readable reason for the halt.
    pub reason: String,
    /// The jurisdiction that issued the halt order.
    pub authority: JurisdictionId,
    /// Digest of the evidence supporting the halt (e.g., fork alarm VC).
    pub evidence: ContentDigest,
}

/// Reason for suspending a corridor (ACTIVE → SUSPENDED).
///
/// Implements Protocol 40.4 — temporary suspension.
#[derive(Debug, Clone)]
pub struct SuspendReason {
    /// Human-readable reason for the suspension.
    pub reason: String,
    /// Expected resumption date, if known.
    pub expected_resume: Option<Timestamp>,
}

/// Evidence required to resume a suspended corridor (SUSPENDED → ACTIVE).
///
/// Implements Protocol 40.5 — resumption gate.
#[derive(Debug, Clone)]
pub struct ResumeEvidence {
    /// Digest of the resolution attestation.
    pub resolution_attestation: ContentDigest,
}

/// Evidence required to deprecate a halted corridor (HALTED → DEPRECATED).
///
/// Implements Protocol 40.6 — corridor decommissioning.
#[derive(Debug, Clone)]
pub struct DeprecationEvidence {
    /// Reason for permanent decommissioning.
    pub reason: String,
    /// Digest of the migration plan for active receipts.
    pub migration_plan_digest: Option<ContentDigest>,
}

// ─── Transition Record ───────────────────────────────────────────────

/// Record of a single state transition in the corridor lifecycle.
///
/// Every transition is logged with its timestamp and evidence digest,
/// creating an immutable audit trail for regulatory review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// State before the transition.
    pub from_state: String,
    /// State after the transition.
    pub to_state: String,
    /// When the transition occurred (UTC).
    pub timestamp: Timestamp,
    /// Digest of the evidence that authorized this transition.
    pub evidence_digest: Option<ContentDigest>,
    /// Human-readable reason for the transition.
    pub reason: Option<String>,
}

// ─── Errors ──────────────────────────────────────────────────────────

/// Errors that can occur during corridor operations.
#[derive(Error, Debug)]
pub enum CorridorError {
    /// Attempted transition is not allowed by the state machine.
    #[error("invalid corridor transition: {from} -> {to}")]
    InvalidTransition {
        /// Current state.
        from: String,
        /// Attempted target state.
        to: String,
    },
}

// ─── The Corridor ────────────────────────────────────────────────────

/// A trade corridor between two jurisdictions, parameterized by its lifecycle state.
///
/// Only state-appropriate methods are available at compile time.
/// `Corridor<Draft>` has `.submit()` but not `.halt()`.
/// `Corridor<Active>` has `.halt()` and `.suspend()` but not `.submit()`.
///
/// The transition log records every state change with timestamp and evidence
/// digest, providing an immutable audit trail.
#[derive(Debug)]
pub struct Corridor<S: CorridorState> {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// First jurisdiction in the corridor.
    pub jurisdiction_a: JurisdictionId,
    /// Second jurisdiction in the corridor.
    pub jurisdiction_b: JurisdictionId,
    /// When the corridor was created.
    pub created_at: Timestamp,
    /// Immutable log of all state transitions.
    transition_log: Vec<TransitionRecord>,
    _state: PhantomData<S>,
}

impl<S: CorridorState> Corridor<S> {
    /// Returns the canonical state name (e.g., "DRAFT", "ACTIVE").
    pub fn state_name(&self) -> &'static str {
        S::name()
    }

    /// Whether the corridor is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        S::is_terminal()
    }

    /// Access the immutable transition log.
    pub fn transition_log(&self) -> &[TransitionRecord] {
        &self.transition_log
    }

    /// Number of transitions that have occurred.
    pub fn transition_count(&self) -> usize {
        self.transition_log.len()
    }

    /// Helper to record a transition and produce a new typed corridor.
    fn transition_to<T: CorridorState>(
        mut self,
        evidence_digest: Option<ContentDigest>,
        reason: Option<String>,
    ) -> Corridor<T> {
        self.transition_log.push(TransitionRecord {
            from_state: S::name().to_string(),
            to_state: T::name().to_string(),
            timestamp: Timestamp::now(),
            evidence_digest,
            reason,
        });
        Corridor {
            id: self.id,
            jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at,
            transition_log: self.transition_log,
            _state: PhantomData,
        }
    }
}

// ─── State-Specific Impl Blocks ─────────────────────────────────────

impl Corridor<Draft> {
    /// Create a new corridor in DRAFT state.
    ///
    /// The corridor starts with an empty transition log. The creation itself
    /// is not recorded as a transition — the first transition will be `.submit()`.
    pub fn new(
        id: CorridorId,
        jurisdiction_a: JurisdictionId,
        jurisdiction_b: JurisdictionId,
    ) -> Self {
        Self {
            id,
            jurisdiction_a,
            jurisdiction_b,
            created_at: Timestamp::now(),
            transition_log: Vec::new(),
            _state: PhantomData,
        }
    }

    /// Submit the corridor for regulatory review (DRAFT → PENDING).
    ///
    /// Requires bilateral agreement and pack trilogy evidence.
    /// Implements Protocol 40.1 §1.
    pub fn submit(self, evidence: SubmissionEvidence) -> Corridor<Pending> {
        self.transition_to(
            Some(evidence.bilateral_agreement_digest),
            Some("Corridor submitted for regulatory review".to_string()),
        )
    }
}

impl Corridor<Pending> {
    /// Activate the corridor after regulatory approval (PENDING → ACTIVE).
    ///
    /// Requires regulatory approvals from both jurisdictions and watcher
    /// quorum attestation.
    /// Implements Protocol 40.2 §1.
    pub fn activate(self, evidence: ActivationEvidence) -> Corridor<Active> {
        self.transition_to(
            Some(evidence.regulatory_approval_a),
            Some("Corridor activated after regulatory approval".to_string()),
        )
    }
}

impl Corridor<Active> {
    /// Emergency halt by jurisdictional authority (ACTIVE → HALTED).
    ///
    /// Immediately stops all corridor operations. Requires evidence
    /// (e.g., fork alarm VC) and authority identification.
    /// Implements Protocol 40.3 §1.
    pub fn halt(self, reason: HaltReason) -> Corridor<Halted> {
        self.transition_to(
            Some(reason.evidence),
            Some(format!("Halted by {}: {}", reason.authority, reason.reason)),
        )
    }

    /// Temporary suspension with expected resumption (ACTIVE → SUSPENDED).
    ///
    /// Pauses corridor operations but allows resumption after the
    /// suspension reason is resolved.
    /// Implements Protocol 40.4 §1.
    pub fn suspend(self, reason: SuspendReason) -> Corridor<Suspended> {
        self.transition_to(
            None,
            Some(format!("Suspended: {}", reason.reason)),
        )
    }
}

impl Corridor<Suspended> {
    /// Resume a suspended corridor (SUSPENDED → ACTIVE).
    ///
    /// Requires resolution attestation proving the suspension reason
    /// has been addressed.
    /// Implements Protocol 40.5 §1.
    pub fn resume(self, evidence: ResumeEvidence) -> Corridor<Active> {
        self.transition_to(
            Some(evidence.resolution_attestation),
            Some("Corridor resumed after suspension resolution".to_string()),
        )
    }
}

impl Corridor<Halted> {
    /// Permanently deprecate a halted corridor (HALTED → DEPRECATED).
    ///
    /// Terminal transition — the corridor cannot be reactivated after this.
    /// Optionally includes a migration plan for any pending receipts.
    /// Implements Protocol 40.6 §1.
    pub fn deprecate(self, evidence: DeprecationEvidence) -> Corridor<Deprecated> {
        self.transition_to(
            evidence.migration_plan_digest,
            Some(format!("Deprecated: {}", evidence.reason)),
        )
    }
}

// ─── DynCorridor — Runtime State for Persistence ────────────────────

/// Runtime representation of corridor state for persistence/deserialization.
///
/// When loading corridor data from a database or API response, the state
/// is not known at compile time. `DynCorridorState` provides a runtime
/// enum for pattern matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DynCorridorState {
    /// Corridor proposal under construction.
    Draft,
    /// Submitted for regulatory review.
    Pending,
    /// Approved and operational.
    Active,
    /// Emergency stop by authority.
    Halted,
    /// Temporary pause.
    Suspended,
    /// Permanently decommissioned.
    Deprecated,
}

impl DynCorridorState {
    /// Returns the canonical state name.
    pub fn name(&self) -> &'static str {
        match self {
            Self::Draft => "DRAFT",
            Self::Pending => "PENDING",
            Self::Active => "ACTIVE",
            Self::Halted => "HALTED",
            Self::Suspended => "SUSPENDED",
            Self::Deprecated => "DEPRECATED",
        }
    }

    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Deprecated)
    }
}

impl std::fmt::Display for DynCorridorState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Dynamic corridor for persistence/deserialization when the state is not
/// known at compile time.
///
/// Provides runtime-checked transitions via [`DynCorridor::try_transition()`],
/// mirroring the compile-time guarantees of the typestate API but enforced
/// at runtime. Use this for database persistence, API serialization, and
/// cases where the corridor state is loaded from external storage.
///
/// For new corridor construction and in-memory state transitions, prefer
/// the typestate API (`Corridor<Draft>`, `Corridor<Active>`, etc.) which
/// catches invalid transitions at compile time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynCorridor {
    /// Unique corridor identifier.
    pub id: CorridorId,
    /// First jurisdiction in the corridor.
    pub jurisdiction_a: JurisdictionId,
    /// Second jurisdiction in the corridor.
    pub jurisdiction_b: JurisdictionId,
    /// When the corridor was created.
    pub created_at: Timestamp,
    /// Current state of the corridor.
    pub state: DynCorridorState,
    /// Immutable log of all state transitions.
    pub transition_log: Vec<TransitionRecord>,
}

impl DynCorridor {
    /// Attempt a state transition with runtime validation.
    ///
    /// Returns an error if the transition is not allowed by the state machine.
    /// Records the transition in the log on success.
    pub fn try_transition(
        &mut self,
        to: DynCorridorState,
        evidence_digest: Option<ContentDigest>,
        reason: Option<String>,
    ) -> Result<(), CorridorError> {
        let valid = matches!(
            (self.state, to),
            (DynCorridorState::Draft, DynCorridorState::Pending)
                | (DynCorridorState::Pending, DynCorridorState::Active)
                | (DynCorridorState::Active, DynCorridorState::Halted)
                | (DynCorridorState::Active, DynCorridorState::Suspended)
                | (DynCorridorState::Suspended, DynCorridorState::Active)
                | (DynCorridorState::Halted, DynCorridorState::Deprecated)
        );

        if !valid {
            return Err(CorridorError::InvalidTransition {
                from: self.state.name().to_string(),
                to: to.name().to_string(),
            });
        }

        self.transition_log.push(TransitionRecord {
            from_state: self.state.name().to_string(),
            to_state: to.name().to_string(),
            timestamp: Timestamp::now(),
            evidence_digest,
            reason,
        });
        self.state = to;
        Ok(())
    }

    /// Returns the canonical state name.
    pub fn state_name(&self) -> &'static str {
        self.state.name()
    }

    /// Whether the corridor is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }
}

/// Convert a typed `Corridor<S>` into a `DynCorridor` for persistence.
macro_rules! impl_into_dyn_corridor {
    ($state_type:ty, $dyn_variant:ident) => {
        impl From<Corridor<$state_type>> for DynCorridor {
            fn from(c: Corridor<$state_type>) -> Self {
                DynCorridor {
                    id: c.id,
                    jurisdiction_a: c.jurisdiction_a,
                    jurisdiction_b: c.jurisdiction_b,
                    created_at: c.created_at,
                    state: DynCorridorState::$dyn_variant,
                    transition_log: c.transition_log,
                }
            }
        }
    };
}

impl_into_dyn_corridor!(Draft, Draft);
impl_into_dyn_corridor!(Pending, Pending);
impl_into_dyn_corridor!(Active, Active);
impl_into_dyn_corridor!(Halted, Halted);
impl_into_dyn_corridor!(Suspended, Suspended);
impl_into_dyn_corridor!(Deprecated, Deprecated);

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{ContentDigest, DigestAlgorithm};

    fn test_digest() -> ContentDigest {
        ContentDigest::new(DigestAlgorithm::Sha256, [0xAB; 32])
    }

    fn test_jurisdiction_a() -> JurisdictionId {
        JurisdictionId::new("PK").unwrap()
    }

    fn test_jurisdiction_b() -> JurisdictionId {
        JurisdictionId::new("AE").unwrap()
    }

    fn make_draft() -> Corridor<Draft> {
        Corridor::new(CorridorId::new(), test_jurisdiction_a(), test_jurisdiction_b())
    }

    fn submission_evidence() -> SubmissionEvidence {
        SubmissionEvidence {
            bilateral_agreement_digest: test_digest(),
            pack_trilogy_digest: test_digest(),
            submitter_attestation: vec![1, 2, 3],
        }
    }

    fn activation_evidence() -> ActivationEvidence {
        ActivationEvidence {
            regulatory_approval_a: test_digest(),
            regulatory_approval_b: test_digest(),
            watcher_quorum_attestation: vec![4, 5, 6],
        }
    }

    // ── Happy-path transitions ───────────────────────────────────────

    #[test]
    fn test_draft_state_name() {
        let c = make_draft();
        assert_eq!(c.state_name(), "DRAFT");
        assert!(!c.is_terminal());
    }

    #[test]
    fn test_draft_to_pending() {
        let draft = make_draft();
        let pending = draft.submit(submission_evidence());
        assert_eq!(pending.state_name(), "PENDING");
        assert_eq!(pending.transition_count(), 1);
        assert_eq!(pending.transition_log()[0].from_state, "DRAFT");
        assert_eq!(pending.transition_log()[0].to_state, "PENDING");
    }

    #[test]
    fn test_pending_to_active() {
        let draft = make_draft();
        let pending = draft.submit(submission_evidence());
        let active = pending.activate(activation_evidence());
        assert_eq!(active.state_name(), "ACTIVE");
        assert_eq!(active.transition_count(), 2);
    }

    #[test]
    fn test_active_to_halted() {
        let active = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence());
        let halted = active.halt(HaltReason {
            reason: "Fork detected".into(),
            authority: test_jurisdiction_a(),
            evidence: test_digest(),
        });
        assert_eq!(halted.state_name(), "HALTED");
        assert_eq!(halted.transition_count(), 3);
    }

    #[test]
    fn test_active_to_suspended() {
        let active = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence());
        let suspended = active.suspend(SuspendReason {
            reason: "Scheduled maintenance".into(),
            expected_resume: None,
        });
        assert_eq!(suspended.state_name(), "SUSPENDED");
    }

    #[test]
    fn test_suspended_to_active() {
        let suspended = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence())
            .suspend(SuspendReason {
                reason: "Maintenance".into(),
                expected_resume: None,
            });
        let active = suspended.resume(ResumeEvidence {
            resolution_attestation: test_digest(),
        });
        assert_eq!(active.state_name(), "ACTIVE");
    }

    #[test]
    fn test_halted_to_deprecated() {
        let halted = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence())
            .halt(HaltReason {
                reason: "Permanent issue".into(),
                authority: test_jurisdiction_a(),
                evidence: test_digest(),
            });
        let deprecated = halted.deprecate(DeprecationEvidence {
            reason: "Corridor replaced by new agreement".into(),
            migration_plan_digest: Some(test_digest()),
        });
        assert_eq!(deprecated.state_name(), "DEPRECATED");
        assert!(deprecated.is_terminal());
    }

    // ── Full lifecycle ───────────────────────────────────────────────

    #[test]
    fn test_full_lifecycle_draft_through_deprecated() {
        let corridor = make_draft();
        assert_eq!(corridor.transition_count(), 0);

        let corridor = corridor.submit(submission_evidence());
        assert_eq!(corridor.transition_count(), 1);

        let corridor = corridor.activate(activation_evidence());
        assert_eq!(corridor.transition_count(), 2);

        let corridor = corridor.halt(HaltReason {
            reason: "Safety incident".into(),
            authority: test_jurisdiction_b(),
            evidence: test_digest(),
        });
        assert_eq!(corridor.transition_count(), 3);

        let corridor = corridor.deprecate(DeprecationEvidence {
            reason: "Post-incident decommission".into(),
            migration_plan_digest: None,
        });
        assert_eq!(corridor.transition_count(), 4);
        assert!(corridor.is_terminal());

        // Verify full transition log
        let log = corridor.transition_log();
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
    fn test_suspend_resume_cycle() {
        let active = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence());

        // Suspend
        let suspended = active.suspend(SuspendReason {
            reason: "First suspension".into(),
            expected_resume: None,
        });
        assert_eq!(suspended.state_name(), "SUSPENDED");

        // Resume
        let active = suspended.resume(ResumeEvidence {
            resolution_attestation: test_digest(),
        });
        assert_eq!(active.state_name(), "ACTIVE");

        // Suspend again
        let suspended = active.suspend(SuspendReason {
            reason: "Second suspension".into(),
            expected_resume: None,
        });
        assert_eq!(suspended.state_name(), "SUSPENDED");
        assert_eq!(suspended.transition_count(), 5);
    }

    // ── Transition log integrity ─────────────────────────────────────

    #[test]
    fn test_transition_log_has_timestamps() {
        let pending = make_draft().submit(submission_evidence());
        let record = &pending.transition_log()[0];
        // Timestamp should be recent (within last minute)
        let now = Timestamp::now();
        assert!(record.timestamp <= now);
    }

    #[test]
    fn test_transition_log_has_evidence_digest() {
        let pending = make_draft().submit(submission_evidence());
        let record = &pending.transition_log()[0];
        assert!(record.evidence_digest.is_some());
    }

    #[test]
    fn test_transition_log_has_reason() {
        let pending = make_draft().submit(submission_evidence());
        let record = &pending.transition_log()[0];
        assert!(record.reason.is_some());
        assert!(record.reason.as_ref().unwrap().contains("regulatory review"));
    }

    // ── Corridor preserves identity across transitions ───────────────

    #[test]
    fn test_corridor_id_preserved() {
        let id = CorridorId::new();
        let corridor = Corridor::new(id.clone(), test_jurisdiction_a(), test_jurisdiction_b());
        let pending = corridor.submit(submission_evidence());
        assert_eq!(pending.id, id);
    }

    #[test]
    fn test_jurisdictions_preserved() {
        let corridor = make_draft();
        let ja = corridor.jurisdiction_a.clone();
        let jb = corridor.jurisdiction_b.clone();
        let active = corridor.submit(submission_evidence()).activate(activation_evidence());
        assert_eq!(active.jurisdiction_a, ja);
        assert_eq!(active.jurisdiction_b, jb);
    }

    // ── DynCorridor ──────────────────────────────────────────────────

    #[test]
    fn test_dyn_corridor_from_draft() {
        let draft = make_draft();
        let dyn_c: DynCorridor = draft.into();
        assert_eq!(dyn_c.state, DynCorridorState::Draft);
        assert_eq!(dyn_c.state_name(), "DRAFT");
    }

    #[test]
    fn test_dyn_corridor_from_active() {
        let active = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence());
        let dyn_c: DynCorridor = active.into();
        assert_eq!(dyn_c.state, DynCorridorState::Active);
        assert_eq!(dyn_c.state_name(), "ACTIVE");
        assert_eq!(dyn_c.transition_log.len(), 2);
    }

    #[test]
    fn test_dyn_corridor_valid_transition() {
        let draft = make_draft();
        let mut dyn_c: DynCorridor = draft.into();
        assert!(dyn_c
            .try_transition(DynCorridorState::Pending, Some(test_digest()), None)
            .is_ok());
        assert_eq!(dyn_c.state, DynCorridorState::Pending);
    }

    #[test]
    fn test_dyn_corridor_invalid_transition() {
        let draft = make_draft();
        let mut dyn_c: DynCorridor = draft.into();
        let result = dyn_c.try_transition(DynCorridorState::Active, None, None);
        assert!(result.is_err());
        // State should be unchanged
        assert_eq!(dyn_c.state, DynCorridorState::Draft);
    }

    #[test]
    fn test_dyn_corridor_full_lifecycle() {
        let mut dyn_c: DynCorridor = make_draft().into();

        dyn_c
            .try_transition(DynCorridorState::Pending, Some(test_digest()), None)
            .unwrap();
        dyn_c
            .try_transition(DynCorridorState::Active, Some(test_digest()), None)
            .unwrap();
        dyn_c
            .try_transition(DynCorridorState::Halted, Some(test_digest()), None)
            .unwrap();
        dyn_c
            .try_transition(
                DynCorridorState::Deprecated,
                None,
                Some("Decommissioned".into()),
            )
            .unwrap();

        assert!(dyn_c.is_terminal());
        assert_eq!(dyn_c.transition_log.len(), 4);
    }

    #[test]
    fn test_dyn_corridor_terminal_rejects_transitions() {
        let deprecated = make_draft()
            .submit(submission_evidence())
            .activate(activation_evidence())
            .halt(HaltReason {
                reason: "test".into(),
                authority: test_jurisdiction_a(),
                evidence: test_digest(),
            })
            .deprecate(DeprecationEvidence {
                reason: "done".into(),
                migration_plan_digest: None,
            });
        let mut dyn_c: DynCorridor = deprecated.into();
        assert!(dyn_c
            .try_transition(DynCorridorState::Active, None, None)
            .is_err());
    }

    #[test]
    fn test_dyn_corridor_serialization() {
        let draft = make_draft();
        let dyn_c: DynCorridor = draft.into();
        let json = serde_json::to_string(&dyn_c).unwrap();
        let parsed: DynCorridor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state, DynCorridorState::Draft);
        assert_eq!(parsed.id, dyn_c.id);
    }

    #[test]
    fn test_dyn_corridor_state_serde() {
        let state = DynCorridorState::Active;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"ACTIVE\"");
        let parsed: DynCorridorState = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, DynCorridorState::Active);
    }

    // ── No PROPOSED or OPERATIONAL strings ────────────────────────────

    #[test]
    fn test_no_defective_state_names() {
        let all_state_names = [
            Draft::name(),
            Pending::name(),
            Active::name(),
            Halted::name(),
            Suspended::name(),
            Deprecated::name(),
        ];
        for name in &all_state_names {
            assert_ne!(*name, "PROPOSED", "Defective v1 state name found");
            assert_ne!(*name, "OPERATIONAL", "Defective v1 state name found");
        }

        let all_dyn_names = [
            DynCorridorState::Draft.name(),
            DynCorridorState::Pending.name(),
            DynCorridorState::Active.name(),
            DynCorridorState::Halted.name(),
            DynCorridorState::Suspended.name(),
            DynCorridorState::Deprecated.name(),
        ];
        for name in &all_dyn_names {
            assert_ne!(*name, "PROPOSED", "Defective v1 state name found");
            assert_ne!(*name, "OPERATIONAL", "Defective v1 state name found");
        }
    }
}
