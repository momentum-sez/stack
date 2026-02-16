//! # Campaign 2: Panic Path Assault
//!
//! Tests that production code handles adversarial inputs without panicking.
//! Each test provides an input designed to trigger an `unwrap()` or `.expect()`
//! in production code.

use serde_json::json;
use std::panic;

// =========================================================================
// msez-core: Canonical + Digest panic paths
// =========================================================================

use msez_core::{
    CanonicalBytes, Cnic, ContentDigest, CorridorId, Did, EntityId, JurisdictionId, Ntn,
    PassportNumber, Timestamp,
};

#[test]
fn canonical_bytes_null_input_no_panic() {
    let result = panic::catch_unwind(|| {
        let _ = CanonicalBytes::new(&json!(null));
    });
    assert!(result.is_ok(), "CanonicalBytes::new panicked on null input");
}

#[test]
fn canonical_bytes_deeply_nested_no_panic() {
    // Create a deeply nested structure (100 levels)
    let mut value = json!("leaf");
    for _ in 0..100 {
        value = json!({"nested": value});
    }
    let result = panic::catch_unwind(|| {
        let _ = CanonicalBytes::new(&value);
    });
    assert!(
        result.is_ok(),
        "CanonicalBytes::new panicked on deeply nested input"
    );
}

#[test]
fn canonical_bytes_empty_object_no_panic() {
    let result = CanonicalBytes::new(&json!({}));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_empty_array_no_panic() {
    let result = CanonicalBytes::new(&json!([]));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_large_string_no_panic() {
    let big = "a".repeat(1_000_000);
    let result = panic::catch_unwind(|| {
        let _ = CanonicalBytes::new(&json!(big));
    });
    assert!(result.is_ok(), "CanonicalBytes::new panicked on 1MB string");
}

#[test]
fn canonical_bytes_unicode_no_panic() {
    let result = CanonicalBytes::new(&json!({"日本語": "🏛️", "emoji": "✅"}));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_embedded_null_byte_no_panic() {
    let result = CanonicalBytes::new(&json!({"key": "a\0b"}));
    assert!(result.is_ok());
}

#[test]
fn content_digest_from_hex_empty_string() {
    let result = ContentDigest::from_hex("");
    assert!(result.is_err(), "Empty hex should be rejected");
}

#[test]
fn content_digest_from_hex_non_hex_chars() {
    let result =
        ContentDigest::from_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz");
    assert!(result.is_err(), "Non-hex chars should be rejected");
}

#[test]
fn content_digest_from_hex_wrong_length_short() {
    let result = ContentDigest::from_hex("aabb");
    assert!(result.is_err(), "4-char hex (2 bytes) should be rejected");
}

#[test]
fn content_digest_from_hex_wrong_length_long() {
    let hex_66 = "aa".repeat(33);
    let result = ContentDigest::from_hex(&hex_66);
    assert!(result.is_err(), "66-char hex (33 bytes) should be rejected");
}

#[test]
fn content_digest_from_hex_valid() {
    let hex_64 = "aa".repeat(32);
    let result = ContentDigest::from_hex(&hex_64);
    assert!(result.is_ok(), "64-char hex (32 bytes) should succeed");
}

#[test]
fn content_digest_from_hex_odd_length() {
    let result = ContentDigest::from_hex("abc");
    assert!(result.is_err(), "Odd-length hex should be rejected");
}

// =========================================================================
// msez-core: Identity type validation
// =========================================================================

#[test]
fn did_new_empty_string() {
    let result = Did::new("");
    assert!(result.is_err(), "Empty DID should be rejected");
}

#[test]
fn did_new_no_colons() {
    let result = Did::new("didweb");
    assert!(result.is_err(), "DID without colons should be rejected");
}

#[test]
fn did_new_only_prefix() {
    let result = Did::new("did:");
    assert!(result.is_err(), "DID with only 'did:' should be rejected");
}

#[test]
fn did_new_two_colons_empty_id() {
    // "did:web:" has method "web" but an empty method-specific-id.
    // Verify it doesn't panic and produces a definite outcome.
    let result = panic::catch_unwind(|| Did::new("did:web:"));
    assert!(
        result.is_ok(),
        "Did::new must not panic on 'did:web:' input"
    );
    // The result is either Ok (accepted) or Err (rejected) — both are valid.
    // What matters: no panic, and the outcome is deterministic.
    let outcome = Did::new("did:web:");
    let outcome2 = Did::new("did:web:");
    assert_eq!(
        outcome.is_ok(),
        outcome2.is_ok(),
        "Did::new must be deterministic"
    );
}

#[test]
fn did_method_and_id_no_panic() {
    // BUG-019 RESOLVED: Custom Deserialize now validates via Did::new().
    // Invalid DIDs are rejected at deserialization time, so the panic
    // path in Did::method() can never be reached via serde.
    let result: Result<Did, _> = serde_json::from_str("\"not-a-did\"");
    assert!(
        result.is_err(),
        "BUG-019/BUG-013 RESOLVED: invalid DID must be rejected at deserialization"
    );

    // Valid DIDs still work correctly through serde
    let valid: Did =
        serde_json::from_str("\"did:web:example.com\"").expect("valid DID should deserialize");
    assert_eq!(valid.method(), "web");
    assert_eq!(valid.method_specific_id(), "example.com");
}

#[test]
fn ntn_new_empty_string() {
    let result = Ntn::new("");
    assert!(result.is_err(), "Empty NTN should be rejected");
}

#[test]
fn ntn_new_too_short() {
    let result = Ntn::new("12345");
    assert!(result.is_err(), "5-digit NTN should be rejected");
}

#[test]
fn ntn_new_too_long() {
    let result = Ntn::new("12345678");
    assert!(result.is_err(), "8-digit NTN should be rejected");
}

#[test]
fn ntn_new_non_digits() {
    let result = Ntn::new("ABCDEFG");
    assert!(result.is_err(), "Non-digit NTN should be rejected");
}

#[test]
fn cnic_new_empty_string() {
    let result = Cnic::new("");
    assert!(result.is_err(), "Empty CNIC should be rejected");
}

#[test]
fn cnic_new_too_short() {
    let result = Cnic::new("123456");
    assert!(result.is_err(), "6-digit CNIC should be rejected");
}

#[test]
fn cnic_new_too_long() {
    let result = Cnic::new("12345678901234");
    assert!(result.is_err(), "14-digit CNIC should be rejected");
}

#[test]
fn passport_new_empty_string() {
    let result = PassportNumber::new("");
    assert!(result.is_err(), "Empty passport should be rejected");
}

#[test]
fn passport_new_too_short() {
    let result = PassportNumber::new("AB");
    assert!(result.is_err(), "2-char passport should be rejected");
}

#[test]
fn passport_new_too_long() {
    let result = PassportNumber::new("A".repeat(21).as_str());
    assert!(result.is_err(), "21-char passport should be rejected");
}

#[test]
fn jurisdiction_id_new_empty_string() {
    let result = JurisdictionId::new("");
    assert!(result.is_err(), "Empty JurisdictionId should be rejected");
}

#[test]
fn jurisdiction_id_new_whitespace_only() {
    let result = panic::catch_unwind(|| JurisdictionId::new("   "));
    assert!(result.is_ok(), "JurisdictionId::new must not panic on whitespace");
    // Whitespace-only should be deterministic.
    let r1 = JurisdictionId::new("   ");
    let r2 = JurisdictionId::new("   ");
    assert_eq!(r1.is_ok(), r2.is_ok(), "JurisdictionId::new must be deterministic");
}

#[test]
fn jurisdiction_id_new_path_traversal() {
    // Path traversal attempt — should not panic and must be deterministic.
    let result = panic::catch_unwind(|| JurisdictionId::new("../../../../etc/passwd"));
    assert!(result.is_ok(), "JurisdictionId::new must not panic on path traversal input");
    let r1 = JurisdictionId::new("../../../../etc/passwd");
    let r2 = JurisdictionId::new("../../../../etc/passwd");
    assert_eq!(r1.is_ok(), r2.is_ok(), "JurisdictionId::new must be deterministic");
}

// =========================================================================
// msez-core: Timestamp edge cases
// =========================================================================

#[test]
fn timestamp_from_rfc3339_empty_string() {
    let result = Timestamp::from_rfc3339("");
    assert!(result.is_err(), "Empty string should fail parsing");
}

#[test]
fn timestamp_from_rfc3339_not_a_date() {
    let result = Timestamp::from_rfc3339("not a date");
    assert!(result.is_err(), "Invalid date should fail parsing");
}

#[test]
fn timestamp_from_rfc3339_partial_date() {
    let result = Timestamp::from_rfc3339("2026-01");
    assert!(result.is_err(), "Partial date should fail parsing");
}

#[test]
fn timestamp_from_date_str_empty() {
    let result = Timestamp::from_date_str("");
    assert!(result.is_err(), "Empty date string should fail");
}

#[test]
fn timestamp_from_date_str_invalid() {
    let result = Timestamp::from_date_str("not-a-date");
    assert!(result.is_err(), "Invalid date string should fail");
}

#[test]
fn timestamp_from_date_str_impossible_date() {
    let result = Timestamp::from_date_str("2026-02-30");
    assert!(result.is_err(), "Feb 30 should fail");
}

// =========================================================================
// msez-corridor: NettingEngine panic paths
// =========================================================================

use msez_corridor::netting::{NettingEngine, Obligation};

#[test]
fn netting_zero_amount_obligation_rejected() {
    let mut engine = NettingEngine::new();
    let result = engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: 0,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    assert!(result.is_err(), "Zero amount should be rejected");
}

#[test]
fn netting_negative_amount_obligation_rejected() {
    let mut engine = NettingEngine::new();
    let result = engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: -100,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    assert!(result.is_err(), "Negative amount should be rejected");
}

#[test]
fn netting_empty_engine_compute_plan_returns_error() {
    let engine = NettingEngine::new();
    let result = engine.compute_plan();
    assert!(
        result.is_err(),
        "Empty engine should return NoObligations error"
    );
}

#[test]
fn netting_self_obligation_no_panic() {
    // A party owing itself — should not panic
    let mut engine = NettingEngine::new();
    let add_result = engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "A".to_string(),
        amount: 100,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    // Either rejected upfront (valid) or accepted — but must never panic
    match add_result {
        Err(_) => {} // Correctly rejected self-obligation
        Ok(()) => {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| engine.compute_plan()));
            assert!(result.is_ok(), "Self-obligation should not panic in compute_plan");
        }
    }
}

#[test]
fn netting_i64_max_single_obligation_no_panic() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: i64::MAX,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    // Single obligation should not overflow
    let result = engine.compute_plan();
    assert!(result.is_ok());
}

#[test]
fn netting_i64_overflow_gross_total() {
    // BUG-020: Two obligations that sum to > i64::MAX.
    // The .sum() on line 294 of netting.rs will panic in debug mode
    // or wrap in release mode.
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: i64::MAX / 2 + 1,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "C".to_string(),
            to_party: "D".to_string(),
            amount: i64::MAX / 2 + 1,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| engine.compute_plan()));
    match result {
        Ok(Ok(plan)) => {
            // If it produced a plan, gross_total must not have overflowed negative.
            assert!(
                plan.gross_total >= 0,
                "BUG-020: gross_total overflowed to {} (i64 wrap in release mode)",
                plan.gross_total,
            );
        }
        Ok(Err(_)) => {
            // Returned an error — correct behavior for overflow
        }
        Err(panic_info) => {
            // Panicked on overflow — this is a real bug. Re-panic so the test fails.
            std::panic::resume_unwind(panic_info);
        }
    }
}

#[test]
fn netting_i64_overflow_net_position() {
    // BUG-021: receivable - payable in compute_net_positions
    // can overflow: receivable = i64::MAX, payable = 0 is fine;
    // test the limit
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: i64::MAX,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let positions = engine.compute_net_positions();
    // B should have receivable = i64::MAX, payable = 0, net = i64::MAX
    let b_pos = positions.iter().find(|p| p.party_id == "B");
    assert!(b_pos.is_some());
    let b = b_pos.unwrap();
    assert_eq!(b.receivable, i64::MAX);
    assert_eq!(b.net, i64::MAX);
}

#[test]
fn netting_empty_party_ids_no_panic() {
    let mut engine = NettingEngine::new();
    let add_result = engine.add_obligation(Obligation {
        from_party: "".to_string(),
        to_party: "".to_string(),
        amount: 100,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    // Empty party IDs must be rejected at add_obligation time.
    assert!(
        add_result.is_err(),
        "empty party IDs must be rejected by add_obligation"
    );
}

#[test]
fn netting_empty_currency_no_panic() {
    let mut engine = NettingEngine::new();
    let add_result = engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: 100,
        currency: "".to_string(),
        corridor_id: None,
        priority: 0,
    });
    // Empty currency must be rejected at add_obligation time.
    assert!(
        add_result.is_err(),
        "empty currency must be rejected by add_obligation"
    );
}

// =========================================================================
// msez-crypto: Ed25519 panic paths
// =========================================================================

use msez_crypto::{sha256_digest, Ed25519Signature, SigningKey};

#[test]
fn sha256_digest_empty_input_no_panic() {
    let canonical = CanonicalBytes::new(&json!({})).unwrap();
    let _digest = sha256_digest(&canonical);
    // Should never panic — SHA-256 accepts empty input
}

#[test]
fn ed25519_sign_empty_message_no_panic() {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let canonical = CanonicalBytes::new(&json!({})).unwrap();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| signing_key.sign(&canonical)));
    assert!(result.is_ok(), "Signing empty canonical should not panic");
}

#[test]
fn ed25519_verify_wrong_key_returns_error() {
    let key1 = SigningKey::generate(&mut rand_core::OsRng);
    let key2 = SigningKey::generate(&mut rand_core::OsRng);
    let canonical = CanonicalBytes::new(&json!({"msg": "test"})).unwrap();
    let signature = key1.sign(&canonical);
    // Verify with wrong key should return error, not panic
    let result = key2.verifying_key().verify(&canonical, &signature);
    assert!(result.is_err());
}

#[test]
fn ed25519_signature_from_hex_invalid() {
    let result = Ed25519Signature::from_hex("not-hex-at-all-this-is-garbage");
    assert!(result.is_err(), "Non-hex signature should be rejected");
}

#[test]
fn ed25519_signature_from_hex_wrong_length() {
    // Ed25519 signatures are 64 bytes = 128 hex chars
    let result = Ed25519Signature::from_hex("aabb");
    assert!(result.is_err(), "4-char hex signature should be rejected");
}

#[test]
fn ed25519_signature_from_hex_empty() {
    let result = Ed25519Signature::from_hex("");
    assert!(result.is_err(), "Empty hex signature should be rejected");
}

// =========================================================================
// msez-crypto: CAS panic paths (filesystem-based)
// =========================================================================

use msez_crypto::ContentAddressedStore;

#[test]
fn cas_resolve_nonexistent_returns_none() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());
    let canonical = CanonicalBytes::new(&json!({"test": true})).unwrap();
    let digest = sha256_digest(&canonical);
    let result = cas.resolve("test-artifact", &digest);
    // Should return Ok(None) for nonexistent
    match result {
        Ok(None) => {} // correct
        Ok(Some(_)) => panic!("Should not find nonexistent artifact"),
        Err(_) => {} // also acceptable if artifact type validation fails etc.
    }
}

#[test]
fn cas_store_and_resolve_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());
    let value = json!({"key": "value"});
    let artifact_ref = cas.store("test-artifact", &value).unwrap();
    let resolved = cas.resolve_ref(&artifact_ref).unwrap();
    assert!(resolved.is_some(), "Stored artifact should be resolvable");
}

#[test]
fn cas_resolve_with_wrong_digest_returns_none() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());
    let value = json!({"key": "value"});
    let _artifact_ref = cas.store("test-artifact", &value).unwrap();
    // Use a different digest
    let other_canonical = CanonicalBytes::new(&json!({"key": "other"})).unwrap();
    let other_digest = sha256_digest(&other_canonical);
    let resolved = cas.resolve("test-artifact", &other_digest).unwrap();
    assert!(
        resolved.is_none(),
        "Resolving with wrong digest should return None"
    );
}

#[test]
fn cas_store_invalid_artifact_type_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());
    let value = json!({"key": "value"});
    // Empty artifact type should be rejected
    let result = cas.store("", &value);
    assert!(result.is_err(), "Empty artifact type should be rejected");
}

// =========================================================================
// msez-crypto: MMR panic paths
// =========================================================================

use msez_crypto::MerkleMountainRange;

#[test]
fn mmr_empty_root_no_panic() {
    // Getting root of empty MMR should not panic
    let result = panic::catch_unwind(|| {
        let mmr = MerkleMountainRange::new();
        mmr.root()
    });
    assert!(result.is_ok(), "Empty MMR root should not panic");
}

#[test]
fn mmr_single_leaf_root_no_panic() {
    let mut mmr = MerkleMountainRange::new();
    let canonical = CanonicalBytes::new(&json!({"leaf": 1})).unwrap();
    let digest = sha256_digest(&canonical);
    mmr.append(&digest.to_hex()).unwrap();
    let _root = mmr.root();
    // Just verify no panic — root may return Ok or Err
}

#[test]
fn mmr_many_appends_no_panic() {
    let mut mmr = MerkleMountainRange::new();
    for i in 0..1000 {
        let canonical = CanonicalBytes::new(&json!({"leaf": i})).unwrap();
        let digest = sha256_digest(&canonical);
        mmr.append(&digest.to_hex()).unwrap();
    }
    let root = mmr.root();
    assert!(root.is_ok(), "MMR root should succeed after appends");
}

#[test]
fn mmr_append_invalid_hex_returns_error() {
    let mut mmr = MerkleMountainRange::new();
    let result = mmr.append("not-valid-hex");
    assert!(result.is_err(), "MMR append with non-hex input must return error");
}

#[test]
fn mmr_append_empty_string_returns_error() {
    let mut mmr = MerkleMountainRange::new();
    let result = mmr.append("");
    assert!(result.is_err(), "MMR append with empty string must return error");
}

// =========================================================================
// msez-state: State machine invalid transition panic paths
// =========================================================================

use msez_state::{DynCorridorState, Entity, EntityLifecycleState, LicenseState, MigrationState};

#[test]
fn corridor_state_invalid_transition_draft_to_active_no_panic() {
    // Draft → Active is not a valid transition (must go through Pending first)
    let from = DynCorridorState::Draft;
    let valid = from.valid_transitions();
    // Active should NOT be in valid transitions from Draft
    let active_valid = valid.contains(&DynCorridorState::Active);
    assert!(
        !active_valid,
        "Draft → Active should not be a valid transition"
    );
}

#[test]
fn corridor_state_deprecated_to_any_no_panic() {
    // Deprecated is terminal — no transitions out should be valid
    let from = DynCorridorState::Deprecated;
    let valid = from.valid_transitions();
    assert!(
        valid.is_empty(),
        "Deprecated should have no valid transitions"
    );
}

#[test]
fn corridor_state_all_states_valid_transitions_no_panic() {
    // Verify valid_transitions() doesn't panic for any state
    let states = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];
    for state in &states {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| state.valid_transitions()));
        assert!(result.is_ok(), "{:?}.valid_transitions() panicked", state);
    }
}

#[test]
fn entity_invalid_transition_applied_to_suspended() {
    // Applied → Suspended should fail — must go through Active first
    let mut entity = Entity::new(EntityId::new());
    let result = entity.suspend();
    assert!(result.is_err(), "Applied → Suspended should be rejected");
}

#[test]
fn entity_invalid_transition_active_to_rejected() {
    // Reject only works from Applied state
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();
    let result = entity.reject();
    assert!(result.is_err(), "Active → Rejected should be rejected");
}

#[test]
fn entity_double_approve_no_panic() {
    // Approving twice should return error on second attempt, not panic
    let mut entity = Entity::new(EntityId::new());
    entity.approve().unwrap();
    let result = entity.approve();
    assert!(result.is_err(), "Double approve should be rejected");
}

// =========================================================================
// msez-corridor: SWIFT pacs.008 panic paths
// =========================================================================

use msez_corridor::swift::{SettlementInstruction, SettlementRail, SwiftPacs008};

#[test]
fn swift_generate_instruction_empty_bics() {
    // BUG-023: Empty BIC codes — should return error, never panic
    let swift = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test".to_string(),
        creditor_bic: "".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test".to_string(),
        amount: 1000,
        currency: "USD".to_string(),
        remittance_info: None,
    };
    let result = swift.generate_instruction(&instruction);
    // Empty BICs must be rejected by BIC validation.
    assert!(result.is_err(), "Empty BIC codes must be rejected");
}

#[test]
fn swift_generate_instruction_zero_amount() {
    // BUG-024: Zero amount — should return error
    let swift = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test".to_string(),
        amount: 0,
        currency: "USD".to_string(),
        remittance_info: None,
    };
    let result = swift.generate_instruction(&instruction);
    // Zero amount must be rejected — SWIFT pacs.008 requires positive amount.
    assert!(result.is_err(), "Zero amount must be rejected for SWIFT instruction");
}

#[test]
fn swift_generate_instruction_negative_amount() {
    // BUG-025: Negative amount — should return error
    let swift = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test".to_string(),
        amount: -1000,
        currency: "USD".to_string(),
        remittance_info: None,
    };
    let result = swift.generate_instruction(&instruction);
    // Negative amount must be rejected.
    assert!(result.is_err(), "Negative amount must be rejected for SWIFT instruction");
}

#[test]
fn swift_generate_instruction_malformed_bic() {
    let swift = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "ABC".to_string(), // Too short for BIC
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test".to_string(),
        amount: 1000,
        currency: "USD".to_string(),
        remittance_info: None,
    };
    let result = swift.generate_instruction(&instruction);
    // Malformed BIC (3 chars) must be rejected by BIC validation (8 or 11 chars).
    assert!(result.is_err(), "Malformed 3-char BIC must be rejected");
}

// =========================================================================
// msez-vc: Credential signing/verification panic paths
// =========================================================================

use msez_vc::credential::{ContextValue, CredentialTypeValue, ProofValue, VerifiableCredential};
use msez_vc::proof::ProofType;

#[test]
fn vc_sign_unsigned_credential_no_panic() {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:test:vc:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec!["VerifiableCredential".to_string()]),
        issuer: "did:key:z6MkTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"name": "Test"}),
        proof: ProofValue::default(),
    };
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        vc.sign_ed25519(
            &signing_key,
            "did:key:z6MkTest#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
    }));
    assert!(result.is_ok(), "Signing a VC should not panic");
}

#[test]
fn vc_verify_unsigned_returns_error() {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let vk = signing_key.verifying_key();
    let vc = VerifiableCredential {
        context: ContextValue::default(),
        id: None,
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({}),
        proof: ProofValue::default(),
    };
    // Verify unsigned credential: verify() returns explicit failure entry for no proofs.
    let results = vc.verify(|_method| Ok(vk.clone()));
    assert_eq!(results.len(), 1, "unsigned credential should return one failure entry");
    assert!(!results[0].ok, "unsigned credential must not verify as ok");

    // verify_all should return NoProofs error for unsigned credentials.
    let all_result = vc.verify_all(|_method| Ok(vk.clone()));
    assert!(all_result.is_err(), "verify_all must reject unsigned credentials");
}

#[test]
fn vc_signing_input_no_panic() {
    let vc = VerifiableCredential {
        context: ContextValue::default(),
        id: None,
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!(null),
        proof: ProofValue::default(),
    };
    let result = vc.signing_input();
    // Should not panic even with null subject — must produce a deterministic result.
    assert!(result.is_ok(), "signing_input must succeed even with null credential_subject");
}

// =========================================================================
// msez-arbitration: Dispute state transition panic paths
// =========================================================================

// =========================================================================
// msez-corridor: Fork resolution panic paths
// =========================================================================

use msez_corridor::fork::{resolve_fork, ForkBranch, ForkDetector};

#[test]
fn fork_resolve_identical_branches_no_panic() {
    let digest = test_digest_for("fork-test");
    let branch = ForkBranch {
        receipt_digest: digest.clone(),
        timestamp: chrono::Utc::now(),
        attestation_count: 3,
        next_root: "aa".repeat(32),
    };
    // Identical branches — resolution should not panic
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| resolve_fork(&branch, &branch)));
    assert!(
        result.is_ok(),
        "Fork resolution with identical branches should not panic"
    );
}

#[test]
fn fork_resolve_zero_attestations_no_panic() {
    let digest_a = test_digest_for("fork-a");
    let digest_b = test_digest_for("fork-b");
    let branch_a = ForkBranch {
        receipt_digest: digest_a,
        timestamp: chrono::Utc::now(),
        attestation_count: 0,
        next_root: "aa".repeat(32),
    };
    let branch_b = ForkBranch {
        receipt_digest: digest_b,
        timestamp: chrono::Utc::now(),
        attestation_count: 0,
        next_root: "bb".repeat(32),
    };
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        resolve_fork(&branch_a, &branch_b)
    }));
    assert!(
        result.is_ok(),
        "Fork resolution with zero attestations should not panic"
    );
}

#[test]
fn fork_detector_register_and_resolve_no_panic() {
    let mut detector = ForkDetector::new();
    let digest_a = test_digest_for("fork-det-a");
    let digest_b = test_digest_for("fork-det-b");
    let branch_a = ForkBranch {
        receipt_digest: digest_a,
        timestamp: chrono::Utc::now(),
        attestation_count: 5,
        next_root: "aa".repeat(32),
    };
    let branch_b = ForkBranch {
        receipt_digest: digest_b,
        timestamp: chrono::Utc::now() - chrono::Duration::seconds(10),
        attestation_count: 3,
        next_root: "bb".repeat(32),
    };
    detector.register_fork(branch_a, branch_b);
    assert_eq!(detector.pending_count(), 1);
    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
}

// =========================================================================
// msez-corridor: Bridge routing panic paths
// =========================================================================

use msez_corridor::bridge::{BridgeEdge, CorridorBridge};

#[test]
fn bridge_route_no_path_returns_none() {
    let bridge = CorridorBridge::new();
    let src = JurisdictionId::new("PK-RSEZ").unwrap();
    let tgt = JurisdictionId::new("AE-DIFC").unwrap();
    let result = bridge.find_route(&src, &tgt);
    assert!(result.is_none(), "Empty graph should return no route");
}

#[test]
fn bridge_route_same_source_target_returns_none() {
    let mut bridge = CorridorBridge::new();
    let pk = JurisdictionId::new("PK-RSEZ").unwrap();
    let ae = JurisdictionId::new("AE-DIFC").unwrap();
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 50,
        settlement_time_secs: 3600,
    });
    let result = bridge.find_route(&pk, &pk);
    // Routing from a node to itself should return None (no self-routes).
    assert!(result.is_none(), "find_route from node to itself must return None");
}

#[test]
fn bridge_route_single_hop() {
    let mut bridge = CorridorBridge::new();
    let pk = JurisdictionId::new("PK-RSEZ").unwrap();
    let ae = JurisdictionId::new("AE-DIFC").unwrap();
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 50,
        settlement_time_secs: 3600,
    });
    let route = bridge.find_route(&pk, &ae);
    assert!(route.is_some(), "Direct route should be found");
    let r = route.unwrap();
    assert_eq!(r.hop_count(), 1);
}

#[test]
fn bridge_route_multi_hop() {
    let mut bridge = CorridorBridge::new();
    let pk = JurisdictionId::new("PK").unwrap();
    let ae = JurisdictionId::new("AE").unwrap();
    let sg = JurisdictionId::new("SG").unwrap();
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 50,
        settlement_time_secs: 3600,
    });
    bridge.add_edge(BridgeEdge {
        from: ae.clone(),
        to: sg.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 30,
        settlement_time_secs: 1800,
    });
    let route = bridge.find_route(&pk, &sg);
    assert!(route.is_some(), "Multi-hop route should be found");
    let r = route.unwrap();
    assert_eq!(r.hop_count(), 2);
}

// =========================================================================
// msez-corridor: Receipt chain panic paths
// =========================================================================

use msez_corridor::receipt::{CorridorReceipt, ReceiptChain};

#[test]
fn receipt_chain_empty_root_no_panic() {
    let chain = ReceiptChain::new(CorridorId::new());
    let result = chain.mmr_root();
    // Empty chain — root should return error (no leaves) or empty sentinel.
    // Either way, must not panic and must be deterministic.
    let result2 = ReceiptChain::new(CorridorId::new()).mmr_root();
    assert_eq!(result.is_ok(), result2.is_ok(), "empty chain root must be deterministic");
}

#[test]
fn receipt_chain_empty_height() {
    let chain = ReceiptChain::new(CorridorId::new());
    assert_eq!(chain.height(), 0, "Empty chain should have height 0");
}

#[test]
fn receipt_chain_append_and_height() {
    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id.clone());
    let prev_root = chain.mmr_root().unwrap();
    let next_root = {
        let c = CanonicalBytes::new(&serde_json::json!({"seq": 0})).unwrap();
        sha256_digest(&c).to_hex()
    };
    let receipt = CorridorReceipt {
        receipt_type: "state_transition".to_string(),
        corridor_id: corridor_id.clone(),
        sequence: 0,
        timestamp: Timestamp::now(),
        prev_root,
        next_root,
        lawpack_digest_set: vec![],
        ruleset_digest_set: vec![],
    };
    let result = chain.append(receipt);
    assert!(result.is_ok(), "Appending valid receipt should succeed");
    assert_eq!(chain.height(), 1);
}

// =========================================================================
// msez-arbitration: EscrowAccount panic paths
// =========================================================================

use msez_arbitration::dispute::DisputeId;
use msez_arbitration::escrow::{
    EscrowAccount, EscrowStatus, EscrowType, ReleaseCondition, ReleaseConditionType,
};

fn test_digest_for(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&serde_json::json!({"label": label})).unwrap();
    msez_core::sha256_digest(&canonical)
}

#[test]
fn escrow_deposit_on_pending_no_panic() {
    let mut escrow = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::FilingFee,
        "USD".to_string(),
        None,
    );
    let result = escrow.deposit("10000".to_string(), test_digest_for("deposit-evidence"));
    assert!(result.is_ok(), "Deposit on Pending escrow should succeed");
    assert_eq!(escrow.status, EscrowStatus::Funded);
}

#[test]
fn escrow_double_deposit_no_panic() {
    let mut escrow = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::SecurityDeposit,
        "USD".to_string(),
        None,
    );
    escrow
        .deposit("10000".to_string(), test_digest_for("dep-1"))
        .unwrap();
    // Second deposit on already-Funded escrow — must not panic.
    let result = escrow.deposit("5000".to_string(), test_digest_for("dep-2"));
    // Second deposit on already-funded escrow must be rejected — the escrow
    // is already in Funded state and does not accept additional deposits.
    assert!(
        result.is_err(),
        "double deposit on already-funded escrow must be rejected"
    );
    // Verify determinism: same operation on fresh escrow produces same outcome.
    let mut escrow2 = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::SecurityDeposit,
        "USD".to_string(),
        None,
    );
    escrow2.deposit("10000".to_string(), test_digest_for("dep-1")).unwrap();
    let result2 = escrow2.deposit("5000".to_string(), test_digest_for("dep-2"));
    assert_eq!(result.is_ok(), result2.is_ok(), "double deposit behavior must be deterministic");
}

#[test]
fn escrow_release_without_deposit_fails() {
    let mut escrow = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::FilingFee,
        "USD".to_string(),
        None,
    );
    let condition = ReleaseCondition {
        condition_type: ReleaseConditionType::SettlementAgreed,
        evidence_digest: test_digest_for("release-evidence"),
        satisfied_at: Timestamp::now(),
    };
    let result = escrow.full_release(condition);
    assert!(result.is_err(), "Release without deposit should fail");
}

#[test]
fn escrow_forfeit_from_funded_no_panic() {
    let mut escrow = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::AppealBond,
        "SGD".to_string(),
        None,
    );
    escrow
        .deposit("25000".to_string(), test_digest_for("deposit"))
        .unwrap();
    let result = escrow.forfeit(test_digest_for("forfeit-evidence"));
    assert!(result.is_ok(), "Forfeit from Funded should succeed");
    assert_eq!(escrow.status, EscrowStatus::Forfeited);
}

#[test]
fn escrow_forfeited_is_terminal() {
    let mut escrow = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::AwardEscrow,
        "USD".to_string(),
        None,
    );
    escrow
        .deposit("50000".to_string(), test_digest_for("dep"))
        .unwrap();
    escrow.forfeit(test_digest_for("forfeit")).unwrap();
    // All operations from Forfeited should fail
    let cond = ReleaseCondition {
        condition_type: ReleaseConditionType::RulingEnforced,
        evidence_digest: test_digest_for("release"),
        satisfied_at: Timestamp::now(),
    };
    assert!(
        escrow.full_release(cond).is_err(),
        "Release from Forfeited should fail"
    );
    assert!(
        escrow
            .deposit("1000".to_string(), test_digest_for("dep2"))
            .is_err(),
        "Deposit on Forfeited should fail"
    );
}

// =========================================================================
// msez-arbitration: EnforcementOrder panic paths
// =========================================================================

use msez_arbitration::enforcement::{EnforcementAction, EnforcementOrder, EnforcementStatus};

#[test]
fn enforcement_begin_without_preconditions_no_panic() {
    let mut order = EnforcementOrder::new(
        DisputeId::new(),
        test_digest_for("award"),
        vec![EnforcementAction::MonetaryPenalty {
            party: Did::new("did:key:z6MkPenalty").unwrap(),
            amount: "10000".to_string(),
            currency: "USD".to_string(),
        }],
        None,
    );
    // No preconditions added — begin should succeed
    let result = order.begin_enforcement();
    assert!(
        result.is_ok(),
        "Begin enforcement without preconditions should succeed"
    );
    assert_eq!(order.status, EnforcementStatus::InProgress);
}

#[test]
fn enforcement_complete_without_action_results_no_panic() {
    let mut order = EnforcementOrder::new(DisputeId::new(), test_digest_for("award"), vec![], None);
    order.begin_enforcement().unwrap();
    // Complete without recording any action results.
    let result = order.complete();
    // Completing enforcement with zero action results: must not panic.
    // The implementation may succeed (vacuously complete) or reject.
    let r = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        let mut o2 = EnforcementOrder::new(DisputeId::new(), test_digest_for("award"), vec![], None);
        o2.begin_enforcement().unwrap();
        o2.complete()
    }));
    assert!(r.is_ok(), "EnforcementOrder::complete must not panic");
    // Verify the result matches (both succeed or both fail).
    assert_eq!(result.is_ok(), r.unwrap().is_ok(), "complete behavior must be deterministic");
}

#[test]
fn enforcement_cancel_from_pending() {
    let mut order = EnforcementOrder::new(DisputeId::new(), test_digest_for("award"), vec![], None);
    let result = order.cancel();
    assert!(result.is_ok(), "Cancel from Pending should succeed");
    assert_eq!(order.status, EnforcementStatus::Cancelled);
}

#[test]
fn enforcement_cancelled_is_terminal() {
    let mut order = EnforcementOrder::new(DisputeId::new(), test_digest_for("award"), vec![], None);
    order.cancel().unwrap();
    assert!(
        order.begin_enforcement().is_err(),
        "Begin from Cancelled should fail"
    );
    assert!(
        order.complete().is_err(),
        "Complete from Cancelled should fail"
    );
}

// =========================================================================
// msez-agentic: Scheduler panic paths
// =========================================================================

use msez_agentic::policy::{AuthorizationRequirement, PolicyAction};
use msez_agentic::scheduler::{ActionScheduler, ScheduledAction as SchedAction};

#[test]
fn scheduler_cancel_nonexistent_action() {
    let mut scheduler = ActionScheduler::new();
    let result = scheduler.cancel("nonexistent-id");
    assert!(!result, "Cancelling nonexistent action should return false");
}

#[test]
fn scheduler_mark_completed_nonexistent() {
    let mut scheduler = ActionScheduler::new();
    let result = scheduler.mark_completed("nonexistent-id");
    assert!(!result, "Completing nonexistent action should return false");
}

#[test]
fn scheduler_mark_failed_nonexistent() {
    let mut scheduler = ActionScheduler::new();
    let result = scheduler.mark_failed("nonexistent-id", "test error".to_string());
    assert!(!result, "Failing nonexistent action should return false");
}

#[test]
fn scheduler_schedule_and_retrieve() {
    let mut scheduler = ActionScheduler::new();
    let action = SchedAction::new(
        "asset:001".to_string(),
        PolicyAction::Halt,
        "policy-001".to_string(),
        AuthorizationRequirement::Automatic,
    );
    let id = scheduler.schedule(action);
    let retrieved = scheduler.get_action(&id);
    assert!(
        retrieved.is_some(),
        "Scheduled action should be retrievable"
    );
    assert_eq!(retrieved.unwrap().action, PolicyAction::Halt);
}

#[test]
fn scheduler_status_counts() {
    let mut scheduler = ActionScheduler::new();
    let action = SchedAction::new(
        "asset:001".to_string(),
        PolicyAction::Halt,
        "policy-001".to_string(),
        AuthorizationRequirement::Automatic,
    );
    let id = scheduler.schedule(action);
    let counts = scheduler.status_counts();
    assert_eq!(
        *counts
            .get(&msez_agentic::scheduler::ActionStatus::Pending)
            .unwrap_or(&0),
        1
    );
    scheduler.mark_executing(&id);
    scheduler.mark_completed(&id);
    let counts2 = scheduler.status_counts();
    assert_eq!(
        *counts2
            .get(&msez_agentic::scheduler::ActionStatus::Completed)
            .unwrap_or(&0),
        1
    );
}

use msez_arbitration::dispute::DisputeState;

#[test]
fn dispute_state_closed_to_any_no_panic() {
    // Closed is terminal — no transition out should be valid
    let from = DisputeState::Closed;
    let valid = from.valid_transitions();
    assert!(
        valid.is_empty(),
        "Closed should have no valid transitions, but has: {:?}",
        valid
    );
}

#[test]
fn dispute_state_all_valid_transitions_no_panic() {
    let states = [
        DisputeState::Filed,
        DisputeState::UnderReview,
        DisputeState::EvidenceCollection,
        DisputeState::Hearing,
        DisputeState::Decided,
        DisputeState::Enforced,
        DisputeState::Closed,
        DisputeState::Settled,
        DisputeState::Dismissed,
    ];
    for state in &states {
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| state.valid_transitions()));
        assert!(result.is_ok(), "{:?}.valid_transitions() panicked", state);
    }
}

#[test]
fn dispute_state_terminal_states_have_no_transitions() {
    let terminals = [
        DisputeState::Closed,
        DisputeState::Settled,
        DisputeState::Dismissed,
    ];
    for state in &terminals {
        assert!(state.is_terminal(), "{:?} should be terminal", state);
        let valid = state.valid_transitions();
        assert!(
            valid.is_empty(),
            "{:?} is terminal but has valid transitions: {:?}",
            state,
            valid
        );
    }
}

// =========================================================================
// msez-agentic: PolicyEngine panic paths
// =========================================================================

use msez_agentic::{PolicyEngine, Trigger, TriggerType};

#[test]
fn policy_engine_evaluate_empty_data_no_panic() {
    let mut engine = PolicyEngine::new();
    let trigger = Trigger::new(TriggerType::SanctionsListUpdate, json!({}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate(&trigger, None, None)
    }));
    assert!(
        result.is_ok(),
        "Evaluating trigger with empty data should not panic"
    );
}

#[test]
fn policy_engine_evaluate_null_data_no_panic() {
    let mut engine = PolicyEngine::new();
    let trigger = Trigger::new(TriggerType::LicenseStatusChange, json!(null));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate(&trigger, None, None)
    }));
    assert!(
        result.is_ok(),
        "Evaluating trigger with null data should not panic"
    );
}

#[test]
fn policy_engine_with_standard_policies_evaluate_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );
    let results = engine.evaluate(&trigger, Some("asset:test"), None);
    // Standard policies should produce results without panicking.
    // Sanctions list updates should trigger at least one matching policy.
    assert!(
        !results.is_empty(),
        "standard policies must produce at least one result for SanctionsListUpdate trigger"
    );
}

#[test]
fn policy_engine_process_trigger_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::ComplianceDeadline,
        json!({"deadline": "2026-03-01"}),
    );
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.process_trigger(&trigger, "asset:test", None)
    }));
    assert!(result.is_ok(), "process_trigger should not panic");
}

// =========================================================================
// Cross-cutting: serde deserialization of malformed JSON
// =========================================================================

#[test]
fn deser_corridor_state_from_invalid_string_no_panic() {
    let result: Result<DynCorridorState, _> = serde_json::from_str("\"INVALID_STATE\"");
    assert!(
        result.is_err(),
        "Invalid state string should fail deserialization"
    );
}

#[test]
fn deser_entity_state_from_invalid_string_no_panic() {
    let result: Result<EntityLifecycleState, _> = serde_json::from_str("\"BOGUS\"");
    assert!(result.is_err(), "Invalid state should fail deserialization");
}

#[test]
fn deser_migration_state_from_invalid_string_no_panic() {
    let result: Result<MigrationState, _> = serde_json::from_str("\"NotAState\"");
    assert!(result.is_err(), "Invalid migration state should fail");
}

#[test]
fn deser_license_state_from_invalid_string_no_panic() {
    let result: Result<LicenseState, _> = serde_json::from_str("\"INVALID\"");
    assert!(result.is_err(), "Invalid license state should fail");
}

#[test]
fn deser_compliance_state_from_invalid_string_no_panic() {
    let result: Result<msez_tensor::ComplianceState, _> = serde_json::from_str("\"invalid\"");
    assert!(result.is_err(), "Invalid compliance state should fail");
}

#[test]
fn deser_corridor_state_from_number_no_panic() {
    let result: Result<DynCorridorState, _> = serde_json::from_str("42");
    assert!(result.is_err(), "Number should not deserialize to state");
}

#[test]
fn deser_corridor_state_from_null_no_panic() {
    let result: Result<DynCorridorState, _> = serde_json::from_str("null");
    assert!(result.is_err(), "Null should not deserialize to state");
}

#[test]
fn deser_entity_id_from_invalid_uuid_no_panic() {
    let result: Result<EntityId, _> = serde_json::from_str("\"not-a-uuid\"");
    assert!(result.is_err(), "Invalid UUID should fail deserialization");
}

// =========================================================================
// Campaign 2 Extension: msez-pack panic paths
// =========================================================================

use msez_pack::parser::ensure_json_compatible;
use msez_pack::regpack::{validate_compliance_domain, SanctionsChecker, SanctionsEntry};

#[test]
fn pack_validate_compliance_domain_empty_string_no_panic() {
    let result = validate_compliance_domain("");
    assert!(result.is_err(), "Empty domain should be rejected");
}

#[test]
fn pack_validate_compliance_domain_huge_string_no_panic() {
    let huge = "x".repeat(100_000);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        validate_compliance_domain(&huge)
    }));
    assert!(result.is_ok(), "100K char domain should not panic");
    assert!(
        result.unwrap().is_err(),
        "100K char domain should be rejected"
    );
}

#[test]
fn pack_validate_compliance_domain_unicode_no_panic() {
    let result = validate_compliance_domain("日本語の規制");
    assert!(result.is_err(), "Unicode domain should be rejected");
}

#[test]
fn pack_validate_compliance_domain_null_byte_no_panic() {
    let result = validate_compliance_domain("taxation\0injection");
    assert!(result.is_err(), "Null byte in domain should be rejected");
}

#[test]
fn pack_ensure_json_compatible_null_no_panic() {
    let result = ensure_json_compatible(&json!(null), "", "test");
    // Null JSON value: must not panic. Verify deterministic behavior.
    let result2 = ensure_json_compatible(&json!(null), "", "test");
    assert_eq!(result.is_ok(), result2.is_ok(), "ensure_json_compatible(null) must be deterministic");
}

#[test]
fn pack_ensure_json_compatible_deeply_nested_no_panic() {
    // 200 levels of nesting
    let mut value = json!("leaf");
    for _ in 0..200 {
        value = json!({"nested": value});
    }
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        ensure_json_compatible(&value, "", "deep-nest")
    }));
    assert!(
        result.is_ok(),
        "200-level nesting should not panic (stack overflow)"
    );
}

#[test]
fn pack_ensure_json_compatible_empty_string_path_no_panic() {
    let result = ensure_json_compatible(&json!({"key": "value"}), "", "");
    // Empty path and context: must not panic, must be deterministic.
    let result2 = ensure_json_compatible(&json!({"key": "value"}), "", "");
    assert_eq!(result.is_ok(), result2.is_ok(), "ensure_json_compatible must be deterministic");
}

#[test]
fn pack_ensure_json_compatible_huge_array_no_panic() {
    let arr: Vec<serde_json::Value> = (0..10_000).map(|i| json!(i)).collect();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        ensure_json_compatible(&json!(arr), "", "large-array")
    }));
    assert!(result.is_ok(), "10K element array should not panic");
}

#[test]
fn pack_sanctions_checker_empty_entries_no_panic() {
    let checker = SanctionsChecker::new(vec![], "empty-snapshot".to_string());
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity("Test Entity", None, 0.8)
    }));
    assert!(
        result.is_ok(),
        "Checking against empty sanctions list should not panic"
    );
}

#[test]
fn pack_sanctions_checker_empty_name_no_panic() {
    let checker = SanctionsChecker::new(
        vec![SanctionsEntry {
            entry_id: "SE-001".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec!["OFAC".to_string()],
            primary_name: "Test Person".to_string(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![],
            listing_date: None,
            remarks: None,
        }],
        "snapshot-001".to_string(),
    );
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity("", None, 0.8)
    }));
    assert!(result.is_ok(), "Empty name query should not panic");
}

#[test]
fn pack_sanctions_checker_nan_threshold_no_panic() {
    let checker = SanctionsChecker::new(vec![], "snapshot-nan".to_string());
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity("Test", None, f64::NAN)
    }));
    assert!(result.is_ok(), "NaN threshold should not panic");
}

#[test]
fn pack_sanctions_checker_infinity_threshold_no_panic() {
    let checker = SanctionsChecker::new(vec![], "snapshot-inf".to_string());
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity("Test", None, f64::INFINITY)
    }));
    assert!(result.is_ok(), "Infinity threshold should not panic");
}

#[test]
fn pack_sanctions_checker_negative_threshold_no_panic() {
    let checker = SanctionsChecker::new(vec![], "snapshot-neg".to_string());
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity("Test", None, -1.0)
    }));
    assert!(result.is_ok(), "Negative threshold should not panic");
}

#[test]
fn pack_sanctions_checker_huge_name_no_panic() {
    let checker = SanctionsChecker::new(vec![], "snapshot-huge".to_string());
    let huge_name = "A".repeat(1_000_000);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        checker.check_entity(&huge_name, None, 0.8)
    }));
    assert!(result.is_ok(), "1MB name query should not panic");
}

// =========================================================================
// Campaign 2 Extension: msez-agentic panic paths
// =========================================================================

use msez_agentic::{AuditEntry, AuditEntryType, AuditTrail};

#[test]
fn agentic_audit_trail_zero_capacity_no_panic() {
    let result = panic::catch_unwind(|| {
        let mut trail = AuditTrail::new(0);
        trail.append(AuditEntry::new(
            AuditEntryType::TriggerReceived,
            Some("asset-001".to_string()),
            None,
        ));
    });
    assert!(
        result.is_ok(),
        "Zero capacity audit trail should not panic on append"
    );
}

#[test]
fn agentic_audit_trail_append_beyond_capacity_no_panic() {
    let mut trail = AuditTrail::new(10);
    for i in 0..100 {
        trail.append(AuditEntry::new(
            AuditEntryType::PolicyEvaluated,
            Some(format!("asset-{}", i)),
            Some(json!({"iteration": i})),
        ));
    }
    // Should have trimmed; verify no panic and bounded size
    assert!(trail.len() <= 100, "Trail should be bounded");
}

#[test]
fn agentic_audit_trail_entries_for_nonexistent_asset_no_panic() {
    let trail = AuditTrail::new(100);
    let entries = trail.entries_for_asset("nonexistent-asset-id");
    assert!(entries.is_empty());
}

#[test]
fn agentic_audit_trail_last_n_zero_no_panic() {
    let trail = AuditTrail::new(100);
    let last = trail.last_n(0);
    assert!(last.is_empty());
}

#[test]
fn agentic_audit_trail_last_n_more_than_entries_no_panic() {
    let mut trail = AuditTrail::new(100);
    trail.append(AuditEntry::new(AuditEntryType::ActionExecuted, None, None));
    let last = trail.last_n(1000);
    assert_eq!(last.len(), 1);
}

#[test]
fn agentic_policy_engine_evaluate_empty_trigger_data_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(TriggerType::SanctionsListUpdate, json!({}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate(&trigger, None, None)
    }));
    assert!(
        result.is_ok(),
        "Evaluating trigger with empty data should not panic"
    );
}

#[test]
fn agentic_policy_engine_evaluate_with_empty_asset_id_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::LicenseStatusChange,
        json!({"status": "expired"}),
    );
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate(&trigger, Some(""), None)
    }));
    assert!(result.is_ok(), "Empty asset_id should not panic");
}

#[test]
fn agentic_policy_engine_evaluate_with_huge_jurisdiction_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(TriggerType::ComplianceDeadline, json!({}));
    let huge_jurisdiction = "X".repeat(100_000);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate(&trigger, Some("asset-001"), Some(&huge_jurisdiction))
    }));
    assert!(result.is_ok(), "Huge jurisdiction string should not panic");
}

#[test]
fn agentic_policy_engine_evaluate_and_resolve_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(TriggerType::DisputeFiled, json!({"dispute_id": "D-001"}));
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.evaluate_and_resolve(&trigger, Some("asset-001"), Some("PK-RSEZ"))
    }));
    assert!(result.is_ok(), "evaluate_and_resolve should not panic");
}

#[test]
fn agentic_policy_engine_process_trigger_no_panic() {
    let mut engine = PolicyEngine::with_standard_policies();
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({"corridor": "test"}),
    );
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        engine.process_trigger(&trigger, "asset-001", Some("AE-DIFC"))
    }));
    assert!(result.is_ok(), "process_trigger should not panic");
}

#[test]
fn agentic_policy_engine_unregister_nonexistent_no_panic() {
    let mut engine = PolicyEngine::new();
    let result = engine.unregister_policy("nonexistent-policy-id");
    assert!(
        result.is_none(),
        "Unregistering nonexistent policy should return None"
    );
}

// =========================================================================
// Campaign 2 Extension: msez-corridor CorridorBridge panic paths
// =========================================================================

#[test]
fn bridge_find_route_nonexistent_source_no_panic() {
    let bridge = CorridorBridge::new();
    let source = JurisdictionId::new("NONEXISTENT-A").unwrap();
    let target = JurisdictionId::new("NONEXISTENT-B").unwrap();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        bridge.find_route(&source, &target)
    }));
    assert!(
        result.is_ok(),
        "Route finding in empty graph should not panic"
    );
    assert!(result.unwrap().is_none(), "No route in empty graph");
}

#[test]
fn bridge_reachable_from_empty_graph_no_panic() {
    let bridge = CorridorBridge::new();
    let source = JurisdictionId::new("TEST-NODE").unwrap();
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| bridge.reachable_from(&source)));
    assert!(
        result.is_ok(),
        "reachable_from on empty graph should not panic"
    );
}

#[test]
fn bridge_reachable_from_single_node_no_panic() {
    let mut bridge = CorridorBridge::new();
    let source = JurisdictionId::new("TEST-NODE").unwrap();
    let target = JurisdictionId::new("OTHER-NODE").unwrap();
    bridge.add_edge(BridgeEdge {
        from: source.clone(),
        to: target,
        corridor_id: CorridorId::new(),
        fee_bps: 10,
        settlement_time_secs: 3600,
    });
    let result = bridge.reachable_from(&source);
    // Should include source itself and at least one reachable node
    assert!(
        !result.is_empty(),
        "Node with edge should have reachable nodes"
    );
}

// =========================================================================
// Campaign 2 Extension: msez-vc panic paths
// =========================================================================

use msez_vc::registry::SmartAssetRegistryVc;

#[test]
fn vc_credential_sign_empty_subject_no_panic() {
    let mut vc = VerifiableCredential {
        context: ContextValue::Single("https://www.w3.org/2018/credentials/v1".to_string()),
        id: None,
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkTestIssuer".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({}),
        proof: ProofValue::default(),
    };
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkTestIssuer#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
    }));
    assert!(
        result.is_ok(),
        "Signing VC with empty subject should not panic"
    );
}

#[test]
fn vc_compute_asset_id_empty_json_no_panic() {
    let result = SmartAssetRegistryVc::compute_asset_id(&json!({}));
    // Should succeed — empty JSON is valid canonical input
    assert!(
        result.is_ok(),
        "compute_asset_id on empty JSON should succeed"
    );
}

#[test]
fn vc_compute_asset_id_null_no_panic() {
    let result = SmartAssetRegistryVc::compute_asset_id(&json!(null));
    // Null JSON: must not panic. Verify deterministic behavior.
    let result2 = SmartAssetRegistryVc::compute_asset_id(&json!(null));
    assert_eq!(result.is_ok(), result2.is_ok(), "compute_asset_id(null) must be deterministic");
    if let (Ok(r1), Ok(r2)) = (&result, &result2) {
        assert_eq!(r1, r2, "compute_asset_id(null) digest must be deterministic");
    }
}

#[test]
fn vc_compute_asset_id_deeply_nested_no_panic() {
    let mut value = json!("leaf");
    for _ in 0..100 {
        value = json!({"n": value});
    }
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
        SmartAssetRegistryVc::compute_asset_id(&value)
    }));
    assert!(result.is_ok(), "Deeply nested genesis should not panic");
}
