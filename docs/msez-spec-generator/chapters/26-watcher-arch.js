const {
  partHeading, chapterHeading, h2,
  p, codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter26() {
  return [
    ...partHeading("PART X: WATCHER ECONOMY"),
    chapterHeading("Chapter 26: Watcher Architecture"),

    // --- 26.1 Watcher Identity ---
    h2("26.1 Watcher Identity"),
    p("The Watcher Economy transforms watchers from passive observers to accountable economic actors whose attestations carry weight backed by staked collateral."),
    ...codeBlock(
      "/// A watcher is an accountable attestation agent in the watcher economy.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Watcher {\n" +
      "    pub id: WatcherId,\n" +
      "    pub public_key: PublicKey,\n" +
      "    pub bond: BondState,\n" +
      "    pub roles: Vec<WatcherRole>,\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub reputation_score: u64,\n" +
      "    pub registered_at: DateTime<Utc>,\n" +
      "    pub last_attestation: Option<DateTime<Utc>>,\n" +
      "}"
    ),
    spacer(),

    // --- 26.2 Watcher Roles ---
    h2("26.2 Watcher Roles"),
    table(
      ["Role", "Function", "Scope"],
      [
        ["Compliance Watcher", "Attests to compliance state of entities and assets against regulatory requirements", "Per-jurisdiction, per-domain"],
        ["Corridor Watcher", "Monitors corridor state transitions and attests to receipt chain integrity", "Per-corridor"],
        ["Settlement Watcher", "Verifies settlement finality and anchoring to external chains", "Cross-corridor"],
        ["Audit Watcher", "Performs periodic audits of watcher attestations and flags inconsistencies", "System-wide"],
      ],
      [2400, 3600, 3360]
    ),
    spacer(),
  ];
};
