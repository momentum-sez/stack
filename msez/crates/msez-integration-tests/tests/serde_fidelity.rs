//! # Campaign 1: Serde Round-Trip Fidelity
//!
//! Verifies that every type deriving both Serialize and Deserialize
//! survives a JSON round-trip without data loss.
//!
//! Types missing PartialEq are logged as bugs — they cannot be
//! mechanically verified for round-trip fidelity.

use chrono::Utc;
use serde_json::json;

// =========================================================================
// msez-core types
// =========================================================================

use msez_core::{
    CanonicalBytes, ComplianceDomain, ContentDigest, CorridorId, DigestAlgorithm, EntityId,
    JurisdictionId, MigrationId, Timestamp, WatcherId,
};

// =========================================================================
// msez-state types
// =========================================================================

use msez_state::{
    DynCorridorData, DynCorridorState, EntityLifecycleState, LicenseState, MigrationState,
    SlashingCondition, TransitionRecord, WatcherState,
};

// =========================================================================
// msez-corridor types
// =========================================================================

use msez_corridor::netting::{
    Currency, NetPosition, NettingEngine, Obligation, Party, SettlementLeg, SettlementPlan,
};
use msez_corridor::swift::SettlementInstruction;

// =========================================================================
// msez-vc types
// =========================================================================

use msez_vc::credential::{ContextValue, CredentialTypeValue, ProofValue, VerifiableCredential};
use msez_vc::proof::{Proof, ProofPurpose, ProofType};

// =========================================================================
// msez-arbitration types
// =========================================================================

use msez_arbitration::dispute::{DisputeId, DisputeState, DisputeType};
use msez_arbitration::enforcement::{EnforcementOrderId, EnforcementReceiptId, EnforcementStatus};
use msez_arbitration::escrow::{
    EscrowId, EscrowStatus, EscrowType, ReleaseConditionType, TransactionType,
};

// =========================================================================
// msez-agentic types
// =========================================================================

use msez_agentic::audit::{AuditEntry, AuditEntryType};
use msez_agentic::policy::{
    AuthorizationRequirement, Condition, ImpactLevel, PolicyAction, TriggerType,
};
use msez_agentic::scheduler::ActionStatus;

// =========================================================================
// msez-tensor types
// =========================================================================

use msez_tensor::evaluation::{AttestationRef, ComplianceState};
use msez_tensor::tensor::TensorCell;

// =========================================================================
// Helpers
// =========================================================================

fn test_digest() -> ContentDigest {
    let canonical = CanonicalBytes::new(&json!({"test": "serde_fidelity"})).unwrap();
    msez_core::sha256_digest(&canonical)
}

// =========================================================================
// msez-core serde round-trips
// =========================================================================

#[test]
fn serde_rt_entity_id() {
    let original = EntityId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EntityId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_migration_id() {
    let original = MigrationId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MigrationId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_watcher_id() {
    let original = WatcherId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: WatcherId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_jurisdiction_id() {
    let original = JurisdictionId::new("PK-RSEZ").unwrap();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: JurisdictionId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_corridor_id() {
    let original = CorridorId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: CorridorId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_content_digest() {
    let original = test_digest();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ContentDigest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_digest_algorithm() {
    for alg in [DigestAlgorithm::Sha256, DigestAlgorithm::Poseidon2] {
        let json = serde_json::to_string(&alg).expect("serialize");
        let recovered: DigestAlgorithm = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(alg, recovered);
    }
}

#[test]
fn serde_rt_compliance_domain_all_variants() {
    for domain in ComplianceDomain::all() {
        let json = serde_json::to_string(domain).expect("serialize");
        let recovered: ComplianceDomain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*domain, recovered);
    }
}

#[test]
fn serde_rt_timestamp() {
    let original = Timestamp::from_rfc3339("2026-01-15T12:00:00Z").unwrap();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Timestamp = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

// =========================================================================
// msez-state serde round-trips
// =========================================================================

#[test]
fn serde_rt_dyn_corridor_state_all_variants() {
    let states = [
        DynCorridorState::Draft,
        DynCorridorState::Pending,
        DynCorridorState::Active,
        DynCorridorState::Halted,
        DynCorridorState::Suspended,
        DynCorridorState::Deprecated,
    ];
    for state in &states {
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: DynCorridorState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_entity_lifecycle_state_all_variants() {
    let states = [
        EntityLifecycleState::Applied,
        EntityLifecycleState::Active,
        EntityLifecycleState::Suspended,
        EntityLifecycleState::Dissolving,
        EntityLifecycleState::Dissolved,
        EntityLifecycleState::Rejected,
    ];
    for state in &states {
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: EntityLifecycleState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_license_state_all_variants() {
    let states = [
        LicenseState::Applied,
        LicenseState::UnderReview,
        LicenseState::Active,
        LicenseState::Suspended,
        LicenseState::Revoked,
        LicenseState::Expired,
        LicenseState::Surrendered,
        LicenseState::Rejected,
    ];
    for state in &states {
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: LicenseState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_migration_state_all_variants() {
    let states = [
        MigrationState::Initiated,
        MigrationState::ComplianceCheck,
        MigrationState::AttestationGathering,
        MigrationState::SourceLocked,
        MigrationState::InTransit,
        MigrationState::DestinationVerification,
        MigrationState::DestinationUnlock,
        MigrationState::Completed,
        MigrationState::Compensated,
        MigrationState::TimedOut,
        MigrationState::Cancelled,
    ];
    for state in &states {
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: MigrationState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_watcher_state_all_variants() {
    let states = [
        WatcherState::Registered,
        WatcherState::Bonded,
        WatcherState::Active,
        WatcherState::Slashed,
        WatcherState::Unbonding,
        WatcherState::Deactivated,
        WatcherState::Banned,
    ];
    for state in &states {
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: WatcherState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_slashing_condition_all_variants() {
    let conditions = [
        SlashingCondition::Equivocation,
        SlashingCondition::AvailabilityFailure,
        SlashingCondition::FalseAttestation,
        SlashingCondition::Collusion,
    ];
    for cond in &conditions {
        let json = serde_json::to_string(cond).expect("serialize");
        let recovered: SlashingCondition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*cond, recovered);
    }
}

#[test]
fn serde_rt_transition_record() {
    // BUG-001: TransitionRecord does not derive PartialEq, so we cannot
    // assert_eq on the deserialized value. We can only verify round-trip
    // does not panic and key fields survive.
    let original = TransitionRecord {
        from_state: DynCorridorState::Draft,
        to_state: DynCorridorState::Pending,
        timestamp: Utc::now(),
        evidence_digest: Some(test_digest()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: TransitionRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.from_state, DynCorridorState::Draft);
    assert_eq!(recovered.to_state, DynCorridorState::Pending);
    assert!(recovered.evidence_digest.is_some());
}

#[test]
fn serde_rt_dyn_corridor_data() {
    // BUG-002: DynCorridorData does not derive PartialEq
    let original = DynCorridorData {
        id: CorridorId::new(),
        jurisdiction_a: JurisdictionId::new("PK").unwrap(),
        jurisdiction_b: JurisdictionId::new("AE").unwrap(),
        state: DynCorridorState::Active,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        transition_log: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: DynCorridorData = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.state, DynCorridorState::Active);
    assert_eq!(recovered.id, original.id);
}

// =========================================================================
// msez-corridor serde round-trips
// =========================================================================

#[test]
fn serde_rt_party() {
    let original = Party {
        id: "party-001".to_string(),
        name: "Test Corp".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Party = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_currency() {
    let original = Currency {
        code: "USD".to_string(),
        precision: 2,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Currency = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_obligation() {
    // BUG-003: Obligation does not derive PartialEq — cannot verify round-trip fidelity.
    let original = Obligation {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: 100_000,
        currency: "USD".to_string(),
        corridor_id: Some("corridor-001".to_string()),
        priority: 5,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Obligation = serde_json::from_str(&json).expect("deserialize");
    // Must check field-by-field since no PartialEq
    assert_eq!(recovered.from_party, "A");
    assert_eq!(recovered.to_party, "B");
    assert_eq!(recovered.amount, 100_000);
    assert_eq!(recovered.currency, "USD");
    assert_eq!(recovered.corridor_id.as_deref(), Some("corridor-001"));
    assert_eq!(recovered.priority, 5);
}

#[test]
fn serde_rt_net_position() {
    // BUG-004: NetPosition does not derive PartialEq
    let original = NetPosition {
        party_id: "A".to_string(),
        currency: "USD".to_string(),
        receivable: 100,
        payable: 60,
        net: 40,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: NetPosition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.party_id, "A");
    assert_eq!(recovered.net, 40);
}

#[test]
fn serde_rt_settlement_leg() {
    // BUG-005: SettlementLeg does not derive PartialEq
    let original = SettlementLeg {
        from_party: "A".to_string(),
        to_party: "B".to_string(),
        amount: 40,
        currency: "USD".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SettlementLeg = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.from_party, "A");
    assert_eq!(recovered.amount, 40);
}

#[test]
fn serde_rt_settlement_plan() {
    // BUG-006/041 RESOLVED: SettlementPlan now derives PartialEq and uses
    // reduction_bps (u32 basis points) instead of f64.
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: 100,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    engine
        .add_obligation(Obligation {
            from_party: "B".to_string(),
            to_party: "A".to_string(),
            amount: 60,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .unwrap();
    let plan = engine.compute_plan().unwrap();
    let json = serde_json::to_string(&plan).expect("serialize");
    let recovered: SettlementPlan = serde_json::from_str(&json).expect("deserialize");
    // BUG-006 RESOLVED: PartialEq now derived, direct comparison works
    assert_eq!(
        recovered, plan,
        "SettlementPlan serde round-trip should be lossless"
    );
}

#[test]
fn serde_rt_settlement_instruction() {
    // BUG-007: SettlementInstruction does not derive PartialEq
    let original = SettlementInstruction {
        message_id: "MSG001".to_string(),
        debtor_bic: "DEUTDEFF".to_string(),
        debtor_account: "DE89370400440532013000".to_string(),
        debtor_name: "Test Debtor".to_string(),
        creditor_bic: "BKCHCNBJ".to_string(),
        creditor_account: "CN12345678".to_string(),
        creditor_name: "Test Creditor".to_string(),
        amount: 1_000_000,
        currency: "USD".to_string(),
        remittance_info: Some("Invoice INV-2026-001".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SettlementInstruction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.message_id, "MSG001");
    assert_eq!(recovered.amount, 1_000_000);
    assert_eq!(recovered.currency, "USD");
    assert_eq!(
        recovered.remittance_info.as_deref(),
        Some("Invoice INV-2026-001")
    );
}

// =========================================================================
// msez-vc serde round-trips
// =========================================================================

#[test]
fn serde_rt_proof_type_all_variants() {
    let types = [
        ProofType::Ed25519Signature2020,
        ProofType::MsezEd25519Signature2025,
        ProofType::BbsBlsSignature2020,
    ];
    for pt in &types {
        let json = serde_json::to_string(pt).expect("serialize");
        let recovered: ProofType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*pt, recovered);
    }
}

#[test]
fn serde_rt_proof_purpose_all_variants() {
    let purposes = [ProofPurpose::AssertionMethod, ProofPurpose::Authentication];
    for pp in &purposes {
        let json = serde_json::to_string(pp).expect("serialize");
        let recovered: ProofPurpose = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*pp, recovered);
    }
}

#[test]
fn serde_rt_proof() {
    // BUG-008: Proof does not derive PartialEq — cannot assert_eq on round-trip
    let original = Proof {
        proof_type: ProofType::Ed25519Signature2020,
        created: Utc::now(),
        verification_method: "did:key:z6MkTest#key-1".to_string(),
        proof_purpose: ProofPurpose::AssertionMethod,
        proof_value: "aa".repeat(64),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Proof = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.proof_type, ProofType::Ed25519Signature2020);
    assert_eq!(recovered.verification_method, "did:key:z6MkTest#key-1");
    assert_eq!(recovered.proof_value, "aa".repeat(64));
    // Verify W3C field names are used in JSON
    assert!(json.contains("\"type\""));
    assert!(json.contains("\"verificationMethod\""));
    assert!(json.contains("\"proofPurpose\""));
    assert!(json.contains("\"proofValue\""));
}

#[test]
fn serde_rt_verifiable_credential_unsigned() {
    // BUG-009: VerifiableCredential does not derive PartialEq
    let original = VerifiableCredential {
        context: ContextValue::Array(vec![json!("https://www.w3.org/2018/credentials/v1")]),
        id: Some("urn:msez:vc:test:serde-001".to_string()),
        credential_type: CredentialTypeValue::Array(vec![
            "VerifiableCredential".to_string(),
            "TestCredential".to_string(),
        ]),
        issuer: "did:key:z6MkTestIssuer".to_string(),
        issuance_date: Utc::now(),
        expiration_date: None,
        credential_subject: json!({"id": "subject-001", "name": "Test Subject"}),
        proof: ProofValue::default(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: VerifiableCredential = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.issuer, original.issuer);
    assert_eq!(recovered.id, original.id);
    // Verify W3C field naming in JSON
    assert!(json.contains("\"@context\""));
    assert!(json.contains("\"issuanceDate\""));
    assert!(json.contains("\"credentialSubject\""));
    // Verify proof is absent when empty (skip_serializing_if)
    assert!(
        !json.contains("\"proof\""),
        "Empty proof should be skipped in serialization"
    );
}

#[test]
fn serde_rt_verifiable_credential_with_expiration() {
    let original = VerifiableCredential {
        context: ContextValue::default(),
        id: Some("urn:msez:vc:test:expiry".to_string()),
        credential_type: CredentialTypeValue::Single("VerifiableCredential".to_string()),
        issuer: "did:key:z6MkIssuer".to_string(),
        issuance_date: Utc::now(),
        expiration_date: Some(Utc::now() + chrono::Duration::days(365)),
        credential_subject: json!({"status": "active"}),
        proof: ProofValue::default(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    assert!(json.contains("\"expirationDate\""));
    let recovered: VerifiableCredential = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.expiration_date.is_some());
}

#[test]
fn serde_rt_verifiable_credential_optional_id_absent() {
    let original = VerifiableCredential {
        context: ContextValue::default(),
        id: None,
        credential_type: CredentialTypeValue::Array(vec!["VerifiableCredential".to_string()]),
        issuer: "did:key:z6MkIssuer".to_string(),
        issuance_date: Utc::now(),
        expiration_date: None,
        credential_subject: json!({}),
        proof: ProofValue::default(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    // id should be absent (skip_serializing_if)
    assert!(
        !json.contains("\"id\""),
        "None id should be skipped: {json}"
    );
    let recovered: VerifiableCredential = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.id.is_none());
}

#[test]
fn serde_rt_context_value_single() {
    let original = ContextValue::Single("https://www.w3.org/2018/credentials/v1".to_string());
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ContextValue = serde_json::from_str(&json).expect("deserialize");
    match recovered {
        ContextValue::Single(s) => {
            assert_eq!(s, "https://www.w3.org/2018/credentials/v1");
        }
        _ => panic!("Expected ContextValue::Single"),
    }
}

#[test]
fn serde_rt_context_value_array() {
    let original = ContextValue::Array(vec![
        json!("https://www.w3.org/2018/credentials/v1"),
        json!({"@vocab": "https://mass.inc/vc/v1#"}),
    ]);
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ContextValue = serde_json::from_str(&json).expect("deserialize");
    match recovered {
        ContextValue::Array(arr) => assert_eq!(arr.len(), 2),
        _ => panic!("Expected ContextValue::Array"),
    }
}

// =========================================================================
// msez-arbitration serde round-trips
// =========================================================================

#[test]
fn serde_rt_dispute_id() {
    let original = DisputeId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: DisputeId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_dispute_state_all_variants() {
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
        let json = serde_json::to_string(state).expect("serialize");
        let recovered: DisputeState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*state, recovered);
    }
}

#[test]
fn serde_rt_dispute_type_all_variants() {
    let types = [
        DisputeType::BreachOfContract,
        DisputeType::NonConformingGoods,
        DisputeType::PaymentDefault,
        DisputeType::DeliveryFailure,
        DisputeType::QualityDefect,
        DisputeType::DocumentaryDiscrepancy,
        DisputeType::ForceMajeure,
        DisputeType::FraudulentMisrepresentation,
    ];
    for dt in &types {
        let json = serde_json::to_string(dt).expect("serialize");
        let recovered: DisputeType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*dt, recovered);
    }
}

#[test]
fn serde_rt_escrow_id() {
    let original = EscrowId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EscrowId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_escrow_type_all_variants() {
    let types = [
        EscrowType::FilingFee,
        EscrowType::SecurityDeposit,
        EscrowType::AwardEscrow,
        EscrowType::AppealBond,
    ];
    for et in &types {
        let json = serde_json::to_string(et).expect("serialize");
        let recovered: EscrowType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*et, recovered);
    }
}

#[test]
fn serde_rt_escrow_status_all_variants() {
    let statuses = [
        EscrowStatus::Pending,
        EscrowStatus::Funded,
        EscrowStatus::PartiallyReleased,
        EscrowStatus::FullyReleased,
        EscrowStatus::Forfeited,
    ];
    for es in &statuses {
        let json = serde_json::to_string(es).expect("serialize");
        let recovered: EscrowStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*es, recovered);
    }
}

#[test]
fn serde_rt_release_condition_type_all_variants() {
    let types = [
        ReleaseConditionType::RulingEnforced,
        ReleaseConditionType::AppealPeriodExpired,
        ReleaseConditionType::SettlementAgreed,
        ReleaseConditionType::DisputeWithdrawn,
        ReleaseConditionType::InstitutionOrder,
    ];
    for rct in &types {
        let json = serde_json::to_string(rct).expect("serialize");
        let recovered: ReleaseConditionType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*rct, recovered);
    }
}

#[test]
fn serde_rt_transaction_type_all_variants() {
    let types = [
        TransactionType::Deposit,
        TransactionType::FullRelease,
        TransactionType::PartialRelease,
        TransactionType::Forfeit,
        TransactionType::Refund,
    ];
    for tt in &types {
        let json = serde_json::to_string(tt).expect("serialize");
        let recovered: TransactionType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*tt, recovered);
    }
}

#[test]
fn serde_rt_enforcement_order_id() {
    let original = EnforcementOrderId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EnforcementOrderId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_enforcement_receipt_id() {
    let original = EnforcementReceiptId::new();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EnforcementReceiptId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_enforcement_status_all_variants() {
    let statuses = [
        EnforcementStatus::Pending,
        EnforcementStatus::InProgress,
        EnforcementStatus::Completed,
        EnforcementStatus::Blocked,
        EnforcementStatus::Cancelled,
    ];
    for es in &statuses {
        let json = serde_json::to_string(es).expect("serialize");
        let recovered: EnforcementStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*es, recovered);
    }
}

// =========================================================================
// msez-agentic serde round-trips
// =========================================================================

#[test]
fn serde_rt_trigger_type_all_variants() {
    for tt in TriggerType::all() {
        let json = serde_json::to_string(tt).expect("serialize");
        let recovered: TriggerType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*tt, recovered);
    }
}

#[test]
fn serde_rt_policy_action_all_variants() {
    let actions = [
        PolicyAction::Transfer,
        PolicyAction::Mint,
        PolicyAction::Burn,
        PolicyAction::ActivateBinding,
        PolicyAction::DeactivateBinding,
        PolicyAction::MigrateBinding,
        PolicyAction::UpdateManifest,
        PolicyAction::AmendGovernance,
        PolicyAction::AddGovernor,
        PolicyAction::RemoveGovernor,
        PolicyAction::Dividend,
        PolicyAction::Split,
        PolicyAction::Merger,
        PolicyAction::Halt,
        PolicyAction::Resume,
        PolicyAction::ArbitrationEnforce,
    ];
    for pa in &actions {
        let json = serde_json::to_string(pa).expect("serialize");
        let recovered: PolicyAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*pa, recovered);
    }
}

#[test]
fn serde_rt_impact_level_all_variants() {
    let levels = [
        ImpactLevel::None,
        ImpactLevel::Low,
        ImpactLevel::Medium,
        ImpactLevel::High,
        ImpactLevel::Critical,
    ];
    for il in &levels {
        let json = serde_json::to_string(il).expect("serialize");
        let recovered: ImpactLevel = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*il, recovered);
    }
}

#[test]
fn serde_rt_authorization_requirement_all_variants() {
    let reqs = [
        AuthorizationRequirement::Automatic,
        AuthorizationRequirement::Quorum,
        AuthorizationRequirement::Unanimous,
        AuthorizationRequirement::Governance,
    ];
    for ar in &reqs {
        let json = serde_json::to_string(ar).expect("serialize");
        let recovered: AuthorizationRequirement = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*ar, recovered);
    }
}

#[test]
fn serde_rt_action_status_all_variants() {
    let statuses = [
        ActionStatus::Pending,
        ActionStatus::Executing,
        ActionStatus::Completed,
        ActionStatus::Failed,
        ActionStatus::Cancelled,
    ];
    for s in &statuses {
        let json = serde_json::to_string(s).expect("serialize");
        let recovered: ActionStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*s, recovered);
    }
}

#[test]
fn serde_rt_audit_entry_type_all_variants() {
    let types = [
        AuditEntryType::TriggerReceived,
        AuditEntryType::PolicyEvaluated,
        AuditEntryType::ActionScheduled,
        AuditEntryType::ActionExecuted,
        AuditEntryType::ActionFailed,
        AuditEntryType::ActionCancelled,
    ];
    for aet in &types {
        let json = serde_json::to_string(aet).expect("serialize");
        let recovered: AuditEntryType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*aet, recovered);
    }
}

#[test]
fn serde_rt_audit_entry() {
    // BUG-010: AuditEntry does not derive PartialEq
    let original = AuditEntry::new(
        AuditEntryType::TriggerReceived,
        Some("asset-001".to_string()),
        Some(json!({"trigger": "sanctions_list_update"})),
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AuditEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.entry_type, AuditEntryType::TriggerReceived);
    assert_eq!(recovered.asset_id, Some("asset-001".to_string()));
    assert!(recovered.metadata.is_some());
}

#[test]
fn serde_rt_audit_entry_with_none_fields() {
    let original = AuditEntry::new(AuditEntryType::ActionExecuted, None, None);
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AuditEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.entry_type, AuditEntryType::ActionExecuted);
    assert!(recovered.asset_id.is_none());
    assert!(recovered.metadata.is_none());
}

#[test]
fn serde_rt_condition_threshold() {
    let original = Condition::Threshold {
        field: "risk_score".to_string(),
        threshold: json!(0.8),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Condition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_condition_and_nested() {
    let original = Condition::And {
        conditions: vec![
            Condition::Equals {
                field: "status".to_string(),
                value: json!("active"),
            },
            Condition::GreaterThan {
                field: "balance".to_string(),
                threshold: json!(1000),
            },
        ],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Condition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

// =========================================================================
// msez-tensor serde round-trips
// =========================================================================

#[test]
fn serde_rt_compliance_state_all_variants() {
    let states = [
        ComplianceState::Compliant,
        ComplianceState::NonCompliant,
        ComplianceState::Pending,
        ComplianceState::Exempt,
        ComplianceState::NotApplicable,
    ];
    for cs in &states {
        let json = serde_json::to_string(cs).expect("serialize");
        let recovered: ComplianceState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*cs, recovered);
    }
}

#[test]
fn serde_rt_attestation_ref() {
    let original = AttestationRef {
        attestation_id: "att-001".to_string(),
        attestation_type: "kyc_verification".to_string(),
        issuer_did: "did:key:z6MkTest".to_string(),
        issued_at: "2026-01-15T12:00:00Z".to_string(),
        expires_at: Some("2027-01-15T12:00:00Z".to_string()),
        digest: test_digest().to_hex(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AttestationRef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_tensor_cell() {
    // BUG-011: TensorCell does not derive PartialEq — cannot verify
    // round-trip fidelity mechanically.
    // BUG-012: TensorCell.determined_at is a raw String instead of
    // Timestamp newtype — no validation, no normalization, no type safety.
    let original = TensorCell {
        state: ComplianceState::Compliant,
        attestations: vec![AttestationRef {
            attestation_id: "att-002".to_string(),
            attestation_type: "kyc_verification".to_string(),
            issuer_did: "did:key:z6MkKyc".to_string(),
            issued_at: "2026-01-15T12:00:00Z".to_string(),
            expires_at: None,
            digest: test_digest().to_hex(),
        }],
        determined_at: "2026-01-15T12:00:00Z".to_string(),
        reason: Some("All checks passed".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: TensorCell = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.state, ComplianceState::Compliant);
    assert_eq!(recovered.attestations.len(), 1);
    assert_eq!(recovered.determined_at, "2026-01-15T12:00:00Z");
    assert_eq!(recovered.reason, Some("All checks passed".to_string()));
}

#[test]
fn serde_rt_tensor_cell_skip_empty_attestations() {
    // Verify skip_serializing_if works for empty attestations
    let original = TensorCell {
        state: ComplianceState::NotApplicable,
        attestations: vec![],
        determined_at: "2026-01-15T12:00:00Z".to_string(),
        reason: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    // Empty attestations should be skipped
    assert!(
        !json.contains("\"attestations\""),
        "Empty attestations should be omitted: {json}"
    );
    // None reason should be skipped
    assert!(
        !json.contains("\"reason\""),
        "None reason should be omitted: {json}"
    );
    // Must deserialize correctly despite missing fields
    let recovered: TensorCell = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.attestations.is_empty());
    assert!(recovered.reason.is_none());
}

// =========================================================================
// BUG-013 through BUG-017 RESOLVED: Custom serde Deserialize impls
// now route through validated constructors. Invalid values are rejected
// at deserialization time.
// =========================================================================

#[test]
fn serde_rt_did_rejects_invalid() {
    // BUG-013 RESOLVED: Custom Deserialize validates via Did::new().
    let invalid_json = "\"not-a-did\"";
    let result: Result<msez_core::Did, _> = serde_json::from_str(invalid_json);
    assert!(
        result.is_err(),
        "invalid DID must be rejected at deserialization"
    );
}

#[test]
fn serde_rt_ntn_rejects_invalid() {
    // BUG-014 RESOLVED: Custom Deserialize validates via Ntn::new().
    let invalid_json = "\"12345\""; // Only 5 digits, should be 7
    let result: Result<msez_core::Ntn, _> = serde_json::from_str(invalid_json);
    assert!(
        result.is_err(),
        "invalid NTN must be rejected at deserialization"
    );
}

#[test]
fn serde_rt_cnic_rejects_invalid() {
    // BUG-015 RESOLVED: Custom Deserialize validates via Cnic::new().
    let invalid_json = "\"123\""; // Only 3 digits, should be 13
    let result: Result<msez_core::Cnic, _> = serde_json::from_str(invalid_json);
    assert!(
        result.is_err(),
        "invalid CNIC must be rejected at deserialization"
    );
}

#[test]
fn serde_rt_passport_number_rejects_invalid() {
    // BUG-016 RESOLVED: Custom Deserialize validates via PassportNumber::new().
    let invalid_json = "\"AB\""; // Only 2 chars, minimum is 5
    let result: Result<msez_core::PassportNumber, _> = serde_json::from_str(invalid_json);
    assert!(
        result.is_err(),
        "invalid passport must be rejected at deserialization"
    );
}

#[test]
fn serde_rt_jurisdiction_id_rejects_invalid() {
    // BUG-017 RESOLVED: Custom Deserialize validates via JurisdictionId::new().
    let invalid_json = "\"\""; // Empty string, should be rejected
    let result: Result<JurisdictionId, _> = serde_json::from_str(invalid_json);
    assert!(
        result.is_err(),
        "empty JurisdictionId must be rejected at deserialization"
    );
}

// =========================================================================
// msez-arbitration: Full struct round-trip tests
// =========================================================================

use msez_arbitration::dispute::{ArbitrationInstitution, Claim, Dispute, Money, Party as ArbParty};
use msez_arbitration::enforcement::{EnforcementAction, EnforcementOrder, EnforcementPrecondition};
use msez_arbitration::escrow::{EscrowAccount, EscrowTransaction, ReleaseCondition};
use msez_arbitration::evidence::{
    AuthenticityAttestation, AuthenticityType, ChainOfCustodyEntry, EvidenceType,
};
use msez_core::Did;

#[test]
fn serde_rt_money() {
    let original = Money::new("150000.50", "USD").unwrap();
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Money = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.amount, "150000.50");
    assert_eq!(recovered.currency, "USD");
}

#[test]
fn serde_rt_arb_party() {
    let original = ArbParty {
        did: Did::new("did:key:z6MkTest123").unwrap(),
        legal_name: "Test Corp".to_string(),
        jurisdiction_id: Some(JurisdictionId::new("PK-RSEZ").unwrap()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ArbParty = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.legal_name, "Test Corp");
    assert!(recovered.jurisdiction_id.is_some());
}

#[test]
fn serde_rt_claim() {
    let original = Claim {
        claim_id: "CLM-001".to_string(),
        claim_type: DisputeType::BreachOfContract,
        description: "Failed to deliver goods".to_string(),
        amount: Some(Money::new("50000", "USD").unwrap()),
        supporting_evidence_digests: vec![test_digest()],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Claim = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.claim_id, "CLM-001");
    assert_eq!(recovered.claim_type, DisputeType::BreachOfContract);
    assert!(recovered.amount.is_some());
}

#[test]
fn serde_rt_claim_without_amount() {
    let original = Claim {
        claim_id: "CLM-002".to_string(),
        claim_type: DisputeType::ForceMajeure,
        description: "Force majeure event".to_string(),
        amount: None,
        supporting_evidence_digests: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Claim = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.amount.is_none());
    assert!(recovered.supporting_evidence_digests.is_empty());
}

#[test]
fn serde_rt_evidence_type_all_variants() {
    let types = [
        EvidenceType::SmartAssetReceipt,
        EvidenceType::CorridorReceipt,
        EvidenceType::ComplianceEvidence,
        EvidenceType::ExpertReport,
        EvidenceType::WitnessStatement,
        EvidenceType::ContractDocument,
        EvidenceType::CommunicationRecord,
        EvidenceType::PaymentRecord,
        EvidenceType::ShippingDocument,
        EvidenceType::InspectionReport,
    ];
    for et in &types {
        let json = serde_json::to_string(et).expect("serialize");
        let recovered: EvidenceType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*et, recovered);
    }
}

#[test]
fn serde_rt_authenticity_type_all_variants() {
    let types = [
        AuthenticityType::CorridorCheckpointInclusion,
        AuthenticityType::SmartAssetCheckpointInclusion,
        AuthenticityType::NotarizedDocument,
        AuthenticityType::ExpertCertification,
        AuthenticityType::ChainOfCustody,
    ];
    for at in &types {
        let json = serde_json::to_string(at).expect("serialize");
        let recovered: AuthenticityType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*at, recovered);
    }
}

#[test]
fn serde_rt_escrow_account_full() {
    let original = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::SecurityDeposit,
        "USD".to_string(),
        None,
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EscrowAccount = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.currency, "USD");
    assert_eq!(recovered.status, EscrowStatus::Pending);
}

#[test]
fn serde_rt_enforcement_status_round_trip() {
    for status in [
        EnforcementStatus::Pending,
        EnforcementStatus::InProgress,
        EnforcementStatus::Completed,
        EnforcementStatus::Blocked,
        EnforcementStatus::Cancelled,
    ] {
        let json = serde_json::to_string(&status).expect("serialize");
        let recovered: EnforcementStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            status, recovered,
            "EnforcementStatus round-trip failed for {:?}",
            status
        );
    }
}

#[test]
fn serde_rt_enforcement_action_escrow_release() {
    let original = EnforcementAction::EscrowRelease {
        escrow_id: EscrowId::new(),
        beneficiary: Did::new("did:key:z6MkBeneficiary").unwrap(),
        amount: Some("50000".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EnforcementAction = serde_json::from_str(&json).expect("deserialize");
    match recovered {
        EnforcementAction::EscrowRelease { amount, .. } => {
            assert_eq!(amount, Some("50000".to_string()));
        }
        _ => panic!("Expected EscrowRelease variant"),
    }
}

#[test]
fn serde_rt_enforcement_action_all_variants() {
    let actions = vec![
        EnforcementAction::EscrowRelease {
            escrow_id: EscrowId::new(),
            beneficiary: Did::new("did:key:z6MkTest").unwrap(),
            amount: None,
        },
        EnforcementAction::LicenseSuspension {
            license_id: "LIC-001".to_string(),
            reason: "Violation".to_string(),
        },
        EnforcementAction::CorridorSuspension {
            corridor_id: CorridorId::new(),
            reason: "Sanctions".to_string(),
        },
        EnforcementAction::CorridorReceiptGeneration {
            corridor_id: CorridorId::new(),
        },
        EnforcementAction::AssetTransfer {
            asset_digest: test_digest(),
            recipient: Did::new("did:key:z6MkRecipient").unwrap(),
        },
        EnforcementAction::MonetaryPenalty {
            party: Did::new("did:key:z6MkParty").unwrap(),
            amount: "10000".to_string(),
            currency: "PKR".to_string(),
        },
    ];
    for action in &actions {
        let json = serde_json::to_string(action).expect("serialize");
        let _recovered: EnforcementAction = serde_json::from_str(&json).expect("deserialize");
    }
}

// =========================================================================
// msez-agentic: Condition variant exhaustive round-trips
// =========================================================================

#[test]
fn serde_rt_condition_all_variants() {
    let conditions = vec![
        Condition::Threshold {
            field: "score".to_string(),
            threshold: json!(0.9),
        },
        Condition::Equals {
            field: "status".to_string(),
            value: json!("active"),
        },
        Condition::NotEquals {
            field: "status".to_string(),
            value: json!("inactive"),
        },
        Condition::Contains {
            field: "parties".to_string(),
            item: json!("self"),
        },
        Condition::In {
            field: "jurisdiction".to_string(),
            values: vec![json!("PK"), json!("AE"), json!("SG")],
        },
        Condition::LessThan {
            field: "amount".to_string(),
            threshold: json!(1000),
        },
        Condition::GreaterThan {
            field: "amount".to_string(),
            threshold: json!(0),
        },
        Condition::Exists {
            field: "certification".to_string(),
        },
        Condition::And {
            conditions: vec![
                Condition::Equals {
                    field: "a".to_string(),
                    value: json!(1),
                },
                Condition::Equals {
                    field: "b".to_string(),
                    value: json!(2),
                },
            ],
        },
        Condition::Or {
            conditions: vec![
                Condition::Exists {
                    field: "x".to_string(),
                },
                Condition::Exists {
                    field: "y".to_string(),
                },
            ],
        },
    ];
    for (i, cond) in conditions.iter().enumerate() {
        let json = serde_json::to_string(cond).expect("serialize");
        let recovered: Condition = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            *cond, recovered,
            "Condition variant {} failed round-trip",
            i
        );
    }
}

#[test]
fn serde_rt_condition_deeply_nested() {
    let deep = Condition::And {
        conditions: vec![Condition::Or {
            conditions: vec![Condition::And {
                conditions: vec![Condition::Equals {
                    field: "deep".to_string(),
                    value: json!("value"),
                }],
            }],
        }],
    };
    let json = serde_json::to_string(&deep).expect("serialize");
    let recovered: Condition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deep, recovered, "Deeply nested Condition failed round-trip");
}

// =========================================================================
// msez-agentic: ScheduledAction and CronSchedule round-trips
// =========================================================================

use msez_agentic::scheduler::{CronSchedule, SchedulePattern, ScheduledAction};

#[test]
fn serde_rt_schedule_pattern_all_variants() {
    let patterns = [
        SchedulePattern::Hourly,
        SchedulePattern::Daily,
        SchedulePattern::Weekly,
        SchedulePattern::Monthly,
        SchedulePattern::Yearly,
    ];
    for sp in &patterns {
        let json = serde_json::to_string(sp).expect("serialize");
        let recovered: SchedulePattern = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*sp, recovered);
    }
}

#[test]
fn serde_rt_scheduled_action() {
    let original = ScheduledAction::new(
        "asset:001".to_string(),
        PolicyAction::Halt,
        "policy-sanctions-001".to_string(),
        AuthorizationRequirement::Automatic,
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ScheduledAction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.action, PolicyAction::Halt);
    assert_eq!(recovered.policy_id, "policy-sanctions-001");
    assert_eq!(recovered.status, ActionStatus::Pending);
}

#[test]
fn serde_rt_cron_schedule() {
    let original = CronSchedule::new("sched-001", "Daily sanctions check", SchedulePattern::Daily);
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: CronSchedule = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.schedule_id, "sched-001");
    assert_eq!(recovered.pattern, SchedulePattern::Daily);
    assert!(recovered.active);
}

// =========================================================================
// msez-corridor: Anchor and Fork types round-trips
// =========================================================================

use msez_corridor::anchor::{AnchorCommitment, AnchorStatus};
use msez_corridor::fork::ResolutionReason;

#[test]
fn serde_rt_anchor_status_all_variants() {
    let statuses = [
        AnchorStatus::Pending,
        AnchorStatus::Confirmed,
        AnchorStatus::Finalized,
        AnchorStatus::Failed,
    ];
    for s in &statuses {
        let json = serde_json::to_string(s).expect("serialize");
        let recovered: AnchorStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*s, recovered);
    }
}

#[test]
fn serde_rt_resolution_reason_all_variants() {
    let reasons = [
        ResolutionReason::EarlierTimestamp,
        ResolutionReason::MoreAttestations,
        ResolutionReason::LexicographicTiebreak,
    ];
    for r in &reasons {
        let json = serde_json::to_string(r).expect("serialize");
        let recovered: ResolutionReason = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*r, recovered);
    }
}

#[test]
fn serde_rt_anchor_commitment() {
    let original = AnchorCommitment {
        checkpoint_digest: test_digest(),
        chain_id: Some("ethereum-mainnet".to_string()),
        checkpoint_height: 42,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let _recovered: AnchorCommitment = serde_json::from_str(&json).expect("deserialize");
}

// =========================================================================
// msez-corridor: CorridorReceipt and Checkpoint round-trips
// =========================================================================

use msez_corridor::receipt::CorridorReceipt;

#[test]
fn serde_rt_corridor_receipt() {
    let original = CorridorReceipt {
        receipt_type: "state_transition".to_string(),
        corridor_id: CorridorId::new(),
        sequence: 1,
        timestamp: Timestamp::now(),
        prev_root: "".to_string(),
        next_root: "aa".repeat(32),
        lawpack_digest_set: vec!["bb".repeat(32)],
        ruleset_digest_set: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let _recovered: CorridorReceipt = serde_json::from_str(&json).expect("deserialize");
}

// =========================================================================
// msez-state: DissolutionStage round-trips
// =========================================================================

use msez_state::DissolutionStage;

#[test]
fn serde_rt_dissolution_stage_all_variants() {
    for stage in DissolutionStage::all_stages() {
        let json = serde_json::to_string(stage).expect("serialize");
        let recovered: DissolutionStage = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            *stage, recovered,
            "DissolutionStage {:?} failed round-trip",
            stage
        );
    }
}

// =========================================================================
// Netting engine i64 overflow — financial calculation
// =========================================================================

#[test]
fn serde_rt_settlement_plan_large_amounts() {
    // BUG-018 RESOLVED: gross_total now uses checked arithmetic.
    // Overflow returns NettingError::ArithmeticOverflow instead of
    // panicking or silently wrapping.
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
    // With checked arithmetic, this returns an error instead of overflowing.
    let result = engine.compute_plan();
    assert!(
        result.is_err(),
        "BUG-018 RESOLVED: overflow must return ArithmeticOverflow error"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("arithmetic overflow"),
        "error should mention arithmetic overflow, got: {err_msg}"
    );
}

// =========================================================================
// msez-pack: Lawpack types serde round-trips
// =========================================================================

use msez_pack::lawpack::{
    Lawpack, LawpackLock, LawpackLockComponents, LawpackLockProvenance, LawpackManifest,
    LawpackRef, LawpackSource, NormalizationInfo, NormalizationInput,
};
use std::collections::BTreeMap;

#[test]
fn serde_rt_lawpack_ref() {
    let original = LawpackRef {
        jurisdiction_id: "PAK".to_string(),
        domain: "corporate".to_string(),
        lawpack_digest_sha256: "aa".repeat(32),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LawpackRef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_lawpack_ref_empty_fields() {
    // BUG-033 RESOLVED: LawpackRef custom Deserialize now rejects empty fields.
    let json = r#"{"jurisdiction_id":"","domain":"","lawpack_digest_sha256":""}"#;
    let result: Result<LawpackRef, _> = serde_json::from_str(json);
    // BUG-033 RESOLVED: deserialization correctly fails for empty required fields.
    assert!(
        result.is_err(),
        "BUG-033 RESOLVED: LawpackRef rejects empty fields via serde"
    );
}

#[test]
fn serde_rt_lawpack_source() {
    let original = LawpackSource {
        source_id: "fbr-income-tax-2001".to_string(),
        uri: Some("https://fbr.gov.pk/laws/income-tax-ordinance-2001".to_string()),
        reference: Some("Income Tax Ordinance, 2001 (XLIX of 2001)".to_string()),
        retrieved_at: Some("2026-01-15".to_string()),
        sha256: Some("bb".repeat(32)),
        media_type: Some("text/html".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LawpackSource = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.source_id, original.source_id);
    assert_eq!(recovered.uri, original.uri);
    assert_eq!(recovered.sha256, original.sha256);
}

#[test]
fn serde_rt_normalization_info() {
    let original = NormalizationInfo {
        recipe_id: "akn-pak-v1".to_string(),
        tool: "msez-normalize".to_string(),
        tool_version: "0.4.44".to_string(),
        inputs: vec![NormalizationInput {
            module_id: "income-tax".to_string(),
            module_version: "2001.1".to_string(),
            sources_manifest_sha256: "cc".repeat(32),
        }],
        notes: "Normalized from FBR HTML".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: NormalizationInfo = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.recipe_id, original.recipe_id);
    assert_eq!(recovered.tool, original.tool);
    assert_eq!(recovered.inputs.len(), 1);
}

#[test]
fn serde_rt_lawpack_manifest() {
    let original = LawpackManifest {
        lawpack_format_version: "1.0".to_string(),
        jurisdiction_id: "PAK".to_string(),
        domain: "corporate".to_string(),
        as_of_date: "2026-01-15".to_string(),
        sources: vec![json!({"source_id": "src-1", "uri": "https://example.com"})],
        license: "CC-BY-4.0".to_string(),
        normalization: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LawpackManifest = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.lawpack_format_version, "1.0");
    assert_eq!(recovered.jurisdiction_id, "PAK");
    assert_eq!(recovered.sources.len(), 1);
}

#[test]
fn serde_rt_lawpack_lock_components() {
    let mut akn_sha256 = BTreeMap::new();
    akn_sha256.insert("income-tax.akn".to_string(), "ee".repeat(32));
    akn_sha256.insert("sales-tax.akn".to_string(), "ff".repeat(32));

    let original = LawpackLockComponents {
        lawpack_yaml_sha256: "aa".repeat(32),
        index_json_sha256: "bb".repeat(32),
        akn_sha256,
        sources_sha256: "cc".repeat(32),
        module_manifest_sha256: Some("dd".repeat(32)),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LawpackLockComponents = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.lawpack_yaml_sha256, original.lawpack_yaml_sha256);
    assert_eq!(recovered.akn_sha256.len(), 2);
}

#[test]
fn serde_rt_lawpack_lock() {
    let mut akn_sha256 = BTreeMap::new();
    akn_sha256.insert("mod.akn".to_string(), "ee".repeat(32));

    let original = LawpackLock {
        lawpack_digest_sha256: "aa".repeat(32),
        jurisdiction_id: "PAK".to_string(),
        domain: "corporate".to_string(),
        as_of_date: "2026-01-15".to_string(),
        artifact_path: "packs/PAK/corporate/".to_string(),
        artifact_sha256: "bb".repeat(32),
        components: LawpackLockComponents {
            lawpack_yaml_sha256: "cc".repeat(32),
            index_json_sha256: "dd".repeat(32),
            akn_sha256,
            sources_sha256: "ee".repeat(32),
            module_manifest_sha256: None,
        },
        provenance: LawpackLockProvenance {
            module_manifest_path: "manifest.yaml".to_string(),
            sources_manifest_path: "sources.yaml".to_string(),
            raw_sources: BTreeMap::new(),
            normalization: None,
        },
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LawpackLock = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        recovered.lawpack_digest_sha256,
        original.lawpack_digest_sha256
    );
    assert_eq!(recovered.jurisdiction_id, "PAK");
}

#[test]
fn serde_rt_lawpack_full() {
    let jid = JurisdictionId::new("PAK").unwrap();
    let mut section_mappings = BTreeMap::new();
    section_mappings.insert("s.1".to_string(), "rule-001".to_string());

    let original = Lawpack {
        jurisdiction: jid,
        name: "Pakistan Corporate Law".to_string(),
        domain: "corporate".to_string(),
        version: "2026.1".to_string(),
        digest: Some(test_digest()),
        as_of_date: Some("2026-01-15".to_string()),
        effective_date: Some("2001-07-01".to_string()),
        section_mappings,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Lawpack = serde_json::from_str(&json).expect("deserialize");
    // Lawpack does not derive PartialEq
    assert_eq!(recovered.name, original.name);
    assert_eq!(recovered.domain, original.domain);
    assert!(recovered.digest.is_some());
    assert_eq!(recovered.section_mappings.len(), 1);
}

#[test]
fn serde_rt_lawpack_skip_serializing_if_empty() {
    // Lawpack has skip_serializing_if for optional/empty fields.
    // Verify omitted fields deserialize correctly via #[serde(default)].
    let jid = JurisdictionId::new("AE").unwrap();
    let original = Lawpack {
        jurisdiction: jid,
        name: "UAE Free Zone".to_string(),
        domain: "financial".to_string(),
        version: "1.0".to_string(),
        digest: None,
        as_of_date: None,
        effective_date: None,
        section_mappings: BTreeMap::new(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    // The JSON should NOT contain "digest", "as_of_date", "effective_date", "section_mappings"
    assert!(!json.contains("digest"), "None digest should be omitted");
    assert!(
        !json.contains("section_mappings"),
        "Empty section_mappings should be omitted"
    );

    let recovered: Lawpack = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.digest.is_none());
    assert!(recovered.section_mappings.is_empty());
}

// =========================================================================
// msez-pack: Regpack types serde round-trips
// =========================================================================

use msez_pack::regpack::{
    ComplianceDeadline, RegLicenseType, RegPackMetadata, Regpack, RegpackRef, RegulatorProfile,
    ReportingRequirement, SanctionsCheckResult, SanctionsEntry, SanctionsMatch, SanctionsSnapshot,
};

#[test]
fn serde_rt_regpack_ref() {
    let original = RegpackRef {
        jurisdiction_id: "PAK".to_string(),
        domain: "financial".to_string(),
        regpack_digest_sha256: "aa".repeat(32),
        as_of_date: Some("2026-01-15".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: RegpackRef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_sanctions_entry() {
    let mut alias = BTreeMap::new();
    alias.insert("name".to_string(), "Alias One".to_string());
    let original = SanctionsEntry {
        entry_id: "SDN-12345".to_string(),
        entry_type: "individual".to_string(),
        source_lists: vec!["OFAC-SDN".to_string(), "UN-1267".to_string()],
        primary_name: "Test Person".to_string(),
        aliases: vec![alias],
        identifiers: vec![],
        addresses: vec![],
        nationalities: vec!["PK".to_string()],
        date_of_birth: Some("1980-01-01".to_string()),
        programs: vec!["SDGT".to_string()],
        listing_date: Some("2020-06-15".to_string()),
        remarks: Some("Test entry".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SanctionsEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.entry_id, "SDN-12345");
    assert_eq!(recovered.source_lists.len(), 2);
    assert_eq!(recovered.aliases.len(), 1);
}

#[test]
fn serde_rt_sanctions_snapshot() {
    let original = SanctionsSnapshot {
        snapshot_id: "snap-001".to_string(),
        snapshot_timestamp: "2026-01-15T12:00:00Z".to_string(),
        sources: BTreeMap::new(),
        consolidated_counts: BTreeMap::new(),
        delta_from_previous: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SanctionsSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.snapshot_id, "snap-001");
}

#[test]
fn serde_rt_sanctions_check_result() {
    let original = SanctionsCheckResult {
        query: "Test Person".to_string(),
        checked_at: "2026-01-15T12:00:00Z".to_string(),
        snapshot_id: "snap-001".to_string(),
        matched: false,
        matches: vec![],
        match_score: 0.0,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SanctionsCheckResult = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.query, "Test Person");
    assert!(!recovered.matched);
}

#[test]
fn serde_rt_sanctions_check_result_with_match() {
    let entry = SanctionsEntry {
        entry_id: "SDN-99999".to_string(),
        entry_type: "individual".to_string(),
        source_lists: vec!["OFAC-SDN".to_string()],
        primary_name: "Matched Person".to_string(),
        aliases: vec![],
        identifiers: vec![],
        addresses: vec![],
        nationalities: vec![],
        date_of_birth: None,
        programs: vec![],
        listing_date: None,
        remarks: None,
    };
    let original = SanctionsCheckResult {
        query: "Matched Person".to_string(),
        checked_at: "2026-01-15T12:00:00Z".to_string(),
        snapshot_id: "snap-002".to_string(),
        matched: true,
        matches: vec![SanctionsMatch {
            entry,
            match_type: "exact".to_string(),
            score: 1.0,
            identifier_type: Some("name".to_string()),
        }],
        match_score: 1.0,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SanctionsCheckResult = serde_json::from_str(&json).expect("deserialize");
    assert!(recovered.matched);
    assert_eq!(recovered.matches.len(), 1);
    assert_eq!(recovered.matches[0].entry.entry_id, "SDN-99999");
}

#[test]
fn serde_rt_regulator_profile() {
    let original = RegulatorProfile {
        regulator_id: "FBR".to_string(),
        name: "Federal Board of Revenue".to_string(),
        jurisdiction_id: "PAK".to_string(),
        parent_authority: Some("Ministry of Finance".to_string()),
        scope: {
            let mut m = BTreeMap::new();
            m.insert(
                "taxation".to_string(),
                vec!["income_tax".to_string(), "sales_tax".to_string()],
            );
            m
        },
        contact: {
            let mut m = BTreeMap::new();
            m.insert("email".to_string(), "info@fbr.gov.pk".to_string());
            m
        },
        api_capabilities: BTreeMap::new(),
        timezone: "Asia/Karachi".to_string(),
        business_days: vec!["Mon".to_string(), "Tue".to_string(), "Wed".to_string()],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.regulator_id, "FBR");
    assert_eq!(recovered.scope.len(), 1);
    assert_eq!(recovered.scope["taxation"].len(), 2);
}

#[test]
fn serde_rt_reg_license_type() {
    let original = RegLicenseType {
        license_type_id: "SECP-NBF".to_string(),
        name: "Non-Banking Finance Company License".to_string(),
        regulator_id: "SECP".to_string(),
        requirements: {
            let mut m = BTreeMap::new();
            m.insert("min_capital".to_string(), json!("200000000"));
            m
        },
        application: BTreeMap::new(),
        ongoing_obligations: {
            let mut m = BTreeMap::new();
            m.insert("quarterly_returns".to_string(), json!(true));
            m
        },
        validity_period_years: 5,
        renewal_lead_time_days: 90,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: RegLicenseType = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.license_type_id, "SECP-NBF");
    assert_eq!(recovered.validity_period_years, 5);
}

#[test]
fn serde_rt_reporting_requirement() {
    let original = ReportingRequirement {
        report_type_id: "FBR-WHT".to_string(),
        name: "Withholding Tax Return".to_string(),
        regulator_id: "FBR".to_string(),
        applicable_to: vec!["all_entities".to_string()],
        frequency: "monthly".to_string(),
        deadlines: BTreeMap::new(),
        submission: BTreeMap::new(),
        late_penalty: BTreeMap::new(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ReportingRequirement = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.report_type_id, "FBR-WHT");
}

#[test]
fn serde_rt_compliance_deadline() {
    let original = ComplianceDeadline {
        deadline_id: "FBR-ANNUAL-2026".to_string(),
        regulator_id: "FBR".to_string(),
        deadline_type: "annual_return".to_string(),
        description: "Annual income tax return filing".to_string(),
        due_date: "2026-09-30".to_string(),
        grace_period_days: 30,
        applicable_license_types: vec!["SECP-COMPANY".to_string()],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ComplianceDeadline = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.deadline_id, "FBR-ANNUAL-2026");
    assert_eq!(recovered.grace_period_days, 30);
}

#[test]
fn serde_rt_regpack_full() {
    let jid = JurisdictionId::new("PAK").unwrap();
    let original = Regpack {
        jurisdiction: jid,
        name: "Pakistan Financial Regulatory Pack".to_string(),
        version: "2026.1".to_string(),
        digest: Some(test_digest()),
        metadata: Some(RegPackMetadata {
            regpack_id: "pak-fin-2026".to_string(),
            jurisdiction_id: "PAK".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            snapshot_type: "full".to_string(),
            sources: vec![json!("SBP"), json!("SECP")],
            includes: BTreeMap::new(),
            previous_regpack_digest: None,
            created_at: Some("2026-01-15T00:00:00Z".to_string()),
            expires_at: Some("2026-06-30T00:00:00Z".to_string()),
            digest_sha256: Some("ff".repeat(32)),
        }),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Regpack = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.name, "Pakistan Financial Regulatory Pack");
    assert!(recovered.metadata.is_some());
}

// =========================================================================
// msez-pack: Licensepack types serde round-trips
// =========================================================================

use msez_pack::licensepack::{
    License, LicenseComplianceState, LicenseCondition, LicenseDomain, LicenseHolder,
    LicensePermission, LicenseRestriction, LicenseStatus, LicenseTypeDefinition, Licensepack,
    LicensepackArtifactInfo, LicensepackLock, LicensepackLockInfo, LicensepackMetadata,
    LicensepackRef, LicensepackRegulator,
};

#[test]
fn serde_rt_license_status_all_variants() {
    let variants = [
        LicenseStatus::Active,
        LicenseStatus::Suspended,
        LicenseStatus::Revoked,
        LicenseStatus::Expired,
        LicenseStatus::Pending,
        LicenseStatus::Surrendered,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: LicenseStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered, "LicenseStatus {:?} failed round-trip", v);
    }
}

#[test]
fn serde_rt_license_domain_all_variants() {
    let variants = [
        LicenseDomain::Financial,
        LicenseDomain::Corporate,
        LicenseDomain::Professional,
        LicenseDomain::Trade,
        LicenseDomain::Insurance,
        LicenseDomain::Mixed,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: LicenseDomain = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered, "LicenseDomain {:?} failed round-trip", v);
    }
}

#[test]
fn serde_rt_license_compliance_state_all_variants() {
    let variants = [
        LicenseComplianceState::Compliant,
        LicenseComplianceState::NonCompliant,
        LicenseComplianceState::Pending,
        LicenseComplianceState::Suspended,
        LicenseComplianceState::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: LicenseComplianceState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered, "LicenseComplianceState {:?} round-trip", v);
    }
}

#[test]
fn serde_rt_license_condition() {
    let original = LicenseCondition {
        condition_id: "cond-001".to_string(),
        condition_type: "capital_requirement".to_string(),
        description: "Minimum paid-up capital".to_string(),
        metric: Some("paid_up_capital".to_string()),
        threshold: Some("200000000".to_string()),
        currency: Some("PKR".to_string()),
        operator: Some(">=".to_string()),
        frequency: Some("continuous".to_string()),
        reporting_frequency: Some("quarterly".to_string()),
        effective_date: Some("2026-01-01".to_string()),
        expiry_date: None,
        status: "active".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicenseCondition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.condition_id, "cond-001");
    assert_eq!(recovered.status, "active");
}

#[test]
fn serde_rt_license_permission() {
    let original = LicensePermission {
        permission_id: "perm-001".to_string(),
        activity: "accept_deposits".to_string(),
        scope: {
            let mut m = BTreeMap::new();
            m.insert("client_type".to_string(), json!("corporate"));
            m
        },
        limits: {
            let mut m = BTreeMap::new();
            m.insert("max_deposit".to_string(), json!("1000000000"));
            m
        },
        effective_date: Some("2026-01-01".to_string()),
        status: "active".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicensePermission = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.permission_id, "perm-001");
    assert_eq!(recovered.scope.len(), 1);
}

#[test]
fn serde_rt_license_restriction() {
    let original = LicenseRestriction {
        restriction_id: "rest-001".to_string(),
        restriction_type: "geographical".to_string(),
        description: "Cannot operate in sanctioned jurisdictions".to_string(),
        blocked_jurisdictions: vec!["KP".to_string(), "IR".to_string()],
        allowed_jurisdictions: vec![],
        blocked_activities: vec![],
        blocked_products: vec![],
        blocked_client_types: vec![],
        max_leverage: None,
        effective_date: Some("2026-01-01".to_string()),
        status: "active".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicenseRestriction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.restriction_id, "rest-001");
    assert_eq!(recovered.blocked_jurisdictions.len(), 2);
}

#[test]
fn serde_rt_license_holder() {
    let original = LicenseHolder {
        holder_id: "holder-001".to_string(),
        entity_type: "company".to_string(),
        legal_name: "Test Financial Services Pvt Ltd".to_string(),
        trading_names: vec!["Test Finance".to_string()],
        registration_number: Some("0123456".to_string()),
        incorporation_date: Some("2020-01-01".to_string()),
        jurisdiction_of_incorporation: Some("PAK".to_string()),
        did: None,
        registered_address: BTreeMap::new(),
        contact: BTreeMap::new(),
        controllers: vec![],
        beneficial_owners: vec![],
        group_structure: BTreeMap::new(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicenseHolder = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.holder_id, "holder-001");
    assert_eq!(recovered.trading_names.len(), 1);
}

#[test]
fn serde_rt_license_full() {
    let original = License {
        license_id: "lic-SECP-001".to_string(),
        license_type_id: "SECP-NBF".to_string(),
        license_number: Some("NBF-2024-001".to_string()),
        status: LicenseStatus::Active,
        issued_date: "2024-01-15".to_string(),
        holder_id: "holder-001".to_string(),
        holder_legal_name: "Test Corp".to_string(),
        regulator_id: "SECP".to_string(),
        status_effective_date: None,
        status_reason: None,
        effective_date: None,
        expiry_date: Some("2029-01-15".to_string()),
        holder_registration_number: None,
        holder_did: None,
        issuing_authority: None,
        permitted_activities: vec!["deposit_taking".to_string()],
        asset_classes_authorized: vec![],
        client_types_permitted: vec![],
        geographic_scope: vec![],
        prudential_category: None,
        capital_requirement: BTreeMap::new(),
        conditions: vec![],
        permissions: vec![],
        restrictions: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: License = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.license_id, "lic-SECP-001");
    assert_eq!(recovered.status, LicenseStatus::Active);
}

#[test]
fn serde_rt_license_type_definition() {
    let original = LicenseTypeDefinition {
        license_type_id: "SECP-NBF".to_string(),
        name: "Non-Banking Finance Company".to_string(),
        description: "License for non-banking financial companies".to_string(),
        regulator_id: "SECP".to_string(),
        category: Some("financial".to_string()),
        permitted_activities: vec!["lending".to_string()],
        requirements: BTreeMap::new(),
        application_fee: BTreeMap::new(),
        annual_fee: BTreeMap::new(),
        validity_period_years: Some(5),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicenseTypeDefinition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.license_type_id, "SECP-NBF");
}

#[test]
fn serde_rt_licensepack_regulator() {
    let original = LicensepackRegulator {
        regulator_id: "SECP".to_string(),
        name: "Securities and Exchange Commission of Pakistan".to_string(),
        jurisdiction_id: "PAK".to_string(),
        registry_url: Some("https://secp.gov.pk".to_string()),
        did: None,
        api_capabilities: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicensepackRegulator = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.regulator_id, "SECP");
}

#[test]
fn serde_rt_licensepack_metadata() {
    let regulator = LicensepackRegulator {
        regulator_id: "SECP".to_string(),
        name: "SECP".to_string(),
        jurisdiction_id: "PAK".to_string(),
        registry_url: None,
        did: None,
        api_capabilities: vec![],
    };
    let original = LicensepackMetadata {
        licensepack_id: "lp-pak-fin-2026".to_string(),
        jurisdiction_id: "PAK".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        snapshot_type: "full".to_string(),
        regulator,
        license: "CC-BY-4.0".to_string(),
        sources: vec![],
        includes: BTreeMap::new(),
        normalization: BTreeMap::new(),
        previous_licensepack_digest: None,
        delta: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicensepackMetadata = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.licensepack_id, "lp-pak-fin-2026");
}

#[test]
fn serde_rt_licensepack_full() {
    let jid = JurisdictionId::new("PAK").unwrap();
    let lp = Licensepack::new(jid, "Pakistan License Registry".to_string());
    let json = serde_json::to_string(&lp).expect("serialize");
    let recovered: Licensepack = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.name, "Pakistan License Registry");
}

#[test]
fn serde_rt_licensepack_ref() {
    let original = LicensepackRef {
        jurisdiction_id: "PAK".to_string(),
        domain: "financial".to_string(),
        licensepack_digest_sha256: "bb".repeat(32),
        as_of_date: Some("2026-01-15".to_string()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicensepackRef = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_licensepack_lock() {
    let original = LicensepackLock {
        lock_version: "1.0".to_string(),
        generated_at: "2026-01-15T00:00:00Z".to_string(),
        generator: "msez-cli".to_string(),
        generator_version: "0.4.44".to_string(),
        licensepack: LicensepackLockInfo {
            licensepack_id: "lp-001".to_string(),
            jurisdiction_id: "PAK".to_string(),
            domain: "financial".to_string(),
            as_of_date: "2026-01-15".to_string(),
            digest_sha256: "aa".repeat(32),
        },
        artifact: LicensepackArtifactInfo {
            artifact_type: "licensepack-snapshot".to_string(),
            digest_sha256: "bb".repeat(32),
            uri: "packs/PAK/licenses/snapshot.json".to_string(),
            media_type: "application/json".to_string(),
            byte_length: 4096,
        },
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: LicensepackLock = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.lock_version, "1.0");
}

// =========================================================================
// msez-mass-client: Mass DTO serde round-trips
// =========================================================================

use msez_mass_client::consent::{MassConsent, MassConsentOperationType, MassConsentStatus};
use msez_mass_client::entities::{MassEntity, MassEntityStatus, MassEntityType};
use msez_mass_client::fiscal::{MassFiscalAccount, MassPayment, MassPaymentStatus, MassTaxEvent};
use msez_mass_client::identity::{
    MassIdentity, MassIdentityStatus, MassIdentityType, MassMember, MassShareholder,
};
use msez_mass_client::ownership::{MassCapTable, MassOwnershipTransfer, MassShareClass};

#[test]
fn serde_rt_mass_entity_type_all_variants() {
    let variants = [
        MassEntityType::Llc,
        MassEntityType::Corporation,
        MassEntityType::Company,
        MassEntityType::Partnership,
        MassEntityType::SoleProprietor,
        MassEntityType::Trust,
        MassEntityType::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassEntityType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered, "MassEntityType {:?} round-trip failed", v);
    }
}

#[test]
fn serde_rt_mass_entity_status_all_variants() {
    let variants = [
        MassEntityStatus::Active,
        MassEntityStatus::Inactive,
        MassEntityStatus::Suspended,
        MassEntityStatus::Dissolved,
        MassEntityStatus::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassEntityStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered, "MassEntityStatus {:?} round-trip failed", v);
    }
}

#[test]
fn serde_rt_mass_entity_full() {
    let original = MassEntity {
        id: msez_core::EntityId::from(uuid::Uuid::new_v4()),
        name: "Momentum Technologies Pvt Ltd".to_string(),
        jurisdiction: Some("PAK-RSEZ".to_string()),
        status: Some(MassEntityStatus::Active),
        address: None,
        tags: vec!["sez".to_string()],
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
        board: None,
        members: None,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassEntity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.name, original.name);
    assert_eq!(recovered.jurisdiction, Some("PAK-RSEZ".to_string()));
}

#[test]
fn serde_rt_mass_consent_operation_type_all_variants() {
    let variants = [
        MassConsentOperationType::EquityOffer,
        MassConsentOperationType::IssueNewShares,
        MassConsentOperationType::AmendOptionsPool,
        MassConsentOperationType::CreateOptionsPool,
        MassConsentOperationType::CreateCommonClass,
        MassConsentOperationType::ModifyCompanyLegalName,
        MassConsentOperationType::ModifyBoardMemberDesignation,
        MassConsentOperationType::CertificateOfAmendment,
        MassConsentOperationType::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassConsentOperationType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered);
    }
}

#[test]
fn serde_rt_mass_consent_full() {
    let original = MassConsent {
        id: uuid::Uuid::new_v4(),
        organization_id: "org-001".to_string(),
        operation_id: Some(uuid::Uuid::new_v4()),
        operation_type: Some(MassConsentOperationType::EquityOffer),
        status: Some(MassConsentStatus::Approved),
        votes: vec![],
        num_votes_required: Some(1),
        approval_count: Some(1),
        rejection_count: Some(0),
        document_url: None,
        signatory: Some("director-001".to_string()),
        jurisdiction: Some("PAK-RSEZ".to_string()),
        requested_by: Some("admin".to_string()),
        expires_at: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassConsent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        recovered.operation_type,
        Some(MassConsentOperationType::EquityOffer)
    );
    assert_eq!(recovered.status, Some(MassConsentStatus::Approved));
}

#[test]
fn serde_rt_mass_fiscal_account() {
    let original = MassFiscalAccount {
        id: uuid::Uuid::new_v4(),
        entity_id: Some("org-001".to_string()),
        treasury_id: Some(uuid::Uuid::new_v4()),
        name: Some("Operating Account".to_string()),
        currency: Some("PKR".to_string()),
        balance: Some("50000000".to_string()),
        available: Some("50000000".to_string()),
        status: None,
        funding_details: None,
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassFiscalAccount = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.currency, Some("PKR".to_string()));
    assert_eq!(recovered.balance, Some("50000000".to_string()));
}

#[test]
fn serde_rt_mass_payment() {
    let original = MassPayment {
        id: uuid::Uuid::new_v4(),
        account_id: Some(uuid::Uuid::new_v4()),
        entity_id: Some("org-001".to_string()),
        transaction_type: Some("payment".to_string()),
        status: Some(MassPaymentStatus::Completed),
        direction: Some("outgoing".to_string()),
        currency: Some("PKR".to_string()),
        amount: Some("1000000".to_string()),
        reference: Some("WHT-2026-Q1".to_string()),
        created_at: Some(Utc::now()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassPayment = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.status, Some(MassPaymentStatus::Completed));
}

#[test]
fn serde_rt_mass_tax_event() {
    let original = MassTaxEvent {
        id: uuid::Uuid::new_v4(),
        entity_id: "org-001".to_string(),
        event_type: "withholding_tax".to_string(),
        amount: "150000".to_string(),
        currency: "PKR".to_string(),
        tax_year: Some("2026".to_string()),
        details: json!({"section": "153", "rate": "4.5%"}),
        created_at: Utc::now(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassTaxEvent = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.event_type, "withholding_tax");
}

#[test]
fn serde_rt_mass_identity_type_all_variants() {
    let variants = [
        MassIdentityType::Individual,
        MassIdentityType::Corporate,
        MassIdentityType::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassIdentityType = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered);
    }
}

#[test]
fn serde_rt_mass_identity_full() {
    let original = MassIdentity {
        organization_id: "org-001".to_string(),
        members: vec![MassMember {
            user_id: Some("user-001".to_string()),
            name: Some("Raeez Lorgat".to_string()),
            email: Some("raeez@momentum.inc".to_string()),
            profile_image: None,
            roles: vec!["admin".to_string()],
        }],
        directors: vec![],
        shareholders: vec![MassShareholder {
            id: uuid::Uuid::new_v4(),
            organization_id: "org-001".to_string(),
            user_id: Some("user-001".to_string()),
            email: Some("raeez@momentum.inc".to_string()),
            first_name: Some("Raeez".to_string()),
            last_name: Some("Lorgat".to_string()),
            business_name: None,
            is_entity: Some(false),
            status: Some(MassIdentityStatus::Verified),
            outstanding_shares: Some(5_000_000),
            fully_diluted_shares: Some(5_000_000),
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassIdentity = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.organization_id, "org-001");
    assert_eq!(recovered.members.len(), 1);
    assert_eq!(recovered.shareholders.len(), 1);
}

#[test]
fn serde_rt_mass_cap_table() {
    let original = MassCapTable {
        id: uuid::Uuid::new_v4(),
        organization_id: "org-001".to_string(),
        authorized_shares: Some(10_000_000),
        outstanding_shares: Some(5_000_000),
        fully_diluted_shares: Some(6_000_000),
        reserved_shares: Some(1_000_000),
        unreserved_shares: Some(4_000_000),
        share_classes: vec![MassShareClass {
            id: None,
            name: "Class A Common".to_string(),
            authorized_shares: 10_000_000,
            outstanding_shares: 5_000_000,
            par_value: Some("1.00".to_string()),
            voting_rights: true,
            restricted: false,
            class_type: None,
        }],
        shareholders: vec![],
        options_pools: vec![],
        created_at: Some(Utc::now()),
        updated_at: Some(Utc::now()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassCapTable = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.share_classes.len(), 1);
    assert_eq!(recovered.share_classes[0].authorized_shares, 10_000_000);
}

#[test]
fn serde_rt_mass_ownership_transfer() {
    let original = MassOwnershipTransfer {
        id: uuid::Uuid::new_v4(),
        from_holder: "holder-001".to_string(),
        to_holder: "holder-002".to_string(),
        share_class: "Class A Common".to_string(),
        quantity: 100_000,
        price_per_share: Some("2.50".to_string()),
        transferred_at: Utc::now(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MassOwnershipTransfer = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.quantity, 100_000);
}

#[test]
fn serde_rt_mass_consent_status_all_variants() {
    let variants = [
        MassConsentStatus::Pending,
        MassConsentStatus::Approved,
        MassConsentStatus::Rejected,
        MassConsentStatus::Expired,
        MassConsentStatus::ForceApproved,
        MassConsentStatus::Completed,
        MassConsentStatus::Canceled,
        MassConsentStatus::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassConsentStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered);
    }
}

#[test]
fn serde_rt_mass_payment_status_all_variants() {
    let variants = [
        MassPaymentStatus::Pending,
        MassPaymentStatus::Completed,
        MassPaymentStatus::Failed,
        MassPaymentStatus::Reversed,
        MassPaymentStatus::Unknown,
    ];
    for v in &variants {
        let json = serde_json::to_string(v).expect("serialize");
        let recovered: MassPaymentStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(*v, recovered);
    }
}

// =========================================================================
// msez-zkp: Circuit and proof type serde round-trips
// =========================================================================

use msez_zkp::circuits::compliance::{
    BalanceSufficiencyCircuit, SanctionsClearanceCircuit, TensorInclusionCircuit,
};
use msez_zkp::circuits::identity::{
    AttestationValidityCircuit, KycAttestationCircuit, ThresholdSignatureCircuit,
};
use msez_zkp::circuits::migration::{
    CompensationRecord as ZkpCompensationRecord, CompensationValidityCircuit,
    MigrationEvidenceCircuit, OwnershipChainCircuit, OwnershipEntry,
};
use msez_zkp::circuits::settlement::{
    MerkleMembershipCircuit, NettingValidityCircuit, RangeProofCircuit,
};
use msez_zkp::mock::{MockCircuit, MockProof};

#[test]
fn serde_rt_mock_proof() {
    let original = MockProof {
        proof_hex: "abcdef1234567890".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MockProof = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(original, recovered);
}

#[test]
fn serde_rt_mock_circuit() {
    let original = MockCircuit {
        circuit_data: json!({"type": "test_circuit", "params": [1, 2, 3]}),
        public_inputs: vec![0xaa, 0xbb, 0xcc],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MockCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.public_inputs, original.public_inputs);
}

#[test]
fn serde_rt_balance_sufficiency_circuit() {
    let original = BalanceSufficiencyCircuit {
        threshold: 1_000_000,
        threshold_public: true,
        result_commitment: [0xaa; 32],
        balance: 5_000_000,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: BalanceSufficiencyCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.threshold, 1_000_000);
    assert_eq!(recovered.balance, 5_000_000);
    assert_eq!(recovered.result_commitment, [0xaa; 32]);
}

#[test]
fn serde_rt_sanctions_clearance_circuit() {
    let original = SanctionsClearanceCircuit {
        sanctions_root: [0xbb; 32],
        verification_timestamp: 1_700_000_000,
        entity_hash: [0xcc; 32],
        merkle_proof: vec![[0xdd; 32], [0xee; 32]],
        merkle_path_indices: vec![true, false],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: SanctionsClearanceCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.merkle_proof.len(), 2);
    assert_eq!(recovered.merkle_path_indices, vec![true, false]);
}

#[test]
fn serde_rt_tensor_inclusion_circuit() {
    let original = TensorInclusionCircuit {
        tensor_commitment: [0xaa; 32],
        claimed_state: 1,
        asset_id: "asset-001".to_string(),
        jurisdiction_id: "PAK".to_string(),
        domain: 3,
        time_quantum: 1_700_000_000,
        merkle_proof: vec![[0xbb; 32]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: TensorInclusionCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.asset_id, "asset-001");
}

#[test]
fn serde_rt_kyc_attestation_circuit() {
    let original = KycAttestationCircuit {
        approved_issuers_root: [0x11; 32],
        min_kyc_level: 2,
        verification_timestamp: 1_700_000_000,
        attestation_hash: [0x22; 32],
        issuer_signature: vec![0x33; 64],
        issuer_pubkey: vec![0x44; 32],
        kyc_level: 3,
        issuer_merkle_proof: vec![[0x55; 32]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: KycAttestationCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.kyc_level, 3);
    assert_eq!(recovered.min_kyc_level, 2);
}

#[test]
fn serde_rt_attestation_validity_circuit() {
    let original = AttestationValidityCircuit {
        attestation_commitment: [0xaa; 32],
        current_timestamp: 1_700_000_000,
        revocation_root: [0xbb; 32],
        attestation_hash: [0xcc; 32],
        expiry_timestamp: 1_800_000_000,
        revocation_non_membership: vec![[0xdd; 32]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AttestationValidityCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.expiry_timestamp, 1_800_000_000);
}

#[test]
fn serde_rt_threshold_signature_circuit() {
    let original = ThresholdSignatureCircuit {
        statement_hash: [0xaa; 32],
        threshold: 3,
        authorized_signers_root: [0xbb; 32],
        signatures: vec![vec![0xcc; 64], vec![0xdd; 64]],
        signer_pubkeys: vec![vec![0xee; 32], vec![0xff; 32]],
        signer_merkle_proofs: vec![vec![[0x11; 32]], vec![[0x22; 32]]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ThresholdSignatureCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.threshold, 3);
    assert_eq!(recovered.signatures.len(), 2);
}

#[test]
fn serde_rt_migration_evidence_circuit() {
    let original = MigrationEvidenceCircuit {
        source_jurisdiction: [0x11; 32],
        target_jurisdiction: [0x22; 32],
        migration_id: [0x33; 32],
        final_state_commitment: [0x44; 32],
        phase_evidence: vec![[0x55; 32], [0x66; 32]],
        transition_timestamps: vec![1_700_000_000, 1_700_001_000],
        approval_signatures: vec![vec![0x77; 64]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MigrationEvidenceCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.phase_evidence.len(), 2);
}

#[test]
fn serde_rt_ownership_chain_circuit() {
    let original = OwnershipChainCircuit {
        asset_digest: [0xaa; 32],
        current_owner_commitment: [0xbb; 32],
        chain_root: [0xcc; 32],
        ownership_entries: vec![OwnershipEntry {
            owner_hash: [0xdd; 32],
            timestamp: 1_700_000_000,
            evidence_hash: [0xee; 32],
        }],
        transfer_proofs: vec![vec![[0xff; 32]]],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: OwnershipChainCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.ownership_entries.len(), 1);
}

#[test]
fn serde_rt_compensation_validity_circuit() {
    let original = CompensationValidityCircuit {
        migration_id: [0xaa; 32],
        compensation_commitment: [0xbb; 32],
        compensation_records: vec![ZkpCompensationRecord {
            action_type: "rollback".to_string(),
            success: true,
            evidence_hash: [0xcc; 32],
            timestamp: 1_700_000_000,
        }],
        failure_evidence: [0xdd; 32],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: CompensationValidityCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.compensation_records.len(), 1);
}

#[test]
fn serde_rt_range_proof_circuit() {
    let original = RangeProofCircuit {
        lower_bound: 0,
        upper_bound: 1_000_000_000,
        value_commitment: [0xaa; 32],
        value: 500_000,
        blinding_factor: [0xbb; 32],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: RangeProofCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.value, 500_000);
    assert_eq!(recovered.lower_bound, 0);
}

#[test]
fn serde_rt_merkle_membership_circuit() {
    let original = MerkleMembershipCircuit {
        merkle_root: [0xaa; 32],
        leaf_hash: [0xbb; 32],
        merkle_proof: vec![[0xcc; 32], [0xdd; 32], [0xee; 32]],
        path_indices: vec![true, false, true],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: MerkleMembershipCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.merkle_proof.len(), 3);
    assert_eq!(recovered.path_indices.len(), 3);
}

#[test]
fn serde_rt_netting_validity_circuit() {
    let original = NettingValidityCircuit {
        gross_positions_commitment: [0xaa; 32],
        net_positions_commitment: [0xbb; 32],
        participant_count: 5,
        gross_positions: vec![100, 200, 300, 400, 500],
        net_positions: vec![50, -50, 100, -100, 0],
        netting_matrix: vec![0; 25],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: NettingValidityCircuit = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.participant_count, 5);
    assert_eq!(recovered.gross_positions.len(), 5);
}

// =========================================================================
// msez-arbitration: Complex struct serde round-trips
// =========================================================================

// Note: Dispute/Enforcement/Escrow/Evidence types already imported at top of file.
// These tests use the existing imports plus new sub-types.

#[test]
fn serde_rt_claim_full_with_evidence() {
    let original = Claim {
        claim_id: "CLM-002".to_string(),
        claim_type: DisputeType::BreachOfContract,
        description: "Failure to deliver goods per corridor agreement".to_string(),
        amount: Some(Money::new("500000", "USD").unwrap()),
        supporting_evidence_digests: vec![test_digest()],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Claim = serde_json::from_str(&json).expect("deserialize");
    // BUG-034: Claim does not derive PartialEq — field-by-field check
    assert_eq!(recovered.claim_id, "CLM-002");
    assert_eq!(recovered.claim_type, DisputeType::BreachOfContract);
    assert_eq!(recovered.supporting_evidence_digests.len(), 1);
}

#[test]
fn serde_rt_arbitration_institution() {
    let original = ArbitrationInstitution {
        id: "SIAC".to_string(),
        name: "Singapore International Arbitration Centre".to_string(),
        jurisdiction_id: "SG".to_string(),
        supported_dispute_types: vec![DisputeType::BreachOfContract, DisputeType::PaymentDefault],
        emergency_arbitrator: true,
        expedited_procedure: true,
        enforcement_jurisdictions: vec!["SG".to_string(), "PK".to_string(), "AE".to_string()],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ArbitrationInstitution = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.id, "SIAC");
    assert_eq!(recovered.supported_dispute_types.len(), 2);
}

#[test]
fn serde_rt_dispute_full_roundtrip() {
    let original = Dispute {
        id: DisputeId::new(),
        state: DisputeState::Filed,
        dispute_type: DisputeType::PaymentDefault,
        claimant: ArbParty {
            did: msez_core::Did::new("did:key:z6MkClaimant").unwrap(),
            legal_name: "Claimant Corp".to_string(),
            jurisdiction_id: Some(JurisdictionId::new("PK").unwrap()),
        },
        respondent: ArbParty {
            did: msez_core::Did::new("did:key:z6MkRespondent").unwrap(),
            legal_name: "Respondent LLC".to_string(),
            jurisdiction_id: Some(JurisdictionId::new("AE").unwrap()),
        },
        jurisdiction: JurisdictionId::new("SG").unwrap(),
        corridor_id: Some(CorridorId::new()),
        institution_id: "SIAC".to_string(),
        claims: vec![Claim {
            claim_id: "CLM-003".to_string(),
            claim_type: DisputeType::PaymentDefault,
            description: "Non-payment".to_string(),
            amount: Some(Money::new("100000", "USD").unwrap()),
            supporting_evidence_digests: vec![],
        }],
        filed_at: Timestamp::now(),
        updated_at: Timestamp::now(),
        transition_log: vec![],
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Dispute = serde_json::from_str(&json).expect("deserialize");
    // BUG-035: Dispute does not derive PartialEq
    assert_eq!(recovered.state, DisputeState::Filed);
    assert_eq!(recovered.dispute_type, DisputeType::PaymentDefault);
    assert_eq!(recovered.claims.len(), 1);
}

#[test]
fn serde_rt_enforcement_precondition() {
    let original = EnforcementPrecondition {
        description: "Appeal period must expire".to_string(),
        satisfied: false,
        evidence_digest: Some(test_digest()),
        checked_at: Some(Timestamp::now()),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EnforcementPrecondition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.description, "Appeal period must expire");
    assert!(!recovered.satisfied);
}

#[test]
fn serde_rt_enforcement_order_full_roundtrip() {
    use msez_arbitration::enforcement::EnforcementAction;
    let original = EnforcementOrder::new(
        DisputeId::new(),
        test_digest(),
        vec![EnforcementAction::MonetaryPenalty {
            party: msez_core::Did::new("did:key:z6MkPenalized").unwrap(),
            amount: "50000".to_string(),
            currency: "USD".to_string(),
        }],
        None, // appeal_deadline
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EnforcementOrder = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.status, EnforcementStatus::Pending);
    assert_eq!(recovered.actions.len(), 1);
}

#[test]
fn serde_rt_release_condition() {
    let original = ReleaseCondition {
        condition_type: ReleaseConditionType::RulingEnforced,
        evidence_digest: test_digest(),
        satisfied_at: Timestamp::now(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ReleaseCondition = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        recovered.condition_type,
        ReleaseConditionType::RulingEnforced
    );
}

#[test]
fn serde_rt_escrow_transaction() {
    let original = EscrowTransaction {
        transaction_type: TransactionType::Deposit,
        amount: "100000".to_string(),
        currency: "USD".to_string(),
        timestamp: Utc::now(),
        evidence_digest: test_digest(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EscrowTransaction = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.transaction_type, TransactionType::Deposit);
    assert_eq!(recovered.amount, "100000");
}

#[test]
fn serde_rt_escrow_account_full_roundtrip() {
    let original = EscrowAccount::create(
        DisputeId::new(),
        EscrowType::SecurityDeposit,
        "USD".to_string(),
        None,
    );
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: EscrowAccount = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.escrow_type, EscrowType::SecurityDeposit);
    assert_eq!(recovered.status, EscrowStatus::Pending);
}

#[test]
fn serde_rt_authenticity_attestation_roundtrip() {
    use msez_arbitration::evidence::AuthenticityType;
    let original = AuthenticityAttestation {
        attestation_type: AuthenticityType::NotarizedDocument,
        proof_digest: test_digest(),
        attester: msez_core::Did::new("did:key:z6MkNotary").unwrap(),
        attested_at: Timestamp::now(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AuthenticityAttestation = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(
        recovered.attestation_type,
        AuthenticityType::NotarizedDocument
    );
}

#[test]
fn serde_rt_chain_of_custody_entry_roundtrip() {
    let original = ChainOfCustodyEntry {
        custodian: msez_core::Did::new("did:key:z6MkCustodian").unwrap(),
        transferred_at: Timestamp::now(),
        evidence_digest_at_transfer: test_digest(),
        description: "Transferred to arbitration chamber".to_string(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ChainOfCustodyEntry = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.description, "Transferred to arbitration chamber");
}

// =========================================================================
// msez-corridor: Untested types
// =========================================================================

use msez_corridor::anchor::AnchorReceipt;
use msez_corridor::fork::ForkBranch;
use msez_corridor::receipt::Checkpoint;

#[test]
fn serde_rt_anchor_receipt() {
    use msez_corridor::anchor::{AnchorCommitment, AnchorStatus};
    let original = AnchorReceipt {
        commitment: AnchorCommitment {
            checkpoint_digest: test_digest(),
            chain_id: Some("eth-mainnet".to_string()),
            checkpoint_height: 100,
        },
        chain_id: "eth-mainnet".to_string(),
        transaction_id: "0xabcdef1234567890".to_string(),
        block_number: 19_000_000,
        status: AnchorStatus::Confirmed,
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: AnchorReceipt = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.chain_id, "eth-mainnet");
}

#[test]
fn serde_rt_fork_branch() {
    let original = ForkBranch {
        receipt_digest: test_digest(),
        timestamp: Utc::now(),
        attestation_count: 3,
        next_root: "bb".repeat(32),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: ForkBranch = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.attestation_count, 3);
}

#[test]
fn serde_rt_checkpoint_full() {
    let original = Checkpoint {
        corridor_id: CorridorId::new(),
        height: 42,
        mmr_root: "cc".repeat(32),
        timestamp: Timestamp::now(),
        checkpoint_digest: test_digest(),
    };
    let json = serde_json::to_string(&original).expect("serialize");
    let recovered: Checkpoint = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(recovered.height, 42);
}
