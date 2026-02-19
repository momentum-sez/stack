//! Tests for hybrid zone deployment configurations.
//!
//! Validates that multi-corridor zone configurations produce deterministic
//! digests and that deployment field ordering does not affect canonical
//! representations.

use mez_core::{sha256_digest, CanonicalBytes, CorridorId, JurisdictionId};
use mez_corridor::{BridgeEdge, CorridorBridge};
use serde_json::json;

// ---------------------------------------------------------------------------
// Hybrid zone with multiple corridors
// ---------------------------------------------------------------------------

#[test]
fn hybrid_zone_with_multiple_corridors() {
    let mut bridge = CorridorBridge::new();

    let ja = JurisdictionId::new("PK-REZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();
    let jc = JurisdictionId::new("KZ-AIFC").unwrap();

    bridge.add_edge(BridgeEdge {
        from: ja.clone(),
        to: jb.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 15,
        settlement_time_secs: 86400,
    });

    bridge.add_edge(BridgeEdge {
        from: jb.clone(),
        to: ja.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 15,
        settlement_time_secs: 86400,
    });

    bridge.add_edge(BridgeEdge {
        from: jb.clone(),
        to: jc.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 20,
        settlement_time_secs: 172800,
    });

    assert_eq!(bridge.edge_count(), 3, "Three directed edges expected");
    assert_eq!(
        bridge.node_count(),
        3,
        "Three unique jurisdictions expected"
    );
}

// ---------------------------------------------------------------------------
// Zone configuration digest
// ---------------------------------------------------------------------------

#[test]
fn zone_configuration_digest() {
    let config = json!({
        "zone_id": "pk-rez",
        "corridors": [
            {"from": "pk-rez", "to": "ae-difc", "fee_bps": 15},
            {"from": "pk-rez", "to": "kz-aifc", "fee_bps": 20}
        ],
        "mode": "hybrid"
    });

    let canonical = CanonicalBytes::new(&config).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);

    // Deterministic on repeated computation.
    let canonical2 = CanonicalBytes::new(&config).unwrap();
    assert_eq!(digest.to_hex(), sha256_digest(&canonical2).to_hex());
}

// ---------------------------------------------------------------------------
// Zone deployment fields deterministic
// ---------------------------------------------------------------------------

#[test]
fn zone_deployment_fields_deterministic() {
    // Key ordering must not affect the canonical digest.
    let config_a = json!({
        "zone_id": "pk-rez",
        "version": "0.4.44",
        "mode": "hybrid",
        "corridors": ["ae-difc", "kz-aifc"]
    });

    let config_b = json!({
        "mode": "hybrid",
        "corridors": ["ae-difc", "kz-aifc"],
        "zone_id": "pk-rez",
        "version": "0.4.44"
    });

    let ca = CanonicalBytes::new(&config_a).unwrap();
    let cb = CanonicalBytes::new(&config_b).unwrap();

    assert_eq!(
        sha256_digest(&ca).to_hex(),
        sha256_digest(&cb).to_hex(),
        "Key ordering must not affect zone deployment digest"
    );
}

#[test]
fn bridge_routing_basic() {
    let mut bridge = CorridorBridge::new();
    let ja = JurisdictionId::new("PK-REZ").unwrap();
    let jb = JurisdictionId::new("AE-DIFC").unwrap();

    bridge.add_edge(BridgeEdge {
        from: ja.clone(),
        to: jb.clone(),
        corridor_id: CorridorId::new(),
        fee_bps: 10,
        settlement_time_secs: 3600,
    });

    let route = bridge.find_route(&ja, &jb);
    assert!(
        route.is_some(),
        "Route from PK-REZ to AE-DIFC should exist"
    );

    let route = route.unwrap();
    assert_eq!(
        route.hops.len(),
        1,
        "Direct route should have exactly 1 hop"
    );
    assert_eq!(route.total_fee_bps, 10);
}
