const { chapterHeading, table, spacer, pageBreak, p } = require("../lib/primitives");

module.exports = function build_appendixK() {
  return [
    chapterHeading("Appendix K: GovOS Deployment Checklist"),
    table(
      ["Phase", "Milestone", "Verification"],
      [
        ["1.1", "Core infrastructure deployed (compute, storage, network)", "msez verify --all passes"],
        ["1.2", "National identity system connected (NADRA for Pakistan)", "Identity API handshake confirmed"],
        ["1.3", "Tax authority connected (FBR IRIS for Pakistan)", "Test tax event round-trip"],
        ["1.4", "Central bank connected (SBP Raast for Pakistan)", "Test payment round-trip"],
        ["1.5", "Corporate registry connected (SECP for Pakistan)", "Test entity formation round-trip"],
        ["2.1", "Pack Trilogy imported for national law", "msez pack verify --all passes"],
        ["2.2", "All ministry accounts provisioned (40+ for Pakistan)", "Ministry dashboard access confirmed"],
        ["2.3", "First corridor activated", "Cross-border test transaction succeeds"],
        ["3.1", "AI models trained on national data", "Revenue projection accuracy > 85%"],
        ["3.2", "Tax gap reduction measured", "Baseline vs. 6-month comparison report"],
        ["4.1", "Pakistani engineering team certified", "All runbook procedures demonstrated"],
        ["4.2", "Full operational handover", "Momentum advisory-only SLA signed"],
      ],
      [800, 4400, 4160]
    ),
    spacer(),
    pageBreak(),
    p("End of Specification", { bold: true, size: 28 }),
    spacer(200),
    p("Momentum Open Source SEZ Stack"),
    p("Version 0.4.44 \u2014 GENESIS"),
    p("February 2026"),
    spacer(200),
    p("For questions or feedback, contact:"),
    p("Momentum"),
    p("https://github.com/momentum-sez/stack"),
    p("research@momentum.inc"),
  ];
};
