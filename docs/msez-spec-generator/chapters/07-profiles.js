const {
  chapterHeading,
  p, table
} = require("../lib/primitives");

module.exports = function build_chapter07() {
  return [
    chapterHeading("Chapter 7: Profile System"),
    p("Profiles are curated bundles of modules, parameters, and jurisdiction-specific configuration. They serve as deployment templates:"),
    table(
      ["Profile", "Use Case", "Key Modules"],
      [
        ["digital-financial-center", "Full-service financial zone (ADGM model)", "All 16 families, full corridor suite, capital markets"],
        ["trade-hub", "Trade and logistics zone", "Corporate, trade, financial, corridors, customs"],
        ["tech-park", "Technology and innovation zone", "Corporate, licensing, IP, identity, light financial"],
        ["sovereign-govos", "National government deployment (Pakistan model)", "All families + GovOS orchestration + national system integration"],
        ["charter-city", "Large-scale developments", "Full civic services, land management"],
        ["digital-native-free-zone", "Technology-focused zones", "Rapid formation, IP protection"],
        ["asset-history-bundle", "Asset provenance", "Enhanced receipt chains, certification"],
      ],
      [2600, 3200, 3560]
    ),
  ];
};
