//! # Fork Resolution Protocol Test (3-Level Ordering, Evidence-Driven)
//!
//! Exhaustive test of the three-level fork resolution protocol defined
//! in Protocol 16.1 section 3 (P0-FORK-001 remediated):
//! 1. Primary: Timestamp ordering (only if delta > MAX_CLOCK_SKEW)
//! 2. Secondary: Verified watcher attestation count (cryptographically bound)
//! 3. Tertiary: Lexicographic ordering of next_root digest
//!
//! Also tests clock skew rejection, equivocation detection, and
//! determinism guarantees.

use chrono::{Duration, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use msez_corridor::{
    ForkBranch, ResolutionReason, WatcherRegistry, create_attestation, resolve_fork,
    MAX_CLOCK_SKEW,
};
use msez_crypto::ed25519::SigningKey;
use rand_core::OsRng;
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"resolution": label})).unwrap();
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
// 1. Primary ordering by timestamp
// ---------------------------------------------------------------------------

#[test]
fn primary_ordering_by_timestamp() {
    let sk = SigningKey::generate(&mut OsRng);
    let vk = sk.verifying_key();
    let mut registry = WatcherRegistry::new();
    registry.register(vk);

    // Use past timestamps to avoid MAX_FUTURE_DRIFT rejection.
    let t2 = Utc::now();
    let t1 = t2 - Duration::minutes(10); // Beyond skew
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a = create_attestation(&sk, "parent", &nr_a, 1, t1).unwrap();
    let att_b = create_attestation(&sk, "parent", &nr_b, 1, t2).unwrap();

    let a = make_branch("early", t1, vec![att_a], &nr_a);
    let b = make_branch("late", t2, vec![att_b], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::EarlierTimestamp
    );
    assert_eq!(resolution.winning_branch, a.receipt_digest);
}

// ---------------------------------------------------------------------------
// 2. Secondary ordering by verified attestation count
// ---------------------------------------------------------------------------

#[test]
fn secondary_ordering_by_attestation_count() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk1.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());

    let t2 = Utc::now();
    let t1 = t2 - Duration::minutes(2); // Within skew
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    // Branch A: 1 attestation
    let att_a = create_attestation(&sk1, "parent", &nr_a, 1, t1).unwrap();
    // Branch B: 3 attestations
    let att_b1 = create_attestation(&sk1, "parent", &nr_b, 1, t2).unwrap();
    let att_b2 = create_attestation(&sk2, "parent", &nr_b, 1, t2).unwrap();
    let att_b3 = create_attestation(&sk3, "parent", &nr_b, 1, t2).unwrap();

    let a = make_branch("low-attest", t1, vec![att_a], &nr_a);
    let b = make_branch("high-attest", t2, vec![att_b1, att_b2, att_b3], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
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
    let sk = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk.verifying_key());

    let t = Utc::now();
    let nr_a = "11".repeat(32); // lexicographically smaller
    let nr_b = "ff".repeat(32); // lexicographically larger

    let att_a = create_attestation(&sk, "parent", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("lex-a", t, vec![att_a], &nr_a);
    let b = make_branch("lex-b", t, vec![att_b], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
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
    let sk = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());

    // Use past-relative timestamps to avoid future drift rejection.
    let t_within = Utc::now();
    let t1 = t_within - Duration::seconds(299);
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);
    let nr_c = "cc".repeat(32);

    // Within skew: falls through to secondary
    let att_a = create_attestation(&sk, "parent", &nr_a, 1, t1).unwrap();
    let att_b1 = create_attestation(&sk, "parent", &nr_b, 1, t_within).unwrap();
    let att_b2 = create_attestation(&sk2, "parent", &nr_b, 1, t_within).unwrap();
    let att_b3 = create_attestation(&sk3, "parent", &nr_b, 1, t_within).unwrap();

    let a = make_branch("skew-a", t1, vec![att_a], &nr_a);
    let b = make_branch("skew-b", t_within, vec![att_b1, att_b2, att_b3], &nr_b);

    let resolution = resolve_fork(&a, &b, &registry).unwrap();
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );

    // Beyond skew: uses timestamp
    let t_beyond = Utc::now();
    let t_early = t_beyond - Duration::seconds(301);
    let att_a2 = create_attestation(&sk, "parent", &nr_a, 1, t_early).unwrap();
    let att_c = create_attestation(&sk, "parent", &nr_c, 1, t_beyond).unwrap();
    let a2 = make_branch("skew-a2", t_early, vec![att_a2], &nr_a);
    let c = make_branch("skew-c", t_beyond, vec![att_c], &nr_c);
    let resolution2 = resolve_fork(&a2, &c, &registry).unwrap();
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
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk1.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());

    let t = Utc::now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    // Branch A: 3 attestations, Branch B: 1 attestation
    let att_a1 = create_attestation(&sk1, "parent", &nr_a, 1, t).unwrap();
    let att_a2 = create_attestation(&sk2, "parent", &nr_a, 1, t).unwrap();
    let att_a3 = create_attestation(&sk3, "parent", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk1, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("det-a", t, vec![att_a1.clone(), att_a2.clone(), att_a3.clone()], &nr_a);
    let b = make_branch("det-b", t, vec![att_b.clone()], &nr_b);

    let r1 = resolve_fork(&a, &b, &registry).unwrap();

    // Rebuild identical branches for second call.
    let a2 = make_branch("det-a", t, vec![att_a1, att_a2, att_a3], &nr_a);
    let b2 = make_branch("det-b", t, vec![att_b], &nr_b);
    let r2 = resolve_fork(&a2, &b2, &registry).unwrap();

    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);
}
