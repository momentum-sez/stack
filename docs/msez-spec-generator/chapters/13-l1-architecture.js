const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table,
  spacer
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
    spacer(),

    // --- 13.2 State Model ---
    h2("13.2 State Model"),
    p("The MASS L1 employs an object-centric state model where each entity, asset, or corridor is represented as an independent state object. Private records are encrypted under the owner's keys and stored as opaque commitments on-chain, while public mappings (such as nullifier sets, registry roots, and anchor hashes) are maintained in a globally-readable Merkle structure. This separation enables full transaction privacy without sacrificing the ability to verify global invariants like double-spend prevention and compliance predicate satisfaction."),

    // --- 13.3 Consensus Mechanism ---
    h2("13.3 Consensus Mechanism"),
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
    spacer(),

    // --- 13.4 Sharding Architecture ---
    h2("13.4 Sharding Architecture"),
    p_runs([bold("Tier 1: Execution Shards."), " The execution layer is partitioned into Harbor Shards (one per jurisdiction or special economic zone) and Corridor Shards (one per active trade corridor). Harbor shards process all local transactions for their jurisdiction, including entity formation, asset transfers, and compliance checks. Corridor shards handle cross-jurisdictional operations such as receipt chain synchronization, netting, and settlement."]),
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
    spacer(),

    // --- 13.5 Jurisdiction Virtual Machine ---
    h2("13.5 Jurisdiction Virtual Machine (JVM)"),
    p("Each Harbor shard runs a Jurisdiction Virtual Machine that executes compliance predicates, asset state transitions, and governance operations within a single jurisdictional context. The JVM extends the SAVM with jurisdiction-specific built-in functions: lawpack evaluation (direct access to the jurisdiction's Akoma Ntoso corpus), regpack queries (real-time sanctions, rate tables, and calendar data), and licensepack verification (license status checks against live registries). These built-in functions execute as coprocessor calls, bypassing the general-purpose instruction set for performance-critical operations."),
    p("JVM isolation ensures that a jurisdiction's internal operations cannot observe or interfere with another jurisdiction's state. Cross-jurisdictional interactions are mediated exclusively through the corridor protocol, with TLC consensus providing the ordering guarantee."),

    // --- 13.6 Asset Orbit ---
    h2("13.6 Asset Orbit"),
    p("An Asset Orbit is the set of Harbor shards that a Smart Asset has transited through during its lifecycle. The orbit determines which jurisdictions hold state commitments for the asset and which watcher sets are authorized to attest to its compliance. As an asset migrates from one jurisdiction to another, its orbit expands to include the destination Harbor. The orbit provides the basis for cross-jurisdictional audit: any jurisdiction in an asset's orbit can verify the asset's full receipt chain against its local state commitments."),
    ...codeBlock(
      "/// Tracks the jurisdictional history of a Smart Asset.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct AssetOrbit {\n" +
      "    pub asset_id: AssetId,\n" +
      "    pub harbors_visited: Vec<(HarborId, DateTime<Utc>)>,\n" +
      "    pub current_harbor: HarborId,\n" +
      "    pub total_migrations: u32,\n" +
      "}"
    ),
    spacer(),
  ];
};
