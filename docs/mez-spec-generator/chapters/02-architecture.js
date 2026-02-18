const {
  chapterHeading, h2, h3, p, p_runs, bold,
  table, codeBlock, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter02() {
  return [
    pageBreak(),

    chapterHeading("Chapter 2: Architecture Overview"),

    p("The EZ Stack is organized as a layered architecture where each layer has a well-defined responsibility and interacts only with adjacent layers. This section provides the structural overview; subsequent chapters specify each component in detail."),

    // --- 2.1 Layer Model ---
    h2("2.1 Layer Model"),

    p("The Stack comprises six layers, from physical infrastructure at the bottom to user-facing applications at the top."),

    table(
      ["Layer", "Name", "Function", "Implementation"],
      [
        ["L0", "Infrastructure", "Compute, storage, networking, HSMs", "AWS/GCP/bare metal, Terraform"],
        ["L1", "Settlement", "Cryptographic finality, state roots, ZK proofs", "Mass Protocol (Groth16 + Plonk)"],
        ["L2", "Primitives", "Five programmable primitives (entity, ownership, fiscal, identity, consent)", "Mass APIs (Java/Spring Boot)"],
        ["L3", "Jurisdiction", "Compliance tensor, pack trilogy, corridors, credentials", "MEZ Stack (Rust)"],
        ["L4", "Orchestration", "Workflow composition, agentic triggers, saga coordination", "MEZ Stack (Rust)"],
        ["L5", "Application", "GovOS console, developer APIs, reporting dashboards", "Web applications"],
      ],
      [800, 2000, 3760, 2800]
    ),

    p("Layers L0\u2013L2 are provided by Mass and its infrastructure. Layers L3\u2013L4 are the MEZ Stack \u2014 the subject of this specification. Layer L5 is built by deployment teams using the Stack\u2019s APIs."),

    // --- 2.2 Module Architecture ---
    h2("2.2 Module Architecture"),

    p("The MEZ Stack is organized into sixteen module families, each addressing a distinct domain of EZ governance. These families compose through well-defined interfaces; no family directly accesses another family\u2019s internal state."),

    table(
      ["Family", "Modules", "Purpose"],
      [
        ["Compliance", "Tensor V2, Manifold, ZK Compliance", "Multi-dimensional compliance state representation and evaluation"],
        ["Corridors", "Bilateral, Multilateral, Bridge, Sync", "Cross-jurisdictional relationships and trade corridors"],
        ["Governance", "Constitution, Voting, Amendment, Council", "Zone governance structures and decision-making processes"],
        ["Financial", "Accounts, Payments, Custody, FX, Treasury", "Banking and payment infrastructure within the zone"],
        ["Regulatory", "KYC, AML, Sanctions, Reporting, Filing", "Compliance frameworks and regulatory reporting"],
        ["Licensing", "Application, Issuance, Monitoring, Renewal, Portability", "Business authorization lifecycle"],
        ["Legal", "Contracts, Disputes, Arbitration, Enforcement", "Legal services infrastructure"],
        ["Operational", "HR, Procurement, Facilities, Utilities", "Administrative and operational functionality"],
        ["Corporate", "Formation, Ownership, Secretary, Dissolution", "Corporate service provider lifecycle"],
        ["Identity", "DID, KYC Tiers, Credentials, Binding, Recovery", "Identity and credentialing infrastructure"],
        ["Tax", "Regimes, Fees, Incentives, Reporting, CRS/FATCA", "Tax and revenue management"],
        ["Capital Markets", "Issuance, Trading, Clearing, CSD, Settlement", "Securities infrastructure"],
        ["Trade", "LC, Documents, Supply Chain Finance, Insurance", "Trade and commerce infrastructure"],
      ],
      [2600, 3200, 3560]
    ),

    // --- 2.2.1 PHOENIX Module Suite ---
    h3("2.2.1 PHOENIX Module Suite"),

    p("The Rust implementation is organized as a Cargo workspace with the following crates, each mapping to a specific domain within the Stack architecture."),

    table(
      ["Crate", "Lines", "Purpose"],
      [
        ["mez-core", "~3,300", "Foundation: Momentum Canonical Form (MCF) digest, ComplianceDomain (20 variants), sovereignty enforcement, identifier newtypes, timestamps"],
        ["mez-crypto", "~3,300", "Cryptography: Ed25519 signing/verification, SHA-256, MMR, CAS, Poseidon2 (feature-gated), BBS+ (feature-gated), key zeroization"],
        ["mez-vc", "~2,200", "W3C Verifiable Credentials: Ed25519 JCS proofs, credential registry, proof validation"],
        ["mez-tensor", "~3,500", "Compliance Tensor: 20 domains \u00d7 N jurisdictions, Compliance Manifold, Dijkstra path optimization, tensor commitments"],
        ["mez-pack", "~9,700", "Pack Trilogy: lawpacks (Akoma Ntoso), regpacks (sanctions, calendars), licensepacks (Pakistan reference data), composition engine"],
        ["mez-corridor", "~5,800", "Corridor operations: dual-commitment receipt chains, evidence-driven fork resolution, netting, SWIFT pacs.008, payment rails, bridge routing"],
        ["mez-state", "~4,400", "State machines: corridor FSM, entity lifecycle, license lifecycle, migration saga, watcher economy (bonds, slashing)"],
        ["mez-agentic", "~5,500", "Automation: trigger taxonomy, policy evaluation, audit trails, tax collection pipeline, scheduling"],
        ["mez-arbitration", "~5,200", "Disputes: dispute lifecycle, evidence packages, ruling enforcement, escrow management"],
        ["mez-mass-client", "~3,800", "Typed HTTP client for all five Mass primitives, NADRA identity adapter, retry logic, contract tests"],
        ["mez-schema", "~1,800", "JSON Schema Draft 2020-12 validation: 116 schemas, $ref resolution, codegen policy"],
        ["mez-zkp", "~3,000", "Zero-knowledge proofs: sealed ProofSystem trait, 5 circuit modules, Groth16 + Plonk backends, production policy enforcement"],
        ["mez-compliance", "~500", "Compliance orchestration: jurisdiction-aware evaluators, compliance evaluation composition"],
        ["mez-api", "~17,100", "HTTP server: Axum routes, orchestration layer, identity/tax/govos/settlement/agentic endpoints, Postgres persistence, auth + rate limiting middleware"],
        ["mez-cli", "~4,800", "CLI: zone validation, lockfile generation, corridor lifecycle, artifact CAS, Ed25519/VC signing"],
      ],
      [3000, 1200, 5160]
    ),

    // --- 2.2.2 Rust Workspace Structure ---
    h3("2.2.2 Rust Workspace Structure"),

    p("The workspace is structured as a flat collection of crates with explicit dependency declarations. The dependency graph is acyclic, with mez-core at the root and mez-api as the composition point."),

    ...codeBlock(
`momentum-ez/stack/
\u251c\u2500\u2500 mez/                       # Rust workspace root
\u2502   \u251c\u2500\u2500 Cargo.toml              # Workspace manifest (16 crates)
\u2502   \u2514\u2500\u2500 crates/
\u2502       \u251c\u2500\u2500 mez-core/          # Foundation (zero internal deps)
\u2502       \u251c\u2500\u2500 mez-crypto/        # Cryptographic primitives
\u2502       \u251c\u2500\u2500 mez-vc/            # Verifiable Credentials
\u2502       \u251c\u2500\u2500 mez-state/         # State Machines
\u2502       \u251c\u2500\u2500 mez-tensor/        # Compliance Tensor
\u2502       \u251c\u2500\u2500 mez-zkp/           # Zero-Knowledge Proofs
\u2502       \u251c\u2500\u2500 mez-pack/          # Pack Trilogy
\u2502       \u251c\u2500\u2500 mez-corridor/      # Corridor Operations
\u2502       \u251c\u2500\u2500 mez-agentic/       # Agentic Automation
\u2502       \u251c\u2500\u2500 mez-arbitration/   # Dispute Resolution
\u2502       \u251c\u2500\u2500 mez-compliance/    # Compliance Orchestration
\u2502       \u251c\u2500\u2500 mez-schema/        # JSON Schema Validation
\u2502       \u251c\u2500\u2500 mez-mass-client/   # Mass API Client
\u2502       \u251c\u2500\u2500 mez-api/           # Axum HTTP Server
\u2502       \u251c\u2500\u2500 mez-cli/           # Command-Line Interface
\u2502       \u2514\u2500\u2500 mez-integration-tests/  # Cross-crate test suites
\u251c\u2500\u2500 schemas/                    # 116 JSON Schema files (Draft 2020-12)
\u251c\u2500\u2500 modules/                    # 298 module descriptors (16 families)
\u251c\u2500\u2500 apis/                       # OpenAPI specifications
\u251c\u2500\u2500 deploy/                     # Docker Compose + AWS Terraform
\u2514\u2500\u2500 docs/                       # Specifications and documentation`
    ),

    // --- 2.3 Live Deployments ---
    h2("2.3 Live Deployments"),

    p("The architecture is validated by production deployments across multiple jurisdictions and deployment models."),

    table(
      ["Deployment", "Status", "Evidence"],
      [
        ["Pakistan GovOS (PDA)", "Active", "Full government OS: 40+ ministries, FBR tax integration, SBP Raast payments, NADRA identity, SECP corporate registry."],
        ["UAE / ADGM", "Live", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
        ["Dubai Free Zone Council", "Integration", "27 free zones. Mass APIs serve entity + fiscal; MEZ provides zone-specific licensing."],
        ["Kazakhstan (Alatau City)", "Partnership", "SEZ + AIFC integration. Composition engine: Kazakh law + AIFC financial regulation."],
        ["Seychelles", "Deployment", "Sovereign GovOS at national scale."],
        ["PAK \u2194 KSA Corridor", "Active", "$5.4B bilateral trade. Receipt chain sync with SBP + SAMA."],
        ["PAK \u2194 UAE Corridor", "Active", "$10.1B bilateral trade. SWIFT pacs.008 adapter for cross-border payments."],
        ["PAK \u2194 CHN Corridor", "Planning", "$23.1B bilateral trade. CPEC integration with SAFE compliance."],
      ],
      [2400, 1200, 5760]
    ),
  ];
};
