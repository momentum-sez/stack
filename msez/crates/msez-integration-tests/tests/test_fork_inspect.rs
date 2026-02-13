//! # Fork Inspection and Analysis Test
//!
//! Tests fork inspection — identifying divergence points, verifying that
//! identical branches are not considered forks, and inspecting resolution
//! reasons. Uses the ForkDetector to register and resolve multiple forks.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{ForkBranch, ForkDetector, ResolutionReason};
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"inspect": label})).unwrap();
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
// 1. Fork detector identifies divergent branches
// ---------------------------------------------------------------------------

#[test]
fn inspect_fork_identifies_divergence_point() {
    let t = Utc::now();
    let a = make_branch("fork-a", t, 5, &"aa".repeat(32));
    let b = make_branch("fork-b", t, 3, &"bb".repeat(32));

    // These have different digests, so they constitute a fork
    assert!(ForkDetector::is_fork(&a, &b));

    // Register and resolve
    let mut detector = ForkDetector::new();
    detector.register_fork(a, b);
    assert_eq!(detector.pending_count(), 1);

    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
    assert_eq!(detector.pending_count(), 0);
}

// ---------------------------------------------------------------------------
// 2. Identical branches are not a fork
// ---------------------------------------------------------------------------

#[test]
fn inspect_no_fork_for_identical_branches() {
    let t = Utc::now();
    let digest = make_digest("identical");
    let branch = ForkBranch {
        receipt_digest: digest,
        timestamp: t,
        attestation_count: 3,
        next_root: "aa".repeat(32),
    };

    assert!(!ForkDetector::is_fork(&branch, &branch));
}

// ---------------------------------------------------------------------------
// 3. Fork resolution reason is correct
// ---------------------------------------------------------------------------

#[test]
fn inspect_fork_resolution_reason() {
    let t = Utc::now();

    // Attestation count resolution (within clock skew)
    let a = make_branch("attest-a", t, 2, &"aa".repeat(32));
    let b = make_branch("attest-b", t, 7, &"bb".repeat(32));
    let resolution = msez_corridor::fork::resolve_fork(&a, &b);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );
    assert_eq!(resolution.winning_branch, b.receipt_digest);

    // Timestamp resolution (beyond clock skew)
    let t_early = Utc::now();
    let t_late = t_early + Duration::minutes(10);
    let c = make_branch("time-c", t_early, 1, &"cc".repeat(32));
    let d = make_branch("time-d", t_late, 100, &"dd".repeat(32));
    let resolution2 = msez_corridor::fork::resolve_fork(&c, &d);
    assert_eq!(
        resolution2.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
    assert_eq!(resolution2.winning_branch, c.receipt_digest);
}

// ---------------------------------------------------------------------------
// 4. Multiple forks can be registered and resolved
// ---------------------------------------------------------------------------

#[test]
fn inspect_multiple_forks() {
    let mut detector = ForkDetector::new();
    let t = Utc::now();

    for i in 0..5 {
        let a = make_branch(
            &format!("multi-a-{i}"),
            t,
            i,
            &format!("{:02x}", i).repeat(32),
        );
        let b = make_branch(
            &format!("multi-b-{i}"),
            t,
            i + 1,
            &format!("{:02x}", i + 10).repeat(32),
        );
        detector.register_fork(a, b);
    }

    assert_eq!(detector.pending_count(), 5);
    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 5);
    assert_eq!(detector.pending_count(), 0);

    // All should resolve by attestation count (same timestamp)
    for r in &resolutions {
        assert_eq!(r.resolution_reason, ResolutionReason::MoreAttestations);
    }
}

// ---------------------------------------------------------------------------
// 5. Resolution is symmetric
// ---------------------------------------------------------------------------

#[test]
fn inspect_resolution_symmetric() {
    let t = Utc::now();
    let a = make_branch("sym-a", t, 5, &"aa".repeat(32));
    let b = make_branch("sym-b", t, 3, &"bb".repeat(32));

    let r1 = msez_corridor::fork::resolve_fork(&a, &b);
    let r2 = msez_corridor::fork::resolve_fork(&b, &a);

    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);
}
