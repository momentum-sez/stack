//! # Campaign 3: Cross-Crate Integration Seams
//!
//! End-to-end tests that exercise data flow across crate boundaries.
//! These test the wiring between crates that was recently connected.

use msez_core::{
    CanonicalBytes, ComplianceDomain, ContentDigest, CorridorId, JurisdictionId,
};
use msez_crypto::{sha256_digest, ContentAddressedStore, MerkleMountainRange, SigningKey};
use msez_vc::{
    ContextValue, CredentialTypeValue, ProofType, ProofValue, VerifiableCredential,
};
use serde_json::json;

// =========================================================================
// Pipeline 1: Canonical → Digest → CAS → Resolve → Verify
// =========================================================================

#[test]
fn canonical_to_cas_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    // 1. Create a value, canonicalize it.
    let value = json!({
        "corridor_id": "corr-001",
        "jurisdiction_a": "PK-RSEZ",
        "jurisdiction_b": "AE-DIFC",
        "amount": 500000
    });
    let canonical = CanonicalBytes::new(&value).expect("canonicalize");

    // 2. Compute its SHA-256 digest.
    let digest = sha256_digest(&canonical);

    // 3. Store it in the CAS.
    let artifact_ref = cas
        .store("corridor-receipt", &value)
        .expect("store in CAS");

    // 4. Resolve it from the CAS by the artifact ref.
    let resolved = cas
        .resolve_ref(&artifact_ref)
        .expect("resolve from CAS")
        .expect("artifact should exist");

    // 5. Verify the resolved bytes, when re-canonicalized, produce the same digest.
    let resolved_value: serde_json::Value =
        serde_json::from_slice(&resolved).expect("deserialize resolved bytes");
    let resolved_canonical = CanonicalBytes::new(&resolved_value).expect("re-canonicalize");
    let resolved_digest = sha256_digest(&resolved_canonical);
    assert_eq!(
        digest.to_hex(),
        resolved_digest.to_hex(),
        "Digest mismatch: CAS round-trip altered content"
    );
}

#[test]
fn canonical_to_cas_different_key_order_same_digest() {
    // JSON key order should not affect digest due to canonicalization
    let value_a = json!({"b": 2, "a": 1, "c": 3});
    let value_b = json!({"c": 3, "a": 1, "b": 2});

    let canonical_a = CanonicalBytes::new(&value_a).unwrap();
    let canonical_b = CanonicalBytes::new(&value_b).unwrap();

    let digest_a = sha256_digest(&canonical_a);
    let digest_b = sha256_digest(&canonical_b);

    assert_eq!(
        digest_a.to_hex(),
        digest_b.to_hex(),
        "Different key order should produce same digest after canonicalization"
    );
}

#[test]
fn cas_store_then_contains_check() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    let value = json!({"entity": "test-entity-001"});
    let artifact_ref = cas.store("entity-snapshot", &value).unwrap();

    // Verify the CAS reports the artifact exists
    let exists = cas
        .contains(artifact_ref.artifact_type.as_str(), &artifact_ref.digest)
        .unwrap();
    assert!(exists, "CAS should contain the stored artifact");

    // Verify a non-existent digest is not found
    let other_canonical = CanonicalBytes::new(&json!({"other": true})).unwrap();
    let other_digest = sha256_digest(&other_canonical);
    let not_exists = cas.contains("entity-snapshot", &other_digest).unwrap();
    assert!(!not_exists, "CAS should not contain non-existent artifact");
}

// =========================================================================
// Pipeline 2: Ed25519 → VC Sign → Serialize → Deserialize → Verify
// =========================================================================

#[test]
fn vc_sign_serialize_deserialize_verify() {
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let vk = sk.verifying_key();

    // 1. Create a VC with a credential subject.
    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:vc:compliance:001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "ComplianceAttestation".to_string(),
        ]),
        issuer: "did:key:z6MkTestIssuer".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({
            "entity_id": "ent-001",
            "jurisdiction": "PK-RSEZ",
            "compliance_state": "Compliant",
            "domains_evaluated": ["Aml", "Kyc", "Sanctions"]
        }),
        proof: ProofValue::default(),
    };

    // 2. Sign the VC.
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTestIssuer#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .expect("signing should succeed");

    // 3. Verify the proof is present.
    assert!(
        !vc.proof.is_empty(),
        "Signed VC should have at least one proof"
    );

    // 4. Serialize the signed VC to JSON.
    let json_str = serde_json::to_string(&vc).expect("serialize VC");

    // 5. Deserialize from JSON.
    let vc2: VerifiableCredential = serde_json::from_str(&json_str).expect("deserialize VC");

    // 6. Verify the signature on the deserialized VC.
    let results = vc2.verify(|_vm| Ok(vk.clone()));
    assert!(
        !results.is_empty(),
        "Verification should produce at least one result"
    );
    for r in &results {
        assert!(
            r.ok,
            "Proof verification failed: {}",
            r.error
        );
    }

    // 7. Also test verify_all
    vc2.verify_all(|_vm| Ok(vk.clone()))
        .expect("verify_all should succeed");
}

#[test]
fn vc_tampered_subject_fails_verification() {
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let vk = sk.verifying_key();

    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:vc:tamper-test".to_string()),
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkTamperTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"amount": 1000, "status": "approved"}),
        proof: ProofValue::default(),
    };

    vc.sign_ed25519(
        &sk,
        "did:key:z6MkTamperTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Tamper with the credential subject
    vc.credential_subject = json!({"amount": 999999, "status": "approved"});

    // Verification should fail
    let results = vc.verify(|_vm| Ok(vk.clone()));
    assert!(
        !results.is_empty(),
        "Should still have proofs to verify"
    );
    let any_failed = results.iter().any(|r| !r.ok);
    assert!(
        any_failed,
        "BUG-023: Tampered VC should fail verification but all proofs passed"
    );
}

#[test]
fn vc_wrong_key_fails_verification() {
    let sk1 = SigningKey::generate(&mut rand_core::OsRng);
    let sk2 = SigningKey::generate(&mut rand_core::OsRng);
    let wrong_vk = sk2.verifying_key();

    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:vc:wrong-key-test".to_string()),
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkWrongKey".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"test": true}),
        proof: ProofValue::default(),
    };

    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkWrongKey#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Verify with wrong key — should fail
    let results = vc.verify(|_vm| Ok(wrong_vk.clone()));
    let any_failed = results.iter().any(|r| !r.ok);
    assert!(any_failed, "Wrong key should fail verification");
}

#[test]
fn vc_signed_then_stored_in_cas_then_retrieved_and_verified() {
    let sk = SigningKey::generate(&mut rand_core::OsRng);
    let vk = sk.verifying_key();
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    // Sign a VC
    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:vc:cas-round-trip".to_string()),
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkCasTest".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"entity_id": "ent-cas-001"}),
        proof: ProofValue::default(),
    };
    vc.sign_ed25519(
        &sk,
        "did:key:z6MkCasTest#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Store in CAS
    let artifact_ref = cas.store("verifiable-credential", &vc).unwrap();

    // Retrieve from CAS
    let bytes = cas.resolve_ref(&artifact_ref).unwrap().unwrap();
    let retrieved_vc: VerifiableCredential = serde_json::from_slice(&bytes).unwrap();

    // Verify the retrieved VC
    retrieved_vc
        .verify_all(|_vm| Ok(vk.clone()))
        .expect("Retrieved VC should verify successfully");
}

// =========================================================================
// Pipeline 3: Corridor State Machine → Transition History → MMR
// =========================================================================

use msez_state::corridor::{
    ActivationEvidence, Corridor, Draft, HaltReason, SubmissionEvidence,
};

fn test_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"label": label})).unwrap();
    sha256_digest(&canonical)
}

#[test]
fn corridor_lifecycle_draft_to_active() {
    let corridor_id = CorridorId::new();
    let jid_a = JurisdictionId::new("PK-RSEZ").unwrap();
    let jid_b = JurisdictionId::new("AE-DIFC").unwrap();

    // 1. Create a corridor in DRAFT.
    let draft: Corridor<Draft> = Corridor::new(corridor_id, jid_a.clone(), jid_b.clone());

    // 2. Transition DRAFT → PENDING.
    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("agreement"),
        pack_trilogy_digest: test_digest("pack-trilogy"),
    });

    // 3. Transition PENDING → ACTIVE.
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-pk"),
        regulatory_approval_b: test_digest("approval-ae"),
    });

    // 4. Verify the transition log has entries for each transition.
    let log = active.transition_log();
    // Draft→Pending + Pending→Active = at least 2 transitions
    assert!(
        log.len() >= 2,
        "Expected at least 2 transition records, got {}",
        log.len()
    );
}

#[test]
fn corridor_lifecycle_to_halted_preserves_history() {
    let corridor_id = CorridorId::new();
    let jid_a = JurisdictionId::new("PK-RSEZ").unwrap();
    let jid_b = JurisdictionId::new("AE-DIFC").unwrap();

    let draft = Corridor::new(corridor_id, jid_a.clone(), jid_b.clone());
    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: test_digest("agreement"),
        pack_trilogy_digest: test_digest("pack-trilogy"),
    });
    let active = pending.activate(ActivationEvidence {
        regulatory_approval_a: test_digest("approval-pk"),
        regulatory_approval_b: test_digest("approval-ae"),
    });

    // Halt the corridor
    let halted = active.halt(HaltReason {
        reason: "Sanctions violation detected".to_string(),
        authority: jid_a,
        evidence: test_digest("sanctions-evidence"),
    });

    let log = halted.transition_log();
    // Should have 3 transitions: Draft→Pending, Pending→Active, Active→Halted
    assert!(
        log.len() >= 3,
        "Expected at least 3 transition records, got {}",
        log.len()
    );
}

#[test]
fn corridor_transition_digests_feed_mmr() {
    // Test that corridor transition evidence digests can be appended to an MMR
    let mut mmr = MerkleMountainRange::new();

    let corridor_id = CorridorId::new();
    let jid_a = JurisdictionId::new("PK-RSEZ").unwrap();
    let jid_b = JurisdictionId::new("AE-DIFC").unwrap();

    let agreement_digest = test_digest("agreement");
    let pack_digest = test_digest("pack-trilogy");
    let approval_a = test_digest("approval-pk");
    let approval_b = test_digest("approval-ae");

    // Build corridor through transitions
    let draft = Corridor::new(corridor_id, jid_a, jid_b);
    let pending = draft.submit(SubmissionEvidence {
        bilateral_agreement_digest: agreement_digest.clone(),
        pack_trilogy_digest: pack_digest.clone(),
    });
    let _active = pending.activate(ActivationEvidence {
        regulatory_approval_a: approval_a.clone(),
        regulatory_approval_b: approval_b.clone(),
    });

    // Append all evidence digests to MMR
    mmr.append(&agreement_digest.to_hex()).unwrap();
    mmr.append(&pack_digest.to_hex()).unwrap();
    mmr.append(&approval_a.to_hex()).unwrap();
    mmr.append(&approval_b.to_hex()).unwrap();

    // MMR should have a root after 4 appends
    let root = mmr.root().expect("MMR root should be computable");
    assert!(!root.is_empty(), "MMR root should not be empty");

    // Appending another leaf should change the root
    let old_root = root;
    mmr.append(&test_digest("additional-evidence").to_hex())
        .unwrap();
    let new_root = mmr.root().unwrap();
    assert_ne!(old_root, new_root, "MMR root should change after append");
}

// =========================================================================
// Pipeline 4: Compliance Tensor → Commitment Digest → CAS
// =========================================================================

use msez_tensor::{AttestationRef, ComplianceState, ComplianceTensor, DefaultJurisdiction};

#[test]
fn compliance_tensor_evaluate_and_store_commitment() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let jurisdiction = DefaultJurisdiction::new(jid.clone());
    let mut tensor = ComplianceTensor::new(jurisdiction);

    // Set some domain states with attestations
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![AttestationRef {
            attestation_id: "vc:kyc:001".to_string(),
            attestation_type: "kyc_verification".to_string(),
            issuer_did: "did:key:z6MkKycIssuer".to_string(),
            issued_at: "2026-01-15T00:00:00Z".to_string(),
            expires_at: Some("2027-01-15T00:00:00Z".to_string()),
            digest: "aa".repeat(32),
        }],
        Some("KYC verification passed".to_string()),
    );

    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![AttestationRef {
            attestation_id: "vc:aml:001".to_string(),
            attestation_type: "aml_screening".to_string(),
            issuer_did: "did:key:z6MkAmlIssuer".to_string(),
            issued_at: "2026-01-15T00:00:00Z".to_string(),
            expires_at: None,
            digest: "bb".repeat(32),
        }],
        None,
    );

    // Evaluate
    let kyc_state = tensor.evaluate("entity-001", ComplianceDomain::Kyc);
    assert_eq!(kyc_state, ComplianceState::Compliant);

    let aml_state = tensor.evaluate("entity-001", ComplianceDomain::Aml);
    assert_eq!(aml_state, ComplianceState::Compliant);

    // Store the evaluate_all results (which is serializable) in CAS
    let all_states = tensor.evaluate_all("entity-001");
    let artifact_ref = cas.store("tensor-snapshot", &all_states).unwrap();

    // Retrieve and verify
    let bytes = cas.resolve_ref(&artifact_ref).unwrap().unwrap();
    let _retrieved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
}

#[test]
fn compliance_tensor_commitment_digest_is_deterministic() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();

    // Create two tensors with same state
    let mut tensor1 = ComplianceTensor::new(DefaultJurisdiction::new(jid.clone()));
    let mut tensor2 = ComplianceTensor::new(DefaultJurisdiction::new(jid.clone()));

    let attestation = vec![AttestationRef {
        attestation_id: "vc:kyc:001".to_string(),
        attestation_type: "kyc_verification".to_string(),
        issuer_did: "did:key:z6MkKycIssuer".to_string(),
        issued_at: "2026-01-15T00:00:00Z".to_string(),
        expires_at: None,
        digest: "cc".repeat(32),
    }];

    tensor1.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        attestation.clone(),
        None,
    );
    tensor2.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        attestation,
        None,
    );

    // Both commitment digests should be identical
    let states: Vec<(ComplianceDomain, ComplianceState)> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, tensor1.get(d)))
        .collect();

    let states2: Vec<(ComplianceDomain, ComplianceState)> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, tensor2.get(d)))
        .collect();

    let digest1 =
        msez_tensor::commitment_digest("PK-RSEZ", &states).expect("commitment digest 1");
    let digest2 =
        msez_tensor::commitment_digest("PK-RSEZ", &states2).expect("commitment digest 2");

    assert_eq!(
        digest1.to_hex(),
        digest2.to_hex(),
        "Same tensor state should produce same commitment digest"
    );
}

// =========================================================================
// Pipeline 5: Netting Engine → Settlement Plan → SWIFT pacs.008
// =========================================================================

use msez_corridor::netting::{NettingEngine, Obligation};
use msez_corridor::swift::{SettlementInstruction, SettlementRail, SwiftPacs008};

#[test]
fn netting_to_swift_pipeline() {
    // 1. Add bilateral obligations to the netting engine.
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "CompanyA".to_string(),
            to_party: "CompanyB".to_string(),
            amount: 500_000,
            currency: "USD".to_string(),
            corridor_id: Some("corr-pk-ae".to_string()),
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "CompanyB".to_string(),
            to_party: "CompanyA".to_string(),
            amount: 200_000,
            currency: "USD".to_string(),
            corridor_id: Some("corr-pk-ae".to_string()),
            priority: 0,
        })
        .unwrap();

    // 2. Compute the settlement plan.
    let plan = engine.compute_plan().expect("compute plan");
    assert!(!plan.settlement_legs.is_empty(), "Plan should have legs");

    // Verify netting reduced the gross total
    assert_eq!(plan.gross_total, 700_000);
    assert!(
        plan.net_total < plan.gross_total,
        "Netting should reduce total: net={} gross={}",
        plan.net_total,
        plan.gross_total
    );

    // 3. For each settlement leg, generate a SWIFT pacs.008 message.
    let swift = SwiftPacs008::new("MSEZSEXX");
    let party_bics: std::collections::HashMap<&str, (&str, &str, &str)> = [
        ("CompanyA", ("DEUTDEFF", "DE89370400440532013000", "Company A GmbH")),
        ("CompanyB", ("BKCHCNBJ", "CN12345678901234", "Company B Ltd")),
    ]
    .into_iter()
    .collect();

    for (i, leg) in plan.settlement_legs.iter().enumerate() {
        let from_info = party_bics.get(leg.from_party.as_str()).expect("known party");
        let to_info = party_bics.get(leg.to_party.as_str()).expect("known party");

        let instruction = SettlementInstruction {
            message_id: format!("MSEZ-{:04}", i),
            debtor_bic: from_info.0.to_string(),
            debtor_account: from_info.1.to_string(),
            debtor_name: from_info.2.to_string(),
            creditor_bic: to_info.0.to_string(),
            creditor_account: to_info.1.to_string(),
            creditor_name: to_info.2.to_string(),
            amount: leg.amount,
            currency: leg.currency.clone(),
            remittance_info: Some(format!("SEZ Settlement Leg {}", i)),
        };

        let xml = swift
            .generate_instruction(&instruction)
            .expect("generate SWIFT XML");

        // 4. Verify the XML is well-formed (contains key elements).
        assert!(
            xml.contains("pacs.008"),
            "SWIFT XML should reference pacs.008"
        );
        assert!(
            xml.contains(&from_info.0.to_string()),
            "XML should contain debtor BIC"
        );
        assert!(
            xml.contains(&to_info.0.to_string()),
            "XML should contain creditor BIC"
        );

        // 5. Verify the amounts match (amounts are in minor units, XML formats as major.minor).
        let amount_major = leg.amount / 100;
        let amount_minor = leg.amount % 100;
        let formatted_amount = format!("{amount_major}.{amount_minor:02}");
        assert!(
            xml.contains(&formatted_amount),
            "XML should contain formatted amount {}, xml={}",
            formatted_amount,
            &xml[..200.min(xml.len())]
        );
    }
}

#[test]
fn netting_multilateral_three_parties() {
    let mut engine = NettingEngine::new();

    // A→B: 100, B→C: 80, C→A: 60
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
            to_party: "C".to_string(),
            amount: 80_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "C".to_string(),
            to_party: "A".to_string(),
            amount: 60_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 240_000);

    // Net positions should balance to zero
    let net_sum: i64 = plan.net_positions.iter().map(|p| p.net).sum();
    assert_eq!(
        net_sum, 0,
        "Sum of net positions should be zero (conservation of value)"
    );
}

// =========================================================================
// Pipeline 6: Agentic Trigger → Policy Evaluation → Scheduled Actions
// =========================================================================

use msez_agentic::{PolicyAction, PolicyEngine, Trigger, TriggerType};

#[test]
fn sanctions_trigger_produces_halt_action() {
    let mut engine = PolicyEngine::with_standard_policies();

    // Fire a sanctions trigger — standard policy requires affected_parties contains "self"
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["self"]}),
    );

    let actions = engine.process_trigger(&trigger, "asset:entity-001", None);

    // Standard policies should include a Halt action for sanctions
    let has_halt = actions.iter().any(|a| a.action == PolicyAction::Halt);
    assert!(
        has_halt,
        "Sanctions trigger with 'self' in affected_parties should produce Halt action, got: {:?}",
        actions.iter().map(|a| &a.action).collect::<Vec<_>>()
    );
}

#[test]
fn policy_evaluation_determinism() {
    // Same trigger should produce same results
    let trigger = Trigger::new(
        TriggerType::SanctionsListUpdate,
        json!({"affected_parties": ["entity-001"]}),
    );

    let mut engine1 = PolicyEngine::with_standard_policies();
    let mut engine2 = PolicyEngine::with_standard_policies();

    let results1 = engine1.evaluate(&trigger, Some("asset:001"), None);
    let results2 = engine2.evaluate(&trigger, Some("asset:001"), None);

    assert_eq!(
        results1.len(),
        results2.len(),
        "Same trigger should produce same number of results"
    );

    for (r1, r2) in results1.iter().zip(results2.iter()) {
        assert_eq!(
            r1.policy_id, r2.policy_id,
            "Policy IDs should be in same order"
        );
        assert_eq!(
            r1.matched, r2.matched,
            "Match results should be identical for policy {}",
            r1.policy_id
        );
        assert_eq!(
            r1.action, r2.action,
            "Actions should be identical for policy {}",
            r1.policy_id
        );
    }
}

#[test]
fn all_trigger_types_evaluate_without_panic() {
    let mut engine = PolicyEngine::with_extended_policies();

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
        TriggerType::CheckpointDue,
        TriggerType::KeyRotationDue,
        TriggerType::GovernanceVoteResolved,
        TriggerType::TaxYearEnd,
        TriggerType::WithholdingDue,
        TriggerType::EntityDissolution,
        TriggerType::PackUpdated,
        TriggerType::AssetTransferInitiated,
        TriggerType::MigrationDeadline,
    ];

    for tt in &trigger_types {
        let trigger = Trigger::new(tt.clone(), json!({"test": true}));
        // Should not panic for any trigger type
        let _results = engine.evaluate(&trigger, Some("asset:test"), Some("PK-RSEZ"));
    }
}

// =========================================================================
// Cross-Crate Type Conversion Tests
// =========================================================================

#[test]
fn content_digest_from_core_used_in_crypto_cas() {
    // Verify ContentDigest from msez-core is compatible with msez-crypto CAS
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    let canonical = CanonicalBytes::new(&json!({"cross": "crate"})).unwrap();
    let digest: ContentDigest = sha256_digest(&canonical);

    // Store raw bytes using the digest
    cas.store_raw("test-type", &digest, canonical.as_bytes())
        .unwrap();

    // Resolve using the same digest
    let resolved = cas.resolve("test-type", &digest).unwrap();
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap(), canonical.as_bytes());
}

#[test]
fn jurisdiction_id_used_across_corridor_and_tensor() {
    // Verify JurisdictionId from msez-core works in both msez-state and msez-tensor
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();

    // Use in corridor construction
    let corridor = Corridor::new(
        CorridorId::new(),
        jid.clone(),
        JurisdictionId::new("AE-DIFC").unwrap(),
    );

    // Use in tensor construction
    let jurisdiction = DefaultJurisdiction::new(jid.clone());
    let tensor = ComplianceTensor::new(jurisdiction);

    // Both should work with the same JurisdictionId
    let _ = corridor;
    let _ = tensor;
}

#[test]
fn mmr_roots_are_valid_hex_for_cas_storage() {
    // Verify MMR root output is valid hex that can be used as a CAS key
    let mut mmr = MerkleMountainRange::new();
    let digest1 = test_digest("leaf-1");
    let digest2 = test_digest("leaf-2");

    mmr.append(&digest1.to_hex()).unwrap();
    mmr.append(&digest2.to_hex()).unwrap();

    let root = mmr.root().unwrap();

    // Root should be valid hex
    assert!(
        root.chars().all(|c| c.is_ascii_hexdigit()),
        "MMR root should be valid hex, got: {}",
        root
    );

    // Root should be 64 hex chars (32 bytes SHA-256)
    assert_eq!(
        root.len(),
        64,
        "MMR root should be 64 hex chars, got {}",
        root.len()
    );
}

// =========================================================================
// Pipeline 7: Evidence Package → CAS → Verify Integrity
// =========================================================================

use msez_arbitration::evidence::{EvidenceItem, EvidencePackage, EvidenceType};
use msez_arbitration::dispute::DisputeId;
use msez_core::Did;

#[test]
fn evidence_package_digest_stored_in_cas() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    let dispute_id = DisputeId::new();
    let content = json!({"contract_ref": "TC-2026-001", "value": 500000});
    let item = EvidenceItem::new(
        EvidenceType::ContractDocument,
        "Trade Contract".to_string(),
        "Bilateral trade agreement for PK-AE corridor".to_string(),
        &content,
        Did::new("did:key:z6MkSubmitter").unwrap(),
    )
    .unwrap();

    let package = EvidencePackage::new(
        dispute_id,
        Did::new("did:key:z6MkSubmitter").unwrap(),
        vec![item],
    )
    .unwrap();

    // Package should have a digest
    let digest = &package.package_digest;
    assert_eq!(digest.to_hex().len(), 64, "Package digest should be 64 hex");

    // Store the package in CAS
    let artifact = cas.store("evidence-package", &package).unwrap();

    // Retrieve and verify integrity
    let bytes = cas.resolve_ref(&artifact).unwrap().unwrap();
    let retrieved: EvidencePackage = serde_json::from_slice(&bytes).unwrap();
    assert!(
        retrieved.verify_package_integrity().is_ok(),
        "Retrieved evidence package should pass integrity check"
    );
}

#[test]
fn evidence_package_add_item_updates_digest() {
    let dispute_id = DisputeId::new();
    let content1 = json!({"ref": "D001"});
    let item1 = EvidenceItem::new(
        EvidenceType::ContractDocument,
        "Doc 1".to_string(),
        "First document".to_string(),
        &content1,
        Did::new("did:key:z6MkA").unwrap(),
    )
    .unwrap();

    let mut package = EvidencePackage::new(
        dispute_id,
        Did::new("did:key:z6MkA").unwrap(),
        vec![item1],
    )
    .unwrap();

    let digest_before = package.package_digest.to_hex();

    let content2 = json!({"ref": "D002"});
    let item2 = EvidenceItem::new(
        EvidenceType::WitnessStatement,
        "Doc 2".to_string(),
        "Second document".to_string(),
        &content2,
        Did::new("did:key:z6MkA").unwrap(),
    )
    .unwrap();

    package.add_item(item2).unwrap();
    let digest_after = package.package_digest.to_hex();

    assert_ne!(
        digest_before, digest_after,
        "Adding an item should change the package digest"
    );
}

// =========================================================================
// Pipeline 8: Receipt Chain → Checkpoint → CAS → Verify Digest
// =========================================================================

use msez_corridor::receipt::{CorridorReceipt, ReceiptChain};

#[test]
fn receipt_chain_checkpoint_stored_in_cas() {
    let tmp = tempfile::tempdir().unwrap();
    let cas = ContentAddressedStore::new(tmp.path());

    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id.clone());

    // Append a receipt
    let prev_root = chain.mmr_root().unwrap();
    let next_root = {
        let c = CanonicalBytes::new(&json!({"event": "state_change"})).unwrap();
        sha256_digest(&c).to_hex()
    };
    chain
        .append(CorridorReceipt {
            receipt_type: "state_transition".to_string(),
            corridor_id: corridor_id.clone(),
            sequence: 0,
            timestamp: msez_core::Timestamp::now(),
            prev_root,
            next_root,
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        })
        .unwrap();

    // Create checkpoint
    let checkpoint = chain.create_checkpoint().unwrap();
    assert_eq!(checkpoint.height, 1);
    assert_eq!(checkpoint.mmr_root.len(), 64);

    // Store checkpoint in CAS
    let artifact = cas.store("checkpoint", &checkpoint).unwrap();
    let bytes = cas.resolve_ref(&artifact).unwrap().unwrap();
    let retrieved: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

    // Checkpoint digest should be present (serialized as ContentDigest object or string)
    let digest_field = &retrieved["checkpoint_digest"];
    assert!(
        !digest_field.is_null(),
        "Checkpoint should have a checkpoint_digest field, got: {:?}",
        retrieved
    );
}

// =========================================================================
// Pipeline 9: CorridorBridge → Route → Settlement Fee Calculation
// =========================================================================

use msez_corridor::bridge::{BridgeEdge, CorridorBridge};

#[test]
fn bridge_route_fees_used_in_settlement() {
    let mut bridge = CorridorBridge::new();

    let pk = JurisdictionId::new("PK-RSEZ").unwrap();
    let ae = JurisdictionId::new("AE-DIFC").unwrap();
    let sg = JurisdictionId::new("SG-MAS").unwrap();

    // Direct PK→AE: 100 bps
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 100,
        settlement_time_secs: 86400,
    });

    // PK→SG: 30 bps, SG→AE: 30 bps (cheaper multi-hop)
    bridge.add_edge(BridgeEdge {
        from: pk.clone(),
        to: sg.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 30,
        settlement_time_secs: 43200,
    });
    bridge.add_edge(BridgeEdge {
        from: sg.clone(),
        to: ae.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 30,
        settlement_time_secs: 43200,
    });

    let route = bridge.find_route(&pk, &ae).unwrap();

    // Dijkstra should find the cheaper multi-hop route (60 bps < 100 bps)
    assert_eq!(route.total_fee_bps, 60);
    assert_eq!(route.hop_count(), 2);

    // Use the fee to adjust a settlement amount
    let gross = 1_000_000i64; // $10,000.00
    let fee = gross * (route.total_fee_bps as i64) / 10_000;
    assert_eq!(fee, 6000, "60 bps on $10k should be $60.00");
}

// =========================================================================
// Pipeline 10: MigrationSaga → Corridor ← Tensor
// =========================================================================

use msez_core::MigrationId;
use msez_state::migration::MigrationBuilder;

#[test]
fn migration_saga_with_corridor_and_tensor() {
    // Build a migration saga
    let deadline = chrono::Utc::now() + chrono::Duration::hours(24);
    let mut saga = MigrationBuilder::new(MigrationId::new())
        .deadline(deadline)
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .asset_description("Entity migration PK→AE".to_string())
        .build();

    // Build a compliance tensor for the source jurisdiction
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid));
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![AttestationRef {
            attestation_id: "vc:kyc:mig".to_string(),
            attestation_type: "kyc".to_string(),
            issuer_did: "did:key:z6MkIssuer".to_string(),
            issued_at: "2026-01-01T00:00:00Z".to_string(),
            expires_at: None,
            digest: "dd".repeat(32),
        }],
        None,
    );

    // Verify entity is compliant before migrating
    let kyc_state = tensor.evaluate("entity-mig", ComplianceDomain::Kyc);
    assert_eq!(kyc_state, ComplianceState::Compliant);

    // Advance the saga (should go from Initiated to ComplianceCheck)
    let state = saga.advance().unwrap();
    assert!(
        format!("{:?}", state).contains("ComplianceCheck")
            || format!("{:?}", state).contains("Compliance"),
        "First advance should reach ComplianceCheck, got {:?}",
        state
    );
}

// =========================================================================
// Pipeline 11: Policy Engine triggers → VC issuance
// =========================================================================

#[test]
fn policy_engine_corridor_trigger_to_vc() {
    let mut engine = PolicyEngine::with_standard_policies();

    // Simulate a corridor state change trigger
    let trigger = Trigger::new(
        TriggerType::CorridorStateChange,
        json!({
            "corridor_id": "corr-pk-ae-001",
            "new_state": "HALTED",
            "reason": "sanctions"
        }),
    );

    let actions = engine.process_trigger(&trigger, "asset:entity-001", Some("PK-RSEZ"));

    // Actions should be serializable (cross-crate: agentic → serde)
    let serialized = serde_json::to_string(&actions).unwrap();
    let deserialized: Vec<serde_json::Value> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(actions.len(), deserialized.len());

    // If there's a halt action, we should be able to issue a compliance VC
    if actions.iter().any(|a| a.action == PolicyAction::Halt) {
        let sk = SigningKey::generate(&mut rand_core::OsRng);
        let mut vc = VerifiableCredential {
            context: ContextValue::default(),
            id: Some("urn:vc:halt-notice".to_string()),
            credential_type: CredentialTypeValue::Array(vec![
                "VerifiableCredential".to_string(),
                "HaltNotice".to_string(),
            ]),
            issuer: "did:key:z6MkSEZ".to_string(),
            issuance_date: chrono::Utc::now(),
            expiration_date: None,
            credential_subject: json!({
                "asset_id": "asset:entity-001",
                "action": "halt",
                "trigger_type": "corridor_state_change",
                "corridor_id": "corr-pk-ae-001"
            }),
            proof: ProofValue::default(),
        };
        vc.sign_ed25519(
            &sk,
            "did:key:z6MkSEZ#key-1".to_string(),
            ProofType::Ed25519Signature2020,
            None,
        )
        .unwrap();

        assert!(
            !vc.proof.is_empty(),
            "Halt notice VC should be signed"
        );
    }
}

// =========================================================================
// Pipeline 12: Tensor commitment → MMR → Receipt chain
// =========================================================================

#[test]
fn tensor_commitment_feeds_mmr_and_receipt_chain() {
    let jid = JurisdictionId::new("PK-RSEZ").unwrap();
    let mut tensor = ComplianceTensor::new(DefaultJurisdiction::new(jid.clone()));

    // Set compliance states across multiple domains
    for (i, domain) in [
        ComplianceDomain::Kyc,
        ComplianceDomain::Aml,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ]
    .iter()
    .enumerate()
    {
        tensor.set(
            *domain,
            ComplianceState::Compliant,
            vec![AttestationRef {
                attestation_id: format!("vc:{}:{}", domain, i),
                attestation_type: format!("{}_verification", domain),
                issuer_did: "did:key:z6MkIssuer".to_string(),
                issued_at: "2026-01-15T00:00:00Z".to_string(),
                expires_at: None,
                digest: format!("{:02x}", i).repeat(32),
            }],
            None,
        );
    }

    // Compute commitment digest
    let states: Vec<(ComplianceDomain, ComplianceState)> = ComplianceDomain::all()
        .iter()
        .map(|&d| (d, tensor.get(d)))
        .collect();
    let commitment = msez_tensor::commitment_digest("PK-RSEZ", &states).unwrap();

    // Feed the commitment digest into an MMR
    let mut mmr = MerkleMountainRange::new();
    mmr.append(&commitment.to_hex()).unwrap();
    let root = mmr.root().unwrap();
    assert_eq!(root.len(), 64, "MMR root should be valid 64-hex");

    // The MMR root can be used as the next_root in a receipt chain
    let corridor_id = CorridorId::new();
    let mut chain = ReceiptChain::new(corridor_id.clone());
    let prev_root = chain.mmr_root().unwrap();
    chain
        .append(CorridorReceipt {
            receipt_type: "compliance_snapshot".to_string(),
            corridor_id,
            sequence: 0,
            timestamp: msez_core::Timestamp::now(),
            prev_root,
            next_root: root,
            lawpack_digest_set: vec![],
            ruleset_digest_set: vec![],
        })
        .unwrap();
    assert_eq!(chain.height(), 1);
}

// =========================================================================
// Pipeline 13: VC multi-proof (two signers on same credential)
// =========================================================================

#[test]
fn vc_multi_proof_sign_and_verify_both() {
    let sk1 = SigningKey::generate(&mut rand_core::OsRng);
    let sk2 = SigningKey::generate(&mut rand_core::OsRng);
    let vk1 = sk1.verifying_key();
    let vk2 = sk2.verifying_key();

    let mut vc = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:vc:multi-proof".to_string()),
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkMulti".to_string(),
        issuance_date: chrono::Utc::now(),
        expiration_date: None,
        credential_subject: json!({"entity": "multi-signer-test"}),
        proof: ProofValue::default(),
    };

    // First signer
    vc.sign_ed25519(
        &sk1,
        "did:key:z6MkSigner1#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Second signer
    vc.sign_ed25519(
        &sk2,
        "did:key:z6MkSigner2#key-1".to_string(),
        ProofType::Ed25519Signature2020,
        None,
    )
    .unwrap();

    // Should have two proofs
    assert_eq!(vc.proof.as_list().len(), 2, "VC should have exactly 2 proofs");

    // Verify with correct key resolver
    let vk1_c = vk1.clone();
    let vk2_c = vk2.clone();
    let results = vc.verify(move |vm: &str| {
        if vm.contains("Signer1") {
            Ok(vk1_c.clone())
        } else {
            Ok(vk2_c.clone())
        }
    });

    assert_eq!(results.len(), 2, "Should verify both proofs");
    for r in &results {
        assert!(r.ok, "Proof verification failed: {}", r.error);
    }
}
