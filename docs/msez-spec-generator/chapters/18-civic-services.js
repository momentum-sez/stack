const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter18() {
  return [
    chapterHeading("Chapter 18: Civic Services Integration"),

    // --- 18.1 Identity Services ---
    h2("18.1 Identity Services"),
    p("Zone identity services provide residents and businesses with verifiable credentials: resident credentials (zone residency status, rights, obligations), business credentials (entity registration, good standing, authorized activities), professional credentials (qualifications, licensing for regulated professions). All credentials support selective disclosure via BBS+."),

    // --- 18.2 Property Services ---
    h2("18.2 Property Services"),
    p("Property rights are represented as Smart Assets with zone-specific lawpack bindings. Title registry maintains the authoritative record of property ownership using append-only receipt chains. Transfer services facilitate property transactions with compliance verification. Encumbrance management tracks liens, mortgages, and other property interests."),

    // --- 18.3 Dispute Resolution Services ---
    h2("18.3 Dispute Resolution Services"),
    p("Small claims procedures handle low-value disputes through expedited processes. Commercial arbitration handles business disputes through international arbitration institutions (DIFC-LCIA, SIAC, AIFC-IAC, ICC). Appellate procedures enable review of initial determinations."),
  ];
};
