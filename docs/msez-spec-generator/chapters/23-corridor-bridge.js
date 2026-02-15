const {
  chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter23() {
  return [
    chapterHeading("Chapter 23: Corridor Bridge Protocol"),

    // --- 23.1 Bridge Architecture ---
    h2("23.1 Bridge Architecture"),
    p("Cross-corridor asset transfers require a bridge protocol that maintains atomicity, compliance, and auditability across disjoint state channels. The bridge architecture decomposes transfers into locked segments: the source corridor locks the asset, the bridge verifies compliance predicates for both source and destination jurisdictions, and the destination corridor mints a corresponding claim. Each segment produces a receipt anchored to the respective corridor's receipt chain, ensuring full traceability even when corridors operate under different governance frameworks."),
    ...codeBlock(
      "/// A bridge transfer request between corridors.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct BridgeTransfer {\n" +
      "    pub transfer_id: BridgeTransferId,\n" +
      "    pub source_corridor: CorridorId,\n" +
      "    pub destination_corridor: CorridorId,\n" +
      "    pub asset_id: AssetId,\n" +
      "    pub amount: Amount,\n" +
      "    pub path: Vec<CorridorId>,\n" +
      "    pub compliance_proofs: Vec<ComplianceProof>,\n" +
      "    pub lock_receipt: Option<Digest>,\n" +
      "    pub state: BridgeTransferState,\n" +
      "}"
    ),
    spacer(),

    // --- 23.2 Path Discovery ---
    h2("23.2 Path Discovery"),
    p("The PathRouter discovers optimal transfer paths across the corridor graph using a modified Dijkstra algorithm. Edge weights incorporate four cost factors:"),
    table(
      ["Factor", "Weight", "Source"],
      [
        ["Transfer Fee", "Direct cost (basis points on value)", "Corridor governance parameters"],
        ["Compliance Overhead", "Tensor evaluation cost per hop", "Number of domains \u00d7 jurisdictions"],
        ["Settlement Latency", "Time to achieve Confirmed finality", "Historical corridor performance"],
        ["Reliability", "Success rate over trailing 30 days", "Corridor monitoring data"],
      ],
      [2400, 3200, 3760]
    ),
    spacer(),
    p("When no direct corridor exists between source and destination, the router identifies multi-hop paths through intermediary corridors, subject to the constraint that every intermediary must satisfy compliance requirements for the asset class being transferred. Path discovery results are cached with corridor-state-dependent invalidation."),

    // --- 23.3 Atomic Execution ---
    h2("23.3 Atomic Execution"),
    p("Bridge transfers execute through a six-phase atomic protocol:"),
    table(
      ["Phase", "Action", "Rollback on Failure"],
      [
        ["1. Lock", "Source corridor locks the asset and emits a lock receipt", "Release lock, return to pre-transfer state"],
        ["2. Verify", "Bridge validates compliance predicates across all corridor hops", "Release source lock"],
        ["3. Prepare", "Destination corridor reserves capacity and confirms acceptance", "Release source lock, cancel reservation"],
        ["4. Commit", "Source corridor finalizes the lock and produces a commitment proof", "Reverse commitment, release locks"],
        ["5. Mint", "Destination corridor creates the corresponding asset claim", "Burn minted claim, reverse commitment"],
        ["6. Confirm", "Both corridors exchange confirmation receipts", "N/A (irreversible after confirmation)"],
      ],
      [1600, 4200, 3560]
    ),
    spacer(),
    p("Failure at any phase triggers a deterministic rollback that restores all corridors to their pre-transfer state. The rollback is idempotent and uses the same compensation pattern as the migration saga."),

    // --- 23.4 Fee Computation ---
    h2("23.4 Fee Computation"),
    p("Bridge fees are computed as the sum of per-hop corridor fees, compliance verification costs, and a bridge coordination fee. Corridor fees are set by each jurisdiction's governance parameters and may vary by asset class, transfer volume, and time of day. Compliance verification costs reflect the computational overhead of evaluating tensor slices across multiple jurisdictions. The bridge coordination fee covers state synchronization and receipt chain anchoring. Fee estimates are provided during path discovery and guaranteed for a configurable hold period (default: 60 seconds)."),
    ...codeBlock(
      "/// Fee breakdown for a bridge transfer.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct BridgeFeeEstimate {\n" +
      "    pub corridor_fees: Vec<(CorridorId, Amount)>,\n" +
      "    pub compliance_cost: Amount,\n" +
      "    pub coordination_fee: Amount,\n" +
      "    pub total: Amount,\n" +
      "    pub currency: CurrencyCode,\n" +
      "    pub valid_until: DateTime<Utc>,\n" +
      "}"
    ),
    spacer(),
  ];
};
