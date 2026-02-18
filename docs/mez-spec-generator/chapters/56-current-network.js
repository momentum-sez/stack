const {
  chapterHeading, h2, h3,
  p, p_runs, bold,
  table
} = require("../lib/primitives");

module.exports = function build_chapter56() {
  return [
    chapterHeading("Chapter 56: Current Network"),

    p("The Momentum network spans five jurisdictions at varying stages of deployment, connected by three bilateral trade corridors that collectively process $38.6B in annual trade volume. This chapter presents the current topology, aggregate metrics, growth trajectory, and planned network expansion."),

    table(
      ["Jurisdiction", "Status", "Profile", "Corridors"],
      [
        ["UAE / ADGM", "Live", "digital-financial-center", "PAK-UAE, KSA-UAE"],
        ["Dubai FZC (27 zones)", "Integration", "digital-financial-center", "PAK-UAE"],
        ["Pakistan", "Active", "sovereign-govos", "PAK-KSA, PAK-UAE, PAK-CHN"],
        ["Kazakhstan (Alatau City)", "Partnership", "digital-financial-center", "Planned"],
        ["Seychelles", "Deployment", "sovereign-govos", "Planned"],
      ],
      [2400, 1400, 3000, 2560]
    ),

    // --- 56.1 Network Topology ---
    h2("56.1 Network Topology"),
    p("The current network topology follows a hub-and-spoke pattern with Pakistan as the primary hub and the UAE as the secondary hub. Pakistan connects outward to three corridor endpoints (UAE, KSA, CHN), while the UAE serves as a regional financial center linking the Gulf economies. Kazakhstan and Seychelles are spoke nodes in the partnership and deployment phases respectively, with planned corridors that will create additional mesh connections as they come online."),

    h3("56.1.1 Jurisdiction Nodes"),
    p("Each jurisdiction operates as a node in the network, running a full EZ Stack deployment configured with jurisdiction-specific lawpacks, regpacks, and licensepacks. The node exposes the five Mass primitives (Entities, Ownership, Fiscal, Identity, Consent) through the EZ Stack orchestration layer, which evaluates the compliance tensor (§10) across all 20 ComplianceDomain variants (§10.1) for every operation. Nodes communicate through corridor channels that enforce bilateral compliance requirements, settlement netting, and mutual credential recognition."),
    table(
      ["Node", "Role", "Pack Configuration", "Mass Integration"],
      [
        ["Pakistan", "Primary Hub", "ITO 2001, Sales Tax Act 1990, FBR SROs, SBP regulations", "Full: organization-info, treasury-info, consent, identity"],
        ["UAE / ADGM", "Secondary Hub", "ADGM Companies Regs 2020, FSRA framework, AML/CFT rules", "Full: organization-info, treasury-info, consent, identity"],
        ["Dubai FZC", "Zone Cluster", "27 zone-specific regulatory packages, DFZC umbrella regulations", "Full: organization-info, treasury-info, consent"],
        ["Kazakhstan", "Spoke (Partnership)", "AIFC Constitutional Statute, AIFC regulations (in progress)", "Planned: organization-info, treasury-info"],
        ["Seychelles", "Spoke (Deployment)", "IBC Act 2016, FSA licensing framework", "Planned: organization-info, treasury-info"],
      ],
      [1600, 1600, 3560, 2600]
    ),

    h3("56.1.2 Corridor Edges"),
    p("Corridors form the edges of the network graph. Each corridor is a bidirectional channel between two jurisdiction nodes that carries trade transactions, settlement instructions, credential exchanges, and compliance attestations. The three active corridors connect Pakistan's $38.6B in bilateral trade with its three largest trading partners. Corridor edges are weighted by annual trade volume, which determines settlement netting frequency, liquidity reserve requirements, and compliance evaluation priority."),
    table(
      ["Corridor", "Direction", "Annual Volume", "Status", "Primary Functions"],
      [
        ["PAK\u2194UAE", "Bidirectional", "$10.1B", "Live", "Free zone integration, remittance processing, SIFC FDI pipeline"],
        ["PAK\u2194KSA", "Bidirectional", "$5.4B", "Launch", "Customs automation, diaspora services, remittance WHT"],
        ["PAK\u2194CHN", "Bidirectional", "$23.1B", "Planned", "CPEC 2.0 EZ coordination, e-trade, CNY/PKR settlement"],
      ],
      [1400, 1400, 1400, 1000, 4160]
    ),

    // --- 56.2 Aggregate Metrics ---
    h2("56.2 Aggregate Metrics"),
    p("The following table presents the current aggregate metrics across all network nodes and corridors as of February 2026. These metrics reflect cumulative totals from the live UAE deployment, active Pakistan operations, and the integration and partnership phases of the remaining jurisdictions."),
    table(
      ["Metric", "Value", "Description"],
      [
        ["Entities Onboarded", "1,000+", "Total entities formed and managed across all jurisdictions via Mass organization-info API"],
        ["Capital Processed", "$1.7B+", "Cumulative capital processed through Mass treasury-info API across all fiscal operations"],
        ["Active Jurisdictions", "5", "Jurisdictions in live, active, integration, partnership, or deployment status"],
        ["Bilateral Corridors", "3", "Operational or planned corridor connections between jurisdiction pairs"],
        ["Combined Corridor Volume", "$38.6B", "Total annual bilateral trade volume across all three corridors (PAK-UAE $10.1B + PAK-KSA $5.4B + PAK-CHN $23.1B)"],
        ["Module Families", "16", "Distinct module families in the EZ Stack covering compliance, crypto, corridors, credentials, and orchestration"],
        ["Total Modules", "298", "Individual modules across all 16 module families in the workspace"],
        ["Test Coverage", "3,800+ tests, 100%", "Total test count (#[test] + #[tokio::test]) across the 16-crate Rust workspace with full pass rate"],
      ],
      [2200, 1800, 5360]
    ),

    p_runs([
      bold("Infrastructure scale. "),
      "The 16 module families decompose into the following crate structure: mez-core (MCF canonical digest, compliance domains, sovereignty enforcement), mez-crypto (Ed25519, MMR, CAS, SHA-256), mez-vc (W3C Verifiable Credentials), mez-tensor (compliance tensor, manifold), mez-pack (lawpacks, regpacks, licensepacks, composition engine), mez-corridor (dual-commitment receipt chains, fork resolution, netting, payment rails), mez-state (FSM, migration saga, watcher economy), mez-agentic (trigger taxonomy, policy evaluation, tax pipeline), mez-arbitration (disputes, evidence, escrow), mez-schema (116 JSON schemas, Draft 2020-12), mez-zkp (proof system trait, production policy, 5 circuit modules), mez-compliance (jurisdiction-aware evaluators), mez-mass-client (typed HTTP client for all five Mass primitives, NADRA adapter), mez-api (Axum HTTP server with Postgres persistence, 10 route groups), and mez-cli (validation, lockfiles, corridor lifecycle, signing). Each crate maintains its own test suite contributing to the aggregate 3,800+ test count."
    ]),

    // --- 56.3 Growth Trajectory ---
    h2("56.3 Growth Trajectory"),
    p("Network growth follows a quarterly deployment cadence with each phase adding new jurisdictions, corridors, and compliance capabilities. The trajectory is designed to compound network effects: each new jurisdiction increases corridor possibilities combinatorially, and each new corridor validates the EZ Stack against a distinct regulatory and trade environment."),

    h3("56.3.1 Quarterly Deployment Milestones"),
    table(
      ["Quarter", "Milestone", "Jurisdictions", "Corridors", "Key Deliverables"],
      [
        ["Q1 2025", "Foundation", "2 (UAE, PAK)", "1 (PAK-UAE)", "Mass API integration, initial lawpacks, PAK-UAE corridor live"],
        ["Q2 2025", "Expansion", "3 (+KSA)", "2 (+PAK-KSA)", "KSA lawpack, SMDA framework integration, remittance WHT automation"],
        ["Q3 2025", "Scale", "4 (+KAZ)", "2", "AIFC partnership, Kazakhstan lawpack development, Alatau City deployment"],
        ["Q4 2025", "Consolidation", "5 (+SYC)", "2", "Seychelles GovOS deployment, IBC Act 2016 lawpack, offshore corridor planning"],
        ["Q1 2026", "Corridor Depth", "5", "3 (+PAK-CHN)", "CPEC 2.0 integration, CNY/PKR settlement, 9-SEZ coordination"],
        ["Q2 2026", "Network Mesh", "7 (+TUR, MYS)", "5", "Turkey and Malaysia onboarding, new bilateral corridors, multilateral netting"],
        ["Q3 2026", "Multilateral", "8 (+KSA)", "7", "Saudi Arabia full deployment, triangular netting, GCC corridor network"],
        ["Q4 2026", "Scale-Out", "10 (+NGA, +1)", "10+", "Africa expansion, 10+ corridors, full multilateral settlement mesh"],
      ],
      [1000, 1400, 1800, 1600, 3560]
    ),

    h3("56.3.2 Growth Drivers"),
    p("Three factors drive network growth acceleration. First, the pack trilogy architecture (lawpacks, regpacks, licensepacks) reduces new jurisdiction onboarding from months to weeks once the regulatory content is digitized into Akoma Ntoso format. Second, the compliance tensor (§10) generalizes across jurisdictions with jurisdiction-specific weightings. Third, corridor infrastructure is symmetric and composable: the same receipt chain, netting, and settlement mechanisms that power PAK-UAE apply to any new bilateral, reducing corridor deployment to configuration rather than custom development."),

    // --- 56.4 Future Network ---
    h2("56.4 Future Network"),
    p("The planned network expansion targets four additional jurisdictions: Turkey, Malaysia, Saudi Arabia (full sovereign deployment beyond the current corridor partnership), and Nigeria. These jurisdictions were selected based on three criteria: bilateral trade volume with existing network nodes, regulatory readiness for digital EZ infrastructure, and strategic importance for corridor mesh density."),

    h3("56.4.1 Turkey"),
    p_runs([
      bold("Profile: "),
      "Emerging digital-financial-center with strong manufacturing and trade corridor potential. Turkey's bilateral trade with Pakistan ($1.2B), UAE ($18B), and Saudi Arabia ($11B) creates immediate corridor opportunities with three existing network nodes. The Istanbul Finance Center (IFC) initiative and Turkey's free zone modernization program provide regulatory alignment for EZ Stack deployment. Key lawpack requirements include Turkish Commercial Code No. 6102, Investment Incentive System regulations, and BRSA banking supervision framework. The Turkey deployment will also pilot integration with the Turkish Central Bank's digital currency infrastructure and FAST instant payment system."
    ]),

    h3("56.4.2 Malaysia"),
    p_runs([
      bold("Profile: "),
      "Islamic finance hub and ASEAN gateway. Malaysia's Labuan International Business and Financial Centre (IBFC) is a natural deployment target, offering an established offshore financial center with strong Islamic finance credentials. Bilateral trade with Pakistan ($3.2B), UAE ($8.4B), and China ($98B) provides substantial corridor volume. Malaysia's position in RCEP and CPTPP trade agreements introduces multilateral compliance requirements that will stress-test the tensor architecture. Key lawpack requirements include Labuan Business Activity Tax Act 1990, LFSA licensing framework, and Bank Negara Malaysia AML/CFT guidelines. The Malaysia deployment is strategically important as the first ASEAN node, opening the path to Singapore, Indonesia, and Thailand expansion."
    ]),

    h3("56.4.3 Saudi Arabia (Full Sovereign)"),
    p_runs([
      bold("Profile: "),
      "Full sovereign GovOS deployment extending beyond the current PAK-KSA corridor partnership. The Saudi Vision 2030 economic diversification program has created 15 new special economic zones, including NEOM, King Abdullah Economic City, and Jazan City for Primary and Downstream Industries. A full sovereign deployment would integrate the EZ Stack with Saudi Arabia's National Single Sign-On (Nafath), Absher government services platform, and ZATCA tax authority systems. Key lawpack requirements include the Saudi Companies Law 2022, ZATCA VAT regulations, CMA Capital Markets regulations, and SAGIA foreign investment licensing rules. This deployment would transform Saudi Arabia from a corridor endpoint to a full network hub, enabling GCC-wide corridor connectivity."
    ]),

    h3("56.4.4 Nigeria"),
    p_runs([
      bold("Profile: "),
      "Africa's largest economy and the anchor node for the continent. Nigeria's Lekki Free Zone, Calabar Free Trade Zone, and newly designated digital economy zones provide deployment targets. Bilateral trade with UAE ($5.1B), China ($23B), and Saudi Arabia ($3.8B) creates corridors with three existing network nodes. Nigeria's recent fintech regulatory framework (CBN licensing), Securities and Exchange Commission digital asset rules, and NIPC investment promotion infrastructure provide regulatory readiness. Key lawpack requirements include NEPZA Act, CAMA 2020, FIRS tax regulations, and CBN foreign exchange guidelines. The Nigeria deployment is strategically critical as the gateway to the broader African Continental Free Trade Area (AfCFTA), which connects 54 countries with a combined GDP of $3.4T."
    ]),

    h3("56.4.5 Future Network Summary"),
    table(
      ["Jurisdiction", "Target Quarter", "Profile", "Key Corridors", "Strategic Value"],
      [
        ["Turkey", "Q2 2026", "digital-financial-center", "TUR-UAE, TUR-PAK, TUR-KSA", "Manufacturing corridor, IFC alignment, FAST payments"],
        ["Malaysia", "Q2 2026", "islamic-finance-hub", "MYS-UAE, MYS-CHN, MYS-PAK", "ASEAN gateway, Islamic finance, RCEP/CPTPP compliance"],
        ["Saudi Arabia (Full)", "Q3 2026", "sovereign-govos", "KSA-UAE, KSA-PAK, KSA-TUR, KSA-NGA", "GCC hub, Vision 2030, 15 new SEZs"],
        ["Nigeria", "Q4 2026", "sovereign-govos", "NGA-UAE, NGA-CHN, NGA-KSA", "Africa anchor, AfCFTA gateway, $3.4T continental market"],
      ],
      [1600, 1200, 1800, 2800, 1960]
    ),

    p("Upon completion of the Q4 2026 expansion, the Momentum network will span 10 jurisdictions across four continents (Asia, Middle East, Africa, Central Asia) with 10 or more bilateral corridors forming a dense settlement mesh. The network will cover combined bilateral trade volumes exceeding $200B annually, with multilateral netting reducing settlement friction across the entire mesh. Each jurisdiction will operate a full EZ Stack deployment with jurisdiction-specific pack configurations, connected through the corridor network with mutual credential recognition and cross-border compliance tensor evaluation."),
  ];
};
