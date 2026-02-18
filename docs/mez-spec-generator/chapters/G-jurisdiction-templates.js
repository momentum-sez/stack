const { chapterHeading, table, h2, p } = require("../lib/primitives");

module.exports = function build_appendixG() {
  return [
    chapterHeading("Appendix G: Jurisdiction Template Reference"),
    table(
      ["Jurisdiction", "Lawpack", "Regpack", "Licensepack", "Profile"],
      [
        ["Pakistan", "ITO 2001, STA 1990, FEA, Customs Act, Companies Act", "FBR calendars, SROs, FATF AML", "SECP, BOI, PTA, PEMRA, DRAP, Provincial", "sovereign-govos"],
        ["ADGM", "ADGM Companies Regulations, Financial Services Regulations", "FSRA rulebook, FATF", "Financial services, corporate", "digital-financial-center"],
        ["Seychelles", "International Business Companies Act, Financial Services Act", "SFSA guidelines", "IBC, CSL, banking", "sovereign-govos"],
        ["Kazakhstan (Alatau)", "Kazakh civil code + AIFC overlay", "AFSA rules, NB KZ", "AIFC categories + Kazakh", "digital-financial-center"],
      ],
      [1600, 2400, 2000, 1800, 1560]
    ),

    h2("G.1 Pakistan \u2014 Detailed Regulatory Encoding"),
    p("Pakistan is the flagship GovOS deployment. The Pack Trilogy encodes the following legislation and regulatory frameworks:"),
    table(
      ["Pack", "Act / Regulation", "Key Provisions Encoded"],
      [
        ["Lawpack", "Income Tax Ordinance 2001 (ITO)", "Withholding tax schedules (sections 148\u2013156A), corporate tax rates, capital gains regimes, tax credits for EZ entities, NTN binding requirements"],
        ["Lawpack", "Sales Tax Act 1990 (STA)", "Registration thresholds, input/output tax computation, zero-rating for exports, special procedures for EZ supplies"],
        ["Lawpack", "Foreign Exchange Act 1947 (FEA)", "Repatriation rules, permitted foreign currency accounts, REER compliance, capital account restrictions"],
        ["Lawpack", "Customs Act 1969", "Tariff classification, duty exemptions for EZ inputs, bonded warehouse rules, transit trade provisions"],
        ["Lawpack", "Companies Act 2017", "Formation requirements (sections 14\u201320), beneficial ownership disclosure, annual filing deadlines, director residency rules"],
        ["Lawpack", "Economic Zones Act 2012", "Zone developer obligations, one-window operation, 10-year tax exemptions, duty-free import provisions"],
        ["Regpack", "FBR SROs and Circulars", "Active SRO calendar, withholding rate updates, return filing deadlines, IRIS integration parameters"],
        ["Regpack", "SBP Regulations", "Raast payment rail specifications, exchange rate feeds, foreign exchange dealer licensing, prudential limits"],
        ["Regpack", "FATF AML/CFT Framework", "Customer due diligence tiers, suspicious transaction reporting thresholds, PEP screening rules, beneficial ownership registers"],
        ["Licensepack", "SECP Registration", "Company registration status, annual return compliance, director change notifications, charge registration"],
        ["Licensepack", "BOI Approval", "Investment facilitation certificates, EZ developer licenses, zone enterprise approvals, incentive eligibility tracking"],
        ["Licensepack", "PTA/PEMRA/DRAP", "Telecom licensing, media licensing, drug regulatory approvals \u2014 sector-specific license monitoring"],
      ],
      [1200, 2800, 5360]
    ),

    h2("G.2 ADGM \u2014 Detailed Regulatory Encoding"),
    p("Abu Dhabi Global Market is a common-law financial free zone. The Pack Trilogy encodes:"),
    table(
      ["Pack", "Act / Regulation", "Key Provisions Encoded"],
      [
        ["Lawpack", "ADGM Companies Regulations 2020", "SPV formation, share classes, registered agent requirements, dissolution procedures, ultimate beneficial ownership"],
        ["Lawpack", "ADGM Financial Services and Markets Regulations 2015", "Regulated activity definitions, capital adequacy, client money rules, market abuse prohibitions"],
        ["Lawpack", "ADGM Insolvency Regulations 2015", "Winding-up procedures, administrator appointments, creditor priority, cross-border insolvency recognition"],
        ["Regpack", "FSRA Rulebook", "Prudential rules (PRU), conduct of business (COBS), anti-money laundering (AML), fund rules, Islamic finance windows"],
        ["Regpack", "FATF Mutual Evaluation (UAE)", "National risk assessment alignment, enhanced due diligence for high-risk corridors, correspondent banking rules"],
        ["Licensepack", "Financial Services Permissions", "Category 1\u20134 licensing tiers, authorized individual registration, controlled function approvals, annual fee schedules"],
        ["Licensepack", "Commercial Licenses", "Non-financial commercial activity permits, tech startup licenses, special purpose vehicle registrations"],
      ],
      [1200, 3200, 4960]
    ),

    h2("G.3 Seychelles \u2014 Detailed Regulatory Encoding"),
    p("Seychelles provides an offshore IBC regime with growing financial services regulation. The Pack Trilogy encodes:"),
    table(
      ["Pack", "Act / Regulation", "Key Provisions Encoded"],
      [
        ["Lawpack", "International Business Companies Act 2016 (IBC Act)", "IBC formation and re-domiciliation, share capital flexibility, bearer share prohibition, registered agent and office requirements, annual license fee obligations"],
        ["Lawpack", "Companies Act 1972 (domestic)", "Domestic company formation, memorandum and articles requirements, annual general meeting obligations, director duties and disqualification"],
        ["Lawpack", "Financial Services Authority Act 2013", "FSA establishment and powers, regulated activity definitions, supervisory and enforcement authority, appeal mechanisms"],
        ["Lawpack", "Securities Act 2007", "Securities offerings, dealer and adviser licensing, collective investment scheme registration, market conduct rules"],
        ["Lawpack", "Anti-Money Laundering and Countering the Financing of Terrorism Act 2020", "Customer due diligence obligations, suspicious transaction reporting, beneficial ownership registers, designated non-financial business coverage"],
        ["Regpack", "SFSA Regulatory Guidelines", "Prudential requirements for licensees, capital adequacy ratios, audit and reporting obligations, fit-and-proper assessments"],
        ["Regpack", "FATF/ESAAMLG Compliance", "Mutual evaluation action items, correspondent banking due diligence, wire transfer rules, proliferation financing controls"],
        ["Regpack", "Central Bank of Seychelles Directives", "Foreign exchange controls, banking license conditions, payment system oversight, reserve requirements"],
        ["Licensepack", "IBC License", "Annual license renewal, registered agent confirmation, compliance certificate issuance, strike-off and restoration procedures"],
        ["Licensepack", "CSL (Company Special License)", "Onshore tax-incentivized license, 1.5% corporate tax rate, substance requirements, eligible activity categories"],
        ["Licensepack", "Banking and Insurance Licenses", "Commercial banking permits, insurance intermediary licenses, capital requirements, annual supervisory assessments"],
        ["Licensepack", "Securities Dealer License", "Dealer categories (A/B), advisory licenses, investment fund administrator approvals, ongoing reporting obligations"],
      ],
      [1200, 3200, 4960]
    ),

    h2("G.4 Kazakhstan (Alatau / AIFC) \u2014 Detailed Regulatory Encoding"),
    p("The Astana International Financial Centre operates under English common law within Kazakhstan's civil law system. The Pack Trilogy encodes:"),
    table(
      ["Pack", "Act / Regulation", "Key Provisions Encoded"],
      [
        ["Lawpack", "AIFC Constitutional Statute 2015", "AIFC jurisdiction boundaries, common-law application, AIFC Court and arbitration centre authority, participant categories"],
        ["Lawpack", "AIFC Companies Regulations", "Company formation, limited partnerships, special purpose companies, recognized company regime, beneficial ownership"],
        ["Lawpack", "AIFC Financial Services Framework Regulations", "Authorized firm categories, regulated activity definitions, capital requirements, client asset protection"],
        ["Lawpack", "Kazakh Civil Code (general)", "Domestic entity formation, contract law, property rights, cross-border enforcement \u2014 applies outside AIFC perimeter"],
        ["Regpack", "AFSA Rules and Guidance", "Prudential (PRU), conduct of business (COB), anti-money laundering (AML), Islamic finance, FinTech sandbox rules"],
        ["Regpack", "National Bank of Kazakhstan Regulations", "Foreign exchange regime, banking supervision, payment system rules, monetary policy parameters"],
        ["Regpack", "FATF/EAG Compliance", "Mutual evaluation action plan, enhanced due diligence requirements, cross-border wire transfer rules, NPO sector oversight"],
        ["Licensepack", "AIFC Financial Services License", "Category tiers, authorized individual registration, controlled functions, annual fee schedules"],
        ["Licensepack", "AIFC FinTech Lab License", "Sandbox participation, testing parameters, graduation requirements, regulatory relief scope"],
        ["Licensepack", "Kazakh Domestic Licenses", "General business permits, sector-specific approvals (telecom, mining, energy), dual-jurisdiction compliance for AIFC participants"],
      ],
      [1200, 3200, 4960]
    ),
  ];
};
