const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer
} = require("../lib/primitives");

module.exports = function build_chapter08() {
  return [
    ...partHeading("PART V: SMART ASSET EXECUTION LAYER"),
    chapterHeading("Chapter 8: Smart Asset Model"),

    // --- 8.1 Smart Asset Definition ---
    h2("8.1 Smart Asset Definition"),
    definition("Definition 8.1 (Smart Asset).", "A Smart Asset is formally defined as a five-tuple (G, R, M, C, H) where G is the Genesis Record, R is the Registry Binding, M is the Manifest, C is the Receipt Chain, and H is the State Machine specification."),
    p_runs([bold("Genesis Record (G)."), " The immutable origin document establishing asset identity, initial state, and creation context. The genesis record is content-addressed and its digest serves as the canonical asset identifier."]),
    p_runs([bold("Registry Binding (R)."), " The linkage between the asset and one or more authoritative registries. Registry bindings establish jurisdictional context and enable cross-referencing with external systems."]),
    p_runs([bold("Manifest (M)."), " The declarative specification of asset capabilities, transition types, compliance requirements, and metadata. The manifest defines what the asset can do and under what constraints."]),
    p_runs([bold("Receipt Chain (C)."), " The append-only sequence of cryptographically linked state transition receipts. The receipt chain provides a complete, verifiable history of every state change."]),
    p_runs([bold("State Machine (H)."), " The formal specification of valid states, transitions, guards, and effects. The state machine enforces invariants and ensures only valid transitions are applied."]),
    ...codeBlock(
      "/// The five-tuple defining a Smart Asset.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct SmartAsset {\n" +
      "    pub genesis: GenesisRecord,\n" +
      "    pub registry_binding: RegistryBinding,\n" +
      "    pub manifest: AssetManifest,\n" +
      "    pub receipt_chain: ReceiptChain,\n" +
      "    pub state_machine: StateMachineSpec,\n" +
      "}"
    ),
    spacer(),

    // --- 8.2 Design Invariants ---
    h2("8.2 Design Invariants"),
    p("Five design invariants govern Smart Asset behavior. These hold regardless of implementation choices and form the foundation for security proofs:"),
    table(
      ["ID", "Name", "Statement"],
      [
        ["I1", "Immutable Identity", "Asset identity is established at genesis and cannot change"],
        ["I2", "Deterministic State", "Current state is uniquely determined by the receipt chain"],
        ["I3", "Explicit Bindings", "All cross-asset relationships are explicitly declared"],
        ["I4", "Resolvability", "Any asset reference can be resolved to current state"],
        ["I5", "Optional Anchoring", "Assets may operate without blockchain anchoring"],
      ],
      [800, 2200, 6360]
    ),
    spacer(),

    // --- 8.3 Asset Lifecycle ---
    h2("8.3 Asset Lifecycle"),
    p("Every Smart Asset progresses through five lifecycle phases: Genesis (creation and identity establishment), Active (normal operation with state transitions), Suspended (temporarily halted pending compliance review or dispute resolution), Migrating (transferring between jurisdictions or registries), and Archived (permanently frozen with full history preserved). Transitions between phases are governed by the state machine and require appropriate authorization and compliance verification."),

    // --- 8.4 Smart Assets as Autonomous Agents ---
    h2("8.4 Smart Assets as Autonomous Agents"),
    p("Smart Assets can exhibit agentic behavior by responding autonomously to environmental events. When configured with trigger policies, a Smart Asset monitors its environment and initiates state transitions without human intervention. This enables automated compliance responses, scheduled regulatory filings, and self-healing behavior in response to external changes."),
    definition("Definition 8.2 (Agentic Transition).", "An agentic transition is a state transition triggered by environmental events. Trigger types: regulatory triggers (SanctionsListUpdate, LicenseExpiration, GuidanceChange), arbitration triggers (RulingReceived, AppealDeadlinePassed, EnforcementDue), settlement triggers (CheckpointRequired, FinalizationAnchor), and asset lifecycle triggers (KeyRotationDue, AttestationExpiring)."),
  ];
};
