const {
  partHeading, chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter19() {
  return [
    ...partHeading("PART VIII: COMPLIANCE AND REGULATORY INTEGRATION"),
    chapterHeading("Chapter 19: Compliance Architecture"),

    // --- 19.1 Compliance Model ---
    h2("19.1 Compliance Model"),
    p("The Stack compliance model separates rule specification from rule enforcement. Rules are encoded in the Pack Trilogy; enforcement occurs through the Smart Asset execution layer. Rule specification uses predicate-based formalism. Predicates compose through logical operators. Enforcement mechanisms vary by rule type: some require pre-transaction verification, others require post-transaction reporting. Violation handling follows configurable escalation paths."),

    // --- 19.2 Identity Verification ---
    h2("19.2 Identity Verification"),
    p("Identity verification follows the zkKYC model, enabling compliance verification without continuous identity disclosure. Verification providers issue Verifiable Credentials attesting to verification completion. Re-verification triggers when circumstances change or credentials expire."),

    // --- 19.3 Transaction Monitoring ---
    h2("19.3 Transaction Monitoring"),
    p("Rule engines evaluate configurable pattern rules: velocity anomalies, structuring patterns, high-risk counterparties. Privacy preservation techniques enable monitoring without mass surveillance. Investigation procedures specify how flagged activity is examined through formal authorization."),
  ];
};
