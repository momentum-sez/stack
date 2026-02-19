//! # UAE Regulatory Authority License Mappings
//!
//! Comprehensive coverage of the United Arab Emirates regulatory landscape:
//!
//! - **Federal level** (CBUAE, SCA, Ministry of Economy)
//! - **Abu Dhabi** (ADDED, ADGM/FSRA, free zones: Masdar, KIZAD, twofour54, KEZAD)
//! - **Dubai** (DED, DIFC/DFSA, DMCC, JAFZA, IFZA, DWTC, DSO, DIC, DHCC)

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

fn reg(id: &str, name: &str, jid: &str, url: Option<&str>, caps: &[&str]) -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: id.to_string(),
        name: name.to_string(),
        jurisdiction_id: jid.to_string(),
        registry_url: url.map(|u| u.to_string()),
        did: None,
        api_capabilities: caps.iter().map(|s| s.to_string()).collect(),
    }
}

#[allow(clippy::too_many_arguments)]
fn lt(
    id: &str, name: &str, desc: &str, reg_id: &str, cat: &str,
    activities: &[&str], fee_app: Option<(&str, &str)>, fee_annual: Option<(&str, &str)>,
    validity: Option<i32>,
) -> LicenseTypeDefinition {
    LicenseTypeDefinition {
        license_type_id: id.to_string(),
        name: name.to_string(),
        description: desc.to_string(),
        regulator_id: reg_id.to_string(),
        category: Some(cat.to_string()),
        permitted_activities: activities.iter().map(|s| s.to_string()).collect(),
        requirements: BTreeMap::new(),
        application_fee: fee_app.map(|(c, v)| [(c.to_string(), v.to_string())].into_iter().collect()).unwrap_or_default(),
        annual_fee: fee_annual.map(|(c, v)| [(c.to_string(), v.to_string())].into_iter().collect()).unwrap_or_default(),
        validity_period_years: validity,
    }
}

// ── UAE Federal ──────────────────────────────────────────────────────────────

pub fn federal_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("ae-cbuae", "Central Bank of the UAE", "ae", Some("https://www.centralbank.ae"), &["bank_registry", "license_query"]),
        reg("ae-sca", "Securities and Commodities Authority", "ae", Some("https://www.sca.gov.ae"), &["license_query", "fund_registry"]),
        reg("ae-moec", "Ministry of Economy", "ae", Some("https://www.moec.gov.ae"), &["company_search"]),
    ]
}

pub fn federal_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-cbuae:commercial-bank", "Commercial Banking License", "License to operate a commercial bank under CBUAE regulations", "ae-cbuae", "financial",
           &["deposit_taking", "lending", "trade_finance", "foreign_exchange"], None, None, None),
        lt("ae-cbuae:islamic-bank", "Islamic Banking License", "License for Sharia-compliant banking", "ae-cbuae", "financial",
           &["islamic_deposit_taking", "murabaha", "ijara", "sukuk"], None, None, None),
        lt("ae-cbuae:exchange-house", "Exchange House License", "License for money exchange and remittance", "ae-cbuae", "financial",
           &["currency_exchange", "remittance", "money_transfer"], Some(("AED", "50000")), Some(("AED", "25000")), Some(1)),
        lt("ae-cbuae:stored-value", "Stored Value Facility License", "License to issue stored value instruments", "ae-cbuae", "financial",
           &["e_money_issuance", "payment_services"], Some(("AED", "100000")), Some(("AED", "50000")), Some(2)),
        lt("ae-cbuae:insurance", "Insurance Company License", "License to underwrite insurance in the UAE", "ae-cbuae", "insurance",
           &["general_insurance", "life_insurance", "takaful"], None, None, None),
        lt("ae-sca:securities-broker", "Securities Broker License", "License for securities brokerage activities", "ae-sca", "financial",
           &["securities_brokerage", "trading"], Some(("AED", "10000")), Some(("AED", "5000")), Some(1)),
        lt("ae-sca:fund-manager", "Fund Manager License", "License for investment fund management", "ae-sca", "financial",
           &["fund_management", "portfolio_management", "asset_allocation"], Some(("AED", "10000")), Some(("AED", "5000")), Some(1)),
        lt("ae-moec:foreign-company", "Foreign Company Registration", "Registration of a foreign company branch in the UAE", "ae-moec", "corporate",
           &["business_operations"], Some(("AED", "3000")), Some(("AED", "1500")), Some(1)),
    ]
}

// ── ADGM (Abu Dhabi Global Market) ──────────────────────────────────────────

pub fn adgm_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("ae-adgm-fsra", "ADGM Financial Services Regulatory Authority", "ae-abudhabi-adgm", Some("https://www.adgm.com/fsra"), &["license_query", "fund_registry", "digital_asset_registry"]),
        reg("ae-adgm-ra", "ADGM Registration Authority", "ae-abudhabi-adgm", Some("https://www.adgm.com/ra"), &["company_search", "filing_status"]),
    ]
}

pub fn adgm_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-adgm-ra:company", "ADGM Company Registration", "Registration of a company in ADGM", "ae-adgm-ra", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "2000")), Some(("USD", "1000")), Some(1)),
        lt("ae-adgm-ra:spv", "ADGM Special Purpose Vehicle", "Registration of an SPV in ADGM", "ae-adgm-ra", "corporate",
           &["special_purpose_activities", "securitization"], Some(("USD", "1500")), Some(("USD", "750")), Some(1)),
        lt("ae-adgm-ra:foundation", "ADGM Foundation", "Registration of a foundation in ADGM", "ae-adgm-ra", "corporate",
           &["charitable_activities", "asset_holding"], Some(("USD", "2000")), Some(("USD", "1000")), Some(1)),
        lt("ae-adgm-fsra:banking", "ADGM Banking License", "License to conduct banking in ADGM", "ae-adgm-fsra", "financial",
           &["deposit_taking", "lending", "trade_finance"], None, None, None),
        lt("ae-adgm-fsra:insurance", "ADGM Insurance License", "License to underwrite insurance in ADGM", "ae-adgm-fsra", "insurance",
           &["general_insurance", "life_insurance", "reinsurance"], None, None, None),
        lt("ae-adgm-fsra:insurance-broker", "ADGM Insurance Broker License", "License for insurance brokerage in ADGM", "ae-adgm-fsra", "insurance",
           &["insurance_brokerage", "insurance_advisory"], Some(("USD", "5000")), Some(("USD", "2500")), Some(1)),
        lt("ae-adgm-fsra:asset-management", "ADGM Asset Management License", "License for asset/fund management in ADGM", "ae-adgm-fsra", "financial",
           &["fund_management", "portfolio_management", "asset_allocation"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-adgm-fsra:digital-asset-exchange", "ADGM Digital Asset Exchange License", "License to operate a digital asset exchange (MLP framework)", "ae-adgm-fsra", "financial",
           &["digital_asset_exchange", "digital_asset_custody", "digital_asset_settlement", "orderbook_matching"], Some(("USD", "25000")), Some(("USD", "15000")), Some(1)),
        lt("ae-adgm-fsra:digital-asset-custodian", "ADGM Digital Asset Custodian License", "License for digital asset custody services", "ae-adgm-fsra", "financial",
           &["digital_asset_custody", "digital_asset_safekeeping"], Some(("USD", "15000")), Some(("USD", "10000")), Some(1)),
        lt("ae-adgm-fsra:casp", "ADGM Crypto-Asset Service Provider License", "CASP license under ADGM Virtual Asset framework", "ae-adgm-fsra", "financial",
           &["crypto_advisory", "crypto_brokerage", "crypto_dealing"], Some(("USD", "15000")), Some(("USD", "10000")), Some(1)),
        lt("ae-adgm-fsra:venture-capital", "ADGM Venture Capital Manager License", "License for venture capital fund management", "ae-adgm-fsra", "financial",
           &["venture_capital", "fund_management"], Some(("USD", "5000")), Some(("USD", "3000")), Some(1)),
        lt("ae-adgm-fsra:crowdfunding", "ADGM Crowdfunding License", "License to operate an investment crowdfunding platform", "ae-adgm-fsra", "financial",
           &["equity_crowdfunding", "debt_crowdfunding", "platform_operation"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-adgm-fsra:credit-rating", "ADGM Credit Rating Agency License", "License to provide credit rating services", "ae-adgm-fsra", "financial",
           &["credit_rating", "rating_advisory"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-adgm-fsra:custody", "ADGM Custody License", "License for traditional asset custody services", "ae-adgm-fsra", "financial",
           &["custody_services", "asset_servicing", "settlement"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-adgm-fsra:advisory", "ADGM Financial Advisory License", "License for financial advisory services", "ae-adgm-fsra", "financial",
           &["investment_advisory", "financial_planning"], Some(("USD", "5000")), Some(("USD", "2500")), Some(1)),
    ]
}

// ── DIFC (Dubai International Financial Centre) ──────────────────────────────

pub fn difc_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("ae-difc-dfsa", "DIFC Dubai Financial Services Authority", "ae-dubai-difc", Some("https://www.dfsa.ae"), &["license_query", "fund_registry", "digital_asset_registry"]),
        reg("ae-difc-roc", "DIFC Registrar of Companies", "ae-dubai-difc", Some("https://www.difc.ae"), &["company_search", "filing_status"]),
    ]
}

pub fn difc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-difc-roc:company", "DIFC Company Registration", "Registration of a company in DIFC", "ae-difc-roc", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "8000")), Some(("USD", "4000")), Some(1)),
        lt("ae-difc-roc:foundation", "DIFC Foundation Registration", "Registration of a foundation in DIFC", "ae-difc-roc", "corporate",
           &["charitable_activities", "asset_holding"], Some(("USD", "8000")), Some(("USD", "4000")), Some(1)),
        lt("ae-difc-roc:spv", "DIFC Special Purpose Vehicle", "Registration of an SPV in DIFC", "ae-difc-roc", "corporate",
           &["special_purpose_activities"], Some(("USD", "4000")), Some(("USD", "2000")), Some(1)),
        lt("ae-difc-dfsa:banking", "DIFC Banking License", "License to conduct banking in DIFC", "ae-difc-dfsa", "financial",
           &["deposit_taking", "lending", "trade_finance", "foreign_exchange"], None, None, None),
        lt("ae-difc-dfsa:insurance", "DIFC Insurance License", "License to underwrite insurance in DIFC", "ae-difc-dfsa", "insurance",
           &["general_insurance", "life_insurance", "reinsurance"], None, None, None),
        lt("ae-difc-dfsa:insurance-broker", "DIFC Insurance Intermediary License", "License for insurance brokerage in DIFC", "ae-difc-dfsa", "insurance",
           &["insurance_brokerage", "insurance_advisory"], Some(("USD", "5000")), Some(("USD", "3000")), Some(1)),
        lt("ae-difc-dfsa:asset-management", "DIFC Asset Management License", "License for asset/fund management in DIFC", "ae-difc-dfsa", "financial",
           &["fund_management", "portfolio_management"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-difc-dfsa:securities-broker", "DIFC Securities Broker License", "License for securities dealing/brokerage in DIFC", "ae-difc-dfsa", "financial",
           &["securities_brokerage", "securities_dealing", "trading"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-difc-dfsa:digital-asset-exchange", "DIFC Digital Asset Exchange License", "License to operate a digital asset trading facility", "ae-difc-dfsa", "financial",
           &["digital_asset_exchange", "digital_asset_custody", "orderbook_matching"], Some(("USD", "20000")), Some(("USD", "15000")), Some(1)),
        lt("ae-difc-dfsa:digital-asset-custodian", "DIFC Digital Asset Custodian", "License for digital asset custody in DIFC", "ae-difc-dfsa", "financial",
           &["digital_asset_custody", "digital_asset_safekeeping"], Some(("USD", "15000")), Some(("USD", "10000")), Some(1)),
        lt("ae-difc-dfsa:crowdfunding", "DIFC Crowdfunding License", "License to operate a crowdfunding platform", "ae-difc-dfsa", "financial",
           &["equity_crowdfunding", "debt_crowdfunding"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-difc-dfsa:credit-rating", "DIFC Credit Rating Agency License", "License for credit rating services", "ae-difc-dfsa", "financial",
           &["credit_rating", "rating_advisory"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-difc-dfsa:money-services", "DIFC Money Services License", "License for money transfer/exchange services", "ae-difc-dfsa", "financial",
           &["money_transmission", "currency_exchange"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
        lt("ae-difc-dfsa:custody", "DIFC Custody License", "License for custody services in DIFC", "ae-difc-dfsa", "financial",
           &["custody_services", "asset_servicing"], Some(("USD", "10000")), Some(("USD", "5000")), Some(1)),
    ]
}

// ── DMCC ─────────────────────────────────────────────────────────────────────

pub fn dmcc_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-dmcc", "DMCC Authority", "ae-dubai-dmcc", Some("https://www.dmcc.ae"), &["company_search", "license_query"])]
}

pub fn dmcc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-dmcc:trade-license", "DMCC Trade License", "General trading license in DMCC", "ae-dmcc", "trade",
           &["commodities_trading", "general_trading", "import_export"], Some(("AED", "15000")), Some(("AED", "15000")), Some(1)),
        lt("ae-dmcc:precious-metals", "DMCC Precious Metals License", "License for precious metals trading in DMCC", "ae-dmcc", "trade",
           &["gold_trading", "silver_trading", "precious_metals_refining"], Some(("AED", "25000")), Some(("AED", "20000")), Some(1)),
        lt("ae-dmcc:diamond-trading", "DMCC Diamond Trading License", "License for diamond trading in DMCC", "ae-dmcc", "trade",
           &["diamond_trading", "gemstone_trading"], Some(("AED", "25000")), Some(("AED", "20000")), Some(1)),
        lt("ae-dmcc:casp", "DMCC Crypto-Asset Service Provider License", "License for crypto-asset services in DMCC", "ae-dmcc", "financial",
           &["crypto_exchange", "crypto_custody", "crypto_advisory"], Some(("AED", "50000")), Some(("AED", "30000")), Some(1)),
    ]
}

// ── JAFZA ────────────────────────────────────────────────────────────────────

pub fn jafza_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-jafza", "JAFZA Authority", "ae-dubai-jafza", Some("https://www.jafza.ae"), &["company_search", "license_query"])]
}

pub fn jafza_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-jafza:trading", "JAFZA Trading License", "General trading license in Jebel Ali Free Zone", "ae-jafza", "trade",
           &["import_export", "re_export", "warehousing", "distribution"], Some(("AED", "15000")), Some(("AED", "10000")), Some(1)),
        lt("ae-jafza:industrial", "JAFZA Industrial License", "Manufacturing and industrial license in JAFZA", "ae-jafza", "trade",
           &["manufacturing", "processing", "assembly"], Some(("AED", "15000")), Some(("AED", "10000")), Some(1)),
        lt("ae-jafza:logistics", "JAFZA Logistics License", "Logistics and distribution license", "ae-jafza", "trade",
           &["logistics", "warehousing", "freight_forwarding"], Some(("AED", "15000")), Some(("AED", "10000")), Some(1)),
        lt("ae-jafza:services", "JAFZA Services License", "Professional services license in JAFZA", "ae-jafza", "professional",
           &["consulting", "professional_services", "it_services"], Some(("AED", "10000")), Some(("AED", "8000")), Some(1)),
    ]
}

// ── Remaining Dubai Free Zones ──────────────────────────────────────────────

pub fn ifza_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-ifza", "IFZA Authority", "ae-dubai-ifza", Some("https://www.ifza.com"), &["company_search"])]
}

pub fn ifza_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-ifza:general-trading", "IFZA General Trading License", "General trading license in IFZA", "ae-ifza", "trade",
           &["general_trading", "import_export"], Some(("AED", "11750")), Some(("AED", "11750")), Some(1)),
        lt("ae-ifza:consultancy", "IFZA Consultancy License", "Consultancy license in IFZA", "ae-ifza", "professional",
           &["consulting", "advisory"], Some(("AED", "11750")), Some(("AED", "11750")), Some(1)),
        lt("ae-ifza:e-commerce", "IFZA E-Commerce License", "E-commerce license in IFZA", "ae-ifza", "trade",
           &["e_commerce", "online_retail"], Some(("AED", "11750")), Some(("AED", "11750")), Some(1)),
    ]
}

pub fn dwtc_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-dwtc", "DWTC Free Zone Authority", "ae-dubai-dwtc", Some("https://www.dwtc.com"), &["company_search"])]
}

pub fn dwtc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-dwtc:events", "DWTC Events License", "Events management license in DWTC Free Zone", "ae-dwtc", "trade",
           &["event_management", "exhibition_services"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
        lt("ae-dwtc:hospitality", "DWTC Hospitality License", "Hospitality and F&B license in DWTC", "ae-dwtc", "trade",
           &["hospitality", "food_beverage"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
    ]
}

pub fn dso_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-dso", "Dubai Silicon Oasis Authority", "ae-dubai-dso", Some("https://www.dsoa.ae"), &["company_search"])]
}

pub fn dso_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-dso:tech", "DSO Technology License", "Technology company license in Dubai Silicon Oasis", "ae-dso", "trade",
           &["technology_services", "software_development", "it_services"], Some(("AED", "12000")), Some(("AED", "10000")), Some(1)),
        lt("ae-dso:freelancer", "DSO Freelancer Permit", "Freelancer permit in DSO", "ae-dso", "professional",
           &["freelance_services", "consulting"], Some(("AED", "7500")), Some(("AED", "7500")), Some(1)),
    ]
}

pub fn dic_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-dic", "TECOM Group (DIC/DMC)", "ae-dubai-dic", Some("https://www.tecom.ae"), &["company_search"])]
}

pub fn dic_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-dic:technology", "DIC Technology License", "Technology company license in Dubai Internet/Media City", "ae-dic", "trade",
           &["technology_services", "software_development", "digital_media"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
        lt("ae-dic:media", "DMC Media License", "Media company license in Dubai Media City", "ae-dic", "trade",
           &["media_production", "publishing", "broadcasting"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
    ]
}

pub fn dhcc_regulators() -> Vec<LicensepackRegulator> {
    vec![reg("ae-dhcc", "Dubai Healthcare City Authority", "ae-dubai-dhcc", Some("https://www.dhcc.ae"), &["license_query"])]
}

pub fn dhcc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-dhcc:healthcare", "DHCC Healthcare License", "Healthcare facility license in DHCC", "ae-dhcc", "professional",
           &["healthcare_services", "medical_practice"], Some(("AED", "20000")), Some(("AED", "15000")), Some(1)),
        lt("ae-dhcc:pharmaceutical", "DHCC Pharmaceutical License", "Pharmaceutical license in DHCC", "ae-dhcc", "professional",
           &["pharmaceutical_services", "drug_distribution"], Some(("AED", "20000")), Some(("AED", "15000")), Some(1)),
    ]
}

// ── Abu Dhabi free zones ─────────────────────────────────────────────────────

pub fn abudhabi_fz_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("ae-abudhabi-added", "Abu Dhabi Department of Economic Development", "ae-abudhabi", Some("https://added.gov.ae"), &["license_query"]),
        reg("ae-masdar", "Masdar City Free Zone Authority", "ae-abudhabi-masdar", Some("https://masdarcityfreezone.ae"), &["company_search"]),
        reg("ae-kizad", "KIZAD Authority", "ae-abudhabi-kizad", Some("https://www.kizad.ae"), &["company_search"]),
        reg("ae-twofour54", "twofour54 Abu Dhabi", "ae-abudhabi-twofour54", Some("https://www.twofour54.com"), &["company_search"]),
        reg("ae-kezad", "KEZAD Abu Dhabi", "ae-abudhabi-kezad", Some("https://www.kezad.ae"), &["company_search"]),
    ]
}

pub fn abudhabi_fz_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("ae-abudhabi-added:trade-license", "Abu Dhabi Trade License", "Trade license in Abu Dhabi mainland", "ae-abudhabi-added", "trade",
           &["general_trading", "import_export", "retail"], Some(("AED", "10000")), Some(("AED", "8000")), Some(1)),
        lt("ae-masdar:cleantech", "Masdar Clean Tech License", "Clean technology company license in Masdar City", "ae-masdar", "trade",
           &["clean_technology", "renewable_energy", "sustainability"], Some(("AED", "12000")), Some(("AED", "10000")), Some(1)),
        lt("ae-kizad:industrial", "KIZAD Industrial License", "Industrial license in Khalifa Industrial Zone", "ae-kizad", "trade",
           &["manufacturing", "processing", "logistics"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
        lt("ae-kizad:trading", "KIZAD Trading License", "Trading license in KIZAD", "ae-kizad", "trade",
           &["import_export", "warehousing", "distribution"], Some(("AED", "12000")), Some(("AED", "10000")), Some(1)),
        lt("ae-twofour54:media", "twofour54 Media License", "Media company license in twofour54", "ae-twofour54", "trade",
           &["media_production", "content_creation", "broadcasting"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
        lt("ae-kezad:logistics", "KEZAD Logistics License", "Logistics license in KEZAD", "ae-kezad", "trade",
           &["logistics", "warehousing", "port_operations"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
        lt("ae-kezad:industrial", "KEZAD Industrial License", "Industrial license in KEZAD", "ae-kezad", "trade",
           &["manufacturing", "processing", "assembly"], Some(("AED", "15000")), Some(("AED", "12000")), Some(1)),
    ]
}

// ── Aggregation ──────────────────────────────────────────────────────────────

/// All UAE regulators.
pub fn uae_regulators() -> Vec<LicensepackRegulator> {
    let mut all = Vec::new();
    all.extend(federal_regulators());
    all.extend(adgm_regulators());
    all.extend(difc_regulators());
    all.extend(dmcc_regulators());
    all.extend(jafza_regulators());
    all.extend(ifza_regulators());
    all.extend(dwtc_regulators());
    all.extend(dso_regulators());
    all.extend(dic_regulators());
    all.extend(dhcc_regulators());
    all.extend(abudhabi_fz_regulators());
    all
}

/// All UAE license type definitions.
pub fn uae_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(federal_license_types());
    all.extend(adgm_license_types());
    all.extend(difc_license_types());
    all.extend(dmcc_license_types());
    all.extend(jafza_license_types());
    all.extend(ifza_license_types());
    all.extend(dwtc_license_types());
    all.extend(dso_license_types());
    all.extend(dic_license_types());
    all.extend(dhcc_license_types());
    all.extend(abudhabi_fz_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uae_has_expected_regulators() {
        let regs = uae_regulators();
        assert!(regs.len() >= 16, "expected >= 16 regulators, got {}", regs.len());
    }

    #[test]
    fn adgm_has_extensive_coverage() {
        let types = adgm_license_types();
        assert!(types.len() >= 15, "ADGM should have >= 15 license types, got {}", types.len());
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ae-adgm-fsra:digital-asset-exchange"), "ADGM missing digital asset exchange");
        assert!(ids.contains(&"ae-adgm-fsra:casp"), "ADGM missing CASP");
    }

    #[test]
    fn difc_has_extensive_coverage() {
        let types = difc_license_types();
        assert!(types.len() >= 14, "DIFC should have >= 14 license types, got {}", types.len());
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = uae_license_types();
        let mut ids = std::collections::HashSet::new();
        for lt in &types {
            assert!(ids.insert(&lt.license_type_id), "duplicate: {}", lt.license_type_id);
        }
    }

    #[test]
    fn all_license_types_have_valid_fields() {
        for lt in uae_license_types() {
            assert!(!lt.license_type_id.is_empty());
            assert!(!lt.name.is_empty(), "empty name for {}", lt.license_type_id);
            assert!(lt.category.is_some(), "missing category for {}", lt.license_type_id);
            assert!(!lt.permitted_activities.is_empty(), "no activities for {}", lt.license_type_id);
        }
    }

    #[test]
    fn serialization_roundtrip() {
        for r in uae_regulators().into_iter().take(5) {
            let json = serde_json::to_string(&r).expect("serialize");
            let d: LicensepackRegulator = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(r.regulator_id, d.regulator_id);
        }
        for lt in uae_license_types().into_iter().take(5) {
            let json = serde_json::to_string(&lt).expect("serialize");
            let d: LicenseTypeDefinition = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, d.license_type_id);
        }
    }
}
