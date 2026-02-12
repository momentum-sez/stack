//! # Fork Resolution Protocol Test (3-Level Ordering)
//!
//! Exhaustive test of the three-level fork resolution protocol defined
//! in Protocol 16.1 section 3:
//! 1. Primary: Timestamp ordering (only if delta > MAX_CLOCK_SKEW)
//! 2. Secondary: Watcher attestation count
//! 3. Tertiary: Lexicographic ordering of next_root digest
//!
//! Also tests clock skew rejection and determinism guarantees.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{ForkBranch, ResolutionReason, MAX_CLOCK_SKEW};
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"resolution": label})).unwrap();
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
// 1. Primary ordering by timestamp
// ---------------------------------------------------------------------------

#[test]
fn primary_ordering_by_timestamp() {
    let t1 = Utc::now();
    let t2 = t1 + Duration::minutes(10); // Beyond skew

    let a = make_branch("early", t1, 1, &"aa".repeat(32));
    let b = make_branch("late", t2, 100, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
    assert_eq!(resolution.winning_branch, a.receipt_digest);
}

// ---------------------------------------------------------------------------
// 2. Secondary ordering by attestation count
// ---------------------------------------------------------------------------

#[test]
fn secondary_ordering_by_attestation_count() {
    let t1 = Utc::now();
    let t2 = t1 + Duration::minutes(2); // Within skew

    let a = make_branch("low-attest", t1, 2, &"aa".repeat(32));
    let b = make_branch("high-attest", t2, 8, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );
    assert_eq!(resolution.winning_branch, b.receipt_digest);
}

// ---------------------------------------------------------------------------
// 3. Tertiary ordering by lexicographic digest
// ---------------------------------------------------------------------------

#[test]
fn tertiary_ordering_by_lexicographic_digest() {
    let t = Utc::now();

    // Same timestamp and attestation count: tiebreak by next_root
    let a = make_branch("lex-a", t, 5, &"11".repeat(32)); // lexicographically smaller
    let b = make_branch("lex-b", t, 5, &"ff".repeat(32)); // lexicographically larger

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::LexicographicTiebreak
    );
    // "11...11" < "ff...ff", so A wins
    assert_eq!(resolution.winning_branch, a.receipt_digest);
}

// ---------------------------------------------------------------------------
// 4. Clock skew rejection
// ---------------------------------------------------------------------------

#[test]
fn clock_skew_rejection() {
    // Within skew: falls through to secondary
    let t1 = Utc::now();
    let t_within = t1 + Duration::seconds(299);
    let a = make_branch("skew-a", t1, 1, &"aa".repeat(32));
    let b = make_branch("skew-b", t_within, 10, &"bb".repeat(32));

    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );

    // Beyond skew: uses timestamp
    let t_beyond = t1 + Duration::seconds(301);
    let c = make_branch("skew-c", t_beyond, 10, &"cc".repeat(32));
    let resolution2 = msez_corridor::fork::resolve_fork(&a, &c);
    assert_eq!(
        resolution2.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );

    // Verify MAX_CLOCK_SKEW value
    assert_eq!(MAX_CLOCK_SKEW.as_secs(), 300);
}

// ---------------------------------------------------------------------------
// 5. Resolution is deterministic
// ---------------------------------------------------------------------------

#[test]
fn resolution_is_deterministic() {
    let t = Utc::now();
    let a = make_branch("det-a", t, 5, &"aa".repeat(32));
    let b = make_branch("det-b", t, 3, &"bb".repeat(32));

    let r1 = msez_corridor::fork::resolve_fork(&a, &b);
    let r2 = msez_corridor::fork::resolve_fork(&a, &b);

    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);

    // Order of arguments should not change winner
    let r3 = msez_corridor::fork::resolve_fork(&b, &a);
    assert_eq!(r1.winning_branch, r3.winning_branch);
}
