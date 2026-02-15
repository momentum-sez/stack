const {
  partHeading, chapterHeading, h2,
  p,
  table, spacer
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
        ["Development Finance", "SEZ-in-a-box for emerging markets", "Rapid economic zone creation, investment facilitation"],
        ["Corporate Service Providers", "Formation + compliance modules", "Automated corporate services, multi-jurisdiction operations"],
      ],
      [2400, 2800, 4160]
    ),
    spacer(),

    // --- 54.2 Network Bootstrapping ---
    h2("54.2 Network Bootstrapping"),
    p("Network effects compound at each deployment layer. A single jurisdiction deployment provides entity formation, compliance evaluation, and credential issuance. Adding a second jurisdiction enables bilateral corridors with cross-border trade, settlement netting, and mutual recognition of credentials. At three or more jurisdictions, multilateral corridors emerge with triangular netting, shared compliance tensors, and network-wide sanctions screening. The bootstrapping strategy targets anchor jurisdictions with existing bilateral trade volumes exceeding $10B annually, ensuring immediate corridor utility. Each new jurisdiction added to the network increases the value for all existing participants by expanding the set of available corridors, compliance domains, and recognized credentials."),
  ];
};
