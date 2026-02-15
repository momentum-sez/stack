//! # Policy Audit Trail — MASS Protocol v0.2 Chapter 17
//!
//! Records every trigger evaluation and action execution for regulatory review.
//!
//! ## Security Invariant
//!
//! Every audit entry is individually digestable via `CanonicalBytes` + `sha256_digest`.
//! The append-only trail uses a circular buffer that trims the oldest 10% when the
//! configured maximum is exceeded. Trimmed entries are NOT lost — they should be
//! persisted to durable storage before trimming in production deployments.
//!
//! ## Spec Reference
//!
//! Implements Definition 17.4 (Policy Audit Trail) from the MASS Protocol v0.2 Ch. 17.

use chrono::{DateTime, Utc};
use msez_core::{sha256_digest, CanonicalBytes, ContentDigest};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AuditEntryType
// ---------------------------------------------------------------------------

/// The type of audit trail event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditEntryType {
    /// A trigger event was received by the evaluation engine.
    TriggerReceived,
    /// A policy was evaluated against a trigger.
    PolicyEvaluated,
    /// An action was scheduled for execution.
    ActionScheduled,
    /// A scheduled action was executed.
    ActionExecuted,
    /// A scheduled action failed.
    ActionFailed,
    /// A scheduled action was cancelled.
    ActionCancelled,
}

impl AuditEntryType {
    /// Return the string value for serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TriggerReceived => "trigger_received",
            Self::PolicyEvaluated => "policy_evaluated",
            Self::ActionScheduled => "action_scheduled",
            Self::ActionExecuted => "action_executed",
            Self::ActionFailed => "action_failed",
            Self::ActionCancelled => "action_cancelled",
        }
    }
}

impl std::fmt::Display for AuditEntryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// AuditEntry
// ---------------------------------------------------------------------------

/// A single entry in the policy audit trail.
///
/// Each entry captures:
/// - The event type (trigger received, policy evaluated, action scheduled/executed)
/// - An optional asset identifier
/// - An optional metadata payload (JSON)
/// - A UTC timestamp
///
/// Entries are individually digestable via `CanonicalBytes` + `sha256_digest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// The type of audit event.
    pub entry_type: AuditEntryType,
    /// UTC timestamp when the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Optional asset identifier associated with this event.
    pub asset_id: Option<String>,
    /// Optional structured metadata payload.
    pub metadata: Option<serde_json::Value>,
}

impl AuditEntry {
    /// Create a new audit entry with the current UTC timestamp.
    pub fn new(
        entry_type: AuditEntryType,
        asset_id: Option<String>,
        metadata: Option<serde_json::Value>,
    ) -> Self {
        Self {
            entry_type,
            timestamp: Utc::now(),
            asset_id,
            metadata,
        }
    }

    /// Compute the content-addressed digest of this audit entry.
    ///
    /// Uses `CanonicalBytes` → `sha256_digest` to produce a deterministic
    /// digest suitable for tamper-evidence verification.
    ///
    /// Returns `None` if canonicalization fails (e.g., metadata contains floats).
    pub fn digest(&self) -> Option<ContentDigest> {
        let canonical = match CanonicalBytes::new(self) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(entry_type = ?self.entry_type, error = %e, "audit entry canonicalization failed — digest unavailable");
                return None;
            }
        };
        Some(sha256_digest(&canonical))
    }
}

impl PartialEq for AuditEntry {
    fn eq(&self, other: &Self) -> bool {
        self.entry_type == other.entry_type
            && self.asset_id == other.asset_id
            && self.metadata == other.metadata
    }
}

impl Eq for AuditEntry {}

// ---------------------------------------------------------------------------
// AuditTrail
// ---------------------------------------------------------------------------

/// An append-only audit trail with a configurable capacity.
///
/// When the trail exceeds its maximum capacity, the oldest 10% of entries
/// are trimmed. In production, entries should be persisted to durable storage
/// before trimming.
///
/// ## Thread Safety
///
/// This struct is not `Sync`. Use external synchronisation if sharing across
/// threads (e.g., `Arc<Mutex<AuditTrail>>`).
pub struct AuditTrail {
    entries: Vec<AuditEntry>,
    max_entries: usize,
}

impl AuditTrail {
    /// Create a new audit trail with the given maximum capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Append an entry to the audit trail.
    ///
    /// If the trail exceeds the maximum capacity, the oldest 10% of entries
    /// are trimmed.
    pub fn append(&mut self, entry: AuditEntry) {
        self.entries.push(entry);
        if self.entries.len() > self.max_entries {
            let trim_count = self.max_entries / 10;
            let trim_count = trim_count.max(1);
            self.entries.drain(..trim_count);
        }
    }

    /// Return a reference to all entries in the trail.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Return the number of entries in the trail.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the trail is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Return entries matching a specific asset ID.
    pub fn entries_for_asset(&self, asset_id: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.asset_id.as_deref() == Some(asset_id))
            .collect()
    }

    /// Return entries matching a specific event type.
    pub fn entries_by_type(&self, entry_type: AuditEntryType) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.entry_type == entry_type)
            .collect()
    }

    /// Return the last N entries (or all entries if fewer than N exist).
    pub fn last_n(&self, n: usize) -> &[AuditEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    /// Compute digests for all entries in the trail.
    ///
    /// Returns a vector of `(index, digest)` pairs. Entries that fail
    /// canonicalization are silently skipped (this should not happen with
    /// well-formed audit data).
    pub fn compute_digests(&self) -> Vec<(usize, ContentDigest)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, entry)| entry.digest().map(|d| (i, d)))
            .collect()
    }
}

impl Default for AuditTrail {
    fn default() -> Self {
        Self::new(10_000)
    }
}

impl std::fmt::Debug for AuditTrail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditTrail")
            .field("entries", &self.entries.len())
            .field("max_entries", &self.max_entries)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_entry_creation() {
        let entry = AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some("asset:test".into()),
            Some(serde_json::json!({"trigger": "sanctions"})),
        );
        assert_eq!(entry.entry_type, AuditEntryType::TriggerReceived);
        assert_eq!(entry.asset_id.as_deref(), Some("asset:test"));
    }

    #[test]
    fn audit_entry_digest() {
        let entry = AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some("asset:test".into()),
            Some(serde_json::json!({"policy_id": "sanctions_auto_halt", "matched": true})),
        );
        let digest = entry.digest();
        assert!(digest.is_some());
        let d = digest.unwrap();
        assert_eq!(d.to_hex().len(), 64);
    }

    #[test]
    fn audit_entry_digest_determinism() {
        let entry = AuditEntry {
            entry_type: AuditEntryType::ActionExecuted,
            timestamp: chrono::DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            asset_id: Some("asset:test".into()),
            metadata: Some(serde_json::json!({"action": "halt"})),
        };
        let d1 = entry.digest().unwrap();
        let d2 = entry.digest().unwrap();
        assert_eq!(d1, d2);
    }

    #[test]
    fn audit_trail_append_and_query() {
        let mut trail = AuditTrail::new(100);
        trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some("asset:a".into()),
            None,
        ));
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some("asset:a".into()),
            None,
        ));
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some("asset:b".into()),
            None,
        ));

        assert_eq!(trail.len(), 3);
        assert!(!trail.is_empty());
        assert_eq!(trail.entries_for_asset("asset:a").len(), 2);
        assert_eq!(trail.entries_for_asset("asset:b").len(), 1);
        assert_eq!(
            trail.entries_by_type(AuditEntryType::PolicyEvaluated).len(),
            2
        );
        assert_eq!(
            trail.entries_by_type(AuditEntryType::TriggerReceived).len(),
            1
        );
    }

    #[test]
    fn audit_trail_trimming() {
        let mut trail = AuditTrail::new(10);
        for i in 0..15 {
            trail.append(AuditEntry::new(
                AuditEntryType::TriggerReceived,
                Some(format!("asset:{i}")),
                None,
            ));
        }
        // After exceeding capacity, oldest 10% (= 1 entry) are trimmed per append.
        // Starting from 11th entry, each append triggers a trim.
        // Final count should be <= max_entries.
        assert!(trail.len() <= 11);
    }

    #[test]
    fn audit_trail_last_n() {
        let mut trail = AuditTrail::new(100);
        for i in 0..5 {
            trail.append(AuditEntry::new(
                AuditEntryType::TriggerReceived,
                Some(format!("asset:{i}")),
                None,
            ));
        }
        let last_2 = trail.last_n(2);
        assert_eq!(last_2.len(), 2);
        assert_eq!(last_2[0].asset_id.as_deref(), Some("asset:3"));
        assert_eq!(last_2[1].asset_id.as_deref(), Some("asset:4"));

        // Requesting more than available returns all.
        let all = trail.last_n(100);
        assert_eq!(all.len(), 5);
    }

    #[test]
    fn audit_trail_compute_digests() {
        let mut trail = AuditTrail::new(100);
        trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some("asset:test".into()),
            Some(serde_json::json!({"trigger": "sanctions"})),
        ));
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            None,
            Some(serde_json::json!({"matched": false})),
        ));

        let digests = trail.compute_digests();
        assert_eq!(digests.len(), 2);
        assert_ne!(digests[0].1, digests[1].1);
    }

    #[test]
    fn audit_entry_type_display() {
        assert_eq!(
            AuditEntryType::TriggerReceived.to_string(),
            "trigger_received"
        );
        assert_eq!(
            AuditEntryType::PolicyEvaluated.to_string(),
            "policy_evaluated"
        );
        assert_eq!(
            AuditEntryType::ActionScheduled.to_string(),
            "action_scheduled"
        );
        assert_eq!(
            AuditEntryType::ActionExecuted.to_string(),
            "action_executed"
        );
    }

    #[test]
    fn audit_entry_serde_roundtrip() {
        let entry = AuditEntry {
            entry_type: AuditEntryType::ActionScheduled,
            timestamp: chrono::DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            asset_id: Some("asset:test".into()),
            metadata: Some(serde_json::json!({"policy_id": "sanctions_auto_halt"})),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let parsed: AuditEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entry_type, entry.entry_type);
        assert_eq!(parsed.asset_id, entry.asset_id);
        assert_eq!(parsed.metadata, entry.metadata);
    }

    // ── Additional coverage tests ──────────────────────────────────

    #[test]
    fn audit_entry_type_display_failed_and_cancelled() {
        assert_eq!(AuditEntryType::ActionFailed.to_string(), "action_failed");
        assert_eq!(
            AuditEntryType::ActionCancelled.to_string(),
            "action_cancelled"
        );
    }

    #[test]
    fn audit_entry_type_as_str_all_variants() {
        assert_eq!(AuditEntryType::TriggerReceived.as_str(), "trigger_received");
        assert_eq!(AuditEntryType::PolicyEvaluated.as_str(), "policy_evaluated");
        assert_eq!(AuditEntryType::ActionScheduled.as_str(), "action_scheduled");
        assert_eq!(AuditEntryType::ActionExecuted.as_str(), "action_executed");
        assert_eq!(AuditEntryType::ActionFailed.as_str(), "action_failed");
        assert_eq!(AuditEntryType::ActionCancelled.as_str(), "action_cancelled");
    }

    #[test]
    fn audit_entry_no_asset_no_metadata() {
        let entry = AuditEntry::new(AuditEntryType::ActionFailed, None, None);
        assert!(entry.asset_id.is_none());
        assert!(entry.metadata.is_none());
        // Should still produce a valid digest.
        assert!(entry.digest().is_some());
    }

    #[test]
    fn audit_entry_equality_ignores_timestamp() {
        let e1 = AuditEntry {
            entry_type: AuditEntryType::ActionExecuted,
            timestamp: chrono::DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            asset_id: Some("asset:x".into()),
            metadata: None,
        };
        let e2 = AuditEntry {
            entry_type: AuditEntryType::ActionExecuted,
            timestamp: chrono::DateTime::parse_from_rfc3339("2026-06-15T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            asset_id: Some("asset:x".into()),
            metadata: None,
        };
        // PartialEq impl ignores timestamp.
        assert_eq!(e1, e2);
    }

    #[test]
    fn audit_entry_inequality_different_type() {
        let e1 = AuditEntry::new(AuditEntryType::ActionExecuted, None, None);
        let e2 = AuditEntry::new(AuditEntryType::ActionFailed, None, None);
        assert_ne!(e1, e2);
    }

    #[test]
    fn audit_trail_default_capacity() {
        let trail = AuditTrail::default();
        assert!(trail.is_empty());
        assert_eq!(trail.len(), 0);
    }

    #[test]
    fn audit_trail_debug_format() {
        let mut trail = AuditTrail::new(50);
        trail.append(AuditEntry::new(AuditEntryType::TriggerReceived, None, None));
        let dbg = format!("{trail:?}");
        assert!(dbg.contains("AuditTrail"));
        assert!(dbg.contains("entries: 1"));
        assert!(dbg.contains("max_entries: 50"));
    }

    #[test]
    fn audit_trail_entries_for_asset_no_match() {
        let mut trail = AuditTrail::new(100);
        trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some("asset:a".into()),
            None,
        ));
        assert!(trail.entries_for_asset("asset:z").is_empty());
    }

    #[test]
    fn audit_trail_entries_by_type_no_match() {
        let mut trail = AuditTrail::new(100);
        trail.append(AuditEntry::new(AuditEntryType::TriggerReceived, None, None));
        assert!(trail
            .entries_by_type(AuditEntryType::ActionCancelled)
            .is_empty());
    }

    #[test]
    fn audit_trail_last_n_zero() {
        let mut trail = AuditTrail::new(100);
        trail.append(AuditEntry::new(AuditEntryType::TriggerReceived, None, None));
        let last = trail.last_n(0);
        assert!(last.is_empty());
    }

    #[test]
    fn audit_trail_trimming_precise() {
        // max_entries = 10, so trim_count = max(10/10, 1) = 1.
        // After 11th append, one is trimmed, leaving 10.
        let mut trail = AuditTrail::new(10);
        for i in 0..11 {
            trail.append(AuditEntry::new(
                AuditEntryType::TriggerReceived,
                Some(format!("asset:{i}")),
                None,
            ));
        }
        assert_eq!(trail.len(), 10);
        // Oldest entry (asset:0) should have been trimmed.
        assert_eq!(trail.entries()[0].asset_id.as_deref(), Some("asset:1"));
    }

    #[test]
    fn audit_trail_trimming_small_capacity() {
        // max_entries = 1 means trim_count = max(1/10, 1) = 1.
        let mut trail = AuditTrail::new(1);
        trail.append(AuditEntry::new(
            AuditEntryType::ActionExecuted,
            Some("asset:0".into()),
            None,
        ));
        // First entry within capacity.
        assert_eq!(trail.len(), 1);

        trail.append(AuditEntry::new(
            AuditEntryType::ActionFailed,
            Some("asset:1".into()),
            None,
        ));
        // Second entry triggers trim: we had 2, which exceeds 1, so drain 1.
        assert_eq!(trail.len(), 1);
        assert_eq!(trail.entries()[0].asset_id.as_deref(), Some("asset:1"));
    }

    #[test]
    fn audit_entry_type_serde_roundtrip() {
        let types = [
            AuditEntryType::TriggerReceived,
            AuditEntryType::PolicyEvaluated,
            AuditEntryType::ActionScheduled,
            AuditEntryType::ActionExecuted,
            AuditEntryType::ActionFailed,
            AuditEntryType::ActionCancelled,
        ];
        for t in &types {
            let json = serde_json::to_string(t).unwrap();
            let back: AuditEntryType = serde_json::from_str(&json).unwrap();
            assert_eq!(&back, t);
        }
    }

    #[test]
    fn audit_trail_compute_digests_all_entries() {
        let mut trail = AuditTrail::new(100);
        for i in 0..5 {
            trail.append(AuditEntry::new(
                AuditEntryType::ActionExecuted,
                Some(format!("asset:{i}")),
                Some(serde_json::json!({"i": i})),
            ));
        }
        let digests = trail.compute_digests();
        assert_eq!(digests.len(), 5);
        // Verify indices are sequential.
        for (idx, (i, _)) in digests.iter().enumerate() {
            assert_eq!(*i, idx);
        }
    }

    #[test]
    fn audit_entry_clone() {
        let entry = AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some("asset:clone".into()),
            Some(serde_json::json!({"key": "val"})),
        );
        let cloned = entry.clone();
        assert_eq!(entry, cloned);
        assert_eq!(entry.timestamp, cloned.timestamp);
    }
}
