const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter45() {
  return [
    chapterHeading("Chapter 45: Agentic Execution Framework"),

    // --- 45.1 Trigger System ---
    h2("45.1 Trigger System"),
    p("The agentic execution framework responds to events across the SEZ Stack by evaluating trigger conditions and executing policy-defined actions. Triggers are categorized by domain: compliance triggers fire on tensor state changes, corridor triggers fire on trade flow events, fiscal triggers fire on payment and tax events, governance triggers fire on consent and voting events, and temporal triggers fire on calendar-driven deadlines. Each trigger type is parameterized by its domain and carries the context needed for policy evaluation."),

    h3("45.1.1 Trigger Taxonomy"),
    p("The framework defines a complete taxonomy of 20 triggers organized across 5 domains. Every trigger carries typed context sufficient for downstream policy evaluation without additional lookups. The taxonomy is closed: adding a new trigger variant requires a schema migration and policy audit."),

    p_runs([bold("Compliance Domain."), " Four triggers covering sanctions screening, license lifecycle, regulatory guidance updates, and compliance tensor state transitions."]),
    table(
      ["Domain", "Trigger", "Description"],
      [
        ["Compliance", "SanctionsListUpdate", "A monitored sanctions list (OFAC SDN, UN Consolidated, EU Sanctions, FATF) has been updated. Carries list identifier, delta of added/removed entries, and effective date."],
        ["Compliance", "LicenseExpiration", "A license held by an entity has expired or will expire within the configured warning window (default: 90/60/30 days). Carries license identifier, expiry date, and issuing authority."],
        ["Compliance", "GuidanceChange", "A regulatory authority has issued new guidance, an SRO, or an amendment affecting a compliance domain. Carries the Akoma Ntoso identifier of the changed instrument and affected domains."],
        ["Compliance", "ComplianceDomainStateChange", "The compliance tensor evaluation for an entity has changed state in one or more of the 20 compliance domains. Carries entity identifier, domain, previous state, and new state."],
      ],
      [1400, 2400, 5560]
    ),
    spacer(),

    p_runs([bold("Fiscal Domain."), " Four triggers covering payment thresholds, withholding tax events, fee schedules, and tax filing deadlines."]),
    table(
      ["Domain", "Trigger", "Description"],
      [
        ["Fiscal", "PaymentThresholdExceeded", "A payment or cumulative payment volume for an entity has exceeded a configured threshold (e.g., PKR 50M annual, USD 10K single transaction). Carries payment identifier, amount, currency, and threshold rule."],
        ["Fiscal", "WithholdingTaxEvent", "A transaction requires withholding tax computation under applicable tax law (e.g., Pakistan Income Tax Ordinance 2001 Sections 149-153). Carries transaction identifier, tax rate, and applicable statute."],
        ["Fiscal", "FeeDueDate", "A recurring fee (zone license fee, regulatory filing fee, annual return fee) is due. Carries fee type, amount, due date, and entity identifier."],
        ["Fiscal", "TaxFilingDeadline", "A statutory tax filing deadline is approaching or has arrived (e.g., annual return, quarterly withholding statement, sales tax return). Carries filing type, jurisdiction, deadline date, and penalty schedule."],
      ],
      [1400, 2400, 5560]
    ),
    spacer(),

    p_runs([bold("Corridor Domain."), " Four triggers covering receipt chain events, netting boundaries, corridor policy updates, and counterparty state changes."]),
    table(
      ["Domain", "Trigger", "Description"],
      [
        ["Corridor", "ReceiptChainAppended", "A new receipt has been appended to a corridor receipt chain. Carries corridor identifier, receipt sequence number, receipt digest, and the MMR root after inclusion."],
        ["Corridor", "NettingBoundary", "A netting window has closed for a corridor, requiring settlement computation. Carries corridor identifier, netting period, gross flows in both directions, and net settlement amount."],
        ["Corridor", "CorridorPolicyUpdate", "The compliance or operational policy for a corridor has been updated (e.g., new sanctions requirement, changed settlement frequency). Carries corridor identifier and policy diff."],
        ["Corridor", "CounterpartyStateChange", "A counterparty on a corridor has changed compliance state, requiring re-evaluation of corridor eligibility. Carries counterparty entity identifier, corridor identifier, and new compliance state."],
      ],
      [1400, 2400, 5560]
    ),
    spacer(),

    p_runs([bold("Governance Domain."), " Four triggers covering vote quorum, proposal deadlines, amendment ratification, and stakeholder threshold events."]),
    table(
      ["Domain", "Trigger", "Description"],
      [
        ["Governance", "VoteQuorumReached", "A governance proposal has reached its required quorum of votes. Carries proposal identifier, vote count, quorum threshold, and current tally (for/against/abstain)."],
        ["Governance", "ProposalDeadline", "A governance proposal has reached its voting deadline. Carries proposal identifier, deadline timestamp, whether quorum was met, and final tally."],
        ["Governance", "AmendmentRatified", "A constitutional or regulatory amendment has been ratified through the governance process. Carries amendment identifier, ratification date, and the set of affected compliance domains."],
        ["Governance", "StakeholderThresholdCrossed", "A stakeholder's ownership or voting power has crossed a disclosure threshold (e.g., 5%, 10%, 25%, 50%). Carries entity identifier, threshold crossed, direction (above/below), and new percentage."],
      ],
      [1400, 2400, 5560]
    ),
    spacer(),

    p_runs([bold("Temporal Domain."), " Four triggers covering license renewal, annual returns, certificate expiry, and scheduled audits."]),
    table(
      ["Domain", "Trigger", "Description"],
      [
        ["Temporal", "LicenseRenewalDue", "A license renewal window has opened. Unlike LicenseExpiration (which fires on expiry), this fires at the configured renewal-open date to initiate the renewal workflow. Carries license identifier, renewal window open/close dates, and renewal fee."],
        ["Temporal", "AnnualReturnDeadline", "The annual return filing deadline for an entity is approaching. Carries entity identifier, jurisdiction, filing type, deadline date, and late-filing penalty schedule."],
        ["Temporal", "CertificateExpiring", "A cryptographic certificate (TLS, signing key, VC issuer credential) is approaching expiry. Carries certificate identifier, expiry date, and key rotation instructions."],
        ["Temporal", "ScheduledAuditDue", "A scheduled audit (internal compliance review, external regulatory audit, financial audit) is due. Carries audit type, entity identifier, auditor assignment, and due date."],
      ],
      [1400, 2400, 5560]
    ),
    spacer(),

    h3("45.1.2 Trigger Enum Definition"),
    p("The complete trigger enum encodes the full 20-variant taxonomy. Each variant carries strongly-typed context fields, ensuring that policy evaluation functions receive all required data without additional lookups."),
    ...codeBlock(
`/// Trigger types for the agentic execution framework.
/// 20 variants across 5 domains: Compliance, Fiscal, Corridor, Governance, Temporal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trigger {
    // ── Compliance Domain (4) ──────────────────────────────────────
    /// A monitored sanctions list has been updated.
    SanctionsListUpdate { list_id: String, entries_added: usize, entries_removed: usize, effective: DateTime<Utc> },
    /// A license held by an entity has expired or is approaching expiry.
    LicenseExpiration { license_id: LicenseId, entity_id: EntityId, expiry_date: DateTime<Utc>, days_remaining: u32 },
    /// Regulatory guidance or SRO has changed.
    GuidanceChange { instrument_id: String, affected_domains: Vec<ComplianceDomain>, effective: DateTime<Utc> },
    /// Compliance tensor state changed for an entity in a domain.
    ComplianceDomainStateChange { entity_id: EntityId, domain: ComplianceDomain, prev_state: ComplianceState, new_state: ComplianceState },

    // ── Fiscal Domain (4) ──────────────────────────────────────────
    /// Payment or cumulative volume exceeded a configured threshold.
    PaymentThresholdExceeded { entity_id: EntityId, payment_id: PaymentId, amount: Amount, currency: CurrencyCode, threshold_rule: String },
    /// A transaction requires withholding tax computation.
    WithholdingTaxEvent { transaction_id: TransactionId, tax_rate_bps: u32, applicable_statute: String },
    /// A recurring fee is due.
    FeeDueDate { entity_id: EntityId, fee_type: String, amount: Amount, due_date: DateTime<Utc> },
    /// A statutory tax filing deadline is approaching or has arrived.
    TaxFilingDeadline { entity_id: EntityId, jurisdiction: JurisdictionCode, filing_type: String, deadline: DateTime<Utc> },

    // ── Corridor Domain (4) ────────────────────────────────────────
    /// A new receipt was appended to a corridor receipt chain.
    ReceiptChainAppended { corridor_id: CorridorId, receipt_seq: u64, receipt_digest: CanonicalDigest, mmr_root: CanonicalDigest },
    /// A netting window has closed, requiring settlement.
    NettingBoundary { corridor_id: CorridorId, period_start: DateTime<Utc>, period_end: DateTime<Utc>, net_amount: Amount },
    /// Corridor compliance or operational policy was updated.
    CorridorPolicyUpdate { corridor_id: CorridorId, policy_diff: String },
    /// A counterparty's compliance state changed.
    CounterpartyStateChange { corridor_id: CorridorId, counterparty_id: EntityId, new_state: ComplianceState },

    // ── Governance Domain (4) ──────────────────────────────────────
    /// A governance proposal reached its required quorum.
    VoteQuorumReached { proposal_id: ProposalId, vote_count: u64, quorum_threshold: u64 },
    /// A governance proposal reached its voting deadline.
    ProposalDeadline { proposal_id: ProposalId, deadline: DateTime<Utc>, quorum_met: bool },
    /// A constitutional or regulatory amendment was ratified.
    AmendmentRatified { amendment_id: String, ratified_at: DateTime<Utc>, affected_domains: Vec<ComplianceDomain> },
    /// A stakeholder crossed an ownership/voting disclosure threshold.
    StakeholderThresholdCrossed { entity_id: EntityId, threshold_pct: u8, direction: ThresholdDirection, new_pct: f64 },

    // ── Temporal Domain (4) ────────────────────────────────────────
    /// A license renewal window has opened.
    LicenseRenewalDue { license_id: LicenseId, renewal_open: DateTime<Utc>, renewal_close: DateTime<Utc>, fee: Amount },
    /// Annual return filing deadline approaching.
    AnnualReturnDeadline { entity_id: EntityId, jurisdiction: JurisdictionCode, deadline: DateTime<Utc> },
    /// A cryptographic certificate is approaching expiry.
    CertificateExpiring { cert_id: String, expiry_date: DateTime<Utc>, key_type: String },
    /// A scheduled audit is due.
    ScheduledAuditDue { audit_type: String, entity_id: EntityId, due_date: DateTime<Utc> },
}`
    ),
    spacer(),

    // --- 45.2 Standard Policy Library ---
    h2("45.2 Standard Policy Library"),
    p("The standard policy library provides pre-built responses to common triggers. Policies are composable: a single trigger can activate multiple policies, and policies can chain to produce cascading actions. Standard policies include: automatic compliance re-evaluation on sanctions list update, license renewal notification 90/60/30 days before expiry, corridor suspension on compliance state degradation to NonCompliant, automatic tax withholding computation on payment execution, governance escalation when a proposal deadline passes without quorum, and regulatory filing generation on calendar-driven deadlines."),
    spacer(),

    // --- 45.3 Policy Composition ---
    h2("45.3 Policy Composition"),
    p("Policies are the unit of autonomous action in the agentic framework. A policy binds a trigger predicate to an action sequence, with optional guard conditions and chaining rules. Policies compose through three mechanisms: fan-out (one trigger activates multiple independent policies), chaining (one policy's output triggers downstream policies), and aggregation (multiple triggers must all fire before a policy activates)."),

    h3("45.3.1 Fan-Out Composition"),
    p("When a single trigger fires, the policy engine evaluates all registered policies whose trigger predicate matches. Each matching policy executes independently with its own error boundary. For example, a SanctionsListUpdate trigger simultaneously activates: (1) a compliance re-screening policy that re-evaluates all entities against the updated list, (2) a corridor review policy that checks all active corridors involving affected jurisdictions, and (3) a notification policy that alerts compliance officers."),

    h3("45.3.2 Chain Composition"),
    p("A policy can declare downstream triggers that fire upon its successful completion. Chains are acyclic by construction: the policy engine maintains a directed acyclic graph of chain dependencies and rejects registrations that would create cycles. Chain depth is bounded by a configurable maximum (default: 5) to prevent runaway cascades. Each link in the chain carries the output context of the preceding policy, enabling progressive enrichment of the action context."),
    ...codeBlock(
`/// A policy definition binding triggers to actions with optional chaining.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub trigger_predicate: TriggerPredicate,
    pub guard_conditions: Vec<GuardCondition>,
    pub actions: Vec<PolicyAction>,
    pub chain_triggers: Vec<Trigger>,
    pub priority: u32,
    pub enabled: bool,
}

/// Guard conditions evaluated before policy actions execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GuardCondition {
    /// Entity must be in a specific compliance state.
    ComplianceStateIs { domain: ComplianceDomain, required_state: ComplianceState },
    /// Corridor must be in a specific lifecycle phase.
    CorridorPhaseIs { required_phase: CorridorPhase },
    /// A minimum time must have elapsed since the last execution.
    CooldownElapsed { min_interval: Duration },
    /// A jurisdictional scope constraint.
    JurisdictionIn { allowed: Vec<JurisdictionCode> },
}`
    ),
    spacer(),

    h3("45.3.3 Aggregation Composition"),
    p("Aggregation policies require multiple triggers to fire within a configurable time window before activating. The policy engine maintains a sliding window of trigger events and evaluates the aggregation predicate on each new event. This supports patterns like: activate a high-risk review only when both a SanctionsListUpdate AND a PaymentThresholdExceeded fire for the same entity within 24 hours. Aggregation windows are bounded and automatically expire incomplete trigger sets."),
    spacer(),

    // --- 45.4 Agentic Execution Guarantees ---
    h2("45.4 Agentic Execution Guarantees"),
    p("The agentic execution engine provides formal guarantees about trigger processing and policy execution. These guarantees are essential for sovereign deployment where autonomous actions affect real entities, real capital, and real regulatory standing."),
    table(
      ["Guarantee", "Mechanism", "Failure Mode"],
      [
        ["Idempotency", "Every policy execution is assigned a unique execution ID derived from (trigger_id, policy_id, timestamp). The engine maintains a deduplication log and silently discards duplicate executions. Actions are designed to be safe to retry: state transitions check preconditions, and external calls use idempotency keys.", "If the deduplication log is unavailable, the engine halts rather than risk duplicate execution."],
        ["Ordered Delivery", "Triggers within the same domain are delivered to policies in causal order. Cross-domain triggers are delivered in wall-clock order with a configurable skew tolerance (default: 500ms). The engine uses a per-domain sequence counter backed by Postgres to enforce ordering.", "If a gap is detected in the sequence counter, the engine pauses delivery for that domain and raises a SequenceGap alert."],
        ["At-Least-Once Processing", "Every trigger is persisted to the trigger log (Postgres) before acknowledgment. The engine uses a pull-based consumption model with explicit checkpointing. Unacknowledged triggers are redelivered after a configurable timeout (default: 30 seconds).", "Delivery may be delayed but never lost, provided the Postgres WAL is intact."],
        ["Bounded Retry", "Failed policy executions are retried with exponential backoff (base 1s, max 60s, jitter +/-25%). Retry count is bounded (default: 5 attempts). After exhausting retries, the trigger is moved to a dead-letter queue and a FailedPolicyExecution alert is raised.", "Permanently failing policies are quarantined; they do not block other policies."],
        ["Audit Trail", "Every trigger event, policy evaluation, guard condition result, action execution, and chain propagation is recorded in an append-only audit log with cryptographic integrity (each entry includes the SHA-256 digest of the previous entry). The audit log is queryable by entity, corridor, domain, and time range.", "Audit log writes are synchronous; if the log is unavailable, execution pauses."],
        ["Graceful Degradation", "If the policy engine cannot reach an external dependency (Mass API, Postgres, signing service), it enters a degraded mode: triggers continue to be persisted to the trigger log, but policy evaluation is suspended. When the dependency recovers, the engine replays all queued triggers in order.", "Manual intervention is required if degraded mode persists beyond the configurable threshold (default: 1 hour)."],
      ],
      [1800, 4200, 3360]
    ),
    spacer(),
  ];
};
