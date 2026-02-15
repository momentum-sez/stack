const {
  chapterHeading, h2,
  p, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter25() {
  return [
    chapterHeading("Chapter 25: Live Corridors"),

    // --- 25.1 PAK<->KSA Corridor ---
    h2("25.1 PAK\u2194KSA Corridor ($5.4B Bilateral)"),
    p("Saudi-Pakistan bilateral trade totals $5.4B annually. The corridor automates customs duties, withholding tax on remittances from 2.5M Pakistani diaspora, and trade documentation. Status: launch phase under the Saudi-Pakistan SMDA 2025 framework."),
    table(
      ["Component", "Implementation"],
      [
        ["Customs Automation", "HS code harmonization, duty calculation, preferential rates under bilateral agreements"],
        ["Remittance WHT", "Automatic withholding on $2.1B annual remittances per ITO 2001 Schedule"],
        ["Diaspora Services", "NTN registration, tax filing for 2.5M Pakistanis in KSA"],
        ["Trade Docs", "Electronic bills of lading, certificates of origin, phytosanitary certificates"],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- 25.2 PAK<->UAE Corridor ---
    h2("25.2 PAK\u2194UAE Corridor ($10.1B Bilateral)"),
    p("UAE-Pakistan bilateral trade totals $10.1B annually with $6.7B in remittances. Mass operates in 27 Dubai Free Zones through the Dubai Free Zone Council integration. The SIFC (Special Investment Facilitation Council) FDI pipeline channels investment through the corridor. Status: live and processing transactions."),
    table(
      ["Component", "Implementation"],
      [
        ["Free Zone Integration", "27 Dubai free zones connected via DFZC. Mass APIs serve entity + fiscal; MSEZ provides zone-specific licensing per free zone authority."],
        ["Remittance Processing", "$6.7B annual remittances via SBP Raast + UAE Central Bank. Automatic WHT per ITO 2001 \u00a7153. SWIFT pacs.008 adapter for cross-border payments."],
        ["FDI Pipeline", "SIFC investment proposals routed through corridor. Compliance tensor evaluates across SECURITIES, CORPORATE, and AML_CFT domains."],
        ["ADGM Integration", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- 25.3 PAK<->CHN Corridor ---
    h2("25.3 PAK\u2194CHN Corridor ($23.1B Bilateral)"),
    p("China-Pakistan trade totals $23.1B annually, primarily through CPEC 2.0 (China-Pakistan Economic Corridor). The corridor targets nine SEZs, Gwadar Port customs operations, and full e-trade documentation. Integration with SAFE (State Administration of Foreign Exchange) compliance requirements is planned for the cross-border payment leg. Status: planned, with infrastructure build underway."),
    table(
      ["Corridor", "Volume", "Status", "Key Features"],
      [
        ["PAK\u2194KSA", "$5.4B", "Launch", "Customs automation, remittance WHT, diaspora services, trade docs"],
        ["PAK\u2194UAE", "$10.1B", "Live", "27 free zones, SIFC FDI pipeline, $6.7B remittances"],
        ["PAK\u2194CHN", "$23.1B", "Planned", "CPEC 2.0, 9 SEZs, Gwadar customs, e-trade documentation"],
      ],
      [1600, 1400, 1200, 5160]
    ),
    spacer(),
  ];
};
