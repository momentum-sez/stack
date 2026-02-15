const {
  chapterHeading, h2,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter20() {
  return [
    chapterHeading("Chapter 20: Compliance Manifold"),

    // --- 20.1 Manifold Definition ---
    h2("20.1 Manifold Definition"),
    definition(
      "Definition 20.1 (Compliance Manifold).",
      "The compliance manifold M is a continuous surface over jurisdictional coordinates where height represents compliance burden: M: R^n \u2192 R, where n is the number of jurisdictional dimensions."
    ),
    p("The manifold extends the discrete compliance tensor into a continuous analytical surface. While the tensor tracks individual (asset, jurisdiction, domain) compliance states, the manifold provides a global view of compliance cost across the entire jurisdictional space. This enables path optimization for cross-border operations: rather than evaluating compliance pairwise for each hop, the manifold provides gradient information that directs migration and corridor routing toward paths of minimal compliance overhead."),
    ...codeBlock(
      "/// The compliance manifold: continuous surface over jurisdictional coordinates.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ComplianceManifold {\n" +
      "    pub jurisdictions: Vec<JurisdictionId>,\n" +
      "    pub dimensions: usize,\n" +
      "    pub samples: HashMap<Vec<f64>, f64>,\n" +
      "    pub interpolation: InterpolationMethod,\n" +
      "}\n" +
      "\n" +
      "impl ComplianceManifold {\n" +
      "    /// Find the optimal path between two jurisdictional coordinates\n" +
      "    /// minimizing total compliance burden.\n" +
      "    pub fn optimal_path(\n" +
      "        &self,\n" +
      "        from: &JurisdictionId,\n" +
      "        to: &JurisdictionId,\n" +
      "        waypoints: &[JurisdictionId],\n" +
      "    ) -> Result<MigrationPath, ManifoldError> {\n" +
      "        dijkstra::shortest_path(&self.graph(), from, to, waypoints)\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 20.2 Migration Path Optimization ---
    h2("20.2 Migration Path Optimization"),
    p("Path cost is computed as the line integral of compliance burden along the path: Cost(P) = \u222B_P compliance_burden(x) dx. The optimal path minimizes this integral while satisfying all waypoint constraints. The optimization accounts for four cost components:"),
    table(
      ["Component", "Weight", "Description"],
      [
        ["Compliance Evaluation", "Direct tensor evaluation cost", "Number of domains requiring fresh evaluation at each hop"],
        ["Attestation Acquisition", "Time and fee to obtain required attestations", "Watcher availability, bond requirements per domain"],
        ["Settlement Latency", "Time from initiation to confirmed finality", "Corridor-specific settlement parameters and queue depth"],
        ["Regulatory Overhead", "Filing and reporting obligations triggered by transit", "Per-jurisdiction reporting thresholds and filing requirements"],
      ],
      [2400, 3200, 3760]
    ),
    spacer(),
    p("The PathRouter pre-computes manifold gradients for all active corridor pairs during quiet periods. When a migration request arrives, the router queries the pre-computed gradient cache and produces a ranked list of candidate paths within the latency budget. Cache invalidation occurs on any tensor update, regpack refresh, or corridor state transition affecting the relevant jurisdictions."),

    // --- 20.3 Attestation Gap Analysis ---
    h2("20.3 Attestation Gap Analysis"),
    definition(
      "Definition 20.3 (Attestation Gap).",
      "An attestation gap represents a compliance requirement not satisfied by current attestations. It includes the requirement specification, domain, jurisdiction, severity, blocking status, and remediation options with estimated time and cost to fill."
    ),
    p("Gap analysis runs automatically when a migration path is computed. For each hop, the manifold evaluates the destination jurisdiction's compliance requirements against the asset's current tensor state. Requirements not covered by valid attestations produce gaps. Gaps are classified as blocking (migration cannot proceed until filled) or non-blocking (migration proceeds with reduced finality level). The gap report includes remediation options: which watcher roles can provide the needed attestation, estimated time to acquisition, and associated fees. The agentic trigger system can initiate gap remediation automatically for non-blocking gaps, reducing migration latency for subsequent transfers along the same path."),
  ];
};
