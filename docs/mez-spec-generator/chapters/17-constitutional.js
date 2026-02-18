const {
  partHeading, chapterHeading, h2, h3,
  p, p_runs, bold,
  codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter17() {
  return [
    ...partHeading("PART VII: GOVERNANCE AND CIVIC SYSTEMS"),
    chapterHeading("Chapter 17: Constitutional Framework"),

    // --- 17.1 Constitutional Structure ---
    h2("17.1 Constitutional Structure"),
    p("Zone constitutions are hierarchical documents with provisions at multiple protection levels:"),
    table(
      ["Level", "Modification Requirement", "Typical Content"],
      [
        ["Level 1", "Zone dissolution/reformation", "Fundamental rights, structural provisions"],
        ["Level 2", "Supermajority approval (75%+)", "Constitutional amendments"],
        ["Level 3", "Simple majority approval", "Major policy changes"],
        ["Level 4", "Administrative action", "Operational policies, fee schedules"],
      ],
      [1600, 3200, 4560]
    ),
    ...codeBlock(
      "/// Zone constitutional framework.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct Constitution {\n" +
      "    pub zone_id: ZoneId,\n" +
      "    pub preamble: String,\n" +
      "    pub core_provisions: Vec<Provision>,\n" +
      "    pub standard_provisions: Vec<Provision>,\n" +
      "    pub operational_policies: Vec<Provision>,\n" +
      "    pub amendment_procedures: Vec<AmendmentProcedure>,\n" +
      "    pub stakeholder_rights: Vec<StakeholderRight>,\n" +
      "}"
    ),

    // --- 17.2 Voting Mechanisms ---
    h2("17.2 Voting Mechanisms"),
    p_runs([bold("Token-Weighted Voting."), " Voting power proportional to token holdings. Suitable for economic decisions where financial stake should determine influence."]),
    p_runs([bold("One-Entity-One-Vote."), " Equal voting power regardless of economic stake. Suitable for governance decisions affecting fundamental rights."]),
    p_runs([bold("Quadratic Voting."), " Voting power proportional to the square root of tokens committed. Balances economic stake with broader participation."]),
    p_runs([bold("Conviction Voting."), " Voting power accumulates over time as tokens remain staked on a proposal. Rewards sustained conviction over impulsive voting."]),

    // --- 17.2.1 Delegation and Representation ---
    h3("17.2.1 Delegation and Representation"),
    p("The constitutional framework supports liquid democracy features: stakeholders may delegate their voting power to representatives on a per-domain basis, and may revoke or reassign delegation at any time. Delegation is transitive but cycle-detection prevents infinite loops. Vote weight flows through the delegation graph at evaluation time, ensuring that final tallies reflect the current delegation state rather than a stale snapshot."),

    // --- 17.3 Amendment Protocol ---
    h2("17.3 Amendment Protocol"),
    p("Constitutional amendments follow a structured protocol: proposal submission (any qualified stakeholder), review period (minimum 30 days for Level 2-3 changes), public comment (recorded in the amendment's receipt chain), voting window (configurable per amendment level, typically 14 days), and enactment (automatic lawpack update upon vote success). Each stage transition produces a Verifiable Credential recording the amendment's progress. Failed amendments enter a cooldown period (configurable, default 90 days) before resubmission. The amendment protocol integrates with the agentic trigger system to generate notifications at each stage transition."),
    table(
      ["Amendment Level", "Proposal Threshold", "Voting Period", "Approval Requirement"],
      [
        ["Level 2 (Constitutional)", "10% of stakeholders", "30 days", "75% supermajority"],
        ["Level 3 (Policy)", "5% of stakeholders", "14 days", "Simple majority (50%+1)"],
        ["Level 4 (Operational)", "Zone administrator", "7 days", "Administrative approval"],
      ],
      [2400, 2200, 2000, 2760]
    ),

    // --- 17.3.1 Governance Interoperability ---
    h3("17.3.1 Governance Interoperability"),
    p("Governance decisions in one zone may affect connected zones through corridor agreements. When a zone amends a provision that affects corridor compliance baselines, the amendment protocol propagates a notification to all affected corridor counterparties. The counterparty zone evaluates the amendment against its own governance rules and either acknowledges acceptance (corridor continues), requests renegotiation (corridor enters SUSPENDED state pending resolution), or triggers termination (if the amendment violates a fundamental corridor term). This cascading governance mechanism ensures that unilateral policy changes do not silently degrade corridor compliance guarantees."),
  ];
};
