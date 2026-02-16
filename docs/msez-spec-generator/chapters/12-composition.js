const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, pageBreak
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

    // --- 12.2.1 Composition Data Model ---
    h3("12.2.1 Composition Data Model"),
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
    p_runs([
      bold("Kazakhstan Example. "),
      "The Astana International Financial Centre (AIFC) composes three jurisdiction layers: Kazakhstan national law (base layer providing CIVIC, CORPORATE, TAX, and LABOR domains), AIFC common law framework (overlay replacing COMMERCIAL, FINANCIAL, SECURITIES, BANKING, and ARBITRATION with English common law equivalents), and FATF/EAG mutual evaluation requirements (overlay strengthening AML_CFT domain to meet enhanced due diligence standards). The resulting composition has 20 domains with 8 overridden by the AIFC layer, producing a compliance tensor of shape (3, 20) with 60 cells."
    ]),

    // --- 12.3 Composition Validation ---
    h2("12.3 Composition Validation"),
    p("Composition validation enforces four constraints: completeness (all twenty compliance domains must be covered by at least one jurisdiction layer), consistency (no two layers may impose contradictory requirements on the same domain without an explicit conflict resolution rule), temporal validity (all referenced lawpacks, regpacks, and licensepacks must have overlapping validity periods covering the composition effective date), and digest integrity (the composition digest must equal the SHA-256 hash of the canonical serialization of all layers, ensuring tamper detection)."),

    // --- 12.3.1 Composition Factory ---
    h3("12.3.1 Composition Factory"),
    p("The compose_zone factory function accepts a zone identifier, a list of jurisdiction identifiers with their pack references, and an optional set of domain override policies. It fetches the referenced packs, validates completeness and consistency, resolves conflicts using the strictest-rule-wins principle, computes the composition digest, and returns a ComposedJurisdiction. The factory is idempotent: given the same inputs and pack versions, it always produces the same composition digest."),
    ...codeBlock(
      "/// Compose a multi-jurisdiction zone from pack references.\n" +
      "///\n" +
      "/// Fetches all referenced lawpacks, regpacks, and licensepacks,\n" +
      "/// validates completeness across all 20 compliance domains,\n" +
      "/// resolves conflicts via strictest-rule-wins, and returns\n" +
      "/// the composed zone with a deterministic composition digest.\n" +
      "pub fn compose_zone(\n" +
      "    zone_id: &str,\n" +
      "    name: &str,\n" +
      "    layers: Vec<JurisdictionLayerInput>,\n" +
      "    overrides: Option<HashMap<ComplianceDomain, DomainPolicy>>,\n" +
      "    effective_from: chrono::NaiveDate,\n" +
      "    pack_store: &dyn PackStore,\n" +
      ") -> Result<ComposedJurisdiction, CompositionError> {\n" +
      "    // 1. Fetch all packs from content-addressed store\n" +
      "    let resolved = resolve_packs(&layers, pack_store)?;\n" +
      "    // 2. Validate completeness: all 20 domains must be covered\n" +
      "    validate_domain_completeness(&resolved)?;\n" +
      "    // 3. Validate temporal overlap of all pack validity periods\n" +
      "    validate_temporal_consistency(&resolved, effective_from)?;\n" +
      "    // 4. Resolve domain conflicts (strictest-rule-wins)\n" +
      "    let merged = resolve_conflicts(&resolved, &overrides)?;\n" +
      "    // 5. Compute deterministic composition digest\n" +
      "    let digest = CanonicalBytes::new(&merged)?.digest();\n" +
      "    // 6. Build and return the ComposedJurisdiction\n" +
      "    Ok(ComposedJurisdiction {\n" +
      "        zone_id: zone_id.to_string(),\n" +
      "        name: name.to_string(),\n" +
      "        layers: merged.layers,\n" +
      "        effective_from,\n" +
      "        composition_digest: digest,\n" +
      "        tensor_shape: (merged.layers.len(), 20),\n" +
      "    })\n" +
      "}"
    ),

    // --- 12.3.2 Composition Example: Pakistan GovOS ---
    h3("12.3.2 Composition Example: Pakistan GovOS"),
    p("The Pakistan GovOS deployment composes three jurisdiction layers into a single zone configuration: the Pakistan national base layer, the SEZ overlay providing special economic zone exemptions, and the FATF overlay strengthening AML/CFT and financial monitoring. The resulting composition covers all 20 ComplianceDomain variants (§10.1) with a tensor shape of (3, 20) yielding 60 cells."),
    p_runs([
      bold("Layer 1 — PAK Base (Pakistan National Law). "),
      "Provides the foundational legal framework covering all 20 compliance domains. Sources include the Income Tax Ordinance 2001, Sales Tax Act 1990, Companies Act 2017, Foreign Exchange Regulation Act 1947, Anti-Money Laundering Act 2010, SECP Act 1997, and Pakistan Penal Code. This layer binds entities to FBR registration (NTN) and SBP regulatory oversight. The lawpack references Akoma Ntoso encodings of all applicable statutes."
    ]),
    p_runs([
      bold("Layer 2 — SEZ Overlay (Special Economic Zone Act 2012). "),
      "Overrides 6 domains from the PAK base layer with SEZ-specific exemptions and incentives. TAX is overridden with 10-year income tax exemption per SEZ Act Section 37, customs duty exemption on capital goods, and zero-rated sales tax on exports. CORPORATE is overridden with one-window SECP registration and reduced paid-up capital requirements. COMMERCIAL is overridden with streamlined import/export licensing. LABOR is overridden with SEZ-specific labor terms per BOI guidelines. LICENSING is overridden with single-window BOI/zone developer approval. CUSTOMS (mapped to PAYMENTS) is overridden with bonded warehouse and duty drawback provisions."
    ]),
    p_runs([
      bold("Layer 3 — FATF Overlay (FATF Mutual Evaluation). "),
      "Strengthens 3 domains beyond the PAK base layer requirements to meet FATF grey-list remediation commitments. AML_CFT is overridden with enhanced due diligence (EDD) for all cross-border transactions above USD 10,000, beneficial ownership transparency per FATF Recommendation 24, and STR filing within 24 hours. FINANCIAL is overridden with SBP real-time transaction monitoring and cross-border wire transfer reporting per FATF Recommendation 16. DATA_PROTECTION is overridden with mandatory 5-year record retention for all KYC/KYB data and secure data sharing protocols with FMU (Financial Monitoring Unit)."
    ]),
    p("The domain assignment matrix for the Pakistan GovOS composition:"),
    table(
      ["Domain", "Governing Layer", "Key Provision"],
      [
        ["CIVIC", "PAK Base", "Pakistan Citizenship Act 1951; NADRA CNIC verification"],
        ["CORPORATE", "SEZ Overlay", "One-window SECP registration; reduced capital requirements"],
        ["COMMERCIAL", "SEZ Overlay", "Streamlined import/export; SEZ Act Section 23"],
        ["FINANCIAL", "FATF Overlay", "SBP real-time monitoring; FATF Rec. 16 wire transfers"],
        ["SECURITIES", "PAK Base", "Securities Act 2015; SECP regulatory framework"],
        ["BANKING", "PAK Base", "Banking Companies Ordinance 1962; SBP prudential regulations"],
        ["PAYMENTS", "SEZ Overlay", "Bonded warehouse; duty drawback; SBP Raast integration"],
        ["DIGITAL_ASSETS", "PAK Base", "SECP position paper 2023; pending Digital Assets Act"],
        ["TAX", "SEZ Overlay", "10-year exemption; zero-rated exports; SEZ Act Section 37"],
        ["AML_CFT", "FATF Overlay", "EDD above USD 10K; UBO transparency; 24-hour STR"],
        ["DATA_PROTECTION", "FATF Overlay", "5-year KYC retention; FMU data sharing protocols"],
        ["ARBITRATION", "PAK Base", "Arbitration Act 1940; ICSID Convention recognition"],
        ["LICENSING", "SEZ Overlay", "Single-window BOI approval; zone developer licensing"],
        ["INSURANCE", "PAK Base", "Insurance Ordinance 2000; SECP insurance division"],
        ["ENVIRONMENTAL", "PAK Base", "Pakistan Environmental Protection Act 1997; EPA NOCs"],
        ["LABOR", "SEZ Overlay", "SEZ-specific labor terms; BOI workforce guidelines"],
        ["INTELLECTUAL_PROPERTY", "PAK Base", "IPO Pakistan; Patents Ordinance 2000; Copyright Ordinance 1962"],
        ["IMMIGRATION", "PAK Base", "Foreigners Act 1946; BOI investor visa facilitation"],
        ["REAL_ESTATE", "PAK Base", "Transfer of Property Act 1882; land revenue codes"],
        ["HEALTH_SAFETY", "PAK Base", "Factories Act 1934; OSHA-equivalent provincial rules"],
      ],
      [2200, 1800, 5360]
    ),
    p("The composition digest is computed as SHA-256 over the canonical serialization of all three layers, their pack references, and the domain override matrix. This digest is embedded in every corridor state and verifiable credential issued under the Pakistan GovOS zone, binding all compliance attestations to a specific, reproducible regulatory snapshot."),

    // --- 12.3.3 Generated Artifacts ---
    h3("12.3.3 Generated Artifacts"),
    p("The composition engine produces three artifacts: zone.yaml (the human-readable zone configuration specifying jurisdiction layers, domain assignments, and override policies), stack.lock (the machine-readable lockfile pinning exact pack versions with their content-addressed digests, ensuring reproducible builds), and composition_digest (the SHA-256 commitment over the entire composed configuration, embedded in corridor state and verifiable credentials to bind compliance attestations to a specific regulatory snapshot)."),
  ];
};
