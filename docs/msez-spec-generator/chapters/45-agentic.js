const {
  chapterHeading, h2,
  p,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter45() {
  return [
    chapterHeading("Chapter 45: Agentic Execution Framework"),

    // --- 45.1 Trigger Taxonomy ---
    h2("45.1 Trigger Taxonomy"),
    p("The agentic execution framework defines twenty trigger types organized across five domains. Each trigger carries the context needed for policy evaluation and can activate one or more policy responses."),
    table(
      ["Domain", "Trigger", "Fires When", "Context"],
      [
        ["Compliance", "ComplianceStateChange", "Tensor cell transitions to a new state", "entity_id, domain, old_state, new_state"],
        ["Compliance", "SanctionsListUpdated", "OFAC/EU/UN sanctions list is refreshed", "list_id, entries_added, entries_removed"],
        ["Compliance", "LicenseExpiryApproaching", "License within renewal window", "license_id, days_remaining, authority"],
        ["Compliance", "AttestationExpired", "Watcher attestation validity period elapsed", "attestation_id, domain, expired_at"],
        ["Corridor", "CorridorReceiptAppended", "New receipt added to corridor chain", "corridor_id, receipt_seq, receipt_type"],
        ["Corridor", "CorridorStateTransition", "Corridor FSM transitions state", "corridor_id, old_state, new_state"],
        ["Corridor", "NettingCycleComplete", "Bilateral/multilateral netting computed", "corridor_id, net_positions, settlement_due"],
        ["Corridor", "BridgeTransferInitiated", "Cross-corridor transfer begins", "source_corridor, dest_corridor, asset_id"],
        ["Fiscal", "PaymentExecuted", "Payment processed through treasury API", "payment_id, amount, currency, parties"],
        ["Fiscal", "TaxEventGenerated", "Tax computation produces a liability", "tax_event_id, tax_type, amount, due_date"],
        ["Fiscal", "WithholdingApplied", "WHT deducted at source", "transaction_id, wht_amount, rate, section"],
        ["Fiscal", "FeeCollected", "SEZ fee deducted from transaction", "fee_type, amount, entity_id"],
        ["Governance", "GovernanceQuorumReached", "Proposal achieves required quorum", "proposal_id, vote_count, quorum_type"],
        ["Governance", "ConsentRequested", "Multi-party approval workflow initiated", "consent_id, parties, deadline"],
        ["Governance", "AmendmentProposed", "Constitutional amendment submitted", "amendment_id, provision_level, proposer"],
        ["Governance", "RulingIssued", "Arbitration ruling published", "dispute_id, ruling_type, enforcement_due"],
        ["Temporal", "ScheduledDeadline", "Calendar deadline arrives", "deadline_id, deadline_type, entity_id"],
        ["Temporal", "FilingDueDate", "Regulatory filing due date reached", "filing_type, authority, entity_id"],
        ["Temporal", "KeyRotationDue", "Cryptographic key rotation period elapsed", "key_id, key_type, last_rotated"],
        ["Temporal", "CheckpointRequired", "Periodic checkpoint interval reached", "asset_id, checkpoint_seq, last_checkpoint"],
      ],
      [1400, 2400, 2800, 2760]
    ),
    spacer(),

    ...codeBlock(
`/// Trigger types for the agentic execution framework (20 types \u00d7 5 domains).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    // --- Compliance Domain (4 triggers) ---
    ComplianceStateChange { entity_id: EntityId, domain: ComplianceDomain, new_state: ComplianceState },
    SanctionsListUpdated { list_id: String, entries_added: usize, entries_removed: usize },
    LicenseExpiryApproaching { license_id: LicenseId, days_remaining: u32 },
    AttestationExpired { attestation_id: AttestationId, domain: ComplianceDomain },

    // --- Corridor Domain (4 triggers) ---
    CorridorReceiptAppended { corridor_id: CorridorId, receipt_seq: u64 },
    CorridorStateTransition { corridor_id: CorridorId, old_state: CorridorState, new_state: CorridorState },
    NettingCycleComplete { corridor_id: CorridorId, net_positions: Vec<NetPosition> },
    BridgeTransferInitiated { source_corridor: CorridorId, dest_corridor: CorridorId, asset_id: AssetId },

    // --- Fiscal Domain (4 triggers) ---
    PaymentExecuted { payment_id: PaymentId, amount: Amount, currency: CurrencyCode },
    TaxEventGenerated { tax_event_id: TaxEventId, tax_type: TaxEventType, amount: Amount },
    WithholdingApplied { transaction_id: TransactionId, wht_amount: Amount, rate: Decimal },
    FeeCollected { fee_type: FeeType, amount: Amount, entity_id: EntityId },

    // --- Governance Domain (4 triggers) ---
    GovernanceQuorumReached { proposal_id: ProposalId, vote_count: u64 },
    ConsentRequested { consent_id: ConsentId, parties: Vec<EntityId>, deadline: DateTime<Utc> },
    AmendmentProposed { amendment_id: AmendmentId, provision_level: u8 },
    RulingIssued { dispute_id: DisputeId, ruling_type: RulingType },

    // --- Temporal Domain (4 triggers) ---
    ScheduledDeadline { deadline_id: String, deadline_type: DeadlineType },
    FilingDueDate { filing_type: FilingType, authority: RegulatoryAuthority, entity_id: EntityId },
    KeyRotationDue { key_id: KeyId, key_type: KeyType },
    CheckpointRequired { asset_id: AssetId, checkpoint_seq: u64 },
}`
    ),
    spacer(),

    // --- 45.2 Standard Policy Library ---
    h2("45.2 Standard Policy Library"),
    p("The standard policy library provides pre-built responses to common triggers. Policies are composable: a single trigger can activate multiple policies, and policies can chain to produce cascading actions. Standard policies include: automatic compliance re-evaluation on sanctions list update, license renewal notification 90/60/30 days before expiry, corridor suspension on compliance state degradation to NonCompliant, automatic tax withholding computation on payment execution, governance escalation when a proposal deadline passes without quorum, and regulatory filing generation on calendar-driven deadlines."),

    // --- 45.3 Policy Evaluation ---
    h2("45.3 Policy Evaluation"),
    p("When a trigger fires, the policy engine evaluates all registered policies in priority order. Each policy specifies a trigger filter (which trigger types it responds to), a condition predicate (additional guards beyond trigger matching), an action sequence (ordered list of operations to execute), and a failure mode (retry, skip, or escalate). Policy evaluation is transactional: either all actions in a policy complete successfully, or the policy is rolled back and the failure mode determines next steps."),
  ];
};
