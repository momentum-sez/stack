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

use msez_vc::credential::{
    ContextValue, CredentialTypeValue, ProofValue, VerifiableCredential,
};
use msez_vc::proof::{Proof, ProofPurpose, ProofType};

// =========================================================================
// msez-arbitration types
// =========================================================================

use msez_arbitration::dispute::{DisputeId, DisputeState, DisputeType};
use msez_arbitration::escrow::{
    EscrowId, EscrowStatus, EscrowType, ReleaseConditionType, TransactionType,
};
use msez_arbitration::enforcement::{EnforcementOrderId, EnforcementReceiptId, EnforcementStatus};

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
        let recovered: EntityLifecycleState =
            serde_json::from_str(&json).expect("deserialize");
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
    // BUG-006: SettlementPlan does not derive PartialEq
    // Also: SettlementPlan.reduction_percentage is f64, which means
    // serde round-trip may suffer floating-point precision issues.
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
    assert_eq!(recovered.gross_total, plan.gross_total);
    assert_eq!(recovered.net_total, plan.net_total);
    assert_eq!(recovered.settlement_legs.len(), plan.settlement_legs.len());
    // Verify the f64 field survives round-trip
    assert!(
        (recovered.reduction_percentage - plan.reduction_percentage).abs() < f64::EPSILON,
        "reduction_percentage changed on round-trip: {} vs {}",
        recovered.reduction_percentage,
        plan.reduction_percentage
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
    assert_eq!(
        recovered.verification_method,
        "did:key:z6MkTest#key-1"
    );
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
    let recovered: VerifiableCredential =
        serde_json::from_str(&json).expect("deserialize");
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
    let recovered: VerifiableCredential =
        serde_json::from_str(&json).expect("deserialize");
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
    let recovered: VerifiableCredential =
        serde_json::from_str(&json).expect("deserialize");
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
        let recovered: ReleaseConditionType =
            serde_json::from_str(&json).expect("deserialize");
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
    let recovered: EnforcementReceiptId =
        serde_json::from_str(&json).expect("deserialize");
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
        let recovered: EnforcementStatus =
            serde_json::from_str(&json).expect("deserialize");
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
        let recovered: AuthorizationRequirement =
            serde_json::from_str(&json).expect("deserialize");
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
    assert!(result.is_err(), "invalid DID must be rejected at deserialization");
}

#[test]
fn serde_rt_ntn_rejects_invalid() {
    // BUG-014 RESOLVED: Custom Deserialize validates via Ntn::new().
    let invalid_json = "\"12345\""; // Only 5 digits, should be 7
    let result: Result<msez_core::Ntn, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "invalid NTN must be rejected at deserialization");
}

#[test]
fn serde_rt_cnic_rejects_invalid() {
    // BUG-015 RESOLVED: Custom Deserialize validates via Cnic::new().
    let invalid_json = "\"123\""; // Only 3 digits, should be 13
    let result: Result<msez_core::Cnic, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "invalid CNIC must be rejected at deserialization");
}

#[test]
fn serde_rt_passport_number_rejects_invalid() {
    // BUG-016 RESOLVED: Custom Deserialize validates via PassportNumber::new().
    let invalid_json = "\"AB\""; // Only 2 chars, minimum is 5
    let result: Result<msez_core::PassportNumber, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "invalid passport must be rejected at deserialization");
}

#[test]
fn serde_rt_jurisdiction_id_rejects_invalid() {
    // BUG-017 RESOLVED: Custom Deserialize validates via JurisdictionId::new().
    let invalid_json = "\"\""; // Empty string, should be rejected
    let result: Result<JurisdictionId, _> = serde_json::from_str(invalid_json);
    assert!(result.is_err(), "empty JurisdictionId must be rejected at deserialization");
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
