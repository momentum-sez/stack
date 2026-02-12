//! # Entity Lifecycle State Machine
//!
//! Models the lifecycle of legal entities (companies, SPVs, trusts) within
//! a jurisdiction, including the 10-stage dissolution process.
//!
//! ## States
//!
//! ```text
//! Formation ──▶ Active ──▶ Suspended ──▶ Active (reinstatement)
//!                  │
//!                  ▼
//!          Dissolution Stage 1: BoardResolution
//!                  │
//!                  ▼
//!          Dissolution Stage 2: RegulatoryNotification
//!                  │
//!                  ▼
//!          Dissolution Stage 3: CreditorNotification
//!                  │
//!                  ▼
//!          Dissolution Stage 4: TaxClearance
//!                  │
//!                  ▼
//!          Dissolution Stage 5: AssetLiquidation
//!                  │
//!                  ▼
//!          Dissolution Stage 6: LiabilitySettlement
//!                  │
//!                  ▼
//!          Dissolution Stage 7: EmployeeSettlement
//!                  │
//!                  ▼
//!          Dissolution Stage 8: FinalAudit
//!                  │
//!                  ▼
//!          Dissolution Stage 9: RegulatoryFiling
//!                  │
//!                  ▼
//!          Dissolution Stage 10: Deregistration
//!                  │
//!                  ▼
//!              Dissolved (terminal)
//! ```
//!
//! ## Design Decision
//!
//! The dissolution process uses an enum with validated transitions rather than
//! 10 typestate types. With 10 dissolution stages plus Formation, Active,
//! Suspended, and Dissolved, a full typestate approach would require 14 zero-sized
//! types and 14 impl blocks — unwieldy without proportional safety benefit.
//! The enum approach with `transition()` returning `Result` rejects invalid
//! transitions at runtime, which is appropriate since dissolution stages are
//! sequential and the invariant (stage N requires stage N-1 completion) is
//! straightforward to validate.
//!
//! ## Implements
//!
//! Spec §5 — Entity lifecycle and dissolution protocol.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::{EntityId, Timestamp};

// ─── Dissolution Stages ──────────────────────────────────────────────

/// The 10 stages of entity dissolution.
///
/// These stages must be completed sequentially. Each stage has specific
/// evidence requirements and regulatory gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum DissolutionStage {
    /// Stage 1: Board resolution to dissolve.
    BoardResolution = 1,
    /// Stage 2: Notification to regulatory authorities.
    RegulatoryNotification = 2,
    /// Stage 3: Notification to creditors (public notice period).
    CreditorNotification = 3,
    /// Stage 4: Tax clearance from revenue authority (FBR for Pakistan).
    TaxClearance = 4,
    /// Stage 5: Liquidation of entity assets.
    AssetLiquidation = 5,
    /// Stage 6: Settlement of outstanding liabilities.
    LiabilitySettlement = 6,
    /// Stage 7: Settlement of employee obligations.
    EmployeeSettlement = 7,
    /// Stage 8: Final financial audit.
    FinalAudit = 8,
    /// Stage 9: Filing final regulatory documents.
    RegulatoryFiling = 9,
    /// Stage 10: Deregistration from corporate registry (SECP for Pakistan).
    Deregistration = 10,
}

impl DissolutionStage {
    /// The numeric stage number (1-10).
    pub fn number(&self) -> u8 {
        *self as u8
    }

    /// The next stage in the dissolution sequence, if any.
    pub fn next(&self) -> Option<DissolutionStage> {
        match self {
            Self::BoardResolution => Some(Self::RegulatoryNotification),
            Self::RegulatoryNotification => Some(Self::CreditorNotification),
            Self::CreditorNotification => Some(Self::TaxClearance),
            Self::TaxClearance => Some(Self::AssetLiquidation),
            Self::AssetLiquidation => Some(Self::LiabilitySettlement),
            Self::LiabilitySettlement => Some(Self::EmployeeSettlement),
            Self::EmployeeSettlement => Some(Self::FinalAudit),
            Self::FinalAudit => Some(Self::RegulatoryFiling),
            Self::RegulatoryFiling => Some(Self::Deregistration),
            Self::Deregistration => None,
        }
    }

    /// Whether this is the final dissolution stage.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Deregistration)
    }

    /// Total number of dissolution stages.
    pub const STAGE_COUNT: u8 = 10;
}

impl std::fmt::Display for DissolutionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::BoardResolution => "BOARD_RESOLUTION",
            Self::RegulatoryNotification => "REGULATORY_NOTIFICATION",
            Self::CreditorNotification => "CREDITOR_NOTIFICATION",
            Self::TaxClearance => "TAX_CLEARANCE",
            Self::AssetLiquidation => "ASSET_LIQUIDATION",
            Self::LiabilitySettlement => "LIABILITY_SETTLEMENT",
            Self::EmployeeSettlement => "EMPLOYEE_SETTLEMENT",
            Self::FinalAudit => "FINAL_AUDIT",
            Self::RegulatoryFiling => "REGULATORY_FILING",
            Self::Deregistration => "DEREGISTRATION",
        };
        f.write_str(s)
    }
}

// ─── Entity Lifecycle State ──────────────────────────────────────────

/// The lifecycle state of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityLifecycleState {
    /// Entity is being formed (initial registration).
    Formation,
    /// Entity is active and operational.
    Active,
    /// Entity is temporarily suspended by regulatory authority.
    Suspended,
    /// Entity is undergoing dissolution at the specified stage.
    Dissolution(DissolutionStage),
    /// Entity has been fully dissolved (terminal).
    Dissolved,
}

impl EntityLifecycleState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Dissolved)
    }

    /// Whether the entity is in dissolution.
    pub fn is_dissolving(&self) -> bool {
        matches!(self, Self::Dissolution(_))
    }

    /// The dissolution stage, if currently dissolving.
    pub fn dissolution_stage(&self) -> Option<DissolutionStage> {
        match self {
            Self::Dissolution(stage) => Some(*stage),
            _ => None,
        }
    }
}

impl std::fmt::Display for EntityLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Formation => write!(f, "FORMATION"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Suspended => write!(f, "SUSPENDED"),
            Self::Dissolution(stage) => write!(f, "DISSOLUTION_STAGE_{}", stage.number()),
            Self::Dissolved => write!(f, "DISSOLVED"),
        }
    }
}

// ─── Errors ──────────────────────────────────────────────────────────

/// Errors that can occur during entity lifecycle transitions.
#[derive(Error, Debug)]
pub enum EntityError {
    /// Attempted transition is not valid from the current state.
    #[error("invalid entity transition: {from} -> {to}")]
    InvalidTransition {
        /// Current state.
        from: String,
        /// Attempted target state.
        to: String,
    },

    /// Attempted to skip a dissolution stage.
    #[error("dissolution stage {attempted} requires completion of stage {required} first")]
    StageSkipped {
        /// The stage that was attempted.
        attempted: String,
        /// The stage that must be completed first.
        required: String,
    },

    /// Entity is in a terminal state.
    #[error("entity {entity_id} is dissolved and cannot transition")]
    AlreadyDissolved {
        /// The entity identifier.
        entity_id: String,
    },
}

// ─── Transition Evidence ─────────────────────────────────────────────

/// Evidence for an entity lifecycle transition.
#[derive(Debug, Clone)]
pub struct EntityTransitionEvidence {
    /// Reason for the transition.
    pub reason: String,
    /// Actor who initiated the transition.
    pub actor: Option<String>,
    /// Evidence digest.
    pub evidence_digest: Option<String>,
}

/// Record of an entity state transition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTransitionRecord {
    /// State before the transition.
    pub from_state: EntityLifecycleState,
    /// State after the transition.
    pub to_state: EntityLifecycleState,
    /// When the transition occurred.
    pub timestamp: Timestamp,
    /// Reason for the transition.
    pub reason: String,
}

// ─── Entity ──────────────────────────────────────────────────────────

/// An entity with its lifecycle state and transition history.
///
/// Enforces valid state transitions. Invalid transitions are rejected
/// with structured errors identifying the current state, attempted
/// transition, and the reason for rejection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    /// Unique entity identifier.
    pub id: EntityId,
    /// Current lifecycle state.
    pub state: EntityLifecycleState,
    /// When the entity was created.
    pub created_at: Timestamp,
    /// Ordered log of all state transitions.
    pub transitions: Vec<EntityTransitionRecord>,
}

impl Entity {
    /// Create a new entity in the Formation state.
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            state: EntityLifecycleState::Formation,
            created_at: Timestamp::now(),
            transitions: Vec::new(),
        }
    }

    /// Activate the entity (FORMATION → ACTIVE).
    ///
    /// Requires that the entity is currently in Formation state.
    pub fn activate(&mut self, evidence: EntityTransitionEvidence) -> Result<(), EntityError> {
        self.require_state(EntityLifecycleState::Formation, "ACTIVE")?;
        self.do_transition(EntityLifecycleState::Active, &evidence.reason);
        Ok(())
    }

    /// Suspend the entity (ACTIVE → SUSPENDED).
    ///
    /// Only active entities can be suspended.
    pub fn suspend(&mut self, evidence: EntityTransitionEvidence) -> Result<(), EntityError> {
        self.require_state(EntityLifecycleState::Active, "SUSPENDED")?;
        self.do_transition(EntityLifecycleState::Suspended, &evidence.reason);
        Ok(())
    }

    /// Reinstate a suspended entity (SUSPENDED → ACTIVE).
    pub fn reinstate(&mut self, evidence: EntityTransitionEvidence) -> Result<(), EntityError> {
        self.require_state(EntityLifecycleState::Suspended, "ACTIVE")?;
        self.do_transition(EntityLifecycleState::Active, &evidence.reason);
        Ok(())
    }

    /// Initiate dissolution (ACTIVE → DISSOLUTION Stage 1).
    ///
    /// Begins the 10-stage dissolution process starting with BoardResolution.
    pub fn initiate_dissolution(
        &mut self,
        evidence: EntityTransitionEvidence,
    ) -> Result<(), EntityError> {
        self.require_state(EntityLifecycleState::Active, "DISSOLUTION_STAGE_1")?;
        self.do_transition(
            EntityLifecycleState::Dissolution(DissolutionStage::BoardResolution),
            &evidence.reason,
        );
        Ok(())
    }

    /// Advance to the next dissolution stage.
    ///
    /// Dissolution stages must be completed sequentially. Attempting to
    /// skip a stage returns an error.
    pub fn advance_dissolution(
        &mut self,
        evidence: EntityTransitionEvidence,
    ) -> Result<DissolutionStage, EntityError> {
        let current_stage = match self.state {
            EntityLifecycleState::Dissolution(stage) => stage,
            _ => {
                return Err(EntityError::InvalidTransition {
                    from: self.state.to_string(),
                    to: "next dissolution stage".to_string(),
                });
            }
        };

        match current_stage.next() {
            Some(next_stage) => {
                self.do_transition(
                    EntityLifecycleState::Dissolution(next_stage),
                    &evidence.reason,
                );
                Ok(next_stage)
            }
            None => {
                // Final stage completed → transition to Dissolved
                self.do_transition(EntityLifecycleState::Dissolved, &evidence.reason);
                // Return the final stage that was just completed
                Ok(current_stage)
            }
        }
    }

    /// Complete dissolution after the final stage (Deregistration → Dissolved).
    ///
    /// This is called when the final dissolution stage (Deregistration) is
    /// complete and the entity should transition to the terminal Dissolved state.
    pub fn finalize_dissolution(
        &mut self,
        evidence: EntityTransitionEvidence,
    ) -> Result<(), EntityError> {
        match self.state {
            EntityLifecycleState::Dissolution(DissolutionStage::Deregistration) => {
                self.do_transition(EntityLifecycleState::Dissolved, &evidence.reason);
                Ok(())
            }
            _ => Err(EntityError::InvalidTransition {
                from: self.state.to_string(),
                to: "DISSOLVED".to_string(),
            }),
        }
    }

    /// Whether the entity is dissolved (terminal state).
    pub fn is_dissolved(&self) -> bool {
        self.state.is_terminal()
    }

    /// The current dissolution stage, if the entity is dissolving.
    pub fn dissolution_stage(&self) -> Option<DissolutionStage> {
        self.state.dissolution_stage()
    }

    /// Validate that the entity is in the expected state.
    fn require_state(
        &self,
        expected: EntityLifecycleState,
        target: &str,
    ) -> Result<(), EntityError> {
        if self.state.is_terminal() {
            return Err(EntityError::AlreadyDissolved {
                entity_id: self.id.to_string(),
            });
        }
        if self.state != expected {
            return Err(EntityError::InvalidTransition {
                from: self.state.to_string(),
                to: target.to_string(),
            });
        }
        Ok(())
    }

    /// Record a state transition.
    fn do_transition(&mut self, to: EntityLifecycleState, reason: &str) {
        self.transitions.push(EntityTransitionRecord {
            from_state: self.state,
            to_state: to,
            timestamp: Timestamp::now(),
            reason: reason.to_string(),
        });
        self.state = to;
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(reason: &str) -> EntityTransitionEvidence {
        EntityTransitionEvidence {
            reason: reason.to_string(),
            actor: Some("test-actor".to_string()),
            evidence_digest: None,
        }
    }

    fn make_entity() -> Entity {
        Entity::new(EntityId::new())
    }

    fn make_active_entity() -> Entity {
        let mut e = make_entity();
        e.activate(evidence("Registered")).unwrap();
        e
    }

    // ── Basic lifecycle tests ────────────────────────────────────────

    #[test]
    fn test_new_entity_is_in_formation() {
        let e = make_entity();
        assert_eq!(e.state, EntityLifecycleState::Formation);
        assert!(!e.is_dissolved());
    }

    #[test]
    fn test_formation_to_active() {
        let mut e = make_entity();
        e.activate(evidence("Registration complete")).unwrap();
        assert_eq!(e.state, EntityLifecycleState::Active);
        assert_eq!(e.transitions.len(), 1);
    }

    #[test]
    fn test_active_to_suspended() {
        let mut e = make_active_entity();
        e.suspend(evidence("Regulatory suspension")).unwrap();
        assert_eq!(e.state, EntityLifecycleState::Suspended);
    }

    #[test]
    fn test_suspended_to_active() {
        let mut e = make_active_entity();
        e.suspend(evidence("Suspended")).unwrap();
        e.reinstate(evidence("Reinstated")).unwrap();
        assert_eq!(e.state, EntityLifecycleState::Active);
    }

    #[test]
    fn test_cannot_suspend_formation() {
        let mut e = make_entity();
        let result = e.suspend(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_activate_active() {
        let mut e = make_active_entity();
        let result = e.activate(evidence("test"));
        assert!(result.is_err());
    }

    // ── Dissolution tests ────────────────────────────────────────────

    #[test]
    fn test_initiate_dissolution() {
        let mut e = make_active_entity();
        e.initiate_dissolution(evidence("Board voted to dissolve"))
            .unwrap();
        assert_eq!(
            e.state,
            EntityLifecycleState::Dissolution(DissolutionStage::BoardResolution)
        );
        assert!(e.state.is_dissolving());
    }

    #[test]
    fn test_cannot_dissolve_from_formation() {
        let mut e = make_entity();
        let result = e.initiate_dissolution(evidence("test"));
        assert!(result.is_err());
    }

    #[test]
    fn test_advance_all_10_dissolution_stages() {
        let mut e = make_active_entity();
        e.initiate_dissolution(evidence("Begin dissolution")).unwrap();

        let expected_stages = [
            DissolutionStage::RegulatoryNotification,
            DissolutionStage::CreditorNotification,
            DissolutionStage::TaxClearance,
            DissolutionStage::AssetLiquidation,
            DissolutionStage::LiabilitySettlement,
            DissolutionStage::EmployeeSettlement,
            DissolutionStage::FinalAudit,
            DissolutionStage::RegulatoryFiling,
            DissolutionStage::Deregistration,
        ];

        // Advance through stages 2-10
        for expected in &expected_stages {
            let stage = e.advance_dissolution(evidence(&format!("Stage {}", expected.number()))).unwrap();
            assert_eq!(stage, *expected);
            assert_eq!(e.state, EntityLifecycleState::Dissolution(*expected));
        }

        // One more advance from Deregistration completes dissolution
        let final_stage = e.advance_dissolution(evidence("Deregistration complete")).unwrap();
        assert_eq!(final_stage, DissolutionStage::Deregistration);
        assert!(e.is_dissolved());
        assert_eq!(e.state, EntityLifecycleState::Dissolved);
    }

    #[test]
    fn test_dissolution_stage_count() {
        assert_eq!(DissolutionStage::STAGE_COUNT, 10);
    }

    #[test]
    fn test_dissolution_stage_ordering() {
        assert!(DissolutionStage::BoardResolution < DissolutionStage::RegulatoryNotification);
        assert!(DissolutionStage::RegulatoryNotification < DissolutionStage::Deregistration);
    }

    #[test]
    fn test_finalize_dissolution_from_deregistration() {
        let mut e = make_active_entity();
        e.initiate_dissolution(evidence("dissolve")).unwrap();

        // Advance to final stage (9 advances: stage 1 → stage 10)
        for _ in 0..9 {
            e.advance_dissolution(evidence("next stage")).unwrap();
        }
        assert_eq!(
            e.state,
            EntityLifecycleState::Dissolution(DissolutionStage::Deregistration)
        );

        e.finalize_dissolution(evidence("Final deregistration"))
            .unwrap();
        assert!(e.is_dissolved());
    }

    #[test]
    fn test_finalize_dissolution_from_wrong_stage_fails() {
        let mut e = make_active_entity();
        e.initiate_dissolution(evidence("dissolve")).unwrap();

        // Still at stage 1 — cannot finalize
        let result = e.finalize_dissolution(evidence("too early"));
        assert!(result.is_err());
    }

    #[test]
    fn test_dissolved_is_terminal() {
        let mut e = make_active_entity();
        e.initiate_dissolution(evidence("dissolve")).unwrap();
        // 9 advances to reach Deregistration, 1 more to complete dissolution
        for _ in 0..10 {
            e.advance_dissolution(evidence("next")).unwrap();
        }
        assert!(e.is_dissolved());

        // Cannot transition from dissolved
        let result = e.activate(evidence("should fail"));
        assert!(result.is_err());
        match result.unwrap_err() {
            EntityError::AlreadyDissolved { .. } => {}
            other => panic!("Expected AlreadyDissolved, got: {other:?}"),
        }
    }

    // ── Transition log tests ─────────────────────────────────────────

    #[test]
    fn test_transition_log_records_all_changes() {
        let mut e = make_active_entity();
        e.suspend(evidence("suspend")).unwrap();
        e.reinstate(evidence("reinstate")).unwrap();

        assert_eq!(e.transitions.len(), 3);
        assert_eq!(e.transitions[0].from_state, EntityLifecycleState::Formation);
        assert_eq!(e.transitions[0].to_state, EntityLifecycleState::Active);
        assert_eq!(e.transitions[1].from_state, EntityLifecycleState::Active);
        assert_eq!(e.transitions[1].to_state, EntityLifecycleState::Suspended);
        assert_eq!(e.transitions[2].from_state, EntityLifecycleState::Suspended);
        assert_eq!(e.transitions[2].to_state, EntityLifecycleState::Active);
    }

    // ── Display tests ────────────────────────────────────────────────

    #[test]
    fn test_entity_state_display() {
        assert_eq!(EntityLifecycleState::Formation.to_string(), "FORMATION");
        assert_eq!(EntityLifecycleState::Active.to_string(), "ACTIVE");
        assert_eq!(EntityLifecycleState::Suspended.to_string(), "SUSPENDED");
        assert_eq!(
            EntityLifecycleState::Dissolution(DissolutionStage::TaxClearance).to_string(),
            "DISSOLUTION_STAGE_4"
        );
        assert_eq!(EntityLifecycleState::Dissolved.to_string(), "DISSOLVED");
    }

    #[test]
    fn test_dissolution_stage_display() {
        assert_eq!(
            DissolutionStage::BoardResolution.to_string(),
            "BOARD_RESOLUTION"
        );
        assert_eq!(
            DissolutionStage::Deregistration.to_string(),
            "DEREGISTRATION"
        );
    }

    // ── Serialization tests ──────────────────────────────────────────

    #[test]
    fn test_entity_serialization() {
        let e = make_active_entity();
        let json = serde_json::to_string(&e).unwrap();
        let parsed: Entity = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.state, e.state);
        assert_eq!(parsed.id, e.id);
    }
}
