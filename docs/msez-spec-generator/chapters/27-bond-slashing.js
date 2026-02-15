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
      "pub struct Bond {\n" +
      "    pub watcher_id: WatcherId,\n" +
      "    pub amount: u128,\n" +
      "    pub currency: CurrencyCode,\n" +
      "    pub locked_at: DateTime<Utc>,\n" +
      "    pub unlock_period: Duration,\n" +
      "    pub status: BondStatus,\n" +
      "    pub slashing_history: Vec<SlashingEvent>,\n" +
      "    /// Minimum bond required for this watcher's tier.\n" +
      "    pub min_bond_amount: u128,\n" +
      "    /// Maximum attestation value this bond covers (typically 10x bond amount).\n" +
      "    pub max_attestation_value: u128,\n" +
      "    /// Jurisdictions this bond is scoped to.\n" +
      "    pub jurisdiction_scope: Vec<JurisdictionCode>,\n" +
      "    /// Compliance domains this bond covers.\n" +
      "    pub domain_scope: Vec<ComplianceDomain>,\n" +
      "    /// Graduated slashing rate schedule keyed by offense count.\n" +
      "    pub slashing_rate_schedule: Vec<SlashingRateTier>,\n" +
      "}"
    ),
    p("Bond amount determines the maximum value a watcher can attest to, typically 10x the bond amount. The max_attestation_value field encodes this limit explicitly and is enforced at attestation time. The jurisdiction_scope and domain_scope fields restrict the watcher to attesting only within their registered jurisdictions and compliance domains; any attestation outside these scopes triggers SC3 slashing. The slashing_rate_schedule enables graduated penalties: a first offense may incur a lower rate than repeated violations, incentivizing good behavior while allowing recovery from isolated mistakes."),

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

    // --- 27.3 Bond Lifecycle ---
    h2("27.3 Bond Lifecycle"),
    p("A bond progresses through a well-defined lifecycle from posting through release. Understanding this lifecycle is essential for watchers managing their capital commitment and for the protocol enforcing attestation authority."),
    table(
      ["Phase", "Description", "Status Value", "Transitions"],
      [
        ["Posting", "Watcher submits bond transaction specifying amount, currency, jurisdiction_scope, and domain_scope. The protocol verifies amount >= min_bond_amount for the requested scope. Funds are locked in escrow.", "Pending", "Pending -> Active (on confirmation)"],
        ["Active", "Bond is confirmed on-chain. Watcher gains attestation authority for the declared scope up to max_attestation_value. The bond accrues eligibility time toward the unbonding requirement.", "Active", "Active -> Slashed, Active -> Unbonding"],
        ["Accrual", "While active, the bond accumulates tenure. Longer tenure may increase the watcher's priority in quorum selection and may qualify for reduced slashing rates under the slashing_rate_schedule.", "Active (sub-state)", "Continuous while Active"],
        ["Slashing", "Upon detection of a slashing condition (SC1-SC4), the penalty amount is deducted from the bond. If remaining amount < min_bond_amount, the bond transitions to Suspended and the watcher loses attestation authority until they top up.", "Slashed / Suspended", "Slashed -> Active (top-up), Slashed -> Unbonding"],
        ["Unbonding", "Watcher initiates withdrawal. Attestation authority is revoked immediately. Funds remain locked for the full unlock_period (typically 7-30 days depending on jurisdiction) to allow pending slashing challenges.", "Unbonding", "Unbonding -> Released (after unlock_period)"],
        ["Release", "After unlock_period elapses with no pending challenges, remaining funds are returned to the watcher. The bond record is retained for audit trail purposes.", "Released", "Terminal state"],
      ],
      [1400, 3960, 1600, 2400]
    ),
    spacer(),

    // --- 27.4 Quorum Staleness Requirements ---
    h2("27.4 Quorum Staleness Requirements"),
    p("To prevent compliance attestations from relying on outdated information, the protocol enforces quorum staleness bounds. Every attestation quorum must satisfy the following: (1) at least two-thirds of participating watchers must have refreshed their evidence within the staleness window, which is jurisdiction-dependent but defaults to 24 hours; (2) no individual attestation within the quorum may reference evidence older than the maximum staleness threshold (72 hours by default); (3) if any watcher in a quorum submits an attestation that references evidence past its expiry timestamp, that attestation is excluded from the quorum tally and the watcher is flagged for SC2 review. These bounds ensure that compliance state reflects current regulatory reality rather than stale snapshots, which is critical for jurisdictions with rapidly changing sanctions lists or regulatory calendars."),
    spacer(),
  ];
};
