const {
  chapterHeading, h2, h3,
  p, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter25() {
  return [
    chapterHeading("Chapter 25: Live Corridors"),

    // --- 25.1 PAK<->KSA Corridor ---
    h2("25.1 PAK\u2194KSA Corridor ($5.4B Bilateral)"),
    p("Saudi-Pakistan bilateral trade totals $5.4B annually. The corridor automates customs duties, withholding tax on remittances from 2.5M Pakistani diaspora, and trade documentation. Status: launch phase under the Saudi-Pakistan SMDA 2025 framework."),

    h3("25.1.1 Strategic Importance"),
    p("The PAK\u2194KSA corridor is the anchor bilateral for Momentum's Middle East strategy. Saudi Arabia hosts 2.5 million Pakistani workers who remit $2.1B annually, making it the second-largest remittance source after the UAE. The Saudi-Pakistan Special Measures for Development Agreement (SMDA) signed in 2025 provides the legal framework for digital customs automation, cross-border tax compliance, and diaspora financial services. Vision 2030 reforms in Saudi Arabia have created new opportunities for Pakistani enterprises in construction, IT services, and logistics, all of which require automated entity formation, tax withholding, and trade documentation that the corridor provides. The corridor also serves as Momentum's proving ground for integrating with national-level customs systems before expanding to more complex multilateral configurations."),

    h3("25.1.2 Component Architecture"),
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
    p("UAE-Pakistan bilateral trade totals $10.1B annually with $6.7B in remittances. Mass operates in 27 Dubai Free Zones. The SIFC FDI pipeline channels investment through the corridor. Status: live."),

    h3("25.2.1 Strategic Importance"),
    p("The PAK\u2194UAE corridor is Momentum's highest-volume live corridor and the operational backbone of cross-border financial infrastructure between South Asia and the Gulf. The UAE is Pakistan's largest remittance source at $6.7B annually, driven by 1.7 million Pakistani workers concentrated in Dubai, Abu Dhabi, and Sharjah. Beyond remittances, bilateral merchandise trade of $10.1B spans petroleum products, textiles, rice, and re-exports, generating continuous demand for automated compliance, duty calculation, and trade documentation. The Special Investment Facilitation Council (SIFC) established by Pakistan in 2023 has designated the UAE as a priority FDI source, creating a structured pipeline for Emirati capital into Pakistani infrastructure, technology, and agriculture. Momentum's presence across 27 Dubai Financial Zone Council (DFZC) free zones means that entity formation, fiscal operations, and compliance verification are already integrated into the corridor at scale. This corridor demonstrates the full Mass-to-SEZ Stack orchestration: every entity formed in a DFZC zone triggers compliance tensor evaluation, lawpack checking against UAE Commercial Companies Law and Pakistani FDI regulations, and automatic issuance of formation VCs that are recognized by both jurisdictions."),

    h3("25.2.2 Component Architecture"),
    table(
      ["Component", "Implementation"],
      [
        ["Free Zone Integration", "27 DFZC zones with full Mass API integration for entity formation (organization-info) and fiscal operations (treasury-info); automated license renewal, visa quota tracking, and zone-specific compliance per DFZC regulations"],
        ["Remittance Processing", "$6.7B annual volume routed through SBP Raast instant payment system to UAE Central Bank settlement; real-time FX rate locking, AML screening via regpack sanctions lists, automatic WHT deduction per ITO 2001 Section 231A"],
        ["SIFC FDI Pipeline", "Investment facilitation for Emirati capital into Pakistani infrastructure and technology sectors; compliance verification against BOI approval requirements, SECP foreign investment rules, and SBP foreign exchange regulations; automated investor onboarding with KYC/KYB via Mass identity services"],
        ["Trade Documentation", "Electronic bills of lading, certificates of origin, and customs declarations; integration with Dubai Trade portal and Pakistan Single Window; phytosanitary and halal certificates for food exports; HS code harmonization between UAE and Pakistan tariff schedules"],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- 25.3 PAK<->CHN Corridor ---
    h2("25.3 PAK\u2194CHN Corridor ($23.1B Bilateral)"),
    p("China-Pakistan trade totals $23.1B annually, primarily through CPEC 2.0. Nine SEZs, Gwadar customs operations, and e-trade documentation planned for corridor integration. Status: planned."),

    h3("25.3.1 Strategic Importance"),
    p("The PAK\u2194CHN corridor is the largest by trade volume at $23.1B annually and the most architecturally complex due to the depth of China-Pakistan Economic Corridor (CPEC) 2.0 integration. CPEC has invested over $62B in Pakistani infrastructure since 2015, creating nine operational Special Economic Zones, the Gwadar deep-water port, and extensive road and rail networks connecting Chinese manufacturing to Arabian Sea shipping lanes. Phase 2 of CPEC shifts focus from infrastructure construction to industrial cooperation, agricultural modernization, and digital trade, all of which require the kind of automated compliance, multi-jurisdictional entity management, and trade documentation that the SEZ Stack provides. The corridor must handle the unique complexity of Chinese customs regulations, PBOC currency controls, and the intersection of Pakistani and Chinese SEZ incentive regimes. Direct PKR/CNY settlement via the SBP-PBOC bilateral swap agreement eliminates USD intermediation for corridor transactions, reducing settlement costs and FX exposure. This corridor, once fully operational, will process more transaction volume than the KSA and UAE corridors combined."),

    h3("25.3.2 Component Architecture"),
    table(
      ["Component", "Implementation"],
      [
        ["CPEC 2.0 Integration", "Nine operational SEZs with Gwadar customs automation; HS code mapping between Chinese GB/T and Pakistani PTC tariff classifications; duty-free import verification for CPEC-designated machinery and raw materials; integration with China Customs SWAPP platform"],
        ["Trade Documentation", "E-trade platform for electronic bills of lading, packing lists, and commercial invoices; Chinese customs integration via CIQ inspection certificates and CCC product certification; automated certificate of origin generation for CPFTA-II preferential rates"],
        ["Currency Settlement", "PKR/CNY direct settlement via SBP-PBOC bilateral currency swap agreement; elimination of USD intermediation for corridor transactions; real-time exchange rate feeds from both central banks; automatic conversion and reconciliation through Mass treasury-info APIs"],
        ["SEZ Coordination", "Coordinated compliance across Allama Iqbal (Faisalabad), Rashakai (Nowshera), Dhabeji (Thatta), and M-3 Industrial City (Faisalabad); zone-specific incentive tracking including tax holidays, duty exemptions, and profit repatriation rules; unified entity registration across Pakistani BOI and Chinese MOFCOM requirements"],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- Summary ---
    h2("25.4 Corridor Summary"),
    p("The three live and planned corridors represent Pakistan's three largest bilateral trade relationships, collectively accounting for $38.6B in annual trade volume. Each corridor operates at a different maturity level, allowing Momentum to validate the SEZ Stack architecture progressively: the UAE corridor proves full orchestration at scale, the KSA corridor validates customs automation and diaspora services under a new bilateral framework, and the CHN corridor will stress-test the system against the most complex multi-SEZ, multi-currency configuration in the portfolio."),
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
