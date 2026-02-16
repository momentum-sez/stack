const {
  chapterHeading, h2, h3,
  p, p_runs, bold, italic, code,
  definition, table,
  spacer, pageBreak
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
    spacer(),

    definition("Definition 7.1 (Profile).", "A profile P is a tuple (M, \u0398, T, R) where M is the set of active module families, \u0398 is the parameter configuration map, T is the compliance tensor weight matrix, and R is the resource requirement specification. A zone Z instantiated with profile P inherits all four components and may override \u0398 entries within the bounds declared by P's parameter constraints."),
    spacer(),

    // =========================================================================
    // 7.2 digital-financial-center
    // =========================================================================
    h2("7.2 digital-financial-center"),
    p("The digital-financial-center profile targets jurisdictions seeking to establish full-service financial zones comparable to the Abu Dhabi Global Market, Dubai International Financial Centre, or Singapore's financial district. This is the most comprehensive profile in the MSEZ Stack, activating all 16 module families with maximum compliance depth. It is designed for zones that will host regulated financial institutions, capital markets, fund administration, insurance underwriting, and cross-border settlement."),

    h3("7.2.1 Deployed Capabilities"),
    p("This profile deploys the complete module set with financial-grade configuration. Corporate services include full entity lifecycle management with complex ownership structures, nominee arrangements, and regulated fund vehicles. Financial services include multi-currency account management, payment processing across SWIFT, SEPA, and local rails, custody services with multi-signature quorum policies, and capital markets infrastructure supporting primary issuance and secondary trading. The compliance surface is maximized: all 20 compliance domains are active, sanctions screening operates in real-time with sub-second latency requirements, and transaction monitoring applies jurisdiction-specific rule engines continuously."),
    p("Corridor capabilities are fully enabled. The profile supports unlimited bilateral and multilateral corridors with real-time settlement, netting, and fork resolution. Receipt chains operate at full fidelity with complete provenance tracking. The arbitration module provides formal dispute resolution with escrow, evidence packages, and ruling enforcement via Verifiable Credentials."),

    h3("7.2.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Full entity lifecycle, complex structures, nominee arrangements"],
        ["Financial", "Active", "Multi-currency accounts, custody, capital markets, FX"],
        ["Trade", "Active", "Trade finance, letters of credit, documentary collections"],
        ["Corridors", "Active", "Unlimited corridors, real-time settlement, netting"],
        ["Governance", "Active", "Constitutional framework, voting, amendment procedures"],
        ["Regulatory", "Active", "Full KYC/KYB, AML/CFT, sanctions screening, STR filing"],
        ["Licensing", "Active", "All license categories, mutual recognition, portability"],
        ["Legal", "Active", "Contract management, dispute resolution, arbitration"],
        ["Identity", "Active", "zkKYC, credential issuance, cross-zone portability"],
        ["Compliance", "Active", "All 20 domains active, full tensor evaluation"],
        ["Tax", "Active", "Withholding, reporting, treaty application, transfer pricing"],
        ["Insurance", "Active", "Underwriting, claims, reinsurance, captive vehicles"],
        ["IP", "Active", "Patent, trademark, trade secret registries"],
        ["Customs", "Active", "Tariff classification, bonded warehousing, duty deferral"],
        ["Land/Property", "Active", "Commercial property registry, lease management"],
        ["Civic", "Active", "Workforce permits, health and safety, environmental"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.2.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "16 vCPU, 64 GB RAM", "32 vCPU, 128 GB RAM"],
        ["Storage", "500 GB NVMe SSD", "2 TB NVMe SSD (RAID-10)"],
        ["Database", "PostgreSQL 16, 3-node cluster", "PostgreSQL 16, 5-node cluster + read replicas"],
        ["Network", "1 Gbps dedicated", "10 Gbps with DDoS mitigation"],
        ["HSM", "FIPS 140-2 Level 3 required", "FIPS 140-3 Level 3, dual-redundant"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.2.4 Example Deployment: Abu Dhabi Digital Financial Centre"),
    p("A sovereign deploying the digital-financial-center profile for a new ADGM-class zone would configure the profile with UAE-specific lawpacks encoding Federal Decree-Law No. 32/2021 (Commercial Companies), ADGM Financial Services and Markets Regulations 2015, and the ADGM Insolvency Regulations. Regpacks would include UAE Central Bank prudential requirements, FATF mutual evaluation follow-up items, and the OFAC/EU/UN consolidated sanctions lists with daily synchronization. Licensepacks would encode ADGM FSRA license categories (Category 1 through Category 4), regulated activity permissions, and capital adequacy requirements per license type."),
    p("Corridor configuration would establish the UAE-GCC corridor (real-time settlement via UAESWITCH and GCC-RTGS), the UAE-India corridor (high-volume remittance with UPI integration), and the UAE-UK corridor (DIFC-London financial services passporting). Each corridor binds specific lawpack and regpack digests establishing the regulatory context at corridor creation time."),
    spacer(),

    // =========================================================================
    // 7.3 trade-hub
    // =========================================================================
    pageBreak(),
    h2("7.3 trade-hub"),
    p("The trade-hub profile targets zones focused on international trade facilitation, logistics, and supply chain management. Modeled on successful trade zones such as Jebel Ali Free Zone, Singapore Free Trade Zones, and the Shenzhen Special Economic Zone, this profile prioritizes customs processing, trade finance, corridor management, and supply chain documentation over capital markets and complex financial instruments. It is the appropriate choice for zones where the primary economic activity is the movement of physical goods across borders."),

    h3("7.3.1 Deployed Capabilities"),
    p("Trade-hub deployments activate corporate services for entity formation optimized for trading companies, freight forwarders, and logistics operators. Financial services are configured for trade finance instruments: letters of credit, documentary collections, trade receivables financing, and supply chain finance. The customs module operates at full depth with tariff classification, bonded warehousing, duty deferral schemes, and preferential origin determination under applicable free trade agreements."),
    p("Corridor capabilities focus on trade corridors with receipt chain tracking (§9) for goods movement. The compliance tensor is configured with elevated weights for CUSTOMS, TRADE, SANCTIONS, and AML_CFT domains, reflecting the regulatory priorities of trade facilitation zones."),

    h3("7.3.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Streamlined formation for trading entities, branch offices"],
        ["Financial", "Active", "Trade finance focus: LCs, documentary collections, supply chain finance"],
        ["Trade", "Active", "Full trade documentation, Incoterms, origin determination"],
        ["Corridors", "Active", "Trade corridors with goods-movement receipt chains"],
        ["Governance", "Active", "Zone governance, fee schedules, operator licensing"],
        ["Regulatory", "Active", "Trade-focused KYC, dual-use goods screening, end-user verification"],
        ["Licensing", "Active", "Trading licenses, customs broker permits, warehousing authorizations"],
        ["Legal", "Active", "Trade dispute resolution, cargo claims, insurance arbitration"],
        ["Identity", "Active", "Entity verification, authorized signatory management"],
        ["Compliance", "Active", "14 domains active (insurance, IP, land/property inactive)"],
        ["Tax", "Active", "Customs duties, VAT on imports, withholding on services"],
        ["Insurance", "Minimal", "Cargo insurance verification only"],
        ["IP", "Inactive", "Not required for trade-hub operations"],
        ["Customs", "Active", "Full depth: tariff classification, bonded zones, FTA preferences"],
        ["Land/Property", "Inactive", "Warehouse allocation via operational module"],
        ["Civic", "Minimal", "Workforce permits for zone employees only"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.3.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "8 vCPU, 32 GB RAM", "16 vCPU, 64 GB RAM"],
        ["Storage", "250 GB NVMe SSD", "1 TB NVMe SSD"],
        ["Database", "PostgreSQL 16, single node", "PostgreSQL 16, 3-node cluster"],
        ["Network", "500 Mbps dedicated", "1 Gbps with low-latency peering"],
        ["HSM", "Software HSM acceptable", "FIPS 140-2 Level 3 for production"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.3.4 Example Deployment: Pakistan-UAE Trade Corridor Zone"),
    p("A trade-hub deployment for a Pakistan-UAE bilateral trade zone would configure lawpacks encoding Pakistan's Customs Act 1969, Sales Tax Act 1990, and the Pakistan-UAE Bilateral Investment Treaty alongside UAE Federal Customs Law and the GCC Common Customs Tariff. Regpacks would include Pakistan Single Window integration parameters, CPEC preferential tariff schedules, and SBP foreign exchange regulations governing trade settlements in PKR, AED, and USD."),
    p("The primary corridor (PAK-UAE) would be configured for containerized cargo flows with receipt chains tracking bill of lading issuance, customs declaration filing, inspection clearance, bonded transit, and final delivery confirmation. A secondary corridor (PAK-UAE-KSA) would enable triangular trade routing through UAE free zones with appropriate re-export documentation and certificate of origin management under the GCC-Pakistan FTA framework."),
    spacer(),

    // =========================================================================
    // 7.4 tech-park
    // =========================================================================
    pageBreak(),
    h2("7.4 tech-park"),
    p("The tech-park profile targets technology and innovation zones where the primary tenants are software companies, R&D laboratories, startups, and technology-focused enterprises. Modeled on zones such as Dubai Internet City, Bangalore's Electronic City, and the Hsinchu Science Park, this profile emphasizes rapid entity formation, intellectual property protection, lightweight financial services, and talent mobility. Capital markets, complex trade finance, and heavy customs infrastructure are not required and are deactivated to reduce operational overhead and compliance burden."),

    h3("7.4.1 Deployed Capabilities"),
    p("Tech-park deployments provide streamlined corporate formation with sub-24-hour entity registration, simplified share structures suitable for venture-backed startups, and SAFE/convertible instrument support integrated through the Mass Ownership primitive. Licensing is configured for technology-sector permits: software development, IT services, data processing, and telecommunications value-added services. The IP module operates at full depth with patent application tracking, trademark registration, trade secret protection, and technology transfer agreement management."),
    p("Financial services are configured in lightweight mode: operating accounts, payroll processing, and investor capital receipt. Capital markets functionality is disabled. Identity services emphasize talent credentials: professional qualifications, employment history verification, and cross-zone workforce portability. The compliance tensor activates 10 domains, omitting customs, insurance, land/property, trade-specific, and heavy financial regulation domains that are irrelevant to pure technology operations."),

    h3("7.4.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Rapid formation, startup-optimized structures, SAFE support"],
        ["Financial", "Minimal", "Operating accounts, payroll, investor capital receipt"],
        ["Trade", "Inactive", "Not required for technology park operations"],
        ["Corridors", "Minimal", "Service export corridors only (no goods movement)"],
        ["Governance", "Active", "Zone governance, tenant council, innovation grants"],
        ["Regulatory", "Active", "Simplified KYC for technology entities, data protection"],
        ["Licensing", "Active", "Tech licenses: software, IT services, data processing, telecom VAS"],
        ["Legal", "Active", "IP dispute resolution, employment arbitration, NDA enforcement"],
        ["Identity", "Active", "Professional credentials, talent mobility, workforce permits"],
        ["Compliance", "Active", "10 domains active, technology-sector focus"],
        ["Tax", "Active", "Corporate tax, withholding on services, R&D tax incentives"],
        ["Insurance", "Inactive", "Not required at zone level"],
        ["IP", "Active", "Full depth: patents, trademarks, trade secrets, tech transfer"],
        ["Customs", "Inactive", "No physical goods handling"],
        ["Land/Property", "Minimal", "Office allocation only, no complex property registry"],
        ["Civic", "Active", "Workforce permits, visa processing, talent programs"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.4.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "4 vCPU, 16 GB RAM", "8 vCPU, 32 GB RAM"],
        ["Storage", "100 GB SSD", "500 GB NVMe SSD"],
        ["Database", "PostgreSQL 16, single node", "PostgreSQL 16, primary + standby"],
        ["Network", "100 Mbps", "1 Gbps"],
        ["HSM", "Software HSM acceptable", "Software HSM acceptable"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.4.4 Example Deployment: Islamabad Technology Park"),
    p("A tech-park deployment for a Pakistani technology zone would configure lawpacks encoding the relevant provisions of the Companies Act 2017 (simplified private limited formation), the Income Tax Ordinance 2001 (Section 100C IT exemptions for SEZ enterprises), and SECP regulatory requirements for technology companies. Regpacks would include PSEB (Pakistan Software Export Board) registration requirements, PTA licensing categories for IT and telecom services, and withholding tax exemptions applicable to IT export revenues under SRO 1371(I)/2022."),
    p("The zone would operate a single service-export corridor (PAK-GLOBAL) enabling technology service delivery to international clients with automatic withholding tax computation based on treaty status, PSEB certification verification, and foreign exchange receipt tracking through SBP-authorized dealer banks. Entity formation would target sub-4-hour registration leveraging the Mass Entities primitive with SECP integration, producing a formation Verifiable Credential that serves as portable proof of incorporation across all system participants."),
    spacer(),

    // =========================================================================
    // 7.5 sovereign-govos
    // =========================================================================
    pageBreak(),
    h2("7.5 sovereign-govos"),
    p("The sovereign-govos profile is the most demanding deployment configuration in the MSEZ Stack. It transforms the Stack from a zone management system into a national operating system for government services. This profile activates all 16 module families at maximum depth, adds GovOS orchestration capabilities, and integrates with existing national government systems. It is not a zone profile in the traditional sense; it is the profile that enables a sovereign nation to operate its entire economic infrastructure through the MSEZ Stack and Mass primitives."),
    p("Pakistan serves as the reference implementation for sovereign-govos. The four-layer GovOS architecture (Experience, Platform Engine, Jurisdictional Configuration, National System Integration) described in Chapter 38 is the canonical deployment model for this profile. Every capability that exists in any other profile is active in sovereign-govos, plus national-scale capabilities that no other profile requires."),

    h3("7.5.1 Deployed Capabilities"),
    p("Sovereign-govos deploys the complete MSEZ Stack with national-scale extensions. Entity management covers the full lifecycle from company incorporation through SECP to dissolution, including beneficial ownership reporting, NTN binding with FBR, and provincial registration. Fiscal operations integrate with national payment rails: SBP Raast for instant payments, RTGS for large-value settlements, and commercial bank APIs for account management. Tax administration operates across all federal and provincial tax types with automatic assessment, withholding, filing, and reconciliation."),
    p("Identity services integrate with NADRA for CNIC verification, enabling zkKYC workflows that verify identity without exposing biometric data. Consent management provides multi-party governance for tax assessments, regulatory approvals, and inter-ministry coordination. The compliance tensor (§10) evaluates all 20 domains simultaneously for every operation, ensuring that national policy is enforced programmatically rather than through manual review. Agentic automation triggers tax events on every transaction, generates regulatory filings on schedule, and escalates anomalies for human review."),

    h3("7.5.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Full national company registry, SECP integration, NTN binding"],
        ["Financial", "Active", "National payment rails: Raast, RTGS, commercial bank integration"],
        ["Trade", "Active", "Pakistan Single Window, customs, CPEC corridor management"],
        ["Corridors", "Active", "Bilateral corridors with all treaty partners, multilateral CPEC"],
        ["Governance", "Active", "Federal/provincial constitutional framework, inter-ministry consent"],
        ["Regulatory", "Active", "Full national regulatory framework, all federal and provincial authorities"],
        ["Licensing", "Active", "All 15+ license categories across SECP, BOI, PTA, PEMRA, DRAP, provincial"],
        ["Legal", "Active", "Court system integration, arbitration, enforcement"],
        ["Identity", "Active", "NADRA integration, zkKYC, credential issuance, DID management"],
        ["Compliance", "Active", "All 20 domains, national policy enforcement"],
        ["Tax", "Active", "Full FBR integration: income tax, sales tax, FED, customs duties"],
        ["Insurance", "Active", "SECP insurance division, underwriting, claims"],
        ["IP", "Active", "IPO-Pakistan integration, patent/trademark registries"],
        ["Customs", "Active", "FBR Customs, Pakistan Single Window, CPEC preferences"],
        ["Land/Property", "Active", "Provincial land registries, federal property management"],
        ["Civic", "Active", "Full civic services: health, education, labor, environment"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.5.3 National System Integration"),
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
    spacer(),

    h3("7.5.4 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "64 vCPU, 256 GB RAM", "128+ vCPU, 512+ GB RAM (multi-region)"],
        ["Storage", "5 TB NVMe SSD", "20+ TB NVMe SSD (RAID-10, multi-region)"],
        ["Database", "PostgreSQL 16, 5-node cluster", "PostgreSQL 16, multi-region, sharded"],
        ["Network", "10 Gbps dedicated", "100 Gbps backbone, multi-provider redundancy"],
        ["HSM", "FIPS 140-3 Level 3, dual-redundant", "FIPS 140-3 Level 3, triple-redundant, geo-distributed"],
        ["GPU (AI/ML)", "4x NVIDIA A100 (optional)", "8x NVIDIA H100 for sovereign AI workloads"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.5.5 Example Deployment: Pakistan GovOS"),
    p("The Pakistan GovOS deployment is the canonical sovereign-govos reference implementation. Lawpacks encode the complete Pakistani legal corpus relevant to economic activity: Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act 2005, Customs Act 1969, Companies Act 2017, Foreign Exchange Regulation Act 1947, and all applicable SROs. Regpacks encode FBR withholding rate tables (updated per SRO), SBP monetary policy parameters, FATF mutual evaluation action items, and the consolidated UN/OFAC/EU sanctions lists."),
    p("Corridor configuration establishes Pakistan's bilateral economic corridors: PAK-UAE (trade and remittance, USD/AED/PKR settlement via SBP Raast and UAESWITCH), PAK-KSA (labor remittance and trade, SAR/PKR settlement), PAK-CHN (CPEC trade corridor with RMB/PKR settlement and preferential tariff application), and PAK-UK (services export and diaspora remittance). The multilateral CPEC corridor operates as a bridge corridor connecting Pakistan, China, and participating Central Asian economies with unified customs transit and multi-currency netting."),
    p("The deployment operates across three availability zones within Pakistan (Islamabad, Lahore, Karachi) with disaster recovery in a fourth region. Each zone runs the full MSEZ Stack with regional database replicas. The Experience Layer serves the GovOS Console for federal and provincial government officers, citizen portals for tax filing and business registration, and AI-powered interfaces for natural language regulatory queries."),
    spacer(),

    // =========================================================================
    // 7.6 charter-city
    // =========================================================================
    pageBreak(),
    h2("7.6 charter-city"),
    p("The charter-city profile targets large-scale planned developments that require comprehensive civic governance infrastructure. Unlike a traditional SEZ that operates within an existing city's civic framework, a charter city must provision its own land management, civic services, infrastructure governance, and resident administration. This profile is modeled on developments such as Neom, Naya Pakistan Housing, Lusail City, and the various planned cities in the Gulf states and Southeast Asia that require institutional infrastructure built from first principles."),

    h3("7.6.1 Deployed Capabilities"),
    p("Charter-city deployments activate all civic and governance modules at full depth. Land and property management operates as a complete registry with parcel subdivision, title issuance, lease management, zoning enforcement, and development permit processing. Civic services cover workforce administration, health and safety regulation, environmental compliance, public utility management, and resident services. The governance module implements a full constitutional framework with citizen participation mechanisms, council voting, and charter amendment procedures."),
    p("Corporate and financial modules support the full range of commercial activity within the charter city. The licensing module manages municipal business permits, professional licenses, and construction authorizations. Unlike the digital-financial-center profile, the charter-city profile gives equal weight to physical-world governance (land, infrastructure, civic services) and digital governance (corporate, financial, compliance). The compliance tensor is configured with elevated weights for ENVIRONMENTAL, LABOR, LAND, and CIVIC domains reflecting the planning-intensive nature of charter city operations."),

    h3("7.6.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Full entity lifecycle, developer/contractor licensing"],
        ["Financial", "Active", "Municipal finance, project finance, resident accounts"],
        ["Trade", "Minimal", "Procurement of construction materials, import facilitation"],
        ["Corridors", "Active", "Corridors to supplier jurisdictions, workforce source countries"],
        ["Governance", "Active", "Full constitutional framework, council, charter amendments"],
        ["Regulatory", "Active", "Municipal regulation, building codes, environmental standards"],
        ["Licensing", "Active", "Construction permits, business licenses, professional certifications"],
        ["Legal", "Active", "Property disputes, contract enforcement, resident rights"],
        ["Identity", "Active", "Resident registration, workforce credentials, visitor management"],
        ["Compliance", "Active", "16 domains active, civic-governance emphasis"],
        ["Tax", "Active", "Municipal taxes, property tax, service charges, development levies"],
        ["Insurance", "Active", "Construction insurance, property insurance, liability coverage"],
        ["IP", "Minimal", "Technology company tenants only"],
        ["Customs", "Minimal", "Construction material imports, equipment clearance"],
        ["Land/Property", "Active", "Full depth: registry, zoning, title, lease, development permits"],
        ["Civic", "Active", "Full depth: all civic services, utilities, resident administration"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.6.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "16 vCPU, 64 GB RAM", "32 vCPU, 128 GB RAM"],
        ["Storage", "500 GB NVMe SSD", "2 TB NVMe SSD"],
        ["Database", "PostgreSQL 16, 3-node cluster", "PostgreSQL 16, 5-node cluster"],
        ["Network", "1 Gbps dedicated", "10 Gbps with IoT/sensor network backhaul"],
        ["HSM", "FIPS 140-2 Level 3 required", "FIPS 140-3 Level 3, dual-redundant"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.6.4 Example Deployment: Gulf Charter City Development"),
    p("A charter-city deployment for a new planned city in the Gulf would configure lawpacks encoding the host country's commercial companies law, land registration law, municipal governance framework, and the charter city's founding legislation (typically a royal decree or special law establishing the development authority). Regpacks would encode building codes, environmental impact assessment requirements, labor welfare regulations for construction workers, and fire safety standards."),
    p("The land/property module would be initialized with the master plan GIS data, establishing parcel boundaries, zoning designations (residential, commercial, industrial, mixed-use, green space), and infrastructure easements. Development permits would track the lifecycle from architectural submission through regulatory review, construction inspection, and occupancy certification. The corridor configuration would establish procurement corridors to construction material suppliers (steel from Turkey, cement from UAE, fixtures from China) and workforce corridors to labor source countries with credential verification and welfare monitoring."),
    spacer(),

    // =========================================================================
    // 7.7 digital-native-free-zone
    // =========================================================================
    pageBreak(),
    h2("7.7 digital-native-free-zone"),
    p("The digital-native-free-zone profile targets zones designed from inception for digital-first operations. Unlike the tech-park profile, which accommodates existing technology companies within a traditional zone framework, the digital-native-free-zone is built for organizations that may never require physical premises: SaaS companies, DAOs, digital asset custodians, Web3 protocols, and remote-first enterprises. This profile is modeled on DMCC Crypto Centre, Cayman Enterprise City, and the emerging class of digital residency zones pioneered by Estonia's e-Residency program."),

    h3("7.7.1 Deployed Capabilities"),
    p("Digital-native-free-zone deployments prioritize speed of formation, digital identity, and IP protection above all else. Entity formation targets sub-one-hour registration with fully digital onboarding, no physical presence requirement, and immediate issuance of formation Verifiable Credentials. Identity services operate entirely on digital credentials: no physical document verification, instead relying on zkKYC proofs composed from credentials issued by recognized identity providers in the entity's home jurisdiction."),
    p("Financial services are configured for digital asset operations: multi-currency accounts supporting both fiat and tokenized assets, custody services for digital assets with programmable release conditions, and settlement rails optimized for stablecoin and CBDC transfers. The IP module provides rapid trademark filing, software copyright registration, and open-source license compliance verification. Physical-world modules (customs, land/property, heavy civic services) are completely deactivated."),

    h3("7.7.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Active", "Sub-1-hour digital formation, DAO structures, virtual office"],
        ["Financial", "Active", "Fiat + digital asset accounts, stablecoin settlement"],
        ["Trade", "Inactive", "No physical goods movement"],
        ["Corridors", "Active", "Digital service corridors, token transfer corridors"],
        ["Governance", "Active", "Digital-first governance, on-chain voting integration"],
        ["Regulatory", "Active", "Digital asset regulation, VASP licensing, travel rule"],
        ["Licensing", "Active", "VASP licenses, fintech licenses, data processing permits"],
        ["Legal", "Active", "Smart contract disputes, cross-border digital arbitration"],
        ["Identity", "Active", "Fully digital zkKYC, DID-native, credential-first onboarding"],
        ["Compliance", "Active", "12 domains active, digital-asset-specific rules"],
        ["Tax", "Active", "Digital services tax, withholding on token events, treaty claims"],
        ["Insurance", "Inactive", "Not required at zone level"],
        ["IP", "Active", "Software copyright, trademark, open-source compliance"],
        ["Customs", "Inactive", "No physical goods"],
        ["Land/Property", "Inactive", "No physical premises required"],
        ["Civic", "Inactive", "No resident population"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.7.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "4 vCPU, 16 GB RAM", "16 vCPU, 64 GB RAM"],
        ["Storage", "100 GB SSD", "500 GB NVMe SSD"],
        ["Database", "PostgreSQL 16, single node", "PostgreSQL 16, 3-node cluster"],
        ["Network", "100 Mbps", "1 Gbps with global CDN"],
        ["HSM", "Software HSM acceptable", "FIPS 140-2 Level 3 for VASP operations"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.7.4 Example Deployment: Digital Free Zone Authority"),
    p("A digital-native-free-zone deployment for a new digital asset hub would configure lawpacks encoding the host jurisdiction's virtual asset regulatory framework, data protection legislation, electronic transactions law, and any applicable digital asset-specific legislation (such as the ADGM DLT Foundations Regulations or Cayman Virtual Asset Service Providers Act). Regpacks would include FATF Recommendation 16 (travel rule) implementation parameters, digital asset custody standards, and smart contract audit requirements."),
    p("The formation flow would operate as a fully automated pipeline: applicant submits digital identity credentials, the compliance tensor evaluates KYC/KYB, sanctions, and jurisdictional eligibility in parallel, and upon passing, the Mass Entities primitive creates the entity while the Mass Ownership primitive establishes the initial share structure. The entire process from application submission to formation VC issuance targets sub-60-minute completion with zero human intervention for standard applications. Corridor configuration would establish digital asset corridors with major counterparty zones, enabling token transfers with automated travel rule compliance and real-time sanctions screening."),
    spacer(),

    // =========================================================================
    // 7.8 asset-history-bundle
    // =========================================================================
    pageBreak(),
    h2("7.8 asset-history-bundle"),
    p("The asset-history-bundle profile is the most specialized configuration in the MSEZ Stack. Rather than provisioning a comprehensive zone, this profile focuses narrowly on asset provenance, certification, and receipt chain management. It is designed for deployments where the primary requirement is maintaining cryptographically verifiable histories of high-value assets: real estate titles, fine art, luxury goods, commodity lots, industrial equipment, and any asset class where provenance determines value and regulatory compliance depends on traceable chain of custody."),

    h3("7.8.1 Deployed Capabilities"),
    p("Asset-history-bundle deployments activate the receipt chain subsystem (§9) at maximum fidelity. Every state transition of a tracked asset generates a receipt: creation, inspection, certification, transfer, encumbrance, release, and disposal. The credential module issues Verifiable Credentials for certifications, appraisals, and compliance attestations that can be independently verified without contacting the issuing system."),
    p("Corporate services are minimal: just enough to register the entities involved in asset transactions. Financial services support escrow and settlement for asset transfers. The compliance tensor is configured with narrow focus on the specific regulatory domains relevant to the asset class being tracked. For art provenance, this might emphasize AML_CFT and SANCTIONS (anti-money-laundering in art transactions). For commodity lots, it would emphasize CUSTOMS, TRADE, and ENVIRONMENTAL (ethical sourcing). The profile is designed to be embedded within a larger system rather than to operate as a standalone zone."),

    h3("7.8.2 Module Families"),
    table(
      ["Family", "Status", "Configuration Notes"],
      [
        ["Corporate", "Minimal", "Entity registration for asset owners and custodians only"],
        ["Financial", "Minimal", "Escrow and settlement for asset transfers"],
        ["Trade", "Active", "Asset transfer documentation, export licenses, provenance papers"],
        ["Corridors", "Active", "Asset transfer corridors with full receipt chain tracking"],
        ["Governance", "Minimal", "Asset registry governance, certification authority management"],
        ["Regulatory", "Active", "Asset-class-specific regulation, AML for high-value goods"],
        ["Licensing", "Minimal", "Appraiser/certifier/custodian licensing"],
        ["Legal", "Active", "Ownership disputes, lien enforcement, title claims"],
        ["Identity", "Active", "Owner verification, custodian credentials, appraiser certification"],
        ["Compliance", "Active", "6-10 domains active, asset-class-specific configuration"],
        ["Tax", "Minimal", "Transfer taxes, capital gains on disposal, import duties"],
        ["Insurance", "Active", "Asset insurance verification, coverage tracking"],
        ["IP", "Minimal", "Authenticity marks, design rights for luxury goods"],
        ["Customs", "Active", "High-value goods customs, cultural property export controls"],
        ["Land/Property", "Conditional", "Active for real estate asset bundles, inactive otherwise"],
        ["Civic", "Inactive", "Not applicable to asset tracking"],
      ],
      [1800, 1200, 6360]
    ),
    spacer(),

    h3("7.8.3 Resource Requirements"),
    table(
      ["Resource", "Minimum", "Recommended"],
      [
        ["Compute", "4 vCPU, 16 GB RAM", "8 vCPU, 32 GB RAM"],
        ["Storage", "100 GB SSD", "500 GB NVMe SSD (receipt chain growth)"],
        ["Database", "PostgreSQL 16, single node", "PostgreSQL 16, primary + standby"],
        ["Network", "100 Mbps", "500 Mbps"],
        ["HSM", "Software HSM acceptable", "FIPS 140-2 Level 3 for high-value assets"],
      ],
      [2000, 3200, 4160]
    ),
    spacer(),

    h3("7.8.4 Example Deployment: Commodity Provenance Registry"),
    p("An asset-history-bundle deployment for commodity provenance would configure lawpacks encoding applicable commodity trading regulations, ethical sourcing requirements (EU Conflict Minerals Regulation, US Dodd-Frank Section 1502), and transit-country customs laws. Regpacks would include commodity exchange standards, assay and grading specifications, and sanctions screening parameters for commodity-origin jurisdictions."),
    p("Each commodity lot would be registered as a tracked asset with an initial certification receipt recording the assay results, origin mine or farm, and ethical sourcing attestation. Subsequent receipts track every custody transfer, blending operation, quality re-certification, and cross-border movement. The receipt chain enables any downstream buyer to verify the complete provenance of a lot by requesting a Merkle proof from the MMR, verifiable against the published root hash without requiring access to the full chain. Corridor configuration would establish commodity flow corridors with receipt chain synchronization between origin, transit, and destination zones."),
    spacer(),

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
    spacer(),
    p("When a deployment spans multiple archetypes, the recommended approach is to deploy the more comprehensive profile and deactivate unnecessary modules rather than attempting to compose two lighter profiles. This ensures compliance domain coverage remains complete and avoids gaps in the tensor evaluation surface."),
  ];
};
