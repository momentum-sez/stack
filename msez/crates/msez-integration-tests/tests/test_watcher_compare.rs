//! # Watcher State Comparison and Bonding Test
//!
//! Tests the watcher lifecycle state machine: bonding, activation,
//! slashing across all 4 conditions, rebonding, and unbonding.
//! Verifies that state transitions are correctly enforced and that
//! slashing reduces the watcher's available stake as specified.

use msez_core::WatcherId;
use msez_state::{SlashingCondition, Watcher, WatcherState};

fn active_watcher(stake: u64) -> Watcher {
    let mut w = Watcher::new(WatcherId::new());
    w.bond(stake).unwrap();
    w.activate().unwrap();
    w
}

// ---------------------------------------------------------------------------
// 1. Bonding and activation lifecycle
// ---------------------------------------------------------------------------

#[test]
fn watcher_bonding_and_activation() {
    let mut w = Watcher::new(WatcherId::new());
    assert_eq!(w.state, WatcherState::Registered);
    assert_eq!(w.bonded_stake, 0);

    w.bond(1_000_000).unwrap();
    assert_eq!(w.state, WatcherState::Bonded);
    assert_eq!(w.bonded_stake, 1_000_000);

    w.activate().unwrap();
    assert_eq!(w.state, WatcherState::Active);
    assert_eq!(w.available_stake(), 1_000_000);
}

// ---------------------------------------------------------------------------
// 2. Slashing reduces bond
// ---------------------------------------------------------------------------

#[test]
fn watcher_slashing_reduces_bond() {
    let mut w = active_watcher(1_000_000);

    // 1% slash for availability failure
    let slashed = w.slash(SlashingCondition::AvailabilityFailure).unwrap();
    assert_eq!(slashed, 10_000);
    assert_eq!(w.available_stake(), 990_000);
    assert_eq!(w.slash_count, 1);
    assert_eq!(w.state, WatcherState::Slashed);
}

// ---------------------------------------------------------------------------
// 3. All 4 slashing conditions
// ---------------------------------------------------------------------------

#[test]
fn watcher_slashing_conditions_all_four() {
    // Equivocation: 100%
    let mut w1 = active_watcher(1_000_000);
    let slashed = w1.slash(SlashingCondition::Equivocation).unwrap();
    assert_eq!(slashed, 1_000_000);
    assert_eq!(w1.available_stake(), 0);
    assert_eq!(w1.state, WatcherState::Slashed);

    // Availability Failure: 1%
    let mut w2 = active_watcher(1_000_000);
    let slashed = w2.slash(SlashingCondition::AvailabilityFailure).unwrap();
    assert_eq!(slashed, 10_000);

    // False Attestation: 50%
    let mut w3 = active_watcher(1_000_000);
    let slashed = w3.slash(SlashingCondition::FalseAttestation).unwrap();
    assert_eq!(slashed, 500_000);
    assert_eq!(w3.available_stake(), 500_000);

    // Collusion: 100% + permanent ban
    let mut w4 = active_watcher(1_000_000);
    let slashed = w4.slash(SlashingCondition::Collusion).unwrap();
    assert_eq!(slashed, 1_000_000);
    assert_eq!(w4.state, WatcherState::Banned);
    assert!(w4.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 4. State transitions
// ---------------------------------------------------------------------------

#[test]
fn watcher_state_transitions() {
    let mut w = Watcher::new(WatcherId::new());
    assert_eq!(w.state, WatcherState::Registered);

    w.bond(500_000).unwrap();
    assert_eq!(w.state, WatcherState::Bonded);

    w.activate().unwrap();
    assert_eq!(w.state, WatcherState::Active);

    w.unbond().unwrap();
    assert_eq!(w.state, WatcherState::Unbonding);

    let returned = w.complete_unbond().unwrap();
    assert_eq!(returned, 500_000);
    assert_eq!(w.state, WatcherState::Deactivated);
    assert!(w.state.is_terminal());
}

// ---------------------------------------------------------------------------
// 5. Cannot slash inactive watcher
// ---------------------------------------------------------------------------

#[test]
fn watcher_cannot_slash_inactive() {
    // Cannot slash from Registered
    let mut w1 = Watcher::new(WatcherId::new());
    assert!(w1.slash(SlashingCondition::Equivocation).is_err());

    // Cannot slash from Bonded
    let mut w2 = Watcher::new(WatcherId::new());
    w2.bond(1_000_000).unwrap();
    assert!(w2.slash(SlashingCondition::Equivocation).is_err());

    // Cannot slash from Unbonding
    let mut w3 = active_watcher(1_000_000);
    w3.unbond().unwrap();
    assert!(w3.slash(SlashingCondition::Equivocation).is_err());
}

// ---------------------------------------------------------------------------
// 6. Rebond after slash
// ---------------------------------------------------------------------------

#[test]
fn watcher_rebond_after_slash() {
    let mut w = active_watcher(1_000_000);
    w.slash(SlashingCondition::AvailabilityFailure).unwrap();
    assert_eq!(w.state, WatcherState::Slashed);

    w.rebond(50_000).unwrap();
    assert_eq!(w.state, WatcherState::Bonded);
    assert_eq!(w.bonded_stake, 1_050_000);
}
