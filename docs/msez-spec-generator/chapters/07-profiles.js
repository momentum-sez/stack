const {
  chapterHeading, h2, h3,
  p, definition, table, pageBreak
} = require("../lib/primitives");

module.exports = function build_chapter07() {
  return [
    chapterHeading("Chapter 7: Profile System"),
    p("Profiles are curated bundles of modules, parameters, and jurisdiction-specific configuration. They serve as deployment templates that encode institutional intent: what kind of economic zone is being created, what regulatory posture it assumes, and what operational capabilities it requires from day one. A profile is not merely a feature toggle list. It is a complete specification of the governance surface area a zone presents to its participants, regulators, and corridor counterparties."),
    p("Each profile selects from the 16 module families available in the MSEZ Stack, configures their parameters for the target use case, and establishes default compliance tensor weights appropriate to the zone's regulatory context. Profiles also determine infrastructure resource requirements, Pack Trilogy composition rules, and corridor eligibility constraints. A zone deployed with the wrong profile will either over-provision capabilities it does not need or, worse, under-provision compliance infrastructure that its jurisdiction requires."),

    // --- Overview Table ---
    h2("7.1 Profile Overview"),
    p("The MSEZ Stack ships seven canonical profiles. Each targets a distinct institutional archetype drawn from real-world special economic zone models. Operators may extend or compose these profiles, but the canonical seven represent the tested, audited configurations that Momentum supports for production deployment."),
    table(
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
    ),

    definition("Definition 7.1 (Profile).", "A profile P is a tuple (M, \u0398, T, R) where M is the set of active module families, \u0398 is the parameter configuration map, T is the compliance tensor weight matrix, and R is the resource requirement specification. A zone Z instantiated with profile P inherits all four components and may override \u0398 entries within the bounds declared by P's parameter constraints."),

    p("Module families and compliance domains are distinct concepts. Module families (16) represent functional capabilities deployed in a zone: corporate services, financial services, customs processing, and so on. Compliance domains (20, defined in \u00a710.2) represent regulatory dimensions evaluated by the compliance tensor. A single module family may trigger evaluation across multiple compliance domains (e.g., the Financial module family activates the Banking, Payments, Clearing, and Settlement domains), and a single compliance domain may be relevant to multiple module families. The matrices below use module family names; see \u00a710.2 for the canonical ComplianceDomain enum variants."),

    h3("7.1.1 Module Activation Matrix"),
    p("The following matrix summarizes module activation status across all seven profiles. Active: full functionality. Minimal: limited functionality for the specific use case. Inactive: module not deployed. Per-profile sections (\u00a77.2\u2013\u00a77.8) describe the configuration rationale for each activation decision."),
    table(
      ["Module Family", "financial-center", "trade-hub", "tech-park", "sovereign-govos", "charter-city", "digital-native", "asset-history"],
      [
        ["Corporate", "Active", "Active", "Active", "Active", "Active", "Active", "Minimal"],
        ["Financial", "Active", "Active", "Minimal", "Active", "Active", "Active", "Minimal"],
        ["Trade", "Active", "Active", "Inactive", "Active", "Minimal", "Inactive", "Active"],
        ["Corridors", "Active", "Active", "Minimal", "Active", "Active", "Active", "Active"],
        ["Governance", "Active", "Active", "Active", "Active", "Active", "Active", "Minimal"],
        ["Regulatory", "Active", "Active", "Active", "Active", "Active", "Active", "Active"],
        ["Licensing", "Active", "Active", "Active", "Active", "Active", "Active", "Minimal"],
        ["Legal", "Active", "Active", "Active", "Active", "Active", "Active", "Active"],
        ["Identity", "Active", "Active", "Active", "Active", "Active", "Active", "Active"],
        ["Compliance", "20 domains", "14 domains", "10 domains", "20 domains", "16 domains", "12 domains", "6-10 domains"],
        ["Tax", "Active", "Active", "Active", "Active", "Active", "Active", "Minimal"],
        ["Insurance", "Active", "Minimal", "Inactive", "Active", "Active", "Inactive", "Active"],
        ["IP", "Active", "Inactive", "Active", "Active", "Minimal", "Active", "Minimal"],
        ["Customs", "Active", "Active", "Inactive", "Active", "Minimal", "Inactive", "Active"],
        ["Land/Property", "Active", "Inactive", "Minimal", "Active", "Active", "Inactive", "Conditional"],
        ["Civic", "Active", "Minimal", "Active", "Active", "Active", "Inactive", "Inactive"],
      ],
      [1400, 1200, 1100, 1000, 1200, 1100, 1100, 1260]
    ),

    h3("7.1.2 Resource Comparison"),
    table(
      ["Resource", "financial-center", "trade-hub", "tech-park", "sovereign-govos", "charter-city", "digital-native", "asset-history"],
      [
        ["Min vCPU", "16", "8", "4", "64", "16", "4", "4"],
        ["Min RAM", "64 GB", "32 GB", "16 GB", "256 GB", "64 GB", "16 GB", "16 GB"],
        ["Min Storage", "500 GB", "250 GB", "100 GB", "5 TB", "500 GB", "100 GB", "100 GB"],
        ["HSM Required", "FIPS 140-2 L3", "Software OK", "Software OK", "FIPS 140-3 L3", "FIPS 140-2 L3", "Software OK", "Software OK"],
        ["DB Nodes", "3-node cluster", "Single node", "Single node", "5-node cluster", "3-node cluster", "Single node", "Single node"],
      ],
      [1400, 1200, 1100, 1000, 1200, 1100, 1100, 1260]
    ),

    // =========================================================================
    // 7.2 digital-financial-center
    // =========================================================================
    h2("7.2 digital-financial-center"),
    p("The digital-financial-center profile targets jurisdictions seeking to establish full-service financial zones comparable to the Abu Dhabi Global Market, Dubai International Financial Centre, or Singapore's financial district. It activates all 16 module families with maximum compliance depth, designed for zones that will host regulated financial institutions, capital markets, fund administration, insurance underwriting, and cross-border settlement."),

    h3("7.2.1 Deployed Capabilities"),
    p("This profile deploys the complete module set with financial-grade configuration. Corporate services include full entity lifecycle management with complex ownership structures, nominee arrangements, and regulated fund vehicles. Financial services include multi-currency account management, payment processing across SWIFT, SEPA, and local rails, custody services with multi-signature quorum policies, and capital markets infrastructure supporting primary issuance and secondary trading. The compliance surface is maximized: all 20 compliance domains are active, sanctions screening operates in real-time with sub-second latency requirements, and transaction monitoring applies jurisdiction-specific rule engines continuously."),
    p("Corridor capabilities are fully enabled. The profile supports unlimited bilateral and multilateral corridors with real-time settlement, netting, and fork resolution. Receipt chains operate at full fidelity with complete provenance tracking. The arbitration module provides formal dispute resolution with escrow, evidence packages, and ruling enforcement via Verifiable Credentials."),

    h3("7.2.2 Example Deployment: Abu Dhabi Digital Financial Centre"),
    p("A sovereign deploying the digital-financial-center profile for a new ADGM-class zone would configure the profile with UAE-specific lawpacks encoding Federal Decree-Law No. 32/2021 (Commercial Companies), ADGM Financial Services and Markets Regulations 2015, and the ADGM Insolvency Regulations. Regpacks would include UAE Central Bank prudential requirements, FATF mutual evaluation follow-up items, and the OFAC/EU/UN consolidated sanctions lists with daily synchronization. Licensepacks would encode ADGM FSRA license categories (Category 1 through Category 4), regulated activity permissions, and capital adequacy requirements per license type."),
    p("Corridor configuration would establish the UAE-GCC corridor (real-time settlement via UAESWITCH and GCC-RTGS), the UAE-India corridor (high-volume remittance with UPI integration), and the UAE-UK corridor (DIFC-London financial services passporting). Each corridor binds specific lawpack and regpack digests establishing the regulatory context at corridor creation time."),

    // =========================================================================
    // 7.3 trade-hub
    // =========================================================================
    pageBreak(),
    h2("7.3 trade-hub"),
    p("The trade-hub profile targets zones focused on international trade facilitation, logistics, and supply chain management. Modeled on successful trade zones such as Jebel Ali Free Zone, Singapore Free Trade Zones, and the Shenzhen Special Economic Zone, this profile prioritizes customs processing, trade finance, corridor management, and supply chain documentation over capital markets and complex financial instruments."),

    h3("7.3.1 Deployed Capabilities"),
    p("Trade-hub deployments activate corporate services for entity formation optimized for trading companies, freight forwarders, and logistics operators. Financial services are configured for trade finance instruments: letters of credit, documentary collections, trade receivables financing, and supply chain finance. The customs module operates at full depth with tariff classification, bonded warehousing, duty deferral schemes, and preferential origin determination under applicable free trade agreements."),
    p("Corridor capabilities focus on trade corridors with receipt chain tracking (\u00a79) for goods movement. The compliance tensor is configured with elevated weights for Trade, Sanctions, and Aml domains, reflecting the regulatory priorities of trade facilitation zones."),

    h3("7.3.2 Example Deployment: Pakistan-UAE Trade Corridor Zone"),
    p("A trade-hub deployment for a Pakistan-UAE bilateral trade zone would configure lawpacks encoding Pakistan's Customs Act 1969, Sales Tax Act 1990, and the Pakistan-UAE Bilateral Investment Treaty alongside UAE Federal Customs Law and the GCC Common Customs Tariff. Regpacks would include Pakistan Single Window integration parameters, CPEC preferential tariff schedules, and SBP foreign exchange regulations governing trade settlements in PKR, AED, and USD."),
    p("The primary corridor (PAK-UAE) would be configured for containerized cargo flows with receipt chains tracking bill of lading issuance, customs declaration filing, inspection clearance, bonded transit, and final delivery confirmation. A secondary corridor (PAK-UAE-KSA) would enable triangular trade routing through UAE free zones with appropriate re-export documentation and certificate of origin management under the GCC-Pakistan FTA framework."),

    // =========================================================================
    // 7.4 tech-park
    // =========================================================================
    pageBreak(),
    h2("7.4 tech-park"),
    p("The tech-park profile targets technology and innovation zones where the primary tenants are software companies, R&D laboratories, startups, and technology-focused enterprises. Modeled on zones such as Dubai Internet City, Bangalore's Electronic City, and the Hsinchu Science Park, this profile emphasizes rapid entity formation, intellectual property protection, lightweight financial services, and talent mobility. Capital markets, complex trade finance, and heavy customs infrastructure are deactivated to reduce operational overhead and compliance burden."),

    h3("7.4.1 Deployed Capabilities"),
    p("Tech-park deployments provide streamlined corporate formation with sub-24-hour entity registration, simplified share structures suitable for venture-backed startups, and SAFE/convertible instrument support integrated through the Mass Ownership primitive. Licensing is configured for technology-sector permits: software development, IT services, data processing, and telecommunications value-added services. The IP module operates at full depth with patent application tracking, trademark registration, trade secret protection, and technology transfer agreement management."),
    p("Financial services are configured in lightweight mode: operating accounts, payroll processing, and investor capital receipt. Capital markets functionality is disabled. Identity services emphasize talent credentials: professional qualifications, employment history verification, and cross-zone workforce portability. The compliance tensor activates 10 domains, omitting customs, insurance, land/property, trade-specific, and heavy financial regulation domains that are irrelevant to pure technology operations."),

    h3("7.4.2 Example Deployment: Islamabad Technology Park"),
    p("A tech-park deployment for a Pakistani technology zone would configure lawpacks encoding the relevant provisions of the Companies Act 2017 (simplified private limited formation), the Income Tax Ordinance 2001 (Section 100C IT exemptions for SEZ enterprises), and SECP regulatory requirements for technology companies. Regpacks would include PSEB (Pakistan Software Export Board) registration requirements, PTA licensing categories for IT and telecom services, and withholding tax exemptions applicable to IT export revenues under SRO 1371(I)/2022."),
    p("The zone would operate a single service-export corridor (PAK-GLOBAL) enabling technology service delivery to international clients with automatic withholding tax computation based on treaty status, PSEB certification verification, and foreign exchange receipt tracking through SBP-authorized dealer banks. Entity formation would target sub-4-hour registration leveraging the Mass Entities primitive with SECP integration, producing a formation Verifiable Credential that serves as portable proof of incorporation across all system participants."),

    // =========================================================================
    // 7.5 sovereign-govos
    // =========================================================================
    pageBreak(),
    h2("7.5 sovereign-govos"),
    p("The sovereign-govos profile transforms the Stack from a zone management system into a national operating system for government services. This profile activates all 16 module families at maximum depth, adds GovOS orchestration capabilities, and integrates with existing national government systems. It is not a zone profile in the traditional sense; it is the profile that enables a sovereign nation to operate its entire economic infrastructure through the MSEZ Stack and Mass primitives."),
    p("Pakistan serves as the reference implementation for sovereign-govos. The four-layer GovOS architecture (Experience, Platform Engine, Jurisdictional Configuration, National System Integration) described in Chapter 38 is the canonical deployment model for this profile. Every capability that exists in any other profile is active in sovereign-govos, plus national-scale capabilities that no other profile requires."),

    h3("7.5.1 Deployed Capabilities"),
    p("Sovereign-govos deploys the complete MSEZ Stack with national-scale extensions. Entity management covers the full lifecycle from company incorporation through SECP to dissolution, including beneficial ownership reporting, NTN binding with FBR, and provincial registration. Fiscal operations integrate with national payment rails: SBP Raast for instant payments, RTGS for large-value settlements, and commercial bank APIs for account management. Tax administration operates across all federal and provincial tax types with automatic assessment, withholding, filing, and reconciliation."),
    p("Identity services integrate with NADRA for CNIC verification, enabling zkKYC workflows that verify identity without exposing biometric data. Consent management provides multi-party governance for tax assessments, regulatory approvals, and inter-ministry coordination. The compliance tensor (\u00a710) evaluates all 20 domains simultaneously for every operation, ensuring that national policy is enforced programmatically rather than through manual review. Agentic automation triggers tax events on every transaction, generates regulatory filings on schedule, and escalates anomalies for human review."),

    h3("7.5.2 National System Integration"),
    p("The sovereign-govos profile uniquely requires integration with existing national systems. These integrations are additive and reversible: Mass enhances the existing system, it never replaces it. The following national system integrations are required for the Pakistan reference deployment:"),
    table(
      ["System", "Integration Type", "Function"],
      [
        ["FBR IRIS", "API + Data Sync", "Tax administration, NTN issuance, return filing, assessment"],
        ["SBP Raast", "Real-time API", "Instant payment settlement, QR payments, request-to-pay"],
        ["NADRA", "Verification API", "CNIC verification, biometric matching, identity attestation"],
        ["SECP eServices", "API + Webhook", "Company registration, annual filing, beneficial ownership"],
        ["SBP RTGS", "SWIFT/ISO 20022", "Large-value interbank settlements"],
        ["Pakistan Single Window", "API + EDI", "Trade facilitation, customs declarations, permits"],
        ["Provincial Systems", "API (varies)", "Land registry, excise, professional licensing"],
      ],
      [2400, 2000, 4960]
    ),

    h3("7.5.3 Example Deployment: Pakistan GovOS"),
    p("The Pakistan GovOS deployment is the canonical sovereign-govos reference implementation. Lawpacks encode the complete Pakistani legal corpus relevant to economic activity: Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act 2005, Customs Act 1969, Companies Act 2017, Foreign Exchange Regulation Act 1947, and all applicable SROs. Regpacks encode FBR withholding rate tables (updated per SRO), SBP monetary policy parameters, FATF mutual evaluation action items, and the consolidated UN/OFAC/EU sanctions lists."),
    p("Corridor configuration establishes Pakistan's bilateral economic corridors: PAK-UAE (trade and remittance, USD/AED/PKR settlement via SBP Raast and UAESWITCH), PAK-KSA (labor remittance and trade, SAR/PKR settlement), PAK-CHN (CPEC trade corridor with RMB/PKR settlement and preferential tariff application), and PAK-UK (services export and diaspora remittance). The multilateral CPEC corridor operates as a bridge corridor connecting Pakistan, China, and participating Central Asian economies with unified customs transit and multi-currency netting."),
    p("The deployment operates across three availability zones within Pakistan (Islamabad, Lahore, Karachi) with disaster recovery in a fourth region. Each zone runs the full MSEZ Stack with regional database replicas. The Experience Layer serves the GovOS Console for federal and provincial government officers, citizen portals for tax filing and business registration, and AI-powered interfaces for natural language regulatory queries."),

    // =========================================================================
    // 7.6 charter-city
    // =========================================================================
    pageBreak(),
    h2("7.6 charter-city"),
    p("The charter-city profile targets large-scale planned developments that require comprehensive civic governance infrastructure. Unlike a traditional SEZ that operates within an existing city's civic framework, a charter city must provision its own land management, civic services, infrastructure governance, and resident administration. This profile is modeled on developments such as Neom, Naya Pakistan Housing, Lusail City, and the various planned cities in the Gulf states and Southeast Asia."),

    h3("7.6.1 Deployed Capabilities"),
    p("Charter-city deployments activate all civic and governance modules at full depth. Land and property management operates as a complete registry with parcel subdivision, title issuance, lease management, zoning enforcement, and development permit processing. Civic services cover workforce administration, health and safety regulation, environmental compliance, public utility management, and resident services. The governance module implements a full constitutional framework with citizen participation mechanisms, council voting, and charter amendment procedures."),
    p("Corporate and financial modules support the full range of commercial activity within the charter city. The licensing module manages municipal business permits, professional licenses, and construction authorizations. The charter-city profile gives equal weight to physical-world governance (land, infrastructure, civic services) and digital governance (corporate, financial, compliance). The compliance tensor is configured with elevated weights for Employment, Consumer Protection, and Corporate domains reflecting the planning-intensive nature of charter city operations."),

    h3("7.6.2 Example Deployment: Gulf Charter City Development"),
    p("A charter-city deployment for a new planned city in the Gulf would configure lawpacks encoding the host country's commercial companies law, land registration law, municipal governance framework, and the charter city's founding legislation (typically a royal decree or special law establishing the development authority). Regpacks would encode building codes, environmental impact assessment requirements, labor welfare regulations for construction workers, and fire safety standards."),
    p("The land/property module would be initialized with the master plan GIS data, establishing parcel boundaries, zoning designations (residential, commercial, industrial, mixed-use, green space), and infrastructure easements. Development permits would track the lifecycle from architectural submission through regulatory review, construction inspection, and occupancy certification. The corridor configuration would establish procurement corridors to construction material suppliers (steel from Turkey, cement from UAE, fixtures from China) and workforce corridors to labor source countries with credential verification and welfare monitoring."),

    // =========================================================================
    // 7.7 digital-native-free-zone
    // =========================================================================
    pageBreak(),
    h2("7.7 digital-native-free-zone"),
    p("The digital-native-free-zone profile targets zones designed for digital-first operations: SaaS companies, DAOs, digital asset custodians, Web3 protocols, and remote-first enterprises that may never require physical premises. This profile is modeled on DMCC Crypto Centre, Cayman Enterprise City, and the emerging class of digital residency zones pioneered by Estonia's e-Residency program."),

    h3("7.7.1 Deployed Capabilities"),
    p("Digital-native-free-zone deployments prioritize speed of formation, digital identity, and IP protection above all else. Entity formation targets sub-one-hour registration with fully digital onboarding, no physical presence requirement, and immediate issuance of formation Verifiable Credentials. Identity services operate entirely on digital credentials: no physical document verification, instead relying on zkKYC proofs composed from credentials issued by recognized identity providers in the entity's home jurisdiction."),
    p("Financial services are configured for digital asset operations: multi-currency accounts supporting both fiat and tokenized assets, custody services for digital assets with programmable release conditions, and settlement rails optimized for stablecoin and CBDC transfers. The IP module provides rapid trademark filing, software copyright registration, and open-source license compliance verification. Physical-world modules (customs, land/property, heavy civic services) are completely deactivated."),

    h3("7.7.2 Example Deployment: Digital Free Zone Authority"),
    p("A digital-native-free-zone deployment for a new digital asset hub would configure lawpacks encoding the host jurisdiction's virtual asset regulatory framework, data protection legislation, electronic transactions law, and any applicable digital asset-specific legislation (such as the ADGM DLT Foundations Regulations or Cayman Virtual Asset Service Providers Act). Regpacks would include FATF Recommendation 16 (travel rule) implementation parameters, digital asset custody standards, and smart contract audit requirements."),
    p("The formation flow would operate as a fully automated pipeline: applicant submits digital identity credentials, the compliance tensor evaluates KYC/KYB, sanctions, and jurisdictional eligibility in parallel, and upon passing, the Mass Entities primitive creates the entity while the Mass Ownership primitive establishes the initial share structure. The entire process from application submission to formation VC issuance targets sub-60-minute completion with zero human intervention for standard applications. Corridor configuration would establish digital asset corridors with major counterparty zones, enabling token transfers with automated travel rule compliance and real-time sanctions screening."),

    // =========================================================================
    // 7.8 asset-history-bundle
    // =========================================================================
    pageBreak(),
    h2("7.8 asset-history-bundle"),
    p("The asset-history-bundle profile is the most specialized configuration in the MSEZ Stack. Rather than provisioning a comprehensive zone, this profile focuses narrowly on asset provenance, certification, and receipt chain management. It is designed for deployments where the primary requirement is maintaining cryptographically verifiable histories of high-value assets: real estate titles, fine art, luxury goods, commodity lots, industrial equipment, and any asset class where provenance determines value and regulatory compliance depends on traceable chain of custody."),

    h3("7.8.1 Deployed Capabilities"),
    p("Asset-history-bundle deployments activate the receipt chain subsystem (\u00a79) at maximum fidelity. Every state transition of a tracked asset generates a receipt: creation, inspection, certification, transfer, encumbrance, release, and disposal. The credential module issues Verifiable Credentials for certifications, appraisals, and compliance attestations that can be independently verified without contacting the issuing system."),
    p("Corporate services are minimal: just enough to register the entities involved in asset transactions. Financial services support escrow and settlement for asset transfers. The compliance tensor is configured with narrow focus on the specific regulatory domains relevant to the asset class being tracked. For art provenance, this might emphasize Aml and Sanctions (anti-money-laundering in art transactions). For commodity lots, it would emphasize Trade and Aml (ethical sourcing). The profile is designed to be embedded within a larger system rather than to operate as a standalone zone."),

    h3("7.8.2 Example Deployment: Commodity Provenance Registry"),
    p("An asset-history-bundle deployment for commodity provenance would configure lawpacks encoding applicable commodity trading regulations, ethical sourcing requirements (EU Conflict Minerals Regulation, US Dodd-Frank Section 1502), and transit-country customs laws. Regpacks would include commodity exchange standards, assay and grading specifications, and sanctions screening parameters for commodity-origin jurisdictions."),
    p("Each commodity lot would be registered as a tracked asset with an initial certification receipt recording the assay results, origin mine or farm, and ethical sourcing attestation. Subsequent receipts track every custody transfer, blending operation, quality re-certification, and cross-border movement. The receipt chain enables any downstream buyer to verify the complete provenance of a lot by requesting a Merkle proof from the MMR, verifiable against the published root hash without requiring access to the full chain. Corridor configuration would establish commodity flow corridors with receipt chain synchronization between origin, transit, and destination zones."),

    // =========================================================================
    // 7.9 Profile Selection and Composition
    // =========================================================================
    pageBreak(),
    h2("7.9 Profile Selection and Composition"),
    p("Selecting the correct profile is a jurisdictional architecture decision, not a feature selection exercise. The profile determines the compliance surface area of the zone, which in turn determines regulatory obligations, audit requirements, and operational costs. Deploying a digital-financial-center profile for a zone that only needs tech-park capabilities wastes resources on compliance infrastructure that serves no purpose. Deploying a tech-park profile for a zone that needs capital markets capabilities creates a compliance gap that cannot be closed without reprovisioning."),

    h3("7.9.1 Profile Composition Rules"),
    p("Profiles may be composed through extension, but not through arbitrary module mixing. A composed profile must satisfy three invariants. First, all compliance domain dependencies must be met: if a module requires a compliance domain, that domain must be active in the composed profile. Second, all module dependencies must be satisfied: if module A requires module B, both must be active. Third, resource requirements must be additive: the composed profile's resource specification is the component-wise maximum of all constituent profiles."),

    h3("7.9.2 Selection Decision Matrix"),
    table(
      ["If your zone needs...", "Use this profile"],
      [
        ["Regulated financial services, capital markets, institutional custody", "digital-financial-center"],
        ["International trade facilitation, customs, supply chain management", "trade-hub"],
        ["Software companies, startups, R&D labs, IT services", "tech-park"],
        ["National-scale government digital infrastructure", "sovereign-govos"],
        ["Planned city, large-scale real estate, comprehensive civic services", "charter-city"],
        ["Digital-first entities, DAOs, VASPs, remote-first companies", "digital-native-free-zone"],
        ["Asset provenance tracking, certification, chain of custody", "asset-history-bundle"],
      ],
      [5360, 4000]
    ),
    p("When a deployment spans multiple archetypes, the recommended approach is to deploy the more comprehensive profile and deactivate unnecessary modules rather than attempting to compose two lighter profiles. This ensures compliance domain coverage remains complete and avoids gaps in the tensor evaluation surface."),
  ];
};
