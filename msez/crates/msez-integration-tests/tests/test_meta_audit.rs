//! # Meta Audit Tests
//!
//! Tests the integrity of the agentic policy engine's audit trail:
//! append-and-retrieve, digest chain integrity, circular buffer trimming,
//! and audit entry type completeness.

use msez_agentic::{AuditEntry, AuditEntryType, AuditTrail};
use msez_core::{sha256_digest, CanonicalBytes};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Audit trail append and retrieve
// ---------------------------------------------------------------------------

#[test]
fn audit_trail_append_and_retrieve() {
    let mut trail = AuditTrail::new(1000);
    assert!(trail.is_empty());
    assert_eq!(trail.len(), 0);

    // Append entries
    trail.append(AuditEntry::new(
        AuditEntryType::TriggerReceived,
        Some("asset:mining-001".to_string()),
        Some(json!({"trigger_type": "sanctions_list_update"})),
    ));
    trail.append(AuditEntry::new(
        AuditEntryType::PolicyEvaluated,
        Some("asset:mining-001".to_string()),
        Some(json!({"policy_id": "sanctions_auto_halt", "matched": true})),
    ));
    trail.append(AuditEntry::new(
        AuditEntryType::ActionScheduled,
        Some("asset:mining-001".to_string()),
        Some(json!({"action": "halt"})),
    ));

    assert_eq!(trail.len(), 3);
    assert!(!trail.is_empty());

    // Retrieve all entries
    let entries = trail.entries();
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[0].entry_type, AuditEntryType::TriggerReceived);
    assert_eq!(entries[1].entry_type, AuditEntryType::PolicyEvaluated);
    assert_eq!(entries[2].entry_type, AuditEntryType::ActionScheduled);

    // Filter by asset
    let asset_entries = trail.entries_for_asset("asset:mining-001");
    assert_eq!(asset_entries.len(), 3);

    // Filter by type
    let trigger_entries = trail.entries_by_type(AuditEntryType::TriggerReceived);
    assert_eq!(trigger_entries.len(), 1);
}

#[test]
fn audit_trail_last_n() {
    let mut trail = AuditTrail::new(1000);
    for i in 0..10 {
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some(format!("asset:{i}")),
            None,
        ));
    }

    let last_3 = trail.last_n(3);
    assert_eq!(last_3.len(), 3);
    assert_eq!(last_3[0].asset_id.as_deref(), Some("asset:7"));
    assert_eq!(last_3[1].asset_id.as_deref(), Some("asset:8"));
    assert_eq!(last_3[2].asset_id.as_deref(), Some("asset:9"));
}

// ---------------------------------------------------------------------------
// 2. Audit trail digest chain integrity
// ---------------------------------------------------------------------------

#[test]
fn audit_trail_digest_chain_integrity() {
    let mut trail = AuditTrail::new(1000);

    // Append entries with known data
    trail.append(AuditEntry::new(
        AuditEntryType::TriggerReceived,
        Some("asset:test".to_string()),
        Some(json!({"event": "trigger_1"})),
    ));
    trail.append(AuditEntry::new(
        AuditEntryType::PolicyEvaluated,
        Some("asset:test".to_string()),
        Some(json!({"event": "eval_1", "matched": true})),
    ));
    trail.append(AuditEntry::new(
        AuditEntryType::ActionExecuted,
        Some("asset:test".to_string()),
        Some(json!({"event": "exec_1"})),
    ));

    // Compute digests for all entries
    let digests = trail.compute_digests();
    assert_eq!(digests.len(), 3, "all 3 entries must have computable digests");

    // Each entry has a unique digest
    assert_ne!(digests[0].1, digests[1].1);
    assert_ne!(digests[1].1, digests[2].1);
    assert_ne!(digests[0].1, digests[2].1);

    // Digest indices are correct
    assert_eq!(digests[0].0, 0);
    assert_eq!(digests[1].0, 1);
    assert_eq!(digests[2].0, 2);

    // All digests are valid 64-char hex
    for (_idx, digest) in &digests {
        assert_eq!(digest.to_hex().len(), 64);
    }
}

#[test]
fn audit_entry_individual_digest_determinism() {
    // Create two identical entries (with fixed timestamp for determinism)
    let entry = AuditEntry {
        entry_type: AuditEntryType::ActionScheduled,
        timestamp: chrono::DateTime::parse_from_rfc3339("2026-01-15T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        asset_id: Some("asset:determinism-test".to_string()),
        metadata: Some(json!({"policy_id": "test_policy"})),
    };

    let d1 = entry.digest().unwrap();
    let d2 = entry.digest().unwrap();
    assert_eq!(d1, d2, "audit entry digest must be deterministic");
}

// ---------------------------------------------------------------------------
// 3. Audit trail circular buffer trimming
// ---------------------------------------------------------------------------

#[test]
fn audit_trail_circular_buffer_trimming() {
    let max_entries = 10;
    let mut trail = AuditTrail::new(max_entries);

    // Append more than max_entries
    for i in 0..20 {
        trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some(format!("asset:{i}")),
            None,
        ));
    }

    // Trail should have trimmed oldest entries
    assert!(
        trail.len() <= max_entries + 1,
        "trail length {} should be at most {} after trimming",
        trail.len(),
        max_entries + 1
    );

    // The most recent entries should still be present
    let entries = trail.entries();
    let last = entries.last().unwrap();
    assert_eq!(last.asset_id.as_deref(), Some("asset:19"));
}

#[test]
fn audit_trail_trimming_preserves_newest() {
    let mut trail = AuditTrail::new(5);

    for i in 0..10 {
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some(format!("asset:{i}")),
            None,
        ));
    }

    // The newest entry should always be the last one appended
    let entries = trail.entries();
    assert!(entries.len() <= 6);
    assert_eq!(
        entries.last().unwrap().asset_id.as_deref(),
        Some("asset:9")
    );
}

// ---------------------------------------------------------------------------
// 4. Audit entry types all represented
// ---------------------------------------------------------------------------

#[test]
fn audit_entry_types_all_represented() {
    let types = [
        AuditEntryType::TriggerReceived,
        AuditEntryType::PolicyEvaluated,
        AuditEntryType::ActionScheduled,
        AuditEntryType::ActionExecuted,
        AuditEntryType::ActionFailed,
        AuditEntryType::ActionCancelled,
    ];

    assert_eq!(types.len(), 6, "there must be exactly 6 audit entry types");

    for entry_type in &types {
        let name = entry_type.as_str();
        assert!(!name.is_empty(), "entry type {entry_type:?} has empty name");

        // Verify each type can be used in an audit entry
        let entry = AuditEntry::new(*entry_type, None, None);
        assert_eq!(entry.entry_type, *entry_type);

        // Each type should produce a valid digest
        let digest = entry.digest();
        assert!(
            digest.is_some(),
            "audit entry of type {name} must produce a valid digest"
        );
    }
}

#[test]
fn audit_entry_type_display_format() {
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
    assert_eq!(AuditEntryType::ActionFailed.to_string(), "action_failed");
    assert_eq!(
        AuditEntryType::ActionCancelled.to_string(),
        "action_cancelled"
    );
}

// ---------------------------------------------------------------------------
// 5. Audit entry serde roundtrip
// ---------------------------------------------------------------------------

#[test]
fn audit_entry_serde_roundtrip() {
    let entry = AuditEntry {
        entry_type: AuditEntryType::ActionExecuted,
        timestamp: chrono::DateTime::parse_from_rfc3339("2026-02-12T00:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        asset_id: Some("asset:roundtrip-test".to_string()),
        metadata: Some(json!({"action": "halt", "reason": "sanctions"})),
    };

    let serialized = serde_json::to_string(&entry).unwrap();
    let deserialized: AuditEntry = serde_json::from_str(&serialized).unwrap();

    assert_eq!(entry.entry_type, deserialized.entry_type);
    assert_eq!(entry.asset_id, deserialized.asset_id);
    assert_eq!(entry.metadata, deserialized.metadata);
}
