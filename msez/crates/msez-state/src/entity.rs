//! # Entity Lifecycle State Machine
//!
//! Manages entity lifecycle from formation through the 10-stage dissolution
//! process. Entities represent companies, organizations, and individuals
//! registered within Special Economic Zones.
//!
//! ## Design Decision
//!
//! The 10-stage dissolution uses a validated enum with a transition function
//! that returns `Result` and rejects invalid transitions, rather than 10
//! typestate types. This is because:
//!
//! 1. Dissolution stages are linear (stage N → stage N+1), making typestate
//!    overhead unjustified — there are no branching transitions within dissolution.
//! 2. The dissolution stage must be persisted/deserialized from storage where
//!    the stage is a runtime value, not a compile-time type.
//! 3. The entity's top-level lifecycle (Applied → Active → Dissolving → Dissolved)
//!    does use validated transitions with `Result`.
//!
//! ## 10-Stage Voluntary Dissolution
//!
//! Per `modules/corporate/dissolution/workflows/voluntary-dissolution.yaml`:
//!
//! 1. Board Resolution (7 days)
//! 2. Shareholder Resolution (21 days, 75% supermajority)
//! 3. Appoint Liquidator (7 days)
//! 4. Notify Creditors (14 days gazette publication, 90 day claim deadline)
//! 5. Realize Assets (90 days)
//! 6. Settle Liabilities (60 days, statutory priority order)
//! 7. Final Distribution to Shareholders (14 days)
//! 8. Final General Meeting (35 days)
//! 9. File Final Documents (14 days)
//! 10. Dissolution (30 days, certificate issued)

use serde::{Deserialize, Serialize};
use thiserror::Error;

use msez_core::EntityId;

// ── Entity Lifecycle State ───────────────────────────────────────────

/// The lifecycle state of an entity within a jurisdiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityLifecycleState {
    /// Entity formation application submitted.
    Applied,
    /// Entity registered and active.
    Active,
    /// Entity operations temporarily suspended by regulator.
    Suspended,
    /// Dissolution process initiated (10 stages).
    Dissolving,
    /// Entity has been fully dissolved. Terminal state.
    Dissolved,
    /// Entity registration was rejected. Terminal state.
    Rejected,
}

impl EntityLifecycleState {
    /// Whether this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Dissolved | Self::Rejected)
    }

    /// The canonical string name of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Applied => "APPLIED",
            Self::Active => "ACTIVE",
            Self::Suspended => "SUSPENDED",
            Self::Dissolving => "DISSOLVING",
            Self::Dissolved => "DISSOLVED",
            Self::Rejected => "REJECTED",
        }
    }
}

impl std::fmt::Display for EntityLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Dissolution Stage ────────────────────────────────────────────────

/// The 10 stages of voluntary dissolution.
///
/// Each stage maps to a specific legal process with defined timelines
/// and evidence requirements. The stages are strictly sequential:
/// stage N must complete before stage N+1 can begin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum DissolutionStage {
    /// Stage 1: Board passes resolution to recommend dissolution.
    BoardResolution = 1,
    /// Stage 2: Shareholders pass special resolution (75% supermajority).
    ShareholderResolution = 2,
    /// Stage 3: Licensed insolvency practitioner appointed.
    AppointLiquidator = 3,
    /// Stage 4: Gazette notice published, creditors notified (90-day deadline).
    NotifyCreditors = 4,
    /// Stage 5: Liquidator realizes all company assets.
    RealizeAssets = 5,
    /// Stage 6: Pay creditors in statutory priority order.
    SettleLiabilities = 6,
    /// Stage 7: Distribute remaining assets to shareholders.
    FinalDistribution = 7,
    /// Stage 8: Liquidator presents final accounts at general meeting.
    FinalMeeting = 8,
    /// Stage 9: File final return and surrender licenses.
    FileFinalDocuments = 9,
    /// Stage 10: Registry processes dissolution, certificate issued.
    Dissolution = 10,
}

impl DissolutionStage {
    /// Return the next stage, or None if this is the final stage.
    pub fn next(&self) -> Option<DissolutionStage> {
        match self {
            Self::BoardResolution => Some(Self::ShareholderResolution),
            Self::ShareholderResolution => Some(Self::AppointLiquidator),
            Self::AppointLiquidator => Some(Self::NotifyCreditors),
            Self::NotifyCreditors => Some(Self::RealizeAssets),
            Self::RealizeAssets => Some(Self::SettleLiabilities),
            Self::SettleLiabilities => Some(Self::FinalDistribution),
            Self::FinalDistribution => Some(Self::FinalMeeting),
            Self::FinalMeeting => Some(Self::FileFinalDocuments),
            Self::FileFinalDocuments => Some(Self::Dissolution),
            Self::Dissolution => None,
        }
    }

    /// The stage number (1-10).
    pub fn number(&self) -> u8 {
        *self as u8
    }

    /// The canonical name of this stage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BoardResolution => "BOARD_RESOLUTION",
            Self::ShareholderResolution => "SHAREHOLDER_RESOLUTION",
            Self::AppointLiquidator => "APPOINT_LIQUIDATOR",
            Self::NotifyCreditors => "NOTIFY_CREDITORS",
            Self::RealizeAssets => "REALIZE_ASSETS",
            Self::SettleLiabilities => "SETTLE_LIABILITIES",
            Self::FinalDistribution => "FINAL_DISTRIBUTION",
            Self::FinalMeeting => "FINAL_MEETING",
            Self::FileFinalDocuments => "FILE_FINAL_DOCUMENTS",
            Self::Dissolution => "DISSOLUTION",
        }
    }

    /// Whether this is the final stage.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Dissolution)
    }

    /// All 10 stages in order.
    pub fn all_stages() -> &'static [DissolutionStage] {
        &[
            Self::BoardResolution,
            Self::ShareholderResolution,
            Self::AppointLiquidator,
            Self::NotifyCreditors,
            Self::RealizeAssets,
            Self::SettleLiabilities,
            Self::FinalDistribution,
            Self::FinalMeeting,
            Self::FileFinalDocuments,
            Self::Dissolution,
        ]
    }
}

impl std::fmt::Display for DissolutionStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Stage {} ({})", self.number(), self.as_str())
    }
}

// ── Entity Error ─────────────────────────────────────────────────────

/// Errors during entity lifecycle operations.
#[derive(Error, Debug)]
pub enum EntityError {
    /// Invalid lifecycle state transition.
    #[error("invalid entity transition from {from} to {to}: {reason}")]
    InvalidTransition {
        /// Current state.
        from: EntityLifecycleState,
        /// Attempted target state.
        to: EntityLifecycleState,
        /// Human-readable reason.
        reason: String,
    },
    /// Invalid dissolution stage transition.
    #[error("invalid dissolution stage advance from {from} (entity must be in DISSOLVING state, currently: {entity_state})")]
    InvalidDissolutionAdvance {
        /// Current dissolution stage.
        from: DissolutionStage,
        /// Current entity lifecycle state.
        entity_state: EntityLifecycleState,
    },
    /// Entity is already in a terminal state.
    #[error("entity {id} is in terminal state {state}")]
    AlreadyTerminal {
        /// Entity identifier.
        id: EntityId,
        /// The terminal state.
        state: EntityLifecycleState,
    },
    /// Dissolution already complete (stage 10 reached).
    #[error("dissolution already complete at stage 10")]
    DissolutionComplete,
}

// ── Entity ───────────────────────────────────────────────────────────

/// An entity within the SEZ lifecycle system.
///
/// Tracks both the high-level lifecycle state and, when dissolving,
/// the current dissolution stage (1-10).
#[derive(Debug)]
pub struct Entity {
    /// Unique entity identifier.
    pub id: EntityId,
    /// The current lifecycle state.
    pub state: EntityLifecycleState,
    /// The current dissolution stage (1-10), if dissolving.
    pub dissolution_stage: Option<DissolutionStage>,
}

impl Entity {
    /// Create a new entity in the Applied state.
    pub fn new(id: EntityId) -> Self {
        Self {
            id,
            state: EntityLifecycleState::Applied,
            dissolution_stage: None,
        }
    }

    /// Approve the entity application.
    ///
    /// Transitions: Applied → Active.
    pub fn approve(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Applied {
            return Err(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Active,
                reason: "can only approve from APPLIED state".to_string(),
            });
        }
        self.state = EntityLifecycleState::Active;
        Ok(())
    }

    /// Reject the entity application. Terminal.
    ///
    /// Transitions: Applied → Rejected.
    pub fn reject(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Applied {
            return Err(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Rejected,
                reason: "can only reject from APPLIED state".to_string(),
            });
        }
        self.state = EntityLifecycleState::Rejected;
        Ok(())
    }

    /// Suspend entity operations.
    ///
    /// Transitions: Active → Suspended.
    pub fn suspend(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Active {
            return Err(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Suspended,
                reason: "can only suspend from ACTIVE state".to_string(),
            });
        }
        self.state = EntityLifecycleState::Suspended;
        Ok(())
    }

    /// Reinstate a suspended entity.
    ///
    /// Transitions: Suspended → Active.
    pub fn reinstate(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Suspended {
            return Err(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Active,
                reason: "can only reinstate from SUSPENDED state".to_string(),
            });
        }
        self.state = EntityLifecycleState::Active;
        Ok(())
    }

    /// Initiate the dissolution process (10-stage workflow).
    ///
    /// Transitions: Active → Dissolving, sets dissolution stage to 1 (Board Resolution).
    pub fn initiate_dissolution(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Active {
            return Err(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Dissolving,
                reason: "can only initiate dissolution from ACTIVE state".to_string(),
            });
        }
        self.state = EntityLifecycleState::Dissolving;
        self.dissolution_stage = Some(DissolutionStage::BoardResolution);
        Ok(())
    }

    /// Advance to the next dissolution stage.
    ///
    /// Must be in `Dissolving` state. Advances from stage N to N+1.
    /// After stage 10 completes, transitions to `Dissolved` (terminal).
    pub fn advance_dissolution(&mut self) -> Result<(), EntityError> {
        if self.state != EntityLifecycleState::Dissolving {
            let stage = self
                .dissolution_stage
                .unwrap_or(DissolutionStage::BoardResolution);
            return Err(EntityError::InvalidDissolutionAdvance {
                from: stage,
                entity_state: self.state,
            });
        }

        let current = self
            .dissolution_stage
            .ok_or(EntityError::InvalidTransition {
                from: self.state,
                to: EntityLifecycleState::Dissolving,
                reason: "dissolving entity has no dissolution stage set".to_string(),
            })?;

        match current.next() {
            Some(next_stage) => {
                self.dissolution_stage = Some(next_stage);
                Ok(())
            }
            None => {
                // Stage 10 completed → entity is dissolved.
                self.state = EntityLifecycleState::Dissolved;
                Ok(())
            }
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new(EntityId::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_entity() -> Entity {
        Entity::new(EntityId::new())
    }

    #[test]
    fn new_entity_is_applied() {
        let entity = test_entity();
        assert_eq!(entity.state, EntityLifecycleState::Applied);
        assert!(entity.dissolution_stage.is_none());
    }

    #[test]
    fn applied_to_active() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        assert_eq!(entity.state, EntityLifecycleState::Active);
    }

    #[test]
    fn applied_to_rejected() {
        let mut entity = test_entity();
        entity.reject().unwrap();
        assert_eq!(entity.state, EntityLifecycleState::Rejected);
        assert!(entity.state.is_terminal());
    }

    #[test]
    fn active_to_suspended_and_back() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        entity.suspend().unwrap();
        assert_eq!(entity.state, EntityLifecycleState::Suspended);

        entity.reinstate().unwrap();
        assert_eq!(entity.state, EntityLifecycleState::Active);
    }

    #[test]
    fn cannot_approve_from_active() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        let err = entity.approve().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_suspend_from_applied() {
        let mut entity = test_entity();
        let err = entity.suspend().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn dissolution_10_stages() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        entity.initiate_dissolution().unwrap();

        assert_eq!(entity.state, EntityLifecycleState::Dissolving);
        assert_eq!(
            entity.dissolution_stage,
            Some(DissolutionStage::BoardResolution)
        );

        // Advance through all 10 stages.
        let expected_stages = [
            DissolutionStage::ShareholderResolution,
            DissolutionStage::AppointLiquidator,
            DissolutionStage::NotifyCreditors,
            DissolutionStage::RealizeAssets,
            DissolutionStage::SettleLiabilities,
            DissolutionStage::FinalDistribution,
            DissolutionStage::FinalMeeting,
            DissolutionStage::FileFinalDocuments,
            DissolutionStage::Dissolution,
        ];

        for expected in &expected_stages {
            entity.advance_dissolution().unwrap();
            if *expected != DissolutionStage::Dissolution {
                assert_eq!(entity.dissolution_stage, Some(*expected));
                assert_eq!(entity.state, EntityLifecycleState::Dissolving);
            }
        }

        // After stage 10 advance, entity should be dissolved.
        entity.advance_dissolution().unwrap();
        assert_eq!(entity.state, EntityLifecycleState::Dissolved);
        assert!(entity.state.is_terminal());
    }

    #[test]
    fn dissolution_stage_count() {
        assert_eq!(DissolutionStage::all_stages().len(), 10);
    }

    #[test]
    fn dissolution_stage_numbers() {
        for (i, stage) in DissolutionStage::all_stages().iter().enumerate() {
            assert_eq!(stage.number(), (i + 1) as u8);
        }
    }

    #[test]
    fn dissolution_stage_sequence() {
        let mut stage = DissolutionStage::BoardResolution;
        let mut count = 1;
        while let Some(next) = stage.next() {
            stage = next;
            count += 1;
        }
        assert_eq!(count, 10);
        assert!(stage.is_final());
    }

    #[test]
    fn cannot_dissolve_from_applied() {
        let mut entity = test_entity();
        let err = entity.initiate_dissolution().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_advance_dissolution_when_not_dissolving() {
        let mut entity = test_entity();
        entity.approve().unwrap();

        let err = entity.advance_dissolution().unwrap_err();
        assert!(matches!(err, EntityError::InvalidDissolutionAdvance { .. }));
    }

    #[test]
    fn state_display_names() {
        assert_eq!(EntityLifecycleState::Applied.as_str(), "APPLIED");
        assert_eq!(EntityLifecycleState::Active.as_str(), "ACTIVE");
        assert_eq!(EntityLifecycleState::Dissolving.as_str(), "DISSOLVING");
        assert_eq!(EntityLifecycleState::Dissolved.as_str(), "DISSOLVED");
    }

    #[test]
    fn dissolution_stage_display() {
        let stage = DissolutionStage::NotifyCreditors;
        assert_eq!(format!("{stage}"), "Stage 4 (NOTIFY_CREDITORS)");
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn entity_lifecycle_state_display_all_variants() {
        assert_eq!(format!("{}", EntityLifecycleState::Applied), "APPLIED");
        assert_eq!(format!("{}", EntityLifecycleState::Active), "ACTIVE");
        assert_eq!(format!("{}", EntityLifecycleState::Suspended), "SUSPENDED");
        assert_eq!(format!("{}", EntityLifecycleState::Dissolving), "DISSOLVING");
        assert_eq!(format!("{}", EntityLifecycleState::Dissolved), "DISSOLVED");
        assert_eq!(format!("{}", EntityLifecycleState::Rejected), "REJECTED");
    }

    #[test]
    fn entity_lifecycle_state_as_str_all_variants() {
        assert_eq!(EntityLifecycleState::Suspended.as_str(), "SUSPENDED");
        assert_eq!(EntityLifecycleState::Rejected.as_str(), "REJECTED");
    }

    #[test]
    fn is_terminal_non_terminal_states() {
        assert!(!EntityLifecycleState::Applied.is_terminal());
        assert!(!EntityLifecycleState::Active.is_terminal());
        assert!(!EntityLifecycleState::Suspended.is_terminal());
        assert!(!EntityLifecycleState::Dissolving.is_terminal());
    }

    #[test]
    fn is_terminal_terminal_states() {
        assert!(EntityLifecycleState::Dissolved.is_terminal());
        assert!(EntityLifecycleState::Rejected.is_terminal());
    }

    #[test]
    fn dissolution_stage_all_stages_returns_ten_ordered() {
        let stages = DissolutionStage::all_stages();
        assert_eq!(stages.len(), 10);
        assert_eq!(stages[0], DissolutionStage::BoardResolution);
        assert_eq!(stages[1], DissolutionStage::ShareholderResolution);
        assert_eq!(stages[2], DissolutionStage::AppointLiquidator);
        assert_eq!(stages[3], DissolutionStage::NotifyCreditors);
        assert_eq!(stages[4], DissolutionStage::RealizeAssets);
        assert_eq!(stages[5], DissolutionStage::SettleLiabilities);
        assert_eq!(stages[6], DissolutionStage::FinalDistribution);
        assert_eq!(stages[7], DissolutionStage::FinalMeeting);
        assert_eq!(stages[8], DissolutionStage::FileFinalDocuments);
        assert_eq!(stages[9], DissolutionStage::Dissolution);
    }

    #[test]
    fn dissolution_stage_number_all_variants() {
        assert_eq!(DissolutionStage::BoardResolution.number(), 1);
        assert_eq!(DissolutionStage::ShareholderResolution.number(), 2);
        assert_eq!(DissolutionStage::AppointLiquidator.number(), 3);
        assert_eq!(DissolutionStage::NotifyCreditors.number(), 4);
        assert_eq!(DissolutionStage::RealizeAssets.number(), 5);
        assert_eq!(DissolutionStage::SettleLiabilities.number(), 6);
        assert_eq!(DissolutionStage::FinalDistribution.number(), 7);
        assert_eq!(DissolutionStage::FinalMeeting.number(), 8);
        assert_eq!(DissolutionStage::FileFinalDocuments.number(), 9);
        assert_eq!(DissolutionStage::Dissolution.number(), 10);
    }

    #[test]
    fn dissolution_stage_as_str_all_variants() {
        assert_eq!(DissolutionStage::BoardResolution.as_str(), "BOARD_RESOLUTION");
        assert_eq!(DissolutionStage::ShareholderResolution.as_str(), "SHAREHOLDER_RESOLUTION");
        assert_eq!(DissolutionStage::AppointLiquidator.as_str(), "APPOINT_LIQUIDATOR");
        assert_eq!(DissolutionStage::NotifyCreditors.as_str(), "NOTIFY_CREDITORS");
        assert_eq!(DissolutionStage::RealizeAssets.as_str(), "REALIZE_ASSETS");
        assert_eq!(DissolutionStage::SettleLiabilities.as_str(), "SETTLE_LIABILITIES");
        assert_eq!(DissolutionStage::FinalDistribution.as_str(), "FINAL_DISTRIBUTION");
        assert_eq!(DissolutionStage::FinalMeeting.as_str(), "FINAL_MEETING");
        assert_eq!(DissolutionStage::FileFinalDocuments.as_str(), "FILE_FINAL_DOCUMENTS");
        assert_eq!(DissolutionStage::Dissolution.as_str(), "DISSOLUTION");
    }

    #[test]
    fn dissolution_stage_display_all_variants() {
        assert_eq!(format!("{}", DissolutionStage::BoardResolution), "Stage 1 (BOARD_RESOLUTION)");
        assert_eq!(format!("{}", DissolutionStage::ShareholderResolution), "Stage 2 (SHAREHOLDER_RESOLUTION)");
        assert_eq!(format!("{}", DissolutionStage::AppointLiquidator), "Stage 3 (APPOINT_LIQUIDATOR)");
        assert_eq!(format!("{}", DissolutionStage::RealizeAssets), "Stage 5 (REALIZE_ASSETS)");
        assert_eq!(format!("{}", DissolutionStage::SettleLiabilities), "Stage 6 (SETTLE_LIABILITIES)");
        assert_eq!(format!("{}", DissolutionStage::FinalDistribution), "Stage 7 (FINAL_DISTRIBUTION)");
        assert_eq!(format!("{}", DissolutionStage::FinalMeeting), "Stage 8 (FINAL_MEETING)");
        assert_eq!(format!("{}", DissolutionStage::FileFinalDocuments), "Stage 9 (FILE_FINAL_DOCUMENTS)");
        assert_eq!(format!("{}", DissolutionStage::Dissolution), "Stage 10 (DISSOLUTION)");
    }

    #[test]
    fn dissolution_stage_is_final_non_final() {
        assert!(!DissolutionStage::BoardResolution.is_final());
        assert!(!DissolutionStage::ShareholderResolution.is_final());
        assert!(!DissolutionStage::AppointLiquidator.is_final());
        assert!(!DissolutionStage::NotifyCreditors.is_final());
        assert!(!DissolutionStage::RealizeAssets.is_final());
        assert!(!DissolutionStage::SettleLiabilities.is_final());
        assert!(!DissolutionStage::FinalDistribution.is_final());
        assert!(!DissolutionStage::FinalMeeting.is_final());
        assert!(!DissolutionStage::FileFinalDocuments.is_final());
        assert!(DissolutionStage::Dissolution.is_final());
    }

    #[test]
    fn dissolution_stage_next_returns_none_for_final() {
        assert!(DissolutionStage::Dissolution.next().is_none());
    }

    #[test]
    fn cannot_reject_from_active() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        let err = entity.reject().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reinstate_from_applied() {
        let mut entity = test_entity();
        let err = entity.reinstate().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_reinstate_from_active() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        let err = entity.reinstate().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_initiate_dissolution_from_suspended() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        entity.suspend().unwrap();
        let err = entity.initiate_dissolution().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_approve_from_rejected() {
        let mut entity = test_entity();
        entity.reject().unwrap();
        let err = entity.approve().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_approve_from_dissolved() {
        let mut entity = test_entity();
        entity.approve().unwrap();
        entity.initiate_dissolution().unwrap();
        // Advance through all 10 stages + final advance to dissolved
        for _ in 0..10 {
            entity.advance_dissolution().unwrap();
        }
        assert_eq!(entity.state, EntityLifecycleState::Dissolved);
        let err = entity.approve().unwrap_err();
        assert!(matches!(err, EntityError::InvalidTransition { .. }));
    }

    #[test]
    fn cannot_advance_dissolution_from_rejected() {
        let mut entity = test_entity();
        entity.reject().unwrap();
        let err = entity.advance_dissolution().unwrap_err();
        assert!(matches!(err, EntityError::InvalidDissolutionAdvance { .. }));
    }

    #[test]
    fn entity_default_is_applied() {
        let entity = Entity::default();
        assert_eq!(entity.state, EntityLifecycleState::Applied);
        assert!(entity.dissolution_stage.is_none());
    }

    #[test]
    fn entity_error_invalid_transition_display() {
        let err = EntityError::InvalidTransition {
            from: EntityLifecycleState::Applied,
            to: EntityLifecycleState::Suspended,
            reason: "not allowed".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("APPLIED"));
        assert!(msg.contains("SUSPENDED"));
        assert!(msg.contains("not allowed"));
    }

    #[test]
    fn entity_error_invalid_dissolution_advance_display() {
        let err = EntityError::InvalidDissolutionAdvance {
            from: DissolutionStage::BoardResolution,
            entity_state: EntityLifecycleState::Active,
        };
        let msg = format!("{err}");
        assert!(msg.contains("ACTIVE"));
    }

    #[test]
    fn entity_error_already_terminal_display() {
        let err = EntityError::AlreadyTerminal {
            id: EntityId::new(),
            state: EntityLifecycleState::Dissolved,
        };
        let msg = format!("{err}");
        assert!(msg.contains("terminal state"));
        assert!(msg.contains("DISSOLVED"));
    }

    #[test]
    fn entity_error_dissolution_complete_display() {
        let err = EntityError::DissolutionComplete;
        let msg = format!("{err}");
        assert!(msg.contains("dissolution already complete"));
    }
}
