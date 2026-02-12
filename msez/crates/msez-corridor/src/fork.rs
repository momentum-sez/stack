//! # Fork Detection and Resolution
//!
//! Detects and resolves forks in the corridor receipt chain using
//! three-level ordering:
//!
//! 1. **Primary:** Timestamp (earlier wins).
//! 2. **Secondary:** Watcher attestation count (more independent attestations wins).
//! 3. **Tertiary:** Lexicographic ordering of `next_root` digest (deterministic tiebreaker).
//!
//! Maximum clock skew tolerance: reject branches with timestamps more
//! than 5 minutes in the future.
//!
//! ## Audit Reference
//!
//! Finding §3.5: The Python implementation used only timestamp ordering,
//! allowing an attacker with backdated timestamps to always win fork resolution.

use std::time::Duration;

use msez_core::ContentDigest;

/// Maximum allowed clock skew for fork resolution timestamps.
pub const MAX_CLOCK_SKEW: Duration = Duration::from_secs(5 * 60);

/// The result of resolving a fork in the corridor receipt chain.
#[derive(Debug)]
pub struct ForkResolution {
    /// The digest of the winning branch.
    pub winning_branch: ContentDigest,
    /// The reason the winning branch was selected.
    pub resolution_reason: ResolutionReason,
}

/// Why a particular branch won fork resolution.
#[derive(Debug)]
pub enum ResolutionReason {
    /// Won by earlier timestamp (primary ordering).
    EarlierTimestamp,
    /// Won by more watcher attestations (secondary ordering).
    MoreAttestations,
    /// Won by lexicographic digest ordering (tertiary tiebreaker).
    LexicographicTiebreak,
}
