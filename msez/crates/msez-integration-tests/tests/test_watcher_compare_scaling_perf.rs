//! Rust counterpart of tests/perf/test_watcher_compare_scaling_perf.py
//! Performance tests for watcher operations at scale.

use msez_core::WatcherId;
use msez_state::{SlashingCondition, Watcher, WatcherState};

#[test]
fn create_many_watchers() {
    let watchers: Vec<Watcher> = (0..50).map(|_| Watcher::new(WatcherId::new())).collect();
    assert_eq!(watchers.len(), 50);
    for w in &watchers {
        assert_eq!(w.state, WatcherState::Registered);
    }
}

#[test]
fn activate_and_slash_at_scale() {
    let mut watchers: Vec<Watcher> = (0..20)
        .map(|i| {
            let mut w = Watcher::new(WatcherId::new());
            w.bond(100_000 * (i + 1)).unwrap();
            w
        })
        .collect();
    for w in &mut watchers {
        w.activate().unwrap();
    }
    let conditions = [
        SlashingCondition::Equivocation,
        SlashingCondition::AvailabilityFailure,
        SlashingCondition::FalseAttestation,
        SlashingCondition::Collusion,
    ];
    for (i, w) in watchers.iter_mut().enumerate().take(8) {
        w.slash(conditions[i % 4]).unwrap();
        // Equivocation -> Slashed, AvailabilityFailure -> Slashed,
        // FalseAttestation -> Slashed, Collusion -> Banned
        if conditions[i % 4] == SlashingCondition::Collusion {
            assert_eq!(w.state, WatcherState::Banned);
        } else {
            assert_eq!(w.state, WatcherState::Slashed);
        }
    }
    for w in watchers.iter().skip(8) {
        assert_eq!(w.state, WatcherState::Active);
    }
}

#[test]
fn watcher_state_transitions_at_scale() {
    let mut watchers: Vec<Watcher> = (0..10)
        .map(|_| {
            let mut w = Watcher::new(WatcherId::new());
            w.bond(500_000).unwrap();
            w
        })
        .collect();
    for w in &mut watchers {
        w.activate().unwrap();
    }
    for w in &mut watchers[..5] {
        w.unbond().unwrap();
        assert_eq!(w.state, WatcherState::Unbonding);
    }
    for w in &mut watchers[..5] {
        w.complete_unbond().unwrap();
        assert_eq!(w.state, WatcherState::Deactivated);
    }
    for w in &watchers[5..] {
        assert_eq!(w.state, WatcherState::Active);
    }
}
