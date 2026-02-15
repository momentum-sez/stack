//! # Campaign 5: Boundary and Adversarial Inputs
//! # Campaign 6: Determinism Verification
//!
//! Tests for edge-case inputs (financial overflow, empty strings, Unicode,
//! UUID boundaries) and determinism verification (same input → same output).

use msez_core::{
    CanonicalBytes, ComplianceDomain, ContentDigest, JurisdictionId,
};
use msez_crypto::{sha256_digest, MerkleMountainRange, SigningKey};
use msez_corridor::netting::{NettingEngine, Obligation};
use msez_corridor::swift::{SettlementInstruction, SettlementRail, SwiftPacs008};
use msez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};
use msez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use serde_json::json;
use std::panic;

// =========================================================================
// Campaign 5: Financial amount boundaries
// =========================================================================

#[test]
fn netting_i64_max_overflow_two_obligations() {
    // BUG-018 RESOLVED: Two obligations that sum to > i64::MAX now return
    // NettingError::ArithmeticOverflow instead of panicking or wrapping.
    let mut engine = NettingEngine::new();
    engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: i64::MAX / 2 + 1,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    }).unwrap();
    engine.add_obligation(Obligation {
        from_party: "C".to_string(),
        to_party: "D".to_string(),
        amount: i64::MAX / 2 + 1,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    }).unwrap();

    let result = engine.compute_plan();
    assert!(result.is_err(), "BUG-018 RESOLVED: overflow must return error");
    let err_msg = format!("{}", result.unwrap_err());
    assert!(err_msg.contains("arithmetic overflow"), "expected ArithmeticOverflow, got: {err_msg}");
}

#[test]
fn netting_many_small_obligations_no_overflow() {
    let mut engine = NettingEngine::new();
    // 1000 unique obligations well within i64 range.
    // Use unique (from, to, amount) to avoid duplicate detection.
    for i in 0..1000 {
        engine.add_obligation(Obligation {
            from_party: format!("party_{}", i % 10),
            to_party: format!("party_{}", (i + 1) % 10),
            amount: 1_000_000 + i as i64, // unique amount per obligation
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        }).unwrap();
    }
    let plan = engine.compute_plan().unwrap();
    // Sum of (1_000_000 + i) for i in 0..1000 = 1000*1_000_000 + 999*1000/2 = 1_000_499_500
    assert_eq!(plan.gross_total, 1_000_499_500);
}

#[test]
fn netting_single_obligation_amount_1() {
    let mut engine = NettingEngine::new();
    engine.add_obligation(Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: 1,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    }).unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 1);
    assert_eq!(plan.net_total, 1);
}

#[test]
fn swift_i64_max_amount_no_panic() {
    let swift = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test".to_string(),
        amount: i64::MAX,
        currency: "USD".to_string(),
        remittance_info: None,
    };
    // Should not panic even with huge amount
    let result = swift.generate_instruction(&instruction);
    assert!(result.is_ok(), "i64::MAX amount should generate XML without error");
}

// =========================================================================
// Campaign 5: String input boundaries
// =========================================================================

#[test]
fn jurisdiction_id_unicode_no_panic() {
    let _ = JurisdictionId::new("日本語");
    let _ = JurisdictionId::new("🏛️");
    let _ = JurisdictionId::new("a\0b"); // embedded null
}

#[test]
fn jurisdiction_id_very_long_string() {
    let long = "A".repeat(10_000);
    let result = JurisdictionId::new(&long);
    // Should not panic; may or may not be accepted
    let _ = result;
}

#[test]
fn canonical_bytes_xss_payload_no_interpretation() {
    let xss = "<script>alert(1)</script>";
    let canonical = CanonicalBytes::new(&json!({"input": xss})).unwrap();
    let bytes_str = String::from_utf8_lossy(canonical.as_bytes());
    // The XSS payload should be preserved as-is in canonical bytes (no interpretation)
    assert!(
        bytes_str.contains(xss),
        "XSS payload should pass through without interpretation"
    );
}

#[test]
fn canonical_bytes_sql_injection_no_interpretation() {
    let sqli = "'; DROP TABLE entities; --";
    let canonical = CanonicalBytes::new(&json!({"input": sqli})).unwrap();
    let bytes_str = String::from_utf8_lossy(canonical.as_bytes());
    assert!(
        bytes_str.contains("DROP TABLE"),
        "SQL injection payload should pass through without interpretation"
    );
}

#[test]
fn netting_unicode_party_ids() {
    let mut engine = NettingEngine::new();
    engine.add_obligation(Obligation {
        from_party: "企業A".to_string(),
        to_party: "企業B".to_string(),
        amount: 100,
        currency: "JPY".to_string(),
        corridor_id: None,
        priority: 0,
    }).unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 100);
}

#[test]
fn netting_very_long_party_ids() {
    let mut engine = NettingEngine::new();
    let long_a = "A".repeat(10_000);
    let long_b = "B".repeat(10_000);
    engine.add_obligation(Obligation {
        from_party: long_a,
        to_party: long_b,
        amount: 100,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    }).unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 100);
}

// =========================================================================
// Campaign 5: UUID boundaries
// =========================================================================

#[test]
fn entity_id_nil_uuid() {
    // EntityId::new() generates a random UUID, but we can test nil UUID via serde
    let nil_uuid = "00000000-0000-0000-0000-000000000000";
    let result: Result<msez_core::EntityId, _> =
        serde_json::from_str(&format!("\"{}\"", nil_uuid));
    // Nil UUID should be accepted or explicitly rejected, never panic
    let _ = result;
}

// =========================================================================
// Campaign 5: Timestamp boundaries
// =========================================================================

use msez_core::Timestamp;

#[test]
fn timestamp_epoch_zero() {
    let result = Timestamp::from_rfc3339("1970-01-01T00:00:00Z");
    // Epoch zero should be valid
    assert!(result.is_ok(), "Unix epoch should be a valid timestamp");
}

#[test]
fn timestamp_far_future() {
    let result = Timestamp::from_rfc3339("9999-12-31T23:59:59Z");
    // Far future should be valid
    assert!(result.is_ok(), "Year 9999 should be a valid timestamp");
}

#[test]
fn timestamp_leap_second() {
    let result = Timestamp::from_rfc3339("2015-06-30T23:59:60Z");
    // Leap second — chrono may or may not support this
    let _ = result;
}

// =========================================================================
// Campaign 5: Hex digest boundaries
// =========================================================================

#[test]
fn content_digest_all_zeros() {
    let hex = "00".repeat(32);
    let result = ContentDigest::from_hex(&hex);
    assert!(result.is_ok(), "All-zero digest should be valid");
}

#[test]
fn content_digest_all_ff() {
    let hex = "ff".repeat(32);
    let result = ContentDigest::from_hex(&hex);
    assert!(result.is_ok(), "All-ff digest should be valid");
}

#[test]
fn content_digest_mixed_case() {
    let hex = "aAbBcCdDeEfF".to_string() + &"00".repeat(26);
    let result = ContentDigest::from_hex(&hex);
    // Mixed case may or may not be accepted
    let _ = result;
}

// =========================================================================
// Campaign 5: Compliance tensor boundaries
// =========================================================================

#[test]
fn tensor_evaluate_all_20_domains() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    let all = tensor.evaluate_all("entity-001");
    // Should have entries for all 20 domains
    assert_eq!(
        all.len(),
        20,
        "Tensor should evaluate all 20 compliance domains, got {}",
        all.len()
    );
}

#[test]
fn tensor_set_then_overwrite_domain() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));

    tensor.set(ComplianceDomain::Kyc, ComplianceState::Compliant, vec![], None);
    assert_eq!(tensor.get(ComplianceDomain::Kyc), ComplianceState::Compliant);

    // Overwrite to NonCompliant
    tensor.set(ComplianceDomain::Kyc, ComplianceState::NonCompliant, vec![], None);
    assert_eq!(
        tensor.get(ComplianceDomain::Kyc),
        ComplianceState::NonCompliant,
        "Overwriting domain state should update the value"
    );
}

// =========================================================================
// Campaign 6: Determinism Verification
// =========================================================================

#[test]
fn canonical_bytes_deterministic_100_runs() {
    let input = json!({"b": 2, "a": 1, "nested": {"z": 26, "a": 1}});
    let mut results = Vec::new();
    for _ in 0..100 {
        let canonical = CanonicalBytes::new(&input).unwrap();
        results.push(canonical.as_bytes().to_vec());
    }
    assert!(
        results.windows(2).all(|w| w[0] == w[1]),
        "CanonicalBytes produced different output for identical input"
    );
}

#[test]
fn sha256_digest_deterministic_100_runs() {
    let canonical = CanonicalBytes::new(&json!({"test": "determinism"})).unwrap();
    let mut digests = Vec::new();
    for _ in 0..100 {
        let digest = sha256_digest(&canonical);
        digests.push(digest.to_hex());
    }
    assert!(
        digests.windows(2).all(|w| w[0] == w[1]),
        "sha256_digest produced different output for identical input"
    );
}

#[test]
fn netting_deterministic_same_order_100_runs() {
    let mut plans = Vec::new();
    for _ in 0..100 {
        let mut engine = NettingEngine::new();
        engine.add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 500_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        }).unwrap();
        engine.add_obligation(Obligation {
            from_party: "B".to_string(),
            to_party: "C".to_string(),
            amount: 300_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        }).unwrap();
        engine.add_obligation(Obligation {
            from_party: "C".to_string(),
            to_party: "A".to_string(),
            amount: 200_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        }).unwrap();
        let plan = engine.compute_plan().unwrap();
        plans.push((plan.gross_total, plan.net_total, plan.settlement_legs.len()));
    }
    assert!(
        plans.windows(2).all(|w| w[0] == w[1]),
        "NettingEngine produced different plans for identical obligations in same order"
    );
}

#[test]
fn netting_deterministic_different_insertion_order() {
    // BUG-024: If NettingEngine uses HashMap internally, different insertion order
    // could produce different settlement plans. This tests that invariant.
    let make_plan = |obligations: Vec<(String, String, i64)>| {
        let mut engine = NettingEngine::new();
        for (from, to, amount) in obligations {
            engine.add_obligation(Obligation {
                from_party: from,
                to_party: to,
                amount,
                currency: "USD".to_string(),
                corridor_id: None,
                priority: 0,
            }).unwrap();
        }
        engine.compute_plan().unwrap()
    };

    let plan1 = make_plan(vec![
        ("A".into(), "B".into(), 100),
        ("B".into(), "C".into(), 80),
        ("C".into(), "A".into(), 60),
    ]);

    let plan2 = make_plan(vec![
        ("C".into(), "A".into(), 60),
        ("A".into(), "B".into(), 100),
        ("B".into(), "C".into(), 80),
    ]);

    let plan3 = make_plan(vec![
        ("B".into(), "C".into(), 80),
        ("C".into(), "A".into(), 60),
        ("A".into(), "B".into(), 100),
    ]);

    // Gross totals and net totals must be identical regardless of insertion order
    assert_eq!(plan1.gross_total, plan2.gross_total);
    assert_eq!(plan2.gross_total, plan3.gross_total);
    assert_eq!(plan1.net_total, plan2.net_total);
    assert_eq!(plan2.net_total, plan3.net_total);

    // Net positions should have the same values (may be in different order)
    let mut nets1: Vec<(String, i64)> = plan1.net_positions.iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    let mut nets2: Vec<(String, i64)> = plan2.net_positions.iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    let mut nets3: Vec<(String, i64)> = plan3.net_positions.iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    nets1.sort();
    nets2.sort();
    nets3.sort();
    assert_eq!(nets1, nets2, "Net positions differ between insertion orders 1 and 2");
    assert_eq!(nets2, nets3, "Net positions differ between insertion orders 2 and 3");

    // Settlement legs should be equivalent (same amounts, possibly different order)
    let mut legs1: Vec<(String, String, i64)> = plan1.settlement_legs.iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    let mut legs2: Vec<(String, String, i64)> = plan2.settlement_legs.iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    let mut legs3: Vec<(String, String, i64)> = plan3.settlement_legs.iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    legs1.sort();
    legs2.sort();
    legs3.sort();
    assert_eq!(legs1, legs2, "BUG-024: Settlement legs differ between insertion orders 1 and 2");
    assert_eq!(legs2, legs3, "BUG-024: Settlement legs differ between insertion orders 2 and 3");
}

#[test]
fn mmr_deterministic_100_runs() {
    let leaf1 = {
        let c = CanonicalBytes::new(&json!({"leaf": 1})).unwrap();
        sha256_digest(&c).to_hex()
    };
    let leaf2 = {
        let c = CanonicalBytes::new(&json!({"leaf": 2})).unwrap();
        sha256_digest(&c).to_hex()
    };
    let leaf3 = {
        let c = CanonicalBytes::new(&json!({"leaf": 3})).unwrap();
        sha256_digest(&c).to_hex()
    };

    let mut roots = Vec::new();
    for _ in 0..100 {
        let mut mmr = MerkleMountainRange::new();
        mmr.append(&leaf1).unwrap();
        mmr.append(&leaf2).unwrap();
        mmr.append(&leaf3).unwrap();
        roots.push(mmr.root().unwrap());
    }
    assert!(
        roots.windows(2).all(|w| w[0] == w[1]),
        "MMR produced different roots for identical leaves"
    );
}

#[test]
fn compliance_tensor_commitment_deterministic_100_runs() {
    let states: Vec<(ComplianceDomain, ComplianceState)> = vec![
        (ComplianceDomain::Kyc, ComplianceState::Compliant),
        (ComplianceDomain::Aml, ComplianceState::Compliant),
        (ComplianceDomain::Sanctions, ComplianceState::Pending),
    ];

    let mut digests = Vec::new();
    for _ in 0..100 {
        let digest = msez_tensor::commitment_digest("PK-RSEZ", &states).unwrap();
        digests.push(digest.to_hex());
    }
    assert!(
        digests.windows(2).all(|w| w[0] == w[1]),
        "Compliance tensor commitment produced different digests for identical state"
    );
}

#[test]
fn vc_signing_deterministic() {
    // Note: Ed25519 signatures are deterministic (RFC 8032), so signing the
    // same message with the same key should produce the same signature.
    let sk = SigningKey::from_bytes(&[42u8; 32]);
    let _vk = sk.verifying_key();

    let make_vc = || {
        let mut vc = VerifiableCredential {
            context: ContextValue::default(),
            id: Some("urn:vc:determinism-test".to_string()),
            credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
            issuer: "did:key:z6MkDeterminism".to_string(),
            issuance_date: chrono::DateTime::parse_from_rfc3339("2026-01-15T00:00:00Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            expiration_date: None,
            credential_subject: json!({"entity": "test", "amount": 1000}),
            proof: ProofValue::default(),
        };
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkDeterminism#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            Some(
                msez_core::Timestamp::from_rfc3339("2026-01-15T12:00:00Z").unwrap(),
            ),
        )
        .unwrap();
        serde_json::to_string(&vc).unwrap()
    };

    let json1 = make_vc();
    let json2 = make_vc();
    assert_eq!(
        json1, json2,
        "Signing the same VC with the same key and timestamp should produce identical JSON"
    );
}

#[test]
fn swift_xml_deterministic_100_runs() {
    let swift = SwiftPacs008::new("MSEZSEXX");
    let instruction = SettlementInstruction {
        message_id: "MSG-DET-001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Determinism Test GmbH".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678901234".to_string(),
        creditor_name: "Determinism Test Ltd".to_string(),
        amount: 500_000,
        currency: "USD".to_string(),
        remittance_info: Some("Test settlement".to_string()),
    };

    let mut xmls = Vec::new();
    for _ in 0..100 {
        let xml = swift.generate_instruction(&instruction).unwrap();
        xmls.push(xml);
    }
    assert!(
        xmls.windows(2).all(|w| w[0] == w[1]),
        "SWIFT pacs.008 generated different XML for identical instruction"
    );
}

// =========================================================================
// Campaign 5: Adversarial serde inputs
// =========================================================================

#[test]
fn canonical_bytes_with_duplicate_keys() {
    // JSON with duplicate keys — behavior is implementation-defined
    // but should not panic
    let raw = r#"{"a": 1, "a": 2}"#;
    let value: serde_json::Value = serde_json::from_str(raw).unwrap();
    let result = CanonicalBytes::new(&value);
    assert!(result.is_ok(), "Duplicate keys should not cause panic");
}

#[test]
fn canonical_bytes_empty_string_values() {
    let result = CanonicalBytes::new(&json!({"": "", "a": ""}));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_numeric_extremes() {
    // Test with extreme numeric values
    let result = CanonicalBytes::new(&json!({
        "max_u64": u64::MAX,
        "min_i64": i64::MIN,
        "zero": 0
    }));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_boolean_and_null() {
    let result = CanonicalBytes::new(&json!({
        "true": true,
        "false": false,
        "null": null
    }));
    assert!(result.is_ok());
}

#[test]
fn canonical_bytes_array_with_mixed_types() {
    let result = CanonicalBytes::new(&json!([1, "two", true, null, {"nested": []}]));
    assert!(result.is_ok());
}
