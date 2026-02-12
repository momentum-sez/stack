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
//! Non-UTC inputs are **rejected at construction** — there is no silent
//! conversion that could introduce ambiguity.
//!
//! ## Cross-Language Compatibility
//!
//! The `to_iso8601()` output matches the Python equivalent in
//! `tools/lawpack.py:_coerce_json_types()`:
//!
//! ```python
//! obj.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
//! ```
//!
//! Both produce: `YYYY-MM-DDTHH:MM:SSZ` — no sub-seconds, no `+00:00`, always `Z`.
//!
//! ## Implements
//!
//! Spec §8 — Temporal normalization rules for JCS canonicalization.

use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::error::MsezError;

/// A UTC-only timestamp, truncated to seconds precision.
///
/// This type guarantees that all timestamps in the system are in UTC
/// with no sub-second components, matching the JCS canonicalization
/// rule that normalizes datetimes to `YYYY-MM-DDTHH:MM:SSZ`.
///
/// # Construction
///
/// - [`Timestamp::now()`] — current UTC time, truncated.
/// - [`Timestamp::from_utc()`] — from a `DateTime<Utc>`, truncating sub-seconds.
/// - [`Timestamp::parse()`] — from an ISO8601 string, rejecting non-UTC offsets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp from the current UTC time, truncated to seconds.
    pub fn now() -> Self {
        let now = Utc::now();
        Self(truncate_to_seconds(now))
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`, truncating sub-seconds.
    pub fn from_utc(dt: DateTime<Utc>) -> Self {
        Self(truncate_to_seconds(dt))
    }

    /// Parse a timestamp from an RFC 3339 / ISO8601 string.
    ///
    /// **Rejects non-UTC inputs.** Only timestamps with the `Z` suffix are
    /// accepted. Timestamps with explicit offsets like `+00:00`, `+05:30`,
    /// or `-04:00` are rejected — even `+00:00` which is semantically
    /// equivalent to `Z`. This strict policy ensures that canonical byte
    /// representations are deterministic.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The string is not valid RFC 3339.
    /// - The string uses a non-Z timezone offset.
    pub fn parse(s: &str) -> Result<Self, MsezError> {
        if !s.ends_with('Z') {
            return Err(MsezError::SchemaValidation(format!(
                "Timestamp must use Z suffix (UTC only), got: {s:?}"
            )));
        }

        let dt = DateTime::parse_from_rfc3339(s)
            .map_err(|e| MsezError::SchemaValidation(format!(
                "Invalid RFC 3339 timestamp {s:?}: {e}"
            )))?;

        Ok(Self(truncate_to_seconds(dt.with_timezone(&Utc))))
    }

    /// Parse a timestamp from an RFC 3339 string, accepting any timezone
    /// offset and converting to UTC.
    ///
    /// This is a lenient parser for ingesting external data. The result
    /// is always UTC with seconds precision, matching the strict invariant.
    ///
    /// For digest computation paths, prefer [`Timestamp::parse()`] which
    /// rejects non-UTC inputs.
    pub fn parse_lenient(s: &str) -> Result<Self, MsezError> {
        let dt = DateTime::parse_from_rfc3339(s)
            .map_err(|e| MsezError::SchemaValidation(format!(
                "Invalid RFC 3339 timestamp {s:?}: {e}"
            )))?;
        Ok(Self(truncate_to_seconds(dt.with_timezone(&Utc))))
    }

    /// Create a timestamp from a Unix epoch timestamp (seconds).
    pub fn from_epoch_secs(secs: i64) -> Result<Self, MsezError> {
        let dt = DateTime::from_timestamp(secs, 0)
            .ok_or_else(|| MsezError::SchemaValidation(format!(
                "Invalid Unix timestamp: {secs}"
            )))?;
        Ok(Self(dt))
    }

    /// Access the inner `DateTime<Utc>`.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Returns the Unix epoch timestamp in seconds.
    pub fn epoch_secs(&self) -> i64 {
        self.0.timestamp()
    }

    /// Render as ISO8601 with Z suffix (e.g., `2026-01-15T12:00:00Z`).
    ///
    /// This output format matches the Python canonicalization in
    /// `tools/lawpack.py:_coerce_json_types()`.
    pub fn to_iso8601(&self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_iso8601())
    }
}

/// Truncate a `DateTime<Utc>` to seconds precision (discard nanoseconds).
fn truncate_to_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).unwrap_or(dt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_now_has_no_subseconds() {
        let ts = Timestamp::now();
        assert_eq!(ts.as_datetime().nanosecond(), 0);
    }

    #[test]
    fn test_from_utc_truncates() {
        let dt = Utc.with_ymd_and_hms(2026, 1, 15, 12, 30, 45).unwrap();
        let dt_with_nanos = dt.with_nanosecond(123_456_789).unwrap();
        let ts = Timestamp::from_utc(dt_with_nanos);
        assert_eq!(ts.as_datetime().nanosecond(), 0);
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:30:45Z");
    }

    #[test]
    fn test_to_iso8601_format() {
        let dt = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        let ts = Timestamp::from_utc(dt);
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn test_display_matches_iso8601() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 30, 23, 59, 59).unwrap();
        let ts = Timestamp::from_utc(dt);
        assert_eq!(format!("{ts}"), ts.to_iso8601());
    }

    // ---- parse() strict mode ----

    #[test]
    fn test_parse_z_suffix_accepted() {
        let ts = Timestamp::parse("2026-01-15T12:00:00Z").unwrap();
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn test_parse_plus_zero_rejected() {
        assert!(Timestamp::parse("2026-01-15T12:00:00+00:00").is_err());
    }

    #[test]
    fn test_parse_positive_offset_rejected() {
        assert!(Timestamp::parse("2026-01-15T17:00:00+05:00").is_err());
    }

    #[test]
    fn test_parse_negative_offset_rejected() {
        assert!(Timestamp::parse("2026-01-15T08:00:00-04:00").is_err());
    }

    #[test]
    fn test_parse_subseconds_truncated() {
        let ts = Timestamp::parse("2026-01-15T12:00:00.123456Z").unwrap();
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
        assert_eq!(ts.as_datetime().nanosecond(), 0);
    }

    #[test]
    fn test_parse_invalid_format() {
        assert!(Timestamp::parse("not-a-date").is_err());
        assert!(Timestamp::parse("2026-01-15").is_err());
        assert!(Timestamp::parse("").is_err());
    }

    // ---- parse_lenient() ----

    #[test]
    fn test_parse_lenient_converts_offset() {
        let ts = Timestamp::parse_lenient("2026-01-15T17:00:00+05:00").unwrap();
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn test_parse_lenient_accepts_z() {
        let ts = Timestamp::parse_lenient("2026-01-15T12:00:00Z").unwrap();
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
    }

    // ---- epoch ----

    #[test]
    fn test_epoch_roundtrip() {
        let ts = Timestamp::parse("2026-01-15T12:00:00Z").unwrap();
        let secs = ts.epoch_secs();
        let ts2 = Timestamp::from_epoch_secs(secs).unwrap();
        assert_eq!(ts, ts2);
    }

    // ---- ordering ----

    #[test]
    fn test_ordering() {
        let earlier = Timestamp::parse("2026-01-15T12:00:00Z").unwrap();
        let later = Timestamp::parse("2026-01-15T12:00:01Z").unwrap();
        assert!(earlier < later);
    }

    // ---- serde ----

    #[test]
    fn test_serde_roundtrip() {
        let ts = Timestamp::parse("2026-01-15T12:00:00Z").unwrap();
        let json = serde_json::to_string(&ts).unwrap();
        let parsed: Timestamp = serde_json::from_str(&json).unwrap();
        assert_eq!(ts, parsed);
    }

    // ---- cross-language format compatibility ----

    #[test]
    fn test_format_matches_python_lawpack() {
        // Python: datetime(2026, 1, 15, 12, 0, 0, tzinfo=timezone.utc)
        //   .replace(microsecond=0).isoformat().replace("+00:00", "Z")
        // = "2026-01-15T12:00:00Z"
        let dt = Utc.with_ymd_and_hms(2026, 1, 15, 12, 0, 0).unwrap();
        let ts = Timestamp::from_utc(dt);
        assert_eq!(ts.to_iso8601(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn test_midnight_format() {
        let dt = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let ts = Timestamp::from_utc(dt);
        assert_eq!(ts.to_iso8601(), "2026-01-01T00:00:00Z");
    }
}
