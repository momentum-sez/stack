//! # Fork Detection and Resolution (Evidence-Driven)
//!
//! Detects and resolves forks in the corridor receipt chain using
//! cryptographically-bound watcher attestations.
//!
//! ## Three-Level Ordering
//!
//! 1. **Primary:** Timestamp — earlier wins, but only if the difference
//!    exceeds the clock skew tolerance AND both timestamps are within
//!    the monotonic bound relative to `now`.
//! 2. **Secondary:** Verified watcher attestation count — more independent
//!    attestations wins. Each attestation must be a signed binding of
//!    `(parent_root, candidate_root, height, timestamp)` by a registered
//!    watcher public key.
//! 3. **Tertiary:** Lexicographic ordering of receipt digest —
//!    deterministic tiebreaker when all else is equal.
//!
//! ## Security Invariant (P0-FORK-001 Remediation)
//!
//! `attestation_count` is NO LONGER a self-reported field. The fork
//! resolver counts ONLY cryptographically verified attestations from
//! distinct registered watchers. Backdated timestamps beyond the
//! drift bound are rejected outright.
//!
//! ## Spec Reference
//!
//! Implements Protocol 16.1 §3 from `spec/40-corridors.md`.

use std::collections::BTreeSet;
use std::time::Duration;

use chrono::{DateTime, Utc};
use msez_core::{CanonicalBytes, ContentDigest};
use msez_crypto::ed25519::{Ed25519Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Maximum allowed clock skew for fork resolution timestamps.
///
/// If two competing branches have timestamps within this window, the
/// timestamp ordering is considered inconclusive and resolution falls
/// through to secondary ordering (watcher attestation count).
///
/// This prevents the timestamp-backdating attack described in audit §3.5.
pub const MAX_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);

/// Maximum allowed timestamp drift from `now` into the future.
///
/// Any branch with `timestamp > now + MAX_FUTURE_DRIFT` is considered
/// invalid and rejected by the fork resolver.
pub const MAX_FUTURE_DRIFT: Duration = Duration::from_secs(60);

/// Maximum allowed timestamp age from `now` into the past.
///
/// Any branch with `timestamp < now - MAX_PAST_AGE` is considered
/// stale and rejected by the fork resolver. This prevents the
/// timestamp-backdating attack where an attacker submits a branch
/// with `ts=epoch` to deterministically win timestamp ordering.
pub const MAX_PAST_AGE: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours

/// Errors during fork resolution.
#[derive(Error, Debug)]
pub enum ForkError {
    /// A watcher attestation signature is invalid.
    #[error("invalid attestation signature from watcher {watcher_key_hex}: {reason}")]
    InvalidAttestation {
        /// Hex-encoded watcher public key.
        watcher_key_hex: String,
        /// Reason the attestation was rejected.
        reason: String,
    },

    /// Branch timestamp is in the future beyond the drift bound.
    #[error("branch timestamp {timestamp} is beyond future drift bound (now={now}, max_drift={max_drift_secs}s)")]
    FutureTimestamp {
        /// The offending timestamp.
        timestamp: DateTime<Utc>,
        /// Current time at evaluation.
        now: DateTime<Utc>,
        /// Maximum allowed drift in seconds.
        max_drift_secs: u64,
    },

    /// Branch timestamp is too far in the past (stale/backdated).
    #[error("branch timestamp {timestamp} is beyond past age bound (now={now}, max_age={max_age_secs}s)")]
    PastTimestamp {
        /// The offending timestamp.
        timestamp: DateTime<Utc>,
        /// Current time at evaluation.
        now: DateTime<Utc>,
        /// Maximum allowed age in seconds.
        max_age_secs: u64,
    },

    /// Watcher equivocation detected during fork resolution.
    /// The equivocating watchers' keys are reported for slashing.
    #[error("equivocation detected during fork resolution: {equivocating_watchers:?}")]
    EquivocationDetected {
        /// Hex-encoded public keys of equivocating watchers.
        equivocating_watchers: Vec<String>,
    },

    /// Watcher equivocation detected: same watcher signed conflicting
    /// attestations at the same height.
    #[error("watcher equivocation: {watcher_key_hex} signed conflicting attestations at height {height}")]
    WatcherEquivocation {
        /// Hex-encoded watcher public key.
        watcher_key_hex: String,
        /// Height at which equivocation was detected.
        height: u64,
    },

    /// Canonicalization error when verifying attestation payload.
    #[error("canonicalization error: {0}")]
    Canonicalization(String),
}

/// A signed watcher attestation binding a watcher's identity to a
/// specific fork branch at a given height.
///
/// ## Security Model
///
/// Each attestation is a signed statement: "I, watcher W, attest that
/// `candidate_root` is the valid next root following `parent_root` at
/// height H, observed at time T."
///
/// The attestation payload (excluding signature) is canonicalized and
/// signed with the watcher's Ed25519 key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatcherAttestation {
    /// The watcher's public key (hex-encoded).
    pub watcher_key: String,
    /// The parent root this attestation is relative to.
    pub parent_root: String,
    /// The candidate next_root being attested.
    pub candidate_root: String,
    /// The sequence height of the receipt being attested.
    pub height: u64,
    /// Timestamp of when the watcher observed this branch.
    pub timestamp: DateTime<Utc>,
    /// Ed25519 signature over the canonical attestation payload.
    pub signature: Ed25519Signature,
}

/// A branch in a fork — one of two (or more) competing receipt chains
/// that reference the same parent with different content.
///
/// ## P0-FORK-001 Remediation
///
/// The `attestation_count` field has been removed. Instead, branches
/// carry a set of signed [`WatcherAttestation`]s. The fork resolver
/// verifies each signature and counts only valid, distinct attestations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkBranch {
    /// Content digest of the branch's receipt.
    pub receipt_digest: ContentDigest,
    /// Timestamp of the branch's receipt.
    pub timestamp: DateTime<Utc>,
    /// Signed watcher attestations for this branch.
    pub attestations: Vec<WatcherAttestation>,
    /// The receipt's next_root digest (64 hex chars).
    pub next_root: String,
}

/// The result of resolving a fork in the corridor receipt chain.
#[derive(Debug, Clone)]
pub struct ForkResolution {
    /// The digest of the winning branch's receipt.
    pub winning_branch: ContentDigest,
    /// The losing branch's receipt digest.
    pub losing_branch: ContentDigest,
    /// The reason the winning branch was selected.
    pub resolution_reason: ResolutionReason,
    /// Number of verified attestations for the winning branch.
    pub winning_attestation_count: usize,
    /// Number of verified attestations for the losing branch.
    pub losing_attestation_count: usize,
}

/// Why a particular branch won fork resolution.
///
/// The three-level ordering ensures deterministic resolution even under
/// adversarial conditions (timestamp manipulation, watcher collusion).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolutionReason {
    /// Won by earlier timestamp (primary ordering).
    /// Timestamp difference exceeded the 5-minute clock skew tolerance.
    EarlierTimestamp,
    /// Won by more verified watcher attestations (secondary ordering).
    /// Timestamps were within the clock skew tolerance window.
    MoreAttestations,
    /// Won by lexicographic digest ordering (tertiary tiebreaker).
    /// Both timestamps and attestation counts were equal or within tolerance.
    LexicographicTiebreak,
}

/// Set of registered watcher public keys that are authorized to
/// produce attestations for fork resolution.
#[derive(Debug, Clone, Default)]
pub struct WatcherRegistry {
    /// Registered watcher verifying keys, indexed by hex-encoded key.
    watchers: std::collections::HashMap<String, VerifyingKey>,
}

impl WatcherRegistry {
    /// Create a new empty watcher registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a watcher's public key.
    pub fn register(&mut self, key: VerifyingKey) {
        self.watchers.insert(key.to_hex(), key);
    }

    /// Check if a watcher key is registered.
    pub fn is_registered(&self, key_hex: &str) -> bool {
        self.watchers.contains_key(key_hex)
    }

    /// Get a watcher's verifying key by hex string.
    pub fn get(&self, key_hex: &str) -> Option<&VerifyingKey> {
        self.watchers.get(key_hex)
    }

    /// Number of registered watchers.
    pub fn count(&self) -> usize {
        self.watchers.len()
    }
}

/// Fork detector and resolver for corridor receipt chains.
///
/// Detects forks by identifying two receipts that reference the same
/// parent (same sequence number and prev_root) with different content,
/// and resolves them using the three-level ordering protocol.
///
/// ## P0-FORK-001 Remediation
///
/// Fork resolution now requires a [`WatcherRegistry`] to verify
/// attestation signatures. Self-reported attestation counts are rejected.
#[derive(Debug)]
pub struct ForkDetector {
    /// Detected forks awaiting resolution.
    detected_forks: Vec<(ForkBranch, ForkBranch)>,
    /// Registry of authorized watcher keys.
    registry: WatcherRegistry,
}

impl ForkDetector {
    /// Create a new fork detector with the given watcher registry.
    pub fn new(registry: WatcherRegistry) -> Self {
        Self {
            detected_forks: Vec::new(),
            registry,
        }
    }

    /// Register a detected fork between two competing branches.
    pub fn register_fork(&mut self, branch_a: ForkBranch, branch_b: ForkBranch) {
        self.detected_forks.push((branch_a, branch_b));
    }

    /// Return the number of unresolved forks.
    pub fn pending_count(&self) -> usize {
        self.detected_forks.len()
    }

    /// Resolve all pending forks and return the resolutions.
    ///
    /// Each fork is resolved using the three-level ordering with
    /// cryptographic attestation verification.
    pub fn resolve_all(&mut self) -> Vec<Result<ForkResolution, ForkError>> {
        let forks = std::mem::take(&mut self.detected_forks);
        forks
            .into_iter()
            .map(|(a, b)| resolve_fork(&a, &b, &self.registry))
            .collect()
    }

    /// Check if two receipts constitute a fork (same parent, different content).
    pub fn is_fork(receipt_a: &ForkBranch, receipt_b: &ForkBranch) -> bool {
        receipt_a.receipt_digest != receipt_b.receipt_digest
    }

    /// Detect watcher equivocation: a single watcher signing attestations
    /// for conflicting branches at the same height.
    ///
    /// Returns the hex key of equivocating watchers (for slashing).
    pub fn detect_equivocation(
        branch_a: &ForkBranch,
        branch_b: &ForkBranch,
    ) -> Vec<String> {
        let watchers_a: BTreeSet<&str> = branch_a
            .attestations
            .iter()
            .map(|a| a.watcher_key.as_str())
            .collect();
        let watchers_b: BTreeSet<&str> = branch_b
            .attestations
            .iter()
            .map(|a| a.watcher_key.as_str())
            .collect();

        watchers_a
            .intersection(&watchers_b)
            .map(|s| s.to_string())
            .collect()
    }
}

/// Verify a single watcher attestation.
///
/// Checks:
/// 1. Watcher key is registered in the registry.
/// 2. Attestation payload signature is valid.
/// 3. Attestation binds to the correct candidate_root.
fn verify_attestation(
    attestation: &WatcherAttestation,
    expected_candidate_root: &str,
    registry: &WatcherRegistry,
) -> Result<(), ForkError> {
    // 1. Check watcher is registered.
    let vk = registry.get(&attestation.watcher_key).ok_or_else(|| {
        ForkError::InvalidAttestation {
            watcher_key_hex: attestation.watcher_key.clone(),
            reason: "watcher not registered".to_string(),
        }
    })?;

    // 2. Check attestation binds to the correct candidate_root.
    if attestation.candidate_root != expected_candidate_root {
        return Err(ForkError::InvalidAttestation {
            watcher_key_hex: attestation.watcher_key.clone(),
            reason: format!(
                "candidate_root mismatch: attestation={}, expected={}",
                attestation.candidate_root, expected_candidate_root
            ),
        });
    }

    // 3. Verify signature over canonical attestation payload.
    let payload = serde_json::json!({
        "candidate_root": attestation.candidate_root,
        "height": attestation.height,
        "parent_root": attestation.parent_root,
        "timestamp": attestation.timestamp.to_rfc3339(),
        "watcher_key": attestation.watcher_key,
    });
    let canonical = CanonicalBytes::from_value(payload).map_err(|e| {
        ForkError::Canonicalization(e.to_string())
    })?;

    vk.verify(&canonical, &attestation.signature).map_err(|e| {
        ForkError::InvalidAttestation {
            watcher_key_hex: attestation.watcher_key.clone(),
            reason: format!("signature verification failed: {e}"),
        }
    })
}

/// Count verified attestations for a branch, deduplicating by watcher key.
///
/// Returns the count of unique, valid attestations.
fn count_verified_attestations(
    branch: &ForkBranch,
    registry: &WatcherRegistry,
) -> usize {
    let mut seen_watchers = BTreeSet::new();
    let mut count = 0;

    for attestation in &branch.attestations {
        // Skip duplicate attestations from the same watcher.
        if !seen_watchers.insert(&attestation.watcher_key) {
            continue;
        }
        // Only count valid attestations.
        if verify_attestation(attestation, &branch.next_root, registry).is_ok() {
            count += 1;
        }
    }

    count
}

/// Resolve a fork between two competing branches using three-level ordering
/// with cryptographic attestation verification.
///
/// ## Three-Level Ordering Protocol (P0-FORK-001 Remediated)
///
/// 1. **Primary — Timestamp:** If the absolute time difference between
///    the two branches exceeds [`MAX_CLOCK_SKEW`] (5 minutes), the
///    earlier-timestamped branch wins. Both timestamps must be within
///    the monotonic bound (`now + MAX_FUTURE_DRIFT`).
///
/// 2. **Secondary — Verified Attestations:** If timestamps are within
///    the skew tolerance, the branch with more **cryptographically
///    verified** watcher attestations wins. Each attestation signature
///    is checked against the watcher registry.
///
/// 3. **Tertiary — Lexicographic Digest:** If both timestamps and
///    verified attestation counts are equal, the branch with the
///    lexicographically smaller `next_root` digest wins.
pub fn resolve_fork(
    branch_a: &ForkBranch,
    branch_b: &ForkBranch,
    registry: &WatcherRegistry,
) -> Result<ForkResolution, ForkError> {
    let now = Utc::now();
    let future_bound = chrono::Duration::seconds(MAX_FUTURE_DRIFT.as_secs() as i64);
    let past_bound = chrono::Duration::seconds(MAX_PAST_AGE.as_secs() as i64);

    // Reject branches with timestamps too far in the future.
    if branch_a.timestamp > now + future_bound {
        return Err(ForkError::FutureTimestamp {
            timestamp: branch_a.timestamp,
            now,
            max_drift_secs: MAX_FUTURE_DRIFT.as_secs(),
        });
    }
    if branch_b.timestamp > now + future_bound {
        return Err(ForkError::FutureTimestamp {
            timestamp: branch_b.timestamp,
            now,
            max_drift_secs: MAX_FUTURE_DRIFT.as_secs(),
        });
    }

    // Reject branches with timestamps too far in the past (backdating attack).
    if branch_a.timestamp < now - past_bound {
        return Err(ForkError::PastTimestamp {
            timestamp: branch_a.timestamp,
            now,
            max_age_secs: MAX_PAST_AGE.as_secs(),
        });
    }
    if branch_b.timestamp < now - past_bound {
        return Err(ForkError::PastTimestamp {
            timestamp: branch_b.timestamp,
            now,
            max_age_secs: MAX_PAST_AGE.as_secs(),
        });
    }

    // Detect watcher equivocation: same watcher attesting for both branches.
    let equivocators = ForkDetector::detect_equivocation(branch_a, branch_b);
    if !equivocators.is_empty() {
        return Err(ForkError::EquivocationDetected {
            equivocating_watchers: equivocators,
        });
    }

    // Count verified attestations for each branch.
    let count_a = count_verified_attestations(branch_a, registry);
    let count_b = count_verified_attestations(branch_b, registry);

    let time_diff = if branch_a.timestamp >= branch_b.timestamp {
        branch_a.timestamp - branch_b.timestamp
    } else {
        branch_b.timestamp - branch_a.timestamp
    };

    let skew_tolerance = chrono::Duration::seconds(MAX_CLOCK_SKEW.as_secs() as i64);

    // Level 1: Timestamp ordering (only if outside skew tolerance).
    if time_diff > skew_tolerance {
        let (winner, loser, w_count, l_count) = if branch_a.timestamp < branch_b.timestamp {
            (branch_a, branch_b, count_a, count_b)
        } else {
            (branch_b, branch_a, count_b, count_a)
        };
        return Ok(ForkResolution {
            winning_branch: winner.receipt_digest.clone(),
            losing_branch: loser.receipt_digest.clone(),
            resolution_reason: ResolutionReason::EarlierTimestamp,
            winning_attestation_count: w_count,
            losing_attestation_count: l_count,
        });
    }

    // Level 2: Verified watcher attestation count (more attestations wins).
    if count_a != count_b {
        let (winner, loser, w_count, l_count) = if count_a > count_b {
            (branch_a, branch_b, count_a, count_b)
        } else {
            (branch_b, branch_a, count_b, count_a)
        };
        return Ok(ForkResolution {
            winning_branch: winner.receipt_digest.clone(),
            losing_branch: loser.receipt_digest.clone(),
            resolution_reason: ResolutionReason::MoreAttestations,
            winning_attestation_count: w_count,
            losing_attestation_count: l_count,
        });
    }

    // Level 3: Lexicographic ordering of next_root digest (deterministic tiebreaker).
    let (winner, loser, w_count, l_count) = if branch_a.next_root <= branch_b.next_root {
        (branch_a, branch_b, count_a, count_b)
    } else {
        (branch_b, branch_a, count_b, count_a)
    };
    Ok(ForkResolution {
        winning_branch: winner.receipt_digest.clone(),
        losing_branch: loser.receipt_digest.clone(),
        resolution_reason: ResolutionReason::LexicographicTiebreak,
        winning_attestation_count: w_count,
        losing_attestation_count: l_count,
    })
}

/// Create a signed watcher attestation for a branch.
///
/// This is a helper for watchers to produce attestations that
/// can be included in a [`ForkBranch`].
pub fn create_attestation(
    signing_key: &msez_crypto::ed25519::SigningKey,
    parent_root: &str,
    candidate_root: &str,
    height: u64,
    timestamp: DateTime<Utc>,
) -> Result<WatcherAttestation, ForkError> {
    let watcher_key = signing_key.verifying_key().to_hex();

    let payload = serde_json::json!({
        "candidate_root": candidate_root,
        "height": height,
        "parent_root": parent_root,
        "timestamp": timestamp.to_rfc3339(),
        "watcher_key": watcher_key,
    });
    let canonical = CanonicalBytes::from_value(payload).map_err(|e| {
        ForkError::Canonicalization(e.to_string())
    })?;

    let signature = signing_key.sign(&canonical);

    Ok(WatcherAttestation {
        watcher_key,
        parent_root: parent_root.to_string(),
        candidate_root: candidate_root.to_string(),
        height,
        timestamp,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use msez_core::{sha256_digest, CanonicalBytes};
    use msez_crypto::ed25519::SigningKey;
    use rand_core::OsRng;

    fn make_digest(label: &str) -> ContentDigest {
        let canonical = CanonicalBytes::new(&serde_json::json!({"branch": label})).unwrap();
        sha256_digest(&canonical)
    }

    fn make_watcher_key() -> (SigningKey, VerifyingKey) {
        let sk = SigningKey::generate(&mut OsRng);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn make_registry(keys: &[&VerifyingKey]) -> WatcherRegistry {
        let mut registry = WatcherRegistry::new();
        for key in keys {
            registry.register((*key).clone());
        }
        registry
    }

    fn make_attestation(
        sk: &SigningKey,
        parent_root: &str,
        candidate_root: &str,
        height: u64,
        timestamp: DateTime<Utc>,
    ) -> WatcherAttestation {
        create_attestation(sk, parent_root, candidate_root, height, timestamp).unwrap()
    }

    fn make_branch(
        label: &str,
        timestamp: DateTime<Utc>,
        attestations: Vec<WatcherAttestation>,
        next_root: &str,
    ) -> ForkBranch {
        ForkBranch {
            receipt_digest: make_digest(label),
            timestamp,
            attestations,
            next_root: next_root.to_string(),
        }
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    // -- Level 1: Timestamp ordering beyond skew tolerance --

    #[test]
    fn earlier_timestamp_wins_beyond_skew() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t2 = now();
        let t1 = t2 - chrono::Duration::minutes(10);
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t1);
        let att_b = make_attestation(&sk2, "parent", &next_root_b, 1, t2);

        let branch_a = make_branch("A", t1, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t2, vec![att_b], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        assert_eq!(resolution.winning_branch, branch_a.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::EarlierTimestamp);
    }

    // -- Level 2: Verified attestation count within skew tolerance --

    #[test]
    fn more_verified_attestations_wins_within_skew() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let (sk3, vk3) = make_watcher_key();
        let (sk4, vk4) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2, &vk3, &vk4]);

        let t2 = now();
        let t1 = t2 - chrono::Duration::minutes(3);
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        // Branch A: 1 attestation (sk1)
        let att_a1 = make_attestation(&sk1, "parent", &next_root_a, 1, t1);
        // Branch B: 3 attestations (sk2, sk3, sk4 — no overlap with A)
        let att_b1 = make_attestation(&sk2, "parent", &next_root_b, 1, t2);
        let att_b2 = make_attestation(&sk3, "parent", &next_root_b, 1, t2);
        let att_b3 = make_attestation(&sk4, "parent", &next_root_b, 1, t2);

        let branch_a = make_branch("A", t1, vec![att_a1], &next_root_a);
        let branch_b = make_branch("B", t2, vec![att_b1, att_b2, att_b3], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        assert_eq!(resolution.winning_branch, branch_b.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
        assert_eq!(resolution.winning_attestation_count, 3);
        assert_eq!(resolution.losing_attestation_count, 1);
    }

    // -- Level 3: Lexicographic tiebreaker --

    #[test]
    fn lexicographic_tiebreak_when_all_equal() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        let att_b = make_attestation(&sk2, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        assert_eq!(resolution.winning_branch, branch_a.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
    }

    // -- Adversarial: attacker with inflated attestation count --

    #[test]
    fn unregistered_watcher_attestations_ignored() {
        let (sk1, vk1) = make_watcher_key();
        let (sk_rogue, _vk_rogue) = make_watcher_key();
        // Only sk1 is registered
        let registry = make_registry(&[&vk1]);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        // Branch A: 1 valid attestation
        let att_a1 = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        // Branch B: rogue attestation (unregistered key)
        let att_b1 = make_attestation(&sk_rogue, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a1], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b1], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        // Branch A wins by attestation count (1 valid vs 0 valid)
        assert_eq!(resolution.winning_branch, branch_a.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
        assert_eq!(resolution.winning_attestation_count, 1);
        assert_eq!(resolution.losing_attestation_count, 0);
    }

    // -- Adversarial: forged signature --

    #[test]
    fn forged_attestation_signature_rejected() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let (sk3, _vk3) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        let att_a1 = make_attestation(&sk1, "parent", &next_root_a, 1, t);

        // Create attestation signed by sk3 but claiming to be from vk2
        let mut forged = make_attestation(&sk3, "parent", &next_root_b, 1, t);
        forged.watcher_key = vk2.to_hex(); // Claim to be registered watcher vk2

        let branch_a = make_branch("A", t, vec![att_a1], &next_root_a);
        let branch_b = make_branch("B", t, vec![forged], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        // Forged attestation fails signature check, so A wins
        assert_eq!(resolution.winning_branch, branch_a.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
    }

    // -- Adversarial: duplicate attestations from same watcher --

    #[test]
    fn duplicate_attestations_deduplicated() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t = now();
        let next_root_b = "bb".repeat(32);
        let next_root_a = "aa".repeat(32);

        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t);

        // Same watcher (sk2) attesting twice for same branch — should dedup to 1
        let att_b1 = make_attestation(&sk2, "parent", &next_root_b, 1, t);
        let att_b2 = make_attestation(&sk2, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b1, att_b2], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        // Both branches have 1 unique attestation, falls to tiebreaker
        assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
    }

    // -- Future timestamp rejection --

    #[test]
    fn future_timestamp_beyond_drift_rejected() {
        let registry = WatcherRegistry::new();

        let t_future = now() + chrono::Duration::minutes(5);
        let t_normal = now();

        let branch_a = make_branch("A", t_future, vec![], &"aa".repeat(32));
        let branch_b = make_branch("B", t_normal, vec![], &"bb".repeat(32));

        let result = resolve_fork(&branch_a, &branch_b, &registry);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ForkError::FutureTimestamp { .. }));
    }

    // -- Watcher equivocation detection --

    #[test]
    fn equivocation_detected_by_helper() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, _vk2) = make_watcher_key();

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        // sk1 attests for both branches = equivocation
        let att_a1 = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        let att_a2 = make_attestation(&sk2, "parent", &next_root_a, 1, t);
        let att_b1 = make_attestation(&sk1, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a1, att_a2], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b1], &next_root_b);

        let equivocators = ForkDetector::detect_equivocation(&branch_a, &branch_b);
        assert_eq!(equivocators.len(), 1);
        assert_eq!(equivocators[0], vk1.to_hex());
    }

    // -- Adversarial: equivocation blocks resolve_fork --

    #[test]
    fn equivocation_blocks_fork_resolution() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        // sk1 attests for both branches = equivocation
        let att_a1 = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        let att_a2 = make_attestation(&sk2, "parent", &next_root_a, 1, t);
        let att_b1 = make_attestation(&sk1, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a1, att_a2], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b1], &next_root_b);

        let result = resolve_fork(&branch_a, &branch_b, &registry);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ForkError::EquivocationDetected { .. }),
            "equivocating watcher must block fork resolution"
        );
    }

    // -- Adversarial: backdated timestamp rejected --

    #[test]
    fn past_timestamp_beyond_age_rejected() {
        let registry = WatcherRegistry::new();

        // Branch with epoch timestamp — extreme backdating attack
        let t_epoch = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
        let t_normal = now();

        let branch_a = make_branch("A", t_epoch, vec![], &"aa".repeat(32));
        let branch_b = make_branch("B", t_normal, vec![], &"bb".repeat(32));

        let result = resolve_fork(&branch_a, &branch_b, &registry);
        assert!(result.is_err());
        assert!(
            matches!(result.unwrap_err(), ForkError::PastTimestamp { .. }),
            "epoch-backdated branch must be rejected"
        );
    }

    // -- ForkDetector lifecycle --

    #[test]
    fn fork_detector_lifecycle() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);
        let mut detector = ForkDetector::new(registry);
        assert_eq!(detector.pending_count(), 0);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);
        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        let att_b = make_attestation(&sk2, "parent", &next_root_b, 1, t);

        let branch_a = make_branch("A", t, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b], &next_root_b);

        assert!(ForkDetector::is_fork(&branch_a, &branch_b));
        detector.register_fork(branch_a, branch_b);
        assert_eq!(detector.pending_count(), 1);

        let resolutions = detector.resolve_all();
        assert_eq!(resolutions.len(), 1);
        assert!(resolutions[0].is_ok());
        assert_eq!(detector.pending_count(), 0);
    }

    // -- WatcherRegistry --

    #[test]
    fn watcher_registry_operations() {
        let (_sk, vk) = make_watcher_key();
        let mut registry = WatcherRegistry::new();
        assert_eq!(registry.count(), 0);
        assert!(!registry.is_registered(&vk.to_hex()));

        registry.register(vk.clone());
        assert_eq!(registry.count(), 1);
        assert!(registry.is_registered(&vk.to_hex()));
        assert!(registry.get(&vk.to_hex()).is_some());
    }

    // -- Boundary: exactly at skew boundary --

    #[test]
    fn exactly_at_skew_boundary_falls_to_secondary() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let (sk3, vk3) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2, &vk3]);

        let t2 = now();
        let t1 = t2 - chrono::Duration::seconds(300); // Exactly 5 minutes
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t1);
        let att_b1 = make_attestation(&sk2, "parent", &next_root_b, 1, t2);
        let att_b2 = make_attestation(&sk3, "parent", &next_root_b, 1, t2);

        let branch_a = make_branch("A", t1, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t2, vec![att_b1, att_b2], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        // Exactly at boundary: falls to secondary
        assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
    }

    // -- No attestations on either branch --

    #[test]
    fn no_attestations_falls_to_tiebreaker() {
        let registry = WatcherRegistry::new();
        let t = now();

        let branch_a = make_branch("A", t, vec![], &"aa".repeat(32));
        let branch_b = make_branch("B", t, vec![], &"bb".repeat(32));

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
    }

    // -- Attestation with wrong candidate_root --

    #[test]
    fn attestation_for_wrong_candidate_root_rejected() {
        let (sk1, vk1) = make_watcher_key();
        let (sk2, vk2) = make_watcher_key();
        let registry = make_registry(&[&vk1, &vk2]);

        let t = now();
        let next_root_a = "aa".repeat(32);
        let next_root_b = "bb".repeat(32);

        let att_a = make_attestation(&sk1, "parent", &next_root_a, 1, t);
        // Attestation signed for next_root_a but attached to branch_b with next_root_b
        let att_b_wrong = make_attestation(&sk2, "parent", &next_root_a, 1, t);

        let branch_a = make_branch("A", t, vec![att_a], &next_root_a);
        let branch_b = make_branch("B", t, vec![att_b_wrong], &next_root_b);

        let resolution = resolve_fork(&branch_a, &branch_b, &registry).unwrap();
        // Branch B's attestation has wrong candidate_root, so 0 valid
        assert_eq!(resolution.winning_branch, branch_a.receipt_digest);
        assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
    }

    // -- Identical branches not a fork --

    #[test]
    fn identical_branches_not_a_fork() {
        let digest = make_digest("same");
        let branch = ForkBranch {
            receipt_digest: digest,
            timestamp: now(),
            attestations: vec![],
            next_root: "aa".repeat(32),
        };
        assert!(!ForkDetector::is_fork(&branch, &branch));
    }
}
