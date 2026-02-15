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
  ];
};
