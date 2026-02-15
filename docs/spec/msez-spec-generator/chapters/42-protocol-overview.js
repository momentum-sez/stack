const {
  partHeading, chapterHeading, h2,
  p,
  table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter42() {
  return [
    ...partHeading("PART XV: MASS PROTOCOL INTEGRATION"),

    chapterHeading("Chapter 42: Protocol Overview"),

    // --- 42.1 Protocol Architecture ---
    h2("42.1 Protocol Architecture"),
    p("Mass Protocol provides the settlement layer for the SEZ Stack. The protocol architecture comprises: transaction layer (private and public transaction types), consensus layer (DAG-based with jurisdictional awareness), proving layer (Plonky3 STARKs with Groth16 wrapping), and anchoring layer (periodic state commitment to external chains)."),
    p("Integration patterns between the MSEZ Stack and Mass Protocol follow the anchor-and-verify model: MSEZ Stack operations produce receipts, receipts aggregate into checkpoints, checkpoints anchor to Mass Protocol periodically, and protocol provides finality guarantees for anchored state."),

    // --- 42.2 Integration Patterns ---
    h2("42.2 Integration Patterns"),
    table(
      ["Pattern", "Use Case", "Flow"],
      [
        ["Direct Anchoring", "High-value settlements", "Receipt -> Checkpoint -> L1 Anchor -> Finality"],
        ["Batch Anchoring", "Routine operations", "Multiple Receipts -> Aggregated Checkpoint -> L1 Anchor"],
        ["Corridor Settlement", "Cross-border operations", "Corridor State -> Bilateral Checkpoint -> L1 Anchor"],
        ["Deferred Anchoring", "Low-priority operations", "Receipts accumulated, anchored at next epoch"],
      ],
      [2200, 2800, 4360]
    ),
    spacer(),
  ];
};
