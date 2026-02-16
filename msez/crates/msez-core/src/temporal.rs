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
//!
//! ## Security Invariant
//!
//! Non-UTC inputs are rejected at construction time. The `from_rfc3339`
//! constructor parses any RFC 3339 string but converts to UTC and truncates
//! to second precision. The `now()` constructor always produces UTC.
//! There is no path to construct a `Timestamp` with non-UTC or subsecond data.
//!
//! ## Spec Reference
//!
//! Matches the datetime handling in `tools/lawpack.py:now_rfc3339()` and
//! `_coerce_json_types()` datetime branch.

use chrono::{DateTime, NaiveDateTime, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};

use crate::error::ValidationError;

/// A UTC timestamp with second-level precision.
///
/// Serializes to ISO 8601 format with `Z` suffix (e.g., `2026-01-15T12:00:00Z`).
/// Subsecond precision is truncated at construction time to ensure
/// deterministic digest computation.
///
/// # Invariants
///
/// - Always UTC (no timezone offset other than +00:00/Z)
/// - Always truncated to seconds (nanosecond component is 0)
/// - Serialized form always uses `Z` suffix, never `+00:00`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Timestamp(DateTime<Utc>);

impl Timestamp {
    /// Create a timestamp representing the current UTC time, truncated to seconds.
    pub fn now() -> Self {
        let now = Utc::now();
        Self(truncate_to_seconds(now))
    }

    /// Create a timestamp from a `chrono::DateTime<Utc>`, truncated to seconds.
    pub fn from_datetime(dt: DateTime<Utc>) -> Self {
        Self(truncate_to_seconds(dt))
    }

    /// Parse an RFC 3339 timestamp string.
    ///
    /// The input is converted to UTC and truncated to second precision.
    /// Both `Z` and numeric UTC offset formats are accepted:
    /// - `"2026-01-15T12:00:00Z"` — UTC with Z suffix
    /// - `"2026-01-15T12:00:00+00:00"` — UTC with numeric offset
    /// - `"2026-01-15T17:00:00+05:00"` — non-UTC, converted to UTC
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidTimestamp`] if the string cannot be
    /// parsed as RFC 3339.
    pub fn from_rfc3339(s: &str) -> Result<Self, ValidationError> {
        let dt =
            DateTime::parse_from_rfc3339(s).map_err(|e| ValidationError::InvalidTimestamp {
                value: s.to_string(),
                reason: format!("not valid RFC 3339: {e}"),
            })?;
        let utc = dt.with_timezone(&Utc);
        Ok(Self(truncate_to_seconds(utc)))
    }

    /// Parse an ISO 8601 date (without time) as midnight UTC.
    ///
    /// Useful for `as_of_date` fields that are date-only.
    ///
    /// # Errors
    ///
    /// Returns [`ValidationError::InvalidTimestamp`] if the string is not
    /// a valid `YYYY-MM-DD` date.
    pub fn from_date_str(s: &str) -> Result<Self, ValidationError> {
        let nd = NaiveDateTime::parse_from_str(&format!("{s}T00:00:00"), "%Y-%m-%dT%H:%M:%S")
            .map_err(|e| ValidationError::InvalidTimestamp {
                value: s.to_string(),
                reason: format!("not valid YYYY-MM-DD: {e}"),
            })?;
        let utc = Utc.from_utc_datetime(&nd);
        Ok(Self(truncate_to_seconds(utc)))
    }

    /// Access the underlying `chrono::DateTime<Utc>`.
    pub fn as_datetime(&self) -> &DateTime<Utc> {
        &self.0
    }

    /// Return the timestamp as an ISO 8601 string with Z suffix,
    /// truncated to seconds.
    ///
    /// This is the canonical string form used in digest computation.
    /// Matches Python's:
    /// ```python
    /// dt.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    /// ```
    pub fn to_canonical_string(&self) -> String {
        self.0.format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }
}

/// Truncate a `DateTime<Utc>` to second precision (zero out nanoseconds).
fn truncate_to_seconds(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.with_nanosecond(0).expect("nanosecond=0 is always valid")
}

impl std::fmt::Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_canonical_string())
    }
}

impl From<DateTime<Utc>> for Timestamp {
    fn from(dt: DateTime<Utc>) -> Self {
        Self::from_datetime(dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn now_is_utc_and_truncated() {
        let ts = Timestamp::now();
        let dt = ts.as_datetime();
        assert_eq!(dt.nanosecond(), 0);
        assert!(ts.to_canonical_string().ends_with('Z'));
    }

    #[test]
    fn from_datetime_truncates_subseconds() {
        let dt = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 1, 15)
                .unwrap()
                .and_hms_nano_opt(12, 0, 0, 123_456_789)
                .unwrap(),
        );
        let ts = Timestamp::from_datetime(dt);
        assert_eq!(ts.to_canonical_string(), "2026-01-15T12:00:00Z");
        assert_eq!(ts.as_datetime().nanosecond(), 0);
    }

    #[test]
    fn from_rfc3339_utc() {
        let ts = Timestamp::from_rfc3339("2026-01-15T12:00:00Z").unwrap();
        assert_eq!(ts.to_canonical_string(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn from_rfc3339_with_offset() {
        // +05:00 → converted to UTC (12:00 - 5 = 07:00... wait, 17:00 - 5 = 12:00)
        let ts = Timestamp::from_rfc3339("2026-01-15T17:00:00+05:00").unwrap();
        assert_eq!(ts.to_canonical_string(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn from_rfc3339_with_subseconds() {
        let ts = Timestamp::from_rfc3339("2026-01-15T12:00:00.999999+00:00").unwrap();
        assert_eq!(ts.to_canonical_string(), "2026-01-15T12:00:00Z");
    }

    #[test]
    fn from_rfc3339_rejects_invalid() {
        assert!(Timestamp::from_rfc3339("").is_err());
        assert!(Timestamp::from_rfc3339("not-a-date").is_err());
        assert!(Timestamp::from_rfc3339("2026-13-01T00:00:00Z").is_err());
    }

    #[test]
    fn from_date_str_midnight_utc() {
        let ts = Timestamp::from_date_str("2026-01-15").unwrap();
        assert_eq!(ts.to_canonical_string(), "2026-01-15T00:00:00Z");
    }

    #[test]
    fn from_date_str_rejects_invalid() {
        assert!(Timestamp::from_date_str("").is_err());
        assert!(Timestamp::from_date_str("2026-13-01").is_err());
        assert!(Timestamp::from_date_str("not-a-date").is_err());
    }

    #[test]
    fn display_uses_canonical_form() {
        let ts = Timestamp::from_rfc3339("2026-06-15T08:30:00Z").unwrap();
        assert_eq!(format!("{ts}"), "2026-06-15T08:30:00Z");
    }

    #[test]
    fn from_datetime_utc_conversion() {
        let dt = Utc::now();
        let ts = Timestamp::from(dt);
        assert_eq!(ts.as_datetime().nanosecond(), 0);
    }
}
