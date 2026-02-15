const fs = require("fs");
const {
  Document, Packer, Paragraph, TextRun, Table, TableRow, TableCell,
  Header, Footer, AlignmentType, LevelFormat,
  TableOfContents, HeadingLevel, BorderStyle, WidthType, ShadingType,
  VerticalAlign, PageNumber, PageBreak, TabStopType, TabStopPosition,
  PositionalTab, PositionalTabAlignment, PositionalTabRelativeTo, PositionalTabLeader,
} = require("docx");

const { buildAllSections, heading1, heading2, heading3, p, bold, italic, code, makeTable, codeBlock, codeParagraph, spacer, pageBreak, definitionBlock, theoremBlock, BODY_FONT, CODE_FONT, DARK, ACCENT, LIGHT_GRAY, CONTENT_W, PAGE_W, PAGE_H, MARGIN, borders } = require("./generate-spec.js");

function buildPartsII_III() {
  const c = [];

  // ═══════════════════════════════════════════════════════════
  // PART II: CRYPTOGRAPHIC PRIMITIVES
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART II: CRYPTOGRAPHIC PRIMITIVES"));

  c.push(heading2("Chapter 3: Cryptographic Primitives"));
  c.push(p("The SEZ Stack relies on a carefully selected set of cryptographic primitives that balance security, performance, and zero-knowledge compatibility."));

  c.push(heading3("3.1 Hash Functions"));
  c.push(p([bold("SHA-256 for Content Addressing."), " SHA-256 provides collision-resistant hashing for content addressing. All artifacts, receipts, and credentials are identified by their SHA-256 digest. The choice of SHA-256 ensures broad compatibility with existing systems and hardware acceleration support."]));
  c.push(p([bold("Poseidon2 Hash Function."), " Poseidon2 is an arithmetic-friendly hash function designed for zero-knowledge proof systems. Unlike traditional hash functions, Poseidon2 operates natively over prime fields, dramatically reducing the constraint count when computing hashes inside ZK circuits. The Stack uses Poseidon2, an optimized variant achieving four times the performance of the original Poseidon construction. Operating over the BabyBear prime field (p = 2^31 - 2^27 + 1), Poseidon2 enables efficient proving on both CPUs and GPUs. All internal commitments, Merkle trees, and state digests in the Stack use Poseidon2. Approximately 160 constraints per hash in R1CS."]));
  c.push(...codeBlock([
    "use ark_bn254::Fr;",
    "use msez_core::hash::Poseidon2;",
    "",
    "/// Poseidon2 hash over BN254 scalar field.",
    "/// Width = 3 (2 inputs + 1 capacity), 8 full + 56 partial rounds.",
    "pub struct Poseidon2Hasher {",
    "    state: [Fr; 3],",
    "    round_constants: Vec<Fr>,",
    "    mds_matrix: [[Fr; 3]; 3],",
    "}",
    "",
    "impl Poseidon2Hasher {",
    "    pub fn hash(inputs: &[Fr]) -> Fr {",
    "        let mut hasher = Self::new();",
    "        for chunk in inputs.chunks(2) {",
    "            hasher.absorb(chunk);",
    "            hasher.permute();",
    "        }",
    "        hasher.squeeze()",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("3.2 The Canonical Digest Bridge"));
  c.push(p("The Canonical Digest Bridge solves a fundamental interoperability problem: how to maintain consistent cryptographic commitments across different proof systems and encoding formats."));
  c.push(...definitionBlock("Definition 3.1 (Canonical Digest Bridge).", "For any artifact A, the canonical digest is computed as: CDB(A) = Poseidon2(Split256(SHA256(JCS(A)))), where JCS(A) is the JSON Canonicalization Scheme serialization, SHA256 produces a 256-bit digest, Split256 converts to eight 32-bit BabyBear field elements, and Poseidon2 compresses to a single field element."));
  c.push(p("The bridge provides stability (any valid JSON serialization produces identical digest), composability (field-element output integrates directly into ZK circuits), and verifiability (external systems verify using standard SHA-256; ZK circuits verify the bridge mapping)."));
  c.push(...codeBlock([
    "/// Digest types supported by the bridge.",
    "#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]",
    "pub enum DigestType {",
    "    Poseidon2Bn254,",
    "    Sha256,",
    "    Blake3,",
    "}",
    "",
    "/// A content-addressed digest with type tag.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Digest {",
    "    pub digest_type: DigestType,",
    "    pub bytes: [u8; 32],",
    "}",
    "",
    "/// Bridge trait: canonical serialization -> digest.",
    "pub trait CanonicalDigest {",
    "    fn canonical_bytes(&self) -> Vec<u8>;",
    "    fn digest(&self, dt: DigestType) -> Digest {",
    "        let bytes = self.canonical_bytes();",
    "        match dt {",
    "            DigestType::Poseidon2Bn254 => Poseidon2Hasher::digest_bytes(&bytes),",
    "            DigestType::Sha256 => sha256_digest(&bytes),",
    "            DigestType::Blake3 => blake3_digest(&bytes),",
    "        }",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("3.3 Commitment Schemes"));
  c.push(p([bold("Pedersen Commitments."), " Pedersen commitments over the Jubjub curve provide hiding and binding for all value commitments. For value v and randomness r: Commit(v, r) = vG + rH, where G and H are independent generators. Pedersen commitments support homomorphic addition, enabling balance proofs without revealing individual amounts."]));
  c.push(...codeBlock([
    "use ark_ed_on_bn254::{EdwardsProjective as Jubjub, Fr as JubjubScalar};",
    "",
    "pub struct PedersenCommitment {",
    "    pub commitment: Jubjub,",
    "}",
    "",
    "impl PedersenCommitment {",
    "    pub fn commit(value: &JubjubScalar, blinding: &JubjubScalar) -> Self {",
    "        let g = Jubjub::generator();",
    "        let h = hash_to_curve(b\"pedersen_h\");",
    "        Self { commitment: g.mul(value) + h.mul(blinding) }",
    "    }",
    "}",
  ]));
  c.push(p([bold("KZG Polynomial Commitments."), " KZG commitments provide constant-size polynomial commitments with efficient verification. KZG enables efficient batch verification and proof aggregation, critical for scalable settlement."]));

  c.push(heading3("3.4 Nullifier System"));
  c.push(p("The nullifier system prevents double-spending while preserving transaction privacy. Each spendable record has an associated nullifier computable only by the owner."));
  c.push(...definitionBlock("Definition 3.2 (Nullifier Derivation).", "For record R with secret key sk: Nullifier(R, sk) = Poseidon2(R.commitment, sk, \"nullifier\"). Properties: uniqueness (each record produces exactly one nullifier), unforgeability (only the owner can compute the nullifier), unlinkability (observers cannot link nullifiers to records)."));

  c.push(heading3("3.5 BBS+ Signatures for Credentials"));
  c.push(p("BBS+ signatures provide the cryptographic foundation for selective disclosure in Verifiable Credentials. A BBS+ signature over a message vector allows the holder to derive presentation proofs revealing arbitrary message subsets while hiding others, without interaction with the issuer."));
  c.push(...codeBlock([
    "/// BBS+ keypair for credential issuance.",
    "pub struct BbsKeyPair {",
    "    pub public: BbsPublicKey,",
    "    pub secret: BbsSecretKey,",
    "}",
    "",
    "/// A BBS+ signed credential with selective disclosure.",
    "pub struct BbsCredential {",
    "    pub attributes: Vec<Fr>,",
    "    pub signature: BbsSignature,",
    "    pub issuer_pk: BbsPublicKey,",
    "}",
    "",
    "impl BbsCredential {",
    "    /// Produce a proof revealing only the specified attribute indices.",
    "    pub fn selective_disclose(",
    "        &self,",
    "        revealed_indices: &[usize],",
    "        nonce: &[u8],",
    "    ) -> Result<BbsProof, CryptoError> {",
    "        bbs_prove(self, revealed_indices, nonce)",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("3.6 Zero-Knowledge Proof Systems"));
  c.push(p("The Stack supports five Non-Interactive Zero-Knowledge proof systems organized in a hierarchy balancing different tradeoffs:"));
  c.push(makeTable(
    ["System", "Proof Size", "Setup", "Best For", "Crate"],
    [
      ["Groth16", "~288 bytes", "Trusted (per-circuit)", "Final wrapping, on-chain verification", "arkworks-groth16"],
      ["PLONK", "~400 bytes", "Universal", "Frequent updates, custom logic", "halo2"],
      ["Plonky3 STARK", "50-100 KB", "Transparent", "Native proving, post-quantum", "plonky3"],
      ["Bulletproofs", "Logarithmic", "Transparent", "Range proofs, inner products", "bulletproofs"],
      ["Halo2", "Variable", "Transparent", "Recursive composition", "halo2"],
    ],
    [1600, 1400, 1800, 2800, 1760]
  ));
  c.push(spacer());
  c.push(p("Twelve circuit types are implemented:"));
  c.push(makeTable(
    ["Circuit", "Constraints", "Purpose"],
    [
      ["\u03C0priv", "~34,000", "Privacy proof for shielded transfers"],
      ["\u03C0comp", "~25,000", "Compliance proof with tensor evaluation"],
      ["\u03C0asset", "~15,000", "Asset state transition proof"],
      ["\u03C0exec", "Variable", "Block execution proof"],
      ["\u03C0agg", "Variable", "Proof aggregation"],
      ["\u03C0ruling", "~35,000", "Arbitration ruling verification"],
      ["\u03C0sanctions", "~18,000", "Sanctions list non-membership (PPOI)"],
      ["\u03C0license", "~12,000", "License validity proof"],
      ["\u03C0migration", "~45,000", "Migration state transition"],
      ["\u03C0checkpoint", "~20,000", "Checkpoint validity"],
      ["\u03C0credential", "~8,000", "Credential selective disclosure"],
      ["\u03C0bridge", "~40,000", "Cross-corridor bridge proof"],
    ],
    [2000, 2000, 5360]
  ));
  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART III: CONTENT-ADDRESSED ARTIFACT MODEL
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART III: CONTENT-ADDRESSED ARTIFACT MODEL"));
  c.push(heading2("Chapter 4: Artifact Architecture"));

  c.push(heading3("4.1 Digest Type"));
  c.push(p("Every artifact in the MSEZ Stack is content-addressed. The artifact identifier is its cryptographic digest. This provides integrity (modification changes the identifier), deduplication (identical content shares an identifier), and auditability (any party can verify artifact integrity)."));
  c.push(...definitionBlock("Definition 4.1 (Artifact Reference).", "An artifact reference contains artifact_type (indicating interpretation), digest_sha256 (canonical identifier), and uri_hints (suggestions for retrieval). The digest provides the canonical identifier; uri_hints do not affect identity."));
  c.push(p([bold("Stability Invariant."), " For all valid JSON serializations j1, j2 of the same logical object A: Digest(j1) = Digest(j2). This is guaranteed by JCS canonicalization."]));
  c.push(...codeBlock([
    "/// Every artifact is content-addressed via its canonical digest.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Artifact {",
    "    pub artifact_type: ArtifactType,",
    "    pub digest: Digest,",
    "    pub payload: Vec<u8>,",
    "    pub metadata: ArtifactMetadata,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum ArtifactType {",
    "    Lawpack, Regpack, Licensepack, Schema,",
    "    VerifiableCredential, Receipt, Checkpoint,",
    "    ProofKey, TransitionType, Blob,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("4.2 Artifact Type Registry"));
  c.push(makeTable(
    ["Type", "MIME Type", "Description"],
    [
      ["lawpack", "application/vnd.momentum.lawpack+zip", "Legal corpus with Akoma Ntoso documents"],
      ["regpack", "application/vnd.momentum.regpack+zip", "Dynamic regulatory state container"],
      ["licensepack", "application/vnd.momentum.licensepack+zip", "Live license registry snapshot"],
      ["genesis", "application/vnd.momentum.genesis+json", "Smart Asset genesis document"],
      ["receipt", "application/vnd.momentum.receipt+json", "State transition receipt"],
      ["checkpoint", "application/vnd.momentum.checkpoint+json", "MMR checkpoint"],
      ["vc", "application/vc+json", "W3C Verifiable Credential"],
      ["proof", "application/vnd.momentum.proof+bin", "ZK proof bytes"],
      ["corridor-def", "application/vnd.momentum.corridor+json", "Corridor definition VC"],
      ["tensor", "application/vnd.momentum.tensor+json", "Compliance tensor definition"],
      ["attestation", "application/vnd.momentum.attestation+json", "Compliance attestation"],
    ],
    [1800, 4200, 3360]
  ));
  c.push(spacer());

  c.push(heading3("4.3 Artifact Closure and Availability"));
  c.push(...definitionBlock("Definition 4.3 (Artifact Closure).", "The transitive closure of artifact A is the set of all artifacts reachable by following references: Closure(A) = {A} \u222A \u222A{Closure(resolve(r)) : r \u2208 refs(A)}."));
  c.push(p([bold("Axiom 4.1 (Availability Enforcement)."), " A proof is valid only if all artifacts in its artifact_bundle_root are retrievable by authorized parties. Enforcement levels: Best-Effort (S13 off) where provers commit to bundle root and auditors fetch out-of-band, and Enforced (S13 on) where DA committees verify retrievability before block acceptance."]));
  c.push(p("The CLI provides artifact graph operations:"));
  c.push(...codeBlock([
    "# Verify artifact graph closure with strict digest recomputation",
    "msez artifact graph verify transition-types <digest> --strict --json",
    "",
    "# Generate witness bundle for offline transfer",
    "msez artifact graph verify transition-types <digest> --bundle /tmp/witness.zip",
    "",
    "# Attest (sign) a witness bundle for provenance",
    "msez artifact bundle attest /tmp/witness.zip \\",
    "  --issuer did:example:watcher \\",
    "  --sign --key keys/dev.ed25519.jwk \\",
    "  --out /tmp/witness.attestation.vc.json",
  ]));
  c.push(pageBreak());

  return c;
}

function buildPartIV() {
  const c = [];
  // ═══════════════════════════════════════════════════════════
  // PART IV: PACK TRILOGY, MODULES, AND PROFILES
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART IV: CORE COMPONENTS \u2014 MODULES, PACK TRILOGY, PROFILES"));

  c.push(heading2("Chapter 5: Module Specifications"));
  c.push(p("Modules are the unit of composition in the MSEZ Stack. Each module provides a discrete governance capability. Modules declare dependencies, expose interfaces, and can be composed into profiles."));

  c.push(heading3("5.1 Corridors Module"));
  c.push(p("The Corridors module manages economic relationships between jurisdictions. A corridor represents a bilateral or multilateral agreement enabling coordinated economic activity with cryptographic compliance guarantees. Corridor establishment follows Protocol 14.1 for cross-jurisdiction transfer setup. Each party publishes their policy requirements as a machine-readable specification. The corridor negotiation process identifies compatible policy overlaps and generates a corridor manifest encoding the agreed terms."));
  c.push(p("Corridor state synchronization maintains consistent views across participants. The sync protocol uses vector clocks for causality tracking and Merkle proofs for efficient delta synchronization. Compliance verification operates through the Compliance Tensor V2 model. Cross-border operations verify all applicable predicates through ZK proofs."));
  c.push(makeTable(
    ["Component", "Version", "Description"],
    [
      ["corridor-state-api", "3.2.1", "OpenAPI specification for corridor state management"],
      ["corridor-manifest-schema", "2.1.0", "JSON Schema for corridor manifests"],
      ["sync-protocol", "1.4.0", "State synchronization protocol specification"],
      ["compliance-tensor", "2.1.0", "Compliance Tensor V2 data structures"],
      ["corridor-bridge", "1.0.0", "Cross-corridor bridge protocol"],
    ],
    [2800, 1200, 5360]
  ));
  c.push(spacer());

  c.push(heading3("5.2 Governance Module"));
  c.push(p("The Governance module implements institutional decision-making processes including constitutional frameworks, voting mechanisms, amendment procedures, and stakeholder coordination. Constitutional frameworks define the fundamental rules governing zone operations. The Stack supports hierarchical constitutions with multiple amendment thresholds. Core provisions may require supermajority approval or external ratification, while operational policies may be modifiable through administrative action. Voting mechanisms support multiple models including token-weighted voting, one-entity-one-vote, quadratic voting, and conviction voting."));

  c.push(heading3("5.3 Financial Module"));
  c.push(p("The Financial module provides banking and payment infrastructure: account management, payment processing, foreign exchange, custody services, and capital markets integration. Account management supports both fiat and digital asset accounts. Fiat accounts integrate with traditional banking rails through the Mass Treasury API. Custody services provide institutional-grade asset protection with multi-signature wallets, configurable quorum policies, time-locked releases, and automated compliance holds."));

  c.push(heading3("5.4 Regulatory Module"));
  c.push(p("The Regulatory module implements compliance frameworks required for lawful economic activity: identity verification, transaction monitoring, sanctions screening, and regulatory reporting. Identity verification follows zkKYC principles. Transaction monitoring operates through configurable rule engines evaluated against jurisdiction-specific rules."));

  c.push(heading3("5.5 Licensing Module"));
  c.push(p("The Licensing module manages business authorization: license application processing, compliance monitoring, renewal management, and portability across compatible jurisdictions. License portability enables mutual recognition across compatible jurisdictions through credential verification."));

  c.push(heading3("5.6 Legal Module"));
  c.push(p("The Legal module provides infrastructure for contract management, dispute resolution, and enforcement. Enforcement mechanisms translate legal determinations into system actions. Arbitration rulings encoded as Verifiable Credentials trigger automatic state transitions in affected Smart Assets."));

  c.push(heading3("5.7 Operational Module"));
  c.push(p("The Operational module provides administrative functionality for zone management: human resources, procurement, facility management, and general administration."));

  // Chapter 6: Pack Trilogy
  c.push(pageBreak());
  c.push(heading2("Chapter 6: The Pack Trilogy"));
  c.push(p("The Pack Trilogy \u2014 lawpacks, regpacks, and licensepacks \u2014 provides comprehensive, cryptographically verifiable snapshots of jurisdictional state across all temporal frequencies:"));
  c.push(makeTable(
    ["Pack Type", "Content", "Change Frequency"],
    [
      ["Lawpack", "Statutes, regulations (Akoma Ntoso XML)", "Months/Years"],
      ["Regpack", "Sanctions, calendars, guidance, SROs", "Days/Weeks"],
      ["Licensepack", "Live license registries", "Hours/Days"],
    ],
    [2000, 4200, 3160]
  ));
  c.push(spacer());

  c.push(heading3("6.1 Lawpack System"));
  c.push(p("Lawpacks encode jurisdiction-specific legal and regulatory requirements in machine-readable format. A lawpack consists of five components: regulatory manifest, rule definitions, evidence requirements, attestation schema, and tensor definitions. The regulatory manifest identifies the lawpack and its scope. Rule definitions encode specific requirements as evaluatable predicates. Evidence requirements specify documentation needed to demonstrate compliance. Attestation schema defines the structure of compliance attestations. Tensor definitions specify the compliance tensor structure for the jurisdiction."));
  c.push(p([bold("Pakistan Example."), " The Pakistan GovOS deployment encodes the following primary legislation:"]));
  c.push(makeTable(
    ["Act", "Akoma Ntoso ID", "Key Provisions"],
    [
      ["Income Tax Ordinance 2001", "pk-ito-2001", "Income classification, withholding schedules, tax credits, NTN requirements"],
      ["Sales Tax Act 1990", "pk-sta-1990", "GST rates, input/output tax, exempt supplies, e-invoicing requirements"],
      ["Federal Excise Act 2005", "pk-fea-2005", "Excise duties, manufacturing levies, excisable services"],
      ["Customs Act 1969", "pk-ca-1969", "Import/export duties, tariff schedules, bonded warehouses, CPEC preferences"],
      ["Companies Act 2017", "pk-ca-2017", "Entity formation, director duties, beneficial ownership, SECP registration"],
    ],
    [2600, 1800, 4960]
  ));
  c.push(spacer());
  c.push(...codeBlock([
    "/// A lawpack: content-addressed bundle of legislation in Akoma Ntoso.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Lawpack {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub version: SemanticVersion,",
    "    pub as_of_date: chrono::NaiveDate,",
    "    pub acts: Vec<AkomaAct>,",
    "    pub digest: Digest,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct AkomaAct {",
    "    pub akn_id: String,",
    "    pub title: String,",
    "    pub body_xml: String,  // Akoma Ntoso XML",
    "    pub provisions: Vec<Provision>,",
    "    pub effective_date: chrono::NaiveDate,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("6.2 Lawpack Composition"));
  c.push(p("Lawpacks compose hierarchically through import and extension mechanisms. A jurisdiction may import rules from international standards, regional agreements, or template packs, then extend with local modifications. Import semantics bring external rules into scope with optional namespace prefixing. Extension semantics enable modification of inherited rules. Local rules may strengthen, weaken, or replace requirements entirely."));

  c.push(heading3("6.3 Lawpack Attestation and Binding"));
  c.push(p("Lawpacks become operative through attestation and binding. Attestation confirms the lawpack accurately represents legal requirements. Binding associates the lawpack with specific system components including corridors, assets, or entities. The attestation process produces a Verifiable Credential signed by the issuing authority."));

  c.push(heading3("6.4 RegPack System"));
  c.push(p("The RegPack system provides dynamic regulatory state management, enabling real-time policy updates without system downtime."));
  c.push(p([bold("Pakistan Example."), " The FBR regpack includes:"]));
  c.push(makeTable(
    ["Component", "Update Frequency", "Content"],
    [
      ["WHT Rate Tables", "Per SRO (days)", "Withholding rates by income category, payee type, and NTN status"],
      ["Filing Calendar", "Quarterly", "Monthly/quarterly/annual return deadlines for income tax, sales tax, FED"],
      ["SRO Registry", "As issued", "Statutory Regulatory Orders modifying tax rates, exemptions, procedures"],
      ["FATF AML/CFT", "FATF plenary cycle", "Customer due diligence tiers, STR thresholds, PEP definitions"],
      ["OFAC/EU/UN Sanctions", "Daily sync", "Designated persons lists, entity matches, fuzzy matching thresholds"],
    ],
    [2400, 2000, 4960]
  ));
  c.push(spacer());
  c.push(...codeBlock([
    "/// A regpack: machine-readable regulatory state.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Regpack {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub version: SemanticVersion,",
    "    pub effective_from: chrono::NaiveDate,",
    "    pub tax_calendars: Vec<TaxCalendar>,",
    "    pub withholding_tables: Vec<WithholdingTable>,",
    "    pub sanctions_lists: Vec<SanctionsList>,",
    "    pub aml_cft_rules: AmlCftRules,",
    "    pub sro_registry: Vec<StatutoryRegulatoryOrder>,",
    "    pub digest: Digest,",
    "}",
  ]));
  c.push(p("RegPack digests provide cryptographic commitments to regulatory state at specific times. Corridor bindings include RegPack digests to establish the regulatory context. The \u03C0sanctions ZK circuit enables privacy-preserving sanctions verification with approximately 18,000 constraints."));

  c.push(heading3("6.5 Licensepack System (v0.4.44)"));
  c.push(p("Licensepacks complete the Pack Trilogy, providing cryptographically verifiable snapshots of jurisdictional licensing state. Licensing state is critical for corridor operations and compliance verification. Licensepacks enable offline license verification, audit trails proving licensing state at any historical point, cross-zone settlement with counterparty authorization verification, and LICENSING domain population for compliance tensors."));
  c.push(p([bold("Pakistan Example."), " Fifteen-plus license categories across regulatory authorities:"]));
  c.push(makeTable(
    ["Authority", "License Categories", "Key Requirements"],
    [
      ["SECP", "Company registration, NTN issuance", "Memorandum/Articles, director KYC, registered office"],
      ["BOI", "Industrial licenses, SEZ registrations", "Investment thresholds, sector restrictions, incentive eligibility"],
      ["PTA", "Telecom licenses, spectrum allocation", "Technical standards, coverage obligations, fee schedules"],
      ["PEMRA", "Media licenses, broadcasting permits", "Content standards, ownership limits, renewal cycles"],
      ["DRAP", "Drug/device manufacturing, import permits", "GMP compliance, clinical trial data, product registration"],
      ["Provincial", "Trade licenses, professional permits", "Varies by province: Punjab, Sindh, KP, Balochistan"],
    ],
    [1600, 3200, 4560]
  ));
  c.push(spacer());
  c.push(...codeBlock([
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Licensepack {",
    "    pub jurisdiction: JurisdictionId,",
    "    pub authority: RegulatoryAuthority,",
    "    pub license_types: Vec<LicenseType>,",
    "    pub version: SemanticVersion,",
    "    pub digest: Digest,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct LicenseType {",
    "    pub id: String,",
    "    pub name: String,",
    "    pub permitted_activities: Vec<ActivityCode>,",
    "    pub requirements: LicenseRequirements,",
    "    pub fees: FeeSchedule,",
    "    pub renewal: RenewalSchedule,",
    "    pub compliance_obligations: Vec<ComplianceObligation>,",
    "}",
  ]));

  c.push(heading3("6.5.1 License Data Model"));
  c.push(p("Six status values track license lifecycle: ACTIVE (license in good standing), SUSPENDED (temporarily non-operative), REVOKED (permanently terminated), EXPIRED (validity period elapsed), PENDING (application under review), and SURRENDERED (voluntarily relinquished). Six domains categorize license types: FINANCIAL, CORPORATE, PROFESSIONAL, TRADE, INSURANCE, and MIXED."));

  c.push(heading3("6.5.2 License Verification"));
  c.push(p("The licensepack verify_license method performs full authorization verification: license existence and ACTIVE status, activity within permitted scope, amount within authorized limits, currency within permitted instruments, no active restrictions blocking the operation, and all conditions satisfied."));

  c.push(heading3("6.5.3 Compliance Tensor Integration"));
  c.push(p("Licensepacks populate the LICENSING compliance domain in the Compliance Tensor V2:"));
  c.push(makeTable(
    ["License Status", "Tensor State", "Effect"],
    [
      ["ACTIVE", "COMPLIANT", "Operations permitted"],
      ["SUSPENDED", "SUSPENDED", "Operations blocked temporarily"],
      ["REVOKED/EXPIRED", "NON_COMPLIANT", "Operations blocked"],
      ["PENDING", "PENDING", "Limited operations"],
      ["No license required", "EXEMPT", "De minimis exemption"],
    ],
    [2400, 2400, 4560]
  ));
  c.push(spacer());

  c.push(heading3("6.5.4 Licensepack Schemas"));
  c.push(p("Three JSON schemas define licensepack structure: licensepack.schema.json (main structure and metadata), licensepack.license.schema.json (individual license records with conditions), and licensepack.lock.schema.json (version pinning)."));

  c.push(heading3("6.5.5 Zone Integration"));
  c.push(p("Zones specify licensepack requirements in zone.yaml, including refresh policies per domain. Financial domain licensepacks refresh hourly with maximum 4-hour staleness. Default domain licensepacks refresh daily with 24-hour maximum staleness."));

  // Chapter 7: Profile System
  c.push(heading2("Chapter 7: Profile System"));
  c.push(p("Profiles are curated bundles of modules, parameters, and jurisdiction-specific configuration. They serve as deployment templates:"));
  c.push(makeTable(
    ["Profile", "Use Case", "Key Modules"],
    [
      ["digital-financial-center", "Full-service financial zone (ADGM model)", "All 16 families, full corridor suite, capital markets"],
      ["trade-hub", "Trade and logistics zone", "Corporate, trade, financial, corridors, customs"],
      ["tech-park", "Technology and innovation zone", "Corporate, licensing, IP, identity, light financial"],
      ["sovereign-govos", "National government deployment (Pakistan model)", "All families + GovOS orchestration + national system integration"],
      ["charter-city", "Large-scale developments", "Full civic services, land management"],
      ["digital-native-free-zone", "Technology-focused zones", "Rapid formation, IP protection"],
      ["asset-history-bundle", "Asset provenance", "Enhanced receipt chains, certification"],
    ],
    [2600, 3200, 3560]
  ));
  c.push(pageBreak());

  return c;
}

function buildPartV() {
  const c = [];
  // ═══════════════════════════════════════════════════════════
  // PART V: SMART ASSET EXECUTION LAYER
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART V: SMART ASSET EXECUTION LAYER"));

  // Chapter 8: Smart Asset Model
  c.push(heading2("Chapter 8: Smart Asset Model"));

  c.push(heading3("8.1 Smart Asset Definition"));
  c.push(...definitionBlock("Definition 8.1 (Smart Asset).", "A Smart Asset is formally defined as a five-tuple (G, R, M, C, H) where G is the Genesis Record, R is the Registry Binding, M is the Manifest, C is the Receipt Chain, and H is the State Machine specification."));
  c.push(p([bold("Genesis Record (G)."), " Establishes immutable asset identity. It contains the asset type classification, initial ownership, creation timestamp, and genesis proof. Once created, the Genesis Record cannot be modified; asset identity remains stable across all subsequent operations. The canonical asset identifier is the SHA256 hash of the genesis document."]));
  c.push(p([bold("Registry Binding (R)."), " Associates the asset with one or more registries. Registries provide discovery, query, and notification services for registered assets. An asset may bind to multiple registries in different jurisdictions for cross-border visibility."]));
  c.push(p([bold("Manifest (M)."), " Declares asset properties and capabilities: human-readable description, machine-readable attributes, applicable lawpacks, and authorized operations. The Manifest evolves over the asset lifecycle through specified update procedures."]));
  c.push(p([bold("Receipt Chain (C)."), " Maintains the cryptographic history of asset state transitions. Each transition produces a receipt cryptographically linked to its predecessor, forming an immutable audit trail. The Receipt Chain enables verification of current state through traversal from genesis."]));
  c.push(p([bold("State Machine (H)."), " Specification defines valid transitions and their effects: transition types, guard conditions, state transformations, and side effects."]));

  c.push(...codeBlock([
    "/// The five-tuple defining a Smart Asset.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct SmartAsset {",
    "    pub genesis: GenesisRecord,",
    "    pub registry_binding: RegistryBinding,",
    "    pub manifest: AssetManifest,",
    "    pub receipt_chain: ReceiptChain,",
    "    pub state_machine: StateMachineSpec,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("8.2 Design Invariants"));
  c.push(p("Five design invariants govern Smart Asset behavior. These hold regardless of implementation choices and form the foundation for security proofs:"));
  c.push(makeTable(
    ["ID", "Name", "Statement"],
    [
      ["I1", "Immutable Identity", "Asset identity is established at genesis and cannot change"],
      ["I2", "Deterministic State", "Current state is uniquely determined by the receipt chain"],
      ["I3", "Explicit Bindings", "All cross-asset relationships are explicitly declared"],
      ["I4", "Resolvability", "Any asset reference can be resolved to current state"],
      ["I5", "Optional Anchoring", "Assets may operate without blockchain anchoring"],
    ],
    [800, 2200, 6360]
  ));
  c.push(spacer());

  c.push(heading3("8.3 Asset Lifecycle"));
  c.push(p("Smart Assets progress through five lifecycle phases: Creation (genesis through initial activation), Active (normal operation with state transitions, ownership transfers, and compliance updates), Suspended (temporarily halts operations due to compliance issues, dispute filings, or administrative holds), Terminal (permanent cessation of activity), and Archived (long-term record retention)."));

  c.push(heading3("8.4 Smart Assets as Autonomous Agents"));
  c.push(p("Smart Assets exhibit agentic behavior: they autonomously respond to environmental changes without requiring explicit user requests."));
  c.push(...definitionBlock("Definition 8.2 (Agentic Transition).", "An agentic transition is a state transition triggered by environmental events. Trigger types: regulatory triggers (SanctionsListUpdate, LicenseExpiration, GuidanceChange), arbitration triggers (RulingReceived, AppealDeadlinePassed, EnforcementDue), settlement triggers (CheckpointRequired, FinalizationAnchor), and asset lifecycle triggers (KeyRotationDue, AttestationExpiring)."));

  // Chapter 9
  c.push(heading2("Chapter 9: Receipt Chain Architecture"));
  c.push(p("Receipt chains provide the cryptographic backbone for Smart Asset state management."));

  c.push(heading3("9.1 Receipt Structure"));
  c.push(...codeBlock([
    "/// A receipt: a signed state transition in the asset's chain.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Receipt {",
    "    pub asset_id: AssetId,",
    "    pub sequence: u64,",
    "    pub prev_digest: Digest,",
    "    pub transition: Transition,",
    "    pub state_commitment: Digest,",
    "    pub tensor_commitment: Digest,",
    "    pub timestamp: chrono::DateTime<chrono::Utc>,",
    "    pub signatures: Vec<Ed25519Signature>,",
    "    pub watcher_attestations: Vec<WatcherAttestation>,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("9.2 MMR Checkpoints"));
  c.push(...definitionBlock("Definition 9.1 (MMR Checkpoint).", "An MMR checkpoint contains: asset_id and checkpoint_seq for identity; receipt_range indicating the covered receipts; mmr_root and state_commitment for cryptographic binding; watcher_attestations for validation; and optional l1_anchor for settlement layer integration."));
  c.push(p("Merkle Mountain Range checkpoints provide efficient verification of long receipt chains. Rather than verifying every receipt from genesis, a checkpoint enables verification from a trusted intermediate state. The Agentic Framework includes the asset.checkpoint_due trigger for automated checkpoint creation."));

  c.push(heading3("9.3 Fork Resolution"));
  c.push(p("Forks occur when conflicting receipts extend the same chain position. Fork detection occurs through receipt hash comparison. Resolution priority follows timestamp ordering. The earlier-timestamped branch is presumptively valid. This presumption can be overridden by cryptographic evidence of invalidity (signature failure or predicate violation)."));
  c.push(theoremBlock("Theorem 9.1 (Object Survivability).", "A Smart Asset with a valid receipt chain maintains full operational capability without connectivity to any external system, including the MASS L1 settlement layer. Proof: The receipt chain provides total ordering, the Compliance Tensor carries compliance state, and the state machine specification enables deterministic execution. No external oracle is required for continued operation."));

  // Chapter 10
  c.push(pageBreak());
  c.push(heading2("Chapter 10: Compliance Tensor V2"));

  c.push(heading3("10.1 Mathematical Definition"));
  c.push(...definitionBlock("Definition 10.1 (Compliance Tensor).", "The compliance tensor C is a function: C: AssetID x JurisdictionID x ComplianceDomain x TimeQuantum -> ComplianceState."));
  c.push(...codeBlock([
    "/// Compliance state for a single (jurisdiction, domain) pair.",
    "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
    "pub enum ComplianceState {",
    "    Unknown,       // No attestation",
    "    NonCompliant,  // Attested non-compliant",
    "    Pending,       // Evaluation in progress",
    "    Compliant,     // Attested compliant",
    "    Exempt,        // Jurisdiction exempts this domain",
    "}",
    "",
    "/// Partial order: Unknown < NonCompliant < Pending < Compliant, Exempt",
    "impl PartialOrd for ComplianceState {",
    "    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {",
    "        use ComplianceState::*;",
    "        match (self, other) {",
    "            (a, b) if a == b => Some(std::cmp::Ordering::Equal),",
    "            (Unknown, _) => Some(std::cmp::Ordering::Less),",
    "            (_, Unknown) => Some(std::cmp::Ordering::Greater),",
    "            (NonCompliant, Pending | Compliant | Exempt) => Some(std::cmp::Ordering::Less),",
    "            (Pending, Compliant | Exempt) => Some(std::cmp::Ordering::Less),",
    "            (Compliant, Exempt) | (Exempt, Compliant) => None,",
    "            _ => other.partial_cmp(self).map(|o| o.reverse()),",
    "        }",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("10.2 Compliance Domains"));
  c.push(p("Twenty compliance domains span the regulatory landscape:"));
  c.push(makeTable(
    ["Domain", "Description"],
    [
      ["CIVIC", "Municipal governance, residency, civic participation"],
      ["CORPORATE", "Entity formation, governance, fiduciary duties"],
      ["COMMERCIAL", "Contracts, sales, commercial transactions"],
      ["FINANCIAL", "Financial services regulation, prudential requirements"],
      ["SECURITIES", "Securities issuance, trading, disclosure"],
      ["BANKING", "Banking licenses, capital requirements, supervision"],
      ["PAYMENTS", "Payment services, e-money, remittances"],
      ["DIGITAL_ASSETS", "Cryptocurrency, tokenization, DeFi regulation"],
      ["TAX", "Tax regime, withholding, reporting"],
      ["AML_CFT", "Anti-money laundering, counter-terrorism financing"],
      ["DATA_PROTECTION", "Privacy, data localization, consent"],
      ["ARBITRATION", "Dispute resolution, enforcement, recognition"],
      ["LICENSING", "Business licensing, permits, authorizations"],
    ],
    [2400, 6960]
  ));
  c.push(spacer());

  c.push(heading3("10.3 Compliance States"));
  c.push(p("Compliance states follow a strict lattice: NON_COMPLIANT < EXPIRED < UNKNOWN < PENDING < EXEMPT < COMPLIANT. Meet (\u2227): pessimistic composition returning the lower state. Join (\u2228): optimistic composition returning the higher state. NON_COMPLIANT is the absorbing element under meet. EXPIRED is a temporal state that transitions to NON_COMPLIANT after a grace period."));

  c.push(heading3("10.4 Tensor Operations"));
  c.push(...definitionBlock("Definition 10.2 (Tensor Slice).", "A slice fixes one or more dimensions. slice_aj = tensor[A, J, :, :] retrieves all compliance states for asset A in jurisdiction J. Slice operations enable efficient cross-border verification without full tensor materialization."));
  c.push(...definitionBlock("Definition 10.3 (Tensor Update).", "Incremental update from attestation integrates new attestation references into tensor cells, updates compliance state, and recomputes aggregate values."));
  c.push(...definitionBlock("Definition 10.4 (Tensor Commitment).", "The tensor commitment enables ZK proofs by building a Merkle tree over all cells using Poseidon2 hashes. The commitment includes the root, jurisdiction count, domain count, and last update timestamp."));
  c.push(...codeBlock([
    "/// The Compliance Tensor: jurisdiction x domain -> state.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct ComplianceTensor {",
    "    pub asset_id: AssetId,",
    "    pub entries: BTreeMap<(JurisdictionId, ComplianceDomain), TensorEntry>,",
    "    pub commitment: Digest,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct TensorEntry {",
    "    pub state: ComplianceState,",
    "    pub attestation: Option<WatcherAttestation>,",
    "    pub expires: Option<chrono::DateTime<chrono::Utc>>,",
    "    pub evidence_digest: Option<Digest>,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("10.5 Cross-Border Compliance Verification"));
  c.push(...definitionBlock("Definition 10.5 (Cross-Border Predicate).", "For transfer from jurisdiction J1 to J2: verify export requirements from source, verify import requirements at destination, compute combined compliance using pessimistic meet operation, and return true only if combined state is COMPLIANT or EXEMPT."));
  c.push(p("Tensor caching dramatically improves verification performance. Common operation patterns hit cached tensor slices, avoiding repeated predicate lookup. Cache invalidation triggers when underlying lawpacks, regpacks, licensepacks, or corridors update."));

  // Chapter 11: SAVM
  c.push(pageBreak());
  c.push(heading2("Chapter 11: Smart Asset Virtual Machine"));

  c.push(heading3("11.1 Architecture"));
  c.push(p("The SAVM architecture comprises four main components: Stack (256 slots), Memory (64KB max), Storage (Merkleized), and World State. The Instruction Decoder handles Arithmetic, Stack, Memory, Storage, Control, and Compliance categories. The Compliance Coprocessor handles Tensor Operations, ZK Verification, Attestation Checking, and Migration FSM."));

  c.push(heading3("11.2 Instruction Categories"));
  c.push(makeTable(
    ["Range", "Category", "Instructions"],
    [
      ["0x00-0x0F", "Stack", "PUSH, POP, DUP, SWAP"],
      ["0x10-0x1F", "Arithmetic", "ADD, SUB, MUL, DIV, MOD"],
      ["0x20-0x2F", "Comparison", "EQ, LT, GT, AND, OR, NOT"],
      ["0x30-0x3F", "Memory", "MLOAD, MSTORE, MSIZE"],
      ["0x40-0x4F", "Storage", "SLOAD, SSTORE, SDELETE"],
      ["0x50-0x5F", "Control Flow", "JUMP, JUMPI, CALL, RETURN, REVERT"],
      ["0x60-0x6F", "Context", "CALLER, ORIGIN, JURISDICTION, TIMESTAMP"],
      ["0x70-0x7F", "Compliance", "TENSOR_GET, TENSOR_SET, ATTEST, VERIFY_ZK"],
      ["0x80-0x8F", "Migration", "LOCK, UNLOCK, TRANSIT, SETTLE"],
      ["0x90-0x9F", "Crypto", "HASH, VERIFY_SIG, MERKLE_VERIFY"],
      ["0xF0-0xFF", "System", "HALT, LOG, DEBUG"],
    ],
    [1800, 2000, 5560]
  ));
  c.push(spacer());

  c.push(heading3("11.3 Compliance Coprocessor"));
  c.push(p([bold("Tensor Operations."), " TENSOR_GET(jurisdiction, domain) retrieves compliance state. TENSOR_SET(jurisdiction, domain, state, attestation) updates with attestation. TENSOR_SLICE(dimension, value) extracts tensor slice. TENSOR_COMMIT() computes tensor commitment."]));
  c.push(p([bold("ZK Verification."), " VERIFY_PROOF(proof_type, public_inputs, proof) verifies ZK proofs. VERIFY_CREDENTIAL(vc, disclosure_set) verifies BBS+ credentials with selective disclosure."]));
  c.push(p([bold("Migration Support."), " MIGRATION_STATE() returns current migration state. MIGRATION_ADVANCE(target_state, proof) advances migration state machine with validation."]));
  c.push(...codeBlock([
    "/// SAVM execution context.",
    "pub struct SavmContext {",
    "    pub stack: Vec<Fr>,",
    "    pub memory: Vec<u8>,",
    "    pub storage: MerkleStorage,",
    "    pub gas_remaining: u64,",
    "    pub compliance_coprocessor: ComplianceCoprocessor,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("11.4 Gas Metering"));
  c.push(makeTable(
    ["Category", "Base Gas", "Notes"],
    [
      ["Stack", "1-3", "O(1) operations"],
      ["Arithmetic", "3-10", "Higher for MUL/DIV/EXP"],
      ["Memory", "3 + expansion", "Linear in expansion size"],
      ["Storage", "100-20,000", "Higher for writes"],
      ["Compliance", "1,000-50,000", "Coprocessor operations"],
      ["Migration", "10,000-100,000", "State machine transitions"],
      ["Crypto", "3,000-100,000", "Signature/proof verification"],
    ],
    [2400, 2400, 4560]
  ));
  c.push(spacer());

  c.push(heading3("11.5 Execution Receipts"));
  c.push(p("Each VM execution produces an execution receipt containing: asset_id, program_hash, input_commitment, output_commitment, gas_used, state_delta, logs, and optional execution_proof. The execution proof enables trustless verification that the execution was performed correctly."));

  // Chapter 12: Composition Engine
  c.push(pageBreak());
  c.push(heading2("Chapter 12: Multi-Jurisdiction Composition Engine (v0.4.44)"));

  c.push(heading3("12.1 Design Thesis"));
  c.push(p("The composition engine implements a thesis central to the Momentum vision: deploy the civic code of New York with the corporate law of Delaware, but with the digital asset clearance, settlement and securities laws of ADGM with automated AI arbitration turned on. This capability addresses a fundamental limitation of traditional jurisdictional design. Existing special economic zones inherit complete legal frameworks from parent jurisdictions, accepting both strengths and weaknesses."));

  c.push(heading3("12.2 Domain Enumeration"));
  c.push(p("Twenty compliance domains span the complete regulatory landscape: CIVIC, CORPORATE, COMMERCIAL, FINANCIAL, SECURITIES, BANKING, PAYMENTS, CUSTODY, CLEARING, SETTLEMENT, DIGITAL_ASSETS, TAX, EMPLOYMENT, IMMIGRATION, IP, DATA_PROTECTION, AML_CFT, CONSUMER_PROTECTION, ARBITRATION, and LICENSING."));

  c.push(heading3("12.3 Composition Data Model"));
  c.push(...codeBlock([
    "/// A composed jurisdiction: domain-level sourcing from multiple lawpacks.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct ComposedJurisdiction {",
    "    pub id: JurisdictionId,",
    "    pub name: String,",
    "    pub layers: Vec<JurisdictionLayer>,",
    "    pub conflict_resolution: ConflictResolution,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct JurisdictionLayer {",
    "    pub source_jurisdiction: JurisdictionId,",
    "    pub domains: Vec<ComplianceDomain>,",
    "    pub lawpack_digest: Digest,",
    "    pub regpack_digest: Digest,",
    "    pub priority: u32,",
    "}",
  ]));
  c.push(spacer());
  c.push(p([bold("Kazakhstan Example."), " The Alatau City deployment composes Kazakh national law with AIFC financial regulation. Domain mapping: CORPORATE and CIVIC from Kazakh law; FINANCIAL, SECURITIES, BANKING, DIGITAL_ASSETS from AIFC framework; TAX from Kazakh law with AIFC incentive overlays."]));

  c.push(heading3("12.4 Composition Validation"));
  c.push(p("The composition engine validates proposed compositions against multiple constraints: domain coverage (every required domain must be covered by exactly one layer), compatibility rules (certain domain combinations require compatible lawpacks), and arbitration coherence (the arbitration configuration must be enforceable across all contributing jurisdictions)."));

  c.push(heading3("12.5 Composition Factory"));
  c.push(p("The compose_zone factory function provides a convenient interface for common compositions. It accepts domain-to-jurisdiction mappings and produces deployment artifacts."));

  c.push(heading3("12.6 Generated Artifacts"));
  c.push(p("A validated composition generates: zone.yaml (zone manifest capturing all composition decisions), stack.lock (precise version pinning for all lawpacks, regpacks, and licensepacks), and composition_digest (deterministic SHA256 digest enabling verification and caching)."));

  c.push(pageBreak());
  return c;
}

function buildPartsVI_XVIII() {
  const c = [];

  // ═══════════════════════════════════════════════════════════
  // PART VI: MASS L1 SETTLEMENT INFRASTRUCTURE
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART VI: MASS L1 SETTLEMENT INFRASTRUCTURE"));

  c.push(heading2("Chapter 13: ZK-Native Blockchain Architecture"));
  c.push(heading3("13.1 Design Targets"));
  c.push(makeTable(
    ["Target", "Specification", "Rationale"],
    [
      ["Throughput", "100K-10M+ TPS", "Support major financial center volumes"],
      ["Private TX Latency", "<200ms", "Real-time payment applications"],
      ["Consensus Latency", "<2s", "Cross-shard coordination"],
      ["Privacy", "Untraceable by default", "Commercial confidentiality"],
      ["Compliance", "ZK-proven predicates", "Regulatory satisfaction"],
      ["Post-Quantum", "STARK-native crypto", "Future-proof security"],
      ["Client Proving", "<10s mobile, <60s legacy", "Practical user experience"],
    ],
    [2400, 2800, 4160]
  ));
  c.push(spacer());

  c.push(heading3("13.2 State Model"));
  c.push(p("Mass uses an object-centric state model. State divides into private records (encrypted data objects owned by single parties, analogous to UTXOs) and public mappings (shared state requiring consensus ordering, including compliance registries, corridor state, and asset manifests). Hybrid transactions may consume private records while updating public mappings."));

  c.push(heading3("13.3 Consensus Mechanism"));
  c.push(p("Mass consensus uses a DAG-based protocol derived from Narwhal-Bullshark with Mysticeti optimizations. This architecture decouples data availability from consensus ordering."));
  c.push(p([bold("Jurisdictional DAG Consensus (JDC)."), " A directed acyclic graph where edges encode jurisdictional causality rather than temporal ordering. Each harbor maintains a local chain with cross-references to other harbors. Bilateral operations finalize in O(1) harbor communication."]));
  c.push(p([bold("Treaty Lattice Consensus (TLC)."), " Treaty relationships form a complete lattice. Operations on disjoint treaty subgraphs commute and proceed in parallel. The lattice join combines consensus from independent harbor sets."]));
  c.push(makeTable(
    ["Transaction Type", "Throughput (TPS)", "Latency", "Consensus"],
    [
      ["Private (owned record)", "200K-300K", "<200ms", "No"],
      ["Private (shared object)", "125K-200K", "<2s", "Yes"],
      ["Compliance-revealing", "10K-50K", "<2s", "Yes"],
      ["Cross-shard", "50K-100K", "<3s", "Yes"],
    ],
    [2400, 2400, 1800, 2760]
  ));
  c.push(spacer());

  c.push(heading3("13.4 Sharding Architecture"));
  c.push(p([bold("Tier 1: Execution Shards."), " Harbor Shards H = {H1, ..., Hn} provide jurisdiction-aligned execution environments. Each harbor Hi corresponds to a legal jurisdiction with associated lawpack Li. Corridor Shards C = {C1, ..., Cm} enable cross-jurisdiction settlement channels."]));
  c.push(p([bold("Tier 2: Root Chain."), " Single coordination chain R providing global state aggregation via recursive proof verification, final proof verification and anchoring, asset and corridor anchor registry, fee routing and protocol token economics, and epoch management and validator set updates."]));
  c.push(...codeBlock([
    "/// A harbor: jurisdiction-aligned execution environment.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Harbor {",
    "    pub id: HarborId,",
    "    pub jurisdiction: JurisdictionId,",
    "    pub validators: Vec<ValidatorId>,",
    "    pub local_chain: LocalChain,",
    "    pub dag_references: Vec<DagReference>,",
    "    pub treaty_set: BTreeSet<TreatyId>,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading2("Chapter 14: Proving System"));
  c.push(heading3("14.1 Plonky3 Architecture"));
  c.push(p("Plonky3 represents the third generation of Polygon plonky systems. BabyBear Field (p = 2^31 - 2^27 + 1) chosen for optimal CPU and GPU performance, providing 4-8x proving speedup over BN254. Configuration: Extension Degree 4 (~124-bit security), Poseidon2 hash (width 16, \u03B1 = 7), FRI polynomial commitment (rate 1/8, blowup 8), conjectured 100 bits security, native proof size 50-100 KB."));
  c.push(...codeBlock([
    "/// Plonky3 prover configuration.",
    "#[derive(Debug, Clone)]",
    "pub struct Plonky3Config {",
    "    pub field: BabyBearField,",
    "    pub extension_degree: u32,  // 4",
    "    pub hash: Poseidon2Config,",
    "    pub fri_rate: f64,          // 1/8",
    "    pub security_bits: u32,     // 100",
    "    pub max_constraint_degree: u32,",
    "}",
    "",
    "impl Plonky3Config {",
    "    pub fn production() -> Self {",
    "        Self {",
    "            field: BabyBearField::new(),",
    "            extension_degree: 4,",
    "            hash: Poseidon2Config::width16_alpha7(),",
    "            fri_rate: 0.125,",
    "            security_bits: 100,",
    "            max_constraint_degree: 3,",
    "        }",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("14.2 Proof Aggregation"));
  c.push(p("Transaction proofs aggregate through multiple layers. Layer 1 batches individual transaction proofs into block proofs (1,000-10,000 transactions). Layer 2 combines block proofs into epoch proofs. Final wrapping converts STARK proofs to Groth16 for on-chain verification (~250,000 gas on EVM chains)."));
  c.push(makeTable(
    ["Layer", "Input", "Output", "Aggregation Factor"],
    [
      ["Transaction", "Single transaction", "Transaction proof (STARK)", "1:1"],
      ["Block (Layer 1)", "1K-10K transaction proofs", "Block proof (STARK)", "1000-10000:1"],
      ["Epoch (Layer 2)", "10-100 block proofs", "Epoch proof (STARK)", "10-100:1"],
      ["Final Wrapping", "Single epoch proof", "Groth16 proof (288 bytes)", "1:1"],
    ],
    [1800, 2800, 2800, 1960]
  ));
  c.push(spacer());
  c.push(p("Recursive composition via Halo2 enables proof-of-proof verification without trusted setup. The final Groth16 wrapping provides constant-size proofs for on-chain verification at approximately 250,000 gas cost on EVM chains, making settlement economically viable at scale."));

  c.push(heading3("14.3 Client-Side Proving"));
  c.push(makeTable(
    ["Device Category", "Prove Time", "Notes"],
    [
      ["Modern Smartphone (2023+)", "<10s", "GPU acceleration available"],
      ["Older Smartphone", "<60s", "CPU-only"],
      ["Desktop (GPU)", "<2s", "CUDA/Metal acceleration"],
      ["Server (Multi-GPU)", "<100ms", "Production prover"],
    ],
    [3200, 2000, 4160]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 15: Privacy Architecture"));
  c.push(heading3("15.1 Key Hierarchy"));
  c.push(makeTable(
    ["Key Type", "Capability", "Use Case"],
    [
      ["Spending Key (sk)", "Full account control", "Cold storage, high-value accounts"],
      ["Full Viewing Key (fvk)", "Decrypt all transactions", "Audit functions"],
      ["Incoming Viewing Key (ivk)", "Decrypt received only", "Accounting systems"],
      ["Detection Key (dk)", "Efficient scanning", "Lightweight clients"],
      ["Compliance Viewing Key (cvk)", "Selective disclosure", "Regulatory compliance"],
    ],
    [3000, 2800, 3560]
  ));
  c.push(spacer());

  c.push(heading3("15.2 Transaction Privacy"));
  c.push(p("Private transactions reveal no information to observers beyond the fact that a valid state transition occurred. Amount, sender, recipient, and asset type are hidden through encryption and zero-knowledge proofs. Nullifier mechanisms prevent double-spending without revealing which records are spent."));

  c.push(heading3("15.3 Compliance Integration"));
  c.push(p("Privacy and compliance coexist through zero-knowledge compliance proofs. zkKYC attestations bind verified identity to cryptographic keys without continuous identity disclosure. Private Proofs of Innocence (PPOI) demonstrate non-membership in sanctions lists without revealing identity. Zone-based compliance enables configurable rule sets per jurisdiction through the compliance tensor model."));

  c.push(heading2("Chapter 16: L1 Anchoring Protocol"));
  c.push(heading3("16.1 Anchor Types"));
  c.push(p([bold("Asset Checkpoint Anchor."), " Commits Smart Asset state to the settlement layer including: asset_id, checkpoint_seq, mmr_root, state_commitment, tensor_commitment, watcher_attestations, checkpoint_proof, anchor_block, and anchor_timestamp."]));
  c.push(p([bold("Corridor State Anchor."), " Commits corridor state including: corridor_id, checkpoint_seq, corridor_root, participant_roots, settlement_references, and coordination_proof."]));

  c.push(heading3("16.2 Anchor Targets"));
  c.push(p("The protocol supports multiple anchor targets with configurable chain_type (mass_l1, ethereum, etc.), contract_address, and anchor_method (StateRoot, FullCheckpoint, ZKProven)."));

  c.push(heading3("16.3 L1-Optional Design"));
  c.push(p("The SEZ Stack is L1-optional. All core commitments are content-addressed and signed, and may be anchored to an external chain without redesign. Pre-L1: state commitments via receipt chains, finality via watcher quorum, CAS storage. With L1: periodic anchoring of corridor state roots, ZK-proven state transitions, blockchain-backed finality. No change to the identity or digest model because everything is already content-addressed."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART VII: GOVERNANCE AND CIVIC SYSTEMS
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART VII: GOVERNANCE AND CIVIC SYSTEMS"));
  c.push(heading2("Chapter 17: Constitutional Framework"));

  c.push(heading3("17.1 Constitutional Structure"));
  c.push(p("Zone constitutions are hierarchical documents with provisions at multiple protection levels:"));
  c.push(makeTable(
    ["Level", "Modification Requirement", "Typical Content"],
    [
      ["Level 1", "Zone dissolution/reformation", "Fundamental rights, structural provisions"],
      ["Level 2", "Supermajority approval (75%+)", "Constitutional amendments"],
      ["Level 3", "Simple majority approval", "Major policy changes"],
      ["Level 4", "Administrative action", "Operational policies, fee schedules"],
    ],
    [1600, 3200, 4560]
  ));
  c.push(spacer());
  c.push(...codeBlock([
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Constitution {",
    "    pub zone_id: JurisdictionId,",
    "    pub preamble: String,",
    "    pub core_provisions: Vec<Provision>,      // Supermajority to amend",
    "    pub standard_provisions: Vec<Provision>,   // Simple majority",
    "    pub operational_policies: Vec<Policy>,      // Admin discretion",
    "    pub amendment_procedures: AmendmentProcedures,",
    "    pub stakeholder_rights: StakeholderRights,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("17.2 Voting Mechanisms"));
  c.push(p([bold("Token-Weighted Voting"), " assigns voting power proportional to stake holdings. ", bold("One-Entity-One-Vote"), " assigns equal voting power regardless of size. ", bold("Quadratic Voting"), " reduces the influence of large stakeholders by making marginal votes progressively more expensive. ", bold("Conviction Voting"), " weights votes by duration of commitment."]));

  c.push(heading3("17.3 Delegation and Representation"));
  c.push(p("Liquid democracy features enable flexible delegation while preserving direct participation rights. Topic-specific delegation allows different delegates for different domains. Delegation chains are limited to configurable depth (typically 2-3 levels). Instant recall enables stakeholders to revoke delegation at any time."));

  c.push(heading2("Chapter 18: Civic Services Integration"));
  c.push(heading3("18.1 Identity Services"));
  c.push(p("Zone identity services provide residents and businesses with verifiable credentials: resident credentials (zone residency status, rights, obligations), business credentials (entity registration, good standing, authorized activities), professional credentials (qualifications, licensing for regulated professions). All credentials support selective disclosure via BBS+."));

  c.push(heading3("18.2 Property Services"));
  c.push(p("Property rights are represented as Smart Assets with zone-specific lawpack bindings. Title registry maintains the authoritative record of property ownership using append-only receipt chains. Transfer services facilitate property transactions with compliance verification. Encumbrance management tracks liens, mortgages, and other property interests."));

  c.push(heading3("18.3 Dispute Resolution Services"));
  c.push(p("Small claims procedures handle low-value disputes through expedited processes. Commercial arbitration handles business disputes through international arbitration institutions (DIFC-LCIA, SIAC, AIFC-IAC, ICC). Appellate procedures enable review of initial determinations."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART VIII: COMPLIANCE AND REGULATORY INTEGRATION
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART VIII: COMPLIANCE AND REGULATORY INTEGRATION"));

  c.push(heading2("Chapter 19: Compliance Architecture"));
  c.push(heading3("19.1 Compliance Model"));
  c.push(p("The Stack compliance model separates rule specification from rule enforcement. Rules are encoded in the Pack Trilogy; enforcement occurs through the Smart Asset execution layer. Rule specification uses predicate-based formalism. Predicates compose through logical operators. Enforcement mechanisms vary by rule type: some require pre-transaction verification, others require post-transaction reporting. Violation handling follows configurable escalation paths."));

  c.push(heading3("19.2 Identity Verification"));
  c.push(p("Identity verification follows the zkKYC model, enabling compliance verification without continuous identity disclosure. Verification providers issue Verifiable Credentials attesting to verification completion. Re-verification triggers when circumstances change or credentials expire."));

  c.push(heading3("19.3 Transaction Monitoring"));
  c.push(p("Rule engines evaluate configurable pattern rules: velocity anomalies, structuring patterns, high-risk counterparties. Privacy preservation techniques enable monitoring without mass surveillance. Investigation procedures specify how flagged activity is examined through formal authorization."));

  c.push(heading2("Chapter 20: Compliance Manifold"));
  c.push(heading3("20.1 Manifold Definition"));
  c.push(...definitionBlock("Definition 20.1 (Compliance Manifold).", "The compliance manifold M is a continuous surface over jurisdictional coordinates where height represents compliance burden: M: R^n -> R, where n is the number of jurisdictional dimensions."));
  c.push(...codeBlock([
    "/// The Compliance Manifold: optimization over compliance state space.",
    "pub struct ComplianceManifold {",
    "    pub jurisdictions: Vec<JurisdictionId>,",
    "    pub domains: Vec<ComplianceDomain>,",
    "    pub transition_costs: BTreeMap<(JurisdictionId, JurisdictionId, ComplianceDomain), TransitionCost>,",
    "}",
    "",
    "impl ComplianceManifold {",
    "    /// Find optimal migration path from source to target jurisdiction.",
    "    pub fn optimal_path(",
    "        &self,",
    "        asset: &SmartAsset,",
    "        source: &JurisdictionId,",
    "        target: &JurisdictionId,",
    "    ) -> Result<MigrationPath, ManifoldError> {",
    "        self.dijkstra(asset, source, target)",
    "    }",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("20.2 Migration Path Optimization"));
  c.push(p("Path cost is computed as the line integral of compliance burden along the path: Cost(P) = \u222B_P compliance_burden(x) dx. The optimal path minimizes this integral while satisfying all waypoint constraints."));

  c.push(heading3("20.3 Attestation Gap Analysis"));
  c.push(...definitionBlock("Definition 20.3 (Attestation Gap).", "An attestation gap represents a compliance requirement not satisfied by current attestations. It includes the requirement specification, domain, jurisdiction, severity, blocking status, and remediation options with estimated time and cost to fill."));

  c.push(heading2("Chapter 21: zkKYC and Privacy-Preserving Compliance"));
  c.push(p("zkKYC attestations bind verified identity to cryptographic keys without continuous identity disclosure. Private Proofs of Innocence (PPOI) demonstrate non-membership in sanctions lists without revealing identity. These are implemented as ZK circuits in the proof hierarchy."));
  c.push(...codeBlock([
    "/// zkKYC: prove KYC tier without revealing identity.",
    "pub struct ZkKycProof {",
    "    pub proof: ZkProof,",
    "    pub public_inputs: ZkKycPublicInputs,",
    "}",
    "",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct ZkKycPublicInputs {",
    "    pub credential_commitment: Digest,",
    "    pub kyc_tier: KycTier,",
    "    pub jurisdiction: JurisdictionId,",
    "    pub expiry: chrono::DateTime<chrono::Utc>,",
    "}",
  ]));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART IX: CRYPTOGRAPHIC CORRIDOR SYSTEMS
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART IX: CRYPTOGRAPHIC CORRIDOR SYSTEMS"));

  c.push(heading2("Chapter 22: Corridor Architecture"));
  c.push(heading3("22.1 Corridor Establishment"));
  c.push(p("Corridor establishment follows Protocol 14.1, creating bilateral channels between consenting jurisdictions. The process comprises four phases: policy alignment ensures compatible compliance frameworks between parties, technical integration connects infrastructure through authenticated channels, governance agreement specifies corridor administration, amendment procedures, dispute resolution, and termination conditions, and activation produces a corridor definition Verifiable Credential binding all participants."));
  c.push(p("Policy alignment requires each jurisdiction to publish its policy requirements as a machine-readable specification derived from its Pack Trilogy. The corridor negotiation engine identifies compatible policy overlaps across all twenty compliance domains. Where policies conflict, the engine proposes resolution strategies: union (apply the stricter of two requirements), intersection (apply only shared requirements), or escalation (flag for human resolution). The resulting policy set becomes the corridor compliance baseline."));
  c.push(...codeBlock([
    "/// Corridor establishment request.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct CorridorEstablishmentRequest {",
    "    pub initiator: JurisdictionId,",
    "    pub respondent: JurisdictionId,",
    "    pub proposed_operations: Vec<OperationType>,",
    "    pub policy_requirements: PolicyRequirements,",
    "    pub governance_proposal: GovernanceProposal,",
    "    pub technical_endpoint: TechnicalEndpoint,",
    "    pub valid_from: chrono::DateTime<chrono::Utc>,",
    "    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,",
    "}",
    "",
    "/// Corridor governance: administration, amendments, dispute resolution.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct CorridorGovernance {",
    "    pub administrator: JurisdictionId,",
    "    pub amendment_procedure: AmendmentProcedure,",
    "    pub dispute_resolution: DisputeResolution,",
    "    pub termination_conditions: Vec<TerminationCondition>,",
    "    pub fee_sharing: FeeSharing,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("22.2 Corridor Definition"));
  c.push(...codeBlock([
    "/// A corridor: cryptographic channel between jurisdictions.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Corridor {",
    "    pub id: CorridorId,",
    "    pub participants: Vec<JurisdictionId>,",
    "    pub definition_vc: VerifiableCredential,",
    "    pub agreement_vc: VerifiableCredential,",
    "    pub permitted_operations: Vec<OperationType>,",
    "    pub compliance_requirements: CorridorCompliance,",
    "    pub state_channel: CorridorStateChannel,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("22.3 State Synchronization"));
  c.push(p("Vector clocks track causality across jurisdictions. Each state update increments the local clock component. Merkle proofs enable efficient delta synchronization. Conflict resolution follows deterministic rules specified in the corridor manifest."));

  c.push(heading3("22.4 Lifecycle State Machine"));
  c.push(p("Corridors operate within a lifecycle state machine: DRAFT \u2192 PENDING \u2192 ACTIVE, with branches to HALTED and SUSPENDED, ultimately leading to TERMINATED. Evidence-gated transitions require specific credentials."));

  c.push(heading2("Chapter 23: Corridor Bridge Protocol"));
  c.push(heading3("23.1 Bridge Architecture"));
  c.push(p("The Corridor Bridge Protocol orchestrates cross-corridor asset transfers with atomic execution guarantees. The bridge discovers optimal transfer paths through the corridor graph using Dijkstra with compliance-weighted edges."));

  c.push(heading3("23.2 Path Discovery"));
  c.push(p("The PathRouter maintains a corridor_graph, compliance_manifold, and fee_oracle. Path finding retrieves the k-shortest paths, filters by compliance, and selects the optimal path by total cost."));

  c.push(heading3("23.3 Atomic Execution"));
  c.push(p("Bridge transfers execute atomically through phases: Initiated, PathSelected, SourceLocked, InTransit (with current_hop tracking), DestinationReached, and Completed. Failure at any phase triggers compensation."));

  c.push(heading3("23.4 Fee Computation"));
  c.push(p("Bridge fees are computed per-hop with aggregation: base_fee, per_hop_fee, compliance_fee, optional priority_fee, total, fee_currency, and breakdown array."));

  c.push(heading2("Chapter 24: Multilateral Corridors"));
  c.push(heading3("24.1 Hub and Spoke Model"));
  c.push(p("The hub and spoke model designates one jurisdiction as the corridor hub for state coordination, dispute arbitration, and manifest maintenance. Spoke-to-spoke operations route through hub intermediation. Hub migration enables changing the hub jurisdiction."));

  c.push(heading3("24.2 Mesh Model"));
  c.push(p("The mesh model maintains direct corridors between all participating jurisdictions. Full mesh connectivity requires n(n-1)/2 bilateral corridors for n jurisdictions. Hybrid approaches combine mesh and spoke elements."));

  c.push(heading2("Chapter 25: Live Corridors"));
  c.push(heading3("25.1 PAK\u2194KSA Corridor ($5.4B Bilateral)"));
  c.push(p("Saudi-Pakistan bilateral trade totals $5.4B annually. The corridor automates customs duties, withholding tax on remittances from 2.5M Pakistani diaspora, and trade documentation. Status: launch phase under the Saudi-Pakistan SMDA 2025 framework."));
  c.push(makeTable(
    ["Component", "Implementation"],
    [
      ["Customs Automation", "HS code harmonization, duty calculation, preferential rates under bilateral agreements"],
      ["Remittance WHT", "Automatic withholding on $2.1B annual remittances per ITO 2001 Schedule"],
      ["Diaspora Services", "NTN registration, tax filing for 2.5M Pakistanis in KSA"],
      ["Trade Docs", "Electronic bills of lading, certificates of origin, phytosanitary certificates"],
    ],
    [2400, 6960]
  ));
  c.push(spacer());

  c.push(heading3("25.2 PAK\u2194UAE Corridor ($10.1B Bilateral)"));
  c.push(p("UAE-Pakistan bilateral trade totals $10.1B annually with $6.7B in remittances. Mass operates in 27 Dubai Free Zones. The SIFC FDI pipeline channels investment through the corridor. Status: live."));

  c.push(heading3("25.3 PAK\u2194CHN Corridor ($23.1B Bilateral)"));
  c.push(p("China-Pakistan trade totals $23.1B annually, primarily through CPEC 2.0. Nine SEZs, Gwadar customs operations, and e-trade documentation planned for corridor integration. Status: planned."));

  c.push(makeTable(
    ["Corridor", "Volume", "Status", "Key Features"],
    [
      ["PAK\u2194KSA", "$5.4B", "Launch", "SMDA 2025, customs automation, WHT remittances, 2.5M diaspora"],
      ["PAK\u2194UAE", "$10.1B", "Live", "Mass in 27 Dubai FZs, $6.7B remittances, SIFC FDI pipeline"],
      ["PAK\u2194CHN", "$23.1B", "Planned", "CPEC 2.0, 9 SEZs, Gwadar customs, e-trade docs"],
    ],
    [1600, 1400, 1200, 5160]
  ));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART X: WATCHER ECONOMY
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART X: WATCHER ECONOMY"));
  c.push(heading2("Chapter 26: Watcher Architecture"));
  c.push(heading3("26.1 Watcher Identity"));
  c.push(p("The Watcher Economy transforms watchers from passive observers to accountable economic actors whose attestations carry weight backed by staked collateral."));
  c.push(...codeBlock([
    "/// A watcher: bonded attestation provider.",
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Watcher {",
    "    pub id: WatcherId,",
    "    pub did: String,",
    "    pub bond: Bond,",
    "    pub jurisdictions: Vec<JurisdictionId>,",
    "    pub domains: Vec<ComplianceDomain>,",
    "    pub attestation_history: Vec<AttestationRecord>,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("26.2 Watcher Roles"));
  c.push(makeTable(
    ["Role", "Function", "Scope"],
    [
      ["Checkpoint Watchers", "Attest to Smart Asset checkpoint validity", "Receipt chain integrity, state commitment"],
      ["Corridor Watchers", "Attest to corridor state transitions", "Cross-jurisdictional operations"],
      ["Compliance Watchers", "Verify compliance tensor state", "Authoritative source verification"],
      ["Settlement Watchers", "Observe settlement layer state", "L1 anchor correctness, finality"],
    ],
    [2400, 3600, 3360]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 27: Bond and Slashing Mechanics"));
  c.push(heading3("27.1 Watcher Bonds"));
  c.push(...codeBlock([
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub struct Bond {",
    "    pub amount: u64,",
    "    pub currency: CurrencyCode,",
    "    pub lockup_until: chrono::DateTime<chrono::Utc>,",
    "    pub slashing_conditions: Vec<SlashingCondition>,",
    "}",
  ]));
  c.push(p("Bond amount determines the maximum value a watcher can attest to, typically 10x the bond amount."));

  c.push(heading3("27.2 Slashing Conditions"));
  c.push(makeTable(
    ["Condition", "Trigger", "Evidence", "Penalty"],
    [
      ["SC1: Equivocation", "Conflicting attestations", "Two valid signatures on incompatible checkpoints", "100% bond forfeiture"],
      ["SC2: Availability", "Missing required attestations", "Missing attestations over threshold", "1% per incident"],
      ["SC3: False Attestation", "Attesting to invalid state", "Checkpoint + receipts showing inconsistency", "50% bond forfeiture"],
      ["SC4: Collusion", "Coordinated false attestation", "Pattern analysis", "100% + permanent ban"],
    ],
    [2000, 2000, 3000, 2360]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 28: Quorum and Finality"));
  c.push(heading3("28.1 Quorum Policies"));
  c.push(...codeBlock([
    "#[derive(Debug, Clone, Serialize, Deserialize)]",
    "pub enum QuorumPolicy {",
    "    Simple { threshold: u32 },",
    "    Weighted { threshold: u64, weights: BTreeMap<WatcherId, u64> },",
    "    Jurisdictional {",
    "        per_jurisdiction: BTreeMap<JurisdictionId, u32>,",
    "        global_threshold: u32,",
    "    },",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("28.2 Finality Levels"));
  c.push(makeTable(
    ["Level", "Requirements", "Guarantees"],
    [
      ["Tentative", "Single watcher attestation", "Operational continuity"],
      ["Provisional", "Minority quorum", "Basic safety"],
      ["Confirmed", "Full quorum", "Standard finality"],
      ["Final", "Quorum + L1 anchor", "Maximum security"],
    ],
    [2000, 3600, 3760]
  ));
  c.push(spacer());
  c.push(theoremBlock("Theorem 28.1 (Watcher Accountability).", "The slashing mechanism ensures watcher accountability. Dishonest attestations result in provable collateral loss. Given a conflicting attestation pair from the same watcher for the same (asset, jurisdiction, domain) tuple, the slashing contract verifies signatures, confirms conflict, and executes bond forfeiture."));

  c.push(pageBreak());

  // ═══════════════════════════════════════════════════════════
  // PART XI: MIGRATION PROTOCOL
  // ═══════════════════════════════════════════════════════════
  c.push(heading1("PART XI: MIGRATION PROTOCOL"));
  c.push(heading2("Chapter 29: Cross-Jurisdictional Migration"));
  c.push(heading3("29.1 Migration Request"));
  c.push(p("The Migration Protocol orchestrates Smart Asset movement between jurisdictions while maintaining continuous compliance and operational integrity. A MigrationRequest contains: request_id, asset_id, asset_state_commitment, source_jurisdiction, destination_jurisdiction, preferred_path, requested_at timestamp, deadline, owner_signature, max_fee, and fee_currency."));

  c.push(heading3("29.2 Migration Phases"));
  c.push(makeTable(
    ["Phase", "Action", "Compensation"],
    [
      ["INITIATED", "Request received and validated", "Log and close"],
      ["COMPLIANCE_CHECK", "Source and destination compliance verified", "Log failure reason"],
      ["ATTESTATION_GATHERING", "Required attestations collected", "Release partial attestations"],
      ["SOURCE_LOCK", "Asset locked at source jurisdiction", "Unlock at source"],
      ["TRANSIT", "Asset state in transit", "Rollback to source"],
      ["DESTINATION_VERIFICATION", "Destination compliance verification", "Return to source"],
      ["DESTINATION_UNLOCK", "Asset unlocked at destination", "N/A"],
      ["COMPLETED", "Migration successfully completed", "N/A"],
    ],
    [2800, 3600, 2960]
  ));
  c.push(spacer());

  c.push(heading2("Chapter 30: Migration State Machine"));
  c.push(heading3("30.1 State Transitions"));
  c.push(...codeBlock([
    "/// Migration state machine.",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]",
    "pub enum MigrationState {",
    "    Initiated, SourceLocked, InTransit,",
    "    DestinationVerified, Completed,",
    "    Compensating, Compensated, Failed,",
    "}",
  ]));
  c.push(spacer());

  c.push(heading3("30.2 State Implementation"));
  c.push(...codeBlock([
    "/// The migration saga: atomic cross-jurisdiction asset transfer.",
    "pub struct MigrationSaga {",
    "    pub migration_id: MigrationId,",
    "    pub asset_id: AssetId,",
    "    pub source: JurisdictionId,",
    "    pub target: JurisdictionId,",
    "    pub state: MigrationState,",
    "    pub completed_steps: Vec<MigrationStep>,",
    "    pub compensation_steps: Vec<CompensationStep>,",
    "}",
    "",
    "impl MigrationSaga {",
    "    pub fn advance(&mut self, proof: MigrationProof) -> Result<(), MigrationError

