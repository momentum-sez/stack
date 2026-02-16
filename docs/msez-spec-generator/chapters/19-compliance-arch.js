const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter19() {
  return [
    ...partHeading("PART VIII: COMPLIANCE AND REGULATORY INTEGRATION"),
    chapterHeading("Chapter 19: Compliance Architecture"),

    // --- 19.1 Compliance Model ---
    h2("19.1 Compliance Model"),
    p("The Stack compliance model separates rule specification from rule enforcement. Rules are encoded in the Pack Trilogy; enforcement occurs through the Smart Asset execution layer. Rule specification uses predicate-based formalism. Predicates compose through logical operators. Enforcement mechanisms vary by rule type: some require pre-transaction verification, others require post-transaction reporting. Violation handling follows configurable escalation paths."),

    h3("19.1.1 Predicate-Based Compliance Formalism"),
    definition("Definition 19.1 (Compliance Predicate).", "A compliance predicate P is a boolean-valued function over the transaction context T, entity context E, and jurisdiction context J: P(T, E, J) -> {true, false}. A transaction is compliant if and only if the conjunction of all applicable predicates evaluates to true."),
    p("Predicates compose through standard logical operators. Conjunction (AND) requires all sub-predicates to hold. Disjunction (OR) requires at least one sub-predicate to hold. Negation (NOT) inverts a predicate. Implication (IF-THEN) encodes conditional rules where one condition triggers another requirement. Quantification (FOR-ALL, EXISTS) ranges over collections such as beneficial owners, transaction counterparties, or corridor participants."),
    p("Each predicate carries metadata: the legal authority (statute, regulation, or SRO) it implements, the compliance domain it belongs to (one of the 20 ComplianceDomain variants), the severity of violation (advisory, warning, blocking), and an expiration date tied to the legal instrument's sunset clause. This metadata enables traceability from any compliance decision back to its legal basis."),
    ...codeBlock(
`/// A single compliance rule with predicate, metadata, and enforcement config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRule {
    /// Unique rule identifier, scoped to the issuing jurisdiction.
    pub rule_id: RuleId,
    /// Human-readable rule name.
    pub name: String,
    /// The compliance domain this rule belongs to (from msez-core).
    pub domain: ComplianceDomain,
    /// The predicate expression to evaluate.
    pub predicate: PredicateExpr,
    /// Legal authority: statute, regulation, or SRO reference.
    pub legal_basis: LegalReference,
    /// When this rule takes effect.
    pub effective_from: chrono::DateTime<chrono::Utc>,
    /// When this rule expires (None = no sunset).
    pub effective_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Enforcement timing: pre-transaction, post-transaction, or continuous.
    pub enforcement: EnforcementTiming,
    /// Severity if the predicate evaluates to false.
    pub violation_severity: ViolationSeverity,
    /// Escalation path for violations.
    pub escalation: EscalationPath,
    /// Jurisdictions where this rule applies.
    pub jurisdictions: Vec<JurisdictionId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PredicateExpr {
    /// Atomic predicate: evaluate a single condition.
    Atom(AtomicPredicate),
    /// Conjunction: all sub-predicates must hold.
    And(Vec<PredicateExpr>),
    /// Disjunction: at least one sub-predicate must hold.
    Or(Vec<PredicateExpr>),
    /// Negation: the sub-predicate must not hold.
    Not(Box<PredicateExpr>),
    /// Implication: if condition holds, then consequent must hold.
    IfThen { condition: Box<PredicateExpr>, consequent: Box<PredicateExpr> },
    /// Universal quantification over a collection.
    ForAll { variable: String, collection: CollectionRef, body: Box<PredicateExpr> },
    /// Existential quantification over a collection.
    Exists { variable: String, collection: CollectionRef, body: Box<PredicateExpr> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViolationSeverity {
    /// Informational only; transaction proceeds, logged for audit.
    Advisory,
    /// Warning issued to parties; transaction proceeds with flag.
    Warning,
    /// Transaction blocked until condition resolved.
    Blocking,
    /// Critical regulatory violation; immediate escalation required.
    Critical,
}`
    ),

    h3("19.1.2 Enforcement Mechanisms"),
    p("Enforcement mechanisms are classified by timing relative to the transaction lifecycle. Pre-transaction enforcement prevents non-compliant transactions from executing. Post-transaction enforcement monitors completed transactions and triggers reporting or remediation. Continuous enforcement applies to ongoing obligations that persist beyond any single transaction."),
    table(
      ["Timing", "Mechanism", "Action on Violation", "Example Rules"],
      [
        ["Pre-transaction", "Gate check", "Transaction blocked; compliance gap report issued", "Sanctions screening, KYC/KYB verification, license validity, foreign ownership limits"],
        ["Pre-transaction", "Consent requirement", "Transaction held pending approval", "Board resolution for large transfers, regulatory pre-approval for controlled activities"],
        ["Pre-transaction", "Threshold check", "Transaction blocked if limit exceeded", "Single transaction limits, daily/monthly velocity caps, concentration limits"],
        ["Post-transaction", "Reporting obligation", "Report filed with regulatory authority", "CTR filing (threshold transactions), SAR filing (suspicious patterns), periodic tax reporting"],
        ["Post-transaction", "Audit trail", "Receipt chain entry with compliance attestation", "All transactions: immutable record for regulatory examination"],
        ["Post-transaction", "Reconciliation", "Discrepancy flagged for investigation", "Cross-border corridor settlement matching, tax withholding reconciliation"],
        ["Continuous", "License monitoring", "Entity suspended if license lapses", "Professional licenses, business activity permits, regulatory authorizations"],
        ["Continuous", "Sanctions monitoring", "Entity/asset frozen on sanctions match", "OFAC SDN list updates, UN sanctions updates, EU consolidated list updates"],
        ["Continuous", "Credential expiry", "Re-verification required", "KYC credential expiry, tax residency certificate renewal, good standing attestation refresh"],
      ],
      [1400, 1800, 2800, 3360]
    ),

    h3("19.1.3 Violation Handling and Escalation Paths"),
    p("When a compliance predicate evaluates to false, the system follows the rule's configured escalation path. Escalation paths define a sequence of actions with increasing severity, timeouts between levels, and responsible parties at each level."),
    table(
      ["Level", "Action", "Timeout", "Responsible Party", "Outcome if Unresolved"],
      [
        ["L0 — Advisory", "Log violation, notify entity", "N/A", "Entity compliance officer", "Auto-escalate to L1 if pattern repeats"],
        ["L1 — Warning", "Flag transaction, notify entity and zone compliance", "48 hours", "Zone compliance officer", "Escalate to L2"],
        ["L2 — Restriction", "Suspend entity's ability to initiate new transactions", "5 business days", "Zone compliance committee", "Escalate to L3"],
        ["L3 — Freeze", "Freeze all entity assets and corridor participation", "15 business days", "Zone authority + regulator", "Escalate to L4"],
        ["L4 — Regulatory Referral", "Refer to external regulatory authority; full evidence package", "Per regulator SLA", "External regulator", "Enforcement action per applicable law"],
      ],
      [1400, 2400, 1200, 2200, 2160]
    ),
    p("Escalation paths are configurable per jurisdiction, per compliance domain, and per violation severity. Critical-severity violations skip directly to L3 or L4 depending on the rule configuration. De-escalation is possible when the underlying condition is remediated: the entity provides missing documentation, obtains required licenses, or resolves the flagged condition. De-escalation requires affirmative action by the responsible party at the current level."),

    h3("19.1.4 Rule Engine Configuration"),
    p("The compliance rule engine loads rules from the Pack Trilogy at zone initialization and reloads when packs are updated. Rules are indexed by compliance domain and jurisdiction for efficient lookup. At evaluation time, the engine collects all applicable rules for the transaction's compliance domains and jurisdictions, evaluates each predicate against the transaction context, and returns a ComplianceVerdict that aggregates results across all rules."),
    p("Rule evaluation is deterministic: given the same transaction context, entity context, and pack state, the engine always produces the same verdict. This property is essential for audit reproducibility and dispute resolution. The engine supports dry-run evaluation, allowing entities to pre-check compliance before submitting transactions."),

    // --- 19.2 Identity Verification ---
    h2("19.2 Identity Verification"),
    p("Identity verification follows the zkKYC model, enabling compliance verification without continuous identity disclosure. Verification providers issue Verifiable Credentials attesting to verification completion. Re-verification triggers when circumstances change or credentials expire."),

    h3("19.2.1 Verification Tiers"),
    p("Identity verification is tiered to match the risk profile of the entity's activities. Higher tiers grant access to higher-value transactions, additional corridor participation, and regulated activities. Each tier builds on the previous, adding additional verification requirements."),
    table(
      ["Tier", "Verification Requirements", "Transaction Limits", "Corridor Access"],
      [
        ["Tier 0 — Basic", "Email, phone number, self-declared identity", "Minimal (view-only, no transactions)", "None"],
        ["Tier 1 — Standard", "Government ID (CNIC/passport), address verification", "Up to zone-configured daily limit", "Domestic corridors only"],
        ["Tier 2 — Enhanced", "Tier 1 + source of funds, beneficial ownership declaration", "Up to zone-configured enhanced limit", "Bilateral corridors"],
        ["Tier 3 — Institutional", "Tier 2 + audited financials, regulatory filings, board resolution", "Unlimited (subject to per-transaction compliance)", "All corridors including multilateral"],
      ],
      [1600, 3200, 2200, 2360]
    ),

    h3("19.2.2 Re-Verification Triggers"),
    p("Re-verification is triggered by any of the following events: credential expiration (per the validity period encoded in the VC), material change in beneficial ownership (detected through Mass ownership primitive updates), change in risk profile (transaction patterns exceed tier thresholds), regulatory directive (zone authority mandates re-verification for a class of entities), adverse media or sanctions list update (detected through continuous regpack monitoring). Re-verification suspends the entity's current tier privileges until completed."),

    // --- 19.3 Transaction Monitoring ---
    h2("19.3 Transaction Monitoring"),
    p("Rule engines evaluate configurable pattern rules: velocity anomalies, structuring patterns, high-risk counterparties. Privacy preservation techniques enable monitoring without mass surveillance. Investigation procedures specify how flagged activity is examined through formal authorization."),

    h3("19.3.1 Pattern Detection Rules"),
    p("The transaction monitoring engine evaluates a configurable set of pattern rules against the stream of completed transactions. Rules are expressed as the same predicate formalism used in the compliance model, but operate over windowed transaction histories rather than individual transactions."),
    table(
      ["Pattern Category", "Detection Logic", "Window", "Action on Detection"],
      [
        ["Velocity anomaly", "Transaction count or volume exceeds N standard deviations from entity baseline", "24h / 7d / 30d rolling", "SAR filing, L1 escalation"],
        ["Structuring", "Multiple transactions just below reporting threshold within window", "48h rolling", "SAR filing, L2 escalation"],
        ["Round-tripping", "Funds return to originator through intermediary chain", "30d rolling", "Investigation referral, L2 escalation"],
        ["High-risk counterparty", "Transaction with entity on elevated-risk list or adverse jurisdiction", "Per transaction", "Enhanced due diligence, L1 escalation"],
        ["Dormant account activation", "Transaction on account with no activity for extended period", "90d lookback", "Re-verification trigger, L1 escalation"],
        ["Rapid movement", "Funds received and immediately transferred out", "24h rolling", "Investigation referral, L1 escalation"],
      ],
      [1800, 3200, 1800, 2560]
    ),

    h3("19.3.2 Privacy-Preserving Monitoring"),
    p("Transaction monitoring operates on encrypted or committed transaction data wherever possible. The zkKYC model ensures that monitoring rules can verify compliance predicates (e.g., 'sender has valid KYC at Tier 2 or above') without accessing the underlying identity data. Pattern detection operates on aggregate statistics (transaction counts, volume sums) rather than individual transaction details. When a pattern triggers an alert, investigation access to the underlying data requires formal authorization from the zone compliance committee, creating an auditable access trail."),

    h3("19.3.3 Investigation Procedures"),
    p("When the monitoring engine flags activity, it initiates a structured investigation workflow. The alert is assigned to a compliance officer with appropriate clearance. The officer may request data access elevation, which requires committee approval and is logged as a consent record through the Mass consent primitive. Investigation findings are recorded as evidence packages compatible with the dispute resolution system. If the investigation confirms a violation, the enforcement escalation path is activated at the appropriate level."),
  ];
};
