const { chapterHeading, h2, h3, p, p_runs, bold, italic, table, pageBreak } = require("../lib/primitives");

module.exports = function build_executive_summary() {
  return [
    chapterHeading("Executive Summary"),

    p("The Momentum Open Source SEZ Stack (version 0.4.44, GENESIS) is a software system for instantiating cryptographically auditable, compliance-enforcing Special Economic Zones. It is implemented in Rust (2024 edition), organized as 17 crates totaling approximately 109,000 lines of production code across 257 source files with over 5,000 tests. This specification documents its complete technical architecture in 56 chapters across 18 Parts, plus 11 appendices."),

    p("The architecture enforces a strict separation between two systems. Mass provides five jurisdiction-agnostic programmable primitives (Entities, Ownership, Fiscal, Identity, Consent) deployed as live Java/Spring Boot production APIs. The MSEZ Stack is the jurisdictional orchestration layer that makes those primitives compliance-aware through a compliance tensor (\u00a710), pack trilogy (\u00a76), corridor system (\u00a722), and verifiable credentials (\u00a743). This separation is the central architectural invariant: Mass owns business object CRUD; the MSEZ Stack owns jurisdictional context, compliance evaluation, and cryptographic attestation."),

    p("Version 0.4.44 adds institutional infrastructure (corporate services, identity, tax, capital markets, trade), a four-layer GovOS architecture for sovereign deployment (\u00a738), a multi-jurisdiction composition engine (\u00a712), and production deployment tooling (\u00a749\u201353). The system can be configured for a target jurisdiction and deployed to yield a zone that processes entity formation, enforces tax law, clears securities, manages trade corridors, and produces compliance attestations."),

    p("Production deployments: Pakistan GovOS (40+ ministries, FBR tax integration, SBP Raast payments, NADRA identity, SECP corporate registry); UAE/ADGM (1,000+ entities, $1.7B+ capital processed); Dubai Free Zone Council (27 free zones); Kazakhstan Alatau City (SEZ + AIFC composition). Active trade corridors: PAK\u2194KSA ($5.4B), PAK\u2194UAE ($10.1B), PAK\u2194CHN ($23.1B planned)."),

    h3("Key Capabilities"),
    table(
      ["Module Family", "Description", "Key Components"],
      [
        ["Compliance", "Multi-dimensional compliance representation", "Tensor V2, Manifold, ZK proofs"],
        ["Corridors", "Inter-jurisdiction relationships", "State sync, bridge protocol, multilateral"],
        ["Governance", "Zone governance structures", "Constitutional frameworks, voting"],
        ["Financial", "Banking and payment infrastructure", "Accounts, payments, custody, FX"],
        ["Regulatory", "Compliance frameworks", "KYC, AML, sanctions, reporting"],
        ["Licensing", "Business authorization", "Applications, monitoring, portability"],
        ["Legal", "Legal services infrastructure", "Contracts, disputes, arbitration"],
        ["Operational", "Administrative functionality", "HR, procurement, facilities"],
        ["Corporate", "Corporate service provider lifecycle", "Formation, cap table, dissolution"],
        ["Identity", "Identity and credentialing", "DID management, progressive KYC"],
        ["Tax", "Tax and revenue management", "Regimes, fees, incentives"],
        ["Capital Markets", "Securities infrastructure", "Issuance, trading, clearing, settlement"],
        ["Trade", "Trade and commerce", "LCs, documents, supply chain finance"],
        ["Settlement", "ZK-native L1 settlement", "MASS Protocol, Plonky3 proofs"],
        ["Migration", "Cross-jurisdictional asset movement", "Saga orchestration, compensation"],
        ["Watcher", "Attestation economy", "Bonds, slashing, reputation"],
      ],
      [2400, 3600, 3360]
    ),

    h3("Version 0.4.44 Highlights"),
    table(
      ["Component", "Scope", "Key Features"],
      [
        ["Licensepacks", "900+ lines", "Pack Trilogy completion, license verification, compliance tensor integration"],
        ["Composition Engine", "650+ lines", "Multi-jurisdiction composition, 20 domains, AI arbitration support"],
        ["Corporate Services", "8 modules", "Formation, beneficial ownership, cap table, secretarial, dissolution"],
        ["Identity Module", "5 modules", "DID management, 4-tier KYC, verifiable credentials, binding"],
        ["Tax Module", "5 modules", "Tax regimes, fee schedules, incentive programs, CRS/FATCA"],
        ["Capital Markets", "9 modules", "Securities, trading, clearing, CSD, DVP/PVP settlement"],
        ["Trade Module", "6 modules", "Letters of credit, trade documents, supply chain finance"],
        ["Docker Infrastructure", "373+ lines", "12-service orchestration, database initialization"],
        ["AWS Terraform", "1,250+ lines", "VPC, EKS, RDS, Kubernetes resources"],
        ["Rust Migration", "Full codebase", "2024 edition, tokio async, serde serialization, zero unsafe"],
        ["Mass/MSEZ Bridge", "New crate", "JurisdictionalContext trait, five-primitive mapping"],
        ["GovOS Architecture", "New Part", "Four-layer model, Sovereign AI, Pakistan reference"],
      ],
      [2400, 1800, 5160]
    ),
    p("The complete v0.4.44 implementation comprises sixteen module families totaling 298 modules across approximately 109,000 lines of production Rust code (257 source files, 17 crates), with over 5,000 tests covering all critical paths."),

    h3("Document Organization"),

    p("This specification is organized into eighteen Parts that progress from foundational concepts through cryptographic infrastructure, compliance and governance systems, cross-border corridors, institutional modules, sovereign deployment architecture, security hardening, and operational deployment. Each Part is self-contained enough to be read independently by a domain specialist, but the Parts build on each other in a deliberate sequence: the cryptographic primitives of Part II underpin the compliance tensor of Part V, which feeds the corridor systems of Part IX, which rely on the watcher economy of Part X for attestation finality. The following table maps each Part to its constituent chapters and scope."),

    table(
      ["Part", "Title", "Chapters", "Scope"],
      [
        ["I", "Foundation", "1\u20132", "Mission, vision, programmable institution thesis, high-level architecture, system boundary between Mass and the SEZ Stack"],
        ["II", "Cryptographic Primitives", "3", "Ed25519 signing, SHA-256 canonical digests, Merkle Mountain Range accumulators, content-addressed storage, key zeroization"],
        ["III", "Content-Addressed Artifact Model", "4", "Artifact lifecycle, immutable content addressing, digest-based identity, CAS storage semantics"],
        ["IV", "Core Components", "5\u20137", "Module specifications, the Pack Trilogy (lawpacks, regpacks, licensepacks), Akoma Ntoso legal markup, profile system for zone configuration"],
        ["V", "Smart Asset Execution Layer", "8\u201312", "Smart asset model, receipt chain architecture, Compliance Tensor V2 (20 domains \u00D7 N jurisdictions), SAVM, multi-jurisdiction composition engine"],
        ["VI", "Mass L1 Settlement", "13\u201316", "ZK-native blockchain architecture, Plonky3 proving system, privacy architecture with BBS+ selective disclosure, L1 anchoring protocol"],
        ["VII", "Governance and Civic Systems", "17\u201318", "Constitutional frameworks for zone governance, voting mechanisms, civic services integration with national registries"],
        ["VIII", "Compliance and Regulatory", "19\u201321", "Compliance architecture, compliance manifold for multi-domain evaluation, zkKYC and privacy-preserving compliance"],
        ["IX", "Cryptographic Corridor Systems", "22\u201325", "Corridor architecture, bridge protocol, multilateral corridor composition, live corridor specifications (PAK\u2194UAE, PAK\u2194KSA, PAK\u2194CHN)"],
        ["X", "Watcher Economy", "26\u201328", "Watcher architecture, bond and slashing mechanics, quorum formation, attestation finality"],
        ["XI", "Migration Protocol", "29\u201331", "Cross-jurisdictional entity migration, eight-phase migration state machine, compensation and recovery mechanisms"],
        ["XII", "Institutional Infrastructure", "32\u201336", "Corporate services (formation through dissolution), identity and credentialing, tax and revenue, capital markets (issuance through settlement), trade and commerce"],
        ["XIII", "Mass API Integration", "37", "The msez-mass-bridge crate, JurisdictionalContext trait, typed client mapping to all five Mass primitives"],
        ["XIV", "GovOS Architecture", "38\u201341", "Four-layer sovereign deployment model, Sovereign AI spine, tax collection pipeline, sovereignty handover protocol"],
        ["XV", "Protocol Reference", "42\u201345", "Protocol overview, verifiable credentials (W3C VC, Ed25519 proofs), arbitration system, agentic execution framework"],
        ["XVI", "Security and Hardening", "46\u201348", "Security architecture, production hardening checklist, zero-knowledge proof circuit specifications"],
        ["XVII", "Deployment and Operations", "49\u201353", "Deployment architecture, Docker infrastructure (12-service orchestration), AWS Terraform (VPC, EKS, RDS), one-click deployment, operations management"],
        ["XVIII", "Network Diffusion", "54\u201356", "Adoption strategy, partner network, current network topology and status"],
      ],
      [540, 1800, 1080, 5940]
    ),

    p("Eleven appendices follow the main body: version history (A), test coverage summary (B), scalability switch reference (C), security proofs (D), Rust crate dependency graph (E), Mass API endpoint reference (F), jurisdiction template reference (G), CLI reference (H), module directory structure (I), conformance levels (J), and GovOS deployment checklist (K). The appendices serve as operational reference material; they are not required reading for understanding the architecture but are essential for implementors and auditors."),

    h3("Reading Guide"),

    p("This specification serves multiple audiences. Rather than reading all 56 chapters sequentially, each audience should follow the path most relevant to their concerns."),

    p_runs([
      bold("Regulators and policy officials "),
      "should begin with Chapters 1\u20132 (Foundation) for the programmable institution thesis, then proceed to Part VII (Constitutional Framework, Civic Services) to understand governance structures. Part VIII (Compliance Architecture, Compliance Manifold, zkKYC) explains how regulatory requirements are encoded and enforced. Part XIV (GovOS Architecture) describes the four-layer sovereign deployment model and the sovereignty handover protocol. Appendix K provides a concrete GovOS deployment checklist."
    ]),

    p_runs([
      bold("Systems engineers and Rust developers "),
      "should read Parts II\u2013III (Cryptographic Primitives, Artifact Model) for the foundational data structures, then Part IV (Modules, Pack Trilogy, Profiles) for the plugin architecture. Part V (Smart Asset Execution, Compliance Tensor, Composition Engine) covers the core computation model. Part VI (L1 Settlement, Proving System, Privacy) is essential for anyone working on the settlement layer. Part XVII (Deployment, Docker, Terraform) covers production deployment. Appendices E and F provide the crate dependency graph and API endpoint reference."
    ]),

    p_runs([
      bold("Zone operators and deployment teams "),
      "should start with Part IV, Chapter 7 (Profile System) to understand zone configuration, then move directly to Part XVII (Deployment Architecture, Docker, Terraform, One-Click Deployment, Operations Management). Part XII (Corporate Services, Identity, Tax, Capital Markets, Trade) documents the institutional modules they will configure for their jurisdiction. Part IX (Corridors) is relevant for operators establishing cross-border trade relationships."
    ]),

    p_runs([
      bold("Security auditors "),
      "should prioritize Part II (Cryptographic Primitives) for the signing and digest infrastructure, Part XVI (Security Architecture, Production Hardening, ZK Circuits) for threat models and mitigations, Part X (Watcher Economy, Bond and Slashing, Quorum) for the attestation trust model, and Appendix D (Security Proofs). The unwrap elimination protocol documented in the project\u2019s operational anchor (CLAUDE.md) governs production code safety invariants."
    ]),

    p_runs([
      bold("Compliance officers and legal counsel "),
      "should focus on Part VIII (Compliance Architecture, Manifold, zkKYC) for the compliance evaluation model, Part V Chapter 10 (Compliance Tensor V2) for the 20-domain compliance representation, Part IV Chapter 6 (Pack Trilogy) for how legislation and regulation are encoded as machine-readable packs, and Part XI (Migration Protocol) for cross-jurisdictional entity portability."
    ]),

    // pageBreak() removed — chapterHeading() now includes pageBreakBefore: true
  ];
};
