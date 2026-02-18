//! # Fork Inspection and Analysis Test (Evidence-Driven)
//!
//! Tests fork inspection — identifying divergence points, verifying that
//! identical branches are not considered forks, and inspecting resolution
//! reasons. Uses the ForkDetector to register and resolve multiple forks.

use chrono::{Duration, Utc};
use mez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use mez_corridor::{
    ForkBranch, ForkDetector, ResolutionReason, WatcherRegistry, create_attestation,
    resolve_fork,
};
use mez_crypto::ed25519::SigningKey;
use rand_core::OsRng;
use serde_json::json;

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"inspect": label})).unwrap();
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

// ---------------------------------------------------------------------------
// 1. Fork detector identifies divergent branches
// ---------------------------------------------------------------------------

#[test]
fn inspect_fork_identifies_divergence_point() {
    let sk_a = SigningKey::generate(&mut OsRng);
    let sk_b = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk_a.verifying_key());
    registry.register(sk_b.verifying_key());

    let t = Utc::now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let att_a = create_attestation(&sk_a, "parent", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk_b, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("fork-a", t, vec![att_a], &nr_a);
    let b = make_branch("fork-b", t, vec![att_b], &nr_b);

    // These have different digests, so they constitute a fork
    assert!(ForkDetector::is_fork(&a, &b));

    // Register and resolve
    let mut detector = ForkDetector::new(registry);
    detector.register_fork(a, b);
    assert_eq!(detector.pending_count(), 1);

    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
    assert!(resolutions[0].is_ok());
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
        attestations: vec![],
        next_root: "aa".repeat(32),
    };

    assert!(!ForkDetector::is_fork(&branch, &branch));
}

// ---------------------------------------------------------------------------
// 3. Fork resolution reason is correct
// ---------------------------------------------------------------------------

#[test]
fn inspect_fork_resolution_reason() {
    let sk1 = SigningKey::generate(&mut OsRng);
    let sk2 = SigningKey::generate(&mut OsRng);
    let sk3 = SigningKey::generate(&mut OsRng);
    let sk4 = SigningKey::generate(&mut OsRng);
    let mut registry = WatcherRegistry::new();
    registry.register(sk1.verifying_key());
    registry.register(sk2.verifying_key());
    registry.register(sk3.verifying_key());
    registry.register(sk4.verifying_key());

    let t = Utc::now();
    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    // Attestation count resolution (within clock skew)
    // sk1 attests for A; sk2, sk3, sk4 attest for B — no overlap
    let att_a = create_attestation(&sk1, "parent", &nr_a, 1, t).unwrap();
    let att_b1 = create_attestation(&sk2, "parent", &nr_b, 1, t).unwrap();
    let att_b2 = create_attestation(&sk3, "parent", &nr_b, 1, t).unwrap();
    let att_b3 = create_attestation(&sk4, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("attest-a", t, vec![att_a], &nr_a);
    let b = make_branch("attest-b", t, vec![att_b1, att_b2, att_b3], &nr_b);
    let resolution = resolve_fork(&a, &b, &registry).unwrap();
    assert_eq!(
        resolution.resolution_reason,
        ResolutionReason::MoreAttestations
    );
    assert_eq!(resolution.winning_branch, b.receipt_digest);

    // Timestamp resolution (beyond clock skew)
    // Use separate watchers for each branch to avoid equivocation
    let sk5 = SigningKey::generate(&mut OsRng);
    let sk6 = SigningKey::generate(&mut OsRng);
    registry.register(sk5.verifying_key());
    registry.register(sk6.verifying_key());

    let t_late = Utc::now();
    let t_early = t_late - Duration::minutes(10);
    let nr_c = "cc".repeat(32);
    let nr_d = "dd".repeat(32);
    let att_c = create_attestation(&sk5, "parent", &nr_c, 1, t_early).unwrap();
    let att_d = create_attestation(&sk6, "parent", &nr_d, 1, t_late).unwrap();
    let c = make_branch("time-c", t_early, vec![att_c], &nr_c);
    let d = make_branch("time-d", t_late, vec![att_d], &nr_d);
    let resolution2 = resolve_fork(&c, &d, &registry).unwrap();
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
    let registry = WatcherRegistry::new();
    let mut detector = ForkDetector::new(registry);
    let t = Utc::now();

    for i in 0..5u32 {
        let nr_a = format!("{:02x}", i).repeat(32);
        let nr_b = format!("{:02x}", i + 10).repeat(32);
        let a = make_branch(&format!("multi-a-{i}"), t, vec![], &nr_a);
        let b = make_branch(&format!("multi-b-{i}"), t, vec![], &nr_b);
        detector.register_fork(a, b);
    }

    assert_eq!(detector.pending_count(), 5);
    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 5);
    assert_eq!(detector.pending_count(), 0);

    // No attestations → all should resolve by lexicographic tiebreaker
    for r in &resolutions {
        let r = r.as_ref().unwrap();
        assert_eq!(r.resolution_reason, ResolutionReason::LexicographicTiebreak);
    }
}

// ---------------------------------------------------------------------------
// 5. Resolution is symmetric
// ---------------------------------------------------------------------------

#[test]
fn inspect_resolution_symmetric() {
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

    // sk1 + sk2 attest for A; sk3 attests for B — no overlap
    let att_a1 = create_attestation(&sk1, "parent", &nr_a, 1, t).unwrap();
    let att_a2 = create_attestation(&sk2, "parent", &nr_a, 1, t).unwrap();
    let att_b = create_attestation(&sk3, "parent", &nr_b, 1, t).unwrap();

    let a = make_branch("sym-a", t, vec![att_a1.clone(), att_a2.clone()], &nr_a);
    let b = make_branch("sym-b", t, vec![att_b.clone()], &nr_b);

    let r1 = resolve_fork(&a, &b, &registry).unwrap();

    let a2 = make_branch("sym-a", t, vec![att_a1, att_a2], &nr_a);
    let b2 = make_branch("sym-b", t, vec![att_b], &nr_b);
    let r2 = resolve_fork(&b2, &a2, &registry).unwrap();

    assert_eq!(r1.winning_branch, r2.winning_branch);
    assert_eq!(r1.losing_branch, r2.losing_branch);
    assert_eq!(r1.resolution_reason, r2.resolution_reason);
}
