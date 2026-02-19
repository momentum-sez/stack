//! # Pakistan Regulatory Authority License Mappings (S-013)
//!
//! Pakistan-specific license type definitions covering the five major
//! regulatory authorities referenced in the GovOS architecture:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **SECP** | Securities and Exchange Commission of Pakistan | Financial, Corporate |
//! | **SBP** | State Bank of Pakistan | Financial, Banking |
//! | **PTA** | Pakistan Telecommunication Authority | Trade (telecom) |
//! | **PEMRA** | Pakistan Electronic Media Regulatory Authority | Trade (media) |
//! | **DRAP** | Drug Regulatory Authority of Pakistan | Professional (pharma) |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Pakistan-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── SECP — Securities and Exchange Commission of Pakistan ───────────────────

/// SECP regulator profile.
pub fn secp_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-secp".to_string(),
        name: "Securities and Exchange Commission of Pakistan".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.secp.gov.pk".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string(), "filing_status".to_string()],
    }
}

/// SECP license type definitions.
pub fn secp_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-secp:company-registration".to_string(),
            name: "Company Registration".to_string(),
            description: "Registration of a company under the Companies Act 2017".to_string(),
            regulator_id: "pk-secp".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "capital_raising".to_string(),
            ],
            requirements: [
                ("minimum_directors".to_string(), serde_json::json!("2 for private, 3 for public")),
                ("registered_office".to_string(), serde_json::json!("Required within Pakistan")),
                ("memorandum_of_association".to_string(), serde_json::json!("Required")),
                ("articles_of_association".to_string(), serde_json::json!("Required")),
                ("cnic_of_directors".to_string(), serde_json::json!("NADRA CNIC for all directors")),
                ("ntn_required".to_string(), serde_json::json!("NTN from FBR required at incorporation")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pk-secp:securities-broker".to_string(),
            name: "Securities Broker License".to_string(),
            description: "License to operate as a securities broker on PSX".to_string(),
            regulator_id: "pk-secp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "trading".to_string(),
                "custody_services".to_string(),
            ],
            requirements: [
                ("minimum_net_worth_pkr".to_string(), serde_json::json!("25000000")),
                ("minimum_capital_pkr".to_string(), serde_json::json!("15000000")),
                ("fit_and_proper".to_string(), serde_json::json!("Directors must pass SECP fit & proper test")),
                ("psx_membership".to_string(), serde_json::json!("Trading Right Entitlement (TRE) certificate required")),
                ("compliance_officer".to_string(), serde_json::json!("Designated compliance officer required")),
                ("net_capital_balance".to_string(), serde_json::json!("Monthly NCB certificate to SECP")),
                ("professional_qualification".to_string(), serde_json::json!("ICAP/ICMAP or equivalent for principal")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-secp:nbfc".to_string(),
            name: "Non-Banking Finance Company License".to_string(),
            description: "License for non-banking financial companies under NBFC Rules 2003"
                .to_string(),
            regulator_id: "pk-secp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "lending".to_string(),
                "leasing".to_string(),
                "investment_advisory".to_string(),
                "asset_management".to_string(),
            ],
            requirements: [
                ("minimum_equity_pkr".to_string(), serde_json::json!("200000000")),
                ("minimum_capital_adequacy".to_string(), serde_json::json!("0.15")),
                ("fit_and_proper".to_string(), serde_json::json!("SECP fit & proper criteria for directors and CEO")),
                ("aml_compliance_program".to_string(), serde_json::json!("AML/CFT compliance program per SECP AML Regulations 2018")),
                ("risk_management_framework".to_string(), serde_json::json!("Board-approved risk management policy required")),
                ("internal_audit".to_string(), serde_json::json!("Internal audit function required")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "200000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-secp:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to carry on insurance business under Insurance Ordinance 2000"
                .to_string(),
            regulator_id: "pk-secp".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "general_insurance".to_string(),
                "reinsurance".to_string(),
            ],
            requirements: [
                ("minimum_paid_up_capital_life_pkr".to_string(), serde_json::json!("700000000")),
                ("minimum_paid_up_capital_general_pkr".to_string(), serde_json::json!("500000000")),
                ("statutory_deposit_pkr".to_string(), serde_json::json!("20000000")),
                ("actuarial_valuation".to_string(), serde_json::json!("Annual actuarial valuation required")),
                ("solvency_margin".to_string(), serde_json::json!("Per Insurance Ordinance 2000 Schedule I")),
                ("fit_and_proper".to_string(), serde_json::json!("Directors must meet SECP fit & proper criteria")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "250000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── SBP — State Bank of Pakistan ────────────────────────────────────────────

/// SBP regulator profile.
pub fn sbp_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-sbp".to_string(),
        name: "State Bank of Pakistan".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.sbp.org.pk".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string(), "raast_integration".to_string()],
    }
}

/// SBP license type definitions.
pub fn sbp_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-sbp:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description:
                "License to operate as a scheduled bank under Banking Companies Ordinance 1962"
                    .to_string(),
            regulator_id: "pk-sbp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
                "payment_services".to_string(),
            ],
            requirements: [
                ("minimum_paid_up_capital_pkr".to_string(), serde_json::json!("10000000000")),
                ("capital_adequacy_ratio".to_string(), serde_json::json!("0.115")),
                ("tier_1_capital_ratio".to_string(), serde_json::json!("0.06")),
                ("liquidity_coverage_ratio".to_string(), serde_json::json!("1.00")),
                ("fit_and_proper".to_string(), serde_json::json!("SBP Fit & Proper Criteria (BPRD Circular 2019)")),
                ("aml_compliance_program".to_string(), serde_json::json!("Per SBP AML/CFT Regulations 2020")),
                ("deposit_protection".to_string(), serde_json::json!("Member of Deposit Protection Corporation")),
                ("external_audit".to_string(), serde_json::json!("SBP-approved auditors (Panel A/B)")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "5000000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "2000000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pk-sbp:microfinance-bank".to_string(),
            name: "Microfinance Banking License".to_string(),
            description: "License to operate as a microfinance bank under MFI Ordinance 2001"
                .to_string(),
            regulator_id: "pk-sbp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "micro_lending".to_string(),
                "micro_deposit_taking".to_string(),
                "branchless_banking".to_string(),
            ],
            requirements: [
                ("minimum_paid_up_capital_national_pkr".to_string(), serde_json::json!("1000000000")),
                ("minimum_paid_up_capital_provincial_pkr".to_string(), serde_json::json!("600000000")),
                ("minimum_paid_up_capital_district_pkr".to_string(), serde_json::json!("300000000")),
                ("capital_adequacy_ratio".to_string(), serde_json::json!("0.15")),
                ("maximum_loan_size_pkr".to_string(), serde_json::json!("1000000")),
                ("fit_and_proper".to_string(), serde_json::json!("SBP Fit & Proper Criteria")),
                ("branchless_banking_regulations".to_string(), serde_json::json!("SBP Branchless Banking Regulations 2011")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "2000000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pk-sbp:emi".to_string(),
            name: "Electronic Money Institution License".to_string(),
            description: "License to issue electronic money under SBP EMI Regulations 2019"
                .to_string(),
            regulator_id: "pk-sbp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "payment_services".to_string(),
                "digital_wallet".to_string(),
            ],
            requirements: [
                ("minimum_paid_up_capital_pkr".to_string(), serde_json::json!("200000000")),
                ("float_safeguarding".to_string(), serde_json::json!("100% of outstanding e-money held in trust account at scheduled bank")),
                ("technology_infrastructure".to_string(), serde_json::json!("PCI-DSS certified, SBP IT framework compliance")),
                ("kyc_cdd".to_string(), serde_json::json!("Tiered KYC per SBP AML Regulations — biometric verification via NADRA")),
                ("agent_network".to_string(), serde_json::json!("Agent banking regulations compliance per SBP Branchless Banking Regulations")),
                ("interoperability".to_string(), serde_json::json!("Raast P2P/P2M integration required per SBP circular")),
                ("reporting".to_string(), serde_json::json!("Quarterly compliance report to SBP Payment Systems Department")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "1000000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-sbp:exchange-company".to_string(),
            name: "Exchange Company License".to_string(),
            description: "License to operate foreign exchange business under FERA 1947 / Exchange Companies (Conduct of Business) Regulations"
                .to_string(),
            regulator_id: "pk-sbp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec!["foreign_exchange".to_string(), "remittance".to_string()],
            requirements: [
                ("minimum_paid_up_capital_category_a_pkr".to_string(), serde_json::json!("200000000")),
                ("minimum_paid_up_capital_category_b_pkr".to_string(), serde_json::json!("100000000")),
                ("security_deposit_pkr".to_string(), serde_json::json!("10000000")),
                ("aml_compliance".to_string(), serde_json::json!("Full AML/CFT program per SBP Exchange Company regulations")),
                ("transaction_limits".to_string(), serde_json::json!("Per SBP foreign exchange manual limits")),
                ("reporting".to_string(), serde_json::json!("Monthly forex position report to SBP")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "2000000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "1000000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(3),
        },
    ]
}

// ── PTA — Pakistan Telecommunication Authority ──────────────────────────────

/// PTA regulator profile.
pub fn pta_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-pta".to_string(),
        name: "Pakistan Telecommunication Authority".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.pta.gov.pk".to_string()),
        did: None,
        api_capabilities: vec!["license_query".to_string()],
    }
}

/// PTA license type definitions.
pub fn pta_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-pta:cmto".to_string(),
            name: "Cellular Mobile Telecom Operator License".to_string(),
            description: "License to operate cellular mobile services in Pakistan".to_string(),
            regulator_id: "pk-pta".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "cellular_services".to_string(),
                "mobile_broadband".to_string(),
                "mobile_financial_services".to_string(),
            ],
            requirements: [
                ("spectrum_auction".to_string(), serde_json::json!("Spectrum acquired through PTA auction")),
                ("network_rollout".to_string(), serde_json::json!("Coverage obligations per license conditions")),
                ("quality_of_service".to_string(), serde_json::json!("QoS parameters per PTA regulations")),
                ("sim_registration".to_string(), serde_json::json!("Biometric verification for SIM issuance (PTA directive)")),
                ("lawful_intercept".to_string(), serde_json::json!("Lawful intercept capability per PTA/MoIT")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "varies_by_spectrum".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "varies_by_revenue".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(15),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-pta:isp".to_string(),
            name: "Internet Service Provider License".to_string(),
            description: "License to provide internet services under Pakistan Telecom Authority regulations"
                .to_string(),
            regulator_id: "pk-pta".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "internet_services".to_string(),
                "web_hosting".to_string(),
                "data_center_operations".to_string(),
            ],
            requirements: [
                ("company_registration".to_string(), serde_json::json!("SECP-registered company")),
                ("bandwidth_agreement".to_string(), serde_json::json!("Agreement with licensed backbone provider")),
                ("content_filtering".to_string(), serde_json::json!("PTA-mandated URL filtering system")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "250000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(10),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-pta:llo".to_string(),
            name: "Local Loop Operator License".to_string(),
            description: "License to operate local loop telecommunications".to_string(),
            regulator_id: "pk-pta".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "fixed_line_services".to_string(),
                "broadband_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(20),
        },
    ]
}

// ── PEMRA — Pakistan Electronic Media Regulatory Authority ──────────────────

/// PEMRA regulator profile.
pub fn pemra_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-pemra".to_string(),
        name: "Pakistan Electronic Media Regulatory Authority".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.pemra.gov.pk".to_string()),
        did: None,
        api_capabilities: vec!["license_status".to_string()],
    }
}

/// PEMRA license type definitions.
pub fn pemra_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-pemra:satellite-tv".to_string(),
            name: "Satellite TV Channel License".to_string(),
            description: "License to operate a satellite TV channel".to_string(),
            regulator_id: "pk-pemra".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "tv_broadcasting".to_string(),
                "content_production".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("PKR".to_string(), "1000000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "5000000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(15),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-pemra:fm-radio".to_string(),
            name: "FM Radio License".to_string(),
            description: "License to operate an FM radio station".to_string(),
            regulator_id: "pk-pemra".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "radio_broadcasting".to_string(),
                "content_production".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("PKR".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "1000000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(10),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-pemra:cable-tv".to_string(),
            name: "Cable TV Distribution License".to_string(),
            description: "License to distribute cable TV services".to_string(),
            regulator_id: "pk-pemra".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec!["cable_distribution".to_string()],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
    ]
}

// ── DRAP — Drug Regulatory Authority of Pakistan ────────────────────────────

/// DRAP regulator profile.
pub fn drap_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-drap".to_string(),
        name: "Drug Regulatory Authority of Pakistan".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.drap.gov.pk".to_string()),
        did: None,
        api_capabilities: vec!["drug_registry".to_string()],
    }
}

/// DRAP license type definitions.
pub fn drap_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-drap:drug-manufacturing".to_string(),
            name: "Drug Manufacturing License".to_string(),
            description: "License to manufacture pharmaceutical drugs under DRAP Act 2012"
                .to_string(),
            regulator_id: "pk-drap".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "drug_manufacturing".to_string(),
                "quality_testing".to_string(),
            ],
            requirements: [
                ("gmp_compliance".to_string(), serde_json::json!("WHO-GMP or PIC/S GMP certification")),
                ("qualified_person".to_string(), serde_json::json!("Pharmacist with Pharmacy Council registration")),
                ("quality_control_lab".to_string(), serde_json::json!("In-house QC laboratory with DRAP-approved equipment")),
                ("premises_inspection".to_string(), serde_json::json!("DRAP site inspection and approval")),
            ]
                .into_iter()
                .collect(),
            application_fee: [("PKR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "200000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-drap:drug-import".to_string(),
            name: "Drug Import License".to_string(),
            description: "License to import pharmaceutical drugs".to_string(),
            regulator_id: "pk-drap".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec!["drug_import".to_string(), "drug_distribution".to_string()],
            requirements: BTreeMap::new(),
            application_fee: [("PKR".to_string(), "50000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("PKR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "pk-drap:medical-device".to_string(),
            name: "Medical Device Registration".to_string(),
            description: "Registration of medical devices for sale in Pakistan".to_string(),
            regulator_id: "pk-drap".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "medical_device_sale".to_string(),
                "medical_device_import".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("PKR".to_string(), "75000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
    ]
}

// ── FBR — Federal Board of Revenue ───────────────────────────────────────────

/// FBR regulator profile (tax registration authority).
pub fn fbr_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pk-fbr".to_string(),
        name: "Federal Board of Revenue".to_string(),
        jurisdiction_id: "pk".to_string(),
        registry_url: Some("https://www.fbr.gov.pk".to_string()),
        did: None,
        api_capabilities: vec![
            "ntn_verification".to_string(),
            "active_taxpayer_list".to_string(),
            "iris_efiling".to_string(),
        ],
    }
}

/// FBR registration type definitions.
pub fn fbr_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pk-fbr:ntn".to_string(),
            name: "National Tax Number (NTN) Registration".to_string(),
            description: "Registration with FBR for income tax purposes under ITO 2001".to_string(),
            regulator_id: "pk-fbr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "income_tax_filing".to_string(),
                "withholding_agent".to_string(),
                "government_contracts".to_string(),
            ],
            requirements: [
                ("cnic".to_string(), serde_json::json!("NADRA CNIC of all directors/partners")),
                ("company_registration".to_string(), serde_json::json!("SECP certificate of incorporation (for companies)")),
                ("registered_address".to_string(), serde_json::json!("Utility bill or rent agreement for business premises")),
                ("bank_account".to_string(), serde_json::json!("Business bank account statement")),
            ]
                .into_iter()
                .collect(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pk-fbr:strn".to_string(),
            name: "Sales Tax Registration Number (STRN)".to_string(),
            description: "Registration for sales tax under Sales Tax Act 1990".to_string(),
            regulator_id: "pk-fbr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "sales_tax_collection".to_string(),
                "input_tax_credit".to_string(),
                "sales_tax_filing".to_string(),
            ],
            requirements: [
                ("ntn".to_string(), serde_json::json!("Active NTN required")),
                ("business_premises".to_string(), serde_json::json!("Premises inspection by FBR / provincial revenue authority")),
                ("monthly_return_filing".to_string(), serde_json::json!("Monthly sales tax return by 18th of following month")),
            ]
                .into_iter()
                .collect(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Pakistan Registry Aggregation ───────────────────────────────────────────

/// All Pakistan regulatory authorities.
pub fn pakistan_regulators() -> Vec<LicensepackRegulator> {
    vec![
        secp_regulator(),
        sbp_regulator(),
        pta_regulator(),
        pemra_regulator(),
        drap_regulator(),
        fbr_regulator(),
    ]
}

/// All Pakistan license type definitions across all authorities.
pub fn pakistan_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(secp_license_types());
    all.extend(sbp_license_types());
    all.extend(pta_license_types());
    all.extend(pemra_license_types());
    all.extend(drap_license_types());
    all.extend(fbr_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pakistan_has_six_regulators() {
        let regs = pakistan_regulators();
        assert_eq!(regs.len(), 6);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"pk-secp"), "missing SECP");
        assert!(ids.contains(&"pk-sbp"), "missing SBP");
        assert!(ids.contains(&"pk-pta"), "missing PTA");
        assert!(ids.contains(&"pk-pemra"), "missing PEMRA");
        assert!(ids.contains(&"pk-drap"), "missing DRAP");
        assert!(ids.contains(&"pk-fbr"), "missing FBR");
    }

    #[test]
    fn all_regulators_are_pakistan_jurisdiction() {
        for reg in pakistan_regulators() {
            assert_eq!(reg.jurisdiction_id, "pk", "{} is not pk", reg.regulator_id);
        }
    }

    #[test]
    fn pakistan_license_types_cover_all_authorities() {
        let types = pakistan_license_types();
        assert!(
            types.len() >= 19,
            "expected >= 19 license types, got {}",
            types.len()
        );

        // Verify each authority has at least one license type.
        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("pk-secp"), "no SECP license types");
        assert!(authority_ids.contains("pk-sbp"), "no SBP license types");
        assert!(authority_ids.contains("pk-pta"), "no PTA license types");
        assert!(authority_ids.contains("pk-pemra"), "no PEMRA license types");
        assert!(authority_ids.contains("pk-drap"), "no DRAP license types");
        assert!(authority_ids.contains("pk-fbr"), "no FBR license types");
    }

    #[test]
    fn secp_has_company_registration_and_broker() {
        let types = secp_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-secp:company-registration"));
        assert!(ids.contains(&"pk-secp:securities-broker"));
        assert!(ids.contains(&"pk-secp:nbfc"));
        assert!(ids.contains(&"pk-secp:insurance"));
    }

    #[test]
    fn sbp_has_banking_and_emi() {
        let types = sbp_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-sbp:commercial-bank"));
        assert!(ids.contains(&"pk-sbp:microfinance-bank"));
        assert!(ids.contains(&"pk-sbp:emi"));
        assert!(ids.contains(&"pk-sbp:exchange-company"));
    }

    #[test]
    fn pta_has_telecom_licenses() {
        let types = pta_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-pta:cmto"));
        assert!(ids.contains(&"pk-pta:isp"));
        assert!(ids.contains(&"pk-pta:llo"));
    }

    #[test]
    fn pemra_has_media_licenses() {
        let types = pemra_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-pemra:satellite-tv"));
        assert!(ids.contains(&"pk-pemra:fm-radio"));
        assert!(ids.contains(&"pk-pemra:cable-tv"));
    }

    #[test]
    fn drap_has_pharmaceutical_licenses() {
        let types = drap_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-drap:drug-manufacturing"));
        assert!(ids.contains(&"pk-drap:drug-import"));
        assert!(ids.contains(&"pk-drap:medical-device"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = pakistan_license_types();
        let mut ids = std::collections::HashSet::new();
        for lt in &types {
            assert!(
                ids.insert(&lt.license_type_id),
                "duplicate license_type_id: {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn all_license_types_have_valid_fields() {
        for lt in pakistan_license_types() {
            assert!(
                !lt.license_type_id.is_empty(),
                "empty license_type_id for {}",
                lt.name
            );
            assert!(!lt.name.is_empty(), "empty name for {}", lt.license_type_id);
            assert!(
                !lt.description.is_empty(),
                "empty description for {}",
                lt.license_type_id
            );
            assert!(
                !lt.regulator_id.is_empty(),
                "empty regulator_id for {}",
                lt.license_type_id
            );
            assert!(
                lt.category.is_some(),
                "missing category for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn financial_license_types_have_requirements() {
        // All financial/banking license types must have non-empty requirements
        let financial_types = pakistan_license_types()
            .into_iter()
            .filter(|lt| {
                lt.regulator_id == "pk-sbp"
                    || lt.regulator_id == "pk-secp"
            })
            .collect::<Vec<_>>();
        for lt in &financial_types {
            assert!(
                !lt.requirements.is_empty(),
                "{} has empty requirements — must specify capital/regulatory requirements",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn commercial_bank_has_capital_requirements() {
        let types = sbp_license_types();
        let bank = types
            .iter()
            .find(|t| t.license_type_id == "pk-sbp:commercial-bank")
            .expect("commercial-bank type missing");
        assert!(
            bank.requirements.contains_key("minimum_paid_up_capital_pkr"),
            "commercial bank must have minimum_paid_up_capital_pkr"
        );
        assert!(
            bank.requirements.contains_key("capital_adequacy_ratio"),
            "commercial bank must have CAR"
        );
    }

    #[test]
    fn emi_has_float_safeguarding() {
        let types = sbp_license_types();
        let emi = types
            .iter()
            .find(|t| t.license_type_id == "pk-sbp:emi")
            .expect("emi type missing");
        assert!(
            emi.requirements.contains_key("float_safeguarding"),
            "EMI must have float_safeguarding requirement"
        );
        assert!(
            emi.requirements.contains_key("interoperability"),
            "EMI must have Raast interoperability requirement"
        );
    }

    #[test]
    fn fbr_has_ntn_and_strn() {
        let types = fbr_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-fbr:ntn"), "missing NTN");
        assert!(ids.contains(&"pk-fbr:strn"), "missing STRN");
    }

    #[test]
    fn all_license_types_have_permitted_activities() {
        for lt in pakistan_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in pakistan_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in pakistan_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
