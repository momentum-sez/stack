const {
  partHeading, chapterHeading, h2, h3,
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
    p("The Dissolution Module manages entity winding-up processes through a deterministic ten-stage state machine. Each stage has defined entry conditions, required actions, timeout enforcement, and compensation actions for rollback. The module coordinates with treasury-info.api.mass.inc for final distributions and issues Dissolution Verifiable Credentials upon completion."),

    h3("32.7.1 Dissolution State Machine"),
    table(
      ["Stage", "Name", "Actions", "Timeout"],
      [
        ["D1", "RESOLUTION_FILED", "Board resolution recorded, regulatory notification dispatched", "30 days"],
        ["D2", "CREDITOR_NOTICE", "Public notice published, individual creditor notifications sent", "90 days"],
        ["D3", "CLAIMS_PERIOD", "Creditor claims received and validated against entity records", "180 days"],
        ["D4", "CLAIMS_ADJUDICATION", "Disputed claims resolved through arbitration module", "90 days"],
        ["D5", "ASSET_INVENTORY", "Complete asset inventory compiled, valuations obtained", "60 days"],
        ["D6", "TAX_CLEARANCE", "Final tax returns filed, clearance certificates obtained from FBR/relevant authority", "120 days"],
        ["D7", "ASSET_DISTRIBUTION", "Assets distributed to creditors by priority, surplus to shareholders", "90 days"],
        ["D8", "REGULATORY_DEREGISTRATION", "Licenses surrendered, registrations cancelled with SECP/BOI/relevant bodies", "60 days"],
        ["D9", "FINAL_ACCOUNTING", "Final accounts prepared, auditor sign-off obtained", "30 days"],
        ["D10", "DISSOLVED", "Entity marked dissolved, Dissolution VC issued, all records archived", "Terminal"],
      ],
      [800, 2200, 3800, 2560]
    ),
    spacer(),

    h3("32.7.2 Resolution Types"),
    p("The Secretarial Module supports eleven resolution types that govern corporate decision-making:"),
    table(
      ["Type", "Quorum", "Threshold", "Use Case"],
      [
        ["Ordinary Resolution", "Simple majority present", "50%+1", "Routine business decisions"],
        ["Special Resolution", "Two-thirds present", "75%+", "Constitutional amendments, dissolution"],
        ["Extraordinary Resolution", "As per articles", "As specified", "Major structural changes"],
        ["Written Resolution", "N/A (circulated)", "Unanimous", "Decisions without meeting"],
        ["Board Resolution", "Majority of directors", "50%+1", "Day-to-day management"],
        ["Circular Resolution", "N/A (circulated to directors)", "Unanimous", "Urgent board decisions"],
        ["Members Voluntary Liquidation", "75% of members", "Special + solvency declaration", "Solvent winding up"],
        ["Creditors Voluntary Liquidation", "Simple majority", "50%+1 of creditors by value", "Insolvent winding up"],
        ["Court-Ordered Liquidation", "Court order", "N/A", "Compulsory winding up"],
        ["Scheme of Arrangement", "75% of each class", "75% by value per class", "Restructuring"],
        ["Amalgamation Resolution", "75% of each entity", "Special resolution per entity", "Mergers"],
      ],
      [2400, 2200, 2200, 2560]
    ),
    spacer(),

    h3("32.7.3 Filing Schedule and Reminders"),
    p("The Annual Compliance Module generates automatic reminders through msez-agentic triggers at defined intervals before each filing deadline:"),
    table(
      ["Days Before Deadline", "Action", "Channel"],
      [
        ["90", "First reminder generated, task created in GovOS Console", "Email + Console"],
        ["60", "Second reminder with preparation checklist", "Email + Console + SMS"],
        ["30", "Urgent reminder, manager escalation triggered", "Email + Console + SMS + Escalation"],
        ["14", "Final warning, penalty computation preview displayed", "All channels + Executive alert"],
        ["7", "Critical alert, automatic filing initiated if data complete", "All channels + Auto-file attempt"],
        ["0", "Deadline reached, penalty accrual begins", "All channels + Penalty notification"],
      ],
      [2200, 4200, 2960]
    ),
    spacer(),

    h3("32.7.4 Filing Types"),
    table(
      ["Filing", "Frequency", "Authority", "Penalty (PKR)"],
      [
        ["Annual Return (Form A)", "Annual", "SECP", "100/day late, max 365 days"],
        ["Financial Statements", "Annual", "SECP", "Level 1-3 penalty schedule"],
        ["Beneficial Ownership", "On change + Annual", "SECP", "50,000 + 500/day"],
        ["Income Tax Return", "Annual", "FBR", "Per ITO 2001 \u00a7182"],
        ["Sales Tax Return", "Monthly", "FBR", "Per STA 1990 \u00a733"],
        ["Withholding Tax Statement", "Quarterly/Annual", "FBR", "Per ITO 2001 \u00a7182"],
        ["Director Changes", "Within 15 days", "SECP", "10,000 + 200/day"],
        ["Registered Office Change", "Within 15 days", "SECP", "5,000 + 100/day"],
        ["Share Transfer", "Within 30 days", "SECP", "25,000 + 500/day"],
        ["Charge Registration", "Within 30 days", "SECP", "50,000 + 1,000/day"],
      ],
      [2400, 2000, 1800, 3160]
    ),
    spacer(),
  ];
};
