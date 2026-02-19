const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter32() {
  return [
    ...partHeading("PART XII: INSTITUTIONAL INFRASTRUCTURE MODULES (v0.4.44)"),
    chapterHeading("Chapter 32: Corporate Services Module Family"),

    // --- 32.1 Module Overview ---
    h2("32.1 Module Overview"),
    p("The Corporate Services module family provides institutional infrastructure for entity formation, governance, capitalization, and lifecycle management. Each module maps to specific Mass API endpoints through the mez-mass-client gateway, with the EZ Stack adding jurisdictional compliance, credential issuance, and corridor awareness."),
    table(
      ["Module", "Function", "Mass API Interface"],
      [
        ["Formation", "Entity incorporation and registration", "organization-info.api.mass.inc"],
        ["Beneficial Ownership", "UBO tracking and disclosure", "organization-info.api.mass.inc"],
        ["Capitalization Table", "Securities, SAFEs, vesting", "investment-info.api.mass.inc"],
        ["Secretarial", "Board minutes, resolutions, filings", "consent.api.mass.inc"],
        ["Annual Compliance", "Periodic filings, renewals, audits", "organization-info.api.mass.inc"],
        ["Dissolution", "Winding up, asset distribution", "treasury-info.api.mass.inc"],
        ["Power of Attorney", "Authority delegation and revocation", "consent.api.mass.inc"],
        ["Registered Agent", "Statutory agent management", "organization-info.api.mass.inc"],
      ],
      [2200, 3600, 3560]
    ),

    // --- 32.2 Module Specifications ---
    h2("32.2 Module Specifications"),

    // --- 32.2.1 Formation Module ---
    h3("32.2.1 Formation Module"),
    p("The Formation Module orchestrates entity creation across jurisdictions. It composes Mass API entity creation with compliance tensor evaluation, lawpack checking (e.g., Companies Act requirements), regpack verification (sanctions screening, calendar awareness), and licensepack validation (registry-specific requirements). Upon successful formation, the module issues a Formation Verifiable Credential."),
    p_runs([bold("Pakistan Example."), " Formation of a Private Limited Company under Companies Act 2017 requires: SECP name availability check, digital NTN binding via FBR integration, minimum two directors (CNIC-verified via NADRA), registered office in EZ jurisdiction, and authorized capital declaration. The Formation Module orchestrates all checks through mez-mass-client to organization-info.api.mass.inc, evaluates compliance via mez-tensor for PAK jurisdiction across all 20 domains, and issues a Formation VC anchored to the corridor state."]),
    p_runs([bold("Schema coverage."), " The formation workflow is validated by mez-schema, which maintains 116 JSON Schema definitions across all corporate modules. Formation-related schemas cover entity type specifications, director requirements, registered office validation, capital structure declarations, and registry integration configurations. Schema validation runs at both the API boundary (mez-api route handlers) and the Mass client boundary (mez-mass-client request/response validation)."]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct FormationRules {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub entity_types: Vec<EntityTypeSpec>,\n" +
      "    pub minimum_capital: Option<Amount>,\n" +
      "    pub director_requirements: DirectorRequirements,\n" +
      "    pub registry_integration: RegistryConfig,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct EntityTypeSpec {\n" +
      "    pub type_code: String,\n" +
      "    pub display_name: String,\n" +
      "    pub legal_basis: LawReference,\n" +
      "    pub required_documents: Vec<DocumentType>,\n" +
      "    pub formation_fee: Amount,\n" +
      "}"
    ),

    // --- 32.2.2 Beneficial Ownership Module ---
    h3("32.2.2 Beneficial Ownership Module"),
    p("The Beneficial Ownership Module tracks ultimate beneficial owners across jurisdictional requirements. It enforces FATF Recommendation 24 compliance, manages ownership thresholds (typically 10-25% depending on jurisdiction), and issues Beneficial Ownership Verifiable Credentials. The module integrates with organization-info.api.mass.inc for ownership records and mez-tensor for cross-jurisdictional threshold evaluation."),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct BeneficialOwnershipRules {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub disclosure_threshold_pct: Decimal,\n" +
      "    pub update_deadline_days: u32,\n" +
      "    pub verification_requirements: Vec<VerificationType>,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct BeneficialOwner {\n" +
      "    pub person_id: PersonId,\n" +
      "    pub ownership_pct: Decimal,\n" +
      "    pub control_type: ControlType,\n" +
      "    pub verified_at: DateTime<Utc>,\n" +
      "    pub verification_method: VerificationType,\n" +
      "}"
    ),

    // --- 32.2.3 Cap Table Module ---
    h3("32.2.3 Cap Table Module"),
    p("The Capitalization Table Module manages securities issuance, SAFEs, convertible instruments, and vesting schedules. It interfaces exclusively with investment-info.api.mass.inc through mez-mass-client for all cap table CRUD operations. The EZ Stack adds compliance verification (securities law by jurisdiction), credential issuance for ownership certificates, and corridor-aware transfer restrictions."),
    p("Share transfers require compliance tensor evaluation for both source and destination jurisdictions, sanctions screening via regpack, and may require regulatory approval depending on the entity type and jurisdiction. The module maintains a complete audit trail of all cap table events as Verifiable Credentials."),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct SecuritiesRules {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub max_shareholders: Option<u32>,\n" +
      "    pub transfer_restrictions: Vec<TransferRestriction>,\n" +
      "    pub reporting_thresholds: Vec<ReportingThreshold>,\n" +
      "    pub exemptions: Vec<SecuritiesExemption>,\n" +
      "}"
    ),

    // --- 32.2.4 Secretarial Module ---
    h3("32.2.4 Secretarial Module"),
    p("The Secretarial Module manages board resolutions, meeting minutes, statutory filings, and corporate governance records. It integrates with consent.api.mass.inc for multi-party approval workflows and issues Governance Verifiable Credentials for board decisions. The module enforces quorum requirements, notice periods, and voting thresholds defined in the entity's constitutional documents."),
    h3("32.2.4.1 Resolution Types"),
    p("The Secretarial Module supports a comprehensive taxonomy of corporate resolution types. Each resolution type carries distinct quorum requirements, notice periods, voting thresholds, and regulatory filing obligations that vary by jurisdiction. The compliance tensor evaluates applicable governance rules across the Governance domain for the entity's jurisdiction."),
    table(
      ["Resolution Type", "Description", "Typical Threshold"],
      [
        ["Board Resolution", "Standard board decision on operational matters requiring simple majority of directors present at a quorate meeting", "Simple majority"],
        ["Special Resolution", "Fundamental changes to constitutional documents, capital structure, or entity status requiring supermajority approval", "75% of votes cast"],
        ["Ordinary Resolution", "Routine shareholder decisions including director appointments, auditor confirmations, and dividend declarations", "Simple majority"],
        ["Written Resolution", "Decision taken without a physical meeting through documented written consent of all eligible voters", "Unanimous (or as per articles)"],
        ["Circular Resolution", "Time-sensitive decision circulated to members for approval outside of scheduled meetings with defined response window", "As per constitutional documents"],
        ["Emergency Resolution", "Urgent decision required to prevent material harm or regulatory breach, with abbreviated notice and expedited approval", "As per emergency provisions"],
        ["Elective Resolution", "Optional resolution to adopt simplified governance procedures permitted under applicable companies legislation", "Unanimous"],
      ],
      [2200, 4960, 2200]
    ),

    // --- 32.2.5 Annual Compliance Module ---
    h3("32.2.5 Annual Compliance Module"),
    p("The Annual Compliance Module tracks and orchestrates periodic regulatory obligations: annual returns, financial statement filings, license renewals, tax filings, and audit requirements. It uses mez-agentic triggers to generate automatic reminders and initiate filing workflows before deadlines."),
    p_runs([bold("Pakistan Example."), " Annual return filing with SECP (Form A), annual financial statements, FBR income tax return, withholding tax statements, and Sales Tax returns are all tracked with jurisdiction-specific deadlines from the regpack filing calendar."]),

    // --- 32.2.6 Dissolution Module ---
    h3("32.2.6 Dissolution Module"),
    p("The Dissolution Module manages entity winding-up processes including creditor notification, asset distribution, regulatory de-registration, and final compliance attestation. It coordinates with treasury-info.api.mass.inc for final distributions and issues Dissolution Verifiable Credentials upon completion. The module enforces jurisdiction-specific winding-up procedures and timelines."),
    h3("32.2.6.1 Dissolution Stages"),
    p("Dissolution proceeds through ten sequential stages. Each stage must complete before the next begins, enforced by the corridor lifecycle FSM in mez-state. The compliance tensor is re-evaluated at each stage transition to ensure ongoing regulatory compliance throughout the winding-up process."),
    table(
      ["Stage", "Description", "Timeline"],
      [
        ["1. Initiation", "Board or shareholder resolution to dissolve, filed with the applicable registry. Triggers compliance tensor evaluation for Dissolution domain.", "Day 0"],
        ["2. Creditor Notification", "Statutory notice to all known creditors and public gazette publication. Creditors submit claims within the notice period.", "Day 1\u201330"],
        ["3. Asset Freezing", "All entity assets frozen via treasury-info.api.mass.inc. No outbound transfers permitted except court-ordered or regulator-approved.", "Day 1\u201314"],
        ["4. Liability Settlement", "Verified creditor claims settled in statutory priority order: secured creditors, employee wages, tax obligations, unsecured creditors.", "Day 30\u201390"],
        ["5. Tax Clearance", "Final tax returns filed, outstanding assessments settled, and tax clearance certificate obtained from the revenue authority (e.g., FBR for Pakistan).", "Day 30\u2013120"],
        ["6. Employee Settlement", "Terminal benefits calculated and disbursed: gratuity, provident fund, accrued leave encashment, and any severance per applicable labor law.", "Day 30\u201360"],
        ["7. Asset Distribution", "Remaining assets distributed to shareholders in proportion to their holdings after all liabilities and preferential claims are satisfied.", "Day 90\u2013150"],
        ["8. Regulatory De-registration", "Entity de-registered from all applicable registries (SECP, FBR, SBP, provincial authorities) and all active licenses surrendered.", "Day 120\u2013180"],
        ["9. Final Audit", "Independent audit of the dissolution process, confirming all obligations met, all assets distributed, and all records preserved.", "Day 150\u2013210"],
        ["10. Archive", "Entity records archived with cryptographic integrity proofs. Dissolution VC issued and anchored to the corridor receipt chain. Entity status set to dissolved.", "Day 210+"],
      ],
      [1800, 5760, 1800]
    ),
  ];
};
