const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold, table, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter05() {
  return [
    ...partHeading("PART IV: CORE COMPONENTS \u2014 MODULES, PACK TRILOGY, PROFILES"),
    chapterHeading("Chapter 5: Module Specifications"),
    p("Modules are the unit of composition in the MEZ Stack. Each module provides a discrete governance capability. Modules declare dependencies, expose interfaces, and can be composed into profiles."),

    // --- Module Interface Summary ---
    h3("Module Interface Summary"),
    p_runs([bold("Table 5.0."), " Each module family exposes a principal trait that defines its interface boundary. The trait method count and key methods listed below represent the public contract; all inter-module communication passes through these traits."]),
    table(
      ["Module Family", "Interface Trait", "Methods", "Key Methods"],
      [
        ["Corridors", "CorridorService", "8", "establish(), sync_state(), verify_compliance(), bridge()"],
        ["Governance", "GovernanceEngine", "6", "propose(), vote(), ratify(), amend()"],
        ["Financial", "TreasuryOps", "7", "open_account(), execute_payment(), fx_convert(), custody_hold()"],
        ["Regulatory", "ComplianceEvaluator", "5", "verify_identity(), screen_sanctions(), monitor_tx(), report()"],
        ["Licensing", "LicenseRegistry", "5", "apply(), issue(), renew(), port_credential()"],
        ["Legal", "DisputeResolver", "4", "file_dispute(), submit_evidence(), issue_ruling(), enforce()"],
        ["Operational", "ZoneAdmin", "3", "provision(), configure(), audit_log()"],
      ],
      [1600, 2000, 960, 4800]
    ),

    // --- 5.1 Corridors Module ---
    h2("5.1 Corridors Module"),
    p("The Corridors module manages economic relationships between jurisdictions. A corridor represents a bilateral or multilateral agreement enabling coordinated economic activity with cryptographic compliance guarantees. Corridor establishment follows Protocol 14.1 for cross-jurisdiction transfer setup. Each party publishes their policy requirements as a machine-readable specification. The corridor negotiation process identifies compatible policy overlaps and generates a corridor manifest encoding the agreed terms."),
    p("Corridor state synchronization maintains consistent views across participants. The sync protocol uses vector clocks for causality tracking and Merkle proofs for efficient delta synchronization. Compliance verification operates through the Compliance Tensor (§10). Cross-border operations verify all applicable predicates through ZK proofs."),
    table(
      ["Component", "Version", "Description"],
      [
        ["corridor-state-api", "3.2.1", "OpenAPI specification for corridor state management"],
        ["corridor-manifest-schema", "2.1.0", "JSON Schema for corridor manifests"],
        ["sync-protocol", "1.4.0", "State synchronization protocol specification"],
        ["compliance-tensor", "2.1.0", "Compliance Tensor V2 data structures"],
        ["corridor-bridge", "1.0.0", "Cross-corridor bridge protocol"],
      ],
      [2800, 1200, 5360]
    ),

    // --- 5.1.1 Governance Module ---
    h3("5.1.1 Governance Module"),
    p("The Governance module implements institutional decision-making processes including constitutional frameworks, voting mechanisms, amendment procedures, and stakeholder coordination. Constitutional frameworks define the fundamental rules governing zone operations. The Stack supports hierarchical constitutions with multiple amendment thresholds. Core provisions may require supermajority approval or external ratification, while operational policies may be modifiable through administrative action. Voting mechanisms support multiple models including token-weighted voting, one-entity-one-vote, quadratic voting, and conviction voting."),

    // --- 5.2 Financial Module ---
    h2("5.2 Financial Module"),
    p("The Financial module provides banking and payment infrastructure: account management, payment processing, foreign exchange, custody services, and capital markets integration. Account management supports both fiat and digital asset accounts. Fiat accounts integrate with traditional banking rails through the Mass Treasury API. Custody services provide institutional-grade asset protection with multi-signature wallets, configurable quorum policies, time-locked releases, and automated compliance holds."),

    // --- 5.3 Regulatory Module ---
    h2("5.3 Regulatory Module"),
    p("The Regulatory module implements compliance frameworks required for lawful economic activity: identity verification, transaction monitoring, sanctions screening, and regulatory reporting. Identity verification follows zkKYC principles. Transaction monitoring operates through configurable rule engines evaluated against jurisdiction-specific rules."),

    // --- 5.3.1 Licensing Module ---
    h3("5.3.1 Licensing Module"),
    p("The Licensing module manages business authorization: license application processing, compliance monitoring, renewal management, and portability across compatible jurisdictions. License portability enables mutual recognition across compatible jurisdictions through credential verification."),

    // --- 5.3.2 Legal Module ---
    h3("5.3.2 Legal Module"),
    p("The Legal module provides infrastructure for contract management, dispute resolution, and enforcement. Enforcement mechanisms translate legal determinations into system actions. Arbitration rulings encoded as Verifiable Credentials trigger automatic state transitions in affected Smart Assets."),

    // --- 5.3.3 Operational Module ---
    h3("5.3.3 Operational Module"),
    p("The Operational module provides administrative functionality for zone management: human resources, procurement, facility management, and general administration."),
  ];
};
