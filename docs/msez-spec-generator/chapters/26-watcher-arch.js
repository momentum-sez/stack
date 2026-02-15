const {
  partHeading, chapterHeading, h2, h3,
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

    // --- 26.3 Watcher Profile ---
    h2("26.3 Watcher Profile"),
    p("Beyond the core Watcher struct, each watcher maintains a WatcherProfile that captures operational capabilities and constraints. The profile is used by the quorum selection algorithm to ensure that attestation panels are composed of watchers with the appropriate specializations, jurisdictional coverage, and capacity. Profiles are updated by the watcher operator and validated by Audit Watchers during periodic reviews."),
    ...codeBlock(
      "/// Extended operational profile for a watcher.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct WatcherProfile {\n" +
      "    /// Reference to the base watcher identity.\n" +
      "    pub watcher_id: WatcherId,\n" +
      "    /// Compliance domains this watcher is qualified to attest.\n" +
      "    pub specializations: Vec<ComplianceDomain>,\n" +
      "    /// Jurisdictions this watcher is authorized to operate in.\n" +
      "    pub supported_jurisdictions: Vec<JurisdictionId>,\n" +
      "    /// Compliance domains this watcher can evaluate.\n" +
      "    pub supported_domains: Vec<ComplianceDomain>,\n" +
      "    /// Maximum number of attestations the watcher can process concurrently.\n" +
      "    pub max_concurrent_attestations: u32,\n" +
      "    /// Uptime SLA commitment as a percentage (e.g., 99.9).\n" +
      "    pub uptime_sla: f64,\n" +
      "    /// Timestamp of the last profile validation by an Audit Watcher.\n" +
      "    pub last_validated: Option<DateTime<Utc>>,\n" +
      "    /// Credential proving the watcher's qualifications.\n" +
      "    pub qualification_vc: Option<VerifiableCredential>,\n" +
      "}"
    ),
    spacer(),

    // --- 26.4 Quorum Diversity Requirements ---
    h2("26.4 Quorum Diversity Requirements"),
    p("Attestation quorums must satisfy diversity constraints to prevent jurisdictional capture and ensure independent verification. A quorum is the minimum set of watcher attestations required for a state transition, compliance evaluation, or settlement confirmation to be considered valid. Raw numeric thresholds alone are insufficient; the composition of the quorum matters."),

    h3("26.4.1 Jurisdictional Diversity"),
    p("Every quorum must include watchers from at least two distinct jurisdictions. For corridor operations involving jurisdictions A and B, the quorum must include at least one watcher that is not domiciled in either A or B, providing a neutral third-party perspective. This prevents a situation where all attestors share the same regulatory environment and potential biases."),

    h3("26.4.2 Role Diversity"),
    p("For cross-domain attestations that span compliance and settlement, the quorum must include at least one Compliance Watcher and one Settlement Watcher. Audit Watchers may participate in any quorum but do not count toward the role diversity requirement since their function is meta-verification rather than primary attestation."),

    h3("26.4.3 Anti-Collusion Constraints"),
    p("No single entity may control more than one-third of the watchers in any quorum. Beneficial ownership data from the Mass Identity primitive is cross-referenced to detect common control. Watchers found to be under common control after quorum formation trigger an automatic re-evaluation, and the affected attestations are quarantined until a compliant quorum can be assembled."),
    spacer(),

    // --- 26.5 Reputation Scoring ---
    h2("26.5 Reputation Scoring"),
    p("Each watcher carries a reputation score that quantifies historical reliability and directly influences quorum selection priority, bond requirements, and slashing severity. The reputation system is designed to be monotonically informative: good behavior is rewarded slowly, while bad behavior is penalized swiftly, creating an asymmetric incentive that favors long-term honest participation over short-term exploitation."),

    h3("26.5.1 Scoring Components"),
    table(
      ["Component", "Weight", "Measurement", "Update Frequency"],
      [
        ["Attestation Accuracy", "40%", "Fraction of attestations not challenged or overturned within the dispute window", "Per attestation"],
        ["Uptime Compliance", "20%", "Actual uptime divided by committed SLA over a rolling 30-day window", "Daily"],
        ["Response Latency", "15%", "Median time from attestation request to signed response, relative to corridor SLA", "Per attestation"],
        ["Dispute Record", "15%", "Inverse of disputes lost as a fraction of total attestations, weighted by severity", "Per dispute resolution"],
        ["Tenure", "10%", "Logarithmic function of total days active, rewarding long-term participation with diminishing returns", "Daily"],
      ],
      [1800, 960, 3720, 2880]
    ),
    spacer(),

    h3("26.5.2 Score Mechanics"),
    p("Reputation scores are bounded to the range [0, 1000]. New watchers start at a baseline score of 500. Positive adjustments are capped at +5 per attestation cycle, while negative adjustments for verified failures can reach -50 per incident, enforcing the asymmetric incentive structure. A watcher whose score falls below 200 is automatically suspended from quorum eligibility and must re-stake their bond to resume operations. Scores above 800 qualify the watcher for reduced bond requirements, creating a tangible economic reward for sustained good behavior."),
    spacer(),
  ];
};
