const {
  chapterHeading, h2, h3, p, p_runs, bold,
  table, codeBlock, spacer, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter02() {
  return [
    // No pageBreak() needed here — chapterHeading() has pageBreakBefore: true built in.
    chapterHeading("Chapter 2: Architecture Overview"),

    p("The SEZ Stack is organized as a layered architecture where each layer has a well-defined responsibility and interacts only with adjacent layers. This section provides the structural overview; subsequent chapters specify each component in detail."),

    // --- 2.1 Layer Model ---
    h2("2.1 Layer Model"),

    p("The Stack comprises six layers, from physical infrastructure at the bottom to user-facing applications at the top."),

    table(
      ["Layer", "Name", "Function", "Implementation"],
      [
        ["L0", "Infrastructure", "Compute, storage, networking, HSMs", "AWS/GCP/bare metal, Terraform"],
        ["L1", "Settlement", "Cryptographic finality, state roots, ZK proofs", "Mass Protocol (Plonky3)"],
        ["L2", "Primitives", "Five programmable primitives (entity, ownership, fiscal, identity, consent)", "Mass APIs (Java/Spring Boot)"],
        ["L3", "Jurisdiction", "Compliance tensor, pack trilogy, corridors, credentials", "MSEZ Stack (Rust)"],
        ["L4", "Orchestration", "Workflow composition, agentic triggers, saga coordination", "MSEZ Stack (Rust)"],
        ["L5", "Application", "GovOS console, developer APIs, reporting dashboards", "Web applications"],
      ],
      [800, 2000, 3760, 2800]
    ),

    p("Layers L0\u2013L2 are provided by Mass and its infrastructure. Layers L3\u2013L4 are the MSEZ Stack \u2014 the subject of this specification. Layer L5 is built by deployment teams using the Stack\u2019s APIs."),

    // --- 2.2 Module Architecture ---
    h2("2.2 Module Architecture"),

    p("The MSEZ Stack is organized into sixteen module families, each addressing a distinct domain of SEZ governance. These families compose through well-defined interfaces; no family directly accesses another family\u2019s internal state."),

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
        ["msez-core", "~3,200", "Foundation: canonical digest, ComplianceDomain (20 variants), identifier newtypes, error hierarchy, timestamps"],
        ["msez-crypto", "~2,800", "Cryptography: Ed25519 signing/verification, MMR, CAS, key zeroization"],
        ["msez-vc", "~1,900", "W3C Verifiable Credentials: Ed25519 proofs, BBS+ selective disclosure"],
        ["msez-tensor", "~4,100", "Compliance Tensor V2: 20 domains \u00d7 N jurisdictions, Compliance Manifold, path optimization"],
        ["msez-pack", "~5,500", "Pack Trilogy: lawpacks (Akoma Ntoso), regpacks (sanctions, calendars), licensepacks (live registries)"],
        ["msez-corridor", "~3,600", "Corridor lifecycle: receipt chains, fork resolution, netting, SWIFT pacs.008 adapter"],
        ["msez-state", "~2,400", "State machines: corridor FSM, migration saga (8 phases), watcher economy (bonds, slashing)"],
        ["msez-agentic", "~1,800", "Automation: trigger taxonomy (20 types \u00d7 5 domains), policy evaluation, autonomous actions"],
        ["msez-arbitration", "~1,500", "Disputes: evidence packages, ruling enforcement via VCs, escrow"],
        ["msez-mass-client", "~2,100", "Typed HTTP client for all five Mass primitives. Sole authorized path to Mass APIs."],
        ["msez-schema", "~1,200", "JSON Schema validation: 116 schemas covering all artifact types, pack formats, and API payloads"],
        ["msez-zkp", "~3,400", "Zero-knowledge proofs: sealed ProofSystem trait, 12 circuit definitions, Plonky3 + Groth16 backends"],
        ["msez-compliance", "~1,100", "Compliance orchestration: composes tensor evaluation with pack checking and credential issuance"],
        ["msez-api", "~4,800", "HTTP server: Axum routes, mass proxy, corridor/asset/regulator/agentic/settlement endpoints, Postgres persistence"],
        ["msez-cli", "~2,600", "CLI: zone validation, build, sign, verify, artifact graph operations, deployment commands"],
      ],
      [3000, 1200, 5160]
    ),

    // --- 2.2.2 Rust Workspace Structure ---
    h3("2.2.2 Rust Workspace Structure"),

    p("The workspace is structured as a flat collection of crates with explicit dependency declarations. The dependency graph is acyclic, with msez-core at the root and msez-api as the composition point."),

    ...codeBlock(
`momentum-sez/stack/
\u251c\u2500\u2500 Cargo.toml              # Workspace root
\u251c\u2500\u2500 msez-core/              # Foundation (zero internal deps)
\u251c\u2500\u2500 msez-crypto/            # Cryptographic primitives
\u251c\u2500\u2500 msez-vc/                # Verifiable Credentials
\u251c\u2500\u2500 msez-tensor/            # Compliance Tensor V2
\u251c\u2500\u2500 msez-pack/              # Pack Trilogy
\u251c\u2500\u2500 msez-corridor/          # Trade Corridors
\u251c\u2500\u2500 msez-state/             # State Machines
\u251c\u2500\u2500 msez-agentic/           # Agentic Automation
\u251c\u2500\u2500 msez-arbitration/       # Dispute Resolution
\u251c\u2500\u2500 msez-schema/            # JSON Schema Validation
\u251c\u2500\u2500 msez-zkp/               # Zero-Knowledge Proofs
\u251c\u2500\u2500 msez-compliance/        # Compliance Orchestration
\u251c\u2500\u2500 msez-mass-client/       # Mass API Client
\u251c\u2500\u2500 msez-api/               # Axum HTTP Server
\u251c\u2500\u2500 msez-cli/               # Command-Line Interface
\u2514\u2500\u2500 docs/                   # Specifications and documentation`
    ),

    // --- 2.3 Live Deployments ---
    h2("2.3 Live Deployments"),

    p("The architecture is validated by production deployments across multiple jurisdictions and deployment models."),

    table(
      ["Deployment", "Status", "Evidence"],
      [
        ["Pakistan GovOS (PDA)", "Active", "Full government OS: 40+ ministries, FBR tax integration, SBP Raast payments, NADRA identity, SECP corporate registry."],
        ["UAE / ADGM", "Live", "1,000+ entities onboarded, $1.7B+ capital processed via Northern Trust custody."],
        ["Dubai Free Zone Council", "Integration", "27 free zones. Mass APIs serve entity + fiscal; MSEZ provides zone-specific licensing."],
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
