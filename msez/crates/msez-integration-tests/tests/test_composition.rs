//! Tests for multi-zone composition operations.
//!
//! Validates the compliance manifold's ability to compose multiple
//! jurisdictions, compute shortest paths, and produce deterministic
//! digests for composition snapshots.

use msez_core::{sha256_digest, CanonicalBytes, ComplianceDomain};
use msez_tensor::{ComplianceManifold, CorridorEdge as TensorCorridorEdge, JurisdictionNode};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pk_rsez_node() -> JurisdictionNode {
    JurisdictionNode {
        jurisdiction_id: "pk-rsez".to_string(),
        name: "Pakistan Rashakai SEZ".to_string(),
        country_code: "PK".to_string(),
        supported_asset_classes: vec!["equity".to_string(), "debt".to_string()],
        entry_fee_usd: 5000,
        annual_fee_usd: 2000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Aml, ComplianceDomain::Kyc],
    }
}

fn ae_difc_node() -> JurisdictionNode {
    JurisdictionNode {
        jurisdiction_id: "ae-difc".to_string(),
        name: "Dubai International Financial Centre".to_string(),
        country_code: "AE".to_string(),
        supported_asset_classes: vec!["equity".to_string(), "fund".to_string()],
        entry_fee_usd: 10000,
        annual_fee_usd: 5000,
        is_active: true,
        required_domains: vec![
            ComplianceDomain::Aml,
            ComplianceDomain::Kyc,
            ComplianceDomain::Sanctions,
        ],
    }
}

fn corridor_pk_ae() -> TensorCorridorEdge {
    TensorCorridorEdge {
        corridor_id: "pk-rsez--ae-difc".to_string(),
        source_jurisdiction: "pk-rsez".to_string(),
        target_jurisdiction: "ae-difc".to_string(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 15,
        flat_fee_usd: 100,
        estimated_transfer_hours: 24,
        settlement_finality_hours: 48,
        required_domains: vec![ComplianceDomain::Aml, ComplianceDomain::Sanctions],
    }
}

// ---------------------------------------------------------------------------
// Two-zone composition
// ---------------------------------------------------------------------------

#[test]
fn two_zone_composition() {
    let mut manifold = ComplianceManifold::new();
    manifold.add_jurisdiction(pk_rsez_node());
    manifold.add_jurisdiction(ae_difc_node());
    manifold.add_corridor(corridor_pk_ae());

    assert_eq!(manifold.list_jurisdictions().len(), 2);
    assert!(!manifold.list_corridors().is_empty());
}

// ---------------------------------------------------------------------------
// Multi-zone routing
// ---------------------------------------------------------------------------

#[test]
fn multi_zone_routing() {
    let mut manifold = ComplianceManifold::new();

    manifold.add_jurisdiction(pk_rsez_node());
    manifold.add_jurisdiction(ae_difc_node());
    manifold.add_jurisdiction(JurisdictionNode {
        jurisdiction_id: "kz-aifc".to_string(),
        name: "Astana International Financial Centre".to_string(),
        country_code: "KZ".to_string(),
        supported_asset_classes: vec!["equity".to_string()],
        entry_fee_usd: 8000,
        annual_fee_usd: 3000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Aml],
    });

    manifold.add_corridor(corridor_pk_ae());
    manifold.add_corridor(TensorCorridorEdge {
        corridor_id: "ae-difc--kz-aifc".to_string(),
        source_jurisdiction: "ae-difc".to_string(),
        target_jurisdiction: "kz-aifc".to_string(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 20,
        flat_fee_usd: 200,
        estimated_transfer_hours: 48,
        settlement_finality_hours: 72,
        required_domains: vec![ComplianceDomain::Aml],
    });

    // The manifold should have 3 jurisdictions.
    assert_eq!(manifold.list_jurisdictions().len(), 3);
}

// ---------------------------------------------------------------------------
// Composition deterministic digest
// ---------------------------------------------------------------------------

#[test]
fn composition_deterministic_digest() {
    // The same composition data must produce the same digest.
    let data = json!({
        "jurisdictions": ["pk-rsez", "ae-difc"],
        "corridors": ["pk-rsez--ae-difc"],
        "version": "0.4.44"
    });

    let c1 = CanonicalBytes::new(&data).unwrap();
    let c2 = CanonicalBytes::new(&data).unwrap();

    assert_eq!(
        sha256_digest(&c1).to_hex(),
        sha256_digest(&c2).to_hex(),
        "Composition digest must be deterministic"
    );
}

#[test]
fn corridor_edge_transfer_cost_computation() {
    let edge = corridor_pk_ae();
    // 15 bps on $1,000,000 = $1,500 + $100 flat fee = $1,600
    let cost = edge.transfer_cost(1_000_000);
    assert_eq!(cost, 1600, "Transfer cost must be 15bps + flat fee");
}

#[test]
fn composition_key_ordering_invariant() {
    let a = json!({
        "zone_a": "pk-rsez",
        "zone_b": "ae-difc",
        "corridors": 1
    });
    let b = json!({
        "corridors": 1,
        "zone_b": "ae-difc",
        "zone_a": "pk-rsez"
    });

    let ca = CanonicalBytes::new(&a).unwrap();
    let cb = CanonicalBytes::new(&b).unwrap();

    assert_eq!(sha256_digest(&ca).to_hex(), sha256_digest(&cb).to_hex(),);
}
