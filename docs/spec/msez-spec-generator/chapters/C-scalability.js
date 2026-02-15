const { chapterHeading, table, spacer } = require("../lib/primitives");

module.exports = function build_appendixC() {
  return [
    chapterHeading("Appendix C: Scalability Switch Reference"),
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
    spacer(),
  ];
};
