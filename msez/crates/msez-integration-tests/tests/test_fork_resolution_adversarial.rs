//! # Fork Resolution Adversarial Test
//!
//! Tests the three-level fork resolution protocol (Protocol 16.1 §3):
//! 1. Primary: Timestamp (if difference > MAX_CLOCK_SKEW)
//! 2. Secondary: Watcher attestation count
//! 3. Tertiary: Lexicographic digest tiebreaker
//!
//! Includes adversarial scenarios: timestamp backdating, clock skew boundary
//! conditions, and equal-attestation tiebreaking.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{ForkBranch, ForkDetector, ResolutionReason, MAX_CLOCK_SKEW};
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"branch": label})).unwrap();
    sha256_digest(&canonical)
}

fn make_branch(
    label: &str,
    timestamp: chrono::DateTime<Utc>,
    attestation_count: u32,
    next_root: &str,
) -> ForkBranch {
    ForkBranch {
        receipt_digest: make_digest(label),
        timestamp,
        attestation_count,
        next_root: next_root.to_string(),
    }
}

fn now() -> chrono::DateTime<Utc> {
    Utc::now()
}

// ---------------------------------------------------------------------------
// 1. Timestamp ordering beyond clock skew
// ---------------------------------------------------------------------------

#[test]
fn earlier_timestamp_wins_beyond_skew() {
    let t1 = now();
    let t2 = t1 + Duration::minutes(10); // 10 min > 5 min skew

    let a = make_branch("A", t1, 3, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn earlier_timestamp_wins_reversed() {
    let t1 = now();
    let t2 = t1 + Duration::minutes(10);

    let a = make_branch("A", t2, 10, &"aa".repeat(32));
    let b = make_branch("B", t1, 1, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn large_time_difference_uses_timestamp() {
    let t1 = now();
    let t2 = t1 + Duration::hours(1);

    let a = make_branch("A", t1, 0, &"ff".repeat(32));
    let b = make_branch("B", t2, 100, &"00".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

// ---------------------------------------------------------------------------
// 2. Attestation count ordering within clock skew
// ---------------------------------------------------------------------------

#[test]
fn more_attestations_wins_within_skew() {
    let t1 = now();
    let t2 = t1 + Duration::minutes(3); // 3 min < 5 min skew

    let a = make_branch("A", t1, 3, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn more_attestations_wins_same_timestamp() {
    let t = now();

    let a = make_branch("A", t, 7, &"aa".repeat(32));
    let b = make_branch("B", t, 3, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

// ---------------------------------------------------------------------------
// 3. Lexicographic tiebreaker
// ---------------------------------------------------------------------------

#[test]
fn lexicographic_tiebreak_when_all_equal() {
    let t = now();

    let a = make_branch("A", t, 3, &"aa".repeat(32));
    let b = make_branch("B", t, 3, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
}

#[test]
fn lexicographic_tiebreak_reversed() {
    let t = now();

    let a = make_branch("A", t, 3, &"ff".repeat(32));
    let b = make_branch("B", t, 3, &"11".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    // "11...11" < "ff...ff" lexicographically, so B wins
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
}

// ---------------------------------------------------------------------------
// 4. Clock skew boundary conditions
// ---------------------------------------------------------------------------

#[test]
fn exactly_at_skew_boundary_falls_to_secondary() {
    let t1 = now();
    let t2 = t1 + Duration::seconds(300); // Exactly 5 minutes

    let a = make_branch("A", t1, 2, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    // Exactly at boundary: time_diff == skew (not strictly greater)
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn one_second_beyond_skew_uses_timestamp() {
    let t1 = now();
    let t2 = t1 + Duration::seconds(301); // 5 min + 1 sec

    let a = make_branch("A", t1, 2, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn one_second_within_skew_falls_to_secondary() {
    let t1 = now();
    let t2 = t1 + Duration::seconds(299); // 5 min - 1 sec

    let a = make_branch("A", t1, 1, &"aa".repeat(32));
    let b = make_branch("B", t2, 3, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
    assert_eq!(resolution.winning_branch, b.receipt_digest);
}

// ---------------------------------------------------------------------------
// 5. Adversarial scenarios
// ---------------------------------------------------------------------------

#[test]
fn attacker_backdate_within_skew_loses_to_attestations() {
    let honest_time = now();
    let attacker_time = honest_time - Duration::minutes(4); // 4 min backdate

    let honest = make_branch("honest", honest_time, 5, &"aa".repeat(32));
    let attacker = make_branch("attacker", attacker_time, 1, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&honest, &attacker);
    // Within skew → attestation count → honest wins (5 > 1)
    assert_eq!(resolution.winning_branch, honest.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn attacker_backdate_beyond_skew_wins_timestamp() {
    let honest_time = now();
    let attacker_time = honest_time - Duration::minutes(10); // 10 min backdate

    let honest = make_branch("honest", honest_time, 5, &"aa".repeat(32));
    let attacker = make_branch("attacker", attacker_time, 1, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&honest, &attacker);
    // Beyond skew → timestamp ordering → attacker wins (earlier timestamp)
    assert_eq!(resolution.winning_branch, attacker.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
}

#[test]
fn two_honest_nodes_with_similar_timestamps() {
    // Two honest nodes both submit within reasonable clock drift
    let t1 = now();
    let t2 = t1 + Duration::seconds(2); // 2 seconds apart

    let node_a = make_branch("node-a", t1, 3, &"aa".repeat(32));
    let node_b = make_branch("node-b", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&node_a, &node_b);
    // Well within skew → attestation count wins
    assert_eq!(resolution.winning_branch, node_b.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
}

#[test]
fn identical_branches_are_not_fork() {
    let t = now();
    let digest = make_digest("same");
    let branch = ForkBranch {
        receipt_digest: digest,
        timestamp: t,
        attestation_count: 3,
        next_root: "aa".repeat(32),
    };
    assert!(!ForkDetector::is_fork(&branch, &branch));
}

// ---------------------------------------------------------------------------
// 6. ForkDetector lifecycle
// ---------------------------------------------------------------------------

#[test]
fn fork_detector_register_and_resolve() {
    let mut detector = ForkDetector::new();
    assert_eq!(detector.pending_count(), 0);

    let t = now();
    let a = make_branch("A", t, 3, &"aa".repeat(32));
    let b = make_branch("B", t, 5, &"bb".repeat(32));

    assert!(ForkDetector::is_fork(&a, &b));
    detector.register_fork(a, b);
    assert_eq!(detector.pending_count(), 1);

    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
    assert_eq!(detector.pending_count(), 0);
}

#[test]
fn fork_detector_multiple_forks() {
    let mut detector = ForkDetector::new();
    let t = now();

    for i in 0..10 {
        let a = make_branch(
            &format!("A{i}"),
            t,
            i,
            &format!("{:02x}", i).repeat(32),
        );
        let b = make_branch(
            &format!("B{i}"),
            t,
            i + 1,
            &format!("{:02x}", i + 10).repeat(32),
        );
        detector.register_fork(a, b);
    }

    assert_eq!(detector.pending_count(), 10);
    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 10);
    assert_eq!(detector.pending_count(), 0);

    // All should resolve by attestation count (same timestamp)
    for r in &resolutions {
        assert_eq!(r.resolution_reason, ResolutionReason::MoreAttestations);
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
// 8. Symmetric resolution (order of arguments doesn't matter for result)
// ---------------------------------------------------------------------------

#[test]
fn resolution_result_is_symmetric() {
    let t = now();
    let a = make_branch("A", t, 5, &"aa".repeat(32));
    let b = make_branch("B", t, 3, &"bb".repeat(32));

    let r1 = msez_corridor::fork::resolve_fork(&a, &b);
    let r2 = msez_corridor::fork::resolve_fork(&b, &a);

    // The winner should be the same regardless of argument order
    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);
}
