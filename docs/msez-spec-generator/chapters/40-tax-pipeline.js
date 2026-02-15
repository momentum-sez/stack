const {
  chapterHeading,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter40() {
  return [
    chapterHeading("Chapter 40: Tax Collection Pipeline"),

    p("Every economic activity on Mass generates a tax event. The pipeline operates as follows:"),

    table(
      ["Stage", "Action", "System"],
      [
        ["1. Transaction", "Economic activity occurs via Mass Treasury API", "Mass Fiscal"],
        ["2. Tax Identification", "MSEZ Pack Trilogy identifies applicable tax rules", "MSEZ Pack Trilogy"],
        ["3. Withholding", "Automatic withholding at source per WHT schedule", "Mass Fiscal + MSEZ Bridge"],
        ["4. Reporting", "Real-time reporting to FBR IRIS", "National System Integration"],
        ["5. Gap Analysis", "AI-powered gap analysis identifies evasion patterns", "Sovereign AI Spine"],
        ["6. Enforcement", "Automated compliance actions for non-filing entities", "GovOS Console"],
      ],
      [1800, 4200, 3360]
    ),

    spacer(),

    p("The pipeline processes four tax categories for the Pakistan deployment:"),
    table(
      ["Category", "Legislation", "WHT Rates", "Filing Frequency"],
      [
        ["Income Tax", "Income Tax Ordinance 2001", "Variable by section (e.g., \u00a7153: 4-15% on services/supplies)", "Monthly advance, annual return"],
        ["Sales Tax / GST", "Sales Tax Act 1990", "Standard 18%, reduced rates per SRO", "Monthly return"],
        ["Federal Excise", "Federal Excise Act 2005", "Category-specific rates", "Monthly return"],
        ["Customs Duty", "Customs Act 1969", "Tariff schedule, CPEC preferences", "Per-consignment"],
      ],
      [1800, 2400, 2800, 2360]
    ),
    spacer(),

    p("Withholding tax at source is the pipeline's primary collection mechanism. Every payment processed through Mass Fiscal triggers a WHT lookup against the regpack's withholding table. The lookup considers the payee's NTN status (filers receive reduced rates), the payment category (services, supplies, contracts, rent), the payment amount (de minimis exemptions), and any applicable SRO exemptions. The WHT amount is deducted atomically from the payment and deposited into FBR's designated account via SBP Raast, with a TaxEvent receipt appended to the payer's chain."),
  ];
};
