const { chapterHeading, h2, h3, p, p_runs, bold, table } = require("../lib/primitives");

module.exports = function build_appendixC() {
  return [
    chapterHeading("Appendix C: Scalability Switch Reference"),

    // --- Switch Interaction Narrative ---
    h2("C.1 Switch Interactions"),
    p(
      "The thirteen scalability switches are not independent parameters. They form three " +
      "interaction clusters that must be tuned together to maintain system invariants. " +
      "Operators who adjust one switch without considering its cluster will encounter " +
      "degraded performance or correctness violations."
    ),

    h3("C.1.1 Throughput Cluster (S1, S2, S3, S4)"),
    p(
      "Harbor shards (S1) and corridor shards (S2) control horizontal parallelism. " +
      "Block size (S3) and block interval (S4) control vertical throughput per shard. " +
      "Increasing S1/S2 distributes load across more shards but increases cross-shard " +
      "coordination overhead. Increasing S3 raises throughput per block but increases " +
      "latency for individual transactions. Decreasing S4 produces more frequent blocks " +
      "but increases the proof generation load. The effective throughput formula is: " +
      "TPS = (S1 * S3) / S4 for harbor operations, and cross-corridor TPS scales with " +
      "S2 subject to the bridge hop limit (S11). Operators should increase S1/S2 first " +
      "for linear scaling, and adjust S3/S4 only when shard count reaches infrastructure limits."
    ),

    h3("C.1.2 Verification Cluster (S5, S6, S7, S13)"),
    p(
      "Proof batch size (S5) and checkpoint interval (S6) control amortization of " +
      "cryptographic verification costs. Watcher quorum (S7) determines how many independent " +
      "attestations are required before a state transition is considered finalized. " +
      "DA enforcement (S13) controls whether data availability proofs are checked. " +
      "These four switches jointly determine the security-latency tradeoff: larger batches " +
      "(S5) and checkpoints (S6) reduce per-operation proof cost but increase finalization " +
      "delay. Higher quorums (S7) increase Byzantine fault tolerance but require more watcher " +
      "nodes to be online. Enforced DA (S13) provides the strongest availability guarantees " +
      "but adds a round-trip to the DA layer before finalization. For sovereign deployments " +
      "processing real capital, S7 should be at least 3-of-5 and S13 should be Enforced."
    ),

    h3("C.1.3 Economic Cluster (S8, S9, S10, S11, S12)"),
    p(
      "Staleness bound (S8) controls how long compliance attestations remain valid before " +
      "re-evaluation is required. Max asset value (S9) caps the risk exposure per asset. " +
      "Migration duration (S10) bounds how long a cross-jurisdiction migration may take before " +
      "automatic rollback. Bridge hop limit (S11) constrains multi-hop corridor paths. " +
      "Fee multiplier (S12) tunes the economic cost of operations. These switches interact " +
      "through the risk equation: higher asset values (S9) demand tighter staleness bounds (S8) " +
      "to ensure compliance attestations reflect current regulatory state, shorter migration " +
      "windows (S10) to limit exposure during transfer, fewer bridge hops (S11) to reduce " +
      "counterparty risk, and higher fees (S12) to compensate watchers for the increased " +
      "slashing risk. The compliance tensor evaluation incorporates S8 to determine whether " +
      "cached pack evaluations are still valid."
    ),

    // --- Switch Reference Table (existing) ---
    h2("C.2 Switch Definitions"),
    table(
      ["Switch", "Default", "Range", "Effect"],
      [
        ["S1: Harbor shards", "8", "1-256", "Horizontal capacity"],
        ["S2: Corridor shards", "4", "1-64", "Cross-jurisdiction capacity"],
        ["S3: Block size", "1MB", "256KB-16MB", "Throughput vs latency"],
        ["S4: Block interval", "500ms", "100ms-5s", "Throughput vs latency"],
        ["S5: Proof batch size", "1000", "100-10000", "Amortization"],
        ["S6: Checkpoint interval", "1000", "100-10000", "Verification efficiency"],
        ["S7: Watcher quorum", "3-of-5", "1-of-1 to 7-of-9", "Security vs availability"],
        ["S8: Staleness bound", "24h", "1h-7d", "Freshness vs flexibility"],
        ["S9: Max asset value", "$10M", "$1K-$1B", "Risk management"],
        ["S10: Migration duration", "24h", "1h-7d", "Operation bounds"],
        ["S11: Bridge hop limit", "5", "1-10", "Path complexity"],
        ["S12: Fee multiplier", "1.0", "0.1-10.0", "Economic tuning"],
        ["S13: DA enforcement", "Best-effort", "Off/Best-effort/Enforced", "Availability guarantees"],
      ],
      [2400, 1400, 2800, 2760]
    ),

    // --- Deployment Scenario Table ---
    h2("C.3 Deployment Profiles"),
    p(
      "The following table provides recommended switch values for four deployment profiles. " +
      "Minimal is suitable for development and testing. Standard is appropriate for pilot deployments " +
      "with limited traffic. Enterprise handles production workloads for multi-corridor operations. " +
      "Sovereign-GovOS is the target profile for national-scale deployments processing real capital " +
      "across jurisdictions with full regulatory compliance."
    ),
    table(
      ["Switch", "Minimal", "Standard", "Enterprise", "Sovereign-GovOS"],
      [
        ["S1: Harbor shards", "1", "4", "32", "128"],
        ["S2: Corridor shards", "1", "2", "16", "64"],
        ["S3: Block size", "256KB", "1MB", "4MB", "8MB"],
        ["S4: Block interval", "2s", "500ms", "200ms", "100ms"],
        ["S5: Proof batch size", "100", "500", "2000", "5000"],
        ["S6: Checkpoint interval", "100", "500", "2000", "5000"],
        ["S7: Watcher quorum", "1-of-1", "2-of-3", "3-of-5", "5-of-7"],
        ["S8: Staleness bound", "7d", "24h", "6h", "1h"],
        ["S9: Max asset value", "$10K", "$1M", "$100M", "$1B"],
        ["S10: Migration duration", "7d", "24h", "12h", "4h"],
        ["S11: Bridge hop limit", "2", "3", "5", "8"],
        ["S12: Fee multiplier", "0.1", "1.0", "1.5", "2.0"],
        ["S13: DA enforcement", "Off", "Best-effort", "Best-effort", "Enforced"],
      ],
      [2400, 1400, 1600, 1800, 2160]
    ),

    p_runs([bold("Minimal: "), "Single-shard, relaxed bounds, no DA enforcement. Suitable for local development, " +
      "unit testing, and rapid iteration. Not suitable for any data with real economic value."]),
    p_runs([bold("Standard: "), "Moderate parallelism with reasonable security parameters. Appropriate for " +
      "pilot programs with limited entity counts and supervised corridor operations."]),
    p_runs([bold("Enterprise: "), "High parallelism with strong security guarantees. Designed for " +
      "production multi-corridor operations handling significant capital flows across multiple jurisdictions."]),
    p_runs([bold("Sovereign-GovOS: "), "Maximum parallelism with the strictest security, freshness, and " +
      "availability guarantees. Required for national-scale deployment where the EZ Stack serves as the " +
      "jurisdictional compliance backbone for government operations. S13 must be Enforced, S7 must be at " +
      "least 5-of-7, and S8 must be 1h or less to satisfy regulatory requirements for real-time compliance attestation."]),
  ];
};
