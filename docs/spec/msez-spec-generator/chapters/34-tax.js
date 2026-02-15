const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, spacer
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

    // --- 34.4 Incentive Programs Module ---
    h2("34.4 Incentive Programs Module"),
    p("The Incentive Programs Module tracks tax holidays, reduced rates, exemptions, and credits available within SEZ jurisdictions. It evaluates entity eligibility based on formation date, activity type, investment amount, and employment targets. Incentive claims are verified against lawpack provisions (e.g., SEZ Act 2012 Schedule II exemptions) and issued as Verifiable Credentials."),

    // --- 34.5 International Reporting Module ---
    h2("34.5 International Reporting Module"),
    p("The International Reporting Module generates cross-border tax reports including CRS (Common Reporting Standard) disclosures, FATCA reporting, transfer pricing documentation, and country-by-country reporting. It aggregates data from Mass APIs through msez-mass-client and applies jurisdiction-specific reporting formats and thresholds from the regpack."),
  ];
};
