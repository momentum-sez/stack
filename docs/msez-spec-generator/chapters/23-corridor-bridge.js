const {
  chapterHeading, h2,
  p
} = require("../lib/primitives");

module.exports = function build_chapter23() {
  return [
    chapterHeading("Chapter 23: Corridor Bridge Protocol"),

    // --- 23.1 Bridge Architecture ---
    h2("23.1 Bridge Architecture"),
    p("Cross-corridor asset transfers require a bridge protocol that maintains atomicity, compliance, and auditability across disjoint state channels. The bridge architecture decomposes transfers into locked segments: the source corridor locks the asset, the bridge verifies compliance predicates for both source and destination jurisdictions, and the destination corridor mints a corresponding claim. Each segment produces a receipt anchored to the respective corridor's receipt chain, ensuring full traceability even when corridors operate under different governance frameworks."),

    // --- 23.2 Path Discovery ---
    h2("23.2 Path Discovery"),
    p("The PathRouter discovers optimal transfer paths across the corridor graph using a modified Dijkstra algorithm. Edge weights incorporate transfer fees, compliance overhead, settlement latency, and historical reliability. When no direct corridor exists between source and destination, the router identifies multi-hop paths through intermediary corridors, subject to the constraint that every intermediary must satisfy compliance requirements for the asset class being transferred. Path discovery results are cached with corridor-state-dependent invalidation."),

    // --- 23.3 Atomic Execution ---
    h2("23.3 Atomic Execution"),
    p("Bridge transfers execute through a six-phase atomic protocol: lock (source corridor locks the asset and emits a lock receipt), verify (bridge validates compliance predicates across all corridor hops), prepare (destination corridor reserves capacity and confirms acceptance), commit (source corridor finalizes the lock and produces a commitment proof), mint (destination corridor creates the corresponding asset claim backed by the commitment proof), and confirm (both corridors exchange confirmation receipts and update their state channels). Failure at any phase triggers a deterministic rollback that restores all corridors to their pre-transfer state."),

    // --- 23.4 Fee Computation ---
    h2("23.4 Fee Computation"),
    p("Bridge fees are computed as the sum of per-hop corridor fees, compliance verification costs, and a bridge coordination fee. Corridor fees are set by each jurisdiction's governance parameters and may vary by asset class, transfer volume, and time of day. Compliance verification costs reflect the computational overhead of evaluating tensor slices across multiple jurisdictions. The bridge coordination fee covers state synchronization and receipt chain anchoring. Fee estimates are provided during path discovery and guaranteed for a configurable hold period."),
  ];
};
