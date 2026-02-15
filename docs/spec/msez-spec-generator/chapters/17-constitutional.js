const {
  partHeading, chapterHeading, h2,
  p, p_runs, bold,
  codeBlock, table,
  spacer
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
    spacer(),
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
    spacer(),

    // --- 17.2 Voting Mechanisms ---
    h2("17.2 Voting Mechanisms"),
    p_runs([bold("Token-Weighted Voting."), " Voting power proportional to token holdings. Suitable for economic decisions where financial stake should determine influence."]),
    p_runs([bold("One-Entity-One-Vote."), " Equal voting power regardless of economic stake. Suitable for governance decisions affecting fundamental rights."]),
    p_runs([bold("Quadratic Voting."), " Voting power proportional to the square root of tokens committed. Balances economic stake with broader participation."]),
    p_runs([bold("Conviction Voting."), " Voting power accumulates over time as tokens remain staked on a proposal. Rewards sustained conviction over impulsive voting."]),

    // --- 17.3 Delegation and Representation ---
    h2("17.3 Delegation and Representation"),
    p("The constitutional framework supports liquid democracy features: stakeholders may delegate their voting power to representatives on a per-domain basis, and may revoke or reassign delegation at any time. Delegation is transitive but cycle-detection prevents infinite loops. Vote weight flows through the delegation graph at evaluation time, ensuring that final tallies reflect the current delegation state rather than a stale snapshot."),
  ];
};
