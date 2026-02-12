//! # Corridor Bridge — Dijkstra Routing
//!
//! Routes transactions across multi-hop corridor graphs using
//! Dijkstra's algorithm with edge weights representing (fee, time, risk).

use msez_core::JurisdictionId;

/// A corridor bridge graph for Dijkstra-weighted routing.
#[derive(Debug)]
pub struct CorridorBridge {
    /// Adjacency list of jurisdiction connections.
    _edges: Vec<BridgeEdge>,
}

impl CorridorBridge {
    /// Create an empty corridor bridge graph.
    pub fn new() -> Self {
        Self { _edges: Vec::new() }
    }
}

impl Default for CorridorBridge {
    fn default() -> Self {
        Self::new()
    }
}

/// An edge in the corridor bridge graph.
#[derive(Debug)]
pub struct BridgeEdge {
    /// Source jurisdiction.
    pub from: JurisdictionId,
    /// Destination jurisdiction.
    pub to: JurisdictionId,
    /// Fee for this hop (in basis points).
    pub fee_bps: u32,
}
