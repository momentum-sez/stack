//! # Phoenix Layer Comprehensive Test
//!
//! Tests the Phoenix layer components: compliance tensor creation and
//! evaluation, manifold path optimization, migration saga full lifecycle,
//! watcher economy slashing, and compliance domain completeness.

use chrono::{Duration, Utc};
use msez_core::{ComplianceDomain, JurisdictionId, MigrationId, WatcherId};
use msez_state::{MigrationBuilder, MigrationState, SlashingCondition, Watcher, WatcherState};
use msez_tensor::{
    ComplianceManifold, ComplianceState, ComplianceTensor, CorridorEdge, DefaultJurisdiction,
    JurisdictionNode,
};

fn test_jurisdiction(name: &str) -> DefaultJurisdiction {
    DefaultJurisdiction::new(JurisdictionId::new(name).unwrap())
}

fn build_manifold() -> ComplianceManifold {
    let mut manifold = ComplianceManifold::new();

    manifold.add_jurisdiction(JurisdictionNode {
        jurisdiction_id: "pk-rsez".into(),
        name: "Rashakai SEZ".into(),
        country_code: "PK".into(),
        supported_asset_classes: vec!["trade".into()],
        entry_fee_usd: 200,
        annual_fee_usd: 1000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Tax],
    });

    manifold.add_jurisdiction(JurisdictionNode {
        jurisdiction_id: "ae-difc".into(),
        name: "DIFC".into(),
        country_code: "AE".into(),
        supported_asset_classes: vec!["securities".into()],
        entry_fee_usd: 1000,
        annual_fee_usd: 5000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Aml],
    });

    manifold.add_jurisdiction(JurisdictionNode {
        jurisdiction_id: "kz-aifc".into(),
        name: "AIFC".into(),
        country_code: "KZ".into(),
        supported_asset_classes: vec!["digital_assets".into()],
        entry_fee_usd: 500,
        annual_fee_usd: 2000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Sanctions],
    });

    manifold.add_corridor(CorridorEdge {
        corridor_id: "c-pk-ae".into(),
        source_jurisdiction: "pk-rsez".into(),
        target_jurisdiction: "ae-difc".into(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 20,
        flat_fee_usd: 100,
        estimated_transfer_hours: 24,
        settlement_finality_hours: 48,
        required_domains: vec![ComplianceDomain::Aml],
    });

    manifold.add_corridor(CorridorEdge {
        corridor_id: "c-ae-kz".into(),
        source_jurisdiction: "ae-difc".into(),
        target_jurisdiction: "kz-aifc".into(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 15,
        flat_fee_usd: 50,
        estimated_transfer_hours: 12,
        settlement_finality_hours: 24,
        required_domains: vec![ComplianceDomain::Sanctions],
    });

    manifold
}

// ---------------------------------------------------------------------------
// 1. Tensor creation and evaluation
// ---------------------------------------------------------------------------

#[test]
fn tensor_creation_and_evaluation() {
    let mut tensor = ComplianceTensor::new(test_jurisdiction("PK-RSEZ"));
    assert_eq!(tensor.cell_count(), 20);

    // Set some domains
    tensor.set(
        ComplianceDomain::Aml,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Kyc,
        ComplianceState::Compliant,
        vec![],
        None,
    );
    tensor.set(
        ComplianceDomain::Tax,
        ComplianceState::Pending,
        vec![],
        None,
    );

    assert_eq!(tensor.get(ComplianceDomain::Aml), ComplianceState::Compliant);
    assert_eq!(tensor.get(ComplianceDomain::Tax), ComplianceState::Pending);

    // Slice
    let slice = tensor.slice(&[ComplianceDomain::Aml, ComplianceDomain::Kyc]);
    assert!(slice.all_passing());
    assert_eq!(slice.aggregate_state(), ComplianceState::Compliant);

    // Commitment
    let commitment = tensor.commit().unwrap();
    assert_eq!(commitment.to_hex().len(), 64);
    assert_eq!(commitment.cell_count(), 20);
    assert_eq!(commitment.jurisdiction_id(), "PK-RSEZ");
}

// ---------------------------------------------------------------------------
// 2. Manifold shortest path
// ---------------------------------------------------------------------------

#[test]
fn manifold_shortest_path() {
    let manifold = build_manifold();

    // Direct path
    let path = manifold
        .find_path("pk-rsez", "ae-difc", None, 10_000)
        .expect("direct path should exist");
    assert_eq!(path.hop_count(), 1);
    assert!(path.total_cost_usd > 0);

    // Multi-hop path
    let path2 = manifold
        .find_path("pk-rsez", "kz-aifc", None, 10_000)
        .expect("2-hop path should exist");
    assert_eq!(path2.hop_count(), 2);
    assert!(path2.total_cost_usd > path.total_cost_usd);

    // Compliance distance
    let dist = manifold
        .compliance_distance("pk-rsez", "kz-aifc", None)
        .unwrap();
    assert_eq!(dist.hop_count, 2);
    assert_eq!(dist.source, "pk-rsez");
    assert_eq!(dist.target, "kz-aifc");
}

// ---------------------------------------------------------------------------
// 3. Migration saga full lifecycle
// ---------------------------------------------------------------------------

#[test]
fn migration_saga_full_lifecycle() {
    let id = MigrationId::new();
    let deadline = Utc::now() + Duration::hours(24);

    let mut saga = MigrationBuilder::new(id)
        .deadline(deadline)
        .source(JurisdictionId::new("PK-RSEZ").unwrap())
        .destination(JurisdictionId::new("AE-DIFC").unwrap())
        .asset_description("Trade settlement")
        .build();

    assert_eq!(saga.state, MigrationState::Initiated);

    // Advance through all forward phases
    let expected = [
        MigrationState::ComplianceCheck,
        MigrationState::AttestationGathering,
        MigrationState::SourceLocked,
        MigrationState::InTransit,
        MigrationState::DestinationVerification,
        MigrationState::DestinationUnlock,
        MigrationState::Completed,
    ];

    for expected_state in &expected {
        let next = saga.advance().unwrap();
        assert_eq!(next, *expected_state);
    }

    assert!(saga.state.is_terminal());
    assert_eq!(saga.state, MigrationState::Completed);
}

// ---------------------------------------------------------------------------
// 4. Watcher economy slashing
// ---------------------------------------------------------------------------

#[test]
fn watcher_economy_slashing() {
    let mut w = Watcher::new(WatcherId::new());
    w.bond(1_000_000).unwrap();
    w.activate().unwrap();

    // Availability failure: 1% slash
    let slashed = w.slash(SlashingCondition::AvailabilityFailure).unwrap();
    assert_eq!(slashed, 10_000);
    assert_eq!(w.available_stake(), 990_000);
    assert_eq!(w.state, WatcherState::Slashed);

    // Rebond and re-activate
    w.rebond(10_000).unwrap();
    w.activate().unwrap();

    // False attestation: 50% slash
    let slashed = w.slash(SlashingCondition::FalseAttestation).unwrap();
    assert_eq!(slashed, 505_000); // 50% of 1_010_000
    assert_eq!(w.state, WatcherState::Slashed);
}

// ---------------------------------------------------------------------------
// 5. Compliance domain completeness
// ---------------------------------------------------------------------------

#[test]
fn compliance_domain_completeness() {
    assert_eq!(ComplianceDomain::COUNT, 20);
    assert_eq!(ComplianceDomain::all().len(), 20);

    // Every domain has a non-empty string representation
    for domain in ComplianceDomain::all() {
        assert!(!domain.as_str().is_empty());
        assert!(!domain.to_string().is_empty());
    }

    // Roundtrip through Display/FromStr
    for domain in ComplianceDomain::all() {
        let s = domain.to_string();
        let parsed: ComplianceDomain = s.parse().unwrap();
        assert_eq!(*domain, parsed);
    }

    // Serde roundtrip
    for domain in ComplianceDomain::all() {
        let json = serde_json::to_string(domain).unwrap();
        let deserialized: ComplianceDomain = serde_json::from_str(&json).unwrap();
        assert_eq!(*domain, deserialized);
    }
}
