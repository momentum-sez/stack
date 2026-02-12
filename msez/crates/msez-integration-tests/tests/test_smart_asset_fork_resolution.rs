//! # Smart Asset Fork Resolution Test
//!
//! Tests the three-level fork resolution protocol (Protocol 16.1 section 3):
//! 1. Primary: Timestamp ordering (if difference > MAX_CLOCK_SKEW)
//! 2. Secondary: Watcher attestation count
//! 3. Tertiary: Lexicographic digest tiebreaker
//!
//! Also tests the clock skew boundary condition at exactly 5 minutes.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{ForkBranch, ForkDetector, ResolutionReason, MAX_CLOCK_SKEW};
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"fork_test": label})).unwrap();
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

// ---------------------------------------------------------------------------
// 1. Fork resolved by timestamp (beyond clock skew)
// ---------------------------------------------------------------------------

#[test]
fn fork_resolved_by_timestamp() {
    let t1 = Utc::now();
    let t2 = t1 + Duration::minutes(10); // 10 min > 5 min skew

    let a = make_branch("A", t1, 3, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
}

// ---------------------------------------------------------------------------
// 2. Fork resolved by watcher attestation count (within clock skew)
// ---------------------------------------------------------------------------

#[test]
fn fork_resolved_by_watcher_count() {
    let t1 = Utc::now();
    let t2 = t1 + Duration::minutes(3); // 3 min < 5 min skew

    let a = make_branch("A", t1, 2, &"aa".repeat(32));
    let b = make_branch("B", t2, 7, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(resolution.winning_branch, b.receipt_digest);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );
}

// ---------------------------------------------------------------------------
// 3. Fork resolved by lexicographic tiebreak
// ---------------------------------------------------------------------------

#[test]
fn fork_resolved_by_lexicographic_tiebreak() {
    let t = Utc::now();

    // Same timestamp, same attestation count -> tiebreak by next_root
    let a = make_branch("A", t, 3, &"aa".repeat(32));
    let b = make_branch("B", t, 3, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::LexicographicTiebreak
    );
    // "aa...aa" < "bb...bb" lexicographically, so A wins
    assert_eq!(resolution.winning_branch, a.receipt_digest);
}

// ---------------------------------------------------------------------------
// 4. Clock skew boundary conditions
// ---------------------------------------------------------------------------

#[test]
fn clock_skew_boundary() {
    // Exactly at 5 minutes: falls through to secondary ordering
    let t1 = Utc::now();
    let t2 = t1 + Duration::seconds(300);

    let a = make_branch("A", t1, 2, &"aa".repeat(32));
    let b = make_branch("B", t2, 5, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );

    // One second beyond: uses timestamp
    let t3 = t1 + Duration::seconds(301);
    let c = make_branch("C", t3, 10, &"cc".repeat(32));
    let resolution2 = msez_corridor::fork::resolve_fork(&a, &c);
    assert_eq!(
        resolution2.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
    assert_eq!(resolution2.winning_branch, a.receipt_digest);
}

// ---------------------------------------------------------------------------
// 5. MAX_CLOCK_SKEW constant verification
// ---------------------------------------------------------------------------

#[test]
fn max_clock_skew_is_five_minutes() {
    assert_eq!(MAX_CLOCK_SKEW.as_secs(), 300);
}

// ---------------------------------------------------------------------------
// 6. Identical branches are not a fork
// ---------------------------------------------------------------------------

#[test]
fn identical_branches_are_not_fork() {
    let t = Utc::now();
    let digest = make_digest("same");
    let branch = ForkBranch {
        receipt_digest: digest,
        timestamp: t,
        attestation_count: 3,
        next_root: "aa".repeat(32),
    };
    assert!(!ForkDetector::is_fork(&branch, &branch));
}
