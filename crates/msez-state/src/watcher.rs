//! # Watcher Bonding and Slashing State Machine
//!
//! Models the lifecycle of corridor watcher nodes, including bond
//! management and the 4 slashing conditions.
//!
//! ## States
//!
//! UNBONDED → BONDED → ACTIVE → SLASHED | UNBONDING → UNBONDED
//!
//! ## Slashing Conditions
//!
//! 1. Equivocation — signing conflicting attestations.
//! 2. Unavailability — failing to attest within the required window.
//! 3. Invalid attestation — attesting to an invalid state transition.
//! 4. Collusion — coordinated misbehavior with other watchers.
//!
//! ## Implements
//!
//! Spec §17 — Watcher economy and slashing protocol.

use msez_core::WatcherId;

/// The state of a watcher node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WatcherState {
    /// Watcher has not posted a bond.
    Unbonded,
    /// Watcher has posted a bond but is not yet active.
    Bonded,
    /// Watcher is actively monitoring and attesting.
    Active,
    /// Watcher has been slashed for misbehavior (terminal).
    Slashed,
    /// Watcher is unbonding (cooldown period).
    Unbonding,
}

impl WatcherState {
    /// Whether this state is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Slashed)
    }
}

impl std::fmt::Display for WatcherState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Unbonded => "UNBONDED",
            Self::Bonded => "BONDED",
            Self::Active => "ACTIVE",
            Self::Slashed => "SLASHED",
            Self::Unbonding => "UNBONDING",
        };
        f.write_str(s)
    }
}

/// A watcher node with its state.
///
/// Placeholder — full implementation will include bond amount,
/// attestation history, and slashing evidence.
#[derive(Debug)]
pub struct Watcher {
    /// Unique watcher identifier.
    pub id: WatcherId,
    /// Current watcher state.
    pub state: WatcherState,
}
