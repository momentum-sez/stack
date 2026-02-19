//! # United States Regulatory Authority License Mappings
//!
//! Comprehensive coverage of all 50 US states, District of Columbia, and
//! 5 US territories (Puerto Rico, Guam, US Virgin Islands, American Samoa,
//! Northern Mariana Islands).
//!
//! Each state has at minimum:
//! - Secretary of State (corporate registration)
//! - Banking/Financial regulator (money transmission, lending)
//! - Insurance regulator (insurance producer licensing)
//! - Securities regulator (broker-dealer, investment adviser)
//!
//! Key financial states (NY, CA, DE, WY, TX, FL, NV) include additional
//! detail for digital asset and fintech-specific licenses.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── Helper constructors ─────────────────────────────────────────────────────

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

/// Standard state regulators: SoS, Banking, Insurance, Securities
#[allow(clippy::too_many_arguments)]
fn state_regulators(_st: &str, jid: &str, sos_name: &str, bank_name: &str, ins_name: &str, sec_name: &str, sos_url: &str, bank_url: &str) -> Vec<LicensepackRegulator> {
    vec![
        reg(&format!("{jid}-sos"), sos_name, jid, Some(sos_url), &["company_search", "filing_status"]),
        reg(&format!("{jid}-bank"), bank_name, jid, Some(bank_url), &["license_query"]),
        reg(&format!("{jid}-ins"), ins_name, jid, None, &["license_status"]),
        reg(&format!("{jid}-sec"), sec_name, jid, None, &["registration_query"]),
    ]
}

/// Standard state license types
fn state_license_types(st: &str, jid: &str) -> Vec<LicenseTypeDefinition> {
    vec![
        lt(&format!("{jid}-sos:corp-registration"), "Corporation Registration", &format!("Registration of a corporation in {st}"), &format!("{jid}-sos"), "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "100")), Some(("USD", "50")), None),
        lt(&format!("{jid}-sos:llc-registration"), "LLC Registration", &format!("Registration of a limited liability company in {st}"), &format!("{jid}-sos"), "corporate",
           &["business_operations"], Some(("USD", "100")), Some(("USD", "50")), None),
        lt(&format!("{jid}-bank:money-transmitter"), "Money Transmitter License", &format!("License to transmit money in {st}"), &format!("{jid}-bank"), "financial",
           &["money_transmission", "payment_services", "currency_exchange"], Some(("USD", "500")), Some(("USD", "500")), Some(1)),
        lt(&format!("{jid}-ins:insurance-producer"), "Insurance Producer License", &format!("License to sell insurance in {st}"), &format!("{jid}-ins"), "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "50")), Some(("USD", "50")), Some(2)),
        lt(&format!("{jid}-sec:broker-dealer"), "Broker-Dealer Registration", &format!("Registration as a broker-dealer in {st}"), &format!("{jid}-sec"), "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
        lt(&format!("{jid}-sec:investment-adviser"), "Investment Adviser Registration", &format!("Registration as an investment adviser in {st}"), &format!("{jid}-sec"), "financial",
           &["investment_advisory", "portfolio_management"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── New York ─────────────────────────────────────────────────────────────────

pub fn ny_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-ny-dos", "New York Department of State", "us-ny", Some("https://dos.ny.gov"), &["company_search", "filing_status"]),
        reg("us-ny-dfs", "New York Department of Financial Services", "us-ny", Some("https://www.dfs.ny.gov"), &["license_query", "bitlicense_registry"]),
        reg("us-ny-dfs-ins", "New York DFS Insurance Bureau", "us-ny", None, &["license_status"]),
        reg("us-ny-ag-sec", "New York Attorney General Securities Bureau", "us-ny", None, &["registration_query"]),
    ]
}

pub fn ny_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-ny-dos:corp-registration", "Corporation Registration", "Registration of a corporation under NY Business Corporation Law", "us-ny-dos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "125")), Some(("USD", "9")), None),
        lt("us-ny-dos:llc-registration", "LLC Registration", "Registration of an LLC under NY LLC Law", "us-ny-dos", "corporate",
           &["business_operations"], Some(("USD", "200")), Some(("USD", "9")), None),
        lt("us-ny-dfs:money-transmitter", "Money Transmitter License", "License to engage in money transmission under NY Banking Law Article 13-B", "us-ny-dfs", "financial",
           &["money_transmission", "payment_services", "currency_exchange"], Some(("USD", "3000")), Some(("USD", "1000")), Some(1)),
        lt("us-ny-dfs:bitlicense", "BitLicense", "License for virtual currency business activity under 23 NYCRR Part 200", "us-ny-dfs", "financial",
           &["virtual_currency_exchange", "virtual_currency_transmission", "virtual_currency_custody", "virtual_currency_issuance"], Some(("USD", "5000")), Some(("USD", "2500")), Some(2)),
        lt("us-ny-dfs:banking-charter", "Banking Charter", "Charter to operate a state-chartered bank in New York", "us-ny-dfs", "financial",
           &["deposit_taking", "lending", "trust_services"], None, None, None),
        lt("us-ny-dfs:lending-license", "Licensed Lender", "License to make loans under NY Banking Law Article 9", "us-ny-dfs", "financial",
           &["consumer_lending", "commercial_lending"], Some(("USD", "1000")), Some(("USD", "500")), Some(1)),
        lt("us-ny-dfs-ins:insurance-producer", "Insurance Producer License", "License to sell insurance in New York", "us-ny-dfs-ins", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "40")), Some(("USD", "40")), Some(2)),
        lt("us-ny-ag-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-ny-ag-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
        lt("us-ny-ag-sec:investment-adviser", "Investment Adviser Registration", "State registration for investment advisers", "us-ny-ag-sec", "financial",
           &["investment_advisory", "portfolio_management"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── California ───────────────────────────────────────────────────────────────

pub fn ca_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-ca-sos", "California Secretary of State", "us-ca", Some("https://www.sos.ca.gov"), &["company_search", "filing_status"]),
        reg("us-ca-dfpi", "California Department of Financial Protection and Innovation", "us-ca", Some("https://dfpi.ca.gov"), &["license_query", "casl_registry"]),
        reg("us-ca-cdi", "California Department of Insurance", "us-ca", None, &["license_status"]),
        reg("us-ca-dbo-sec", "California DOC Commissioner of Business Oversight (Securities)", "us-ca", None, &["registration_query"]),
    ]
}

pub fn ca_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-ca-sos:corp-registration", "Corporation Registration", "Registration of a corporation under California Corporations Code", "us-ca-sos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "100")), Some(("USD", "25")), None),
        lt("us-ca-sos:llc-registration", "LLC Registration", "Registration of an LLC under California Revised Uniform LLC Act", "us-ca-sos", "corporate",
           &["business_operations"], Some(("USD", "70")), Some(("USD", "20")), None),
        lt("us-ca-dfpi:money-transmitter", "Money Transmitter License", "License under California Money Transmission Act", "us-ca-dfpi", "financial",
           &["money_transmission", "payment_services", "currency_exchange"], Some(("USD", "5000")), Some(("USD", "2500")), Some(1)),
        lt("us-ca-dfpi:casl", "California Digital Financial Assets License", "License under California Digital Financial Assets Law (DFAL)", "us-ca-dfpi", "financial",
           &["digital_asset_exchange", "digital_asset_custody", "digital_asset_transfer", "digital_asset_issuance"], Some(("USD", "5000")), Some(("USD", "2500")), Some(1)),
        lt("us-ca-dfpi:lending-license", "California Finance Lender License", "License under California Financing Law", "us-ca-dfpi", "financial",
           &["consumer_lending", "commercial_lending"], Some(("USD", "600")), Some(("USD", "200")), Some(1)),
        lt("us-ca-cdi:insurance-producer", "Insurance Producer License", "License to sell insurance in California", "us-ca-cdi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "52")), Some(("USD", "52")), Some(2)),
        lt("us-ca-dbo-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-ca-dbo-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "300")), Some(("USD", "300")), Some(1)),
        lt("us-ca-dbo-sec:investment-adviser", "Investment Adviser Registration", "State registration for investment advisers", "us-ca-dbo-sec", "financial",
           &["investment_advisory", "portfolio_management"], Some(("USD", "300")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Delaware ─────────────────────────────────────────────────────────────────

pub fn de_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-de-dos", "Delaware Division of Corporations", "us-de", Some("https://corp.delaware.gov"), &["company_search", "good_standing", "franchise_tax"]),
        reg("us-de-osbc", "Delaware Office of the State Bank Commissioner", "us-de", Some("https://banking.delaware.gov"), &["license_query"]),
        reg("us-de-doi", "Delaware Department of Insurance", "us-de", None, &["license_status"]),
        reg("us-de-sec", "Delaware Division of Securities", "us-de", None, &["registration_query"]),
    ]
}

pub fn de_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-de-dos:corp-registration", "Corporation Registration", "Registration under Delaware General Corporation Law (Title 8)", "us-de-dos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "89")), Some(("USD", "225")), None),
        lt("us-de-dos:llc-registration", "LLC Registration", "Registration under Delaware LLC Act (Title 6, Ch. 18)", "us-de-dos", "corporate",
           &["business_operations"], Some(("USD", "90")), Some(("USD", "300")), None),
        lt("us-de-dos:lp-registration", "Limited Partnership Registration", "Registration under Delaware Revised Uniform LP Act", "us-de-dos", "corporate",
           &["business_operations", "fund_management"], Some(("USD", "200")), Some(("USD", "300")), None),
        lt("us-de-osbc:money-transmitter", "Money Transmitter License", "License under Delaware Money Transmitters Act (Title 5, Ch. 23)", "us-de-osbc", "financial",
           &["money_transmission", "payment_services"], Some(("USD", "1000")), Some(("USD", "500")), Some(1)),
        lt("us-de-osbc:banking-charter", "Banking Charter", "State bank charter under Delaware banking laws", "us-de-osbc", "financial",
           &["deposit_taking", "lending", "trust_services"], None, None, None),
        lt("us-de-doi:insurance-producer", "Insurance Producer License", "License to sell insurance in Delaware", "us-de-doi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "35")), Some(("USD", "35")), Some(2)),
        lt("us-de-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-de-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Wyoming ──────────────────────────────────────────────────────────────────

pub fn wy_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-wy-sos", "Wyoming Secretary of State", "us-wy", Some("https://sos.wyo.gov"), &["company_search", "filing_status", "dao_registry"]),
        reg("us-wy-dob", "Wyoming Division of Banking", "us-wy", Some("https://banking.wyo.gov"), &["license_query", "spdi_registry"]),
        reg("us-wy-doi", "Wyoming Department of Insurance", "us-wy", None, &["license_status"]),
        reg("us-wy-sec", "Wyoming Securities Division", "us-wy", None, &["registration_query"]),
    ]
}

pub fn wy_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-wy-sos:corp-registration", "Corporation Registration", "Registration under Wyoming Business Corporation Act", "us-wy-sos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "100")), Some(("USD", "60")), None),
        lt("us-wy-sos:llc-registration", "LLC Registration", "Registration under Wyoming LLC Act", "us-wy-sos", "corporate",
           &["business_operations"], Some(("USD", "100")), Some(("USD", "60")), None),
        lt("us-wy-sos:dao-llc", "DAO LLC Registration", "Registration of a Decentralized Autonomous Organization under WY SF0038", "us-wy-sos", "corporate",
           &["business_operations", "governance", "smart_contract_operations"], Some(("USD", "100")), Some(("USD", "60")), None),
        lt("us-wy-dob:spdi-charter", "Special Purpose Depository Institution Charter", "SPDI charter for digital asset custody under WY HB0074", "us-wy-dob", "financial",
           &["digital_asset_custody", "fiat_custody", "asset_servicing"], Some(("USD", "5000")), Some(("USD", "2500")), None),
        lt("us-wy-dob:money-transmitter", "Money Transmitter License", "License under Wyoming Money Transmitter Act", "us-wy-dob", "financial",
           &["money_transmission", "payment_services", "virtual_currency_exchange"], Some(("USD", "500")), Some(("USD", "250")), Some(1)),
        lt("us-wy-doi:insurance-producer", "Insurance Producer License", "License to sell insurance in Wyoming", "us-wy-doi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "25")), Some(("USD", "25")), Some(2)),
        lt("us-wy-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-wy-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Texas ────────────────────────────────────────────────────────────────────

pub fn tx_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-tx-sos", "Texas Secretary of State", "us-tx", Some("https://www.sos.texas.gov"), &["company_search", "filing_status"]),
        reg("us-tx-dob", "Texas Department of Banking", "us-tx", Some("https://www.dob.texas.gov"), &["license_query"]),
        reg("us-tx-tdi", "Texas Department of Insurance", "us-tx", None, &["license_status"]),
        reg("us-tx-tssb", "Texas State Securities Board", "us-tx", None, &["registration_query"]),
    ]
}

pub fn tx_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-tx-sos:corp-registration", "Corporation Registration", "Registration under Texas Business Organizations Code", "us-tx-sos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "300")), None, None),
        lt("us-tx-sos:llc-registration", "LLC Registration", "Registration under Texas Business Organizations Code", "us-tx-sos", "corporate",
           &["business_operations"], Some(("USD", "300")), None, None),
        lt("us-tx-dob:money-transmitter", "Money Transmitter License", "License under Texas Finance Code Chapter 151", "us-tx-dob", "financial",
           &["money_transmission", "payment_services", "currency_exchange"], Some(("USD", "2500")), Some(("USD", "1000")), Some(1)),
        lt("us-tx-tdi:insurance-producer", "Insurance Producer License", "License to sell insurance in Texas", "us-tx-tdi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "50")), Some(("USD", "50")), Some(2)),
        lt("us-tx-tssb:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-tx-tssb", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
        lt("us-tx-tssb:investment-adviser", "Investment Adviser Registration", "State registration for investment advisers", "us-tx-tssb", "financial",
           &["investment_advisory", "portfolio_management"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Florida ──────────────────────────────────────────────────────────────────

pub fn fl_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-fl-dos", "Florida Division of Corporations", "us-fl", Some("https://dos.myflorida.com/sunbiz"), &["company_search", "filing_status"]),
        reg("us-fl-ofr", "Florida Office of Financial Regulation", "us-fl", Some("https://flofr.gov"), &["license_query"]),
        reg("us-fl-doi", "Florida Department of Financial Services (Insurance)", "us-fl", None, &["license_status"]),
        reg("us-fl-sec", "Florida Office of Financial Regulation (Securities)", "us-fl", None, &["registration_query"]),
    ]
}

pub fn fl_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-fl-dos:corp-registration", "Corporation Registration", "Registration under Florida Business Corporation Act", "us-fl-dos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "70")), Some(("USD", "150")), None),
        lt("us-fl-dos:llc-registration", "LLC Registration", "Registration under Florida Revised LLC Act", "us-fl-dos", "corporate",
           &["business_operations"], Some(("USD", "125")), Some(("USD", "139")), None),
        lt("us-fl-ofr:money-transmitter", "Money Transmitter License", "License under Florida Money Transmitters Code (Part III, Ch. 560)", "us-fl-ofr", "financial",
           &["money_transmission", "payment_services"], Some(("USD", "375")), Some(("USD", "375")), Some(1)),
        lt("us-fl-ofr:money-services", "Money Services Business License", "License for money services under Florida statutes", "us-fl-ofr", "financial",
           &["currency_exchange", "check_cashing", "payment_instruments"], Some(("USD", "375")), Some(("USD", "375")), Some(1)),
        lt("us-fl-doi:insurance-producer", "Insurance Producer License", "License to sell insurance in Florida", "us-fl-doi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "55")), Some(("USD", "55")), Some(2)),
        lt("us-fl-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-fl-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Nevada ───────────────────────────────────────────────────────────────────

pub fn nv_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-nv-sos", "Nevada Secretary of State", "us-nv", Some("https://www.nvsos.gov"), &["company_search", "filing_status"]),
        reg("us-nv-fid", "Nevada Financial Institutions Division", "us-nv", Some("https://fid.nv.gov"), &["license_query"]),
        reg("us-nv-doi", "Nevada Division of Insurance", "us-nv", None, &["license_status"]),
        reg("us-nv-sec", "Nevada Securities Division", "us-nv", None, &["registration_query"]),
    ]
}

pub fn nv_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-nv-sos:corp-registration", "Corporation Registration", "Registration under Nevada Revised Statutes Title 7", "us-nv-sos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "75")), Some(("USD", "350")), None),
        lt("us-nv-sos:llc-registration", "LLC Registration", "Registration under Nevada Revised Statutes Chapter 86", "us-nv-sos", "corporate",
           &["business_operations"], Some(("USD", "75")), Some(("USD", "350")), None),
        lt("us-nv-fid:money-transmitter", "Money Transmitter License", "License under Nevada MSB law (NRS 671)", "us-nv-fid", "financial",
           &["money_transmission", "payment_services", "digital_currency"], Some(("USD", "500")), Some(("USD", "500")), Some(1)),
        lt("us-nv-fid:digital-asset-license", "Digital Asset License", "License for digital asset activities in Nevada", "us-nv-fid", "financial",
           &["digital_asset_exchange", "digital_asset_custody"], Some(("USD", "1000")), Some(("USD", "500")), Some(1)),
        lt("us-nv-doi:insurance-producer", "Insurance Producer License", "License to sell insurance in Nevada", "us-nv-doi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "35")), Some(("USD", "35")), Some(2)),
        lt("us-nv-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-nv-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Illinois ─────────────────────────────────────────────────────────────────

pub fn il_regulators() -> Vec<LicensepackRegulator> {
    vec![
        reg("us-il-sos", "Illinois Secretary of State", "us-il", Some("https://www.ilsos.gov"), &["company_search", "filing_status"]),
        reg("us-il-idfpr", "Illinois Department of Financial and Professional Regulation", "us-il", Some("https://idfpr.illinois.gov"), &["license_query"]),
        reg("us-il-doi", "Illinois Department of Insurance", "us-il", None, &["license_status"]),
        reg("us-il-sec", "Illinois Securities Department", "us-il", None, &["registration_query"]),
    ]
}

pub fn il_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        lt("us-il-sos:corp-registration", "Corporation Registration", "Registration under Illinois Business Corporation Act", "us-il-sos", "corporate",
           &["business_operations", "capital_raising"], Some(("USD", "150")), Some(("USD", "75")), None),
        lt("us-il-sos:llc-registration", "LLC Registration", "Registration under Illinois LLC Act", "us-il-sos", "corporate",
           &["business_operations"], Some(("USD", "150")), Some(("USD", "75")), None),
        lt("us-il-idfpr:money-transmitter", "Money Transmitter License", "License under Illinois Transmitters of Money Act", "us-il-idfpr", "financial",
           &["money_transmission", "payment_services"], Some(("USD", "1000")), Some(("USD", "500")), Some(1)),
        lt("us-il-doi:insurance-producer", "Insurance Producer License", "License to sell insurance in Illinois", "us-il-doi", "insurance",
           &["insurance_sales", "insurance_solicitation"], Some(("USD", "50")), Some(("USD", "50")), Some(2)),
        lt("us-il-sec:broker-dealer", "Broker-Dealer Registration", "State registration for securities broker-dealers", "us-il-sec", "financial",
           &["securities_brokerage", "securities_dealing"], Some(("USD", "200")), Some(("USD", "200")), Some(1)),
    ]
}

// ── Remaining 43 states + DC + Territories ──────────────────────────────────
// Using the standard pattern with state-specific regulator names.

macro_rules! define_state {
    ($fn_regs:ident, $fn_lts:ident, $st:expr, $jid:expr, $sos:expr, $bank:expr, $ins:expr, $sec:expr, $sos_url:expr, $bank_url:expr) => {
        pub fn $fn_regs() -> Vec<LicensepackRegulator> {
            state_regulators($st, $jid, $sos, $bank, $ins, $sec, $sos_url, $bank_url)
        }
        pub fn $fn_lts() -> Vec<LicenseTypeDefinition> {
            state_license_types($st, $jid)
        }
    };
}

define_state!(al_regulators, al_license_types, "Alabama", "us-al", "Alabama Secretary of State", "Alabama State Banking Department", "Alabama Department of Insurance", "Alabama Securities Commission", "https://www.sos.alabama.gov", "https://banking.alabama.gov");
define_state!(ak_regulators, ak_license_types, "Alaska", "us-ak", "Alaska Division of Corporations", "Alaska Division of Banking and Securities", "Alaska Division of Insurance", "Alaska Division of Banking and Securities", "https://www.commerce.alaska.gov/web/cbpl/corporations", "https://www.commerce.alaska.gov/web/dbs");
define_state!(az_regulators, az_license_types, "Arizona", "us-az", "Arizona Corporation Commission", "Arizona Department of Financial Institutions", "Arizona Department of Insurance", "Arizona Corporation Commission Securities", "https://ecorp.azcc.gov", "https://difi.az.gov");
define_state!(ar_regulators, ar_license_types, "Arkansas", "us-ar", "Arkansas Secretary of State", "Arkansas State Bank Department", "Arkansas Insurance Department", "Arkansas Securities Department", "https://www.sos.arkansas.gov", "https://banking.arkansas.gov");
define_state!(co_regulators, co_license_types, "Colorado", "us-co", "Colorado Secretary of State", "Colorado Division of Banking", "Colorado Division of Insurance", "Colorado Division of Securities", "https://www.sos.state.co.us", "https://banking.colorado.gov");
define_state!(ct_regulators, ct_license_types, "Connecticut", "us-ct", "Connecticut Secretary of State", "Connecticut Department of Banking", "Connecticut Insurance Department", "Connecticut Department of Banking Securities", "https://portal.ct.gov/sots", "https://portal.ct.gov/dob");
define_state!(ga_regulators, ga_license_types, "Georgia", "us-ga", "Georgia Secretary of State Corporations Division", "Georgia Department of Banking and Finance", "Georgia Office of Insurance", "Georgia Securities Division", "https://sos.ga.gov/corporations-division", "https://dbf.georgia.gov");
define_state!(hi_regulators, hi_license_types, "Hawaii", "us-hi", "Hawaii Department of Commerce", "Hawaii Division of Financial Institutions", "Hawaii Insurance Division", "Hawaii Securities Division", "https://cca.hawaii.gov/breg", "https://cca.hawaii.gov/dfi");
define_state!(id_regulators, id_license_types, "Idaho", "us-id", "Idaho Secretary of State", "Idaho Department of Finance", "Idaho Department of Insurance", "Idaho Department of Finance Securities", "https://sos.idaho.gov", "https://finance.idaho.gov");
define_state!(in_regulators, in_license_types, "Indiana", "us-in", "Indiana Secretary of State", "Indiana Department of Financial Institutions", "Indiana Department of Insurance", "Indiana Securities Division", "https://www.in.gov/sos", "https://www.in.gov/dfi");
define_state!(ia_regulators, ia_license_types, "Iowa", "us-ia", "Iowa Secretary of State", "Iowa Division of Banking", "Iowa Insurance Division", "Iowa Insurance Division Securities", "https://sos.iowa.gov", "https://idob.iowa.gov");
define_state!(ks_regulators, ks_license_types, "Kansas", "us-ks", "Kansas Secretary of State", "Kansas Office of the State Bank Commissioner", "Kansas Insurance Department", "Kansas Securities Commissioner", "https://www.sos.ks.gov", "https://osbckansas.org");
define_state!(ky_regulators, ky_license_types, "Kentucky", "us-ky", "Kentucky Secretary of State", "Kentucky Department of Financial Institutions", "Kentucky Department of Insurance", "Kentucky Department of Financial Institutions Securities", "https://www.sos.ky.gov", "https://kfi.ky.gov");
define_state!(la_regulators, la_license_types, "Louisiana", "us-la", "Louisiana Secretary of State", "Louisiana Office of Financial Institutions", "Louisiana Department of Insurance", "Louisiana Securities Division", "https://www.sos.la.gov", "https://ofi.la.gov");
define_state!(me_regulators, me_license_types, "Maine", "us-me", "Maine Secretary of State", "Maine Bureau of Financial Institutions", "Maine Bureau of Insurance", "Maine Securities Division", "https://www.maine.gov/sos", "https://www.maine.gov/pfr/financialinstitutions");
define_state!(md_regulators, md_license_types, "Maryland", "us-md", "Maryland Department of Assessments and Taxation", "Maryland Commissioner of Financial Regulation", "Maryland Insurance Administration", "Maryland Securities Division", "https://dat.maryland.gov", "https://www.dllr.state.md.us/finance");
define_state!(ma_regulators, ma_license_types, "Massachusetts", "us-ma", "Massachusetts Secretary of the Commonwealth", "Massachusetts Division of Banks", "Massachusetts Division of Insurance", "Massachusetts Securities Division", "https://www.sec.state.ma.us/cor", "https://www.mass.gov/orgs/division-of-banks");
define_state!(mi_regulators, mi_license_types, "Michigan", "us-mi", "Michigan Department of Licensing and Regulatory Affairs", "Michigan Office of Credit Unions and Financial Services", "Michigan Department of Insurance and Financial Services", "Michigan Securities Division", "https://www.michigan.gov/lara", "https://www.michigan.gov/difs");
define_state!(mn_regulators, mn_license_types, "Minnesota", "us-mn", "Minnesota Secretary of State", "Minnesota Department of Commerce Financial Institutions", "Minnesota Department of Commerce Insurance", "Minnesota Department of Commerce Securities", "https://www.sos.state.mn.us", "https://mn.gov/commerce");
define_state!(ms_regulators, ms_license_types, "Mississippi", "us-ms", "Mississippi Secretary of State", "Mississippi Department of Banking and Consumer Finance", "Mississippi Insurance Department", "Mississippi Securities Division", "https://www.sos.ms.gov", "https://dbcf.ms.gov");
define_state!(mo_regulators, mo_license_types, "Missouri", "us-mo", "Missouri Secretary of State", "Missouri Division of Finance", "Missouri Department of Commerce and Insurance", "Missouri Securities Division", "https://www.sos.mo.gov", "https://finance.mo.gov");
define_state!(mt_regulators, mt_license_types, "Montana", "us-mt", "Montana Secretary of State", "Montana Division of Banking and Financial Institutions", "Montana Commissioner of Securities and Insurance", "Montana State Auditor Securities", "https://sosmt.gov", "https://banking.mt.gov");
define_state!(ne_regulators, ne_license_types, "Nebraska", "us-ne", "Nebraska Secretary of State", "Nebraska Department of Banking and Finance", "Nebraska Department of Insurance", "Nebraska Department of Banking and Finance Securities", "https://sos.nebraska.gov", "https://ndbf.nebraska.gov");
define_state!(nh_regulators, nh_license_types, "New Hampshire", "us-nh", "New Hampshire Secretary of State", "New Hampshire Banking Department", "New Hampshire Insurance Department", "New Hampshire Bureau of Securities Regulation", "https://www.sos.nh.gov", "https://www.nh.gov/banking");
define_state!(nj_regulators, nj_license_types, "New Jersey", "us-nj", "New Jersey Division of Revenue", "New Jersey Department of Banking and Insurance", "New Jersey Department of Banking and Insurance", "New Jersey Bureau of Securities", "https://www.njportal.com/DOR/BusinessFormation", "https://www.nj.gov/dobi");
define_state!(nm_regulators, nm_license_types, "New Mexico", "us-nm", "New Mexico Secretary of State", "New Mexico Financial Institutions Division", "New Mexico Office of Superintendent of Insurance", "New Mexico Securities Division", "https://www.sos.state.nm.us", "https://www.rld.nm.gov/financial-institutions");
define_state!(nc_regulators, nc_license_types, "North Carolina", "us-nc", "North Carolina Secretary of State", "North Carolina Commissioner of Banks", "North Carolina Department of Insurance", "North Carolina Securities Division", "https://www.sosnc.gov", "https://www.nccob.gov");
define_state!(nd_regulators, nd_license_types, "North Dakota", "us-nd", "North Dakota Secretary of State", "North Dakota Department of Financial Institutions", "North Dakota Insurance Department", "North Dakota Securities Department", "https://sos.nd.gov", "https://www.nd.gov/dfi");
define_state!(oh_regulators, oh_license_types, "Ohio", "us-oh", "Ohio Secretary of State", "Ohio Division of Financial Institutions", "Ohio Department of Insurance", "Ohio Division of Securities", "https://www.ohiosos.gov", "https://com.ohio.gov/divisions-and-programs/financial-institutions");
define_state!(ok_regulators, ok_license_types, "Oklahoma", "us-ok", "Oklahoma Secretary of State", "Oklahoma State Banking Department", "Oklahoma Insurance Department", "Oklahoma Securities Commission", "https://www.sos.ok.gov", "https://www.ok.gov/banking");
define_state!(or_regulators, or_license_types, "Oregon", "us-or", "Oregon Secretary of State", "Oregon Division of Financial Regulation", "Oregon Division of Financial Regulation Insurance", "Oregon Division of Financial Regulation Securities", "https://sos.oregon.gov", "https://dfr.oregon.gov");
define_state!(pa_regulators, pa_license_types, "Pennsylvania", "us-pa", "Pennsylvania Department of State", "Pennsylvania Department of Banking and Securities", "Pennsylvania Insurance Department", "Pennsylvania Department of Banking and Securities", "https://www.dos.pa.gov", "https://www.dobs.pa.gov");
define_state!(ri_regulators, ri_license_types, "Rhode Island", "us-ri", "Rhode Island Secretary of State", "Rhode Island Division of Banking", "Rhode Island Division of Insurance", "Rhode Island Division of Securities", "https://www.sos.ri.gov", "https://dbr.ri.gov");
define_state!(sc_regulators, sc_license_types, "South Carolina", "us-sc", "South Carolina Secretary of State", "South Carolina Board of Financial Institutions", "South Carolina Department of Insurance", "South Carolina Securities Division", "https://sos.sc.gov", "https://bofi.sc.gov");
define_state!(sd_regulators, sd_license_types, "South Dakota", "us-sd", "South Dakota Secretary of State", "South Dakota Division of Banking", "South Dakota Division of Insurance", "South Dakota Division of Securities", "https://sdsos.gov", "https://dlr.sd.gov/banking");
define_state!(tn_regulators, tn_license_types, "Tennessee", "us-tn", "Tennessee Secretary of State", "Tennessee Department of Financial Institutions", "Tennessee Department of Commerce and Insurance", "Tennessee Securities Division", "https://sos.tn.gov", "https://www.tn.gov/tdfi");
define_state!(ut_regulators, ut_license_types, "Utah", "us-ut", "Utah Division of Corporations", "Utah Department of Financial Institutions", "Utah Insurance Department", "Utah Division of Securities", "https://corporations.utah.gov", "https://dfi.utah.gov");
define_state!(vt_regulators, vt_license_types, "Vermont", "us-vt", "Vermont Secretary of State", "Vermont Department of Financial Regulation", "Vermont Department of Financial Regulation Insurance", "Vermont Department of Financial Regulation Securities", "https://sos.vermont.gov", "https://dfr.vermont.gov");
define_state!(va_regulators, va_license_types, "Virginia", "us-va", "Virginia State Corporation Commission", "Virginia Bureau of Financial Institutions", "Virginia Bureau of Insurance", "Virginia Division of Securities", "https://www.scc.virginia.gov", "https://www.scc.virginia.gov/pages/Bureau-of-Financial-Institutions");
define_state!(wa_regulators, wa_license_types, "Washington", "us-wa", "Washington Secretary of State", "Washington Department of Financial Institutions", "Washington Office of the Insurance Commissioner", "Washington Department of Financial Institutions Securities", "https://www.sos.wa.gov", "https://dfi.wa.gov");
define_state!(wv_regulators, wv_license_types, "West Virginia", "us-wv", "West Virginia Secretary of State", "West Virginia Division of Financial Institutions", "West Virginia Offices of the Insurance Commissioner", "West Virginia Securities Commission", "https://sos.wv.gov", "https://dfi.wv.gov");
define_state!(wi_regulators, wi_license_types, "Wisconsin", "us-wi", "Wisconsin Department of Financial Institutions (Corporations)", "Wisconsin Department of Financial Institutions (Banking)", "Wisconsin Office of the Commissioner of Insurance", "Wisconsin Division of Securities", "https://www.wdfi.org", "https://www.wdfi.org/fi");
define_state!(dc_regulators, dc_license_types, "District of Columbia", "us-dc", "DC Department of Licensing and Consumer Protection", "DC Department of Insurance Securities and Banking", "DC Department of Insurance Securities and Banking", "DC Department of Insurance Securities and Banking Securities", "https://dcra.dc.gov", "https://disb.dc.gov");

// ── US Territories ───────────────────────────────────────────────────────────

define_state!(pr_regulators, pr_license_types, "Puerto Rico", "us-pr", "Puerto Rico Department of State", "Puerto Rico OCIF (Office of the Commissioner of Financial Institutions)", "Puerto Rico Office of the Insurance Commissioner", "Puerto Rico OCIF Securities", "https://www.estado.pr.gov", "https://ocif.pr.gov");
define_state!(gu_regulators, gu_license_types, "Guam", "us-gu", "Guam Department of Revenue and Taxation", "Guam Banking and Insurance Commissioner", "Guam Insurance Branch", "Guam Securities Division", "https://www.guamtax.com", "https://banking.guam.gov");
define_state!(vi_regulators, vi_license_types, "US Virgin Islands", "us-vi", "USVI Office of the Lieutenant Governor", "USVI Division of Banking Insurance and Financial Regulation", "USVI Division of Banking Insurance", "USVI Securities Division", "https://ltg.gov.vi", "https://ltg.gov.vi/division-of-banking");
define_state!(as_regulators, as_license_types, "American Samoa", "us-as", "American Samoa Commerce Department", "American Samoa Banking Board", "American Samoa Insurance Commissioner", "American Samoa Securities", "https://www.americansamoa.gov", "https://www.americansamoa.gov");
define_state!(mp_regulators, mp_license_types, "Northern Mariana Islands", "us-mp", "CNMI Department of Commerce", "CNMI Banking Commissioner", "CNMI Insurance Commissioner", "CNMI Securities Division", "https://commerce.gov.mp", "https://commerce.gov.mp");

// ── Aggregation ──────────────────────────────────────────────────────────────

/// All United States regulators across all 50 states, DC, and territories.
pub fn us_regulators() -> Vec<LicensepackRegulator> {
    let mut all = Vec::new();
    all.extend(ny_regulators()); all.extend(ca_regulators()); all.extend(de_regulators());
    all.extend(wy_regulators()); all.extend(tx_regulators()); all.extend(fl_regulators());
    all.extend(nv_regulators()); all.extend(il_regulators());
    all.extend(al_regulators()); all.extend(ak_regulators()); all.extend(az_regulators());
    all.extend(ar_regulators()); all.extend(co_regulators()); all.extend(ct_regulators());
    all.extend(ga_regulators()); all.extend(hi_regulators()); all.extend(id_regulators());
    all.extend(in_regulators()); all.extend(ia_regulators()); all.extend(ks_regulators());
    all.extend(ky_regulators()); all.extend(la_regulators()); all.extend(me_regulators());
    all.extend(md_regulators()); all.extend(ma_regulators()); all.extend(mi_regulators());
    all.extend(mn_regulators()); all.extend(ms_regulators()); all.extend(mo_regulators());
    all.extend(mt_regulators()); all.extend(ne_regulators()); all.extend(nh_regulators());
    all.extend(nj_regulators()); all.extend(nm_regulators()); all.extend(nc_regulators());
    all.extend(nd_regulators()); all.extend(oh_regulators()); all.extend(ok_regulators());
    all.extend(or_regulators()); all.extend(pa_regulators()); all.extend(ri_regulators());
    all.extend(sc_regulators()); all.extend(sd_regulators()); all.extend(tn_regulators());
    all.extend(ut_regulators()); all.extend(vt_regulators()); all.extend(va_regulators());
    all.extend(wa_regulators()); all.extend(wv_regulators()); all.extend(wi_regulators());
    all.extend(dc_regulators()); all.extend(pr_regulators()); all.extend(gu_regulators());
    all.extend(vi_regulators()); all.extend(as_regulators()); all.extend(mp_regulators());
    all
}

/// All United States license type definitions across all jurisdictions.
pub fn us_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(ny_license_types()); all.extend(ca_license_types()); all.extend(de_license_types());
    all.extend(wy_license_types()); all.extend(tx_license_types()); all.extend(fl_license_types());
    all.extend(nv_license_types()); all.extend(il_license_types());
    all.extend(al_license_types()); all.extend(ak_license_types()); all.extend(az_license_types());
    all.extend(ar_license_types()); all.extend(co_license_types()); all.extend(ct_license_types());
    all.extend(ga_license_types()); all.extend(hi_license_types()); all.extend(id_license_types());
    all.extend(in_license_types()); all.extend(ia_license_types()); all.extend(ks_license_types());
    all.extend(ky_license_types()); all.extend(la_license_types()); all.extend(me_license_types());
    all.extend(md_license_types()); all.extend(ma_license_types()); all.extend(mi_license_types());
    all.extend(mn_license_types()); all.extend(ms_license_types()); all.extend(mo_license_types());
    all.extend(mt_license_types()); all.extend(ne_license_types()); all.extend(nh_license_types());
    all.extend(nj_license_types()); all.extend(nm_license_types()); all.extend(nc_license_types());
    all.extend(nd_license_types()); all.extend(oh_license_types()); all.extend(ok_license_types());
    all.extend(or_license_types()); all.extend(pa_license_types()); all.extend(ri_license_types());
    all.extend(sc_license_types()); all.extend(sd_license_types()); all.extend(tn_license_types());
    all.extend(ut_license_types()); all.extend(vt_license_types()); all.extend(va_license_types());
    all.extend(wa_license_types()); all.extend(wv_license_types()); all.extend(wi_license_types());
    all.extend(dc_license_types()); all.extend(pr_license_types()); all.extend(gu_license_types());
    all.extend(vi_license_types()); all.extend(as_license_types()); all.extend(mp_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn us_has_56_jurisdictions() {
        let regs = us_regulators();
        // 8 detailed states (4 regulators each) + 48 standard states (4 each) = 224
        assert!(regs.len() >= 224, "expected >= 224 regulators, got {}", regs.len());
    }

    #[test]
    fn all_regulators_have_us_jurisdiction() {
        for reg in us_regulators() {
            assert!(
                reg.jurisdiction_id.starts_with("us"),
                "{} has unexpected jurisdiction_id: {}",
                reg.regulator_id, reg.jurisdiction_id
            );
        }
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = us_license_types();
        let mut ids = std::collections::HashSet::new();
        for lt in &types {
            assert!(ids.insert(&lt.license_type_id), "duplicate: {}", lt.license_type_id);
        }
    }

    #[test]
    fn key_states_have_enhanced_coverage() {
        let ny = ny_license_types();
        assert!(ny.len() >= 9, "NY should have >= 9 license types, got {}", ny.len());
        let ids: Vec<&str> = ny.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"us-ny-dfs:bitlicense"), "NY missing BitLicense");

        let ca = ca_license_types();
        assert!(ca.len() >= 8, "CA should have >= 8 license types, got {}", ca.len());

        let wy = wy_license_types();
        let wy_ids: Vec<&str> = wy.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(wy_ids.contains(&"us-wy-sos:dao-llc"), "WY missing DAO LLC");
        assert!(wy_ids.contains(&"us-wy-dob:spdi-charter"), "WY missing SPDI");
    }

    #[test]
    fn all_license_types_have_valid_fields() {
        for lt in us_license_types() {
            assert!(!lt.license_type_id.is_empty(), "empty license_type_id");
            assert!(!lt.name.is_empty(), "empty name for {}", lt.license_type_id);
            assert!(!lt.description.is_empty(), "empty desc for {}", lt.license_type_id);
            assert!(!lt.regulator_id.is_empty(), "empty reg_id for {}", lt.license_type_id);
            assert!(lt.category.is_some(), "missing category for {}", lt.license_type_id);
            assert!(!lt.permitted_activities.is_empty(), "no activities for {}", lt.license_type_id);
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for r in us_regulators().into_iter().take(10) {
            let json = serde_json::to_string(&r).expect("serialize");
            let d: LicensepackRegulator = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(r.regulator_id, d.regulator_id);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in us_license_types().into_iter().take(10) {
            let json = serde_json::to_string(&lt).expect("serialize");
            let d: LicenseTypeDefinition = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, d.license_type_id);
        }
    }
}
