const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter10() {
  return [
    pageBreak(),
    chapterHeading("Chapter 10: Compliance Tensor V2"),

    // --- 10.1 Mathematical Definition ---
    h2("10.1 Mathematical Definition"),
    definition(
      "Definition 10.1 (Compliance Tensor).",
      "The compliance tensor C is a function: C: AssetID \u00D7 JurisdictionID \u00D7 ComplianceDomain \u00D7 TimeQuantum \u2192 ComplianceState."
    ),
    ...codeBlock(
      "/// Compliance state for a single (jurisdiction, domain) pair.\n" +
      "/// Five-state lattice as implemented in mez-tensor.\n" +
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum ComplianceState {\n" +
      "    Compliant,     // Entity meets all requirements in this domain\n" +
      "    NonCompliant,  // Specific violations exist\n" +
      "    Pending,       // Evaluation in progress or awaiting attestation\n" +
      "    Exempt,        // Entity exempt from this domain (e.g., de minimis)\n" +
      "    NotApplicable, // Domain does not apply to entity classification\n" +
      "}\n" +
      "\n" +
      "/// Lattice ordering (lower is more restrictive):\n" +
      "/// NonCompliant(0) < Pending(1) < Compliant(2) = Exempt(2) = NotApplicable(2)\n" +
      "impl ComplianceState {\n" +
      "    fn ordering(self) -> u8 {\n" +
      "        match self {\n" +
      "            Self::NonCompliant  => 0,\n" +
      "            Self::Pending       => 1,\n" +
      "            Self::Compliant     => 2,\n" +
      "            Self::Exempt        => 2,\n" +
      "            Self::NotApplicable => 2,\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),

    // --- 10.2 Compliance Domains ---
    h2("10.2 Compliance Domains"),
    p("Twenty compliance domains span the regulatory landscape:"),
    table(
      ["Domain", "Description"],
      [
        ["Aml", "Anti-money laundering: transaction monitoring, suspicious activity reporting"],
        ["Kyc", "Know Your Customer: identity verification, due diligence tiers"],
        ["Sanctions", "Sanctions screening: OFAC, UN, EU lists, PEP checks"],
        ["Tax", "Tax compliance: withholding, reporting, filing, transfer pricing"],
        ["Securities", "Securities regulation: issuance, trading, disclosure obligations"],
        ["Corporate", "Corporate governance: formation, director duties, beneficial ownership"],
        ["Custody", "Custody requirements: asset safekeeping, segregation, reporting"],
        ["DataPrivacy", "Data privacy: GDPR, PDPA, cross-border data transfer restrictions"],
        ["Licensing", "Licensing: business permits, professional certifications, renewals"],
        ["Banking", "Banking regulation: reserve requirements, capital adequacy, deposit-taking"],
        ["Payments", "Payment services: PSP licensing, payment instrument rules, e-money"],
        ["Clearing", "Clearing and settlement: CCP rules, netting, margin requirements"],
        ["Settlement", "Settlement finality: delivery-versus-payment, settlement cycles"],
        ["DigitalAssets", "Digital asset regulation: token classification, exchange licensing, custody"],
        ["Employment", "Employment law: labor contracts, social security, workplace safety"],
        ["Immigration", "Immigration: work permits, visa sponsorship, foreign worker quotas"],
        ["Ip", "Intellectual property: patents, trademarks, copyrights, trade secrets"],
        ["ConsumerProtection", "Consumer protection: disclosure, dispute resolution, warranties"],
        ["Arbitration", "Arbitration: dispute resolution frameworks, enforcement of awards"],
        ["Trade", "Trade regulation: import/export controls, customs, tariffs"],
      ],
      [2400, 6960]
    ),

    // --- 10.3 Compliance States ---
    h2("10.3 Compliance States"),
    p("The five compliance states form a bounded lattice with partial order:"),
    p("NonCompliant(0) < Pending(1) < {Compliant, Exempt, NotApplicable}(2)"),
    p_runs([
      bold("NonCompliant"), " is the bottom element and absorbing element under meet. It indicates specific violations exist. ",
      bold("Pending"), " indicates an evaluation is in progress or awaiting attestation. In production mode, unimplemented domains default to Pending (fail-closed) rather than passing. ",
      bold("Compliant"), " means the entity meets all requirements in this domain. ",
      bold("Exempt"), " means the entity is exempt from this domain (e.g., de minimis threshold). ",
      bold("NotApplicable"), " means the domain does not apply to the entity\u2019s classification. ",
      "Compliant, Exempt, and NotApplicable share the same lattice rank. Meet (\u2227) returns the pessimistic (lower) state. Join (\u2228) returns the optimistic (higher) state."
    ]),

    // --- 10.3.1 Lattice Operations ---
    h3("10.3.1 Lattice Operations: Meet and Join"),
    p("The meet (\u2227) operation computes the greatest lower bound (pessimistic composition). When composing compliance across multiple domains or jurisdictions, meet ensures the result reflects the weakest link. The join (\u2228) operation computes the least upper bound (optimistic composition), used when any single passing domain suffices."),
    definition(
      "Definition 10.6 (Meet / Greatest Lower Bound).",
      "For states a, b: a \u2227 b = the state with min(ordering(a), ordering(b)). When both are rank 2, meet preserves the more specific (Compliant > Exempt > NotApplicable)."
    ),
    definition(
      "Definition 10.7 (Join / Least Upper Bound).",
      "For states a, b: a \u2228 b = the state with max(ordering(a), ordering(b)). When both are rank 2, join preserves the more permissive."
    ),
    p("The following table shows meet (\u2227) results for all state pairs. The table is symmetric."),
    table(
      ["meet (\u2227)", "NonCompliant", "Pending", "Compliant", "Exempt", "N/A"],
      [
        ["NonCompliant", "NonCompliant", "NonCompliant", "NonCompliant", "NonCompliant", "NonCompliant"],
        ["Pending",      "NonCompliant", "Pending",      "Pending",      "Pending",      "Pending"],
        ["Compliant",    "NonCompliant", "Pending",      "Compliant",    "Exempt",       "NotApplicable"],
        ["Exempt",       "NonCompliant", "Pending",      "Exempt",       "Exempt",       "NotApplicable"],
        ["NotApplicable","NonCompliant", "Pending",      "NotApplicable","NotApplicable", "NotApplicable"],
      ],
      [1560, 1560, 1560, 1560, 1560, 1560]
    ),

    // --- 10.4 Tensor Operations ---
    h2("10.4 Tensor Operations"),
    definition(
      "Definition 10.2 (Tensor Slice).",
      "A slice fixes one or more dimensions. slice_aj = tensor[A, J, :, :] retrieves all compliance states for asset A in jurisdiction J. Slice operations enable efficient cross-border verification without full tensor materialization."
    ),
    definition(
      "Definition 10.3 (Tensor Update).",
      "Incremental update from attestation integrates new attestation references into tensor cells, updates compliance state, and recomputes aggregate values."
    ),
    definition(
      "Definition 10.4 (Tensor Commitment).",
      "The tensor commitment enables ZK proofs by building a Merkle tree over all cells using Poseidon2 hashes. The commitment includes the root, jurisdiction count, domain count, and last update timestamp."
    ),
    ...codeBlock(
      "/// The compliance tensor: 4-dimensional compliance state map.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct ComplianceTensor {\n" +
      "    pub entries: HashMap<(AssetId, JurisdictionId, ComplianceDomain), TensorEntry>,\n" +
      "    pub jurisdiction_count: usize,\n" +
      "    pub domain_count: usize,\n" +
      "    pub last_updated: chrono::DateTime<chrono::Utc>,\n" +
      "    pub commitment: Option<TensorCommitment>,\n" +
      "}\n" +
      "\n" +
      "/// A single tensor cell entry.\n" +
      "#[derive(Debug, Clone, Serialize, Deserialize)]\n" +
      "pub struct TensorEntry {\n" +
      "    pub state: ComplianceState,\n" +
      "    pub attestation_ref: Option<Digest>,\n" +
      "    pub evaluated_at: chrono::DateTime<chrono::Utc>,\n" +
      "    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,\n" +
      "    pub evidence: Vec<EvidenceRef>,\n" +
      "}"
    ),

    // --- 10.5 Cross-Border Compliance Verification ---
    h3("10.4.1 Cross-Border Compliance Verification"),
    definition(
      "Definition 10.5 (Cross-Border Predicate).",
      "For transfer from jurisdiction J1 to J2: verify export requirements from source, verify import requirements at destination, compute combined compliance using pessimistic meet operation, and return true only if combined state is COMPLIANT or EXEMPT."
    ),
    p("Tensor slices are cached per jurisdiction pair with configurable TTL. Cache invalidation occurs on any tensor update affecting the relevant jurisdictions. For high-frequency corridors (e.g., PAK\u2194UAE), pre-computed slices are maintained in memory to avoid re-evaluation on every transaction."),
  ];
};
