const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter20() {
  return [
    chapterHeading("Chapter 20: Compliance Manifold"),

    // --- 20.1 Manifold Definition ---
    h2("20.1 Manifold Definition"),
    definition(
      "Definition 20.1 (Compliance Manifold).",
      "The compliance manifold M is a continuous surface over jurisdictional coordinates where height represents compliance burden: M: R^n -> R, where n is the number of jurisdictional dimensions."
    ),
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

    // --- 20.2 Migration Path Optimization ---
    h2("20.2 Migration Path Optimization"),
    definition(
      "Definition 20.2 (Migration Path).",
      "A migration path P from jurisdiction J1 to J2 is a sequence of intermediate jurisdictions " +
      "[J1, Jk1, Jk2, ..., J2] where each consecutive pair has an active corridor. The path cost " +
      "is the line integral Cost(P) = \u222B_P compliance_burden(x) dx, summing the compliance " +
      "overhead at each waypoint."
    ),
    p(
      "The optimal path minimizes this integral while satisfying all waypoint constraints. " +
      "The manifold's optimal_path method uses Dijkstra's algorithm over the corridor graph, " +
      "where edge weights are the evaluated compliance burden between each consecutive " +
      "jurisdiction pair."
    ),

    h3("20.2.1 Example: PAK \u2192 UAE \u2192 KSA Migration Path"),
    p(
      "Consider an entity migrating from Pakistan (PAK) to Saudi Arabia (KSA) via the " +
      "United Arab Emirates (UAE). Each hop traverses an active corridor with its own " +
      "compliance burden, computed from the tensor evaluation at that jurisdictional coordinate."
    ),
    table(
      ["Hop", "From", "To", "Compliance Burden", "Cumulative Cost"],
      [
        ["1", "PAK", "UAE", "0.42 (FBR exit clearance, SBP forex approval, FATF screening)", "0.42"],
        ["2", "UAE", "KSA", "0.31 (DFSA re-registration, VAT alignment, GCC corridor treaty)", "0.73"],
      ],
      [936, 1170, 1170, 3744, 2340]
    ),
    p_runs([
      bold("Total path cost: "),
      "Cost(P) = 0.42 + 0.31 = 0.73. ",
      "A direct PAK \u2192 KSA path (if a corridor existed) would have an estimated burden of 0.89, ",
      "making the UAE waypoint route 18% cheaper. The manifold surface reveals UAE as a compliance ",
      "valley \u2014 a low-burden intermediate that reduces aggregate migration cost."
    ]),
    ...codeBlock(
      "// Example: computing a migration path cost\n" +
      "let manifold = ComplianceManifold::from_tensor(&tensor, &jurisdictions)?;\n" +
      "let path = manifold.optimal_path(\n" +
      "    &JurisdictionId::from(\"PAK\"),\n" +
      "    &JurisdictionId::from(\"KSA\"),\n" +
      "    &[],  // no forced waypoints; algorithm discovers UAE\n" +
      ")?;\n" +
      "assert_eq!(path.waypoints, vec![\"PAK\", \"UAE\", \"KSA\"]);\n" +
      "assert!((path.total_cost - 0.73).abs() < 1e-6);"
    ),

    // --- 20.3 Manifold Visualization ---
    h2("20.3 Manifold Visualization"),
    p(
      "The compliance manifold exists in n-dimensional jurisdictional space, but operators " +
      "require a 2D projection for dashboard display. The manifold can be projected onto a " +
      "two-dimensional plane using dimensionality reduction techniques that preserve the " +
      "topological structure most relevant to migration path planning."
    ),
    p(
      "The primary projection maps jurisdictions to a 2D coordinate system where spatial " +
      "proximity corresponds to low compliance burden between jurisdictions. Corridors with " +
      "active trade agreements (e.g., GCC member states) cluster together, while jurisdictions " +
      "with high regulatory barriers (sanctions, FATF greylist status) appear as distant peaks. " +
      "The surface height at each point encodes the local compliance burden, rendered as a " +
      "heatmap or contour plot."
    ),

    h3("20.3.1 Projection Method"),
    p(
      "Given the manifold M: R^n \u2192 R, the dashboard projection \u03C0: R^n \u2192 R^2 is computed " +
      "by selecting the two principal components of the jurisdictional distance matrix " +
      "D[i,j] = Cost(optimal_path(Ji, Jj)). This preserves the metric structure that " +
      "operators care about: jurisdictions that are cheap to migrate between appear close " +
      "together, and compliance valleys (low-cost waypoints like UAE free zones) are visually " +
      "identifiable as basins in the rendered surface."
    ),
    p(
      "The visualization overlays active corridors as directed edges, with edge thickness " +
      "proportional to trade volume and color encoding compliance burden (green for low, " +
      "red for high). Attestation gaps (Definition 20.4) are rendered as warning markers at " +
      "the affected jurisdictional coordinate."
    ),

    // --- 20.4 Attestation Gap Analysis ---
    h2("20.4 Attestation Gap Analysis"),
    definition(
      "Definition 20.4 (Attestation Gap).",
      "An attestation gap represents a compliance requirement not satisfied by current " +
      "attestations. It captures the specific requirement, its compliance domain, the " +
      "jurisdiction in which it applies, the severity of non-compliance, whether it blocks " +
      "forward progress, and the available remediation paths with estimated time and cost."
    ),
    ...codeBlock(
      "/// A compliance requirement not yet satisfied by current attestations.\n" +
      "/// Identified during manifold traversal when an entity's credential set\n" +
      "/// does not cover a waypoint's requirements.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct AttestationGap {\n" +
      "    /// Unique identifier for the unsatisfied requirement.\n" +
      "    pub requirement_id: RequirementId,\n" +
      "    /// The compliance domain (e.g., Tax, Sanctions, Licensing).\n" +
      "    pub domain: ComplianceDomain,\n" +
      "    /// The jurisdiction where this gap applies.\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    /// Severity of the gap: determines escalation and reporting.\n" +
      "    pub severity: GapSeverity, // Critical | High | Medium | Low\n" +
      "    /// Whether this gap blocks corridor traversal or entity migration.\n" +
      "    pub blocking: bool,\n" +
      "    /// Available remediation paths to fill the gap.\n" +
      "    pub remediation_options: Vec<RemediationOption>,\n" +
      "    /// Estimated calendar days to remediate (shortest option).\n" +
      "    pub estimated_days: u32,\n" +
      "    /// Estimated cost in USD to remediate (cheapest option).\n" +
      "    pub estimated_cost: f64,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum GapSeverity {\n" +
      "    Critical, // Regulatory violation; immediate enforcement risk\n" +
      "    High,     // Non-compliance with mandatory filing or license\n" +
      "    Medium,   // Best-practice gap; may trigger audit flags\n" +
      "    Low,      // Advisory; no enforcement consequence\n" +
      "}"
    ),
  ];
};
