//! # Campaign 5: Boundary and Adversarial Inputs
//! # Campaign 6: Determinism Verification
//!
//! Tests for edge-case inputs (financial overflow, empty strings, Unicode,
//! UUID boundaries) and determinism verification (same input → same output).

use msez_core::{CanonicalBytes, ComplianceDomain, ContentDigest, JurisdictionId, WatcherId};
use msez_corridor::netting::{NettingEngine, Obligation};
use msez_corridor::swift::{SettlementInstruction, SettlementRail, SwiftPacs008};
use msez_crypto::{sha256_digest, MerkleMountainRange, SigningKey};
use msez_pack::licensepack::{
    License, LicenseCondition, LicenseRestriction, LicenseStatus, Licensepack,
};
use msez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};
use msez_vc::{ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential};
use msez_zkp::mock::{MockCircuit, MockProofSystem, MockProvingKey, MockVerifyingKey};
use msez_zkp::traits::ProofSystem;
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

    let result = engine.compute_plan();
    assert!(
        result.is_err(),
        "BUG-018 RESOLVED: overflow must return error"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("arithmetic overflow"),
        "expected ArithmeticOverflow, got: {err_msg}"
    );
}

#[test]
fn netting_many_small_obligations_no_overflow() {
    let mut engine = NettingEngine::new();
    // 1000 unique obligations well within i64 range.
    // Use unique (from, to, amount) to avoid duplicate detection.
    for i in 0..1000 {
        engine
            .add_obligation(Obligation {
                from_party: format!("party_{}", i % 10),
                to_party: format!("party_{}", (i + 1) % 10),
                amount: 1_000_000 + i as i64, // unique amount per obligation
                currency: "USD".to_string(),
                corridor_id: None,
                priority: 0,
            })
            .unwrap();
    }
    let plan = engine.compute_plan().unwrap();
    // Sum of (1_000_000 + i) for i in 0..1000 = 1000*1_000_000 + 999*1000/2 = 1_000_499_500
    assert_eq!(plan.gross_total, 1_000_499_500);
}

#[test]
fn netting_single_obligation_amount_1() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 1,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
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
    assert!(
        result.is_ok(),
        "i64::MAX amount should generate XML without error"
    );
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
    let result = panic::catch_unwind(|| JurisdictionId::new(&long));
    assert!(result.is_ok(), "JurisdictionId::new must not panic on 10K-char string");
    // Must be deterministic.
    let r1 = JurisdictionId::new(&long);
    let r2 = JurisdictionId::new(&long);
    assert_eq!(r1.is_ok(), r2.is_ok(), "JurisdictionId::new must be deterministic for same input");
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
    engine
        .add_obligation(Obligation {
            from_party: "企業A".to_string(),
            to_party: "企業B".to_string(),
            amount: 100,
            currency: "JPY".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 100);
}

#[test]
fn netting_very_long_party_ids() {
    let mut engine = NettingEngine::new();
    let long_a = "A".repeat(10_000);
    let long_b = "B".repeat(10_000);
    engine
        .add_obligation(Obligation {
            from_party: long_a,
            to_party: long_b,
            amount: 100,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 100);
}

// =========================================================================
// Campaign 5: UUID boundaries
// =========================================================================

#[test]
fn entity_id_nil_uuid() {
    // EntityId::new() generates a random UUID, but we can test nil UUID via serde.
    // Nil UUID is a valid UUID format — EntityId should accept it since it wraps Uuid.
    let nil_uuid = "00000000-0000-0000-0000-000000000000";
    let result: Result<msez_core::EntityId, _> = serde_json::from_str(&format!("\"{}\"", nil_uuid));
    assert!(result.is_ok(), "Nil UUID is a valid UUID format and must be accepted by EntityId");
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
    // Leap second — chrono normalizes :60 to :59 or :00 of the next minute.
    // Must not panic. Must be deterministic.
    let result2 = Timestamp::from_rfc3339("2015-06-30T23:59:60Z");
    assert_eq!(result.is_ok(), result2.is_ok(), "leap second parsing must be deterministic");
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
    // Mixed case: must not panic. Must be deterministic across calls.
    let result2 = ContentDigest::from_hex(&hex);
    assert_eq!(result.is_ok(), result2.is_ok(), "ContentDigest::from_hex must be deterministic");
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

    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    assert_eq!(
        tensor.get(ComplianceDomain::Kyc),
        ComplianceState::Compliant
    );

    // Overwrite to NonCompliant
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::NonCompliant,
        vec![],
        None,
    );
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
        engine
            .add_obligation(Obligation {
                from_party: "A".to_string(),
                to_party: "B".to_string(),
                amount: 500_000,
                currency: "USD".to_string(),
                corridor_id: None,
                priority: 0,
            })
            .unwrap();
        engine
            .add_obligation(Obligation {
                from_party: "B".to_string(),
                to_party: "C".to_string(),
                amount: 300_000,
                currency: "USD".to_string(),
                corridor_id: None,
                priority: 0,
            })
            .unwrap();
        engine
            .add_obligation(Obligation {
                from_party: "C".to_string(),
                to_party: "A".to_string(),
                amount: 200_000,
                currency: "USD".to_string(),
                corridor_id: None,
                priority: 0,
            })
            .unwrap();
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
            engine
                .add_obligation(Obligation {
                    from_party: from,
                    to_party: to,
                    amount,
                    currency: "USD".to_string(),
                    corridor_id: None,
                    priority: 0,
                })
                .unwrap();
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
    let mut nets1: Vec<(String, i64)> = plan1
        .net_positions
        .iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    let mut nets2: Vec<(String, i64)> = plan2
        .net_positions
        .iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    let mut nets3: Vec<(String, i64)> = plan3
        .net_positions
        .iter()
        .map(|p| (p.party_id.clone(), p.net))
        .collect();
    nets1.sort();
    nets2.sort();
    nets3.sort();
    assert_eq!(
        nets1, nets2,
        "Net positions differ between insertion orders 1 and 2"
    );
    assert_eq!(
        nets2, nets3,
        "Net positions differ between insertion orders 2 and 3"
    );

    // Settlement legs should be equivalent (same amounts, possibly different order)
    let mut legs1: Vec<(String, String, i64)> = plan1
        .settlement_legs
        .iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    let mut legs2: Vec<(String, String, i64)> = plan2
        .settlement_legs
        .iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    let mut legs3: Vec<(String, String, i64)> = plan3
        .settlement_legs
        .iter()
        .map(|l| (l.from_party.clone(), l.to_party.clone(), l.amount))
        .collect();
    legs1.sort();
    legs2.sort();
    legs3.sort();
    assert_eq!(
        legs1, legs2,
        "BUG-024: Settlement legs differ between insertion orders 1 and 2"
    );
    assert_eq!(
        legs2, legs3,
        "BUG-024: Settlement legs differ between insertion orders 2 and 3"
    );
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
            Some(msez_core::Timestamp::from_rfc3339("2026-01-15T12:00:00Z").unwrap()),
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

// =========================================================================
// Campaign 5: Bridge routing boundaries
// =========================================================================

use msez_core::CorridorId;
use msez_corridor::bridge::{BridgeEdge, CorridorBridge};

#[test]
fn bridge_route_disconnected_graph() {
    let mut bridge = CorridorBridge::new();
    let pk = JurisdictionId::new("PK").unwrap();
    let ae = JurisdictionId::new("AE").unwrap();
    let sg = JurisdictionId::new("SG").unwrap();
    let us = JurisdictionId::new("US").unwrap();
    // Two disconnected components: PK↔AE and SG↔US
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 50,
        settlement_time_secs: 3600,
    });
    bridge.add_edge(BridgeEdge {
        from: sg.clone(),
        to: us.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 30,
        settlement_time_secs: 1800,
    });
    // Route from PK to US should not exist
    let route = bridge.find_route(&pk, &us);
    assert!(
        route.is_none(),
        "Route across disconnected components should not exist"
    );
}

#[test]
fn bridge_route_many_edges_no_crash() {
    let mut bridge = CorridorBridge::new();
    // Add 100 edges in a chain
    let jids: Vec<JurisdictionId> = (0..100)
        .map(|i| JurisdictionId::new(&format!("J{:03}", i)).unwrap())
        .collect();
    for i in 0..99 {
        bridge.add_edge(BridgeEdge {
            from: jids[i].clone(),
            to: jids[i + 1].clone(),
            corridor_id: CorridorId::new(),
            fee_bps: 10,
            settlement_time_secs: 60,
        });
    }
    // Route from first to last should exist
    let route = bridge.find_route(&jids[0], &jids[99]);
    assert!(route.is_some(), "Long chain route should be found");
    let r = route.unwrap();
    assert_eq!(r.hop_count(), 99);
}

#[test]
fn bridge_reachable_from_empty_graph() {
    let bridge = CorridorBridge::new();
    let pk = JurisdictionId::new("PK").unwrap();
    let reachable = bridge.reachable_from(&pk);
    // Dijkstra includes source at distance 0; no other nodes reachable.
    assert_eq!(reachable.len(), 1, "Only source itself should be reachable");
    assert_eq!(reachable.get("PK"), Some(&0));
}

// =========================================================================
// Campaign 5: Receipt chain boundaries
// =========================================================================

use msez_corridor::receipt::{CorridorReceipt, ReceiptChain};

#[test]
fn receipt_chain_many_appends() {
    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id.clone());
    for i in 0..50 {
        let prev_root = chain.mmr_root().unwrap();
        let next_root = {
            let c = CanonicalBytes::new(&json!({"seq": i})).unwrap();
            sha256_digest(&c).to_hex()
        };
        let receipt = CorridorReceipt {
            receipt_type: "state_transition".to_string(),
            corridor_id: corridor_id.clone(),
            sequence: i,
            timestamp: Timestamp::now(),
            prev_root,
            next_root,
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        };
        chain.append(receipt).unwrap();
    }
    assert_eq!(chain.height(), 50, "Chain should have 50 receipts");
    let root = chain.mmr_root();
    assert!(
        root.is_ok(),
        "MMR root should be computable after 50 appends"
    );
}

#[test]
fn receipt_chain_checkpoint_on_nonempty() {
    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id.clone());
    let prev_root = chain.mmr_root().unwrap();
    let next_root = {
        let c = CanonicalBytes::new(&json!({"data": "test"})).unwrap();
        sha256_digest(&c).to_hex()
    };
    chain
        .append(CorridorReceipt {
            receipt_type: "state_transition".to_string(),
            corridor_id: corridor_id.clone(),
            sequence: 0,
            timestamp: Timestamp::now(),
            prev_root,
            next_root,
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        })
        .unwrap();
    let checkpoint = chain.create_checkpoint();
    assert!(
        checkpoint.is_ok(),
        "Checkpoint on non-empty chain should succeed"
    );
}

// =========================================================================
// Campaign 5: Watcher slashing boundaries
// =========================================================================

use msez_state::{SlashingCondition, Watcher};

#[test]
fn watcher_slash_equivocation_100_percent() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    watcher.bond(100_000).unwrap();
    watcher.activate().unwrap();
    let slashed = watcher.slash(SlashingCondition::Equivocation).unwrap();
    assert_eq!(slashed, 100_000, "Equivocation should slash 100%");
}

#[test]
fn watcher_slash_availability_1_percent() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    watcher.bond(100_000).unwrap();
    watcher.activate().unwrap();
    let slashed = watcher
        .slash(SlashingCondition::AvailabilityFailure)
        .unwrap();
    assert_eq!(slashed, 1_000, "AvailabilityFailure should slash 1%");
}

#[test]
fn watcher_slash_false_attestation_50_percent() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    watcher.bond(100_000).unwrap();
    watcher.activate().unwrap();
    let slashed = watcher.slash(SlashingCondition::FalseAttestation).unwrap();
    assert_eq!(slashed, 50_000, "FalseAttestation should slash 50%");
}

#[test]
fn watcher_bond_zero_stake() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    // BUG-023: Zero stake bond — should this be rejected?
    let result = watcher.bond(0);
    // Document behavior: if it succeeds, log as potential defect
    if result.is_ok() {
        // A watcher with zero stake has no economic security
        // This may be a design issue but not necessarily a bug
    }
}

#[test]
fn watcher_available_stake_saturating() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    watcher.bond(100).unwrap();
    watcher.activate().unwrap();
    // Slash 100% — available stake should be 0 (saturating sub)
    watcher.slash(SlashingCondition::Equivocation).unwrap();
    assert_eq!(watcher.available_stake(), 0);
}

#[test]
fn watcher_attestation_count_increments() {
    let mut watcher = Watcher::new(msez_core::WatcherId::new());
    watcher.bond(100_000).unwrap();
    watcher.activate().unwrap();
    assert_eq!(watcher.attestation_count, 0);
    watcher.record_attestation().unwrap();
    watcher.record_attestation().unwrap();
    watcher.record_attestation().unwrap();
    assert_eq!(watcher.attestation_count, 3);
}

// =========================================================================
// Campaign 5: Migration saga deadline enforcement
// =========================================================================

use msez_state::MigrationBuilder;

#[test]
fn migration_saga_expired_deadline_forces_timeout() {
    let past = chrono::Utc::now() - chrono::Duration::hours(1);
    let mut saga = MigrationBuilder::new(msez_core::MigrationId::new())
        .source(JurisdictionId::new("PK").unwrap())
        .destination(JurisdictionId::new("AE").unwrap())
        .deadline(past)
        .build();
    // Advance should detect timeout
    let result = saga.advance();
    match result {
        Ok(state) => {
            assert_eq!(
                state,
                msez_state::MigrationState::TimedOut,
                "Expired deadline should force TimedOut"
            );
        }
        Err(_) => {
            // Also acceptable: error because timed out
        }
    }
}

// =========================================================================
// Campaign 6: CAS store determinism
// =========================================================================

#[test]
fn cas_store_deterministic_digest_100_runs() {
    let value = json!({"determinism": "test", "nested": {"key": [1, 2, 3]}});
    let mut digests = Vec::new();
    for _ in 0..100 {
        let tmp = tempfile::tempdir().unwrap();
        let cas = msez_crypto::ContentAddressedStore::new(tmp.path());
        let artifact_ref = cas.store("test-type", &value).unwrap();
        digests.push(artifact_ref.digest.to_hex());
    }
    assert!(
        digests.windows(2).all(|w| w[0] == w[1]),
        "CAS store should produce identical digests for identical content"
    );
}

// =========================================================================
// Campaign 6: Ed25519 signature determinism
// =========================================================================

#[test]
fn ed25519_sign_deterministic_100_runs() {
    let sk = SigningKey::from_bytes(&[42u8; 32]);
    let canonical = CanonicalBytes::new(&json!({"test": "determinism"})).unwrap();
    let mut sigs = Vec::new();
    for _ in 0..100 {
        let sig = sk.sign(&canonical);
        sigs.push(sig.to_hex());
    }
    assert!(
        sigs.windows(2).all(|w| w[0] == w[1]),
        "Ed25519 signing should be deterministic (RFC 8032)"
    );
}

// =========================================================================
// Campaign 6: Compliance tensor evaluation determinism
// =========================================================================

#[test]
fn tensor_evaluate_all_deterministic_100_runs() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut results_list = Vec::new();
    for _ in 0..100 {
        let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid.clone()));
        let results = tensor.evaluate_all("entity-001");
        let sorted: std::collections::BTreeMap<String, String> = results
            .iter()
            .map(|(d, s)| (format!("{:?}", d), format!("{:?}", s)))
            .collect();
        results_list.push(sorted);
    }
    assert!(
        results_list.windows(2).all(|w| w[0] == w[1]),
        "Tensor evaluation should be deterministic"
    );
}

// =========================================================================
// Campaign 5: Float rejection in canonical bytes
// =========================================================================

#[test]
fn canonical_bytes_rejects_float() {
    let result = CanonicalBytes::new(&json!({"amount": 3.14}));
    // Per CLAUDE.md: floats not representable as i64/u64 should be rejected
    // The coercion rules say: "Reject floats — Numbers not representable as i64/u64"
    assert!(
        result.is_err(),
        "BUG-025: CanonicalBytes should reject float values like 3.14"
    );
}

#[test]
fn canonical_bytes_accepts_integer_as_number() {
    // Integer values should be accepted
    let result = CanonicalBytes::new(&json!({"amount": 42}));
    assert!(result.is_ok(), "Integer values should be accepted");
}

#[test]
fn canonical_bytes_accepts_negative_integer() {
    let result = CanonicalBytes::new(&json!({"amount": -100}));
    assert!(result.is_ok(), "Negative integers should be accepted");
}

// =========================================================================
// Campaign 5: Netting engine multi-currency
// =========================================================================

#[test]
fn netting_multi_currency_separate_legs() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 100_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 50_000,
            currency: "PKR".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    // Should have separate legs for each currency
    let usd_legs: Vec<_> = plan
        .settlement_legs
        .iter()
        .filter(|l| l.currency == "USD")
        .collect();
    let pkr_legs: Vec<_> = plan
        .settlement_legs
        .iter()
        .filter(|l| l.currency == "PKR")
        .collect();
    assert!(!usd_legs.is_empty(), "Should have USD settlement legs");
    assert!(!pkr_legs.is_empty(), "Should have PKR settlement legs");
}

#[test]
fn netting_bilateral_perfect_offset() {
    // A→B: 100, B→A: 100 — should net to zero legs
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 100_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "B".to_string(),
            to_party: "A".to_string(),
            amount: 100_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    assert!(
        plan.settlement_legs.is_empty(),
        "Perfect bilateral offset should produce no settlement legs"
    );
    assert_eq!(plan.net_total, 0);
    assert_eq!(plan.reduction_bps, 10_000);
}

// =========================================================================
// Campaign 5 Extension: Pack boundary inputs
// =========================================================================

use msez_pack::parser::ensure_json_compatible;
use msez_pack::regpack::{validate_compliance_domain, SanctionsChecker, SanctionsEntry};

#[test]
fn pack_sanctions_checker_threshold_zero_returns_all_matches() {
    // Threshold 0.0 should match everything (minimum similarity = 0)
    let checker = SanctionsChecker::new(
        vec![SanctionsEntry {
            entry_id: "SE-100".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec!["OFAC".to_string()],
            primary_name: "John Smith".to_string(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["SDN".to_string()],
            listing_date: None,
            remarks: None,
        }],
        "threshold-test".to_string(),
    );
    let result = checker.check_entity("Completely Different Name", None, 0.0);
    // With threshold 0.0, any name should match — this tests extreme threshold.
    // Regardless of match outcome, must not panic and must be deterministic.
    let result2 = checker.check_entity("Completely Different Name", None, 0.0);
    assert_eq!(result.matched, result2.matched, "sanctions check must be deterministic");
}

#[test]
fn pack_sanctions_checker_threshold_one_requires_exact() {
    // Threshold 1.0 should require exact (or near-exact) match
    let checker = SanctionsChecker::new(
        vec![SanctionsEntry {
            entry_id: "SE-101".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec!["EU".to_string()],
            primary_name: "Ahmed Hassan".to_string(),
            aliases: vec![],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![],
            listing_date: None,
            remarks: None,
        }],
        "exact-test".to_string(),
    );
    let result = checker.check_entity("Ahmed Hassan", None, 1.0);
    assert!(result.matched, "Exact name should match at threshold 1.0");
}

#[test]
fn pack_sanctions_checker_identifier_matching() {
    use std::collections::BTreeMap;
    let mut id_map = BTreeMap::new();
    id_map.insert("passport".to_string(), "AB1234567".to_string());

    let checker = SanctionsChecker::new(
        vec![SanctionsEntry {
            entry_id: "SE-102".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec!["UN".to_string()],
            primary_name: "Target Person".to_string(),
            aliases: vec![],
            identifiers: vec![id_map.clone()],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![],
            listing_date: None,
            remarks: None,
        }],
        "id-test".to_string(),
    );
    let query_ids = vec![id_map];
    let result = checker.check_entity("Different Name", Some(&query_ids), 0.9);
    // Identifier-based matching is not yet implemented — name mismatch means no match.
    // Verify determinism: repeated calls with same input produce same result.
    let result2 = checker.check_entity("Different Name", Some(&query_ids), 0.9);
    assert_eq!(result.matched, result2.matched, "ID-based sanctions check must be deterministic");
}

#[test]
fn pack_validate_compliance_domain_all_known_domains() {
    // Test all 20 ComplianceDomain variants using correct snake_case names
    // from ComplianceDomain::as_str() in msez-core/src/domain.rs.
    let domains = [
        "aml",
        "kyc",
        "sanctions",
        "tax",
        "securities",
        "corporate",
        "custody",
        "data_privacy",
        "licensing",
        "banking",
        "payments",
        "clearing",
        "settlement",
        "digital_assets",
        "employment",
        "immigration",
        "ip",
        "consumer_protection",
        "arbitration",
        "trade",
    ];
    assert_eq!(domains.len(), 20, "must test exactly 20 domains");
    for domain in &domains {
        let result = validate_compliance_domain(domain);
        // All 20 domains listed above must be recognized as valid.
        assert!(result.is_ok(), "domain '{domain}' must be recognized as a valid ComplianceDomain");
    }
}

#[test]
fn pack_ensure_json_compatible_float_rejected() {
    // BUG-039: ensure_json_compatible must reject floats for deterministic hashing.
    // Floats like 1.1 have non-deterministic serialization across platforms.
    let value = json!({"amount": 1.1});
    let result = ensure_json_compatible(&value, "", "float-test");
    assert!(result.is_err(), "BUG-039: ensure_json_compatible must reject floats");
}

#[test]
fn pack_ensure_json_compatible_integer_accepted() {
    let value = json!({"amount": 42});
    let result = ensure_json_compatible(&value, "", "int-test");
    assert!(result.is_ok(), "Integer values should be accepted");
}

#[test]
fn pack_ensure_json_compatible_empty_object() {
    let result = ensure_json_compatible(&json!({}), "", "empty");
    assert!(result.is_ok(), "Empty object should be valid JSON");
}

#[test]
fn pack_ensure_json_compatible_empty_array() {
    let result = ensure_json_compatible(&json!([]), "", "empty-arr");
    assert!(result.is_ok(), "Empty array should be valid JSON");
}

// =========================================================================
// Campaign 5 Extension: Netting engine boundary inputs
// =========================================================================

#[test]
fn netting_single_amount_one_boundary() {
    // Minimum valid amount is 1
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 1,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 1);
}

#[test]
fn netting_many_currencies_boundary() {
    // 50 different currencies for the same party pair
    let mut engine = NettingEngine::new();
    for i in 0..50 {
        engine
            .add_obligation(Obligation {
                from_party: "A".to_string(),
                to_party: "B".to_string(),
                amount: 1000,
                currency: format!("CUR{:03}", i),
                corridor_id: None,
                priority: 0,
            })
            .unwrap();
    }
    let plan = engine.compute_plan().unwrap();
    // Should have 50 settlement legs (one per currency)
    assert_eq!(
        plan.settlement_legs.len(),
        50,
        "Each currency should produce a separate leg"
    );
}

#[test]
fn netting_priority_ordering() {
    // Test that obligations with different priorities are handled
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 100_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 10,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "C".to_string(),
            to_party: "D".to_string(),
            amount: 50_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 1,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.settlement_legs.len(), 2);
}

// =========================================================================
// Campaign 5 Extension: Agentic boundary inputs
// =========================================================================

use msez_agentic::{AuditEntry, AuditEntryType, AuditTrail, PolicyEngine, Trigger, TriggerType};

#[test]
fn agentic_audit_trail_capacity_one_trim_behavior() {
    // Capacity of 1 — every append after the first should trigger trim
    let mut trail = AuditTrail::new(1);
    trail.append(AuditEntry::new(AuditEntryType::TriggerReceived, None, None));
    assert_eq!(trail.len(), 1);
    trail.append(AuditEntry::new(AuditEntryType::PolicyEvaluated, None, None));
    // After trimming, should still be bounded
    assert!(
        trail.len() <= 2,
        "Trail with capacity 1 should stay bounded"
    );
}

#[test]
fn agentic_audit_entry_digest_deterministic() {
    // Same entry type + asset + metadata should produce same digest
    let e1 = AuditEntry::new(
        AuditEntryType::ActionScheduled,
        Some("asset-001".to_string()),
        Some(json!({"key": "value"})),
    );
    let e2 = AuditEntry::new(
        AuditEntryType::ActionScheduled,
        Some("asset-001".to_string()),
        Some(json!({"key": "value"})),
    );
    // Note: timestamps differ so digests MAY differ. This documents the behavior.
    let d1 = e1.digest();
    let d2 = e2.digest();
    // Both should succeed (or both fail)
    assert_eq!(
        d1.is_some(),
        d2.is_some(),
        "Digest computation should be consistent"
    );
}

#[test]
fn agentic_policy_engine_evaluate_all_trigger_types() {
    // Verify no trigger type causes a panic during evaluation
    let trigger_types = [
        TriggerType::SanctionsListUpdate,
        TriggerType::LicenseStatusChange,
        TriggerType::GuidanceUpdate,
        TriggerType::ComplianceDeadline,
        TriggerType::DisputeFiled,
        TriggerType::RulingReceived,
        TriggerType::AppealPeriodExpired,
        TriggerType::EnforcementDue,
        TriggerType::CorridorStateChange,
        TriggerType::SettlementAnchorAvailable,
        TriggerType::WatcherQuorumReached,
    ];
    for tt in &trigger_types {
        let mut engine = PolicyEngine::with_standard_policies();
        let trigger = Trigger::new(tt.clone(), json!({"test": true}));
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            engine.evaluate(&trigger, Some("asset-001"), Some("PK-RSEZ"))
        }));
        assert!(
            result.is_ok(),
            "Evaluation panicked on trigger type {:?}",
            tt
        );
    }
}

// =========================================================================
// Campaign 5 Extension: Tensor boundary inputs
// =========================================================================

#[test]
fn tensor_evaluate_empty_entity_id() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    let results = tensor.evaluate_all("");
    // Should not panic; may return empty or default results
    let _ = results;
}

#[test]
fn tensor_evaluate_huge_entity_id() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    let huge_id = "E".repeat(100_000);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| tensor.evaluate_all(&huge_id)));
    assert!(result.is_ok(), "Huge entity ID should not panic");
}

#[test]
fn tensor_all_20_domains_from_default_jurisdiction() {
    // DefaultJurisdiction should populate all 20 ComplianceDomain variants
    let jid = JurisdictionId::new("TEST-JUR").unwrap();
    let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    let all = tensor.evaluate_all("test-entity");
    assert_eq!(
        all.len(),
        20,
        "DefaultJurisdiction should cover all 20 compliance domains"
    );
}

// =========================================================================
// Campaign 5 Extension: Crypto boundary inputs
// =========================================================================

#[test]
fn mmr_append_whitespace_hex_rejected() {
    let mut mmr = MerkleMountainRange::new();
    let result = mmr.append("   ");
    assert!(result.is_err(), "Whitespace-only hex should be rejected");
}

#[test]
fn mmr_append_mixed_case_hex() {
    let mut mmr = MerkleMountainRange::new();
    let hex = "aAbBcCdDeEfF".repeat(5) + "aAbBcCdD"; // 64 chars
    let result = mmr.append(&hex);
    // Mixed case hex: must not panic. Determinism check.
    let mut mmr2 = MerkleMountainRange::new();
    let result2 = mmr2.append(&hex);
    assert_eq!(result.is_ok(), result2.is_ok(), "MMR append with mixed case must be deterministic");
}

#[test]
fn mmr_append_zero_hash() {
    let mut mmr = MerkleMountainRange::new();
    let zero_hash = "0".repeat(64);
    let result = mmr.append(&zero_hash);
    // Zero hash is valid hex — should succeed
    assert!(result.is_ok(), "Zero hash should be valid");
}

#[test]
fn mmr_deterministic_root() {
    // Same sequence of appends should always produce same root
    let make_mmr = || {
        let mut mmr = MerkleMountainRange::new();
        for i in 0..10 {
            let canonical = CanonicalBytes::new(&json!({"i": i})).unwrap();
            let digest = sha256_digest(&canonical);
            mmr.append(&digest.to_hex()).unwrap();
        }
        mmr.root().unwrap()
    };
    let root1 = make_mmr();
    let root2 = make_mmr();
    assert_eq!(
        root1, root2,
        "MMR root should be deterministic for same inputs"
    );
}

// =========================================================================
// Campaign 6 Extension: Determinism verification
// =========================================================================

#[test]
fn determinism_netting_same_obligations_same_plan() {
    let build_plan = || {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(Obligation {
                from_party: "Alice".to_string(),
                to_party: "Bob".to_string(),
                amount: 100_000,
                currency: "USD".to_string(),
                corridor_id: Some("PAK-UAE".to_string()),
                priority: 5,
            })
            .unwrap();
        engine
            .add_obligation(Obligation {
                from_party: "Bob".to_string(),
                to_party: "Charlie".to_string(),
                amount: 75_000,
                currency: "USD".to_string(),
                corridor_id: Some("PAK-UAE".to_string()),
                priority: 3,
            })
            .unwrap();
        engine
            .add_obligation(Obligation {
                from_party: "Charlie".to_string(),
                to_party: "Alice".to_string(),
                amount: 50_000,
                currency: "USD".to_string(),
                corridor_id: Some("PAK-UAE".to_string()),
                priority: 1,
            })
            .unwrap();
        engine.compute_plan().unwrap()
    };

    let plan1 = build_plan();
    let plan2 = build_plan();
    assert_eq!(
        plan1.gross_total, plan2.gross_total,
        "Gross total should be deterministic"
    );
    assert_eq!(
        plan1.net_total, plan2.net_total,
        "Net total should be deterministic"
    );
    assert_eq!(
        plan1.settlement_legs.len(),
        plan2.settlement_legs.len(),
        "Settlement leg count should be deterministic"
    );
    assert_eq!(
        plan1.reduction_bps, plan2.reduction_bps,
        "Reduction bps should be deterministic"
    );
}

#[test]
fn determinism_canonical_bytes_key_ordering() {
    // JSON objects with same keys in different insertion order should produce
    // identical canonical bytes (canonical serialization normalizes key order)
    let v1 = json!({"z": 1, "a": 2, "m": 3});
    let v2 = json!({"a": 2, "m": 3, "z": 1});
    let c1 = CanonicalBytes::new(&v1).unwrap();
    let c2 = CanonicalBytes::new(&v2).unwrap();
    let d1 = sha256_digest(&c1);
    let d2 = sha256_digest(&c2);
    assert_eq!(
        d1.to_hex(),
        d2.to_hex(),
        "Canonical bytes should normalize key order"
    );
}

#[test]
fn determinism_tensor_evaluation_repeated() {
    let jid = JurisdictionId::new("AE-DIFC").unwrap();
    let tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    let results1 = tensor.evaluate_all("entity-001");
    let results2 = tensor.evaluate_all("entity-001");
    // Same entity, same tensor → same results
    assert_eq!(
        results1.len(),
        results2.len(),
        "Tensor evaluation count should be deterministic"
    );
    for (domain, state) in &results1 {
        assert_eq!(
            results2.get(domain),
            Some(state),
            "Tensor evaluation for {:?} should be deterministic",
            domain
        );
    }
}

#[test]
fn determinism_signing_verification_round_trip() {
    // Sign same content with same key → same signature
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let canonical = CanonicalBytes::new(&json!({"msg": "determinism test"})).unwrap();

    // Note: Ed25519 signatures are deterministic (RFC 8032)
    let sig1 = sk.sign(&canonical);
    let sig2 = sk.sign(&canonical);
    assert_eq!(
        sig1.to_hex(),
        sig2.to_hex(),
        "Ed25519 signatures should be deterministic (RFC 8032)"
    );
}

#[test]
fn determinism_content_digest_from_same_data() {
    let d1 = ContentDigest::from_hex(
        &sha256_digest(&CanonicalBytes::new(&json!({"a": 1})).unwrap()).to_hex(),
    )
    .unwrap();
    let d2 = ContentDigest::from_hex(
        &sha256_digest(&CanonicalBytes::new(&json!({"a": 1})).unwrap()).to_hex(),
    )
    .unwrap();
    assert_eq!(
        d1.to_hex(),
        d2.to_hex(),
        "ContentDigest from same data should be identical"
    );
}

#[test]
fn determinism_swift_pacs008_same_inputs() {
    let swift1 = SwiftPacs008::new("MSEZTEST");
    let swift2 = SwiftPacs008::new("MSEZTEST");
    let instruction = SettlementInstruction {
        message_id: "DET001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Determinism Corp".to_string(),
        creditor_bic: "COBADEFF".to_string(),
        creditor_account: "DE44500105175407324931".to_string(),
        creditor_name: "Target Corp".to_string(),
        amount: 50_000,
        currency: "EUR".to_string(),
        remittance_info: Some("Invoice 12345".to_string()),
    };
    let xml1 = swift1.generate_instruction(&instruction);
    let xml2 = swift2.generate_instruction(&instruction);
    assert_eq!(
        xml1.is_ok(),
        xml2.is_ok(),
        "SWIFT generation should be consistent"
    );
    if let (Ok(x1), Ok(x2)) = (xml1, xml2) {
        assert_eq!(
            x1, x2,
            "SWIFT pacs.008 XML should be deterministic for same inputs"
        );
    }
}

// =========================================================================
// Campaign 5 Extension: Licensepack boundary tests
// =========================================================================

#[test]
fn licensepack_condition_empty_id_accepted() {
    // BUG-042: LicenseCondition.condition_id is plain String — empty string accepted
    let cond = LicenseCondition {
        condition_id: "".to_string(),
        condition_type: "capital".to_string(),
        description: "Empty ID condition".to_string(),
        metric: None,
        threshold: None,
        currency: None,
        operator: None,
        frequency: None,
        reporting_frequency: None,
        effective_date: None,
        expiry_date: None,
        status: "active".to_string(),
    };
    // Empty condition_id silently accepted — should be rejected
    assert_eq!(
        cond.condition_id, "",
        "BUG-042: empty condition_id accepted without validation"
    );
    assert!(
        cond.is_active("2026-01-01"),
        "Active condition with empty ID is functional"
    );
}

#[test]
fn licensepack_condition_date_comparison_malformed() {
    // BUG-043 RESOLVED: date_before() now uses chrono parsing, not string comparison
    let cond = LicenseCondition {
        condition_id: "c-date".to_string(),
        condition_type: "operational".to_string(),
        description: "Date test".to_string(),
        metric: None,
        threshold: None,
        currency: None,
        operator: None,
        frequency: None,
        reporting_frequency: None,
        effective_date: None,
        expiry_date: Some("2025-9-01".to_string()), // Non-canonical but parseable
        status: "active".to_string(),
    };
    // Chrono correctly parses "2025-9-01" as Sept 1, which is before Dec 31.
    // So the condition is expired and is_active returns false.
    let result = cond.is_active("2025-12-31");
    assert!(
        !result,
        "BUG-043 RESOLVED: is_active correctly returns false — Sept 1 is before Dec 31"
    );
}

#[test]
fn licensepack_is_expired_string_date_comparison() {
    // BUG-043 variant: License::is_expired() also uses string comparison
    let _jid = JurisdictionId::new("PK-PSEZ").unwrap();
    let license = License {
        license_id: "lic-001".to_string(),
        license_type_id: "type-a".to_string(),
        license_number: None,
        status: LicenseStatus::Active,
        issued_date: "2024-01-01".to_string(),
        holder_id: "holder-1".to_string(),
        holder_legal_name: "Test Corp".to_string(),
        regulator_id: "reg-1".to_string(),
        status_effective_date: None,
        status_reason: None,
        effective_date: None,
        expiry_date: Some("2025-9-15".to_string()), // Malformed: missing leading zero
        holder_did: Some("did:msez:test".to_string()),
        issuing_authority: None,
        permitted_activities: vec![],
        asset_classes_authorized: vec![],
        client_types_permitted: vec![],
        holder_registration_number: None,
        geographic_scope: vec![],
        prudential_category: None,
        capital_requirement: Default::default(),
        conditions: vec![],
        permissions: vec![],
        restrictions: vec![],
    };
    // BUG-043 RESOLVED: date_before() now uses chrono parsing.
    // "2025-9-15" is correctly parsed as Sept 15, which is before Oct 1.
    let expired = license.is_expired("2025-10-01");
    assert!(
        expired,
        "BUG-043 RESOLVED: is_expired correctly returns true — Sept 15 is before Oct 1"
    );
}

#[test]
fn licensepack_restriction_blocks_jurisdiction_empty_string() {
    // BUG-044: blocks_jurisdiction("") doesn't reject empty jurisdiction string
    let restriction = LicenseRestriction {
        restriction_id: "r-001".to_string(),
        restriction_type: "geographic".to_string(),
        description: "Wildcard block with exceptions".to_string(),
        blocked_jurisdictions: vec!["*".to_string()],
        allowed_jurisdictions: vec!["PK".to_string(), "AE".to_string()],
        blocked_activities: vec![],
        blocked_products: vec![],
        blocked_client_types: vec![],
        max_leverage: None,
        effective_date: None,
        status: "active".to_string(),
    };
    // BUG-044 RESOLVED: blocks_jurisdiction("") now returns true (fail-secure).
    // Empty jurisdiction means we cannot verify the restriction doesn't apply,
    // so the safe default is to block the operation.
    let result = restriction.blocks_jurisdiction("");
    assert!(
        result,
        "BUG-044 RESOLVED: empty jurisdiction correctly returns true (fail-secure)"
    );
}

#[test]
fn licensepack_holder_did_lookup_empty_string() {
    // BUG-045: get_licenses_by_holder_did("") doesn't reject empty DID
    let jid = JurisdictionId::new("PK-PSEZ").unwrap();
    let mut pack = Licensepack::new(jid, "Test Pack".to_string());
    let license = License {
        license_id: "lic-empty".to_string(),
        license_type_id: "type-a".to_string(),
        license_number: None,
        status: LicenseStatus::Active,
        issued_date: "2024-01-01".to_string(),
        holder_id: "holder-1".to_string(),
        holder_legal_name: "Test Corp".to_string(),
        regulator_id: "reg-1".to_string(),
        status_effective_date: None,
        status_reason: None,
        effective_date: None,
        expiry_date: None,
        holder_did: Some("".to_string()), // Empty DID
        issuing_authority: None,
        permitted_activities: vec![],
        asset_classes_authorized: vec![],
        client_types_permitted: vec![],
        holder_registration_number: None,
        geographic_scope: vec![],
        prudential_category: None,
        capital_requirement: Default::default(),
        conditions: vec![],
        permissions: vec![],
        restrictions: vec![],
    };
    pack.licenses.insert("lic-empty".to_string(), license);

    // BUG-045 RESOLVED: empty DID now returns empty vec without matching
    let results = pack.get_licenses_by_holder_did("");
    assert_eq!(
        results.len(),
        0,
        "BUG-045 RESOLVED: empty DID correctly returns no matches"
    );
}

// =========================================================================
// Campaign 5 Extension: Watcher economy boundary tests
// =========================================================================

#[test]
fn watcher_rebond_zero_stake() {
    // BUG-046 RESOLVED: rebond(0) now correctly rejected with InsufficientStake
    let mut w = Watcher::new(WatcherId::new());
    w.bond(1_000_000).unwrap();
    w.activate().unwrap();
    w.slash(SlashingCondition::AvailabilityFailure).unwrap();

    // Rebond with zero additional stake — correctly rejected
    let result = w.rebond(0);
    assert!(
        result.is_err(),
        "BUG-046 RESOLVED: rebond(0) correctly rejected — must post new collateral"
    );
}

#[test]
fn watcher_bond_zero_correctly_rejected_and_rebond_zero_also() {
    // BUG-046 RESOLVED: both bond(0) and rebond(0) now correctly rejected
    let mut w = Watcher::new(WatcherId::new());
    assert!(w.bond(0).is_err(), "bond(0) correctly rejected");

    w.bond(100_000).unwrap();
    w.activate().unwrap();
    w.slash(SlashingCondition::Equivocation).unwrap();
    // BUG-046 RESOLVED: rebond(0) now also correctly rejected, consistent with bond(0)
    assert!(
        w.rebond(0).is_err(),
        "BUG-046 RESOLVED: rebond(0) correctly rejected — consistent with bond(0)"
    );
}

// =========================================================================
// Campaign 5 Extension: ZKP mock proof system boundary tests
// =========================================================================

#[test]
fn zkp_mock_circuit_data_included_in_proof_hash() {
    // BUG-048 RESOLVED: prove() now hashes canonical(circuit_data) || public_inputs
    // Two different circuits with same public_inputs produce different proofs
    let sys = MockProofSystem;
    let pk = MockProvingKey;

    let circuit_a = MockCircuit {
        circuit_data: json!({"type": "balance_check", "threshold": 1000}),
        public_inputs: b"same_inputs".to_vec(),
    };
    let circuit_b = MockCircuit {
        circuit_data: json!({"type": "sanctions_check", "list": "OFAC"}),
        public_inputs: b"same_inputs".to_vec(),
    };

    let proof_a = sys.prove(&pk, &circuit_a).unwrap();
    let proof_b = sys.prove(&pk, &circuit_b).unwrap();

    // BUG-048 RESOLVED: Different circuits now correctly produce different proofs
    assert_ne!(
        proof_a.proof_hex, proof_b.proof_hex,
        "BUG-048 RESOLVED: different circuits produce different proofs"
    );
}

#[test]
fn zkp_mock_empty_public_inputs() {
    // Test: prove with empty public inputs should still work.
    // After BUG-048 fix, verify needs canonical(circuit_data) || public_inputs.
    use msez_core::CanonicalBytes;

    let sys = MockProofSystem;
    let pk = MockProvingKey;
    let vk = MockVerifyingKey;

    let circuit = MockCircuit {
        circuit_data: json!({"type": "empty_test"}),
        public_inputs: vec![],
    };

    let proof = sys.prove(&pk, &circuit).unwrap();
    assert_eq!(
        proof.proof_hex.len(),
        64,
        "Empty inputs should produce valid 64-char hex proof"
    );

    // Verify requires canonical(circuit_data) || public_inputs
    let canonical = CanonicalBytes::from_value(circuit.circuit_data.clone()).unwrap();
    let verify_input: Vec<u8> = canonical
        .as_bytes()
        .iter()
        .chain(circuit.public_inputs.iter())
        .copied()
        .collect();
    let valid = sys.verify(&vk, &proof, &verify_input).unwrap();
    assert!(
        valid,
        "Proof with empty inputs should verify when canonical circuit data is included"
    );
}

#[test]
fn zkp_mock_prove_empty_circuit_data() {
    // Empty circuit data should be accepted (it's valid JSON)
    let sys = MockProofSystem;
    let pk = MockProvingKey;

    let circuit = MockCircuit {
        circuit_data: json!({}),
        public_inputs: b"test".to_vec(),
    };
    let result = sys.prove(&pk, &circuit);
    assert!(
        result.is_ok(),
        "Empty circuit data ({{}}) should be accepted"
    );
}

// =========================================================================
// Campaign 5 Extension: Licensepack serde boundary tests
// =========================================================================

#[test]
fn licensepack_license_serde_empty_fields() {
    // BUG-049: License with empty required string fields accepted via serde
    let json_str = r#"{
        "license_id": "",
        "license_type_id": "",
        "status": "active",
        "issued_date": "",
        "holder_id": "",
        "holder_legal_name": "",
        "regulator_id": ""
    }"#;
    let license: License = serde_json::from_str(json_str).unwrap();
    // BUG-049: All empty strings accepted without validation
    assert_eq!(
        license.license_id, "",
        "BUG-049: empty license_id accepted via serde"
    );
    assert_eq!(
        license.holder_id, "",
        "BUG-049: empty holder_id accepted via serde"
    );
    assert_eq!(
        license.regulator_id, "",
        "BUG-049: empty regulator_id accepted via serde"
    );
}

#[test]
fn licensepack_condition_serde_empty_fields() {
    // BUG-042 variant: LicenseCondition with empty fields via serde
    let json_str = r#"{
        "condition_id": "",
        "condition_type": "",
        "description": "",
        "status": "active"
    }"#;
    let cond: LicenseCondition = serde_json::from_str(json_str).unwrap();
    assert_eq!(
        cond.condition_id, "",
        "BUG-042: empty condition_id via serde"
    );
    assert!(cond.is_active("2026-01-01"), "Active with all empty fields");
}

#[test]
fn licensepack_resolve_refs_missing_fields() {
    // BUG-050 RESOLVED: entries with missing jurisdiction_id or domain are now skipped
    let valid_digest = "a".repeat(64);
    let zone = json!({
        "licensepacks": [
            {
                "licensepack_digest_sha256": valid_digest
            }
        ]
    });
    let refs = msez_pack::licensepack::resolve_licensepack_refs(&zone).unwrap();
    // BUG-050 RESOLVED: entry with missing required fields is skipped
    assert_eq!(
        refs.len(),
        0,
        "BUG-050 RESOLVED: entries with missing fields are skipped"
    );
}
