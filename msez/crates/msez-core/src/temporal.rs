//! # Temporal Types
//!
//! UTC-only timestamp type for the SEZ Stack. All timestamps are stored
//! in UTC with second-level precision and a `Z` suffix in serialized form.
//!
//! ## Design Decision
//!
//! The SEZ Stack operates across jurisdictions with different local time
//! zones. To prevent ambiguity in corridor receipts, state transitions,
//! and audit trails, all timestamps are UTC. Local time conversion is
//! a presentation concern handled at the API layer.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A UTC timestamp with second-level precision.
///
/// Serializes to ISO 8601 format with `Z` suffix (e.g., `2026-01-15T12:00:00Z`).
/// Subsecond precision is truncated during canonicalization to ensure
/// deterministic digest computation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp representing the current UTC time.
    pub fn now() -> Self {
        Self(Utc::now())
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`.
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }

    /// Access the underlying `chrono::DateTime<Utc>`.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Return the timestamp as an ISO 8601 string with Z suffix,
    /// truncated to seconds (matching canonicalization rules).
    pub fn to_canonical_string(&self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_canonical_string())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self(dt)
    }
}
