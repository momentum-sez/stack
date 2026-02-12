//! # Fork Detection & Resolution
//!
//! Detects and resolves forks in corridor receipt chains using a
//! three-tier ordering:
//!
//! 1. Primary: timestamp (existing branch wins).
//! 2. Secondary: watcher attestation count (more attestations win).
//! 3. Tertiary: lexicographic ordering of branch `next_root` digest
//!    (deterministic tiebreaker).
//!
//! Maximum clock skew tolerance: 5 minutes. Branches with timestamps
//! more than 5 minutes in the future are rejected.
//!
//! ## Security Invariant
//!
//! Fork resolution must be deterministic — any two nodes evaluating
//! the same fork data must arrive at the same resolution.
//!
//! ## Implements
//!
//! Spec §40 — Protocol 16.1 fork resolution.

/// A fork resolver for corridor receipt chains.
///
/// Placeholder — full implementation will compare branch candidates
/// using the three-tier ordering criteria.
#[derive(Debug)]
pub struct ForkResolver {
    /// Maximum allowed clock skew in seconds (default: 300 = 5 minutes).
    pub max_clock_skew_seconds: u64,
}

impl ForkResolver {
    /// Create a new fork resolver with default clock skew tolerance.
    pub fn new() -> Self {
        Self {
            max_clock_skew_seconds: 300,
        }
    }
}

impl Default for ForkResolver {
    fn default() -> Self {
        Self::new()
    }
}
