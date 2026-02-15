const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter32() {
  return [
    ...partHeading("PART XII: INSTITUTIONAL INFRASTRUCTURE MODULES (v0.4.44)"),
    chapterHeading("Chapter 32: Corporate Services Module Family"),

    // --- 32.1 Module Overview ---
    h2("32.1 Module Overview"),
    p("The Corporate Services module family provides institutional infrastructure for entity formation, governance, capitalization, and lifecycle management. Each module maps to specific Mass API endpoints through the msez-mass-client gateway, with the SEZ Stack adding jurisdictional compliance, credential issuance, and corridor awareness."),
    table(
      ["Module", "Function", "Mass API Interface"],
      [
        ["Formation", "Entity incorporation and registration", "organization-info.api.mass.inc"],
        ["Beneficial Ownership", "UBO tracking and disclosure", "organization-info.api.mass.inc"],
        ["Capitalization Table", "Securities, SAFEs, vesting", "investment-info (Heroku)"],
        ["Secretarial", "Board minutes, resolutions, filings", "consent.api.mass.inc"],
        ["Annual Compliance", "Periodic filings, renewals, audits", "organization-info.api.mass.inc"],
        ["Dissolution", "Winding up, asset distribution", "treasury-info.api.mass.inc"],
        ["Power of Attorney", "Authority delegation and revocation", "consent.api.mass.inc"],
        ["Registered Agent", "Statutory agent management", "organization-info.api.mass.inc"],
      ],
      [2200, 3600, 3560]
    ),
    spacer(),

    // --- 32.2 Formation Module ---
    h2("32.2 Formation Module"),
    p("The Formation Module orchestrates entity creation across jurisdictions. It composes Mass API entity creation with compliance tensor evaluation, lawpack checking (e.g., Companies Act requirements), regpack verification (sanctions screening, calendar awareness), and licensepack validation (registry-specific requirements). Upon successful formation, the module issues a Formation Verifiable Credential."),
    p_runs([bold("Pakistan Example."), " Formation of a Private Limited Company under Companies Act 2017 requires: SECP name availability check, digital NTN binding via FBR integration, minimum two directors (CNIC-verified via NADRA), registered office in SEZ jurisdiction, and authorized capital declaration. The Formation Module orchestrates all checks through msez-mass-client to organization-info.api.mass.inc, evaluates compliance via msez-tensor for PAK jurisdiction across all 20 domains, and issues a Formation VC anchored to the corridor state."]),
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
    spacer(),

    // --- 32.3 Beneficial Ownership Module ---
    h2("32.3 Beneficial Ownership Module"),
    p("The Beneficial Ownership Module tracks ultimate beneficial owners across jurisdictional requirements. It enforces FATF Recommendation 24 compliance, manages ownership thresholds (typically 10-25% depending on jurisdiction), and issues Beneficial Ownership Verifiable Credentials. The module integrates with organization-info.api.mass.inc for ownership records and msez-tensor for cross-jurisdictional threshold evaluation."),
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
    spacer(),

    // --- 32.4 Capitalization Table Module ---
    h2("32.4 Capitalization Table Module"),
    p("The Capitalization Table Module manages securities issuance, SAFEs, convertible instruments, and vesting schedules. It interfaces exclusively with investment-info (Heroku) through msez-mass-client for all cap table CRUD operations. The SEZ Stack adds compliance verification (securities law by jurisdiction), credential issuance for ownership certificates, and corridor-aware transfer restrictions."),
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
    spacer(),

    // --- 32.5 Secretarial Module ---
    h2("32.5 Secretarial Module"),
    p("The Secretarial Module manages board resolutions, meeting minutes, statutory filings, and corporate governance records. It integrates with consent.api.mass.inc for multi-party approval workflows and issues Governance Verifiable Credentials for board decisions. The module enforces quorum requirements, notice periods, and voting thresholds defined in the entity's constitutional documents."),

    // --- 32.6 Annual Compliance Module ---
    h2("32.6 Annual Compliance Module"),
    p("The Annual Compliance Module tracks and orchestrates periodic regulatory obligations: annual returns, financial statement filings, license renewals, tax filings, and audit requirements. It uses msez-agentic triggers to generate automatic reminders and initiate filing workflows before deadlines."),
    p_runs([bold("Pakistan Example."), " Annual return filing with SECP (Form A), annual financial statements, FBR income tax return, withholding tax statements, and Sales Tax returns are all tracked with jurisdiction-specific deadlines from the regpack filing calendar."]),

    // --- 32.7 Dissolution Module ---
    h2("32.7 Dissolution Module"),
    p("The Dissolution Module manages entity winding-up processes including creditor notification, asset distribution, regulatory de-registration, and final compliance attestation. It coordinates with treasury-info.api.mass.inc for final distributions and issues Dissolution Verifiable Credentials upon completion. The module enforces jurisdiction-specific winding-up procedures and timelines."),
  ];
};
