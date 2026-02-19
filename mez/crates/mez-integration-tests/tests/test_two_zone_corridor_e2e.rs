//! # Two-Zone Corridor End-to-End Integration Tests (Phase 2)
//!
//! Programmatic Rust integration tests proving the full corridor lifecycle
//! works across two independent zones. This is the Phase 2 proof:
//!
//! 1. Zone A (pk-sifc) and Zone B (ae-difc) initialize independent receipt chains
//! 2. Zone A proposes a corridor to Zone B
//! 3. Zone B accepts the corridor
//! 4. Receipts flow: create → seal → append → verify → transmit → verify on remote
//! 5. Checkpoints capture MMR state with all required fields
//! 6. Watcher attestations bind to candidate roots; fork resolution selects correctly
//! 7. Adversarial: forged next_root rejected (I-RECEIPT-COMMIT)
//! 8. Adversarial: duplicate receipt rejected (I-RECEIPT-LINK)
//! 9. Compliance tensor: all mandatory domains present, no empty slices pass
//! 10. Cross-zone receipt exchange with schema conformance

use chrono::Utc;
use mez_core::{sha256_digest, CanonicalBytes, ComplianceDomain, ContentDigest, CorridorId, JurisdictionId, Timestamp};
use mez_corridor::{
    CorridorReceipt, DigestEntry, ProofObject, ReceiptChain, ReceiptProof,
    ForkBranch, ForkDetector, ResolutionReason, WatcherRegistry,
    create_attestation, resolve_fork,
};
use mez_crypto::ed25519::SigningKey;
use mez_tensor::{ComplianceState, ComplianceTensor, DefaultJurisdiction};
use rand_core::OsRng;
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Create a deterministic genesis root for a zone (different per zone).
fn genesis_root(zone_label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"zone_genesis": zone_label})).unwrap();
    sha256_digest(&canonical)
}

/// Create a receipt with properly computed next_root for a given chain.
fn make_receipt(chain: &ReceiptChain) -> CorridorReceipt {
    make_receipt_with_digests(
        chain,
        vec!["aa".repeat(32).into()],
        vec!["bb".repeat(32).into()],
    )
}

/// Create a receipt with specific digest sets.
fn make_receipt_with_digests(
    chain: &ReceiptChain,
    lawpack_digests: Vec<DigestEntry>,
    ruleset_digests: Vec<DigestEntry>,
) -> CorridorReceipt {
    let mut receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id: chain.corridor_id().clone(),
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: lawpack_digests,
        ruleset_digest_set: ruleset_digests,
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    receipt.seal_next_root().unwrap();
    receipt
}

/// Create a dummy proof object for schema-requiring tests.
fn dummy_proof() -> ReceiptProof {
    ReceiptProof::Single(ProofObject {
        proof_type: "MezEd25519Signature2025".to_string(),
        created: "2026-02-19T12:00:00Z".to_string(),
        verification_method: "did:key:z6MkTestKey#key-1".to_string(),
        proof_purpose: "assertionMethod".to_string(),
        jws: "eyJ0eXAiOiJKV1MiLCJhbGciOiJFZERTQSJ9..test-signature".to_string(),
    })
}

fn make_key() -> SigningKey {
    SigningKey::generate(&mut OsRng)
}

fn make_digest(label: &str) -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"label": label})).unwrap();
    sha256_digest(&canonical)
}

// ---------------------------------------------------------------------------
// 1. Two zones initialize independent receipt chains with distinct genesis roots
// ---------------------------------------------------------------------------

#[test]
fn two_zones_independent_receipt_chains() {
    let corridor_id = CorridorId::new();
    let genesis_a = genesis_root("pk-sifc");
    let genesis_b = genesis_root("ae-difc");

    // Genesis roots must be different (distinct zones).
    assert_ne!(genesis_a.to_hex(), genesis_b.to_hex());

    let chain_a = ReceiptChain::new(corridor_id.clone(), genesis_a.clone());
    let chain_b = ReceiptChain::new(corridor_id, genesis_b.clone());

    assert_eq!(chain_a.height(), 0);
    assert_eq!(chain_b.height(), 0);
    assert_eq!(chain_a.final_state_root_hex(), genesis_a.to_hex());
    assert_eq!(chain_b.final_state_root_hex(), genesis_b.to_hex());
    assert_ne!(chain_a.final_state_root_hex(), chain_b.final_state_root_hex());
}

// ---------------------------------------------------------------------------
// 2. Zone A proposes corridor, Zone B accepts (via corridor definition)
// ---------------------------------------------------------------------------

#[test]
fn corridor_proposal_and_acceptance() {
    let corridor_id = CorridorId::new();
    let ja = JurisdictionId::new("PK-SIFC").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    // Zone A creates a corridor definition (simulated as JSON VC).
    let proposal = json!({
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "CorridorDefinition"],
        "issuer": format!("did:mass:zone:{}", ja.as_str()),
        "credentialSubject": {
            "corridor_id": corridor_id.to_string(),
            "jurisdiction_a": ja.as_str(),
            "jurisdiction_b": jb.as_str(),
            "corridor_type": "bilateral",
            "settlement_currency": "PKR"
        }
    });

    // Zone B creates an acceptance (simulated as JSON VC).
    let acceptance = json!({
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "CorridorAcceptance"],
        "issuer": format!("did:mass:zone:{}", jb.as_str()),
        "credentialSubject": {
            "corridor_id": corridor_id.to_string(),
            "accepted": true,
            "accepted_at": Timestamp::now().to_canonical_string()
        }
    });

    // Verify both are valid JSON and reference the same corridor.
    let prop_subject = proposal["credentialSubject"]["corridor_id"].as_str().unwrap();
    let acc_subject = acceptance["credentialSubject"]["corridor_id"].as_str().unwrap();
    assert_eq!(prop_subject, acc_subject);
    assert_eq!(prop_subject, corridor_id.to_string());
    assert!(acceptance["credentialSubject"]["accepted"].as_bool().unwrap());
}

// ---------------------------------------------------------------------------
// 3. Zone A creates receipts with compliance evaluation, appends to chain
// ---------------------------------------------------------------------------

#[test]
fn zone_a_creates_receipts_with_compliance() {
    let corridor_id = CorridorId::new();
    let genesis_a = genesis_root("pk-sifc");
    let mut chain_a = ReceiptChain::new(corridor_id, genesis_a);

    // Compliance evaluation: set up tensor for Zone A.
    let jid_a = JurisdictionId::new("PK-SIFC").unwrap();
    let mut tensor_a = ComplianceTensor::new(DefaultJurisdiction::new(jid_a));
    tensor_a.set(ComplianceDomain::Aml, ComplianceState::Compliant, vec![], None);
    tensor_a.set(ComplianceDomain::Kyc, ComplianceState::Compliant, vec![], None);
    tensor_a.set(ComplianceDomain::Sanctions, ComplianceState::Compliant, vec![], None);
    tensor_a.set(ComplianceDomain::Tax, ComplianceState::Compliant, vec![], None);

    // Create and append 5 receipts.
    for i in 0..5 {
        let receipt = make_receipt_with_digests(
            &chain_a,
            vec![format!("{:064x}", i).into()],
            vec![format!("{:064x}", i + 100).into()],
        );
        // Verify next_root was properly computed (I-RECEIPT-COMMIT).
        let computed = mez_corridor::compute_next_root(&receipt).unwrap();
        assert_eq!(receipt.next_root, computed.to_hex());

        chain_a.append(receipt).unwrap();
    }

    assert_eq!(chain_a.height(), 5);

    // Verify compliance tensor has mandatory domains evaluated (not empty).
    let base_domains = [
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ];
    let slice = tensor_a.slice(&base_domains);
    assert!(slice.all_passing(), "base domains should all pass after explicit set");
    assert!(!slice.is_empty());
}

// ---------------------------------------------------------------------------
// 4. Receipt serialization → transmission → deserialization → Zone B verification
// ---------------------------------------------------------------------------

#[test]
fn receipt_cross_zone_transmission_and_verification() {
    let corridor_id = CorridorId::new();
    let genesis_a = genesis_root("pk-sifc");
    let genesis_b = genesis_root("ae-difc");

    let mut chain_a = ReceiptChain::new(corridor_id.clone(), genesis_a.clone());
    // Zone B has the same corridor but its own genesis root.
    // For cross-zone receipt verification, Zone B needs Zone A's genesis root.
    let mut chain_b_view = ReceiptChain::new(corridor_id, genesis_a.clone());

    // Zone A creates 3 receipts.
    for _ in 0..3 {
        let receipt = make_receipt(&chain_a);
        chain_a.append(receipt).unwrap();
    }
    assert_eq!(chain_a.height(), 3);

    // Simulate transmission: serialize → deserialize each receipt.
    for receipt in chain_a.receipts() {
        let json_str = serde_json::to_string(receipt).expect("serialize receipt");
        let deserialized: CorridorReceipt =
            serde_json::from_str(&json_str).expect("deserialize receipt");

        // Zone B verifies and appends.
        // next_root verification happens inside append().
        chain_b_view.append(deserialized).unwrap();
    }

    // Zone B's view should match Zone A's state.
    assert_eq!(chain_b_view.height(), chain_a.height());
    assert_eq!(chain_b_view.final_state_root_hex(), chain_a.final_state_root_hex());
    assert_eq!(chain_b_view.mmr_root().unwrap(), chain_a.mmr_root().unwrap());

    // Verify: genesis_b is indeed different from genesis_a (zones are independent).
    assert_ne!(genesis_a.to_hex(), genesis_b.to_hex());
}

// ---------------------------------------------------------------------------
// 5. Zone B creates a checkpoint with all required fields
// ---------------------------------------------------------------------------

#[test]
fn zone_b_checkpoint_contains_required_fields() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("ae-difc");
    let mut chain = ReceiptChain::new(corridor_id, genesis.clone());

    // Append receipts.
    for _ in 0..7 {
        let receipt = make_receipt(&chain);
        chain.append(receipt).unwrap();
    }

    let checkpoint = chain.create_checkpoint().unwrap();

    // Verify all required fields per P0-CORRIDOR-004 and schema.
    assert_eq!(checkpoint.checkpoint_type, "MEZCorridorStateCheckpoint");
    assert_eq!(checkpoint.genesis_root, genesis.to_hex());
    assert_eq!(checkpoint.final_state_root, chain.final_state_root_hex());
    assert_eq!(checkpoint.receipt_count, 7);
    assert_eq!(checkpoint.height(), 7);

    // MMR commitment fields.
    assert_eq!(checkpoint.mmr.mmr_type, "MEZReceiptMMR");
    assert_eq!(checkpoint.mmr.algorithm, "sha256");
    assert_eq!(checkpoint.mmr.size, 7);
    assert_eq!(checkpoint.mmr.root, chain.mmr_root().unwrap());
    assert_eq!(checkpoint.mmr.root.len(), 64);

    // Checkpoint digest is non-zero.
    assert_ne!(checkpoint.checkpoint_digest.to_hex(), "00".repeat(32));
}

// ---------------------------------------------------------------------------
// 6. Watcher attestation: signed attestation binds candidate root + height
// ---------------------------------------------------------------------------

#[test]
fn watcher_attestation_and_fork_resolution() {
    let sk_a = make_key();
    let sk_b1 = make_key();
    let sk_b2 = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_a.verifying_key());
    reg.register(sk_b1.verifying_key());
    reg.register(sk_b2.verifying_key());

    let t = Utc::now();
    let nr_honest = "aa".repeat(32);
    let nr_attacker = "bb".repeat(32);

    // Honest branch with 2 attestations.
    let honest = ForkBranch {
        receipt_digest: make_digest("honest-receipt"),
        timestamp: t,
        attestations: vec![
            create_attestation(&sk_b1, "parent", &nr_honest, 1, t).unwrap(),
            create_attestation(&sk_b2, "parent", &nr_honest, 1, t).unwrap(),
        ],
        next_root: nr_honest.clone(),
    };

    // Attacker branch with 1 attestation.
    let attacker = ForkBranch {
        receipt_digest: make_digest("attacker-receipt"),
        timestamp: t,
        attestations: vec![
            create_attestation(&sk_a, "parent", &nr_attacker, 1, t).unwrap(),
        ],
        next_root: nr_attacker.clone(),
    };

    assert!(ForkDetector::is_fork(&honest, &attacker));

    let resolution = resolve_fork(&honest, &attacker, &reg).unwrap();
    // Same timestamp → falls to attestation count → honest wins (2 > 1).
    assert_eq!(resolution.winning_branch, honest.receipt_digest);
    assert_eq!(resolution.resolution_reason, ResolutionReason::MoreAttestations);
    assert_eq!(resolution.winning_attestation_count, 2);
    assert_eq!(resolution.losing_attestation_count, 1);
}

// ---------------------------------------------------------------------------
// 7. Adversarial: forged next_root is rejected (I-RECEIPT-COMMIT)
// ---------------------------------------------------------------------------

#[test]
fn adversarial_forged_next_root_rejected() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id.clone(), genesis);

    // Append a valid receipt first.
    let valid = make_receipt(&chain);
    chain.append(valid).unwrap();

    // Create a receipt with a forged next_root.
    let forged = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id,
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: "ff".repeat(32), // Forged! Not computed from payload.
        lawpack_digest_set: vec!["cc".repeat(32).into()],
        ruleset_digest_set: vec![],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };

    // Verify the forged next_root doesn't match what compute_next_root produces.
    let expected = mez_corridor::compute_next_root(&forged).unwrap();
    assert_ne!(forged.next_root, expected.to_hex());

    // Append must reject the forged receipt (I-RECEIPT-COMMIT invariant).
    let result = chain.append(forged);
    assert!(result.is_err(), "forged next_root must be rejected");
    let err = result.unwrap_err();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains("NextRootMismatch"),
        "error should be NextRootMismatch, got: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// 8. Adversarial: duplicate receipt (replay) rejected (I-RECEIPT-LINK)
// ---------------------------------------------------------------------------

#[test]
fn adversarial_duplicate_receipt_rejected() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id, genesis);

    // Append a valid receipt.
    let receipt = make_receipt(&chain);
    let receipt_clone = receipt.clone();
    chain.append(receipt).unwrap();
    assert_eq!(chain.height(), 1);

    // Try to replay the same receipt.
    // It should fail because:
    // - sequence 0 != chain.height() (1) → SequenceMismatch
    // - OR prev_root doesn't match final_state_root → PrevRootMismatch
    let result = chain.append(receipt_clone);
    assert!(result.is_err(), "duplicate receipt must be rejected");
    let err = result.unwrap_err();
    let err_str = format!("{err:?}");
    assert!(
        err_str.contains("SequenceMismatch") || err_str.contains("PrevRootMismatch"),
        "error should be SequenceMismatch or PrevRootMismatch, got: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// 9. Adversarial: prev_root mismatch from different chain state
// ---------------------------------------------------------------------------

#[test]
fn adversarial_prev_root_mismatch_rejected() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id.clone(), genesis);

    let receipt = make_receipt(&chain);
    chain.append(receipt).unwrap();

    // Create a receipt with wrong prev_root (pointing to genesis instead of current).
    let mut bad_receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id,
        sequence: chain.height(),
        timestamp: Timestamp::now(),
        prev_root: "00".repeat(32), // Wrong prev_root.
        next_root: String::new(),
        lawpack_digest_set: vec!["dd".repeat(32).into()],
        ruleset_digest_set: vec![],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    bad_receipt.seal_next_root().unwrap();

    let result = chain.append(bad_receipt);
    assert!(result.is_err(), "wrong prev_root must be rejected");
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(
        err_str.contains("PrevRootMismatch"),
        "error should be PrevRootMismatch, got: {err_str}"
    );
}

// ---------------------------------------------------------------------------
// 10. Compliance tensor: all mandatory domains present, empty slices fail
// ---------------------------------------------------------------------------

#[test]
fn compliance_tensor_mandatory_domains_no_empty_slices() {
    let jid_a = JurisdictionId::new("PK-SIFC").unwrap();
    let jid_b = JurisdictionId::new("AE-DIFC").unwrap();

    // Zone A tensor.
    let mut tensor_a = ComplianceTensor::new(DefaultJurisdiction::new(jid_a));
    // Zone B tensor.
    let mut tensor_b = ComplianceTensor::new(DefaultJurisdiction::new(jid_b));

    // Set base compliance domains for both zones.
    let base_domains = [
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
        ComplianceDomain::Securities,
        ComplianceDomain::Corporate,
        ComplianceDomain::Custody,
        ComplianceDomain::DataPrivacy,
    ];

    for domain in &base_domains {
        tensor_a.set(*domain, ComplianceState::Compliant, vec![], None);
        tensor_b.set(*domain, ComplianceState::Compliant, vec![], None);
    }

    // Full slice for both zones.
    let slice_a = tensor_a.full_slice();
    let slice_b = tensor_b.full_slice();

    // Both should have 20 domains (all ComplianceDomain variants).
    assert_eq!(slice_a.len(), 20);
    assert_eq!(slice_b.len(), 20);

    // Base domains should be Compliant.
    for domain in &base_domains {
        assert_eq!(
            tensor_a.get(*domain),
            ComplianceState::Compliant,
            "Zone A domain {:?} should be Compliant",
            domain
        );
        assert_eq!(
            tensor_b.get(*domain),
            ComplianceState::Compliant,
            "Zone B domain {:?} should be Compliant",
            domain
        );
    }

    // Extended domains default to Pending (fail-closed per P0-TENSOR-001).
    let extended_domains = [
        ComplianceDomain::Licensing,
        ComplianceDomain::Banking,
        ComplianceDomain::Payments,
        ComplianceDomain::Clearing,
        ComplianceDomain::Settlement,
        ComplianceDomain::DigitalAssets,
        ComplianceDomain::Employment,
        ComplianceDomain::Immigration,
        ComplianceDomain::Ip,
        ComplianceDomain::ConsumerProtection,
        ComplianceDomain::Arbitration,
        ComplianceDomain::Trade,
    ];

    for domain in &extended_domains {
        assert_eq!(
            tensor_a.get(*domain),
            ComplianceState::Pending,
            "Zone A extended domain {:?} should default to Pending (fail-closed)",
            domain
        );
    }

    // Empty slice must NOT pass (fail-closed: P0-TENSOR-001).
    let empty_slice = tensor_a.slice(&[]);
    assert_eq!(
        empty_slice.aggregate_state(),
        ComplianceState::Pending,
        "empty slice must return Pending, not Compliant"
    );
    assert!(
        !empty_slice.all_passing(),
        "empty slice must not pass"
    );
}

// ---------------------------------------------------------------------------
// 11. Cross-zone receipt chain growth is independent
// ---------------------------------------------------------------------------

#[test]
fn cross_zone_receipt_chains_grow_independently() {
    let corridor_id = CorridorId::new();
    let genesis_a = genesis_root("pk-sifc");
    let genesis_b = genesis_root("ae-difc");

    let mut chain_a = ReceiptChain::new(corridor_id.clone(), genesis_a);
    let mut chain_b = ReceiptChain::new(corridor_id, genesis_b);

    // Zone A appends 5 receipts, Zone B appends 3.
    for _ in 0..5 {
        chain_a.append(make_receipt(&chain_a)).unwrap();
    }
    for _ in 0..3 {
        chain_b.append(make_receipt(&chain_b)).unwrap();
    }

    assert_eq!(chain_a.height(), 5);
    assert_eq!(chain_b.height(), 3);
    assert_ne!(chain_a.mmr_root().unwrap(), chain_b.mmr_root().unwrap());
    assert_ne!(chain_a.final_state_root_hex(), chain_b.final_state_root_hex());
}

// ---------------------------------------------------------------------------
// 12. MMR inclusion proofs work across the full corridor lifecycle
// ---------------------------------------------------------------------------

#[test]
fn mmr_inclusion_proofs_across_full_lifecycle() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id, genesis);

    // Append 10 receipts.
    for _ in 0..10 {
        chain.append(make_receipt(&chain)).unwrap();
    }

    // Verify MMR root.
    let root = chain.mmr_root().unwrap();
    assert_eq!(root.len(), 64);
    assert!(root.chars().all(|c| c.is_ascii_hexdigit()));

    // Build and verify inclusion proofs for all receipts.
    for idx in 0..10 {
        let proof = chain.build_inclusion_proof(idx).unwrap();
        assert!(
            chain.verify_inclusion_proof(&proof).unwrap(),
            "inclusion proof for receipt {idx} must verify"
        );
    }

    // Tamper with a proof and verify it fails.
    let mut bad_proof = chain.build_inclusion_proof(5).unwrap();
    if !bad_proof.path.is_empty() {
        bad_proof.path[0].hash = "00".repeat(32);
    }
    assert!(
        !mez_corridor::receipt::verify_receipt_proof(&bad_proof),
        "tampered proof should fail verification"
    );
}

// ---------------------------------------------------------------------------
// 13. Multiple checkpoints track chain growth
// ---------------------------------------------------------------------------

#[test]
fn multiple_checkpoints_track_chain_growth() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("ae-difc");
    let mut chain = ReceiptChain::new(corridor_id, genesis.clone());

    // Checkpoint at height 3.
    for _ in 0..3 {
        chain.append(make_receipt(&chain)).unwrap();
    }
    let cp1 = chain.create_checkpoint().unwrap();

    // Checkpoint at height 8.
    for _ in 0..5 {
        chain.append(make_receipt(&chain)).unwrap();
    }
    let cp2 = chain.create_checkpoint().unwrap();

    assert_eq!(cp1.height(), 3);
    assert_eq!(cp2.height(), 8);
    assert_ne!(cp1.mmr_root(), cp2.mmr_root());
    assert_ne!(cp1.checkpoint_digest, cp2.checkpoint_digest);

    // Both share the same genesis root.
    assert_eq!(cp1.genesis_root, genesis.to_hex());
    assert_eq!(cp2.genesis_root, genesis.to_hex());

    // final_state_root progresses.
    assert_ne!(cp1.final_state_root, cp2.final_state_root);
    assert_eq!(chain.checkpoints().len(), 2);
}

// ---------------------------------------------------------------------------
// 14. Receipt content digest is deterministic across serialization
// ---------------------------------------------------------------------------

#[test]
fn receipt_content_digest_deterministic_across_serialization() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let chain = ReceiptChain::new(corridor_id, genesis);

    let receipt = make_receipt(&chain);
    let d1 = receipt.content_digest().unwrap();

    // Serialize and deserialize.
    let json = serde_json::to_string(&receipt).unwrap();
    let recovered: CorridorReceipt = serde_json::from_str(&json).unwrap();
    let d2 = recovered.content_digest().unwrap();

    assert_eq!(d1.to_hex(), d2.to_hex(), "content digest must survive serialization roundtrip");
}

// ---------------------------------------------------------------------------
// 15. Fork detector lifecycle with cross-zone branches
// ---------------------------------------------------------------------------

#[test]
fn fork_detector_cross_zone_resolution() {
    let sk_zone_a = make_key();
    let sk_zone_b = make_key();
    let mut reg = WatcherRegistry::new();
    reg.register(sk_zone_a.verifying_key());
    reg.register(sk_zone_b.verifying_key());

    let mut detector = ForkDetector::new(reg);
    let t = Utc::now();

    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    let branch_a = ForkBranch {
        receipt_digest: make_digest("zone-a-receipt"),
        timestamp: t,
        attestations: vec![create_attestation(&sk_zone_a, "p", &nr_a, 1, t).unwrap()],
        next_root: nr_a.clone(),
    };
    let branch_b = ForkBranch {
        receipt_digest: make_digest("zone-b-receipt"),
        timestamp: t,
        attestations: vec![create_attestation(&sk_zone_b, "p", &nr_b, 1, t).unwrap()],
        next_root: nr_b.clone(),
    };

    assert!(ForkDetector::is_fork(&branch_a, &branch_b));
    detector.register_fork(branch_a, branch_b);
    assert_eq!(detector.pending_count(), 1);

    let resolutions = detector.resolve_all();
    assert_eq!(resolutions.len(), 1);
    let resolution = resolutions[0].as_ref().unwrap();
    // Same timestamp, same attestation count → lexicographic tiebreak.
    assert_eq!(resolution.resolution_reason, ResolutionReason::LexicographicTiebreak);
    assert_eq!(detector.pending_count(), 0);
}

// ---------------------------------------------------------------------------
// 16. Compliance tensor merge across zones
// ---------------------------------------------------------------------------

#[test]
fn compliance_tensor_merge_across_zones() {
    let jid = JurisdictionId::new("PK-SIFC").unwrap();
    let mut tensor_local = ComplianceTensor::new(DefaultJurisdiction::new(jid.clone()));
    let mut tensor_remote = ComplianceTensor::new(DefaultJurisdiction::new(jid));

    // Both zones evaluate all four domains — merge uses lattice meet (pessimistic).
    tensor_local.set(ComplianceDomain::Aml, ComplianceState::Compliant, vec![], None);
    tensor_local.set(ComplianceDomain::Kyc, ComplianceState::Compliant, vec![], None);
    tensor_local.set(ComplianceDomain::Sanctions, ComplianceState::Compliant, vec![], None);
    tensor_local.set(ComplianceDomain::Tax, ComplianceState::Compliant, vec![], None);

    // Remote zone: AML and KYC compliant, Sanctions compliant, Tax non-compliant.
    tensor_remote.set(ComplianceDomain::Aml, ComplianceState::Compliant, vec![], None);
    tensor_remote.set(ComplianceDomain::Kyc, ComplianceState::Compliant, vec![], None);
    tensor_remote.set(ComplianceDomain::Sanctions, ComplianceState::Compliant, vec![], None);
    tensor_remote.set(ComplianceDomain::Tax, ComplianceState::NonCompliant, vec![], None);

    // Merge remote into local. Meet (pessimistic): min of each domain.
    tensor_local.merge(&tensor_remote);

    // AML, KYC, Sanctions: Compliant.meet(Compliant) = Compliant.
    assert_eq!(tensor_local.get(ComplianceDomain::Aml), ComplianceState::Compliant);
    assert_eq!(tensor_local.get(ComplianceDomain::Kyc), ComplianceState::Compliant);
    assert_eq!(tensor_local.get(ComplianceDomain::Sanctions), ComplianceState::Compliant);
    // Tax: Compliant.meet(NonCompliant) = NonCompliant (lattice min).
    assert_eq!(tensor_local.get(ComplianceDomain::Tax), ComplianceState::NonCompliant);

    // NonCompliant domain should make the aggregate non-passing.
    let slice = tensor_local.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ]);
    assert!(!slice.all_passing(), "NonCompliant Tax should prevent all_passing");
    assert_eq!(
        slice.non_compliant_domains(),
        vec![ComplianceDomain::Tax]
    );
}

// ---------------------------------------------------------------------------
// 17. Receipt with optional fields validates correctly
// ---------------------------------------------------------------------------

#[test]
fn receipt_with_optional_fields_round_trips() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let chain = ReceiptChain::new(corridor_id.clone(), genesis);

    let mut receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id,
        sequence: 0,
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["aa".repeat(32).into()],
        ruleset_digest_set: vec!["bb".repeat(32).into()],
        proof: None,
        transition: Some(json!({
            "type": "MEZTransitionEnvelope",
            "kind": "cross_zone.transfer.v1",
            "payload_sha256": "cc".repeat(32)
        })),
        transition_type_registry_digest_sha256: Some("dd".repeat(32)),
        zk: Some(json!({"proof_system": "groth16"})),
        anchor: Some(json!({"chain_id": "ethereum", "method": "calldata"})),
    };
    receipt.seal_next_root().unwrap();
    receipt.proof = Some(dummy_proof());

    // Serialize and deserialize.
    let json_str = serde_json::to_string(&receipt).unwrap();
    let recovered: CorridorReceipt = serde_json::from_str(&json_str).unwrap();

    assert!(recovered.transition.is_some());
    assert!(recovered.transition_type_registry_digest_sha256.is_some());
    assert!(recovered.zk.is_some());
    assert!(recovered.anchor.is_some());
    assert!(recovered.proof.is_some());

    // Content digest matches.
    assert_eq!(
        receipt.content_digest().unwrap().to_hex(),
        recovered.content_digest().unwrap().to_hex()
    );
}

// ---------------------------------------------------------------------------
// 18. Watcher equivocation detection across zones
// ---------------------------------------------------------------------------

#[test]
fn watcher_equivocation_detected() {
    let sk_equivocator = make_key();
    let t = Utc::now();

    let nr_a = "aa".repeat(32);
    let nr_b = "bb".repeat(32);

    // Same watcher signs attestations for two different candidate roots.
    let branch_a = ForkBranch {
        receipt_digest: make_digest("branch-a"),
        timestamp: t,
        attestations: vec![
            create_attestation(&sk_equivocator, "parent", &nr_a, 1, t).unwrap(),
        ],
        next_root: nr_a,
    };
    let branch_b = ForkBranch {
        receipt_digest: make_digest("branch-b"),
        timestamp: t,
        attestations: vec![
            create_attestation(&sk_equivocator, "parent", &nr_b, 1, t).unwrap(),
        ],
        next_root: nr_b,
    };

    let equivocating = ForkDetector::detect_equivocation(&branch_a, &branch_b);
    assert_eq!(equivocating.len(), 1, "should detect 1 equivocating watcher");
}

// ---------------------------------------------------------------------------
// 19. Sequence mismatch is caught correctly
// ---------------------------------------------------------------------------

#[test]
fn sequence_mismatch_rejected() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id.clone(), genesis);

    chain.append(make_receipt(&chain)).unwrap();

    // Create a receipt with wrong sequence (jumping ahead).
    let mut bad_receipt = CorridorReceipt {
        receipt_type: "MEZCorridorStateReceipt".to_string(),
        corridor_id,
        sequence: 5, // Should be 1.
        timestamp: Timestamp::now(),
        prev_root: chain.final_state_root_hex(),
        next_root: String::new(),
        lawpack_digest_set: vec!["ee".repeat(32).into()],
        ruleset_digest_set: vec![],
        proof: None,
        transition: None,
        transition_type_registry_digest_sha256: None,
        zk: None,
        anchor: None,
    };
    bad_receipt.seal_next_root().unwrap();

    let result = chain.append(bad_receipt);
    assert!(result.is_err());
    let err_str = format!("{:?}", result.unwrap_err());
    assert!(err_str.contains("SequenceMismatch"), "got: {err_str}");
}

// ---------------------------------------------------------------------------
// 20. DigestEntry types are handled correctly in receipt chains
// ---------------------------------------------------------------------------

#[test]
fn digest_entry_types_in_receipt() {
    let corridor_id = CorridorId::new();
    let genesis = genesis_root("pk-sifc");
    let mut chain = ReceiptChain::new(corridor_id, genesis);

    // Create receipt with both DigestEntry variants.
    let receipt = make_receipt_with_digests(
        &chain,
        vec![
            DigestEntry::Digest("aa".repeat(32)),
            DigestEntry::ArtifactRef {
                digest_sha256: "bb".repeat(32),
                artifact_type: "lawpack".to_string(),
                uri: Some("mez://lawpacks/pk-income-tax".to_string()),
            },
        ],
        vec![DigestEntry::Digest("cc".repeat(32))],
    );

    chain.append(receipt).unwrap();
    assert_eq!(chain.height(), 1);

    // Verify the receipt serializes and deserializes with artifact refs.
    let stored = &chain.receipts()[0];
    let json = serde_json::to_string(stored).unwrap();
    let recovered: CorridorReceipt = serde_json::from_str(&json).unwrap();
    assert_eq!(recovered.lawpack_digest_set.len(), 2);
}

// ---------------------------------------------------------------------------
// 21. End-to-end: full two-zone corridor lifecycle
// ---------------------------------------------------------------------------

#[test]
fn full_two_zone_corridor_lifecycle() {
    // --- Setup ---
    let corridor_id = CorridorId::new();
    let genesis_a = genesis_root("pk-sifc");
    let genesis_b = genesis_root("ae-difc");

    let mut chain_a = ReceiptChain::new(corridor_id.clone(), genesis_a.clone());
    // Zone B maintains its own view of Zone A's chain for verification.
    let mut chain_b_view_of_a = ReceiptChain::new(corridor_id.clone(), genesis_a.clone());
    let mut chain_b = ReceiptChain::new(corridor_id.clone(), genesis_b.clone());

    // --- Phase 1: Zone A creates receipts ---
    for _ in 0..5 {
        let receipt = make_receipt(&chain_a);
        chain_a.append(receipt).unwrap();
    }

    // --- Phase 2: Transmit Zone A receipts to Zone B ---
    for receipt in chain_a.receipts() {
        let serialized = serde_json::to_string(receipt).unwrap();
        let deserialized: CorridorReceipt = serde_json::from_str(&serialized).unwrap();
        chain_b_view_of_a.append(deserialized).unwrap();
    }
    assert_eq!(chain_b_view_of_a.height(), 5);
    assert_eq!(chain_b_view_of_a.mmr_root().unwrap(), chain_a.mmr_root().unwrap());

    // --- Phase 3: Zone B creates its own receipts ---
    for _ in 0..3 {
        let receipt = make_receipt(&chain_b);
        chain_b.append(receipt).unwrap();
    }

    // --- Phase 4: Both zones create checkpoints ---
    let cp_a = chain_a.create_checkpoint().unwrap();
    let cp_b = chain_b.create_checkpoint().unwrap();

    assert_eq!(cp_a.height(), 5);
    assert_eq!(cp_b.height(), 3);
    assert_eq!(cp_a.genesis_root, genesis_a.to_hex());
    assert_eq!(cp_b.genesis_root, genesis_b.to_hex());
    assert_ne!(cp_a.mmr_root(), cp_b.mmr_root());

    // --- Phase 5: Compliance evaluation for both zones ---
    let jid_a = JurisdictionId::new("PK-SIFC").unwrap();
    let jid_b = JurisdictionId::new("AE-DIFC").unwrap();
    let mut tensor_a = ComplianceTensor::new(DefaultJurisdiction::new(jid_a));
    let mut tensor_b = ComplianceTensor::new(DefaultJurisdiction::new(jid_b));

    // Evaluate mandatory base domains.
    for domain in [
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ] {
        tensor_a.set(domain, ComplianceState::Compliant, vec![], None);
        tensor_b.set(domain, ComplianceState::Compliant, vec![], None);
    }

    let slice_a = tensor_a.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ]);
    let slice_b = tensor_b.slice(&[
        ComplianceDomain::Aml,
        ComplianceDomain::Kyc,
        ComplianceDomain::Sanctions,
        ComplianceDomain::Tax,
    ]);

    assert!(slice_a.all_passing());
    assert!(slice_b.all_passing());
    assert_eq!(slice_a.aggregate_state(), ComplianceState::Compliant);
    assert_eq!(slice_b.aggregate_state(), ComplianceState::Compliant);

    // Full corridor lifecycle complete:
    // - Independent chains initialized with distinct genesis roots
    // - Receipts created, sealed, appended, transmitted, verified
    // - Checkpoints created with all required fields
    // - Compliance evaluated across both zones
}
