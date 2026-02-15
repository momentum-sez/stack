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
      "/// A watcher profile: the complete identity and state of an attestation agent.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct WatcherProfile {\n" +
      "    pub id: WatcherId,\n" +
      "    pub public_key: PublicKey,\n" +
      "    pub bond: BondState,\n" +
      "    pub roles: Vec<WatcherRole>,\n" +
      "    pub jurisdiction: JurisdictionId,\n" +
      "    pub reputation_score: u64,\n" +
      "    pub registered_at: DateTime<Utc>,\n" +
      "    pub last_attestation: Option<DateTime<Utc>>,\n" +
      "    pub liveness_window: Duration,\n" +
      "}"
    ),
    p("The WatcherProfile captures nine fields: id (unique watcher identifier), public_key (Ed25519 public key for attestation verification), bond (current bond state and amount), roles (set of authorized attestation roles), jurisdiction (home jurisdiction for scope validation), reputation_score (cumulative score updated on each attestation cycle), registered_at (registration timestamp), last_attestation (timestamp of most recent attestation, None if no attestations yet), and liveness_window (maximum interval between required attestations before SC4 triggers)."),
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
