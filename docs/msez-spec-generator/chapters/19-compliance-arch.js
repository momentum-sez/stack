const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter19() {
  return [
    ...partHeading("PART VIII: COMPLIANCE AND REGULATORY INTEGRATION"),
    chapterHeading("Chapter 19: Compliance Architecture"),

    // --- 19.1 Compliance Model ---
    h2("19.1 Compliance Model"),
    p("The Stack compliance model separates rule specification from rule enforcement. Rules are encoded in the Pack Trilogy (lawpacks, regpacks, licensepacks); enforcement occurs through the Smart Asset execution layer and the compliance tensor. This separation enables regulatory agility: rule changes propagate through pack updates without code modifications or redeployment."),
    p("Rule specification uses predicate-based formalism. Each compliance rule is a predicate over entity state, transaction parameters, and jurisdictional context that evaluates to COMPLIANT, NON_COMPLIANT, PENDING, SUSPENDED, or EXEMPT. Predicates compose through logical operators: conjunction (all rules must pass), disjunction (any rule suffices), and conditional (if jurisdiction X, then evaluate rule set Y). Enforcement mechanisms vary by rule type:"),
    table(
      ["Enforcement Type", "Timing", "Examples"],
      [
        ["Pre-transaction", "Block before execution", "Sanctions screening, license verification, entity status check"],
        ["At-transaction", "Execute with compliance coprocessor", "WHT calculation, fee deduction, compliance tensor update"],
        ["Post-transaction", "Report after settlement", "CTR filing, STR generation, regulatory reporting to FBR/SBP"],
        ["Periodic", "Scheduled evaluation", "Annual compliance review, license renewal check, filing deadline enforcement"],
      ],
      [2400, 2800, 4160]
    ),
    spacer(),

    // --- 19.2 Identity Verification ---
    h2("19.2 Identity Verification"),
    p("Identity verification follows the zkKYC model, enabling compliance verification without continuous identity disclosure. Verification providers issue Verifiable Credentials attesting to verification completion at a specific KYC tier. The credential includes a BBS+ signature enabling selective disclosure of attributes. Re-verification triggers when: credentials approach expiry, the entity's risk profile changes (e.g., a sanctions list update affects the entity's jurisdiction), or the entity requests a higher KYC tier for increased transaction limits."),

    // --- 19.3 Transaction Monitoring ---
    h2("19.3 Transaction Monitoring"),
    p("Rule engines evaluate configurable pattern rules: velocity anomalies (transaction frequency exceeding historical norms), structuring patterns (multiple sub-threshold transactions that aggregate above reporting thresholds), and high-risk counterparty interactions (transactions with entities in jurisdictions flagged by FATF). Privacy preservation techniques enable monitoring without mass surveillance: the monitoring engine operates on encrypted transaction metadata, with decryption only upon formal triggering of an investigation."),
    p("Investigation procedures specify how flagged activity is examined through formal authorization. A Suspicious Transaction Report (STR) requires a Compliance Viewing Key (cvk) issuance authorized by the jurisdiction's financial intelligence unit. The cvk decrypts only the specific transaction set under investigation, and the decryption event is recorded in the compliance audit trail."),

    // --- 19.4 Compliance Escalation ---
    h2("19.4 Compliance Escalation"),
    p("Violation handling follows configurable escalation paths. Minor violations (late filing, minor data discrepancy) trigger automated remediation notifications through the agentic framework. Moderate violations (missed filing deadline, incomplete KYC renewal) trigger entity suspension with a remediation window. Severe violations (sanctions match, fraud detection, regulatory order) trigger immediate entity freeze and corridor suspension pending investigation. Each escalation level is recorded in the entity's receipt chain with a compliance tensor update reflecting the changed state."),
  ];
};
