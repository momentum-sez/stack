const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter06() {
  return [
    // No pageBreak() needed here — chapterHeading() has pageBreakBefore: true built in.
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
    p("Each pack type follows the same content-addressed pattern: deterministic canonicalization of all fields via CanonicalBytes, SHA-256 digest computation over a versioned prefix, and cryptographic binding to corridor state through digest inclusion. This guarantees that any two systems processing the same jurisdictional data produce identical digests, enabling offline verification and cross-zone audit trails without central coordination."),

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

    // --- 6.1.1 Akoma Ntoso Encoding Example ---
    h3("6.1.1 Akoma Ntoso Encoding Example"),
    p("Each act in a lawpack is encoded in Akoma Ntoso XML, the OASIS standard for legislative documents (LegalDocML). The encoding preserves the hierarchical structure of the original legislation \u2014 parts, chapters, sections, and schedules \u2014 while adding machine-readable metadata that enables programmatic evaluation of compliance obligations. Below is a representative fragment showing how the Income Tax Ordinance 2001 encodes the withholding tax provisions that the SEZ Stack evaluates on every transaction through a PAK corridor."),
    ...codeBlock(
      '<akomaNtoso xmlns="http://docs.oasis-open.org/legaldocml/ns/akn/3.0">\n' +
      '  <act name="pk-ito-2001">\n' +
      '    <meta>\n' +
      '      <identification source="#fbr">\n' +
      '        <FRBRWork>\n' +
      '          <FRBRthis value="/akn/pk/act/2001/ito/main"/>\n' +
      '          <FRBRdate date="2001-09-13" name="enactment"/>\n' +
      '          <FRBRauthor href="#national-assembly"/>\n' +
      '        </FRBRWork>\n' +
      '      </identification>\n' +
      '      <references source="#msez">\n' +
      '        <TLCOrganization eId="fbr"\n' +
      '          href="https://fbr.gov.pk" showAs="Federal Board of Revenue"/>\n' +
      '      </references>\n' +
      '    </meta>\n' +
      '    <body>\n' +
      '      <part eId="part-V">\n' +
      '        <num>PART V</num>\n' +
      '        <heading>COLLECTION AND RECOVERY OF TAX</heading>\n' +
      '        <division eId="div-I">\n' +
      '          <heading>Withholding Taxes</heading>\n' +
      '          <section eId="sec-149">\n' +
      '            <num>149</num>\n' +
      '            <heading>Salary income</heading>\n' +
      '            <content>\n' +
      '              <p>Every prescribed person paying salary shall deduct\n' +
      '                 tax at the rates specified in Division I of Part I\n' +
      '                 of the First Schedule.</p>\n' +
      '            </content>\n' +
      '          </section>\n' +
      '          <section eId="sec-153">\n' +
      '            <num>153</num>\n' +
      '            <heading>Payments for goods, services and contracts</heading>\n' +
      '            <content>\n' +
      '              <p>Every prescribed person making a payment in full or\n' +
      '                 part including a payment by way of advance to a\n' +
      '                 resident person shall deduct tax at source.</p>\n' +
      '            </content>\n' +
      '            <hcontainer eId="sec-153__sro-hook">\n' +
      '              <content>\n' +
      '                <!-- SRO rates injected from regpack WHT table -->\n' +
      '                <marker refersTo="#regpack:pk-fbr:wht-rates"/>\n' +
      '              </content>\n' +
      '            </hcontainer>\n' +
      '          </section>\n' +
      '        </division>\n' +
      '      </part>\n' +
      '    </body>\n' +
      '  </act>\n' +
      '</akomaNtoso>'
    ),
    p("The marker element in section 153 creates a cross-reference to the regpack WHT rate table. At compliance evaluation time, the tensor engine resolves this reference to the current SRO-specified rates, ensuring that the lawpack's static legal obligation is evaluated against the regpack's dynamic rate schedule. This separation of law (lawpack) from current rates (regpack) is the Pack Trilogy's fundamental design principle."),

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

    // --- 6.4.1 SBP Exchange Rate Encoding ---
    h3("6.4.1 SBP Exchange Rate Encoding"),
    p("The SBP (State Bank of Pakistan) publishes daily exchange rates that govern cross-border settlement through PAK corridors. These rates are encoded in a structured format within the regpack, enabling the treasury-info Mass API to apply correct conversion rates for PKR collection and the compliance tensor to verify that settlement amounts fall within permitted thresholds. All monetary values use string-encoded decimals to avoid floating-point canonicalization issues in digest computation."),
    ...codeBlock(
      '// regpack.sbp_exchange_rates.json (fragment)\n' +
      '{\n' +
      '  "source": "sbp.org.pk",\n' +
      '  "effective_date": "2026-02-14",\n' +
      '  "publication_time": "2026-02-14T09:30:00+05:00",\n' +
      '  "base_currency": "PKR",\n' +
      '  "rates": [\n' +
      '    {\n' +
      '      "currency": "USD",\n' +
      '      "buying_tt": "278.50",\n' +
      '      "selling_tt": "279.00",\n' +
      '      "buying_od": "278.20",\n' +
      '      "selling_od": "279.30"\n' +
      '    },\n' +
      '    {\n' +
      '      "currency": "AED",\n' +
      '      "buying_tt": "75.85",\n' +
      '      "selling_tt": "76.00",\n' +
      '      "buying_od": "75.70",\n' +
      '      "selling_od": "76.15"\n' +
      '    },\n' +
      '    {\n' +
      '      "currency": "SAR",\n' +
      '      "buying_tt": "74.20",\n' +
      '      "selling_tt": "74.40",\n' +
      '      "buying_od": "74.05",\n' +
      '      "selling_od": "74.55"\n' +
      '    }\n' +
      '  ],\n' +
      '  "digest_note": "All rates are string-encoded decimals. No floats."\n' +
      '}'
    ),
    p("The distinction between TT (telegraphic transfer) and OD (on-demand) rates is critical for corridor settlement: TT rates apply to electronic transfers through SBP Raast, while OD rates apply to negotiable instruments. The PAK-UAE trade corridor uses buying_tt and selling_tt exclusively."),

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
    p("The top-level Licensepack container holds jurisdiction identity, metadata, license type definitions, individual license records, and license holder profiles. BTreeMap ordering guarantees deterministic iteration for digest computation:"),
    ...codeBlock(
      "/// Content-addressed snapshot of jurisdictional licensing state.\n" +
      "///\n" +
      "/// Completes the pack trilogy:\n" +
      "/// - Lawpack: Static law (statutes, regulations)\n" +
      "/// - Regpack: Dynamic guidance (sanctions, calendars)\n" +
      "/// - Licensepack: Live registry (licenses, holders, conditions)\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Licensepack {\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub name: String,\n" +
      "    pub version: String,\n" +
      "    pub digest: Option<ContentDigest>,\n" +
      "    pub metadata: Option<LicensepackMetadata>,\n" +
      "    pub license_types: BTreeMap<String, LicenseTypeDefinition>,\n" +
      "    pub licenses: BTreeMap<String, License>,\n" +
      "    pub holders: BTreeMap<String, LicenseHolder>,\n" +
      "}"
    ),

    // --- 6.5.1 License Data Model ---
    h3("6.5.1 License Data Model"),
    p("The license data model comprises four layers: status lifecycle, domain classification, individual license records with conditions/permissions/restrictions, and license holder profiles."),

    p_runs([bold("License Status."), " Six status values track the license lifecycle. Terminal states (Revoked, Expired, Surrendered) cannot transition back to Active:"]),
    table(
      ["Status", "Meaning", "Terminal", "Tensor Effect"],
      [
        ["Active", "License in good standing, all conditions met", "No", "COMPLIANT"],
        ["Suspended", "Temporarily non-operative pending review or remediation", "No", "SUSPENDED"],
        ["Pending", "Application submitted, under regulatory review", "No", "PENDING"],
        ["Revoked", "Permanently terminated by regulator for cause", "Yes", "NON_COMPLIANT"],
        ["Expired", "Validity period has elapsed without renewal", "Yes", "NON_COMPLIANT"],
        ["Surrendered", "Voluntarily relinquished by the holder", "Yes", "NON_COMPLIANT"],
      ],
      [1400, 3960, 1200, 2800]
    ),
    ...codeBlock(
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n" +
      "#[serde(rename_all = \"snake_case\")]\n" +
      "pub enum LicenseStatus {\n" +
      "    Active,\n" +
      "    Suspended,\n" +
      "    Revoked,    // terminal\n" +
      "    Expired,    // terminal\n" +
      "    Pending,\n" +
      "    Surrendered, // terminal\n" +
      "}\n" +
      "\n" +
      "impl LicenseStatus {\n" +
      "    pub fn is_terminal(&self) -> bool {\n" +
      "        matches!(self, Self::Revoked | Self::Expired | Self::Surrendered)\n" +
      "    }\n" +
      "}"
    ),

    p_runs([bold("License Domains."), " Six domain categories classify licenses by regulatory scope:"]),
    table(
      ["Domain", "Scope", "Pakistan Examples"],
      [
        ["Financial", "Banking, payments, securities, e-money", "SBP banking license, SECP broker-dealer, EMI authorization"],
        ["Corporate", "Company registration, entity lifecycle", "SECP company registration, NTN binding"],
        ["Professional", "Individual certifications, practitioner licenses", "Legal practitioner, chartered accountant, auditor"],
        ["Trade", "Import/export, customs, bonded warehouses", "Customs broker license, bonded warehouse permit"],
        ["Insurance", "Insurance underwriting, brokerage, reinsurance", "SECP insurance license, reinsurance authorization"],
        ["Mixed", "Multi-domain licenses spanning categories", "SEZ developer license (corporate + trade + financial)"],
      ],
      [1400, 3160, 4800]
    ),
    ...codeBlock(
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n" +
      "#[serde(rename_all = \"snake_case\")]\n" +
      "pub enum LicenseDomain {\n" +
      "    Financial,\n" +
      "    Corporate,\n" +
      "    Professional,\n" +
      "    Trade,\n" +
      "    Insurance,\n" +
      "    Mixed,\n" +
      "}"
    ),

    p_runs([bold("License Record."), " Each License struct represents an individual license issued by a regulator to a holder. The record carries conditions (ongoing requirements), permissions (authorized activities with scope and limits), and restrictions (blocked activities, jurisdictions, products):"]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct License {\n" +
      "    pub license_id: String,\n" +
      "    pub license_type_id: String,\n" +
      "    pub license_number: Option<String>,     // regulator-assigned\n" +
      "    pub status: LicenseStatus,\n" +
      "    pub issued_date: String,\n" +
      "    pub holder_id: String,\n" +
      "    pub holder_legal_name: String,\n" +
      "    pub regulator_id: String,\n" +
      "    pub effective_date: Option<String>,\n" +
      "    pub expiry_date: Option<String>,\n" +
      "    pub holder_did: Option<String>,          // DID for cross-zone lookup\n" +
      "    pub permitted_activities: Vec<String>,\n" +
      "    pub asset_classes_authorized: Vec<String>,\n" +
      "    pub geographic_scope: Vec<String>,\n" +
      "    pub prudential_category: Option<String>,\n" +
      "    pub capital_requirement: BTreeMap<String, String>,\n" +
      "    pub conditions: Vec<LicenseCondition>,\n" +
      "    pub permissions: Vec<LicensePermission>,\n" +
      "    pub restrictions: Vec<LicenseRestriction>,\n" +
      "}"
    ),

    p_runs([bold("Conditions."), " A LicenseCondition represents an ongoing obligation the holder must satisfy, such as minimum capital requirements, operational standards, or reporting frequency. Conditions carry metric, threshold, operator, and currency fields for quantitative evaluation:"]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicenseCondition {\n" +
      "    pub condition_id: String,\n" +
      "    pub condition_type: String,   // \"capital\", \"operational\", \"reporting\"\n" +
      "    pub description: String,\n" +
      "    pub metric: Option<String>,   // e.g., \"minimum_capital\"\n" +
      "    pub threshold: Option<String>, // string decimal for precision\n" +
      "    pub currency: Option<String>,\n" +
      "    pub operator: Option<String>, // \">=\", \"<=\", \"==\"\n" +
      "    pub frequency: Option<String>, // \"continuous\", \"quarterly\", \"annual\"\n" +
      "    pub status: String,           // \"active\", \"waived\", \"expired\"\n" +
      "}"
    ),

    p_runs([bold("Permissions."), " A LicensePermission grants the holder authorization to perform a specific activity, optionally with scope constraints (geographic, product) and limits (transaction size, volume):"]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicensePermission {\n" +
      "    pub permission_id: String,\n" +
      "    pub activity: String,\n" +
      "    pub scope: BTreeMap<String, serde_json::Value>,\n" +
      "    pub limits: BTreeMap<String, serde_json::Value>,\n" +
      "    pub effective_date: Option<String>,\n" +
      "    pub status: String,  // \"active\", \"revoked\"\n" +
      "}"
    ),

    p_runs([bold("Restrictions."), " A LicenseRestriction blocks specific activities, jurisdictions, products, or client types. A wildcard \"*\" in blocked_jurisdictions blocks all except those listed in allowed_jurisdictions:"]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicenseRestriction {\n" +
      "    pub restriction_id: String,\n" +
      "    pub restriction_type: String,  // \"geographic\", \"activity\", \"product\"\n" +
      "    pub description: String,\n" +
      "    pub blocked_jurisdictions: Vec<String>,  // [\"*\"] = all-except-allowed\n" +
      "    pub allowed_jurisdictions: Vec<String>,\n" +
      "    pub blocked_activities: Vec<String>,\n" +
      "    pub blocked_products: Vec<String>,\n" +
      "    pub blocked_client_types: Vec<String>,\n" +
      "    pub max_leverage: Option<String>,\n" +
      "    pub status: String,  // \"active\", \"waived\"\n" +
      "}"
    ),

    p_runs([bold("License Holder."), " The LicenseHolder struct profiles the entity that holds one or more licenses. It carries identity, ownership, and contact information that supports cross-zone verification through DID resolution:"]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicenseHolder {\n" +
      "    pub holder_id: String,\n" +
      "    pub entity_type: String,          // \"company\", \"individual\", \"partnership\"\n" +
      "    pub legal_name: String,\n" +
      "    pub trading_names: Vec<String>,\n" +
      "    pub registration_number: Option<String>,\n" +
      "    pub incorporation_date: Option<String>,\n" +
      "    pub jurisdiction_of_incorporation: Option<String>,\n" +
      "    pub did: Option<String>,           // DID for cross-zone identity\n" +
      "    pub registered_address: BTreeMap<String, String>,\n" +
      "    pub contact: BTreeMap<String, String>,\n" +
      "    pub controllers: Vec<serde_json::Value>,\n" +
      "    pub beneficial_owners: Vec<serde_json::Value>,\n" +
      "    pub group_structure: BTreeMap<String, serde_json::Value>,\n" +
      "}"
    ),

    // --- 6.5.2 License Verification ---
    h3("6.5.2 License Verification"),
    p("The Licensepack.verify_license method performs full authorization verification for a given holder DID, activity, and evaluation date. The verification algorithm proceeds in strict order: (1) resolve all licenses held by the DID, (2) for each license evaluate compliance state, (3) return on first COMPLIANT match, (4) if no COMPLIANT license found, return the best non-compliant state with priority ordering SUSPENDED > PENDING > NON_COMPLIANT."),
    ...codeBlock(
      "impl Licensepack {\n" +
      "    /// Verify if a holder has a valid license for an activity.\n" +
      "    /// Returns (is_valid, compliance_state, matching_license_id).\n" +
      "    pub fn verify_license(\n" +
      "        &self,\n" +
      "        holder_did: &str,\n" +
      "        activity: &str,\n" +
      "        today: &str,\n" +
      "    ) -> (bool, LicenseComplianceState, Option<String>) {\n" +
      "        let licenses = self.get_licenses_by_holder_did(holder_did);\n" +
      "        if licenses.is_empty() {\n" +
      "            return (false, LicenseComplianceState::NonCompliant, None);\n" +
      "        }\n" +
      "        for lic in &licenses {\n" +
      "            let state = lic.evaluate_compliance(activity, today);\n" +
      "            if state == LicenseComplianceState::Compliant {\n" +
      "                return (true, LicenseComplianceState::Compliant,\n" +
      "                        Some(lic.license_id.clone()));\n" +
      "            }\n" +
      "        }\n" +
      "        // Return best non-compliant state found\n" +
      "        let states: Vec<_> = licenses.iter()\n" +
      "            .map(|lic| lic.evaluate_compliance(activity, today)).collect();\n" +
      "        if states.contains(&LicenseComplianceState::Suspended) {\n" +
      "            return (false, LicenseComplianceState::Suspended, None);\n" +
      "        }\n" +
      "        if states.contains(&LicenseComplianceState::Pending) {\n" +
      "            return (false, LicenseComplianceState::Pending, None);\n" +
      "        }\n" +
      "        (false, LicenseComplianceState::NonCompliant, None)\n" +
      "    }\n" +
      "}"
    ),
    p("Per-license compliance evaluation (License.evaluate_compliance) applies six checks in sequence. Each check can short-circuit to a terminal state:"),
    table(
      ["Step", "Check", "Failure State"],
      [
        ["1", "Status is Suspended", "SUSPENDED (immediate return)"],
        ["2", "Status is Pending", "PENDING (immediate return)"],
        ["3", "Status is Revoked, Expired, or Surrendered", "NON_COMPLIANT (immediate return)"],
        ["4", "Expiry date has passed (is_expired check)", "NON_COMPLIANT"],
        ["5", "Activity not in permitted_activities or permissions", "NON_COMPLIANT"],
        ["6", "Active restriction blocks the activity", "NON_COMPLIANT"],
      ],
      [600, 4960, 3800]
    ),
    p("Only if all six checks pass does the license evaluate as COMPLIANT. The separation of permitted_activities (simple string list) from permissions (structured grants with scope and limits) allows both coarse-grained and fine-grained authorization models."),

    // --- 6.5.3 Compliance Tensor Integration ---
    h3("6.5.3 Compliance Tensor Integration"),
    p("Licensepacks populate the LICENSING domain of the Compliance Tensor (§10). The LICENSING domain must reach a permissive state for any corridor operation to proceed. The mapping from license status to tensor state follows the ComplianceState lattice (§10.2): NON_COMPLIANT < SUSPENDED < UNKNOWN < PENDING < COMPLIANT."),
    table(
      ["License Status", "Tensor State", "Effect on Corridor Operations"],
      [
        ["Active (all checks pass)", "COMPLIANT", "Operations permitted; corridor settlement proceeds"],
        ["Suspended", "SUSPENDED", "Operations blocked temporarily; auto-retry on reinstatement"],
        ["Revoked", "NON_COMPLIANT", "Operations blocked permanently for this license"],
        ["Expired", "NON_COMPLIANT", "Operations blocked; renewal required"],
        ["Surrendered", "NON_COMPLIANT", "Operations blocked; new application required"],
        ["Pending", "PENDING", "Limited operations; settlement blocked, reads permitted"],
        ["No license found for DID", "NON_COMPLIANT", "No authorization on record"],
        ["No license required for activity", "EXEMPT", "De minimis exemption; operations permitted"],
      ],
      [2800, 2200, 4360]
    ),
    ...codeBlock(
      "/// Compliance tensor states for the LICENSING domain.\n" +
      "///\n" +
      "/// Lattice ordering: NON_COMPLIANT < SUSPENDED < UNKNOWN < PENDING < COMPLIANT\n" +
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]\n" +
      "#[serde(rename_all = \"SCREAMING_SNAKE_CASE\")]\n" +
      "pub enum LicenseComplianceState {\n" +
      "    Compliant,\n" +
      "    NonCompliant,\n" +
      "    Pending,\n" +
      "    Suspended,\n" +
      "    Unknown,\n" +
      "}"
    ),
    p("The evaluate_license_compliance utility function provides a standalone entry point for the compliance tensor engine to query license state without constructing a full verification context:"),
    ...codeBlock(
      "/// Evaluate licensing compliance for an activity.\n" +
      "/// Used by the compliance tensor to populate the LICENSING domain.\n" +
      "pub fn evaluate_license_compliance(\n" +
      "    license_id: &str,\n" +
      "    activity: &str,\n" +
      "    licensepack: &Licensepack,\n" +
      "    today: &str,\n" +
      ") -> LicenseComplianceState {\n" +
      "    match licensepack.get_license(license_id) {\n" +
      "        Some(license) => license.evaluate_compliance(activity, today),\n" +
      "        None => LicenseComplianceState::NonCompliant,\n" +
      "    }\n" +
      "}"
    ),

    // --- 6.5.4 Licensepack Schemas ---
    h3("6.5.4 Licensepack Schemas"),
    p("Three JSON schemas define the complete licensepack structure. These schemas are part of the 116 schemas in msez-schema and are validated at pack build time, pack load time, and before digest computation:"),
    table(
      ["Schema", "Purpose", "Key Definitions"],
      [
        ["licensepack.schema.json", "Top-level pack structure and metadata", "Licensepack container, LicensepackMetadata, LicensepackRegulator, snapshot_type, delta, sources, normalization"],
        ["licensepack.license.schema.json", "Individual license records", "License, LicenseTypeDefinition, LicenseCondition, LicensePermission, LicenseRestriction, LicenseHolder"],
        ["licensepack.lock.schema.json", "Version pinning and artifact binding", "LicensepackLock, LicensepackLockInfo, LicensepackArtifactInfo, digest_sha256, URI, byte_length"],
      ],
      [2800, 2200, 4360]
    ),
    p("The lock schema enables reproducible builds. A licensepack lock file captures the exact digest, artifact URI, and byte length at build time, ensuring that subsequent loads retrieve the identical content-addressed snapshot:"),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicensepackLock {\n" +
      "    pub lock_version: String,\n" +
      "    pub generated_at: String,\n" +
      "    pub generator: String,\n" +
      "    pub generator_version: String,\n" +
      "    pub licensepack: LicensepackLockInfo,\n" +
      "    pub artifact: LicensepackArtifactInfo,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicensepackLockInfo {\n" +
      "    pub licensepack_id: String,\n" +
      "    pub jurisdiction_id: String,\n" +
      "    pub domain: String,\n" +
      "    pub as_of_date: String,\n" +
      "    pub digest_sha256: String,\n" +
      "}\n" +
      "\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct LicensepackArtifactInfo {\n" +
      "    pub artifact_type: String,\n" +
      "    pub digest_sha256: String,\n" +
      "    pub uri: String,\n" +
      "    pub media_type: String,\n" +
      "    pub byte_length: i64,\n" +
      "}"
    ),

    // --- 6.5.5 Zone Integration ---
    h3("6.5.5 Zone Integration"),
    p("Zones specify licensepack requirements in zone.yaml through LicensepackRef entries. Each reference binds a jurisdiction, domain, and digest to the zone, enabling the zone build process to resolve and verify the exact licensing state snapshot that applies. The resolve_licensepack_refs function parses zone manifests and validates digest format before constructing references:"),
    ...codeBlock(
      "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub struct LicensepackRef {\n" +
      "    pub jurisdiction_id: String,\n" +
      "    pub domain: String,\n" +
      "    pub licensepack_digest_sha256: String,\n" +
      "    pub as_of_date: Option<String>,\n" +
      "}"
    ),
    p("Refresh policies are configured per domain within the zone specification. Higher-risk domains require more frequent updates:"),
    table(
      ["Domain", "Refresh Interval", "Max Staleness", "Rationale"],
      [
        ["Financial", "1 hour", "4 hours", "License revocations can occur intraday; settlement must verify current state"],
        ["Corporate", "6 hours", "24 hours", "Entity status changes are less frequent but affect formation operations"],
        ["Trade", "6 hours", "24 hours", "Import/export permits may be suspended on short notice"],
        ["Professional", "24 hours", "72 hours", "Individual certifications change infrequently"],
        ["Insurance", "6 hours", "24 hours", "Underwriting authorization affects settlement guarantees"],
        ["Mixed", "1 hour", "4 hours", "Multi-domain licenses inherit the strictest refresh policy"],
      ],
      [1400, 1800, 1800, 4360]
    ),
    p("A zone manifest example showing licensepack binding for the PAK-KP-RSEZ zone:"),
    ...codeBlock(
      '# zone.yaml (fragment)\n' +
      'zone_id: "pk-kp-rsez"\n' +
      'jurisdiction_id: "pk"\n' +
      'licensepacks:\n' +
      '  - jurisdiction_id: "pk"\n' +
      '    domain: "financial"\n' +
      '    licensepack_digest_sha256: "a1b2c3...64hex..."\n' +
      '    as_of_date: "2026-02-14"\n' +
      '  - jurisdiction_id: "pk"\n' +
      '    domain: "corporate"\n' +
      '    licensepack_digest_sha256: "d4e5f6...64hex..."\n' +
      '    as_of_date: "2026-02-14"\n' +
      '  - jurisdiction_id: "pk"\n' +
      '    domain: "trade"\n' +
      '    licensepack_digest_sha256: "789abc...64hex..."\n' +
      '    as_of_date: "2026-02-14"\n' +
      'licensepack_refresh:\n' +
      '  financial:\n' +
      '    interval_seconds: 3600\n' +
      '    max_staleness_seconds: 14400\n' +
      '  default:\n' +
      '    interval_seconds: 86400\n' +
      '    max_staleness_seconds: 86400'
    ),

    // --- 6.6 Licensepack Digest Computation ---
    h2("6.6 Licensepack Digest Computation"),
    p("Licensepack digests follow the same content-addressed pattern as lawpack and regpack digests: deterministic canonicalization via CanonicalBytes, SHA-256 hashing over a versioned prefix, and BTreeMap-ordered iteration. The digest algorithm processes components in a fixed sequence to guarantee cross-platform reproducibility:"),
    ...codeBlock(
      'SHA256(\n' +
      '    b"msez-licensepack-v1\\0"\n' +
      '    + canonical(metadata) + b"\\0"\n' +
      '    + for each license_type in sorted(license_types.keys()):\n' +
      '        "license-types/{type_id}\\0" + canonical(type_data) + b"\\0"\n' +
      '    + for each license in sorted(licenses.keys()):\n' +
      '        "licenses/{license_id}\\0" + canonical(license_data) + b"\\0"\n' +
      '        + for each condition in sorted(conditions, by condition_id):\n' +
      '            "licenses/{id}/conditions/{cid}\\0" + canonical(cond) + b"\\0"\n' +
      '        + for each permission in sorted(permissions, by permission_id):\n' +
      '            "licenses/{id}/permissions/{pid}\\0" + canonical(perm) + b"\\0"\n' +
      '        + for each restriction in sorted(restrictions, by restriction_id):\n' +
      '            "licenses/{id}/restrictions/{rid}\\0" + canonical(rest) + b"\\0"\n' +
      '    + for each holder in sorted(holders.keys()):\n' +
      '        "holders/{holder_id}\\0" + canonical(holder_data) + b"\\0"\n' +
      ')'
    ),
    p("Key invariants: (1) All canonicalization goes through CanonicalBytes::from_value, which rejects floating-point numbers to ensure deterministic JSON serialization. (2) BTreeMap iteration provides sorted key ordering without explicit sort steps. (3) Null-byte delimiters prevent prefix collisions. (4) Adding, removing, or modifying any license, condition, permission, restriction, or holder produces a different digest. (5) The compute_delta method compares two licensepack snapshots to produce a structured diff (licenses granted, revoked, suspended, reinstated) for audit trails."),
  ];
};
