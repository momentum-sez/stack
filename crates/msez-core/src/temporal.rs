//! # Temporal Types — UTC-Only Timestamps
//!
//! Defines `Timestamp`, a UTC-only timestamp type that enforces the
//! canonicalization requirement of ISO8601 with Z suffix, truncated
//! to seconds precision.
//!
//! ## Security Invariant
//!
//! Timestamps in the SEZ Stack must be UTC with Z suffix for deterministic
//! canonicalization. Local timezone offsets would produce different canonical
//! byte sequences for the same instant, breaking content-addressed integrity.
//!
//! ## Implements
//!
//! Spec §8 — Temporal normalization rules for JCS canonicalization.

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// A UTC-only timestamp, truncated to seconds precision.
///
/// This type guarantees that all timestamps in the system are in UTC
/// with no sub-second components, matching the JCS canonicalization
/// rule that normalizes datetimes to `YYYY-MM-DDTHH:MM:SSZ`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp from the current UTC time, truncated to seconds.
    pub fn now() -> Self {
        let now = Utc::now();
        // Truncate sub-second precision.
        Self(now.with_nanosecond(0).unwrap_or(now))
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`, truncating sub-seconds.
    pub fn from_utc(dt: DateTime<Utc>) -> Self {
        Self(dt.with_nanosecond(0).unwrap_or(dt))
    }

    /// Access the inner `DateTime<Utc>`.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Render as ISO8601 with Z suffix (e.g., `2026-01-15T12:00:00Z`).
    pub fn to_iso8601(&self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_iso8601())
    }
}
