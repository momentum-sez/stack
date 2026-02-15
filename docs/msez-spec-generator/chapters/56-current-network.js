const {
  chapterHeading,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter56() {
  return [
    chapterHeading("Chapter 56: Current Network"),

    table(
      ["Jurisdiction", "Status", "Profile", "Corridors"],
      [
        ["UAE / ADGM", "Live", "digital-financial-center", "PAK-UAE, KSA-UAE"],
        ["Dubai FZC (27 zones)", "Integration", "digital-financial-center", "PAK-UAE"],
        ["Pakistan", "Active", "sovereign-govos", "PAK-KSA, PAK-UAE, PAK-CHN"],
        ["Kazakhstan (Alatau City)", "Partnership", "digital-financial-center", "Planned"],
        ["Seychelles", "Deployment", "sovereign-govos", "Planned"],
      ],
      [2400, 1400, 3000, 2560]
    ),
    spacer(),

    p("Aggregate metrics: 1,000+ entities onboarded, $1.7B+ capital processed, 5 jurisdictions active or deploying, 3 bilateral corridors ($38.6B combined volume), 16 module families (298 modules), 650 tests at 100% coverage."),
  ];
};
