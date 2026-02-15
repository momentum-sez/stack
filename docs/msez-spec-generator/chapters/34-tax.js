const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter34() {
  return [
    chapterHeading("Chapter 34: Tax and Revenue Module Family"),

    // --- 34.1 Module Overview ---
    h2("34.1 Module Overview"),
    p("The Tax and Revenue module family implements jurisdiction-specific tax computation, withholding, collection, and reporting. This module family addresses defect P1-009 (tax collection pipeline) by providing a complete framework for tax event generation, computation, and settlement through the Mass treasury API."),

    // --- 34.2 Tax Framework Module ---
    h2("34.2 Tax Framework Module"),
    p("The Tax Framework Module defines jurisdiction-specific tax rules, rates, thresholds, and exemptions. It consumes lawpack data (e.g., Income Tax Ordinance 2001, Sales Tax Act 1990) and regpack data (FBR SROs, SBP circulars) to maintain current tax parameters. Tax computation is triggered automatically via msez-agentic on every qualifying transaction."),

    h3("34.2.1 Tax Regime Types"),
    p("Six regime types model the full spectrum of tax frameworks across jurisdictions:"),
    table(
      ["Regime Type", "Description", "Example"],
      [
        ["Territorial", "Taxes only income sourced within the jurisdiction", "UAE (no personal income tax), Hong Kong (territorial basis)"],
        ["Worldwide", "Taxes global income of residents with foreign tax credits", "Pakistan (ITO 2001), United States"],
        ["Exemption", "Full exemption from specified tax categories within SEZ", "Pakistan SEZ Act 2012 Schedule II (10-year income tax exemption)"],
        ["Withholding", "Tax collected at source on specified payment types", "Pakistan WHT per ITO 2001 \u00a7153, UAE 0% WHT"],
        ["Consumption", "VAT/GST/Sales tax on goods and services", "Pakistan Sales Tax Act 1990 (17% standard rate)"],
        ["Transfer Pricing", "Arm's length pricing for related-party transactions", "OECD guidelines, Pakistan ITO 2001 \u00a7108"],
      ],
      [2200, 4200, 2960]
    ),
    spacer(),

    p_runs([bold("Pakistan Tax Collection Pipeline."), " Every transaction through a Pakistan SEZ generates tax events: withholding tax (WHT) on payments per Section 153, sales tax on services per provincial laws, capital gains tax on securities transfers per Section 37A, and customs duty exemptions per SEZ Act 2012. The pipeline computes applicable taxes, creates withholding entries via treasury-info.api.mass.inc, and generates tax certificates as Verifiable Credentials."]),
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
    spacer(),

    // --- 34.3 Fee Schedules Module ---
    h2("34.3 Fee Schedules Module"),
    p("The Fee Schedules Module manages SEZ-specific fee structures for entity formation, annual maintenance, transaction processing, and corridor usage. Fee schedules are jurisdiction-specific and may include tiered pricing, volume discounts, and incentive program credits. All fees are collected through treasury-info.api.mass.inc with full audit trail."),
    table(
      ["Fee Category", "Structure", "Collection Method"],
      [
        ["Formation Fees", "Fixed per entity type (e.g., PKR 15,000 for PLC)", "One-time at registration via treasury API"],
        ["Annual Maintenance", "Tiered by authorized capital", "Annual invoice, agentic reminder at 90/60/30 days"],
        ["Transaction Fees", "Basis points on transaction value (1-10 bps)", "Real-time deduction per transaction"],
        ["Corridor Usage", "Per-transaction + monthly minimum", "Aggregated monthly billing"],
        ["License Fees", "Per license type per authority schedule", "Per authority schedule, renewal-linked"],
        ["Filing Fees", "Fixed per filing type", "At time of filing submission"],
      ],
      [2200, 3600, 3560]
    ),
    spacer(),

    // --- 34.4 Incentive Programs Module ---
    h2("34.4 Incentive Programs Module"),
    p("The Incentive Programs Module tracks tax holidays, reduced rates, exemptions, and credits available within SEZ jurisdictions. It evaluates entity eligibility based on formation date, activity type, investment amount, and employment targets. Incentive claims are verified against lawpack provisions and issued as Verifiable Credentials."),
    table(
      ["Incentive Type", "Pakistan SEZ Example", "Duration"],
      [
        ["Income Tax Exemption", "100% exemption per SEZ Act 2012 Schedule II", "10 years from commercial production"],
        ["Customs Duty Exemption", "Zero duty on capital goods imports for SEZ enterprises", "Duration of SEZ status"],
        ["Sales Tax Exemption", "Zero-rated supplies within SEZ", "Duration of SEZ status"],
        ["Capital Gains Relief", "Reduced CGT rate on SEZ enterprise shares", "5 years from listing"],
        ["Stamp Duty Waiver", "Exemption on property transfers within SEZ", "Duration of SEZ status"],
        ["Accelerated Depreciation", "Enhanced first-year allowances on plant and machinery", "First 3 years of operation"],
      ],
      [2200, 4200, 2960]
    ),
    spacer(),

    // --- 34.5 International Reporting Module ---
    h2("34.5 International Reporting Module"),
    p("The International Reporting Module generates cross-border tax reports including CRS, FATCA, transfer pricing documentation, and country-by-country reporting."),

    h3("34.5.1 CRS Reporting"),
    p("Common Reporting Standard (CRS) compliance requires automatic exchange of financial account information with treaty partner jurisdictions. The module identifies reportable accounts (accounts held by non-residents with balances above de minimis thresholds), extracts required data points (name, address, TIN, account balance, income), formats reports per OECD CRS XML schema, and transmits to the jurisdiction's competent authority for exchange."),

    h3("34.5.2 FATCA Reporting"),
    p("Foreign Account Tax Compliance Act (FATCA) reporting identifies U.S. persons holding accounts in non-U.S. financial institutions. The module performs U.S. indicia checks (citizenship, residence, phone numbers, standing instructions), classifies entities under FATCA categories (participating FFI, deemed-compliant, NFFE), generates Form 8966 equivalent reports, and handles recalcitrant account procedures."),

    h3("34.5.3 Transfer Pricing"),
    p("Transfer pricing documentation is generated automatically for related-party transactions crossing jurisdictional boundaries through corridors. The module captures transaction details, identifies comparable uncontrolled transactions, applies the most appropriate method (CUP, resale price, cost plus, TNMM, profit split), and generates master file, local file, and country-by-country report per OECD BEPS Action 13."),
  ];
};
