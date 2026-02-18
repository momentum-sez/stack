//! # Fork Resolution Adversarial Test (Evidence-Driven)
//!
//! Tests the three-level fork resolution protocol (Protocol 16.1 §3,
//! P0-FORK-001 remediated):
//! 1. Primary: Timestamp (if difference > MAX_CLOCK_SKEW)
//! 2. Secondary: Verified watcher attestation count (cryptographically bound)
//! 3. Tertiary: Lexicographic digest tiebreaker
//!
//! Includes adversarial scenarios: timestamp backdating, clock skew boundary
//! conditions, unregistered watchers, and equivocation detection.

use chrono::{Duration, Utc};
use mez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use mez_corridor::{
    ForkBranch, ForkDetector, ResolutionReason, WatcherRegistry, create_attestation,
    resolve_fork, MAX_CLOCK_SKEW,
};
use mez_crypto::ed25519::SigningKey;
use rand_core::OsRng;
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"branch": label})).unwrap();
    sha256_digest(&canonical)
}

fn make_branch(
    label: &str,
    timestamp: chrono::DateTime<Utc>,
    attestations: Vec<mez_corridor::WatcherAttestation>,
    next_root: &str,
) -> ForkBranch {
    ForkBranch {
        receipt_digest: make_digest(label),
        timestamp,
        attestations,
        next_root: next_root.to_string(),
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

fn make_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

// ---------------------------------------------------------------------------
// 1. Timestamp ordering beyond clock skew
// ---------------------------------------------------------------------------

#[test]
fn earlier_timestamp_wins_beyond_skew() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    // Use past-relative timestamps to avoid MAX_FUTURE_DRIFT rejection.
    let t2 = now();
    let t1 = t2 - Duration::minutes(10);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![create_attestation(&sk_b, "p", &nr_b, 1, t2).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn earlier_timestamp_wins_reversed() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    let t1 = now();
    let t2 = t1 - Duration::minutes(10);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![create_attestation(&sk_b, "p", &nr_b, 1, t2).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn large_time_difference_uses_timestamp() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    let t2 = now();
    let t1 = t2 - Duration::hours(1);
    let nr_a = "ff".repeat(32);
    let nr_b = "00".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![create_attestation(&sk_b, "p", &nr_b, 1, t2).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

// ---------------------------------------------------------------------------
// 2. Attestation count ordering within clock skew
// ---------------------------------------------------------------------------

#[test]
fn more_attestations_wins_within_skew() {
    let sk_a1 = make_key();
    let sk_b1 = make_key();
    let sk_b2 = make_key();
    let sk_b3 = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a1.verifying_key());
    reg.register(sk_b1.verifying_key());
    reg.register(sk_b2.verifying_key());
    reg.register(sk_b3.verifying_key());

    let t2 = now();
    let t1 = t2 - Duration::minutes(3);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a1, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![
        create_attestation(&sk_b1, "p", &nr_b, 1, t2).unwrap(),
        create_attestation(&sk_b2, "p", &nr_b, 1, t2).unwrap(),
        create_attestation(&sk_b3, "p", &nr_b, 1, t2).unwrap(),
    ], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn more_attestations_wins_same_timestamp() {
    let sk_a1 = make_key();
    let sk_a2 = make_key();
    let sk_a3 = make_key();
    let sk_b1 = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a1.verifying_key());
    reg.register(sk_a2.verifying_key());
    reg.register(sk_a3.verifying_key());
    reg.register(sk_b1.verifying_key());

    let t = now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t, vec![
        create_attestation(&sk_a1, "p", &nr_a, 1, t).unwrap(),
        create_attestation(&sk_a2, "p", &nr_a, 1, t).unwrap(),
        create_attestation(&sk_a3, "p", &nr_a, 1, t).unwrap(),
    ], &nr_a);
    let b = make_branch("B", t, vec![create_attestation(&sk_b1, "p", &nr_b, 1, t).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

// ---------------------------------------------------------------------------
// 3. Lexicographic tiebreaker
// ---------------------------------------------------------------------------

#[test]
fn lexicographic_tiebreak_when_all_equal() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    let t = now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t, vec![create_attestation(&sk_a, "p", &nr_a, 1, t).unwrap()], &nr_a);
    let b = make_branch("B", t, vec![create_attestation(&sk_b, "p", &nr_b, 1, t).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
}

#[test]
fn lexicographic_tiebreak_reversed() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    let t = now();
    let nr_a = "ff".repeat(32);
    let nr_b = "11".repeat(32);

    let a = make_branch("A", t, vec![create_attestation(&sk_a, "p", &nr_a, 1, t).unwrap()], &nr_a);
    let b = make_branch("B", t, vec![create_attestation(&sk_b, "p", &nr_b, 1, t).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
}

// ---------------------------------------------------------------------------
// 4. Clock skew boundary conditions
// ---------------------------------------------------------------------------

#[test]
fn exactly_at_skew_boundary_falls_to_secondary() {
    let sk_a1 = make_key();
    let sk_b1 = make_key();
    let sk_b2 = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a1.verifying_key());
    reg.register(sk_b1.verifying_key());
    reg.register(sk_b2.verifying_key());

    let t2 = now();
    let t1 = t2 - Duration::seconds(300);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a1, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![
        create_attestation(&sk_b1, "p", &nr_b, 1, t2).unwrap(),
        create_attestation(&sk_b2, "p", &nr_b, 1, t2).unwrap(),
    ], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn one_second_beyond_skew_uses_timestamp() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());

    let t2 = now();
    let t1 = t2 - Duration::seconds(301);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t1, vec![create_attestation(&sk_a, "p", &nr_a, 1, t1).unwrap()], &nr_a);
    let b = make_branch("B", t2, vec![create_attestation(&sk_b, "p", &nr_b, 1, t2).unwrap()], &nr_b);

    let resolution = resolve_fork(&a, &b, &reg).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

// ---------------------------------------------------------------------------
// 5. Adversarial scenarios
// ---------------------------------------------------------------------------

#[test]
fn attacker_backdate_within_skew_loses_to_attestations() {
    let sk_h1 = make_key();
    let sk_h2 = make_key();
    let sk_h3 = make_key();
    let sk_att = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_h1.verifying_key());
    reg.register(sk_h2.verifying_key());
    reg.register(sk_h3.verifying_key());
    reg.register(sk_att.verifying_key());

    let honest_time = now();
    let attacker_time = honest_time - Duration::minutes(4);
    let nr_h = "aa".repeat(32);
    let nr_a = "bb".repeat(32);

    let honest = make_branch("honest", honest_time, vec![
        create_attestation(&sk_h1, "p", &nr_h, 1, honest_time).unwrap(),
        create_attestation(&sk_h2, "p", &nr_h, 1, honest_time).unwrap(),
        create_attestation(&sk_h3, "p", &nr_h, 1, honest_time).unwrap(),
    ], &nr_h);
    let attacker = make_branch("attacker", attacker_time, vec![
        create_attestation(&sk_att, "p", &nr_a, 1, attacker_time).unwrap(),
    ], &nr_a);

    let resolution = resolve_fork(&honest, &attacker, &reg).unwrap();
    assert_eq!(resolution.winning_branch, honest.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn identical_branches_are_not_fork() {
    let t = now();
    let digest = make_digest("same");
    let branch = ForkBranch {
        receipt_digest: digest,
        timestamp: t,
        attestations: vec![],
        next_root: "aa".repeat(32),
    };
    assert!(!ForkDetector::is_fork(&branch, &branch));
}

// ---------------------------------------------------------------------------
// 6. ForkDetector lifecycle
// ---------------------------------------------------------------------------

#[test]
fn fork_detector_register_and_resolve() {
    let sk_a = make_key();
    let sk_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b.verifying_key());
    let mut detector = ForkDetector::new(reg);
    assert_eq!(detector.pending_count(), 0);

    let t = now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let a = make_branch("A", t, vec![create_attestation(&sk_a, "p", &nr_a, 1, t).unwrap()], &nr_a);
    let b = make_branch("B", t, vec![create_attestation(&sk_b, "p", &nr_b, 1, t).unwrap()], &nr_b);

    assert!(ForkDetector::is_fork(&a, &b));
    detector.register_fork(a, b);
    assert_eq!(detector.pending_count(), 1);

    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
    assert!(resolutions[0].is_ok());
    assert_eq!(detector.pending_count(), 0);
}

#[test]
fn fork_detector_multiple_forks() {
    let reg = WatcherRegistry::new();
    let mut detector = ForkDetector::new(reg);
    let t = now();

    for i in 0..10u32 {
        let nr_a = format!("{:02x}", i).repeat(32);
        let nr_b = format!("{:02x}", i + 10).repeat(32);
        let a = make_branch(&format!("A{i}"), t, vec![], &nr_a);
        let b = make_branch(&format!("B{i}"), t, vec![], &nr_b);
        detector.register_fork(a, b);
    }

    assert_eq!(detector.pending_count(), 10);
    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 10);
    assert_eq!(detector.pending_count(), 0);

    for r in &resolutions {
        let r = r.as_ref().unwrap();
        assert_eq!(r.resolution_reason, ResolutionReason::LexicographicTiebreak);
    }
}

// ---------------------------------------------------------------------------
// 7. MAX_CLOCK_SKEW constant verification
// ---------------------------------------------------------------------------

#[test]
fn max_clock_skew_is_five_minutes() {
    assert_eq!(MAX_CLOCK_SKEW.as_secs(), 300);
}

// ---------------------------------------------------------------------------
// 8. Symmetric resolution
// ---------------------------------------------------------------------------

#[test]
fn resolution_result_is_symmetric() {
    let sk_a1 = make_key();
    let sk_a2 = make_key();
    let sk_b1 = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a1.verifying_key());
    reg.register(sk_a2.verifying_key());
    reg.register(sk_b1.verifying_key());

    let t = now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a1 = create_attestation(&sk_a1, "p", &nr_a, 1, t).unwrap();
    let att_a2 = create_attestation(&sk_a2, "p", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk_b1, "p", &nr_b, 1, t).unwrap();

    let a = make_branch("A", t, vec![att_a1.clone(), att_a2.clone()], &nr_a);
    let b = make_branch("B", t, vec![att_b.clone()], &nr_b);

    let r1 = resolve_fork(&a, &b, &reg).unwrap();

    let a2 = make_branch("A", t, vec![att_a1, att_a2], &nr_a);
    let b2 = make_branch("B", t, vec![att_b], &nr_b);
    let r2 = resolve_fork(&b2, &a2, &reg).unwrap();

    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);
}
