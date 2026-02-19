const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter38() {
  return [
    ...partHeading("PART XIV: GovOS ARCHITECTURE"),

    p("GovOS is the deployment configuration that results when the full MEZ Stack + Mass APIs are deployed for a sovereign government. It is not a separate product; it is the Stack deployed at national scale. Pakistan serves as the reference architecture."),

    chapterHeading("Chapter 38: Four-Layer Model"),

    p("The GovOS architecture comprises four layers:"),

    table(
      ["Layer", "Name", "Function"],
      [
        ["01", "Experience", "Dashboards, portals, citizen-facing services, AI-powered interfaces"],
        ["02", "Platform Engine", "Five Mass primitives + supporting infrastructure + regulated organs"],
        ["03", "Jurisdictional Configuration", "MEZ Pack Trilogy encoding national law in machine-readable format"],
        ["04", "National System Integration", "Connections to existing government systems (Mass enhances existing systems; it does not replace them)"],
      ],
      [800, 2400, 6160]
    ),

    p_runs([bold("Layer 01 — Experience."), " The Experience Layer provides all citizen-facing and government-officer-facing interfaces. This includes the GovOS Console (administrative dashboards for ministry officials), citizen portals (tax filing, license applications, entity registration), and AI-powered interfaces (natural language queries against government data). The Experience Layer never contains business logic; it calls the Platform Engine for all operations."]),

    h2("Layer 01: Experience Portals"),

    p("The Experience Layer comprises five distinct portals, each tailored to a specific user class. Every portal integrates AI capabilities from the Sovereign AI Spine for contextual intelligence."),

    table(
      ["Portal", "Users", "Core Functions", "AI Features"],
      [
        ["GovOS Console", "Ministry administrators, cabinet officials", "Cross-ministry dashboards, budget oversight, policy impact analysis, national KPI tracking", "Natural language queries against government data, predictive policy modeling, automated briefing generation"],
        ["Tax & Revenue Dashboard", "FBR officers, tax auditors, revenue analysts", "Tax collection monitoring, return processing, audit case management, WHT tracking, NTN management", "Evasion pattern detection, under-reporting alerts, risk-scored audit prioritization, revenue projection modeling"],
        ["Digital Free Zone Portal", "Zone operators, zone-registered enterprises, investors", "Entity registration, license management, corridor operations, investment tracking, compliance status", "Automated compliance checking, corridor optimization suggestions, investment risk scoring"],
        ["Citizen Tax & Services Portal", "Individual taxpayers, businesses, public users", "Tax filing (income, sales, customs), NTN registration, license applications, entity registration, payment history", "Pre-filled return suggestions, natural language help in Urdu/English, filing deadline reminders, refund status tracking"],
        ["Regulator Console", "SBP, SECP, FATF reporting officers", "Systemic risk monitoring, AML/CFT dashboards, capital flow oversight, cross-border transaction review", "Anomaly detection across financial flows, FATF mutual evaluation readiness scoring, real-time sanctions screening"],
      ],
      [1400, 1600, 3200, 3160]
    ),

    p_runs([bold("Layer 02 — Platform Engine."), " The Platform Engine is the five Mass primitives (Entities, Ownership, Fiscal, Identity, Consent) plus the regulated Organs (Center of Mass, Torque, Inertia) plus supporting infrastructure (templating engine, notification service, document storage). This layer provides the transactional capabilities that power all government operations."]),

    h2("Layer 02: Platform Engine Components"),

    p("The Platform Engine is composed of the five Mass programmable primitives, supporting engines, and regulated organs. Each component has a defined responsibility boundary and communicates through typed API contracts."),

    table(
      ["Component", "Type", "Responsibility"],
      [
        ["Entities", "Mass Primitive", "Formation, lifecycle, dissolution, FBR registration, beneficial ownership, NTN binding via organization-info.api.mass.inc"],
        ["Ownership", "Mass Primitive", "Cap tables, share classes, vesting schedules, SAFE/convertible instruments, transfers, fundraising rounds via investment-info"],
        ["Fiscal", "Mass Primitive", "Accounts, payments, treasury operations, withholding tax, SBP Raast integration, PKR collection via treasury-info.api.mass.inc"],
        ["Identity", "Mass Primitive", "KYC/KYB, CNIC verification, NTN cross-reference, passportable credentials, DIDs; currently split across consent-info and organization-info"],
        ["Consent", "Mass Primitive", "Multi-party governance approvals, audit trails, tax assessment sign-off workflows via consent.api.mass.inc"],
        ["Event / Task Engine", "Supporting Engine", "Asynchronous event processing, scheduled task execution, webhook delivery, retry logic, dead-letter queues for failed operations"],
        ["Cryptographic Attestation", "Supporting Engine", "Ed25519 signing, W3C Verifiable Credential issuance, Merkle Mountain Range proofs, content-addressed storage for audit trails"],
        ["Compliance Tensor", "Supporting Engine", "20-domain compliance evaluation across jurisdictions, manifold path optimization, real-time regulatory status computation"],
        ["App Marketplace", "Supporting Engine", "Third-party application registry, permissioned API access, developer portal, sandboxed execution environment for zone-specific extensions"],
        ["Organs", "Regulated Organ", "Center of Mass (governance), Torque (execution engine), Inertia (state persistence); regulated entities that compose primitives into higher-order operations"],
      ],
      [2200, 1800, 5360]
    ),

    p_runs([bold("Layer 03 — Jurisdictional Configuration."), " The Jurisdictional Configuration layer is the MEZ Pack Trilogy encoding Pakistani law, regulation, and licensing requirements in machine-readable format. Lawpacks encode the Income Tax Ordinance 2001, Sales Tax Act 1990, Companies Act 2017, and all relevant SROs. Regpacks encode SBP rates, FATF sanctions lists, and filing calendars. Licensepacks encode SECP, BOI, PTA, PEMRA, and provincial authority requirements."]),

    h2("Layer 03: Akoma Ntoso Act Registry"),

    p("Each lawpack references legislation encoded in Akoma Ntoso XML format. The following table lists the primary Pakistani statutes, their Akoma Ntoso act identifiers, and the compliance domains they govern."),

    table(
      ["Statute", "Akoma Ntoso Act ID", "Compliance Domains"],
      [
        ["Income Tax Ordinance 2001", "/akn/pk/act/2001/ord-xlix/main", "Taxation, WithholdingTax, TransferPricing, TaxReporting"],
        ["Sales Tax Act 1990", "/akn/pk/act/1990/act-iii/main", "Taxation, IndirectTax, CustomsDuty"],
        ["Companies Act 2017", "/akn/pk/act/2017/act-xix/main", "CorporateGovernance, BeneficialOwnership, Licensing"],
        ["Foreign Exchange Regulation Act 1947", "/akn/pk/act/1947/act-vii/main", "ForeignExchange, CapitalControls, CrossBorder"],
        ["Anti-Money Laundering Act 2010", "/akn/pk/act/2010/act-vii/main", "AML, KYC, SanctionsScreening"],
        ["SECP Act 1997", "/akn/pk/act/1997/act-xlii/main", "SecuritiesRegulation, Licensing, CorporateGovernance"],
        ["SBP Act 1956", "/akn/pk/act/1956/act-xxxiii/main", "BankingRegulation, MonetaryPolicy, ForeignExchange"],
        ["Economic Zones Act 2012", "/akn/pk/act/2012/act-xx/main", "ZoneRegulation, TaxIncentives, Licensing, CustomsDuty"],
        ["Pakistan Single Window Act 2021", "/akn/pk/act/2021/act-ix/main", "TradeCompliance, CustomsDuty, CrossBorder"],
        ["Customs Act 1969", "/akn/pk/act/1969/act-iv/main", "CustomsDuty, TradeCompliance, ImportExport"],
      ],
      [2800, 3200, 3360]
    ),

    p_runs([bold("Layer 04 — National System Integration."), " The National System Integration layer connects GovOS to existing Pakistani government systems. This includes FBR IRIS (tax administration), SBP Raast (instant payments), NADRA (identity verification), SECP (company registration), SBP RTGS (large-value settlements), and Pakistan Single Window (trade facilitation). Mass enhances these systems; it never replaces them. Integration is additive and reversible."]),

    h2("Layer 04: National System Integration Methods"),

    p("Each national system integration follows a defined protocol and data standard. The integration approach is additive: GovOS consumes data from existing systems and enriches it with compliance context, but never modifies the source system's data or operational flow without explicit consent."),

    table(
      ["National System", "Agency", "Integration Method", "Data Standard", "Direction"],
      [
        ["FBR IRIS", "Federal Board of Revenue", "REST API (HTTPS, OAuth 2.0 client credentials)", "JSON (FBR schema v3), NTN as primary key", "Bidirectional: tax events pushed, return status pulled"],
        ["SBP Raast", "State Bank of Pakistan", "ISO 20022 messaging (pacs.008, pacs.002, pain.001)", "ISO 20022 XML, IBAN as account identifier", "Bidirectional: payment initiation and confirmation"],
        ["NADRA VERISYS", "NADRA", "REST/JSON API (mutual TLS)", "CNIC biometric verification request/response schema", "Pull only: identity verification queries"],
        ["SECP eServices", "Securities and Exchange Commission", "REST API (HTTPS, API key authentication)", "JSON (SECP company registry schema), CUIN as key", "Bidirectional: registration events, compliance status"],
        ["Pakistan Single Window", "Pakistan Customs / Ministry of Commerce", "UN/CEFACT (WCO data model, ebXML messaging)", "UN/EDIFACT and WCO DM v3.x, HS codes for classification", "Bidirectional: trade declarations and clearance status"],
        ["SBP RTGS (PRISM)", "State Bank of Pakistan", "Proprietary messaging (SBP format), migrating to ISO 20022", "PRISM native format (migrating to MX pacs.009)", "Bidirectional: large-value settlement instructions"],
      ],
      [1600, 1400, 2200, 2400, 1760]
    ),

  ];
};
