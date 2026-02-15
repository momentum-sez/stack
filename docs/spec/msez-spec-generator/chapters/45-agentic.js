const {
  chapterHeading, h2,
  p,
  codeBlock, spacer
} = require("../lib/primitives");

module.exports = function build_chapter45() {
  return [
    chapterHeading("Chapter 45: Agentic Execution Framework"),

    // --- 45.1 Trigger System ---
    h2("45.1 Trigger System"),
    p("The agentic execution framework responds to events across the SEZ Stack by evaluating trigger conditions and executing policy-defined actions. Triggers are categorized by domain: compliance triggers fire on tensor state changes, corridor triggers fire on trade flow events, fiscal triggers fire on payment and tax events, governance triggers fire on consent and voting events, and temporal triggers fire on calendar-driven deadlines. Each trigger type is parameterized by its domain and carries the context needed for policy evaluation."),
    ...codeBlock(
`/// Trigger types for the agentic execution framework.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    /// Compliance domain state changed for an entity.
    ComplianceStateChange { entity_id: EntityId, domain: ComplianceDomain, new_state: ComplianceState },
    /// A corridor receipt was appended.
    CorridorReceiptAppended { corridor_id: CorridorId, receipt_seq: u64 },
    /// A payment was executed through the fiscal primitive.
    PaymentExecuted { payment_id: PaymentId, amount: Amount, currency: CurrencyCode },
    /// A license is approaching expiry.
    LicenseExpiryApproaching { license_id: LicenseId, days_remaining: u32 },
    /// A sanctions list was updated.
    SanctionsListUpdated { list_id: String, entries_added: usize, entries_removed: usize },
    /// A governance vote reached quorum.
    GovernanceQuorumReached { proposal_id: ProposalId, vote_count: u64 },
    /// A calendar-driven deadline has arrived.
    ScheduledDeadline { deadline_id: String, deadline_type: DeadlineType },
}`
    ),
    spacer(),

    // --- 45.2 Standard Policy Library ---
    h2("45.2 Standard Policy Library"),
    p("The standard policy library provides pre-built responses to common triggers. Policies are composable: a single trigger can activate multiple policies, and policies can chain to produce cascading actions. Standard policies include: automatic compliance re-evaluation on sanctions list update, license renewal notification 90/60/30 days before expiry, corridor suspension on compliance state degradation to NonCompliant, automatic tax withholding computation on payment execution, governance escalation when a proposal deadline passes without quorum, and regulatory filing generation on calendar-driven deadlines."),
  ];
};
