//! # Watcher Bonding State Machine
//!
//! Manages the lifecycle of watcher nodes in the corridor economy.
//! Watchers post bonds, observe corridor activity, and can be slashed
//! for 4 defined conditions.

use serde::{Deserialize, Serialize};

/// The lifecycle state of a watcher node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WatcherState {
    /// Watcher has registered but not yet bonded.
    Registered,
    /// Bond has been posted; watcher is active.
    Bonded,
    /// Watcher is actively monitoring corridor activity.
    Active,
    /// Watcher has been slashed for a protocol violation.
    Slashed,
    /// Watcher is in the unbonding period.
    Unbonding,
    /// Bond has been returned; watcher is deactivated. Terminal state.
    Deactivated,
}

/// The 4 slashing conditions for watcher nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SlashingCondition {
    /// Watcher signed conflicting attestations.
    DoubleAttestation,
    /// Watcher failed to attest within the required window.
    InactivityTimeout,
    /// Watcher attested to invalid data.
    InvalidAttestation,
    /// Watcher colluded with other watchers (detected via quorum analysis).
    CollusionDetected,
}

/// A watcher node in the corridor economy.
#[derive(Debug)]
pub struct Watcher {
    /// Current watcher lifecycle state.
    pub state: WatcherState,
}
