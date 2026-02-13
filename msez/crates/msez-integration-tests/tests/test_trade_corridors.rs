//! # Trade Corridors Integration Test
//!
//! Tests cross-border trade corridor operations including bridge routing,
//! multi-hop path finding, and disconnected jurisdiction detection.
//!
//! Verifies that the corridor bridge graph correctly computes minimum-fee
//! routes using Dijkstra's algorithm and handles edge cases such as
//! disconnected graphs and same-source-target queries.

use msez_core::{CorridorId, JurisdictionId};
use msez_corridor::{BridgeEdge, CorridorBridge};

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
// 1. Create a trade corridor bridge with multiple jurisdictions
// ---------------------------------------------------------------------------

#[test]
fn create_trade_corridor_bridge() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "PK-RSEZ", 55, 86400));
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));

    assert_eq!(bridge.edge_count(), 3);
    assert_eq!(bridge.node_count(), 3);
}

// ---------------------------------------------------------------------------
// 2. Route between two directly connected jurisdictions
// ---------------------------------------------------------------------------

#[test]
fn route_between_two_jurisdictions() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "PK-RSEZ", 55, 86400));

    let route = bridge
        .find_route(&jid("PK-RSEZ"), &jid("AE-DIFC"))
        .expect("direct route should exist");
    assert_eq!(route.hop_count(), 1);
    assert_eq!(route.total_fee_bps, 50);
    assert_eq!(route.total_settlement_time_secs, 86400);

    let jurisdictions: Vec<&str> = route.jurisdictions().iter().map(|j| j.as_str()).collect();
    assert_eq!(jurisdictions, vec!["PK-RSEZ", "AE-DIFC"]);
}

// ---------------------------------------------------------------------------
// 3. Route through intermediate jurisdictions (multi-hop)
// ---------------------------------------------------------------------------

#[test]
fn route_through_intermediate_jurisdictions() {
    let mut bridge = CorridorBridge::new();
    // PK-RSEZ -> AE-DIFC (50 bps)
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    // AE-DIFC -> GB-LNDN (30 bps)
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));
    // GB-LNDN -> US-NYFC (25 bps)
    bridge.add_edge(edge("GB-LNDN", "US-NYFC", 25, 86400));
    // PK-RSEZ -> US-NYFC direct (200 bps, expensive)
    bridge.add_edge(edge("PK-RSEZ", "US-NYFC", 200, 259200));

    let route = bridge
        .find_route(&jid("PK-RSEZ"), &jid("US-NYFC"))
        .expect("route should exist");

    // Multi-hop: 50 + 30 + 25 = 105 bps < 200 bps direct
    assert_eq!(route.hop_count(), 3);
    assert_eq!(route.total_fee_bps, 105);

    let jurisdictions: Vec<&str> = route.jurisdictions().iter().map(|j| j.as_str()).collect();
    assert_eq!(
        jurisdictions,
        vec!["PK-RSEZ", "AE-DIFC", "GB-LNDN", "US-NYFC"]
    );
}

// ---------------------------------------------------------------------------
// 4. No route between disconnected jurisdictions
// ---------------------------------------------------------------------------

#[test]
fn no_route_between_disconnected() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("SG-SGFZ", "JP-TKYO", 40, 86400));

    assert!(bridge
        .find_route(&jid("PK-RSEZ"), &jid("SG-SGFZ"))
        .is_none());
    assert!(bridge
        .find_route(&jid("AE-DIFC"), &jid("JP-TKYO"))
        .is_none());
}

// ---------------------------------------------------------------------------
// 5. Same source and target returns None
// ---------------------------------------------------------------------------

#[test]
fn same_source_and_target_returns_none() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    assert!(bridge
        .find_route(&jid("PK-RSEZ"), &jid("PK-RSEZ"))
        .is_none());
}

// ---------------------------------------------------------------------------
// 6. Reachability map from a source jurisdiction
// ---------------------------------------------------------------------------

#[test]
fn reachable_from_source_jurisdiction() {
    let mut bridge = CorridorBridge::new();
    bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
    bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));

    let reachable = bridge.reachable_from(&jid("PK-RSEZ"));
    assert_eq!(reachable.get("PK-RSEZ"), Some(&0));
    assert_eq!(reachable.get("AE-DIFC"), Some(&50));
    assert_eq!(reachable.get("GB-LNDN"), Some(&80));
}
