const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter34() {
  return [
    chapterHeading("Chapter 34: Tax and Revenue Module Family"),

    // --- 34.1 Module Overview ---
    h2("34.1 Module Overview"),
    p("The Tax and Revenue module family implements jurisdiction-specific tax computation, withholding, collection, and reporting. This module family provides a complete framework for tax event generation, computation, and settlement through the Mass treasury API."),

    // --- 34.2 Tax Framework Module ---
    h2("34.2 Tax Framework Module"),
    p("The Tax Framework Module defines jurisdiction-specific tax rules, rates, thresholds, and exemptions. It consumes lawpack data (e.g., Income Tax Ordinance 2001, Sales Tax Act 1990) and regpack data (FBR SROs, SBP circulars) to maintain current tax parameters. Tax computation is triggered automatically via the agentic framework (§45) on every qualifying transaction."),
    p_runs([bold("Pakistan Tax Collection Pipeline."), " Every transaction through a Pakistan EZ generates tax events: withholding tax (WHT) on payments per Section 153, sales tax on services per provincial laws, capital gains tax on securities transfers per Section 37A, and customs duty exemptions per EZ Act 2012. The pipeline computes applicable taxes, creates withholding entries via treasury-info.api.mass.inc, and generates tax certificates as Verifiable Credentials."]),
    ...codeBlock(
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct TaxEvent {\n" +
      "    pub event_id: TaxEventId,\n" +
      "    pub event_type: TaxEventType,\n" +
      "    pub transaction_ref: TransactionId,\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub taxable_amount: Amount,\n" +
      "    pub tax_rate: Decimal,\n" +
      "    pub tax_amount: Amount,\n" +
      "    pub withholding_required: bool,\n" +
      "    pub due_date: DateTime<Utc>,\n" +
      "    pub status: TaxEventStatus,\n" +
      "}"
    ),

    p_runs([bold("Tax Regime Types."), " The Tax Framework Module supports the following regime types, each defining how taxable income is sourced and computed. Jurisdiction packs declare which regime applies, and the tensor evaluator selects the correct computation path accordingly."]),
    table(
      ["Regime Type", "Description", "Example Jurisdictions"],
      [
        ["Territorial", "Taxes only income sourced within the jurisdiction; foreign-source income is exempt", "Hong Kong, Singapore, Panama"],
        ["Worldwide", "Taxes all income regardless of source; foreign tax credits may apply to avoid double taxation", "United States, India, Brazil"],
        ["Hybrid", "Combines territorial and worldwide elements; typically territorial for active business income, worldwide for passive income", "United Kingdom, Japan, France"],
        ["Flat Rate", "Applies a single uniform tax rate to all taxable income regardless of amount or source", "Estonia (20%), Georgia (20%), Hungary (9%)"],
        ["Progressive", "Applies increasing tax rates to successive income brackets; higher income taxed at higher rates", "Pakistan, Germany, Australia"],
        ["Economic Zone", "Provides preferential tax treatment within designated zones; may include full exemptions or reduced rates for qualifying activities", "Pakistan EZs (EZ Act 2012), UAE Free Zones, China EZs"],
      ],
      [1800, 4560, 3000]
    ),

    // --- 34.2.1 Fee Schedules Module ---
    h3("34.2.1 Fee Schedules Module"),
    p("The Fee Schedules Module manages EZ-specific fee structures for entity formation, annual maintenance, transaction processing, and corridor usage. Fee schedules are jurisdiction-specific and may include tiered pricing, volume discounts, and incentive program credits. All fees are collected through treasury-info.api.mass.inc with full audit trail."),
    p_runs([bold("Fee Schedule Examples."), " The following table illustrates typical fee ranges across EZ jurisdictions. Actual fees are loaded from the regpack for each jurisdiction and may be adjusted by incentive program credits."]),
    table(
      ["Fee Category", "Typical Range", "Billing Frequency", "Notes"],
      [
        ["Entity Formation", "$500 - $5,000", "One-time", "Varies by entity type (LLC, PLC, branch); includes government filing fees"],
        ["Annual Maintenance", "$200 - $2,000", "Annual", "Covers registered agent, compliance filing, annual return preparation"],
        ["Transaction Processing", "0.1% - 0.5%", "Per transaction", "Applied to treasury operations; tiered by volume with caps"],
        ["Corridor Usage", "$50 - $500", "Per corridor operation", "Cross-border corridor fees; includes compliance tensor evaluation"],
        ["License Renewal", "$100 - $1,000", "Annual", "Per license type; regulator-specific fees passed through"],
        ["Document Certification", "$25 - $150", "Per document", "VC issuance for trade documents, tax certificates, compliance attestations"],
      ],
      [2000, 1800, 1800, 3760]
    ),

    // --- 34.2.2 Incentive Programs Module ---
    h3("34.2.2 Incentive Programs Module"),
    p("The Incentive Programs Module tracks tax holidays, reduced rates, exemptions, and credits available within EZ jurisdictions. It evaluates entity eligibility based on formation date, activity type, investment amount, and employment targets. Incentive claims are verified against lawpack provisions (e.g., EZ Act 2012 Schedule II exemptions) and issued as Verifiable Credentials."),
    p_runs([bold("Incentive Program Types."), " The following programs are supported. Each program type defines eligibility criteria, benefit computation, duration limits, and clawback conditions. The compliance tensor evaluates program applicability per entity per jurisdiction."]),
    table(
      ["Program", "Mechanism", "Typical Duration", "Eligibility Criteria"],
      [
        ["Tax Holiday", "Full exemption from income tax for a defined period after formation or investment", "5 - 10 years", "New entity in designated EZ; minimum capital investment; approved activity sector"],
        ["Reduced Rate", "Lower tax rate applied instead of standard rate for qualifying income", "Indefinite or time-limited", "Entity operating within EZ; income derived from approved activities"],
        ["Investment Allowance", "Accelerated depreciation or additional deduction on qualifying capital expenditure", "Per asset useful life", "Capital investment in plant, machinery, or technology above minimum threshold"],
        ["R&D Credit", "Tax credit computed as a percentage of qualifying research and development expenditure", "Annual claim", "R&D expenditure exceeding baseline; approved research activities per lawpack"],
        ["Employment Incentive", "Tax credit or deduction for each qualifying employee hired above baseline headcount", "2 - 5 years per hire", "New hires in EZ; minimum employment duration; local workforce percentage targets"],
        ["Export Incentive", "Reduced rate or exemption on income derived from qualifying export transactions", "Linked to export volume", "Export revenue exceeding threshold; goods/services shipped outside jurisdiction via corridor"],
      ],
      [1600, 3000, 1600, 3160]
    ),

    // --- 34.2.3 International Reporting Module ---
    h3("34.2.3 International Reporting Module"),
    p("The International Reporting Module generates cross-border tax reports including CRS (Common Reporting Standard) disclosures, FATCA reporting, transfer pricing documentation, and country-by-country reporting. It aggregates data from Mass APIs through mez-mass-client and applies jurisdiction-specific reporting formats and thresholds from the regpack."),
    p_runs([bold("CRS and FATCA Reporting Thresholds."), " The module applies jurisdiction-specific reporting thresholds to determine which accounts and entities must be reported. Under CRS, individual accounts with aggregate balances below $250,000 USD equivalent at year-end are subject to simplified due diligence, while those above require enhanced procedures. Entity accounts with aggregate balances above $1,000,000 USD equivalent require look-through to controlling persons. Under FATCA, thresholds for US persons are $50,000 (domestic institutions) or $200,000/$300,000 (foreign institutions, single/joint). The module automatically classifies accounts against these thresholds using balance data from treasury-info.api.mass.inc and beneficial ownership data from organization-info.api.mass.inc."]),
  ];
};
