const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter12() {
  return [
    pageBreak(),
    chapterHeading("Chapter 12: Multi-Jurisdiction Composition Engine (v0.4.44)"),

    // --- 12.1 Design Thesis ---
    h2("12.1 Design Thesis"),
    p("The composition engine solves the fundamental problem of multi-jurisdiction compliance: no single jurisdiction operates in isolation. A Pakistan SEZ entity trading with a UAE free zone counterparty must satisfy Pakistani export controls, UAE import requirements, FATF AML/CFT standards, bilateral treaty obligations, and corridor-specific settlement rules simultaneously. The composition engine takes jurisdiction-specific lawpacks, regpacks, and licensepacks and produces a composed zone configuration that captures all applicable requirements, resolves conflicts between jurisdictions, and generates the compliance tensor structure for the combined regulatory space."),

    // --- 12.2 Domain Enumeration ---
    h2("12.2 Domain Enumeration"),
    p("The composition engine operates across all twenty compliance domains defined in the compliance tensor: CIVIC, CORPORATE, COMMERCIAL, FINANCIAL, SECURITIES, BANKING, PAYMENTS, DIGITAL_ASSETS, TAX, AML_CFT, DATA_PROTECTION, ARBITRATION, LICENSING, INSURANCE, ENVIRONMENTAL, LABOR, INTELLECTUAL_PROPERTY, IMMIGRATION, REAL_ESTATE, and HEALTH_SAFETY. Each domain is composed independently, allowing jurisdiction-specific overrides at domain granularity. Conflict resolution follows the strictest-rule-wins principle: when two jurisdictions impose different requirements on the same domain, the more restrictive requirement prevails."),

    // --- 12.3 Composition Data Model ---
    h2("12.3 Composition Data Model"),
    ...codeBlock(
      "/// A composed multi-jurisdiction zone configuration.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ComposedJurisdiction {\n" +
      "    pub zone_id: String,\n" +
      "    pub name: String,\n" +
      "    pub layers: Vec<JurisdictionLayer>,\n" +
      "    pub effective_from: chrono::NaiveDate,\n" +
      "    pub composition_digest: Digest,\n" +
      "    pub tensor_shape: (usize, usize),  // (jurisdictions, domains)\n" +
      "}\n" +
      "\n" +
      "/// A single jurisdiction layer within a composition.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct JurisdictionLayer {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub lawpack_ref: Digest,\n" +
      "    pub regpack_ref: Digest,\n" +
      "    pub licensepack_ref: Digest,\n" +
      "    pub domain_overrides: HashMap<ComplianceDomain, DomainPolicy>,\n" +
      "    pub bilateral_treaties: Vec<TreatyRef>,\n" +
      "}"
    ),
    spacer(),
    p_runs([
      bold("Kazakhstan Example. "),
      "The Astana International Financial Centre (AIFC) composes three jurisdiction layers: Kazakhstan national law (base layer providing CIVIC, CORPORATE, TAX, and LABOR domains), AIFC common law framework (overlay replacing COMMERCIAL, FINANCIAL, SECURITIES, BANKING, and ARBITRATION with English common law equivalents), and FATF/EAG mutual evaluation requirements (overlay strengthening AML_CFT domain to meet enhanced due diligence standards). The resulting composition has 20 domains with 8 overridden by the AIFC layer, producing a compliance tensor of shape (3, 20) with 60 cells."
    ]),

    // --- 12.4 Composition Validation ---
    h2("12.4 Composition Validation"),
    p("Composition validation enforces four constraints: completeness (all twenty compliance domains must be covered by at least one jurisdiction layer), consistency (no two layers may impose contradictory requirements on the same domain without an explicit conflict resolution rule), temporal validity (all referenced lawpacks, regpacks, and licensepacks must have overlapping validity periods covering the composition effective date), and digest integrity (the composition digest must equal the SHA-256 hash of the canonical serialization of all layers, ensuring tamper detection)."),

    // --- 12.5 Composition Factory ---
    h2("12.5 Composition Factory"),
    p("The compose_zone factory function accepts a zone identifier, a list of jurisdiction identifiers with their pack references, and an optional set of domain override policies. It fetches the referenced packs, validates completeness and consistency, resolves conflicts using the strictest-rule-wins principle, computes the composition digest, and returns a ComposedJurisdiction. The factory is idempotent: given the same inputs and pack versions, it always produces the same composition digest."),

    // --- 12.6 Generated Artifacts ---
    h2("12.6 Generated Artifacts"),
    p("The composition engine produces three artifacts: zone.yaml (the human-readable zone configuration specifying jurisdiction layers, domain assignments, and override policies), stack.lock (the machine-readable lockfile pinning exact pack versions with their content-addressed digests, ensuring reproducible builds), and composition_digest (the SHA-256 commitment over the entire composed configuration, embedded in corridor state and verifiable credentials to bind compliance attestations to a specific regulatory snapshot)."),
  ];
};
