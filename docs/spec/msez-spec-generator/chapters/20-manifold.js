const {
  chapterHeading, h2,
  p,
  definition, codeBlock,
  spacer
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
    spacer(),

    // --- 20.2 Migration Path Optimization ---
    h2("20.2 Migration Path Optimization"),
    p("Path cost is computed as the line integral of compliance burden along the path: Cost(P) = \u222B_P compliance_burden(x) dx. The optimal path minimizes this integral while satisfying all waypoint constraints."),

    // --- 20.3 Attestation Gap Analysis ---
    h2("20.3 Attestation Gap Analysis"),
    definition(
      "Definition 20.3 (Attestation Gap).",
      "An attestation gap represents a compliance requirement not satisfied by current attestations. It includes the requirement specification, domain, jurisdiction, severity, blocking status, and remediation options with estimated time and cost to fill."
    ),
  ];
};
