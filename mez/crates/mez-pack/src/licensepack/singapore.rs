//! # Singapore Regulatory Authority License Mappings
//!
//! Singapore-specific license type definitions covering the major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **MAS** | Monetary Authority of Singapore | Banking, Insurance, Capital Markets, Payments, Trust, FA, VCC, DPT |
//! | **ACRA** | Accounting and Corporate Regulatory Authority | Corporate Registration, CSP |
//! | **IMDA** | Infocomm Media Development Authority | Telecom |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Singapore-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── MAS — Monetary Authority of Singapore ─────────────────────────────────

/// MAS regulator profile.
pub fn mas_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sg-mas".to_string(),
        name: "Monetary Authority of Singapore".to_string(),
        jurisdiction_id: "sg".to_string(),
        registry_url: Some("https://www.mas.gov.sg".to_string()),
        did: None,
        api_capabilities: vec![
            "financial_institutions_directory".to_string(),
            "payment_services_registry".to_string(),
        ],
    }
}

/// MAS license type definitions.
pub fn mas_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        // ── Banking ────────────────────────────────────────────────────
        LicenseTypeDefinition {
            license_type_id: "sg-mas:full-bank".to_string(),
            name: "Full Bank License".to_string(),
            description: "License to operate as a full bank in Singapore under the Banking Act"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
                "payment_services".to_string(),
                "wealth_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:wholesale-bank".to_string(),
            name: "Wholesale Bank License".to_string(),
            description: "License to operate as a wholesale bank (no SGD retail deposits)"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "wholesale_deposit_taking".to_string(),
                "corporate_lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:merchant-bank".to_string(),
            name: "Merchant Bank License".to_string(),
            description: "License to operate as a merchant bank (approved status)".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "corporate_finance".to_string(),
                "underwriting".to_string(),
                "portfolio_management".to_string(),
                "mergers_acquisitions_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:digital-bank".to_string(),
            name: "Digital Bank License".to_string(),
            description:
                "License to operate as a digital full bank or digital wholesale bank"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_deposit_taking".to_string(),
                "digital_lending".to_string(),
                "digital_payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        // ── Insurance ──────────────────────────────────────────────────
        LicenseTypeDefinition {
            license_type_id: "sg-mas:direct-insurer".to_string(),
            name: "Direct Insurer License".to_string(),
            description: "License to carry on direct insurance business under the Insurance Act"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "general_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:reinsurer".to_string(),
            name: "Reinsurer License".to_string(),
            description: "License to carry on reinsurance business in Singapore".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "reinsurance".to_string(),
                "retrocession".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:insurance-broker".to_string(),
            name: "Insurance Broker Registration".to_string(),
            description: "Registration as an insurance broker under the Insurance Act".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_broking".to_string(),
                "risk_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:lloyds-asia".to_string(),
            name: "Lloyd's Asia Scheme Registration".to_string(),
            description: "Registration for Lloyd's syndicates to write business via Singapore"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "specialty_insurance".to_string(),
                "marine_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        // ── Capital Markets Services ───────────────────────────────────
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-dealing".to_string(),
            name: "CMS License — Dealing in Capital Markets Products".to_string(),
            description:
                "Capital markets services license for dealing in securities, futures, and OTC derivatives"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "futures_dealing".to_string(),
                "otc_derivatives_dealing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-advising".to_string(),
            name: "CMS License — Advising on Corporate Finance".to_string(),
            description:
                "Capital markets services license for advising on corporate finance matters"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "corporate_finance_advisory".to_string(),
                "ipo_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-fund-management".to_string(),
            name: "CMS License — Fund Management".to_string(),
            description: "Capital markets services license for fund management".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "discretionary_portfolio_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-reit-management".to_string(),
            name: "CMS License — REIT Management".to_string(),
            description: "Capital markets services license for managing REITs".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "reit_management".to_string(),
                "property_fund_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-credit-rating".to_string(),
            name: "CMS License — Providing Credit Rating Services".to_string(),
            description: "Capital markets services license for credit rating services".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "credit_rating".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:cms-custodial".to_string(),
            name: "CMS License — Providing Custodial Services".to_string(),
            description:
                "Capital markets services license for providing custodial services for securities"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_custody".to_string(),
                "fund_administration".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── Payment Services ───────────────────────────────────────────
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-money-changing".to_string(),
            name: "SPS License — Money-Changing Service".to_string(),
            description: "Standard payment institution license for money-changing services"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_changing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-cross-border-transfer".to_string(),
            name: "SPS License — Cross-Border Money Transfer".to_string(),
            description: "Payment services license for cross-border money transfer".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "cross_border_money_transfer".to_string(),
                "remittance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-domestic-transfer".to_string(),
            name: "SPS License — Domestic Money Transfer".to_string(),
            description: "Payment services license for domestic money transfer".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "domestic_money_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-merchant-acquisition".to_string(),
            name: "SPS License — Merchant Acquisition".to_string(),
            description: "Payment services license for merchant acquisition services".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "merchant_acquisition".to_string(),
                "payment_processing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-e-money-issuance".to_string(),
            name: "SPS License — E-Money Issuance".to_string(),
            description: "Payment services license for e-money issuance".to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "e_money_issuance".to_string(),
                "stored_value_facility".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-dpt".to_string(),
            name: "SPS License — Digital Payment Token Service".to_string(),
            description:
                "Payment services license for digital payment token (DPT) services under the Payment Services Act"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "dpt_dealing".to_string(),
                "dpt_exchange".to_string(),
                "dpt_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:sps-money-lending".to_string(),
            name: "SPS License — Money-Lending".to_string(),
            description: "License for money-lending activities under the Moneylenders Act"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_lending".to_string(),
                "personal_loans".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        // ── Trust, FA, VCC ─────────────────────────────────────────────
        LicenseTypeDefinition {
            license_type_id: "sg-mas:trust-company".to_string(),
            name: "Trust Company License".to_string(),
            description: "License to carry on trust business under the Trust Companies Act"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "trust_administration".to_string(),
                "estate_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:financial-adviser".to_string(),
            name: "Financial Adviser License".to_string(),
            description: "License to act as a financial adviser under the Financial Advisers Act"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "financial_advisory".to_string(),
                "investment_product_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:vcc".to_string(),
            name: "Variable Capital Company (VCC) Registration".to_string(),
            description: "Registration of a Variable Capital Company for fund structuring"
                .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_structuring".to_string(),
                "sub_fund_creation".to_string(),
                "umbrella_fund_operation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-mas:dpt-service-provider".to_string(),
            name: "Digital Payment Token Service Provider License".to_string(),
            description:
                "Full license for digital payment token service providers under the Payment Services Act"
                    .to_string(),
            regulator_id: "sg-mas".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "dpt_dealing".to_string(),
                "dpt_facilitation".to_string(),
                "dpt_custody".to_string(),
                "dpt_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── ACRA — Accounting and Corporate Regulatory Authority ──────────────────

/// ACRA regulator profile.
pub fn acra_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sg-acra".to_string(),
        name: "Accounting and Corporate Regulatory Authority".to_string(),
        jurisdiction_id: "sg".to_string(),
        registry_url: Some("https://www.acra.gov.sg".to_string()),
        did: None,
        api_capabilities: vec!["bizfile_search".to_string(), "entity_profile".to_string()],
    }
}

/// ACRA license type definitions.
pub fn acra_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "sg-acra:private-limited-company".to_string(),
            name: "Private Limited Company Registration".to_string(),
            description: "Registration of a private company limited by shares under the Companies Act"
                .to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "capital_raising_private".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "315".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "60".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:public-company".to_string(),
            name: "Public Company Registration".to_string(),
            description: "Registration of a public company limited by shares".to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "public_capital_raising".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "315".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "60".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:llp".to_string(),
            name: "Limited Liability Partnership (LLP) Registration".to_string(),
            description: "Registration of a limited liability partnership".to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "partnership_operations".to_string(),
                "professional_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "115".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "30".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:lp".to_string(),
            name: "Limited Partnership (LP) Registration".to_string(),
            description: "Registration of a limited partnership under the Limited Partnerships Act"
                .to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "partnership_operations".to_string(),
                "fund_vehicle".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "115".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "30".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:sole-proprietorship".to_string(),
            name: "Sole Proprietorship Registration".to_string(),
            description: "Registration of a sole proprietorship under the Business Names Registration Act"
                .to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "sole_trader_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "115".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "30".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:foreign-company".to_string(),
            name: "Foreign Company Registration".to_string(),
            description: "Registration of a foreign company branch in Singapore".to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "branch_operations".to_string(),
                "representative_office".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SGD".to_string(), "315".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SGD".to_string(), "60".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:registered-filing-agent".to_string(),
            name: "Registered Filing Agent".to_string(),
            description: "Registration as a filing agent authorized to file on behalf of entities"
                .to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "corporate_filing".to_string(),
                "annual_return_filing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "sg-acra:corporate-service-provider".to_string(),
            name: "Corporate Service Provider License".to_string(),
            description:
                "License to provide corporate secretarial and compliance services"
                    .to_string(),
            regulator_id: "sg-acra".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "corporate_secretarial".to_string(),
                "compliance_services".to_string(),
                "registered_office_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── IMDA — Infocomm Media Development Authority ──────────────────────────

/// IMDA regulator profile.
pub fn imda_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sg-imda".to_string(),
        name: "Infocomm Media Development Authority".to_string(),
        jurisdiction_id: "sg".to_string(),
        registry_url: Some("https://www.imda.gov.sg".to_string()),
        did: None,
        api_capabilities: vec!["telecom_licensee_directory".to_string()],
    }
}

/// IMDA license type definitions.
pub fn imda_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "sg-imda:facilities-based-operator".to_string(),
            name: "Facilities-Based Operator License".to_string(),
            description:
                "License to deploy and operate telecommunications network facilities in Singapore"
                    .to_string(),
            regulator_id: "sg-imda".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "network_infrastructure_deployment".to_string(),
                "facilities_based_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(20),
        },
        LicenseTypeDefinition {
            license_type_id: "sg-imda:services-based-operator".to_string(),
            name: "Services-Based Operator License".to_string(),
            description:
                "License to provide telecommunications services using third-party infrastructure"
                    .to_string(),
            regulator_id: "sg-imda".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "telecom_services_provision".to_string(),
                "voip_services".to_string(),
                "internet_access_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(10),
        },
    ]
}

// ── Singapore Registry Aggregation ────────────────────────────────────────

/// All Singapore regulatory authorities.
pub fn singapore_regulators() -> Vec<LicensepackRegulator> {
    vec![
        mas_regulator(),
        acra_regulator(),
        imda_regulator(),
    ]
}

/// All Singapore license type definitions across all authorities.
pub fn singapore_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(mas_license_types());
    all.extend(acra_license_types());
    all.extend(imda_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn singapore_has_three_regulators() {
        let regs = singapore_regulators();
        assert_eq!(regs.len(), 3);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas"), "missing MAS");
        assert!(ids.contains(&"sg-acra"), "missing ACRA");
        assert!(ids.contains(&"sg-imda"), "missing IMDA");
    }

    #[test]
    fn all_regulators_are_sg_jurisdiction() {
        for reg in singapore_regulators() {
            assert_eq!(reg.jurisdiction_id, "sg", "{} is not sg", reg.regulator_id);
        }
    }

    #[test]
    fn singapore_license_types_cover_all_authorities() {
        let types = singapore_license_types();
        assert!(
            types.len() >= 35,
            "expected >= 35 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("sg-mas"), "no MAS license types");
        assert!(authority_ids.contains("sg-acra"), "no ACRA license types");
        assert!(authority_ids.contains("sg-imda"), "no IMDA license types");
    }

    #[test]
    fn mas_has_banking_licenses() {
        let types = mas_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas:full-bank"));
        assert!(ids.contains(&"sg-mas:wholesale-bank"));
        assert!(ids.contains(&"sg-mas:merchant-bank"));
        assert!(ids.contains(&"sg-mas:digital-bank"));
    }

    #[test]
    fn mas_has_insurance_licenses() {
        let types = mas_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas:direct-insurer"));
        assert!(ids.contains(&"sg-mas:reinsurer"));
        assert!(ids.contains(&"sg-mas:insurance-broker"));
        assert!(ids.contains(&"sg-mas:lloyds-asia"));
    }

    #[test]
    fn mas_has_cms_licenses() {
        let types = mas_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas:cms-dealing"));
        assert!(ids.contains(&"sg-mas:cms-advising"));
        assert!(ids.contains(&"sg-mas:cms-fund-management"));
        assert!(ids.contains(&"sg-mas:cms-reit-management"));
        assert!(ids.contains(&"sg-mas:cms-credit-rating"));
        assert!(ids.contains(&"sg-mas:cms-custodial"));
    }

    #[test]
    fn mas_has_payment_services_licenses() {
        let types = mas_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas:sps-money-changing"));
        assert!(ids.contains(&"sg-mas:sps-cross-border-transfer"));
        assert!(ids.contains(&"sg-mas:sps-domestic-transfer"));
        assert!(ids.contains(&"sg-mas:sps-merchant-acquisition"));
        assert!(ids.contains(&"sg-mas:sps-e-money-issuance"));
        assert!(ids.contains(&"sg-mas:sps-dpt"));
        assert!(ids.contains(&"sg-mas:sps-money-lending"));
    }

    #[test]
    fn mas_has_trust_fa_vcc_dpt() {
        let types = mas_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas:trust-company"));
        assert!(ids.contains(&"sg-mas:financial-adviser"));
        assert!(ids.contains(&"sg-mas:vcc"));
        assert!(ids.contains(&"sg-mas:dpt-service-provider"));
    }

    #[test]
    fn acra_has_entity_registrations() {
        let types = acra_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-acra:private-limited-company"));
        assert!(ids.contains(&"sg-acra:public-company"));
        assert!(ids.contains(&"sg-acra:llp"));
        assert!(ids.contains(&"sg-acra:lp"));
        assert!(ids.contains(&"sg-acra:sole-proprietorship"));
        assert!(ids.contains(&"sg-acra:foreign-company"));
        assert!(ids.contains(&"sg-acra:registered-filing-agent"));
        assert!(ids.contains(&"sg-acra:corporate-service-provider"));
    }

    #[test]
    fn imda_has_telecom_licenses() {
        let types = imda_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-imda:facilities-based-operator"));
        assert!(ids.contains(&"sg-imda:services-based-operator"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = singapore_license_types();
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
        for lt in singapore_license_types() {
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
    fn all_license_types_have_permitted_activities() {
        for lt in singapore_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in singapore_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in singapore_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
