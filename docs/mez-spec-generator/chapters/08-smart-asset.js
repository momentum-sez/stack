const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table, pageBreak
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

    // --- 8.3 Asset Lifecycle ---
    h2("8.3 Asset Lifecycle"),
    p("Every Smart Asset progresses through five lifecycle phases: Genesis (creation and identity establishment), Active (normal operation with state transitions), Suspended (temporarily halted pending compliance review or dispute resolution), Migrating (transferring between jurisdictions or registries), and Archived (permanently frozen with full history preserved). Transitions between phases are governed by the state machine and require appropriate authorization and compliance verification."),
    table(
      ["Phase", "Description", "Allowed Transitions"],
      [
        [
          "Creation",
          "Genesis record minted, identity established, manifest bound. The asset receives its content-addressed identifier and initial registry binding.",
          "Creation → Active"
        ],
        [
          "Active",
          "Normal operation. The asset accepts state transitions, accumulates receipts, and participates in corridor flows. Compliance tensor is continuously evaluated.",
          "Active → Suspended, Active → Terminal"
        ],
        [
          "Suspended",
          "Temporarily halted pending compliance review, dispute resolution, or regulatory hold. No state transitions are permitted except resumption or escalation to terminal.",
          "Suspended → Active, Suspended → Terminal"
        ],
        [
          "Terminal",
          "Irreversible end state. All obligations settled, final receipt appended, receipt chain sealed. The asset cannot accept further transitions but remains queryable.",
          "Terminal → Archived"
        ],
        [
          "Archived",
          "Permanently frozen with full history preserved in content-addressed storage. The receipt chain, manifest, and all attestations are retained for audit and regulatory retrieval.",
          "None (final state)"
        ],
      ],
      [1400, 4760, 3200]
    ),

    // --- 8.4 Smart Assets as Autonomous Agents ---
    h2("8.4 Smart Assets as Autonomous Agents"),
    p("Smart Assets can exhibit agentic behavior by responding autonomously to environmental events. When configured with trigger policies, a Smart Asset monitors its environment and initiates state transitions without human intervention. This enables automated compliance responses, scheduled regulatory filings, and automated recovery behavior in response to external changes."),
    definition("Definition 8.2 (Agentic Transition).", "An agentic transition is a state transition triggered by environmental events. Trigger types: regulatory triggers (SanctionsListUpdate, LicenseExpiration, GuidanceChange), arbitration triggers (RulingReceived, AppealDeadlinePassed, EnforcementDue), settlement triggers (CheckpointRequired, FinalizationAnchor), and asset lifecycle triggers (KeyRotationDue, AttestationExpiring)."),
    p("The following table enumerates representative Smart Asset triggers across five domains. The complete 20-trigger taxonomy is specified in \u00a745 (Agentic Execution Framework):"),
    table(
      ["Domain", "Trigger Type", "Description", "Example"],
      [
        [
          "Regulatory",
          "SanctionsListUpdate",
          "A sanctions list referenced by the asset's regpack has been amended",
          "OFAC SDN list revision affecting a counterparty"
        ],
        [
          "Regulatory",
          "LicenseExpiration",
          "A license required for the asset's operation is approaching or past expiry",
          "SECP registration renewal deadline in 30 days"
        ],
        [
          "Regulatory",
          "GuidanceChange",
          "Regulatory guidance or SRO affecting the asset's compliance posture has changed",
          "FBR issues new SRO on withholding tax rates"
        ],
        [
          "Regulatory",
          "TaxRateChange",
          "Applicable tax rate modified by fiscal authority",
          "SBP adjusts policy rate affecting withholding calculations"
        ],
        [
          "Settlement",
          "CheckpointRequired",
          "Periodic settlement checkpoint due per corridor schedule",
          "Weekly PAK↔UAE corridor netting checkpoint"
        ],
        [
          "Settlement",
          "FinalizationAnchor",
          "Settlement finality window reached, requiring anchor commit",
          "72-hour finality window closes on batch transfer"
        ],
        [
          "Settlement",
          "NettingCycleComplete",
          "A netting cycle has completed and net positions must be settled",
          "End-of-day bilateral netting across corridor participants"
        ],
        [
          "Asset",
          "KeyRotationDue",
          "Signing key material is approaching rotation schedule",
          "Ed25519 key pair 90-day rotation policy triggers"
        ],
        [
          "Asset",
          "AttestationExpiring",
          "A verifiable credential attesting asset compliance is near expiry",
          "Formation VC expires in 14 days, re-attestation needed"
        ],
        [
          "Asset",
          "OwnershipTransfer",
          "Beneficial ownership change detected via Mass OWNERSHIP primitive",
          "Cap table update triggers compliance re-evaluation"
        ],
        [
          "Corridor",
          "CounterpartyDefault",
          "A counterparty in a corridor flow has missed a settlement obligation",
          "Importer fails to fund PKR escrow within SLA"
        ],
        [
          "Corridor",
          "ForkDetected",
          "Receipt chain divergence detected between corridor participants",
          "Two conflicting receipts reference the same parent hash"
        ],
        [
          "Corridor",
          "CorridorSuspension",
          "A trade corridor is suspended by regulatory or operational action",
          "PAK↔IRN corridor suspended due to sanctions update"
        ],
        [
          "Temporal",
          "ScheduledFiling",
          "Calendar-driven regulatory filing deadline approaching",
          "Quarterly withholding tax return due to FBR"
        ],
        [
          "Temporal",
          "PeriodicAttestation",
          "Recurring compliance attestation cycle triggered by calendar",
          "Annual AML/KYC re-verification for all corridor entities"
        ],
        [
          "Temporal",
          "SLABreach",
          "Service-level agreement time window has been exceeded",
          "Document generation exceeds 48-hour SLA commitment"
        ],
      ],
      [1200, 2200, 3360, 2600]
    ),

    // --- 8.5 Content-Addressed Storage ---
    h2("8.5 Content-Addressed Storage"),
    p("Smart Assets are persisted using a content-addressed storage (CAS) model. Every artifact — genesis records, manifests, receipts, verifiable credentials, and compliance snapshots — is stored by its SHA-256 digest. This provides immutability, deduplication, and trivial integrity verification. The CAS layer is the persistence substrate for the entire Smart Asset execution layer."),
    h3("8.5.1 CAS Directory Layout"),
    p("The following directory structure defines how Smart Asset artifacts are organized on disk. The two-character prefix partitioning prevents excessive directory entries and enables efficient filesystem-level lookups:"),
    ...codeBlock(
      "dist/artifacts/\n" +
      "├── genesis/\n" +
      "│   ├── ab/\n" +
      "│   │   └── ab3f…c7.json          # Genesis record by SHA-256 digest\n" +
      "│   └── f1/\n" +
      "│       └── f1a2…e9.json\n" +
      "├── manifests/\n" +
      "│   ├── 0c/\n" +
      "│   │   └── 0c87…d4.json          # Asset manifest by digest\n" +
      "│   └── e3/\n" +
      "│       └── e3b0…c4.json\n" +
      "├── receipts/\n" +
      "│   ├── 2f/\n" +
      "│   │   └── 2fa3…b1.json          # Individual state transition receipt\n" +
      "│   └── d8/\n" +
      "│       └── d809…a2.json\n" +
      "├── credentials/\n" +
      "│   ├── 7a/\n" +
      "│   │   └── 7a91…f3.json          # Verifiable Credential (W3C VC)\n" +
      "│   └── bb/\n" +
      "│       └── bb12…c8.json\n" +
      "├── chains/\n" +
      "│   └── <asset-id>/\n" +
      "│       ├── chain.json             # Ordered receipt digest list\n" +
      "│       └── head                   # Current chain head digest\n" +
      "└── snapshots/\n" +
      "    └── <asset-id>/\n" +
      "        ├── latest.json            # Most recent compliance tensor snapshot\n" +
      "        └── <timestamp>.json       # Historical snapshots by ISO-8601 timestamp"
    ),
    h3("8.5.2 Storage Invariants"),
    p_runs([bold("Content integrity."), " Every artifact's filename is its SHA-256 digest. On retrieval, the digest is recomputed and compared. Any mismatch indicates corruption or tampering and causes an immediate rejection. All digest computation flows through CanonicalBytes::new() as specified in the crate dependency invariants."]),
    p_runs([bold("Append-only semantics."), " Once written, a CAS artifact is never modified or deleted during normal operation. The receipt chain for each asset is strictly append-only: new receipts reference the previous chain head, forming a hash-linked sequence."]),
    p_runs([bold("Deduplication."), " Identical artifacts produce identical digests and are stored exactly once. This is particularly valuable for compliance tensor snapshots that may be identical across evaluation cycles when no regulatory changes have occurred."]),
  ];
};
