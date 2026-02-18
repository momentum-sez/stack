//! # Smart Asset Fork Resolution Test (Evidence-Driven)
//!
//! Tests the three-level fork resolution protocol (Protocol 16.1 section 3,
//! P0-FORK-001 remediated):
//! 1. Primary: Timestamp ordering (if difference > MAX_CLOCK_SKEW)
//! 2. Secondary: Verified watcher attestation count (cryptographically bound)
//! 3. Tertiary: Lexicographic digest tiebreaker
//!
//! Also tests the clock skew boundary condition at exactly 5 minutes.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{
    ForkBranch, ForkDetector, ResolutionReason, WatcherRegistry, create_attestation,
    resolve_fork, MAX_CLOCK_SKEW,
};
use msez_crypto::ed25519::SigningKey;
use rand_core::OsRng;
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"fork_test": label})).unwrap();
    sha256_digest(&canonical)
}

fn make_branch(
    label: &str,
    timestamp: chrono::DateTime<Utc>,
    attestations: Vec<msez_corridor::WatcherAttestation>,
    next_root: &str,
) -> ForkBranch {
    ForkBranch {
        receipt_digest: make_digest(label),
        timestamp,
        attestations,
        next_root: next_root.to_string(),
    }
}

// ---------------------------------------------------------------------------
// 1. Fork resolved by timestamp (beyond clock skew)
// ---------------------------------------------------------------------------

#[test]
fn fork_resolved_by_timestamp() {
    let sk = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk.verifying_key());

    // Use past-relative timestamps to avoid MAX_FUTURE_DRIFT rejection.
    let t2 = Utc::now();
    let t1 = t2 - Duration::minutes(10); // 10 min > 5 min skew
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a = create_attestation(&sk, "parent", &nr_a, 1, t1).unwrap();
    let att_b = create_attestation(&sk, "parent", &nr_b, 1, t2).unwrap();

    let a = make_branch("A", t1, vec![att_a], &nr_a);
    let b = make_branch("B", t2, vec![att_b], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
    assert_eq!(resolution.winning_branch, a.receipt_digest);
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
}

// ---------------------------------------------------------------------------
// 2. Fork resolved by verified watcher attestation count (within skew)
// ---------------------------------------------------------------------------

#[test]
fn fork_resolved_by_watcher_count() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk1.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());

    let t2 = Utc::now();
    let t1 = t2 - Duration::minutes(3); // 3 min < 5 min skew
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a = create_attestation(&sk1, "parent", &nr_a, 1, t1).unwrap();
    let att_b1 = create_attestation(&sk1, "parent", &nr_b, 1, t2).unwrap();
    let att_b2 = create_attestation(&sk2, "parent", &nr_b, 1, t2).unwrap();
    let att_b3 = create_attestation(&sk3, "parent", &nr_b, 1, t2).unwrap();

    let a = make_branch("A", t1, vec![att_a], &nr_a);
    let b = make_branch("B", t2, vec![att_b1, att_b2, att_b3], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
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
    let sk = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk.verifying_key());

    let t = Utc::now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a = create_attestation(&sk, "parent", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("A", t, vec![att_a], &nr_a);
    let b = make_branch("B", t, vec![att_b], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
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
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk1.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());

    // Use past-relative timestamps to avoid MAX_FUTURE_DRIFT rejection.
    let t2 = Utc::now();
    let t1 = t2 - Duration::seconds(300);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);
    let nr_c = "cc".repeat(32);

    // Exactly at 5 minutes: falls through to secondary ordering
    let att_a = create_attestation(&sk1, "parent", &nr_a, 1, t1).unwrap();
    let att_b1 = create_attestation(&sk1, "parent", &nr_b, 1, t2).unwrap();
    let att_b2 = create_attestation(&sk2, "parent", &nr_b, 1, t2).unwrap();
    let att_b3 = create_attestation(&sk3, "parent", &nr_b, 1, t2).unwrap();

    let a = make_branch("A", t1, vec![att_a], &nr_a);
    let b = make_branch("B", t2, vec![att_b1, att_b2, att_b3], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );

    // One second beyond: uses timestamp
    let t3 = Utc::now();
    let t0 = t3 - Duration::seconds(301);
    let att_a2 = create_attestation(&sk1, "parent", &nr_a, 1, t0).unwrap();
    let att_c = create_attestation(&sk1, "parent", &nr_c, 1, t3).unwrap();
    let a2 = make_branch("A2", t0, vec![att_a2], &nr_a);
    let c = make_branch("C", t3, vec![att_c], &nr_c);
    let resolution2 = resolve_fork(&a2, &c, &registry).unwrap();
    assert_eq!(
        resolution2.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
    assert_eq!(resolution2.winning_branch, a2.receipt_digest);
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
        attestations: vec![],
        next_root: "aa".repeat(32),
    };
    assert!(!ForkDetector::is_fork(&branch, &branch));
}
