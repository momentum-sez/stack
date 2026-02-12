//! # Entity Lifecycle State Machine
//!
//! Models the lifecycle of legal entities (companies, SPVs, trusts) within
//! a jurisdiction, including the 10-stage dissolution process.
//!
//! ## States
//!
//! FORMATION → ACTIVE → SUSPENDED → DISSOLUTION(stages 1-10) → DISSOLVED
//!
//! ## Implements
//!
//! Spec §5 — Entity lifecycle and dissolution protocol.

use msez_core::EntityId;

/// The lifecycle state of an entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityLifecycleState {
    /// Entity is being formed (initial registration).
    Formation,
    /// Entity is active and operational.
    Active,
    /// Entity is temporarily suspended.
    Suspended,
    /// Entity is undergoing dissolution (10 stages).
    Dissolution(u8),
    /// Entity has been fully dissolved (terminal).
    Dissolved,
}

impl EntityLifecycleState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Dissolved)
    }
}

impl std::fmt::Display for EntityLifecycleState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Formation => write!(f, "FORMATION"),
            Self::Active => write!(f, "ACTIVE"),
            Self::Suspended => write!(f, "SUSPENDED"),
            Self::Dissolution(stage) => write!(f, "DISSOLUTION_STAGE_{stage}"),
            Self::Dissolved => write!(f, "DISSOLVED"),
        }
    }
}

/// An entity with its lifecycle state.
///
/// Placeholder — full implementation will enforce state transitions
/// with evidence requirements at each stage.
#[derive(Debug)]
pub struct Entity {
    /// Unique entity identifier.
    pub id: EntityId,
    /// Current lifecycle state.
    pub state: EntityLifecycleState,
}
