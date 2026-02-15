//! # Corridor Bridge — Dijkstra Routing
//!
//! Routes transactions across multi-hop corridor graphs using
//! Dijkstra's algorithm with edge weights derived from fee basis points.
//!
//! ## Design
//!
//! The bridge maintains a directed graph where:
//! - **Nodes** are jurisdictions ([`JurisdictionId`]).
//! - **Edges** are active corridors with fee and settlement time metadata.
//! - **Weights** are fee basis points for shortest-fee routing.
//!
//! Only corridors in the `Active` state participate in routing. This is
//! enforced by the caller providing only active corridor data.
//!
//! ## Algorithm
//!
//! Standard Dijkstra with a binary heap. Edge relaxation uses fee_bps
//! as the weight. Settlement time is accumulated along the path but does
//! not affect routing priority (fee minimization is primary).
//!
//! ## Spec Reference
//!
//! Implements the corridor bridge routing protocol from `spec/40-corridors.md`.
//! Port of `tools/phoenix/bridge.py` Dijkstra routing logic.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use msez_core::{CorridorId, JurisdictionId};

/// An edge in the corridor bridge graph representing an active corridor.
///
/// Each edge connects two jurisdictions with associated fee and settlement
/// time metadata. Edges are directional — a corridor between PK-RSEZ and
/// AE-DIFC produces two edges (one in each direction) if fees differ by
/// direction, or two identical edges for symmetric corridors.
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeEdge {
    /// Source jurisdiction.
    pub from: JurisdictionId,
    /// Destination jurisdiction.
    pub to: JurisdictionId,
    /// Corridor identifier for audit trail.
    pub corridor_id: CorridorId,
    /// Fee for this hop in basis points (1 bps = 0.01%).
    pub fee_bps: u32,
    /// Estimated settlement time for this hop in seconds.
    pub settlement_time_secs: u64,
}

/// A computed route through the corridor bridge graph.
///
/// Contains the ordered list of hops from source to target, with
/// accumulated fee and settlement time totals.
#[derive(Debug, Clone, PartialEq)]
pub struct BridgeRoute {
    /// Ordered list of edges from source to target.
    pub hops: Vec<BridgeEdge>,
    /// Total fee across all hops in basis points.
    pub total_fee_bps: u64,
    /// Total estimated settlement time across all hops in seconds.
    pub total_settlement_time_secs: u64,
    /// Source jurisdiction.
    pub source: JurisdictionId,
    /// Target jurisdiction.
    pub target: JurisdictionId,
}

impl BridgeRoute {
    /// Return the number of hops in the route.
    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }

    /// Return the ordered list of jurisdictions traversed (including source and target).
    pub fn jurisdictions(&self) -> Vec<&JurisdictionId> {
        let mut result = Vec::with_capacity(self.hops.len() + 1);
        if let Some(first) = self.hops.first() {
            result.push(&first.from);
            for hop in &self.hops {
                result.push(&hop.to);
            }
        }
        result
    }
}

/// Internal node for Dijkstra priority queue.
#[derive(Debug, Clone, Eq, PartialEq)]
struct DijkstraNode {
    cost: u64,
    jurisdiction: String,
}

impl Ord for DijkstraNode {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse for min-heap behavior with BinaryHeap (which is a max-heap).
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| self.jurisdiction.cmp(&other.jurisdiction))
    }
}

impl PartialOrd for DijkstraNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A corridor bridge graph for Dijkstra-weighted routing.
///
/// Maintains an adjacency list of active corridor edges and provides
/// shortest-fee path computation between any two jurisdictions.
///
/// ## Security Invariant
///
/// Only active corridors participate in routing. Suspended, halted, or
/// deprecated corridors must not be added to the graph.
#[derive(Debug, Default)]
pub struct CorridorBridge {
    /// Adjacency list: from_jurisdiction_key -> `Vec<edge>`.
    adjacency: HashMap<String, Vec<BridgeEdge>>,
}

impl CorridorBridge {
    /// Create an empty corridor bridge graph.
    pub fn new() -> Self {
        Self {
            adjacency: HashMap::new(),
        }
    }

    /// Add an edge to the bridge graph.
    ///
    /// Each edge represents one direction of an active corridor.
    /// For bidirectional corridors, call this twice with reversed from/to.
    pub fn add_edge(&mut self, edge: BridgeEdge) {
        self.adjacency
            .entry(edge.from.as_str().to_string())
            .or_default()
            .push(edge);
    }

    /// Return the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.adjacency.values().map(|v| v.len()).sum()
    }

    /// Return the number of unique jurisdictions (nodes) in the graph.
    pub fn node_count(&self) -> usize {
        let mut nodes = std::collections::HashSet::new();
        for edges in self.adjacency.values() {
            for edge in edges {
                nodes.insert(edge.from.as_str().to_string());
                nodes.insert(edge.to.as_str().to_string());
            }
        }
        nodes.len()
    }

    /// Find the shortest-fee route between two jurisdictions using
    /// Dijkstra's algorithm.
    ///
    /// Returns `None` if no path exists between source and target.
    /// Returns `None` if source equals target (no routing needed).
    ///
    /// The algorithm minimizes total fee in basis points. Settlement time
    /// is accumulated but does not affect route selection.
    ///
    /// ## Spec Reference
    ///
    /// Port of `tools/phoenix/bridge.py` path-finding logic.
    pub fn find_route(
        &self,
        source: &JurisdictionId,
        target: &JurisdictionId,
    ) -> Option<BridgeRoute> {
        if source == target {
            return None;
        }

        let source_key = source.as_str().to_string();
        let target_key = target.as_str().to_string();

        let mut dist: HashMap<String, u64> = HashMap::new();
        let mut prev: HashMap<String, (String, usize)> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(source_key.clone(), 0);
        heap.push(DijkstraNode {
            cost: 0,
            jurisdiction: source_key.clone(),
        });

        while let Some(DijkstraNode {
            cost,
            jurisdiction: current_key,
        }) = heap.pop()
        {
            if cost > *dist.get(&current_key).unwrap_or(&u64::MAX) {
                continue;
            }

            if current_key == target_key {
                break;
            }

            if let Some(edges) = self.adjacency.get(&current_key) {
                for (edge_idx, edge) in edges.iter().enumerate() {
                    let next_key = edge.to.as_str().to_string();
                    let next_cost = cost + u64::from(edge.fee_bps);

                    if next_cost < *dist.get(&next_key).unwrap_or(&u64::MAX) {
                        dist.insert(next_key.clone(), next_cost);
                        prev.insert(next_key.clone(), (current_key.clone(), edge_idx));
                        heap.push(DijkstraNode {
                            cost: next_cost,
                            jurisdiction: next_key,
                        });
                    }
                }
            }
        }

        if !prev.contains_key(&target_key) {
            return None;
        }

        let mut hops = Vec::new();
        let mut current_key = target_key;
        while let Some((pred_key, edge_idx)) = prev.get(&current_key) {
            let edge = &self.adjacency[pred_key][*edge_idx];
            hops.push(edge.clone());
            current_key = pred_key.clone();
        }
        hops.reverse();

        let total_fee_bps: u64 = hops.iter().map(|h| u64::from(h.fee_bps)).sum();
        let total_settlement_time_secs: u64 = hops.iter().map(|h| h.settlement_time_secs).sum();

        Some(BridgeRoute {
            hops,
            total_fee_bps,
            total_settlement_time_secs,
            source: source.clone(),
            target: target.clone(),
        })
    }

    /// Find all reachable jurisdictions from a given source.
    ///
    /// Returns a map of jurisdiction key -> minimum fee (in bps) to reach it.
    /// The source node is always included with distance 0, even in an empty graph.
    /// Uses Dijkstra's algorithm over the bidirectional link graph.
    pub fn reachable_from(&self, source: &JurisdictionId) -> HashMap<String, u64> {
        let source_key = source.as_str().to_string();
        let mut dist: HashMap<String, u64> = HashMap::new();
        let mut heap = BinaryHeap::new();

        dist.insert(source_key.clone(), 0);
        heap.push(DijkstraNode {
            cost: 0,
            jurisdiction: source_key,
        });

        while let Some(DijkstraNode {
            cost,
            jurisdiction: current_key,
        }) = heap.pop()
        {
            if cost > *dist.get(&current_key).unwrap_or(&u64::MAX) {
                continue;
            }

            if let Some(edges) = self.adjacency.get(&current_key) {
                for edge in edges {
                    let next_key = edge.to.as_str().to_string();
                    let next_cost = cost + u64::from(edge.fee_bps);
                    if next_cost < *dist.get(&next_key).unwrap_or(&u64::MAX) {
                        dist.insert(next_key.clone(), next_cost);
                        heap.push(DijkstraNode {
                            cost: next_cost,
                            jurisdiction: next_key,
                        });
                    }
                }
            }
        }

        dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn sample_bridge() -> CorridorBridge {
        let mut bridge = CorridorBridge::new();
        // PK-RSEZ <-> AE-DIFC (50 bps, 1 day)
        bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
        bridge.add_edge(edge("AE-DIFC", "PK-RSEZ", 55, 86400));
        // AE-DIFC <-> GB-LNDN (30 bps, 2 days)
        bridge.add_edge(edge("AE-DIFC", "GB-LNDN", 30, 172800));
        bridge.add_edge(edge("GB-LNDN", "AE-DIFC", 35, 172800));
        // GB-LNDN <-> US-NYFC (25 bps, 1 day)
        bridge.add_edge(edge("GB-LNDN", "US-NYFC", 25, 86400));
        bridge.add_edge(edge("US-NYFC", "GB-LNDN", 25, 86400));
        // PK-RSEZ -> US-NYFC direct (expensive: 200 bps, 3 days)
        bridge.add_edge(edge("PK-RSEZ", "US-NYFC", 200, 259200));
        bridge
    }

    #[test]
    fn direct_route() {
        let bridge = sample_bridge();
        let route = bridge.find_route(&jid("PK-RSEZ"), &jid("AE-DIFC")).unwrap();
        assert_eq!(route.hop_count(), 1);
        assert_eq!(route.total_fee_bps, 50);
        assert_eq!(route.total_settlement_time_secs, 86400);
    }

    #[test]
    fn multi_hop_cheaper_than_direct() {
        let bridge = sample_bridge();
        let route = bridge.find_route(&jid("PK-RSEZ"), &jid("US-NYFC")).unwrap();
        // Multi-hop: PK-RSEZ -> AE-DIFC (50) -> GB-LNDN (30) -> US-NYFC (25) = 105 bps
        // Direct: PK-RSEZ -> US-NYFC = 200 bps
        assert_eq!(route.hop_count(), 3);
        assert_eq!(route.total_fee_bps, 105);
        assert_eq!(route.total_settlement_time_secs, 86400 + 172800 + 86400);

        let jurisdictions: Vec<&str> = route.jurisdictions().iter().map(|j| j.as_str()).collect();
        assert_eq!(
            jurisdictions,
            vec!["PK-RSEZ", "AE-DIFC", "GB-LNDN", "US-NYFC"]
        );
    }

    #[test]
    fn no_route_between_disconnected_nodes() {
        let mut bridge = CorridorBridge::new();
        bridge.add_edge(edge("PK-RSEZ", "AE-DIFC", 50, 86400));
        bridge.add_edge(edge("SG-SGFZ", "JP-TKYO", 40, 86400));

        assert!(bridge
            .find_route(&jid("PK-RSEZ"), &jid("SG-SGFZ"))
            .is_none());
    }

    #[test]
    fn same_source_and_target_returns_none() {
        let bridge = sample_bridge();
        assert!(bridge
            .find_route(&jid("PK-RSEZ"), &jid("PK-RSEZ"))
            .is_none());
    }

    #[test]
    fn nonexistent_source() {
        let bridge = sample_bridge();
        assert!(bridge
            .find_route(&jid("NOWHERE"), &jid("PK-RSEZ"))
            .is_none());
    }

    #[test]
    fn empty_graph() {
        let bridge = CorridorBridge::new();
        assert!(bridge
            .find_route(&jid("PK-RSEZ"), &jid("AE-DIFC"))
            .is_none());
        assert_eq!(bridge.edge_count(), 0);
        assert_eq!(bridge.node_count(), 0);
    }

    #[test]
    fn reachable_from_source() {
        let bridge = sample_bridge();
        let reachable = bridge.reachable_from(&jid("PK-RSEZ"));
        assert_eq!(reachable.get("PK-RSEZ"), Some(&0));
        assert_eq!(reachable.get("AE-DIFC"), Some(&50));
        assert_eq!(reachable.get("GB-LNDN"), Some(&80));
        assert_eq!(reachable.get("US-NYFC"), Some(&105));
    }

    #[test]
    fn bidirectional_routing() {
        let bridge = sample_bridge();
        let forward = bridge.find_route(&jid("PK-RSEZ"), &jid("AE-DIFC")).unwrap();
        assert_eq!(forward.total_fee_bps, 50);

        let reverse = bridge.find_route(&jid("AE-DIFC"), &jid("PK-RSEZ")).unwrap();
        assert_eq!(reverse.total_fee_bps, 55);
    }

    #[test]
    fn graph_metadata() {
        let bridge = sample_bridge();
        assert_eq!(bridge.edge_count(), 7);
        assert_eq!(bridge.node_count(), 4);
    }

    #[test]
    fn diamond_graph_picks_cheapest() {
        let mut bridge = CorridorBridge::new();
        bridge.add_edge(edge("A", "C", 10, 3600));
        bridge.add_edge(edge("C", "B", 10, 3600));
        bridge.add_edge(edge("A", "D", 5, 7200));
        bridge.add_edge(edge("D", "B", 5, 7200));

        let route = bridge.find_route(&jid("A"), &jid("B")).unwrap();
        assert_eq!(route.total_fee_bps, 10);
        assert_eq!(route.hop_count(), 2);
        let jurisdictions: Vec<&str> = route.jurisdictions().iter().map(|j| j.as_str()).collect();
        assert_eq!(jurisdictions, vec!["A", "D", "B"]);
    }
}
