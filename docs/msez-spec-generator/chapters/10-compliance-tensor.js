const {
  chapterHeading, h2,
  p, p_runs, bold,
  definition, codeBlock, table,
  spacer, pageBreak
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
      "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n" +
      "pub enum ComplianceState {\n" +
      "    Unknown,       // No attestation\n" +
      "    NonCompliant,  // Attested non-compliant\n" +
      "    Pending,       // Evaluation in progress\n" +
      "    Compliant,     // Attested compliant\n" +
      "    Exempt,        // Jurisdiction exempts this domain\n" +
      "}\n" +
      "\n" +
      "/// Partial order: Unknown < NonCompliant < Pending < Compliant, Exempt\n" +
      "impl PartialOrd for ComplianceState {\n" +
      "    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {\n" +
      "        use ComplianceState::*;\n" +
      "        match (self, other) {\n" +
      "            (a, b) if a == b => Some(std::cmp::Ordering::Equal),\n" +
      "            (Unknown, _) => Some(std::cmp::Ordering::Less),\n" +
      "            (_, Unknown) => Some(std::cmp::Ordering::Greater),\n" +
      "            (NonCompliant, Pending | Compliant | Exempt) => Some(std::cmp::Ordering::Less),\n" +
      "            (Pending, Compliant | Exempt) => Some(std::cmp::Ordering::Less),\n" +
      "            (Compliant, Exempt) | (Exempt, Compliant) => None,\n" +
      "            _ => other.partial_cmp(self).map(|o| o.reverse()),\n" +
      "        }\n" +
      "    }\n" +
      "}"
    ),
    spacer(),

    // --- 10.2 Compliance Domains ---
    h2("10.2 Compliance Domains"),
    p("Twenty compliance domains span the regulatory landscape:"),
    table(
      ["Domain", "Description"],
      [
        ["CIVIC", "Civic obligations including residency, nationality, and public registry requirements"],
        ["CORPORATE", "Corporate governance, formation, director duties, and beneficial ownership"],
        ["COMMERCIAL", "Commercial law including contracts, trade, and consumer protection"],
        ["FINANCIAL", "Financial regulation including prudential requirements and capital adequacy"],
        ["SECURITIES", "Securities law including issuance, trading, and disclosure obligations"],
        ["BANKING", "Banking regulation including deposit-taking, lending, and reserve requirements"],
        ["PAYMENTS", "Payment systems regulation including e-money, remittance, and settlement"],
        ["DIGITAL_ASSETS", "Digital asset regulation including classification, custody, and exchange"],
        ["TAX", "Tax obligations including income, withholding, VAT/GST, and transfer pricing"],
        ["AML_CFT", "Anti-money laundering and counter-terrorism financing requirements"],
        ["DATA_PROTECTION", "Data protection and privacy including cross-border transfer restrictions"],
        ["ARBITRATION", "Dispute resolution including arbitration, mediation, and enforcement"],
        ["LICENSING", "Licensing requirements including permits, registrations, and renewals"],
        ["INSURANCE", "Insurance regulation including mandatory coverage, reinsurance, and claims handling"],
        ["ENVIRONMENTAL", "Environmental compliance including emissions, waste management, and impact assessments"],
        ["LABOR", "Labor law including employment contracts, workplace safety, and social security contributions"],
        ["INTELLECTUAL_PROPERTY", "IP protection including patents, trademarks, copyrights, and trade secrets"],
        ["IMMIGRATION", "Immigration and work permit requirements for foreign nationals and cross-border personnel"],
        ["REAL_ESTATE", "Property law including ownership restrictions, land registration, and transfer taxes"],
        ["HEALTH_SAFETY", "Health and safety regulation including occupational standards, inspections, and certifications"],
      ],
      [2400, 6960]
    ),
    spacer(),

    // --- 10.3 Compliance States ---
    h2("10.3 Compliance States"),
    p("Compliance states follow a strict lattice: NON_COMPLIANT < EXPIRED < UNKNOWN < PENDING < EXEMPT < COMPLIANT. Meet (\u2227): pessimistic composition returning the lower state. Join (\u2228): optimistic composition returning the higher state. NON_COMPLIANT is the absorbing element under meet. EXPIRED is a temporal state that transitions to NON_COMPLIANT after a grace period."),

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
    spacer(),

    // --- 10.5 Cross-Border Compliance Verification ---
    h2("10.5 Cross-Border Compliance Verification"),
    definition(
      "Definition 10.5 (Cross-Border Predicate).",
      "For transfer from jurisdiction J1 to J2: verify export requirements from source, verify import requirements at destination, compute combined compliance using pessimistic meet operation, and return true only if combined state is COMPLIANT or EXEMPT."
    ),
    p("Tensor slices are cached per jurisdiction pair with configurable TTL. Cache invalidation occurs on any tensor update affecting the relevant jurisdictions. For high-frequency corridors (e.g., PAK\u2194UAE), pre-computed slices are maintained in memory to avoid re-evaluation on every transaction."),
  ];
};
