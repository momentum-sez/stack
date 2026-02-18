//! Rust counterpart of tests/integration/test_watcher_attestation_flow.py
//! Integration test for watcher attestation flow.

use mez_core::{sha256_digest, CanonicalBytes, WatcherId};
use mez_state::{SlashingCondition, Watcher, WatcherState};
use serde_json::json;

#[test]
fn watcher_activation_flow() {
    let mut watcher = Watcher::new(WatcherId::new());
    assert_eq!(watcher.state, WatcherState::Registered);
    watcher.bond(1_000_000).unwrap();
    assert_eq!(watcher.state, WatcherState::Bonded);
    watcher.activate().unwrap();
    assert_eq!(watcher.state, WatcherState::Active);
}

#[test]
fn watcher_attestation_produces_evidence() {
    let wid = WatcherId::new();
    let attestation = json!({"watcher_id": wid.to_string(), "corridor_id": "c-001", "receipt_seq": 42, "mmr_root": "ab".repeat(32)});
    let cb = CanonicalBytes::new(&attestation).unwrap();
    let digest = sha256_digest(&cb);
    assert_eq!(digest.to_hex().len(), 64);
}

#[test]
fn watcher_slashing_equivocation() {
    let mut watcher = Watcher::new(WatcherId::new());
    watcher.bond(1_000_000).unwrap();
    watcher.activate().unwrap();
    let slashed = watcher.slash(SlashingCondition::Equivocation).unwrap();
    assert_eq!(slashed, 1_000_000);
    assert_eq!(watcher.state, WatcherState::Slashed);
}

#[test]
fn watcher_unbonding_and_deactivation() {
    let mut watcher = Watcher::new(WatcherId::new());
    watcher.bond(1_000_000).unwrap();
    watcher.activate().unwrap();
    watcher.unbond().unwrap();
    assert_eq!(watcher.state, WatcherState::Unbonding);
    let returned = watcher.complete_unbond().unwrap();
    assert_eq!(returned, 1_000_000);
    assert_eq!(watcher.state, WatcherState::Deactivated);
}

#[test]
fn multiple_watchers_independent() {
    let mut w1 = Watcher::new(WatcherId::new());
    let mut w2 = Watcher::new(WatcherId::new());
    w1.bond(500_000).unwrap();
    w1.activate().unwrap();
    w2.bond(750_000).unwrap();
    w2.activate().unwrap();
    w1.slash(SlashingCondition::FalseAttestation).unwrap();
    assert_eq!(w1.state, WatcherState::Slashed);
    assert_eq!(w2.state, WatcherState::Active);
}
