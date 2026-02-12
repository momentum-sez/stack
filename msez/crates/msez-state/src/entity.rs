//! # Entity Lifecycle State Machine
//!
//! Manages entity lifecycle from formation through the 10-stage dissolution
//! process. Entities represent companies, organizations, and individuals
//! registered within Special Economic Zones.

use serde::{Deserialize, Serialize};

/// The lifecycle state of an entity within a jurisdiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntityLifecycleState {
    /// Entity formation application submitted.
    Applied,
    /// Entity registered and active.
    Active,
    /// Entity operations temporarily suspended.
    Suspended,
    /// Dissolution process initiated (10 stages).
    Dissolving,
    /// Entity has been fully dissolved. Terminal state.
    Dissolved,
    /// Entity registration was rejected. Terminal state.
    Rejected,
}

/// An entity within the SEZ lifecycle system.
#[derive(Debug)]
pub struct Entity {
    /// The current lifecycle state.
    pub state: EntityLifecycleState,
    /// The current dissolution stage (1-10), if dissolving.
    pub dissolution_stage: Option<u8>,
}

impl Entity {
    /// Create a new entity in the Applied state.
    pub fn new() -> Self {
        Self {
            state: EntityLifecycleState::Applied,
            dissolution_stage: None,
        }
    }
}

impl Default for Entity {
    fn default() -> Self {
        Self::new()
    }
}
