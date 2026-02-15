const {
  chapterHeading, h2,
  p, codeBlock, table, spacer
} = require("../lib/primitives");

module.exports = function build_chapter27() {
  return [
    chapterHeading("Chapter 27: Bond and Slashing Mechanics"),

    // --- 27.1 Watcher Bonds ---
    h2("27.1 Watcher Bonds"),
    ...codeBlock(
      "/// Bond backing a watcher's attestation authority.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct WatcherBond {\n" +
      "    pub bond_id: BondId,\n" +
      "    pub watcher_id: WatcherId,\n" +
      "    pub amount: u128,\n" +
      "    pub currency: CurrencyCode,\n" +
      "    pub locked_at: DateTime<Utc>,\n" +
      "    pub unlock_period: Duration,\n" +
      "    pub status: BondStatus,\n" +
      "    pub attestation_limit: u128,\n" +
      "    pub jurisdictions: Vec<JurisdictionId>,\n" +
      "    pub domains: Vec<ComplianceDomain>,\n" +
      "    pub slashing_history: Vec<SlashingEvent>,\n" +
      "    pub last_topped_up: Option<DateTime<Utc>>,\n" +
      "}"
    ),
    p("The WatcherBond contains twelve fields: bond_id (unique bond identifier), watcher_id (owning watcher), amount (staked collateral in base units), currency (bond denomination), locked_at (bond creation timestamp), unlock_period (minimum lock duration before withdrawal), status (Active, Partially_Slashed, Fully_Slashed, Unbonding, Withdrawn), attestation_limit (maximum attestable value, typically 10x bond amount), jurisdictions (authorized jurisdiction scope), domains (authorized compliance domain scope), slashing_history (ordered record of all slashing events against this bond), and last_topped_up (timestamp of most recent bond increase)."),

    // --- 27.2 Slashing Conditions ---
    h2("27.2 Slashing Conditions"),
    table(
      ["Condition", "Trigger", "Evidence", "Penalty"],
      [
        ["SC1: Conflicting Attestation", "Same watcher signs contradictory attestations for the same (asset, jurisdiction, domain) tuple", "Two signed attestations with conflicting compliance states and matching tuple identifiers", "100% bond forfeiture"],
        ["SC2: Stale Attestation", "Watcher attests to compliance state using expired or revoked evidence", "Attestation timestamp post-dates evidence expiry or revocation record", "25% bond forfeiture"],
        ["SC3: Scope Violation", "Watcher attests outside their registered jurisdiction or domain scope", "Attestation references jurisdiction or domain not in watcher's registered roles", "50% bond forfeiture"],
        ["SC4: Liveness Failure", "Watcher fails to produce required attestations within the liveness window", "Absence of attestation records during mandatory reporting period", "10% bond forfeiture per missed period"],
      ],
      [2000, 2000, 3000, 2360]
    ),
    spacer(),
  ];
};
