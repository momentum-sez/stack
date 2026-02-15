const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter06() {
  return [
    pageBreak(),
    chapterHeading("Chapter 6: The Pack Trilogy"),
    p("The Pack Trilogy \u2014 lawpacks, regpacks, and licensepacks \u2014 provides comprehensive, cryptographically verifiable snapshots of jurisdictional state across all temporal frequencies:"),
    table(
      ["Pack Type", "Content", "Change Frequency"],
      [
        ["Lawpack", "Statutes, regulations (Akoma Ntoso XML)", "Months/Years"],
        ["Regpack", "Sanctions, calendars, guidance, SROs", "Days/Weeks"],
        ["Licensepack", "Live license registries", "Hours/Days"],
      ],
      [2000, 4200, 3160]
    ),
    spacer(),

    // --- 6.1 Lawpack System ---
    h2("6.1 Lawpack System"),
    p("Lawpacks encode jurisdiction-specific legal and regulatory requirements in machine-readable format. A lawpack consists of five components: regulatory manifest, rule definitions, evidence requirements, attestation schema, and tensor definitions. The regulatory manifest identifies the lawpack and its scope. Rule definitions encode specific requirements as evaluatable predicates. Evidence requirements specify documentation needed to demonstrate compliance. Attestation schema defines the structure of compliance attestations. Tensor definitions specify the compliance tensor structure for the jurisdiction."),
    p_runs([bold("Pakistan Example."), " The Pakistan GovOS deployment encodes the following primary legislation:"]),
    table(
      ["Act", "Akoma Ntoso ID", "Key Provisions"],
      [
        ["Income Tax Ordinance 2001", "pk-ito-2001", "Income classification, withholding schedules, tax credits, NTN requirements"],
        ["Sales Tax Act 1990", "pk-sta-1990", "GST rates, input/output tax, exempt supplies, e-invoicing requirements"],
        ["Federal Excise Act 2005", "pk-fea-2005", "Excise duties, manufacturing levies, excisable services"],
        ["Customs Act 1969", "pk-ca-1969", "Import/export duties, tariff schedules, bonded warehouses, CPEC preferences"],
        ["Companies Act 2017", "pk-ca-2017", "Entity formation, director duties, beneficial ownership, SECP registration"],
      ],
      [2600, 1800, 4960]
    ),
    spacer(),
    ...codeBlock(
      "/// A lawpack: content-addressed bundle of legislation in Akoma Ntoso.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Lawpack {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub version: SemanticVersion,\n" +
      "    pub as_of_date: chrono::NaiveDate,\n" +
      "    pub acts: Vec<AkomaAct>,\n" +
      "    pub digest: Digest,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct AkomaAct {\n" +
      "    pub akn_id: String,\n" +
      "    pub title: String,\n" +
      "    pub body_xml: String,  // Akoma Ntoso XML\n" +
      "    pub provisions: Vec<Provision>,\n" +
      "    pub effective_date: chrono::NaiveDate,\n" +
      "}"
    ),
    spacer(),

    // --- 6.2 Lawpack Composition ---
    h2("6.2 Lawpack Composition"),
    p("Lawpacks compose hierarchically through import and extension mechanisms. A jurisdiction may import rules from international standards, regional agreements, or template packs, then extend with local modifications. Import semantics bring external rules into scope with optional namespace prefixing. Extension semantics enable modification of inherited rules. Local rules may strengthen, weaken, or replace requirements entirely."),

    // --- 6.3 Lawpack Attestation and Binding ---
    h2("6.3 Lawpack Attestation and Binding"),
    p("Lawpacks become operative through attestation and binding. Attestation confirms the lawpack accurately represents legal requirements. Binding associates the lawpack with specific system components including corridors, assets, or entities. The attestation process produces a Verifiable Credential signed by the issuing authority."),

    // --- 6.4 RegPack System ---
    h2("6.4 RegPack System"),
    p("The RegPack system provides dynamic regulatory state management, enabling real-time policy updates without system downtime."),
    p_runs([bold("Pakistan Example."), " The FBR regpack includes:"]),
    table(
      ["Component", "Update Frequency", "Content"],
      [
        ["WHT Rate Tables", "Per SRO (days)", "Withholding rates by income category, payee type, and NTN status"],
        ["Filing Calendar", "Quarterly", "Monthly/quarterly/annual return deadlines for income tax, sales tax, FED"],
        ["SRO Registry", "As issued", "Statutory Regulatory Orders modifying tax rates, exemptions, procedures"],
        ["FATF AML/CFT", "FATF plenary cycle", "Customer due diligence tiers, STR thresholds, PEP definitions"],
        ["OFAC/EU/UN Sanctions", "Daily sync", "Designated persons lists, entity matches, fuzzy matching thresholds"],
      ],
      [2400, 2000, 4960]
    ),
    spacer(),
    ...codeBlock(
      "/// A regpack: machine-readable regulatory state.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Regpack {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub version: SemanticVersion,\n" +
      "    pub effective_from: chrono::NaiveDate,\n" +
      "    pub tax_calendars: Vec<TaxCalendar>,\n" +
      "    pub withholding_tables: Vec<WithholdingTable>,\n" +
      "    pub sanctions_lists: Vec<SanctionsList>,\n" +
      "    pub aml_cft_rules: AmlCftRules,\n" +
      "    pub sro_registry: Vec<StatutoryRegulatoryOrder>,\n" +
      "    pub digest: Digest,\n" +
      "}"
    ),
    p("RegPack digests provide cryptographic commitments to regulatory state at specific times. Corridor bindings include RegPack digests to establish the regulatory context. The \u03C0sanctions ZK circuit enables privacy-preserving sanctions verification with approximately 18,000 constraints."),

    // --- 6.5 Licensepack System (v0.4.44) ---
    h2("6.5 Licensepack System (v0.4.44)"),
    p("Licensepacks complete the Pack Trilogy, providing cryptographically verifiable snapshots of jurisdictional licensing state. Licensing state is critical for corridor operations and compliance verification. Licensepacks enable offline license verification, audit trails proving licensing state at any historical point, cross-zone settlement with counterparty authorization verification, and LICENSING domain population for compliance tensors."),
    p_runs([bold("Pakistan Example."), " Fifteen-plus license categories across regulatory authorities:"]),
    table(
      ["Authority", "License Categories", "Key Requirements"],
      [
        ["SECP", "Company registration, NTN issuance", "Memorandum/Articles, director KYC, registered office"],
        ["BOI", "Industrial licenses, SEZ registrations", "Investment thresholds, sector restrictions, incentive eligibility"],
        ["PTA", "Telecom licenses, spectrum allocation", "Technical standards, coverage obligations, fee schedules"],
        ["PEMRA", "Media licenses, broadcasting permits", "Content standards, ownership limits, renewal cycles"],
        ["DRAP", "Drug/device manufacturing, import permits", "GMP compliance, clinical trial data, product registration"],
        ["Provincial", "Trade licenses, professional permits", "Varies by province: Punjab, Sindh, KP, Balochistan"],
      ],
      [1600, 3200, 4560]
    ),
    spacer(),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Licensepack {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub authority: RegulatoryAuthority,\n" +
      "    pub license_types: Vec<LicenseType>,\n" +
      "    pub version: SemanticVersion,\n" +
      "    pub digest: Digest,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicenseType {\n" +
      "    pub id: String,\n" +
      "    pub name: String,\n" +
      "    pub permitted_activities: Vec<ActivityCode>,\n" +
      "    pub requirements: LicenseRequirements,\n" +
      "    pub fees: FeeSchedule,\n" +
      "    pub renewal: RenewalSchedule,\n" +
      "    pub compliance_obligations: Vec<ComplianceObligation>,\n" +
      "}"
    ),

    // --- 6.5.1 License Data Model ---
    h3("6.5.1 License Data Model"),
    p("Six status values track license lifecycle: ACTIVE (license in good standing), SUSPENDED (temporarily non-operative), REVOKED (permanently terminated), EXPIRED (validity period elapsed), PENDING (application under review), and SURRENDERED (voluntarily relinquished). Six domains categorize license types: FINANCIAL, CORPORATE, PROFESSIONAL, TRADE, INSURANCE, and MIXED."),

    // --- 6.5.2 License Verification ---
    h3("6.5.2 License Verification"),
    p("The licensepack verify_license method performs full authorization verification: license existence and ACTIVE status, activity within permitted scope, amount within authorized limits, currency within permitted instruments, no active restrictions blocking the operation, and all conditions satisfied."),

    // --- 6.5.3 Compliance Tensor Integration ---
    h3("6.5.3 Compliance Tensor Integration"),
    p("Licensepacks populate the LICENSING compliance domain in the Compliance Tensor V2:"),
    table(
      ["License Status", "Tensor State", "Effect"],
      [
        ["ACTIVE", "COMPLIANT", "Operations permitted"],
        ["SUSPENDED", "SUSPENDED", "Operations blocked temporarily"],
        ["REVOKED/EXPIRED", "NON_COMPLIANT", "Operations blocked"],
        ["PENDING", "PENDING", "Limited operations"],
        ["No license required", "EXEMPT", "De minimis exemption"],
      ],
      [2400, 2400, 4560]
    ),
    spacer(),

    // --- 6.5.4 Licensepack Schemas ---
    h3("6.5.4 Licensepack Schemas"),
    p("Three JSON schemas define licensepack structure: licensepack.schema.json (main structure and metadata), licensepack.license.schema.json (individual license records with conditions), and licensepack.lock.schema.json (version pinning)."),

    // --- 6.5.5 Zone Integration ---
    h3("6.5.5 Zone Integration"),
    p("Zones specify licensepack requirements in zone.yaml, including refresh policies per domain. Financial domain licensepacks refresh hourly with maximum 4-hour staleness. Default domain licensepacks refresh daily with 24-hour maximum staleness."),
  ];
};
