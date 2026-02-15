const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixG() {
  return [
    chapterHeading("Appendix G: Jurisdiction Template Reference"),
    table(
      ["Jurisdiction", "Lawpack", "Regpack", "Licensepack", "Profile"],
      [
        ["Pakistan", "ITO 2001, STA 1990, FEA, Customs Act, Companies Act", "FBR calendars, SROs, FATF AML", "SECP, BOI, PTA, PEMRA, DRAP, Provincial", "sovereign-govos"],
        ["ADGM", "ADGM Companies Regulations, Financial Services Regulations", "FSRA rulebook, FATF", "Financial services, corporate", "digital-financial-center"],
        ["Seychelles", "International Business Companies Act, Financial Services Act", "SFSA guidelines", "IBC, CSL, banking", "sovereign-govos"],
        ["Kazakhstan (Alatau)", "Kazakh civil code + AIFC overlay", "AFSA rules, NB KZ", "AIFC categories + Kazakh", "digital-financial-center"],
      ],
      [1600, 2400, 2000, 1800, 1560]
    ),
    spacer(),
  ];
};
