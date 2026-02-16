const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  definition, codeBlock, table
} = require("../lib/primitives");

module.exports = function build_chapter10() {
  return [
    // pageBreak() removed — chapterHeading() now includes pageBreakBefore: true
    chapterHeading("Chapter 10: Compliance Tensor V2"),

    // --- 10.1 Mathematical Definition ---
    h2("10.1 Mathematical Definition"),
    definition(
      "Definition 10.1 (Compliance Tensor).",
      "The compliance tensor C is a function: C: AssetID \u00D7 JurisdictionID \u00D7 ComplianceDomain \u00D7 TimeQuantum \u2192 ComplianceState."
    ),
    ...codeBlock(
      "/// Compliance state for a single (jurisdiction, domain) pair.\n" +
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum ComplianceState {\n" +
      "    Unknown,       // No attestation\n" +
      "    NonCompliant,  // Attested non-compliant\n" +
      "    Expired,       // Previously compliant, attestation lapsed\n" +
      "    Suspended,     // Compliance temporarily revoked pending review\n" +
      "    Pending,       // Evaluation in progress\n" +
      "    Compliant,     // Attested compliant\n" +
      "    Exempt,        // Jurisdiction exempts this domain\n" +
      "}\n" +
      "\n" +
      "/// Lattice order: NonCompliant < Expired < Suspended < Unknown < Pending\n" +
      "///                < Compliant, Exempt (incomparable top pair)\n" +
      "impl PartialOrd for ComplianceState {\n" +
      "    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {\n" +
      "        use ComplianceState::*;\n" +
      "        fn rank(s: &ComplianceState) -> Option<u8> {\n" +
      "            match s {\n" +
      "                NonCompliant => Some(0),\n" +
      "                Expired      => Some(1),\n" +
      "                Suspended    => Some(2),\n" +
      "                Unknown      => Some(3),\n" +
      "                Pending      => Some(4),\n" +
      "                Compliant    => Some(5),\n" +
      "                Exempt       => Some(5), // Same rank: incomparable\n" +
      "            }\n" +
      "        }\n" +
      "        match (self, other) {\n" +
      "            (a, b) if a == b => Some(std::cmp::Ordering::Equal),\n" +
      "            // Compliant and Exempt are incomparable\n" +
      "            (Compliant, Exempt) | (Exempt, Compliant) => None,\n" +
      "            _ => rank(self)?.partial_cmp(&rank(other)?),\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),

    // --- 10.2 Compliance Domains ---
    // IMPORTANT: These 20 domains match the ComplianceDomain enum in
    // msez-core/src/domain.rs exactly. Any change here must be reflected there and vice versa.
    h2("10.2 Compliance Domains"),
    p("Twenty compliance domains span the regulatory landscape. These are the canonical variants of the ComplianceDomain enum in msez-core, listed in declaration order. The Rust compiler enforces exhaustive match on this enum \u2014 adding a domain forces every handler in the codebase to address it."),
    table(
      ["Domain", "Enum Variant", "Description"],
      [
        ["AML", "Aml", "Anti-money laundering: transaction monitoring, suspicious activity reporting"],
        ["KYC", "Kyc", "Know Your Customer: identity verification, due diligence"],
        ["Sanctions", "Sanctions", "Sanctions screening: OFAC, UN, EU consolidated lists"],
        ["Tax", "Tax", "Tax compliance: withholding, reporting, filing obligations"],
        ["Securities", "Securities", "Securities regulation: issuance, trading, disclosure"],
        ["Corporate", "Corporate", "Corporate governance: formation, dissolution, beneficial ownership"],
        ["Custody", "Custody", "Custody requirements: asset safekeeping, segregation"],
        ["Data Privacy", "DataPrivacy", "Data privacy: GDPR, PDPA, cross-border data transfer restrictions"],
        ["Licensing", "Licensing", "Licensing: business license validity, professional certifications"],
        ["Banking", "Banking", "Banking regulation: reserve requirements, capital adequacy"],
        ["Payments", "Payments", "Payment services: PSP licensing, payment instrument rules"],
        ["Clearing", "Clearing", "Clearing and settlement: CCP rules, netting, finality"],
        ["Settlement", "Settlement", "Settlement finality: delivery-versus-payment, settlement cycles"],
        ["Digital Assets", "DigitalAssets", "Digital asset regulation: token classification, exchange licensing"],
        ["Employment", "Employment", "Employment law: labor contracts, social security, withholding"],
        ["Immigration", "Immigration", "Immigration: work permits, visa sponsorship, residency"],
        ["IP", "Ip", "Intellectual property: patent, trademark, trade secret"],
        ["Consumer Protection", "ConsumerProtection", "Consumer protection: disclosure, dispute resolution, warranties"],
        ["Arbitration", "Arbitration", "Arbitration: dispute resolution frameworks, enforcement"],
        ["Trade", "Trade", "Trade regulation: import/export controls, customs, tariffs"],
      ],
      [1800, 2000, 5560]
    ),

    // --- 10.3 Compliance States ---
    h2("10.3 Compliance States"),
    p("The seven compliance states form a bounded lattice with partial order:"),
    p("NON_COMPLIANT < EXPIRED < SUSPENDED < UNKNOWN < PENDING < {COMPLIANT, EXEMPT}"),
    p_runs([
      bold("NON_COMPLIANT"), " is the bottom element and absorbing element under meet. ",
      bold("COMPLIANT"), " and ", bold("EXEMPT"), " are incomparable top elements \u2014 neither dominates the other. ",
      bold("EXPIRED"), " represents a previously compliant state whose attestation has lapsed; it transitions to NON_COMPLIANT after a configurable grace period. ",
      bold("SUSPENDED"), " represents compliance temporarily revoked pending regulatory review; it ranks above EXPIRED because the underlying attestation still exists and may be reinstated. ",
      bold("UNKNOWN"), " is the default state when no attestation has been submitted. ",
      bold("PENDING"), " indicates an evaluation is in progress and an attestation is expected. ",
      "Meet (\u2227) returns the pessimistic (lower) state of two operands. Join (\u2228) returns the optimistic (higher) state. For the incomparable pair {COMPLIANT, EXEMPT}, meet yields PENDING and join yields EXEMPT."
    ]),

    // --- 10.3.1 Lattice Operations ---
    h3("10.3.1 Lattice Operations: Meet and Join"),
    p("The meet (\u2227) operation computes the greatest lower bound (pessimistic composition). When composing compliance across multiple domains or jurisdictions, meet ensures the result reflects the weakest link. The join (\u2228) operation computes the least upper bound (optimistic composition), used when any single passing domain suffices."),
    definition(
      "Definition 10.6 (Meet / Greatest Lower Bound).",
      "For states a, b: a \u2227 b = max(s) such that s \u2264 a and s \u2264 b. If a and b are comparable, meet is min(a, b). For the incomparable pair {Compliant, Exempt}, meet(Compliant, Exempt) = Pending."
    ),
    definition(
      "Definition 10.7 (Join / Least Upper Bound).",
      "For states a, b: a \u2228 b = min(s) such that s \u2265 a and s \u2265 b. If a and b are comparable, join is max(a, b). For the incomparable pair {Compliant, Exempt}, join(Compliant, Exempt) = Exempt."
    ),
    p("The following table shows meet (\u2227) results for all state pairs. The table is symmetric: meet(a, b) = meet(b, a)."),
    table(
      ["meet (\u2227)", "NonCompl", "Expired", "Suspended", "Unknown", "Pending", "Compliant", "Exempt"],
      [
        ["NonCompliant", "NonCompl", "NonCompl", "NonCompl", "NonCompl", "NonCompl", "NonCompl", "NonCompl"],
        ["Expired",      "NonCompl", "Expired",  "Expired",  "Expired",  "Expired",  "Expired",  "Expired"],
        ["Suspended",    "NonCompl", "Expired",  "Suspended","Suspended","Suspended","Suspended","Suspended"],
        ["Unknown",      "NonCompl", "Expired",  "Suspended","Unknown",  "Unknown",  "Unknown",  "Unknown"],
        ["Pending",      "NonCompl", "Expired",  "Suspended","Unknown",  "Pending",  "Pending",  "Pending"],
        ["Compliant",    "NonCompl", "Expired",  "Suspended","Unknown",  "Pending",  "Compliant","Pending"],
        ["Exempt",       "NonCompl", "Expired",  "Suspended","Unknown",  "Pending",  "Pending",  "Exempt"],
      ],
      [1170, 1170, 1170, 1170, 1170, 1170, 1170, 1170]
    ),
    p("The following table shows join (\u2228) results for all state pairs. The table is symmetric: join(a, b) = join(b, a)."),
    table(
      ["join (\u2228)", "NonCompl", "Expired", "Suspended", "Unknown", "Pending", "Compliant", "Exempt"],
      [
        ["NonCompliant", "NonCompl", "Expired",  "Suspended","Unknown",  "Pending",  "Compliant","Exempt"],
        ["Expired",      "Expired",  "Expired",  "Suspended","Unknown",  "Pending",  "Compliant","Exempt"],
        ["Suspended",    "Suspended","Suspended", "Suspended","Unknown",  "Pending",  "Compliant","Exempt"],
        ["Unknown",      "Unknown",  "Unknown",  "Unknown",  "Unknown",  "Pending",  "Compliant","Exempt"],
        ["Pending",      "Pending",  "Pending",  "Pending",  "Pending",  "Pending",  "Compliant","Exempt"],
        ["Compliant",    "Compliant","Compliant", "Compliant","Compliant","Compliant","Compliant","Exempt"],
        ["Exempt",       "Exempt",   "Exempt",   "Exempt",   "Exempt",   "Exempt",   "Exempt",  "Exempt"],
      ],
      [1170, 1170, 1170, 1170, 1170, 1170, 1170, 1170]
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
