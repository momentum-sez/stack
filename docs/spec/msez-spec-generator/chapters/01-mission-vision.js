const {
  partHeading, chapterHeading, h2, p, p_runs, bold,
  table, spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter01() {
  return [
    // --- PART I: FOUNDATION ---
    ...partHeading("PART I: FOUNDATION"),

    chapterHeading("Chapter 1: Mission and Vision"),

    p("The Momentum SEZ Stack exists to compress the creation of high-quality economic governance from years to months. Traditional Special Economic Zones demand $50\u2013100M in capital, regulatory counsel across multiple domains, and 18\u201336 months before a single license is issued. The SEZ Stack reduces this to a single deployment command, a curated profile, and 90 days to first license."),

    p("The fundamental insight driving the Stack is that economic governance can be decomposed into modular, composable primitives. Just as software libraries enable rapid application development through component reuse, governance primitives enable rapid jurisdiction development through regulatory composition. A new SEZ need not design corporate formation processes from scratch when proven implementations exist and can be adapted to local requirements."),

    p("This modularity extends beyond code reuse to encompass regulatory recognition. When multiple jurisdictions deploy compatible Stack implementations, mutual recognition becomes computationally verifiable rather than diplomatically negotiated. A corporate entity formed in one Stack-compatible jurisdiction can be algorithmically recognized in another, with compliance verified through cryptographic proofs rather than document review."),

    // --- 1.1 The Programmable Institution Thesis ---
    h2("1.1 The Programmable Institution Thesis"),

    p("The SEZ Stack embodies a thesis: institutions can be programmable without sacrificing sovereignty. A programmable institution encodes its rules in machine-executable form, enforces them through cryptographic mechanisms, and provides mathematical guarantees about compliance state. This is not automation of existing paper processes; it is a fundamental reconception of how governance operates."),

    p("The evidence for this thesis comes from production deployments across multiple jurisdictions, each demonstrating that programmable governance delivers measurable improvements in speed, cost, transparency, and compliance coverage."),

    table(
      ["Deployment", "Status", "Evidence"],
      [
        ["Pakistan GovOS (PDA)", "Active", "Full government OS: 40+ ministries, FBR tax integration (Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act, Customs Act), SBP Raast payments, NADRA identity, SECP corporate registry. Target: raise tax-to-GDP from 10.3% to 15%+."],
        ["UAE / ADGM", "Live", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
        ["Dubai Free Zone Council", "Integration", "27 free zones. Mass APIs serve entity + fiscal; MSEZ provides zone-specific licensing."],
        ["Kazakhstan (Alatau City)", "Partnership", "SEZ + AIFC integration. Tests composition engine: Kazakh law + AIFC financial regulation."],
        ["Seychelles", "Deployment", "Sovereign GovOS at national scale."],
      ],
      [2400, 1200, 5760]
    ),
    spacer(),

    // --- 1.2 The Two-System Architecture ---
    h2("1.2 The Two-System Architecture"),

    p("The separation between Mass and the MSEZ Stack is the most important architectural constraint in the system. Every design decision, API boundary, data flow, and deployment topology traces back to this separation. Understanding it is prerequisite to understanding anything else in this specification."),

    p_runs([
      bold("System A: Mass \u2014 The Five Programmable Primitives. "),
      "Mass provides five jurisdiction-agnostic APIs that implement the fundamental operations of economic governance: creating legal entities, managing ownership, processing payments, verifying identity, and recording consent. These APIs are live, deployed, and processing real capital. They are NOT part of this specification \u2014 they are the substrate upon which the MSEZ Stack builds."
    ]),

    table(
      ["Primitive", "Live API Surface", "Function"],
      [
        ["Entities", "organization-info.api.mass.inc", "Formation, lifecycle, dissolution. Each entity is a legal actor, a Smart Asset."],
        ["Ownership", "investment-info (Heroku seed)", "Cap tables, token tables, beneficial ownership, equity instruments, fundraising rounds."],
        ["Fiscal", "treasury-info.api.mass.inc", "Accounts, wallets, on/off-ramps, payments, treasury, withholding tax at source."],
        ["Identity", "Distributed across org + consent", "Passportable KYC/KYB. Onboard once, reuse everywhere."],
        ["Consent", "consent.api.mass.inc", "Multi-party auth, audit trails, board/shareholder/controller sign-off workflows."],
      ],
      [1800, 3200, 4360]
    ),
    spacer(),

    p("Supporting infrastructure includes the Templating Engine (document generation via Heroku) and Organs (composable service modules that extend Mass primitives with domain-specific logic)."),

    p_runs([
      bold("System B: MSEZ Stack \u2014 The Jurisdictional Context. "),
      "The MSEZ Stack sits above Mass and provides jurisdictional context: the legal, regulatory, compliance, and corridor infrastructure that transforms Mass API calls from generic primitive operations into jurisdiction-aware operations. The Stack knows about Pakistani tax law, UAE free zone regulations, Kazakh AIFC rules, and cross-border trade compliance. Mass does not."
    ]),

    p_runs([
      bold("The Interface Contract. "),
      "The boundary between Mass and the MSEZ Stack is defined by a strict interface contract. The MSEZ Stack defines what is permitted, required, and prohibited in each jurisdiction. Mass executes the permitted operations. The Stack never duplicates Mass CRUD operations; it enriches them with compliance context."
    ]),

    table(
      ["Function", "Provided By", "MSEZ Spec Treatment"],
      [
        ["Entity formation", "Mass Org API", "Defines permitted entity types, formation requirements, fees. Does NOT implement formation."],
        ["Cap table management", "Mass Investment API", "Defines securities regulations, issuance rules. Does NOT implement cap tables."],
        ["Bank account opening", "Mass Treasury API", "Defines banking license requirements, AML rules. Does NOT implement accounts."],
        ["KYC/KYB verification", "Mass Identity", "Defines KYC tier requirements per jurisdiction. Does NOT implement verification."],
        ["Board resolution signing", "Mass Consent API", "Defines governance rules, quorum requirements. Does NOT implement workflows."],
        ["Compliance state evaluation", "MSEZ Compliance Tensor", "This IS the MSEZ Stack. Full specification herein."],
        ["Law encoding", "MSEZ Pack Trilogy", "This IS the MSEZ Stack. Full specification herein."],
        ["Cross-border corridors", "MSEZ Corridor System", "This IS the MSEZ Stack. Full specification herein."],
        ["Attestation accountability", "MSEZ Watcher Economy", "This IS the MSEZ Stack. Full specification herein."],
      ],
      [2400, 2400, 4560]
    ),
    spacer(),

    // --- 1.3 The Orthogonal Execution Layer ---
    h2("1.3 The Orthogonal Execution Layer"),

    p("Mass introduces a decentralized execution layer \u2014 the Mass Protocol \u2014 that provides cryptographic settlement for operations coordinated by the SEZ Stack. This layer is orthogonal to the Stack: the Stack can operate without it (using traditional database persistence), and the Protocol can serve applications beyond SEZ governance."),

    p("When deployed together, the Stack provides the jurisdictional intelligence (what operations are legally permitted, what compliance requirements apply, what attestations are needed) and the Protocol provides the execution guarantees (that operations are atomically settled, that state transitions are cryptographically verified, that cross-jurisdictional operations maintain consistency). This separation ensures that neither system depends on the other for correctness, while their composition delivers capabilities neither could achieve alone."),

    // --- 1.4 Design Principles ---
    h2("1.4 Design Principles"),

    p_runs([
      bold("Sovereignty Preservation. "),
      "Every jurisdiction retains full control over its legal framework, regulatory policy, and enforcement mechanisms. The Stack provides tools for encoding and evaluating these rules; it never overrides them. A jurisdiction can modify any rule at any time, and the Stack will reflect the change in all subsequent compliance evaluations."
    ]),
    p_runs([
      bold("Privacy by Default. "),
      "Entity data, financial records, and identity information are never exposed beyond the minimum required for regulatory compliance. BBS+ selective disclosure enables credential verification without revealing underlying data. Zero-knowledge proofs enable compliance demonstration without exposing business logic or transaction details."
    ]),
    p_runs([
      bold("Interoperability First. "),
      "All data structures use open standards: W3C Verifiable Credentials, Akoma Ntoso for legislation, ISO 20022 for financial messaging, SWIFT pacs.008 for cross-border payments. The Stack produces and consumes standard formats, enabling integration with existing systems without custom adapters."
    ]),
    p_runs([
      bold("Cryptographic Verifiability. "),
      "Every compliance evaluation, every credential issuance, every corridor state transition produces a cryptographic proof that can be independently verified. The system does not ask participants to trust the operator; it provides mathematical evidence that operations were performed correctly."
    ]),
    p_runs([
      bold("Graceful Degradation. "),
      "If the compliance tensor is unavailable, operations proceed with cached compliance state and a warning. If a corridor counterparty is unreachable, operations queue and retry with exponential backoff. If the zero-knowledge proof system is not deployed, the Stack falls back to traditional attestation. No single component failure renders the system inoperable."
    ]),
    p_runs([
      bold("Regulatory Agility. "),
      "When regulations change \u2014 and they change frequently \u2014 the Stack reflects updates through regpack modifications, not code deployments. A new sanctions list, a revised tax rate, an updated filing deadline: each is a data change in the regpack, immediately reflected in compliance evaluations without recompilation or redeployment."
    ]),
    p_runs([
      bold("Auditability and Transparency. "),
      "Every decision the Stack makes is logged, attributed, and reproducible. Compliance evaluations include the tensor state, pack versions, and evaluation timestamp. Corridor operations include receipt chains with Merkle proofs. Credential issuances include the issuer identity, evidence chain, and revocation status. An auditor can reconstruct any decision from its audit trail."
    ]),
    p_runs([
      bold("Compile-Time Safety. "),
      "The codebase is pure Rust (2024 edition) with zero unsafe blocks. The type system enforces domain invariants: a JurisdictionCode cannot be used where a CurrencyCode is expected, a ComplianceScore cannot be constructed without evaluation, a Corridor cannot transition to an invalid state. Bugs that survive the compiler are bugs in the specification, not in the implementation."
    ]),
  ];
};
