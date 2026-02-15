const {
  partHeading, chapterHeading, h2, h3,
  p, codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter22() {
  return [
    ...partHeading("PART IX: CRYPTOGRAPHIC CORRIDOR SYSTEMS"),
    chapterHeading("Chapter 22: Corridor Architecture"),

    // --- 22.1 Corridor Establishment ---
    h2("22.1 Corridor Establishment"),
    p("Corridor establishment follows Protocol 14.1, creating bilateral channels between consenting jurisdictions. The process comprises four phases: policy alignment ensures compatible compliance frameworks between parties, technical integration connects infrastructure through authenticated channels, governance agreement specifies corridor administration, amendment procedures, dispute resolution, and termination conditions, and activation produces a corridor definition Verifiable Credential binding all participants."),
    p("Policy alignment requires each jurisdiction to publish its policy requirements as a machine-readable specification derived from its Pack Trilogy. The corridor negotiation engine identifies compatible policy overlaps across all twenty compliance domains. Where policies conflict, the engine proposes resolution strategies: union (apply the stricter of two requirements), intersection (apply only shared requirements), or escalation (flag for human resolution). The resulting policy set becomes the corridor compliance baseline."),
    ...codeBlock(
      "/// Request to establish a new corridor between jurisdictions.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct CorridorEstablishmentRequest {\n" +
      "    pub initiator: JurisdictionId,\n" +
      "    pub counterparty: JurisdictionId,\n" +
      "    pub proposed_operations: Vec<OperationType>,\n" +
      "    pub policy_alignment: PolicyAlignmentResult,\n" +
      "    pub governance: CorridorGovernance,\n" +
      "    pub technical_config: TechnicalIntegrationConfig,\n" +
      "    pub effective_date: DateTime<Utc>,\n" +
      "    pub expiry_date: Option<DateTime<Utc>>,\n" +
      "}\n" +
      "\n" +
      "/// Governance parameters for corridor administration.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct CorridorGovernance {\n" +
      "    pub administrator: GovernanceBody,\n" +
      "    pub amendment_procedure: AmendmentProcedure,\n" +
      "    pub dispute_resolution: DisputeResolutionConfig,\n" +
      "    pub termination_conditions: Vec<TerminationCondition>,\n" +
      "    pub review_schedule: ReviewSchedule,\n" +
      "}"
    ),
    spacer(),

    // --- 22.2 Corridor Definition ---
    h2("22.2 Corridor Definition"),
    ...codeBlock(
      "/// A fully established corridor between jurisdictions.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Corridor {\n" +
      "    pub id: CorridorId,\n" +
      "    pub participants: Vec<JurisdictionId>,\n" +
      "    pub definition_vc: VerifiableCredential,\n" +
      "    pub agreement_vc: VerifiableCredential,\n" +
      "    pub permitted_operations: Vec<OperationType>,\n" +
      "    pub compliance_requirements: ComplianceBaseline,\n" +
      "    pub state_channel: StateChannel,\n" +
      "}"
    ),
    spacer(),

    // --- 22.3 State Synchronization ---
    h2("22.3 State Synchronization"),
    p("Vector clocks track causality across jurisdictions. Each state update increments the local clock component. Merkle proofs enable efficient delta synchronization. Conflict resolution follows deterministic rules specified in the corridor manifest."),

    h3("22.3.1 Vector Clock Structure"),
    p("Each corridor maintains a VectorClock that records the logical timestamp for every participating jurisdiction. When a jurisdiction performs a state update, it increments its own component before broadcasting. On receipt, the receiving jurisdiction merges by taking the component-wise maximum and then incrementing its own entry. This guarantees causal ordering: if event A causally precedes event B, then A's vector clock is strictly less than B's. Concurrent events (neither dominates) are detected and routed to the conflict resolution strategy specified in the corridor governance."),
    ...codeBlock(
      "/// Tracks causal ordering of state updates across corridor jurisdictions.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct VectorClock {\n" +
      "    /// Maps each jurisdiction to its logical timestamp.\n" +
      "    pub clocks: BTreeMap<JurisdictionId, u64>,\n" +
      "    /// The jurisdiction that last incremented this clock.\n" +
      "    pub last_updater: JurisdictionId,\n" +
      "    /// Wall-clock timestamp of the last update (for observability only).\n" +
      "    pub last_updated_at: DateTime<Utc>,\n" +
      "}\n" +
      "\n" +
      "impl VectorClock {\n" +
      "    /// Increment the local component for the given jurisdiction.\n" +
      "    pub fn tick(&mut self, jurisdiction: &JurisdictionId) { ... }\n" +
      "\n" +
      "    /// Merge with a remote clock, taking component-wise maximum.\n" +
      "    pub fn merge(&mut self, remote: &VectorClock) { ... }\n" +
      "\n" +
      "    /// Returns true if self causally precedes other.\n" +
      "    pub fn happened_before(&self, other: &VectorClock) -> bool { ... }\n" +
      "\n" +
      "    /// Returns true if neither clock dominates the other.\n" +
      "    pub fn is_concurrent(&self, other: &VectorClock) -> bool { ... }\n" +
      "}"
    ),
    spacer(),

    // --- 22.4 Lifecycle State Machine ---
    h2("22.4 Lifecycle State Machine"),
    p("Corridors operate within a lifecycle state machine: DRAFT \u2192 PENDING \u2192 ACTIVE, with branches to HALTED and SUSPENDED, ultimately leading to TERMINATED. Evidence-gated transitions require specific credentials. Each transition requires verifiable evidence submitted as a signed credential, ensuring that no corridor state change occurs without an auditable justification anchored to a responsible party."),

    h3("22.4.1 State Transition Table"),
    p("The following table enumerates every valid transition in the corridor lifecycle FSM. Transitions not listed are invalid and must be rejected by the state machine implementation. The Required Evidence column specifies the Verifiable Credential type or governance artifact that must accompany the transition request."),
    table(
      ["From State", "To State", "Trigger", "Required Evidence"],
      [
        ["DRAFT", "PENDING", "Both jurisdictions submit signed policy alignment and governance terms", "PolicyAlignmentVC signed by both parties, CorridorGovernanceVC"],
        ["PENDING", "ACTIVE", "Technical integration verified and activation approved by governance body", "TechnicalIntegrationVC, ActivationApprovalVC from corridor administrator"],
        ["ACTIVE", "HALTED", "Compliance violation detected or sanctions list match triggers automatic halt", "ComplianceViolationVC issued by Compliance Watcher, or SanctionsAlertVC"],
        ["ACTIVE", "SUSPENDED", "Governance body issues voluntary suspension for review or maintenance", "SuspensionOrderVC signed by corridor administrator with stated reason"],
        ["HALTED", "ACTIVE", "Remediation completed and verified by independent Compliance Watcher", "RemediationVC with evidence of corrective action, ClearanceVC from Compliance Watcher"],
        ["SUSPENDED", "ACTIVE", "Governance body lifts suspension after review completion", "ReactivationOrderVC signed by corridor administrator"],
        ["SUSPENDED", "TERMINATED", "Suspension period expires or governance body votes to terminate", "TerminationResolutionVC signed by governance quorum"],
        ["HALTED", "TERMINATED", "Remediation deadline exceeded or governance body determines corridor unrecoverable", "TerminationResolutionVC signed by governance quorum, DeadlineExpiryEvidence"],
      ],
      [1560, 1560, 3120, 3120]
    ),
    spacer(),

    // --- 22.5 Corridor Definition VC Fields ---
    h2("22.5 Corridor Definition VC Fields"),
    p("The corridor definition Verifiable Credential is the canonical binding artifact that records the full specification of an established corridor. It is issued upon successful PENDING \u2192 ACTIVE transition and serves as the authoritative reference for all corridor operations. The definition VC is immutable once issued; amendments produce a new definition VC that references the previous one via the supersedes field."),
    ...codeBlock(
      "/// Fields carried in the credentialSubject of a Corridor Definition VC.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct CorridorDefinitionCredentialSubject {\n" +
      "    /// Unique identifier for this corridor.\n" +
      "    pub corridor_id: CorridorId,\n" +
      "    /// Ordered list of participating jurisdictions.\n" +
      "    pub participants: Vec<JurisdictionId>,\n" +
      "    /// Operations permitted on this corridor.\n" +
      "    pub permitted_operations: Vec<OperationType>,\n" +
      "    /// Compliance baseline derived from policy alignment.\n" +
      "    pub compliance_baseline: ComplianceBaseline,\n" +
      "    /// Digest of the governance agreement document.\n" +
      "    pub governance_digest: CanonicalDigest,\n" +
      "    /// Corridor administrator identity.\n" +
      "    pub administrator: GovernanceBody,\n" +
      "    /// Date from which the corridor is operational.\n" +
      "    pub effective_date: DateTime<Utc>,\n" +
      "    /// Optional expiry; None means perpetual until terminated.\n" +
      "    pub expiry_date: Option<DateTime<Utc>>,\n" +
      "    /// Reference to a prior definition VC this one supersedes, if any.\n" +
      "    pub supersedes: Option<CredentialId>,\n" +
      "    /// Vector clock snapshot at time of issuance.\n" +
      "    pub initial_vector_clock: VectorClock,\n" +
      "}"
    ),
    spacer(),
  ];
};
