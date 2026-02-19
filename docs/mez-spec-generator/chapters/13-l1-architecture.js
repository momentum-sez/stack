const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter13() {
  return [
    ...partHeading("PART VI: MASS L1 SETTLEMENT INFRASTRUCTURE"),
    chapterHeading("Chapter 13: ZK-Native Blockchain Architecture"),

    // --- 13.1 Design Targets ---
    h2("13.1 Design Targets"),
    table(
      ["Target", "Specification", "Rationale"],
      [
        ["Throughput", "100K-10M+ TPS", "Support major financial center volumes"],
        ["Private TX Latency", "<200ms", "Real-time payment applications"],
        ["Consensus Latency", "<2s", "Cross-shard coordination"],
        ["Privacy", "Untraceable by default", "Commercial confidentiality"],
        ["Compliance", "ZK-proven predicates", "Regulatory satisfaction"],
        ["Post-Quantum", "STARK-native crypto", "Future-proof security"],
        ["Client Proving", "<10s mobile, <60s legacy", "Practical user experience"],
      ],
      [2400, 2800, 4160]
    ),

    // --- 13.2 State Model ---
    h2("13.2 State Model"),
    p("The MASS L1 employs an object-centric state model where each entity, asset, or corridor is represented as an independent state object. Private records are encrypted under the owner's keys and stored as opaque commitments on-chain, while public mappings (such as nullifier sets, registry roots, and anchor hashes) are maintained in a globally-readable Merkle structure. This separation enables full transaction privacy without sacrificing the ability to verify global invariants like double-spend prevention and compliance predicate satisfaction."),

    // --- 13.2.1 Consensus Mechanism ---
    h3("13.2.1 Consensus Mechanism"),
    p("Consensus is structured as a two-layer DAG-based protocol optimized for high-throughput settlement across jurisdictional boundaries. The protocol separates local ordering (within a single Harbor shard) from global finality (across the root chain), enabling sub-second local confirmation with cross-shard settlement in under two seconds."),
    p_runs([bold("Jurisdictional DAG Consensus (JDC)."), " Each Harbor shard maintains a local DAG of transaction batches. Validators within a jurisdiction produce blocks that reference multiple parent blocks, forming a DAG rather than a linear chain. This structure enables concurrent block production and eliminates the throughput bottleneck of sequential block proposals. Local consensus achieves finality in under 200ms for transactions confined to a single jurisdiction."]),
    p_runs([bold("Treaty Lattice Consensus (TLC)."), " Cross-jurisdictional transactions are finalized through a lattice-based protocol that aggregates commitments from multiple Harbor DAGs. The root chain validators collect certified DAG snapshots from each Harbor and produce a global ordering that respects causal dependencies across jurisdictions. TLC achieves cross-shard finality in under 2 seconds while maintaining the sovereignty of each jurisdictional shard."]),
    table(
      ["Transaction Type", "Throughput (TPS)", "Latency", "Consensus"],
      [
        ["Intra-Harbor (local)", "100K-1M per shard", "<200ms", "JDC (local DAG)"],
        ["Cross-Harbor (bilateral)", "10K-100K", "<1s", "TLC (bilateral)"],
        ["Cross-Harbor (multilateral)", "1K-10K", "<2s", "TLC (full lattice)"],
        ["Settlement Anchor", "100-1K", "<5s", "Root chain finality"],
      ],
      [2400, 2400, 1800, 2760]
    ),

    // --- 13.3 Sharding Architecture ---
    h2("13.3 Sharding Architecture"),
    p_runs([bold("Tier 1: Execution Shards."), " The execution layer is partitioned into Harbor Shards (one per jurisdiction or economic zone) and Corridor Shards (one per active trade corridor). Harbor shards process all local transactions for their jurisdiction, including entity formation, asset transfers, and compliance checks. Corridor shards handle cross-jurisdictional operations such as receipt chain synchronization, netting, and settlement."]),
    p_runs([bold("Tier 2: Root Chain."), " The root chain aggregates state commitments from all execution shards into a single global state root. It does not execute transactions directly; instead, it verifies STARK proofs submitted by shard validators and maintains the canonical ordering of cross-shard events. The root chain is the anchor point for external L1 bridges and provides the final settlement guarantee."]),
    ...codeBlock(
      "/// A Harbor shard represents a single jurisdictional execution environment.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Harbor {\n" +
      "    pub id: HarborId,\n" +
      "    pub jurisdiction: JurisdictionCode,\n" +
      "    pub validators: Vec<ValidatorId>,\n" +
      "    pub local_chain: DagState,\n" +
      "    pub dag_references: Vec<DagReference>,\n" +
      "    pub treaty_set: Vec<TreatyId>,\n" +
      "}"
    ),

    // --- 13.3.1 Jurisdictional Virtual Machine (JVM) ---
    h3("13.3.1 Jurisdictional Virtual Machine (JVM)"),
    p("Each Harbor shard executes a Jurisdictional Virtual Machine (JVM) -- a jurisdiction-specific execution environment that encodes the legal, fiscal, and regulatory rules of its zone directly into the transaction validation logic. The JVM is not a general-purpose smart contract runtime; it is a constrained, deterministic state machine whose instruction set is derived from the jurisdiction's lawpack, regpack, and licensepack. This design ensures that every transaction processed within a Harbor is valid not merely in the cryptographic sense but in the legal sense: it satisfies the jurisdiction's formation requirements, tax withholding rules, sanctions constraints, and licensing obligations as a precondition of execution."),
    p_runs([bold("Instruction Set Derivation."), " The JVM instruction set is generated at Harbor initialization from the jurisdiction's pack trilogy. Each lawpack provision maps to a validation predicate (e.g., the Pakistan Income Tax Ordinance 2001 Section 153 maps to a withholding tax computation predicate). Each regpack entry maps to a runtime constraint (e.g., SBP foreign exchange limits map to transfer amount bounds). Each licensepack registry maps to a status check (e.g., SECP active registration required for corporate asset transfers). The resulting instruction set is a fixed, auditable mapping from legal requirements to executable predicates."]),
    p_runs([bold("Execution Model."), " Transaction execution within the JVM proceeds in three phases. First, the predicate evaluation phase checks all jurisdictional constraints against the transaction inputs, producing a compliance attestation or a rejection with specific predicate failure codes. Second, the state transition phase applies the transaction to the Harbor's local state, updating commitments, nullifiers, and registry entries. Third, the proof generation phase produces a STARK proof attesting to both the correctness of the state transition and the satisfaction of all jurisdictional predicates. This three-phase model ensures that compliance is not an afterthought but is structurally inseparable from execution."]),
    ...codeBlock(
      "/// Jurisdictional Virtual Machine configuration for a Harbor shard.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct JurisdictionalVM {\n" +
      "    pub harbor_id: HarborId,\n" +
      "    pub jurisdiction: JurisdictionCode,\n" +
      "    pub lawpack_version: PackVersion,\n" +
      "    pub regpack_version: PackVersion,\n" +
      "    pub licensepack_version: PackVersion,\n" +
      "    pub predicate_set: Vec<CompliancePredicate>,\n" +
      "    pub runtime_constraints: Vec<RuntimeConstraint>,\n" +
      "    pub license_checks: Vec<LicenseCheck>,\n" +
      "}"
    ),

    // --- 13.3.2 Asset Orbit Protocol ---
    h3("13.3.2 Asset Orbit Protocol"),
    p("Assets on the MASS L1 do not reside permanently in a single shard. Instead, they orbit between jurisdictional Harbors as they participate in cross-border trade corridors, regulatory migrations, and settlement cycles. The Asset Orbit Protocol governs how an asset's state is transferred from one Harbor to another while maintaining cryptographic continuity, compliance validity, and double-spend prevention across jurisdictional boundaries."),
    p_runs([bold("Orbit Initiation."), " When an asset must transition from Harbor A (origin jurisdiction) to Harbor B (destination jurisdiction), the origin Harbor produces an orbit departure proof: a STARK proof attesting to the asset's current state, its compliance status in the origin jurisdiction, and the nullification of its origin-side commitment. This proof is submitted to the root chain along with an encrypted state capsule containing the asset's full state, encrypted under Harbor B's receiving key."]),
    p_runs([bold("Orbit Transit."), " During transit, the asset exists in a liminal state: nullified in Harbor A but not yet committed in Harbor B. The root chain maintains a transit registry that tracks all in-flight orbits, preventing double-departure attacks and enforcing timeout-based recovery if the destination Harbor fails to accept. Transit duration is bounded by the Treaty agreement between the two jurisdictions, typically under 5 seconds for bilateral corridors."]),
    p_runs([bold("Orbit Arrival."), " Harbor B validates the departure proof, decrypts the state capsule, evaluates the asset against its own jurisdictional predicates (via its JVM), and if compliant, produces an orbit arrival proof that commits the asset into Harbor B's local state. The root chain records the completed orbit, linking the origin nullifier to the destination commitment. If Harbor B's compliance evaluation fails -- for example, if the asset type is not permitted under the destination jurisdiction's licensing regime -- the orbit is rejected and the origin Harbor's recovery path restores the asset to its pre-departure state."]),
    table(
      ["Orbit Phase", "Location", "Duration", "Guarantee"],
      [
        ["Departure", "Origin Harbor", "<200ms", "Nullified + proof generated"],
        ["Transit", "Root Chain registry", "<2s", "Double-departure prevented"],
        ["Arrival", "Destination Harbor", "<200ms", "Re-committed + compliance checked"],
        ["Recovery (timeout)", "Origin Harbor", "<10s", "Restored to pre-departure state"],
      ],
      [2000, 2400, 1800, 3160]
    ),
  ];
};
