//! # Trade Playbook Generator Integration Tests
//!
//! Python counterpart: `tests/test_trade_playbook_generator.py`
//!
//! Tests trade playbook generation using corridor bridge routing:
//! - Playbook with a single corridor route
//! - Playbook with multi-hop route
//! - Playbook digest is deterministic

use msez_core::{sha256_digest, CanonicalBytes, CorridorId, JurisdictionId};
use msez_corridor::{BridgeEdge, CorridorBridge};
use serde_json::json;

fn jid(name: &str) -> JurisdictionId {
    JurisdictionId::new(name).unwrap()
}

fn edge(from: &str, to: &str, fee_bps: u32, time: u64) -> BridgeEdge {
    BridgeEdge {
        from: jid(from),
        to: jid(to),
        corridor_id: CorridorId::new(),
        fee_bps,
        settlement_time_secs: time,
    }
}

// ---------------------------------------------------------------------------
// 1. Playbook with single corridor
// ---------------------------------------------------------------------------

#[test]
fn playbook_with_single_corridor() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));

    let route = bridge
        .find_route(&jid("PK-RSEZ"), &jid("AE-DIFC"))
        .expect("direct route should exist");

    assert_eq!(route.hop_count(), 1);
    assert_eq!(route.total_fee_bps, 50);

    // Generate a playbook descriptor
    let playbook = json!({
        "trade_type": "cross_border_settlement",
        "source": "PK-RSEZ",
        "destination": "AE-DIFC",
        "hop_count": route.hop_count(),
        "total_fee_bps": route.total_fee_bps,
        "total_settlement_time_secs": route.total_settlement_time_secs,
        "jurisdictions": route.jurisdictions().iter().map(|j| j.as_str()).collect::<Vec<_>>()
    });

    let canonical = CanonicalBytes::new(&playbook).unwrap();
    let digest = sha256_digest(&canonical);
    assert_eq!(digest.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 2. Playbook with multi-hop route
// ---------------------------------------------------------------------------

#[test]
fn playbook_with_multi_hop_route() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));
    bridge.add_edge(edge("GB-LNDN", "US-NYFC", 25, 86400));

    let route = bridge
        .find_route(&jid("PK-RSEZ"), &jid("US-NYFC"))
        .expect("multi-hop route should exist");

    assert_eq!(route.hop_count(), 3);
    assert_eq!(route.total_fee_bps, 105); // 50 + 30 + 25

    let jurisdictions: Vec<&str> = route.jurisdictions().iter().map(|j| j.as_str()).collect();
    assert_eq!(
        jurisdictions,
        vec!["PK-RSEZ", "AE-DIFC", "GB-LNDN", "US-NYFC"]
    );

    // Generate a multi-hop playbook
    let playbook = json!({
        "trade_type": "multi_hop_settlement",
        "source": "PK-RSEZ",
        "destination": "US-NYFC",
        "hop_count": route.hop_count(),
        "total_fee_bps": route.total_fee_bps,
        "hops": jurisdictions
    });

    let canonical = CanonicalBytes::new(&playbook).unwrap();
    assert!(!canonical.as_bytes().is_empty());
}

// ---------------------------------------------------------------------------
// 3. Playbook digest is deterministic
// ---------------------------------------------------------------------------

#[test]
fn playbook_digest_deterministic() {
    let playbook = json!({
        "trade_type": "cross_border_settlement",
        "source": "PK-RSEZ",
        "destination": "AE-DIFC",
        "hop_count": 1,
        "total_fee_bps": 50,
        "currency": "PKR",
        "amount": "1000000"
    });

    let d1 = sha256_digest(&CanonicalBytes::new(&playbook).unwrap());
    let d2 = sha256_digest(&CanonicalBytes::new(&playbook).unwrap());
    let d3 = sha256_digest(&CanonicalBytes::new(&playbook).unwrap());

    assert_eq!(d1, d2, "first and second must match");
    assert_eq!(d2, d3, "second and third must match");
    assert_eq!(d1.to_hex().len(), 64);
}

// ---------------------------------------------------------------------------
// 4. Disconnected route returns None
// ---------------------------------------------------------------------------

#[test]
fn playbook_no_route_disconnected() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("SG-SGFZ", "JP-TKYO", 40, 86400));

    assert!(
        bridge.find_route(&jid("PK-RSEZ"), &jid("SG-SGFZ")).is_none(),
        "disconnected jurisdictions should return no route"
    );
}

// ---------------------------------------------------------------------------
// 5. Bridge with optimal path selection
// ---------------------------------------------------------------------------

#[test]
fn bridge_selects_optimal_path() {
    let mut bridge = CorridorBridge::new();
    // Cheap multi-hop: PK -> AE -> GB (50 + 30 = 80)
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 86400));
    // Expensive direct: PK -> GB (200)
    bridge.add_edge(edge("PK-RSEZ", "GB-LNDN", 200, 86400));

    let route = bridge.find_route(&jid("PK-RSEZ"), &jid("GB-LNDN")).unwrap();
    assert_eq!(
        route.total_fee_bps, 80,
        "Dijkstra should find the cheaper multi-hop route"
    );
    assert_eq!(route.hop_count(), 2);
}

// ---------------------------------------------------------------------------
// 6. Reachability from source jurisdiction
// ---------------------------------------------------------------------------

#[test]
fn reachability_map() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));

    let reachable = bridge.reachable_from(&jid("PK-RSEZ"));
    assert_eq!(reachable.get("PK-RSEZ"), Some(&0));
    assert_eq!(reachable.get("AE-DIFC"), Some(&50));
    assert_eq!(reachable.get("GB-LNDN"), Some(&80));
}
