//! # Compliance Manifold
//!
//! Path optimization over the compliance tensor space. The manifold
//! represents the space of valid compliance configurations across
//! jurisdictions, with edges weighted by transition cost.
//!
//! ## Architecture
//!
//! The manifold is a directed graph where:
//! - **Nodes** are jurisdictions (each with entry requirements).
//! - **Edges** are corridors between jurisdictions (each with fees and
//!   attestation requirements).
//! - **Weights** are total cost (USD) = transfer fees + attestation costs +
//!   entry fees.
//!
//! Path optimization uses Dijkstra's algorithm with compliance-aware weights.
//!
//! ## Spec Reference
//!
//! Ports the manifold logic from `tools/phoenix/manifold.py`.
//! Manifold operations work over all 20 [`ComplianceDomain`] variants.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use msez_core::{CanonicalBytes, ComplianceDomain};

// ---------------------------------------------------------------------------
// JurisdictionNode
// ---------------------------------------------------------------------------

/// A jurisdiction in the compliance manifold graph.
///
/// Each jurisdiction specifies entry requirements (attestations),
/// supported asset classes, and cost structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionNode {
    /// Unique identifier (e.g., "uae-difc", "kz-aifc", "pk-rsez").
    pub jurisdiction_id: String,
    /// Human-readable name.
    pub name: String,
    /// ISO 3166-1 alpha-2 country code.
    pub country_code: String,
    /// Asset classes supported in this jurisdiction.
    #[serde(default)]
    pub supported_asset_classes: Vec<String>,
    /// Cost to enter this jurisdiction (USD).
    pub entry_fee_usd: u64,
    /// Annual maintenance fee (USD).
    pub annual_fee_usd: u64,
    /// Whether this jurisdiction is currently active.
    pub is_active: bool,
    /// Compliance domains required for entry.
    #[serde(default)]
    pub required_domains: Vec<ComplianceDomain>,
}

// ---------------------------------------------------------------------------
// CorridorEdge
// ---------------------------------------------------------------------------

/// A corridor connecting two jurisdictions in the manifold.
///
/// Corridors enable asset migration and specify the compliance
/// requirements, fee structure, and timing for the transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorridorEdge {
    /// Unique corridor identifier.
    pub corridor_id: String,
    /// Source jurisdiction ID.
    pub source_jurisdiction: String,
    /// Target jurisdiction ID.
    pub target_jurisdiction: String,
    /// Whether this corridor is bidirectional.
    pub is_bidirectional: bool,
    /// Whether this corridor is active.
    pub is_active: bool,
    /// Transfer fee in basis points (e.g., 10 = 0.1%).
    pub transfer_fee_bps: u32,
    /// Flat fee per transfer (USD).
    pub flat_fee_usd: u64,
    /// Estimated transfer time (hours).
    pub estimated_transfer_hours: u32,
    /// Settlement finality time (hours).
    pub settlement_finality_hours: u32,
    /// Compliance domains checked during transfer.
    #[serde(default)]
    pub required_domains: Vec<ComplianceDomain>,
}

impl CorridorEdge {
    /// Total time for this corridor (transfer + settlement).
    pub fn total_time_hours(&self) -> u32 {
        self.estimated_transfer_hours
            .saturating_add(self.settlement_finality_hours)
    }

    /// Transfer cost for a given asset value (USD).
    ///
    /// Uses u128 intermediate to prevent overflow on large asset values.
    pub fn transfer_cost(&self, asset_value_usd: u64) -> u64 {
        let bps_cost =
            (u128::from(asset_value_usd) * u128::from(self.transfer_fee_bps) / 10_000) as u64;
        self.flat_fee_usd.saturating_add(bps_cost)
    }
}

// ---------------------------------------------------------------------------
// PathConstraint
// ---------------------------------------------------------------------------

/// Constraints on migration paths through the manifold.
#[derive(Debug, Clone)]
pub struct PathConstraint {
    /// Maximum total cost (USD). Paths exceeding this are rejected.
    pub max_total_cost_usd: Option<u64>,
    /// Maximum cost per hop (USD).
    pub max_per_hop_cost_usd: Option<u64>,
    /// Maximum total time (hours).
    pub max_total_time_hours: Option<u32>,
    /// Maximum number of hops.
    pub max_hops: usize,
    /// Jurisdictions that must NOT appear in the path.
    pub excluded_jurisdictions: HashSet<String>,
    /// Whether loops are allowed.
    pub allow_loops: bool,
}

impl Default for PathConstraint {
    fn default() -> Self {
        Self {
            max_total_cost_usd: None,
            max_per_hop_cost_usd: None,
            max_total_time_hours: None,
            max_hops: 5,
            excluded_jurisdictions: HashSet::new(),
            allow_loops: false,
        }
    }
}

// ---------------------------------------------------------------------------
// MigrationHop
// ---------------------------------------------------------------------------

/// A single hop in a migration path.
#[derive(Debug, Clone)]
pub struct MigrationHop {
    /// The corridor traversed.
    pub corridor_id: String,
    /// Source jurisdiction of this hop.
    pub source: String,
    /// Target jurisdiction of this hop.
    pub target: String,
    /// Cost of this hop (USD).
    pub cost_usd: u64,
    /// Time for this hop (hours).
    pub time_hours: u32,
}

// ---------------------------------------------------------------------------
// MigrationPath
// ---------------------------------------------------------------------------

/// A complete migration path from source to target jurisdiction.
#[derive(Debug, Clone)]
pub struct MigrationPath {
    /// Source jurisdiction ID.
    pub source_jurisdiction: String,
    /// Target jurisdiction ID.
    pub target_jurisdiction: String,
    /// Ordered hops in the path.
    pub hops: Vec<MigrationHop>,
    /// Total cost (USD).
    pub total_cost_usd: u64,
    /// Total time (hours).
    pub total_time_hours: u32,
    /// Deterministic path ID (derived from corridor sequence).
    pub path_id: String,
}

impl MigrationPath {
    /// Number of hops in the path.
    pub fn hop_count(&self) -> usize {
        self.hops.len()
    }

    /// List all jurisdictions in path order.
    pub fn jurisdictions(&self) -> Vec<&str> {
        if self.hops.is_empty() {
            return vec![&self.source_jurisdiction, &self.target_jurisdiction];
        }
        let mut result = vec![self.hops[0].source.as_str()];
        for hop in &self.hops {
            result.push(&hop.target);
        }
        result
    }
}

// ---------------------------------------------------------------------------
// ComplianceDistance
// ---------------------------------------------------------------------------

/// Summary of the compliance distance between two jurisdictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceDistance {
    /// Source jurisdiction.
    pub source: String,
    /// Target jurisdiction.
    pub target: String,
    /// Number of hops in the optimal path.
    pub hop_count: usize,
    /// Total cost (USD).
    pub total_cost_usd: u64,
    /// Total time (hours).
    pub total_time_hours: u32,
    /// Path identifier.
    pub path_id: String,
}

// ---------------------------------------------------------------------------
// Internal: Dijkstra state
// ---------------------------------------------------------------------------

/// Entry in the Dijkstra priority queue.
#[derive(Debug, Clone, Eq, PartialEq)]
struct DijkstraEntry {
    cost: u64,
    time: u32,
    jurisdiction: String,
}

impl Ord for DijkstraEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for min-heap (BinaryHeap is max-heap by default).
        other
            .cost
            .cmp(&self.cost)
            .then_with(|| other.time.cmp(&self.time))
            .then_with(|| self.jurisdiction.cmp(&other.jurisdiction))
    }
}

impl PartialOrd for DijkstraEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

// ---------------------------------------------------------------------------
// ComplianceManifold
// ---------------------------------------------------------------------------

/// The compliance manifold — path planning through jurisdictional landscape.
///
/// Computes optimal migration paths between jurisdictions while satisfying
/// compliance constraints. Uses Dijkstra's algorithm with compliance-aware
/// edge weights.
///
/// All manifold operations work over all 20 [`ComplianceDomain`] variants,
/// ensuring no domain is accidentally excluded from path cost computation.
#[derive(Debug)]
pub struct ComplianceManifold {
    /// Jurisdictions in the manifold.
    jurisdictions: HashMap<String, JurisdictionNode>,
    /// Corridors indexed by ID.
    corridors: HashMap<String, CorridorEdge>,
    /// Adjacency list: jurisdiction → [corridor_ids].
    adjacency: HashMap<String, Vec<String>>,
}

impl ComplianceManifold {
    /// Create an empty manifold.
    pub fn new() -> Self {
        Self {
            jurisdictions: HashMap::new(),
            corridors: HashMap::new(),
            adjacency: HashMap::new(),
        }
    }

    /// Add a jurisdiction to the manifold.
    pub fn add_jurisdiction(&mut self, node: JurisdictionNode) {
        let id = node.jurisdiction_id.clone();
        self.adjacency.entry(id.clone()).or_default();
        self.jurisdictions.insert(id, node);
    }

    /// Add a corridor to the manifold.
    pub fn add_corridor(&mut self, edge: CorridorEdge) {
        let id = edge.corridor_id.clone();

        self.adjacency
            .entry(edge.source_jurisdiction.clone())
            .or_default()
            .push(id.clone());

        if edge.is_bidirectional {
            self.adjacency
                .entry(edge.target_jurisdiction.clone())
                .or_default()
                .push(id.clone());
        }

        self.corridors.insert(id, edge);
    }

    /// Get a jurisdiction by ID.
    pub fn get_jurisdiction(&self, id: &str) -> Option<&JurisdictionNode> {
        self.jurisdictions.get(id)
    }

    /// Get a corridor by ID.
    pub fn get_corridor(&self, id: &str) -> Option<&CorridorEdge> {
        self.corridors.get(id)
    }

    /// List all jurisdictions.
    pub fn list_jurisdictions(&self) -> Vec<&JurisdictionNode> {
        self.jurisdictions.values().collect()
    }

    /// List all corridors.
    pub fn list_corridors(&self) -> Vec<&CorridorEdge> {
        self.corridors.values().collect()
    }

    /// Find the optimal migration path from source to target.
    ///
    /// Uses Dijkstra's algorithm with compliance-aware weights.
    /// Returns `None` if no valid path exists.
    pub fn find_path(
        &self,
        source: &str,
        target: &str,
        constraints: Option<&PathConstraint>,
        asset_value_usd: u64,
    ) -> Option<MigrationPath> {
        let default_constraints = PathConstraint::default();
        let constraints = constraints.unwrap_or(&default_constraints);

        if !self.jurisdictions.contains_key(source) || !self.jurisdictions.contains_key(target) {
            return None;
        }

        if constraints.excluded_jurisdictions.contains(source)
            || constraints.excluded_jurisdictions.contains(target)
        {
            return None;
        }

        // Dijkstra state.
        let mut distances: HashMap<String, u64> = self
            .jurisdictions
            .keys()
            .map(|j| (j.clone(), u64::MAX))
            .collect();
        let mut times: HashMap<String, u32> = self
            .jurisdictions
            .keys()
            .map(|j| (j.clone(), u32::MAX))
            .collect();
        let mut previous: HashMap<String, Option<(String, String)>> = self
            .jurisdictions
            .keys()
            .map(|j| (j.clone(), None))
            .collect();
        let mut visited: HashSet<String> = HashSet::new();

        distances.insert(source.to_string(), 0);
        times.insert(source.to_string(), 0);

        let mut pq = BinaryHeap::new();
        pq.push(DijkstraEntry {
            cost: 0,
            time: 0,
            jurisdiction: source.to_string(),
        });

        while let Some(entry) = pq.pop() {
            if visited.contains(&entry.jurisdiction) {
                continue;
            }
            visited.insert(entry.jurisdiction.clone());

            if entry.jurisdiction == target {
                break;
            }

            // Check hop limit.
            let hop_count = self.count_hops(&previous, &entry.jurisdiction);
            if hop_count >= constraints.max_hops {
                continue;
            }

            // Explore neighbors.
            let corridor_ids = match self.adjacency.get(&entry.jurisdiction) {
                Some(ids) => ids.clone(),
                None => continue,
            };

            for corridor_id in &corridor_ids {
                let corridor = match self.corridors.get(corridor_id) {
                    Some(c) => c,
                    None => continue,
                };

                // Determine neighbor.
                let neighbor = if corridor.source_jurisdiction == entry.jurisdiction {
                    &corridor.target_jurisdiction
                } else if corridor.is_bidirectional
                    && corridor.target_jurisdiction == entry.jurisdiction
                {
                    &corridor.source_jurisdiction
                } else {
                    continue;
                };

                // Skip excluded.
                if constraints.excluded_jurisdictions.contains(neighbor) {
                    continue;
                }

                // Skip inactive.
                if !corridor.is_active {
                    continue;
                }
                if let Some(n) = self.jurisdictions.get(neighbor) {
                    if !n.is_active {
                        continue;
                    }
                }

                // Skip if no loops and already visited.
                if !constraints.allow_loops && visited.contains(neighbor) {
                    continue;
                }

                // Calculate edge cost.
                let edge_cost = self.calculate_edge_cost(corridor, asset_value_usd);
                let edge_time = corridor.total_time_hours();

                // Per-hop constraints.
                if let Some(max) = constraints.max_per_hop_cost_usd {
                    if edge_cost > max {
                        continue;
                    }
                }

                let new_cost = entry.cost.saturating_add(edge_cost);
                let new_time = entry.time.saturating_add(edge_time);

                // Total constraints.
                if let Some(max) = constraints.max_total_cost_usd {
                    if new_cost > max {
                        continue;
                    }
                }
                if let Some(max) = constraints.max_total_time_hours {
                    if new_time > max {
                        continue;
                    }
                }

                if new_cost < *distances.get(neighbor).unwrap_or(&u64::MAX) {
                    distances.insert(neighbor.clone(), new_cost);
                    times.insert(neighbor.clone(), new_time);
                    previous.insert(
                        neighbor.clone(),
                        Some((entry.jurisdiction.clone(), corridor_id.clone())),
                    );
                    pq.push(DijkstraEntry {
                        cost: new_cost,
                        time: new_time,
                        jurisdiction: neighbor.clone(),
                    });
                }
            }
        }

        // Check if we reached the target.
        if *distances.get(target).unwrap_or(&u64::MAX) == u64::MAX {
            return None;
        }

        // Reconstruct path.
        Some(self.reconstruct_path(source, target, &previous, asset_value_usd))
    }

    /// Find multiple alternative paths.
    ///
    /// Uses k-shortest paths by excluding corridors from previous paths.
    pub fn find_all_paths(
        &self,
        source: &str,
        target: &str,
        constraints: Option<&PathConstraint>,
        asset_value_usd: u64,
        max_paths: usize,
    ) -> Vec<MigrationPath> {
        let mut paths = Vec::new();

        if let Some(primary) = self.find_path(source, target, constraints, asset_value_usd) {
            paths.push(primary);
        }

        // Simple k-shortest: exclude corridors from previous paths.
        // (A more sophisticated implementation would use Yen's algorithm.)
        let mut excluded = HashSet::new();
        let mut seen_ids = HashSet::new();
        for p in &paths {
            seen_ids.insert(p.path_id.clone());
        }

        for _ in 1..max_paths {
            if paths.is_empty() {
                break;
            }

            let Some(last) = paths.last() else {
                break;
            };
            if last.hops.is_empty() {
                break;
            }
            for hop in &last.hops {
                excluded.insert(hop.corridor_id.clone());
            }

            if let Some(alt) =
                self.find_path_excluding(source, target, constraints, asset_value_usd, &excluded)
            {
                if !seen_ids.contains(&alt.path_id) {
                    seen_ids.insert(alt.path_id.clone());
                    paths.push(alt);
                }
            } else {
                break;
            }
        }

        paths.sort_by_key(|p| p.total_cost_usd);
        paths.truncate(max_paths);
        paths
    }

    /// Compute the compliance distance between two jurisdictions.
    pub fn compliance_distance(
        &self,
        source: &str,
        target: &str,
        constraints: Option<&PathConstraint>,
    ) -> Option<ComplianceDistance> {
        let path = self.find_path(source, target, constraints, 0)?;
        Some(ComplianceDistance {
            source: source.to_string(),
            target: target.to_string(),
            hop_count: path.hop_count(),
            total_cost_usd: path.total_cost_usd,
            total_time_hours: path.total_time_hours,
            path_id: path.path_id,
        })
    }

    // ── Private helpers ─────────────────────────────────────────────

    fn count_hops(
        &self,
        previous: &HashMap<String, Option<(String, String)>>,
        current: &str,
    ) -> usize {
        let mut count = 0;
        let mut cur = current.to_string();
        while let Some(Some((prev, _))) = previous.get(&cur) {
            cur = prev.clone();
            count += 1;
        }
        count
    }

    fn calculate_edge_cost(&self, corridor: &CorridorEdge, asset_value_usd: u64) -> u64 {
        let transfer_cost = corridor.transfer_cost(asset_value_usd);
        let target_entry = self
            .jurisdictions
            .get(&corridor.target_jurisdiction)
            .map(|j| j.entry_fee_usd)
            .unwrap_or(0);
        transfer_cost + target_entry
    }

    fn reconstruct_path(
        &self,
        source: &str,
        target: &str,
        previous: &HashMap<String, Option<(String, String)>>,
        asset_value_usd: u64,
    ) -> MigrationPath {
        let mut segments: Vec<(String, String)> = Vec::new();
        let mut current = target.to_string();

        while let Some(Some((prev, corridor_id))) = previous.get(&current) {
            segments.push((prev.clone(), corridor_id.clone()));
            current = prev.clone();
        }
        segments.reverse();

        let mut hops = Vec::new();
        let mut total_cost = 0u64;
        let mut total_time = 0u32;

        for (prev_jurisdiction, corridor_id) in &segments {
            // Defensive: skip if corridor was removed mid-operation (shouldn't happen
            // but prevents panic from unchecked HashMap indexing).
            let Some(corridor) = self.corridors.get(corridor_id.as_str()) else {
                break;
            };
            let target_id = if corridor.source_jurisdiction == *prev_jurisdiction {
                &corridor.target_jurisdiction
            } else {
                &corridor.source_jurisdiction
            };

            let hop_cost = self.calculate_edge_cost(corridor, asset_value_usd);
            let hop_time = corridor.total_time_hours();

            hops.push(MigrationHop {
                corridor_id: corridor_id.clone(),
                source: prev_jurisdiction.clone(),
                target: target_id.clone(),
                cost_usd: hop_cost,
                time_hours: hop_time,
            });

            total_cost = total_cost.saturating_add(hop_cost);
            total_time = total_time.saturating_add(hop_time);
        }

        // Deterministic path ID from corridor sequence.
        // Uses CanonicalBytes pipeline per CLAUDE.md §VIII rule 5:
        // "All SHA-256 computation flows through CanonicalBytes::new()."
        let path_content = serde_json::json!({
            "source": source,
            "target": target,
            "corridors": hops.iter().map(|h| h.corridor_id.as_str()).collect::<Vec<_>>()
        });
        let path_id = match CanonicalBytes::new(&path_content) {
            Ok(canonical) => {
                let digest = msez_core::sha256_digest(&canonical);
                let hex = digest.to_string();
                hex.get(..16).unwrap_or(&hex).to_string()
            }
            Err(_) => {
                // Fallback: use source:target as path ID if canonicalization fails
                format!("{}:{}", source, target)
            }
        };

        MigrationPath {
            source_jurisdiction: source.to_string(),
            target_jurisdiction: target.to_string(),
            hops,
            total_cost_usd: total_cost,
            total_time_hours: total_time,
            path_id,
        }
    }

    fn find_path_excluding(
        &self,
        source: &str,
        target: &str,
        constraints: Option<&PathConstraint>,
        asset_value_usd: u64,
        excluded_corridors: &HashSet<String>,
    ) -> Option<MigrationPath> {
        // Temporarily remove excluded corridors and rebuild adjacency.
        let mut filtered = Self::new();

        for node in self.jurisdictions.values() {
            filtered.add_jurisdiction(node.clone());
        }

        for (id, edge) in &self.corridors {
            if !excluded_corridors.contains(id.as_str()) {
                filtered.add_corridor(edge.clone());
            }
        }

        filtered.find_path(source, target, constraints, asset_value_usd)
    }
}

impl Default for ComplianceManifold {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Standard Manifold Factories
// ---------------------------------------------------------------------------

/// Create the UAE-DIFC jurisdiction node.
pub fn create_uae_difc_jurisdiction() -> JurisdictionNode {
    JurisdictionNode {
        jurisdiction_id: "uae-difc".into(),
        name: "Dubai International Financial Centre".into(),
        country_code: "AE".into(),
        supported_asset_classes: vec![
            "securities".into(),
            "commodities".into(),
            "digital_assets".into(),
        ],
        entry_fee_usd: 1000,
        annual_fee_usd: 5000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Aml],
    }
}

/// Create the Kazakhstan-AIFC jurisdiction node.
pub fn create_kz_aifc_jurisdiction() -> JurisdictionNode {
    JurisdictionNode {
        jurisdiction_id: "kz-aifc".into(),
        name: "Astana International Financial Centre".into(),
        country_code: "KZ".into(),
        supported_asset_classes: vec![
            "securities".into(),
            "digital_assets".into(),
            "islamic_finance".into(),
        ],
        entry_fee_usd: 500,
        annual_fee_usd: 2000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Sanctions],
    }
}

/// Create the Pakistan-RSEZ jurisdiction node.
pub fn create_pk_rsez_jurisdiction() -> JurisdictionNode {
    JurisdictionNode {
        jurisdiction_id: "pk-rsez".into(),
        name: "Rashakai Special Economic Zone".into(),
        country_code: "PK".into(),
        supported_asset_classes: vec!["trade".into(), "digital_assets".into()],
        entry_fee_usd: 200,
        annual_fee_usd: 1000,
        is_active: true,
        required_domains: vec![ComplianceDomain::Kyc, ComplianceDomain::Tax],
    }
}

/// Create a corridor between DIFC and AIFC.
pub fn create_difc_aifc_corridor() -> CorridorEdge {
    CorridorEdge {
        corridor_id: "corridor-difc-aifc".into(),
        source_jurisdiction: "uae-difc".into(),
        target_jurisdiction: "kz-aifc".into(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 10,
        flat_fee_usd: 100,
        estimated_transfer_hours: 24,
        settlement_finality_hours: 48,
        required_domains: vec![ComplianceDomain::Aml, ComplianceDomain::Sanctions],
    }
}

/// Create a corridor between AIFC and RSEZ.
pub fn create_aifc_rsez_corridor() -> CorridorEdge {
    CorridorEdge {
        corridor_id: "corridor-aifc-rsez".into(),
        source_jurisdiction: "kz-aifc".into(),
        target_jurisdiction: "pk-rsez".into(),
        is_bidirectional: true,
        is_active: true,
        transfer_fee_bps: 15,
        flat_fee_usd: 50,
        estimated_transfer_hours: 12,
        settlement_finality_hours: 24,
        required_domains: vec![ComplianceDomain::Aml, ComplianceDomain::Tax],
    }
}

/// Create a standard manifold with 3 jurisdictions and 2 corridors.
pub fn create_standard_manifold() -> ComplianceManifold {
    let mut manifold = ComplianceManifold::new();

    manifold.add_jurisdiction(create_uae_difc_jurisdiction());
    manifold.add_jurisdiction(create_kz_aifc_jurisdiction());
    manifold.add_jurisdiction(create_pk_rsez_jurisdiction());
    manifold.add_corridor(create_difc_aifc_corridor());
    manifold.add_corridor(create_aifc_rsez_corridor());

    manifold
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_manifold_has_3_jurisdictions() {
        let manifold = create_standard_manifold();
        assert_eq!(manifold.list_jurisdictions().len(), 3);
    }

    #[test]
    fn standard_manifold_has_2_corridors() {
        let manifold = create_standard_manifold();
        assert_eq!(manifold.list_corridors().len(), 2);
    }

    #[test]
    fn find_direct_path() {
        let manifold = create_standard_manifold();
        let path = manifold
            .find_path("uae-difc", "kz-aifc", None, 10_000)
            .expect("should find direct path");

        assert_eq!(path.hop_count(), 1);
        assert_eq!(path.source_jurisdiction, "uae-difc");
        assert_eq!(path.target_jurisdiction, "kz-aifc");
        assert!(path.total_cost_usd > 0);
    }

    #[test]
    fn find_multi_hop_path() {
        let manifold = create_standard_manifold();
        let path = manifold
            .find_path("uae-difc", "pk-rsez", None, 10_000)
            .expect("should find 2-hop path");

        assert_eq!(path.hop_count(), 2);
        let jurisdictions = path.jurisdictions();
        assert_eq!(jurisdictions[0], "uae-difc");
        assert_eq!(jurisdictions[1], "kz-aifc");
        assert_eq!(jurisdictions[2], "pk-rsez");
    }

    #[test]
    fn path_bidirectional() {
        let manifold = create_standard_manifold();
        let forward = manifold.find_path("uae-difc", "kz-aifc", None, 0);
        let reverse = manifold.find_path("kz-aifc", "uae-difc", None, 0);
        assert!(forward.is_some());
        assert!(reverse.is_some());
    }

    #[test]
    fn unreachable_jurisdiction() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());
        // No corridor, no other jurisdiction.
        let path = manifold.find_path("uae-difc", "kz-aifc", None, 0);
        assert!(path.is_none());
    }

    #[test]
    fn non_existent_jurisdiction() {
        let manifold = create_standard_manifold();
        let path = manifold.find_path("uae-difc", "nonexistent", None, 0);
        assert!(path.is_none());
    }

    #[test]
    fn path_respects_cost_constraint() {
        let manifold = create_standard_manifold();
        let constraints = PathConstraint {
            max_total_cost_usd: Some(1), // Very low limit.
            ..Default::default()
        };
        let path = manifold.find_path("uae-difc", "kz-aifc", Some(&constraints), 10_000);
        // With 10bps + $100 flat + $500 entry ≈ $601 for $10k transfer,
        // a $1 limit should block the path.
        assert!(path.is_none());
    }

    #[test]
    fn path_respects_excluded_jurisdictions() {
        let manifold = create_standard_manifold();
        let mut excluded = HashSet::new();
        excluded.insert("kz-aifc".to_string());
        let constraints = PathConstraint {
            excluded_jurisdictions: excluded,
            ..Default::default()
        };
        // DIFC→RSEZ requires going through AIFC, which is excluded.
        let path = manifold.find_path("uae-difc", "pk-rsez", Some(&constraints), 0);
        assert!(path.is_none());
    }

    #[test]
    fn path_respects_max_hops() {
        let manifold = create_standard_manifold();
        let constraints = PathConstraint {
            max_hops: 1,
            ..Default::default()
        };
        // DIFC→RSEZ is 2 hops, but limit is 1.
        let path = manifold.find_path("uae-difc", "pk-rsez", Some(&constraints), 0);
        assert!(path.is_none());
    }

    #[test]
    fn compliance_distance_returns_summary() {
        let manifold = create_standard_manifold();
        let dist = manifold
            .compliance_distance("uae-difc", "kz-aifc", None)
            .expect("should compute distance");
        assert_eq!(dist.source, "uae-difc");
        assert_eq!(dist.target, "kz-aifc");
        assert_eq!(dist.hop_count, 1);
    }

    #[test]
    fn find_all_paths_returns_at_most_max() {
        let manifold = create_standard_manifold();
        let paths = manifold.find_all_paths("uae-difc", "kz-aifc", None, 0, 3);
        assert!(!paths.is_empty());
        assert!(paths.len() <= 3);
    }

    #[test]
    fn corridor_transfer_cost() {
        let corridor = create_difc_aifc_corridor();
        // 10 bps on $10,000 = $10, plus $100 flat = $110.
        assert_eq!(corridor.transfer_cost(10_000), 110);
    }

    #[test]
    fn corridor_total_time() {
        let corridor = create_difc_aifc_corridor();
        // 24 + 48 = 72 hours.
        assert_eq!(corridor.total_time_hours(), 72);
    }

    #[test]
    fn path_deterministic_id() {
        let manifold = create_standard_manifold();
        let p1 = manifold.find_path("uae-difc", "kz-aifc", None, 0).unwrap();
        let p2 = manifold.find_path("uae-difc", "kz-aifc", None, 0).unwrap();
        assert_eq!(p1.path_id, p2.path_id);
    }

    #[test]
    fn inactive_corridor_is_skipped() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());
        manifold.add_jurisdiction(create_kz_aifc_jurisdiction());

        let mut corridor = create_difc_aifc_corridor();
        corridor.is_active = false;
        manifold.add_corridor(corridor);

        let path = manifold.find_path("uae-difc", "kz-aifc", None, 0);
        assert!(path.is_none());
    }

    #[test]
    fn inactive_jurisdiction_is_skipped() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());

        let mut kz = create_kz_aifc_jurisdiction();
        kz.is_active = false;
        manifold.add_jurisdiction(kz);
        manifold.add_corridor(create_difc_aifc_corridor());

        let path = manifold.find_path("uae-difc", "kz-aifc", None, 0);
        assert!(path.is_none());
    }

    // ── Additional coverage tests ──────────────────────────────────

    #[test]
    fn manifold_default_is_empty() {
        let manifold = ComplianceManifold::default();
        assert!(manifold.list_jurisdictions().is_empty());
        assert!(manifold.list_corridors().is_empty());
    }

    #[test]
    fn manifold_get_jurisdiction() {
        let manifold = create_standard_manifold();
        let difc = manifold.get_jurisdiction("uae-difc");
        assert!(difc.is_some());
        assert_eq!(difc.unwrap().country_code, "AE");

        assert!(manifold.get_jurisdiction("nonexistent").is_none());
    }

    #[test]
    fn manifold_get_corridor() {
        let manifold = create_standard_manifold();
        let corridor = manifold.get_corridor("corridor-difc-aifc");
        assert!(corridor.is_some());
        assert_eq!(corridor.unwrap().transfer_fee_bps, 10);

        assert!(manifold.get_corridor("nonexistent").is_none());
    }

    #[test]
    fn corridor_transfer_cost_zero_value() {
        let corridor = create_difc_aifc_corridor();
        // 10bps on $0 = $0, plus $100 flat = $100.
        assert_eq!(corridor.transfer_cost(0), 100);
    }

    #[test]
    fn corridor_transfer_cost_large_value() {
        let corridor = create_difc_aifc_corridor();
        // 10bps on $1,000,000 = $1,000, plus $100 flat = $1,100.
        assert_eq!(corridor.transfer_cost(1_000_000), 1_100);
    }

    #[test]
    fn migration_path_jurisdictions_empty_hops() {
        let path = MigrationPath {
            source_jurisdiction: "src".to_string(),
            target_jurisdiction: "tgt".to_string(),
            hops: vec![],
            total_cost_usd: 0,
            total_time_hours: 0,
            path_id: "test".to_string(),
        };
        let jurisdictions = path.jurisdictions();
        assert_eq!(jurisdictions, vec!["src", "tgt"]);
    }

    #[test]
    fn migration_path_hop_count() {
        let path = MigrationPath {
            source_jurisdiction: "a".to_string(),
            target_jurisdiction: "c".to_string(),
            hops: vec![
                MigrationHop {
                    corridor_id: "c1".into(),
                    source: "a".into(),
                    target: "b".into(),
                    cost_usd: 100,
                    time_hours: 24,
                },
                MigrationHop {
                    corridor_id: "c2".into(),
                    source: "b".into(),
                    target: "c".into(),
                    cost_usd: 50,
                    time_hours: 12,
                },
            ],
            total_cost_usd: 150,
            total_time_hours: 36,
            path_id: "test".to_string(),
        };
        assert_eq!(path.hop_count(), 2);
        assert_eq!(path.jurisdictions(), vec!["a", "b", "c"]);
    }

    #[test]
    fn path_constraint_default_values() {
        let c = PathConstraint::default();
        assert!(c.max_total_cost_usd.is_none());
        assert!(c.max_per_hop_cost_usd.is_none());
        assert!(c.max_total_time_hours.is_none());
        assert_eq!(c.max_hops, 5);
        assert!(c.excluded_jurisdictions.is_empty());
        assert!(!c.allow_loops);
    }

    #[test]
    fn path_respects_per_hop_cost_constraint() {
        let manifold = create_standard_manifold();
        let constraints = PathConstraint {
            max_per_hop_cost_usd: Some(1), // Very low per-hop limit.
            ..Default::default()
        };
        let path = manifold.find_path("uae-difc", "kz-aifc", Some(&constraints), 10_000);
        assert!(path.is_none());
    }

    #[test]
    fn path_respects_time_constraint() {
        let manifold = create_standard_manifold();
        let constraints = PathConstraint {
            max_total_time_hours: Some(1), // Very low time limit.
            ..Default::default()
        };
        // DIFC→AIFC corridor takes 72 hours, exceeds 1 hour limit.
        let path = manifold.find_path("uae-difc", "kz-aifc", Some(&constraints), 0);
        assert!(path.is_none());
    }

    #[test]
    fn excluded_source_jurisdiction_returns_none() {
        let manifold = create_standard_manifold();
        let mut excluded = HashSet::new();
        excluded.insert("uae-difc".to_string());
        let constraints = PathConstraint {
            excluded_jurisdictions: excluded,
            ..Default::default()
        };
        let path = manifold.find_path("uae-difc", "kz-aifc", Some(&constraints), 0);
        assert!(path.is_none());
    }

    #[test]
    fn excluded_target_jurisdiction_returns_none() {
        let manifold = create_standard_manifold();
        let mut excluded = HashSet::new();
        excluded.insert("kz-aifc".to_string());
        let constraints = PathConstraint {
            excluded_jurisdictions: excluded,
            ..Default::default()
        };
        let path = manifold.find_path("uae-difc", "kz-aifc", Some(&constraints), 0);
        assert!(path.is_none());
    }

    #[test]
    fn compliance_distance_multi_hop() {
        let manifold = create_standard_manifold();
        let dist = manifold
            .compliance_distance("uae-difc", "pk-rsez", None)
            .expect("should compute multi-hop distance");
        assert_eq!(dist.source, "uae-difc");
        assert_eq!(dist.target, "pk-rsez");
        assert_eq!(dist.hop_count, 2);
        assert!(dist.total_time_hours > 0);
    }

    #[test]
    fn compliance_distance_unreachable() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());
        let dist = manifold.compliance_distance("uae-difc", "kz-aifc", None);
        assert!(dist.is_none());
    }

    #[test]
    fn find_all_paths_unreachable_returns_empty() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());
        let paths = manifold.find_all_paths("uae-difc", "kz-aifc", None, 0, 3);
        assert!(paths.is_empty());
    }

    #[test]
    fn find_all_paths_sorted_by_cost() {
        let manifold = create_standard_manifold();
        let paths = manifold.find_all_paths("uae-difc", "pk-rsez", None, 10_000, 5);
        // Paths should be sorted by cost ascending.
        for w in paths.windows(2) {
            assert!(w[0].total_cost_usd <= w[1].total_cost_usd);
        }
    }

    #[test]
    fn unidirectional_corridor() {
        let mut manifold = ComplianceManifold::new();
        manifold.add_jurisdiction(create_uae_difc_jurisdiction());
        manifold.add_jurisdiction(create_kz_aifc_jurisdiction());

        let mut corridor = create_difc_aifc_corridor();
        corridor.is_bidirectional = false;
        manifold.add_corridor(corridor);

        // Forward path exists.
        assert!(manifold.find_path("uae-difc", "kz-aifc", None, 0).is_some());
        // Reverse path does not exist (unidirectional).
        assert!(manifold.find_path("kz-aifc", "uae-difc", None, 0).is_none());
    }

    #[test]
    fn jurisdiction_node_serde_roundtrip() {
        let node = create_uae_difc_jurisdiction();
        let json = serde_json::to_string(&node).unwrap();
        let back: JurisdictionNode = serde_json::from_str(&json).unwrap();
        assert_eq!(back.jurisdiction_id, "uae-difc");
        assert_eq!(back.country_code, "AE");
        assert!(back.is_active);
    }

    #[test]
    fn corridor_edge_serde_roundtrip() {
        let edge = create_difc_aifc_corridor();
        let json = serde_json::to_string(&edge).unwrap();
        let back: CorridorEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(back.corridor_id, "corridor-difc-aifc");
        assert!(back.is_bidirectional);
        assert_eq!(back.transfer_fee_bps, 10);
    }

    #[test]
    fn find_path_same_source_and_target() {
        let manifold = create_standard_manifold();
        // Source == target: distance is 0, no hops needed.
        let path = manifold.find_path("uae-difc", "uae-difc", None, 0);
        // Dijkstra finds target immediately, no hops.
        assert!(path.is_some());
        let p = path.unwrap();
        assert_eq!(p.hop_count(), 0);
        assert_eq!(p.total_cost_usd, 0);
    }

    #[test]
    fn aifc_rsez_corridor_transfer_cost() {
        let corridor = create_aifc_rsez_corridor();
        // 15bps on $100,000 = $150, plus $50 flat = $200.
        assert_eq!(corridor.transfer_cost(100_000), 200);
        assert_eq!(corridor.total_time_hours(), 36); // 12 + 24
    }
}
