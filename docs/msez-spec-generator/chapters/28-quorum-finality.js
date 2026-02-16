const {
  chapterHeading, h2, h3,
  p, codeBlock, table, theorem
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

    theorem("Theorem 28.1 (Watcher Accountability).", "The slashing mechanism ensures watcher accountability. Dishonest attestations result in provable collateral loss. Given a conflicting attestation pair from the same watcher for the same (asset, jurisdiction, domain) tuple, the slashing contract verifies signatures, confirms conflict, and executes bond forfeiture."),

    // --- 28.3 Quorum Selection per Asset Class ---
    h2("28.3 Quorum Selection per Asset Class"),
    p("Corridor governance selects quorum policy based on the risk profile of the operation."),
    table(
      ["Operation Class", "Quorum Policy", "Rationale"],
      [
        ["High-value settlement (DVP > $1M)", "SuperMajority or Unanimous", "Maximum economic security for large-value transfers"],
        ["Routine trade finance (LC amendments, doc releases)", "Majority", "Speed: fast attestation for low-risk operations"],
        ["Critical state transitions (corridor suspension, governance amendment)", "BondWeighted (min 0.67)", "Two-thirds of economic stake must agree"],
        ["Standard corridor operations", "Threshold {k, n}", "Configurable per corridor governance agreement"],
      ],
      [2400, 2600, 4360]
    ),

    // --- 28.4 Finality Upgrade Path ---
    h2("28.4 Finality Upgrade Path"),
    p("Operations begin at Provisional finality and upgrade progressively. The finality upgrade is monotonic: once an operation reaches a higher finality level, it never degrades. The upgrade process is asynchronous; business operations can proceed at Provisional finality while Confirmed and Anchored finality accumulate in the background. This enables sub-second operational latency (Provisional) with full settlement guarantee (Anchored) within minutes."),
    table(
      ["Upgrade", "Trigger", "Typical Latency", "Reversible"],
      [
        ["Provisional \u2192 Confirmed", "Quorum attestation received", "1\u201330 seconds", "No (monotonic)"],
        ["Confirmed \u2192 Anchored", "Checkpoint committed to L1", "1\u20135 minutes", "No (monotonic)"],
        ["Anchored \u2192 Sovereign", "Jurisdictional authority ratification", "Hours to days", "No (monotonic)"],
      ],
      [2400, 2800, 2000, 2160]
    ),
  ];
};
