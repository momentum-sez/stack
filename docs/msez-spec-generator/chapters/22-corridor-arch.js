const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table, spacer
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
    p("Vector clocks track causality across jurisdictions. Each state update increments the local clock component and appends the update to the corridor's receipt chain. Merkle proofs enable efficient delta synchronization: rather than transmitting the full corridor state, participants exchange only the receipts added since the last synchronization point, with Merkle inclusion proofs against the receipt chain root."),
    p("Conflict resolution follows deterministic rules specified in the corridor manifest. When concurrent updates from different jurisdictions create divergent receipt chains, the conflict resolver applies a three-step process: detect (identify the fork point via vector clock comparison), classify (determine whether the conflict is semantic or merely ordering-related), and resolve (apply the corridor-specific merge strategy: last-writer-wins for idempotent updates, or escalate to governance for mutually exclusive state changes)."),

    // --- 22.4 Lifecycle State Machine ---
    h2("22.4 Lifecycle State Machine"),
    p("Corridors operate within a lifecycle state machine with six states and evidence-gated transitions:"),
    table(
      ["State", "Description", "Allowed Transitions"],
      [
        ["DRAFT", "Corridor proposal created, terms under negotiation", "PENDING (on mutual agreement)"],
        ["PENDING", "Terms agreed, awaiting technical activation and watcher registration", "ACTIVE (on activation VC), TERMINATED (on rejection)"],
        ["ACTIVE", "Fully operational, processing transactions and synchronizing state", "HALTED (on compliance failure), SUSPENDED (on governance action), TERMINATED (on expiry/mutual termination)"],
        ["HALTED", "Temporarily frozen due to compliance tensor degradation", "ACTIVE (on compliance restoration), TERMINATED (on timeout)"],
        ["SUSPENDED", "Governance-initiated pause for review or amendment", "ACTIVE (on governance resolution), TERMINATED (on dissolution)"],
        ["TERMINATED", "Permanently closed, all pending transactions settled, final netting computed", "Terminal state (no transitions)"],
      ],
      [1800, 4200, 3360]
    ),
    spacer(),
    ...codeBlock(
      "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum CorridorState {\n" +
      "    Draft,\n" +
      "    Pending,\n" +
      "    Active,\n" +
      "    Halted,\n" +
      "    Suspended,\n" +
      "    Terminated,\n" +
      "}\n" +
      "\n" +
      "impl CorridorState {\n" +
      "    pub fn valid_transitions(&self) -> Vec<CorridorState> {\n" +
      "        match self {\n" +
      "            Self::Draft => vec![Self::Pending],\n" +
      "            Self::Pending => vec![Self::Active, Self::Terminated],\n" +
      "            Self::Active => vec![Self::Halted, Self::Suspended, Self::Terminated],\n" +
      "            Self::Halted => vec![Self::Active, Self::Terminated],\n" +
      "            Self::Suspended => vec![Self::Active, Self::Terminated],\n" +
      "            Self::Terminated => vec![],\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 22.5 Corridor Topologies ---
    h2("22.5 Corridor Topologies"),
    p_runs([bold("Hub-and-Spoke."), " A central jurisdiction (hub) maintains bilateral corridors with multiple peripheral jurisdictions (spokes). Cross-spoke transfers route through the hub, which handles compliance verification and netting. This topology suits deployments where one jurisdiction serves as a financial center (e.g., UAE/ADGM as hub for PAK, KSA, and other GCC spokes). The hub bears higher operational cost but simplifies spoke compliance requirements."]),
    p_runs([bold("Mesh."), " Every jurisdiction maintains direct bilateral corridors with every other jurisdiction. Cross-jurisdiction transfers are direct, eliminating hub latency and single-point-of-failure risk. This topology suits mature deployments with high bilateral trade volumes between all participants. The compliance cost is higher (each jurisdiction evaluates tensors for all counterparties) but settlement latency is minimized."]),
    p_runs([bold("Hybrid."), " Combines hub-and-spoke for low-volume corridors with direct bilateral corridors for high-volume pairs. The PAK\u2194UAE corridor operates as a direct bilateral due to $10.1B annual volume, while lower-volume corridors route through a hub. The PathRouter automatically selects the optimal path based on current topology, fees, and latency."]),
  ];
};
