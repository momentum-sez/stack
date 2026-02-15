const {
  chapterHeading, h2,
  p, codeBlock, table, theorem, spacer
} = require("../lib/primitives");

module.exports = function build_chapter28() {
  return [
    chapterHeading("Chapter 28: Quorum and Finality"),

    // --- 28.1 Quorum Policies ---
    h2("28.1 Quorum Policies"),
    ...codeBlock(
      "/// Quorum policy determining how many watcher attestations are required.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub enum QuorumPolicy {\n" +
      "    /// Simple majority of registered watchers.\n" +
      "    Majority,\n" +
      "    /// Two-thirds supermajority weighted by bond amount.\n" +
      "    SuperMajority,\n" +
      "    /// All registered watchers must attest (unanimous).\n" +
      "    Unanimous,\n" +
      "    /// At least K of N registered watchers must attest.\n" +
      "    Threshold { k: u32, n: u32 },\n" +
      "    /// Bond-weighted: attestations weighted by staked collateral.\n" +
      "    BondWeighted { min_weight_fraction: f64 },\n" +
      "}"
    ),
    spacer(),

    // --- 28.2 Finality Levels ---
    h2("28.2 Finality Levels"),
    table(
      ["Level", "Requirements", "Guarantees"],
      [
        ["Provisional", "Single watcher attestation from a bonded watcher", "Immediate availability; subject to challenge during dispute window"],
        ["Confirmed", "Quorum of watchers attest per corridor quorum policy", "Resistant to single-watcher equivocation; economically secured by aggregate bond"],
        ["Anchored", "Confirmed state anchored to external L1 settlement layer", "Inherits settlement layer finality; irreversible after anchor confirmation depth"],
        ["Sovereign", "Anchored state ratified by jurisdictional governance authority", "Full legal finality; enforceable under jurisdictional law and treaty obligations"],
      ],
      [2000, 3600, 3760]
    ),
    spacer(),

    theorem("Theorem 28.1 (Watcher Accountability).", "The slashing mechanism ensures watcher accountability. Dishonest attestations result in provable collateral loss. Given a conflicting attestation pair from the same watcher for the same (asset, jurisdiction, domain) tuple, the slashing contract verifies signatures, confirms conflict, and executes bond forfeiture."),

    // --- 28.3 Quorum Selection per Asset Class ---
    h2("28.3 Quorum Selection per Asset Class"),
    p("Corridor governance selects quorum policy based on the risk profile of the operation. High-value settlement operations (e.g., cross-corridor DVP for securities above $1M) require SuperMajority or Unanimous quorum to ensure maximum economic security. Routine trade finance operations (LC amendments, document releases) use Majority quorum for speed. Critical state transitions (corridor suspension, governance amendment) require BondWeighted quorum with a minimum weight fraction of 0.67, ensuring that the decision represents at least two-thirds of economic stake in the corridor."),

    // --- 28.4 Finality Upgrade Path ---
    h2("28.4 Finality Upgrade Path"),
    p("Operations begin at Provisional finality and upgrade progressively. The finality upgrade is monotonic: once an operation reaches a higher finality level, it never degrades. The upgrade process is asynchronous; business operations can proceed at Provisional finality while Confirmed and Anchored finality accumulate in the background. This enables sub-second operational latency (Provisional) with full settlement guarantee (Anchored) within minutes."),
  ];
};
