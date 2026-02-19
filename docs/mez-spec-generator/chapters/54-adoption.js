const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter54() {
  return [
    ...partHeading("PART XVIII: NETWORK DIFFUSION"),

    chapterHeading("Chapter 54: Adoption Strategy"),

    // --- 54.1 Target Segments ---
    h2("54.1 Target Segments"),
    table(
      ["Segment", "Entry Point", "Value Proposition"],
      [
        ["Sovereign Governments", "National GovOS deployment", "Tax revenue optimization, compliance automation, digital transformation"],
        ["Free Zone Authorities", "Digital free zone stack", "Rapid zone deployment, automated licensing, corridor connectivity"],
        ["Financial Centers", "Capital markets + corridors", "Cross-border settlement, compliance verification, institutional infrastructure"],
        ["Development Finance", "EZ-in-a-Box for emerging markets", "Rapid economic zone creation, investment facilitation"],
        ["Corporate Service Providers", "Formation + compliance modules", "Automated corporate services, multi-jurisdiction operations"],
      ],
      [2400, 2800, 4160]
    ),

    // --- 54.2 Network Bootstrapping ---
    h2("54.2 Network Bootstrapping"),
    p("This chapter describes the target market analysis informing the system's design. Network effects compound at each deployment layer. A single jurisdiction deployment provides entity formation, compliance evaluation, and credential issuance. Adding a second jurisdiction enables bilateral corridors with cross-border trade, settlement netting, and mutual recognition of credentials. At three or more jurisdictions, multilateral corridors emerge with triangular netting, shared compliance tensors, and network-wide sanctions screening. The bootstrapping strategy targets anchor jurisdictions with existing bilateral trade volumes exceeding $10B annually, ensuring immediate corridor utility. Each new jurisdiction added to the network increases the value for all existing participants by expanding the set of available corridors, compliance domains, and recognized credentials."),

    h3("54.2.1 Network Value Progression"),
    table(
      ["Jurisdictions", "Capabilities Unlocked", "Example"],
      [
        ["1", "Entity formation, compliance evaluation, credential issuance", "Pakistan GovOS standalone: 40+ ministries, FBR integration"],
        ["2", "Bilateral corridors, cross-border settlement, mutual credential recognition", "PAK\u2194UAE: $10.1B trade, SWIFT pacs.008, receipt chain sync"],
        ["3+", "Multilateral corridors, triangular netting, shared compliance tensors, network-wide sanctions", "PAK\u2194UAE\u2194KSA: triangular trade netting, shared AML/CFT screening"],
      ],
      [1400, 4200, 3760]
    ),
  ];
};
