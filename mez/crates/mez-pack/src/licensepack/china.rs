//! # China Regulatory Authority License Mappings
//!
//! China-specific license type definitions covering national and sub-national
//! regulatory authorities across key economic zones:
//!
//! ## National Level (jurisdiction: `cn`)
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **PBOC** | People's Bank of China | Banking, Payment Services |
//! | **NFRA** | National Financial Regulatory Administration | Banking Supervision, Insurance |
//! | **CSRC** | China Securities Regulatory Commission | Securities, Funds, Futures |
//! | **SAFE** | State Administration of Foreign Exchange | Forex |
//!
//! ## Sub-National Zones
//!
//! | Zone | Jurisdiction ID | Key Authority |
//! |------|-----------------|---------------|
//! | Hainan Free Trade Port | `cn-hainan` | Hainan FTP Administration |
//! | Hangzhou | `cn-hangzhou` | Hangzhou Commerce Bureau |
//! | Shenzhen EZ | `cn-shenzhen` | Shenzhen Financial Regulatory Bureau |
//! | Shanghai | `cn-shanghai` | Shanghai FTZ Administration, Shanghai Financial Regulatory Bureau |
//! | Beijing | `cn-beijing` | Beijing Financial Regulatory Bureau |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── PBOC — People's Bank of China ─────────────────────────────────────────

/// PBOC regulator profile.
pub fn pboc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-pboc".to_string(),
        name: "People's Bank of China".to_string(),
        jurisdiction_id: "cn".to_string(),
        registry_url: Some("http://www.pbc.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["institution_query".to_string(), "payment_license_registry".to_string()],
    }
}

/// PBOC license type definitions.
pub fn pboc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-pboc:payment-institution".to_string(),
            name: "Payment Institution License".to_string(),
            description: "License to operate non-bank payment services under PBOC regulations"
                .to_string(),
            regulator_id: "cn-pboc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "internet_payment".to_string(),
                "mobile_payment".to_string(),
                "prepaid_card_issuance".to_string(),
                "bank_card_acquiring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-pboc:cross-border-payment".to_string(),
            name: "Cross-Border Payment License".to_string(),
            description: "License for cross-border RMB payment and settlement services"
                .to_string(),
            regulator_id: "cn-pboc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "cross_border_rmb_settlement".to_string(),
                "trade_finance_payment".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-pboc:credit-reporting".to_string(),
            name: "Credit Reporting License".to_string(),
            description: "License to operate credit reporting services under PBOC oversight"
                .to_string(),
            regulator_id: "cn-pboc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "personal_credit_reporting".to_string(),
                "enterprise_credit_reporting".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── NFRA — National Financial Regulatory Administration ───────────────────

/// NFRA regulator profile (formerly CBIRC).
pub fn nfra_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-nfra".to_string(),
        name: "National Financial Regulatory Administration".to_string(),
        jurisdiction_id: "cn".to_string(),
        registry_url: Some("https://www.nfra.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string(), "insurance_registry".to_string()],
    }
}

/// NFRA license type definitions.
pub fn nfra_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-nfra:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "License to operate as a commercial bank under NFRA supervision"
                .to_string(),
            regulator_id: "cn-nfra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "settlement".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-nfra:insurance-company".to_string(),
            name: "Insurance Company License".to_string(),
            description: "License to operate an insurance company in China".to_string(),
            regulator_id: "cn-nfra".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "property_insurance".to_string(),
                "reinsurance".to_string(),
                "health_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-nfra:trust-company".to_string(),
            name: "Trust Company License".to_string(),
            description: "License to operate a trust company under NFRA regulations".to_string(),
            regulator_id: "cn-nfra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "trust_management".to_string(),
                "wealth_management".to_string(),
                "asset_securitization".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-nfra:consumer-finance".to_string(),
            name: "Consumer Finance Company License".to_string(),
            description: "License to provide consumer lending services".to_string(),
            regulator_id: "cn-nfra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "consumer_lending".to_string(),
                "personal_credit".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── CSRC — China Securities Regulatory Commission ─────────────────────────

/// CSRC regulator profile.
pub fn csrc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-csrc".to_string(),
        name: "China Securities Regulatory Commission".to_string(),
        jurisdiction_id: "cn".to_string(),
        registry_url: Some("http://www.csrc.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["securities_registry".to_string(), "fund_registry".to_string()],
    }
}

/// CSRC license type definitions.
pub fn csrc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-csrc:securities-company".to_string(),
            name: "Securities Company License".to_string(),
            description: "License to operate as a securities company (broker-dealer)".to_string(),
            regulator_id: "cn-csrc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "securities_underwriting".to_string(),
                "proprietary_trading".to_string(),
                "asset_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-csrc:fund-management".to_string(),
            name: "Fund Management Company License".to_string(),
            description: "License to manage public and private investment funds".to_string(),
            regulator_id: "cn-csrc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "public_fund_management".to_string(),
                "private_fund_management".to_string(),
                "investment_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-csrc:futures-company".to_string(),
            name: "Futures Company License".to_string(),
            description: "License to operate as a futures brokerage company".to_string(),
            regulator_id: "cn-csrc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "futures_brokerage".to_string(),
                "commodity_futures".to_string(),
                "financial_futures".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-csrc:investment-advisory".to_string(),
            name: "Securities Investment Advisory License".to_string(),
            description: "License to provide securities investment advisory services".to_string(),
            regulator_id: "cn-csrc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_consulting".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── SAFE — State Administration of Foreign Exchange ───────────────────────

/// SAFE regulator profile.
pub fn safe_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-safe".to_string(),
        name: "State Administration of Foreign Exchange".to_string(),
        jurisdiction_id: "cn".to_string(),
        registry_url: Some("https://www.safe.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["forex_registry".to_string()],
    }
}

/// SAFE license type definitions.
pub fn safe_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-safe:forex-trading".to_string(),
            name: "Foreign Exchange Trading License".to_string(),
            description: "License to conduct foreign exchange trading and settlement".to_string(),
            regulator_id: "cn-safe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "forex_trading".to_string(),
                "forex_settlement".to_string(),
                "cross_border_capital".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-safe:cross-border-investment".to_string(),
            name: "Cross-Border Investment Registration".to_string(),
            description: "Registration for qualified cross-border investment activities"
                .to_string(),
            regulator_id: "cn-safe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "outbound_direct_investment".to_string(),
                "inbound_direct_investment".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Hainan FTP Administration ─────────────────────────────────────────────

/// Hainan Free Trade Port Administration regulator profile.
pub fn hainan_ftp_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-hainan-ftp".to_string(),
        name: "Hainan Free Trade Port Administration".to_string(),
        jurisdiction_id: "cn-hainan".to_string(),
        registry_url: Some("https://www.hainan.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["ftp_business_registry".to_string()],
    }
}

/// Hainan FTP license type definitions.
pub fn hainan_ftp_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-hainan-ftp:cross-border-trade".to_string(),
            name: "Cross-Border Trade License".to_string(),
            description: "License for cross-border trade operations within the Hainan FTP"
                .to_string(),
            regulator_id: "cn-hainan-ftp".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "cross_border_goods_trade".to_string(),
                "cross_border_services_trade".to_string(),
                "bonded_logistics".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-hainan-ftp:negative-list-sector".to_string(),
            name: "Negative List Sector Approval".to_string(),
            description:
                "Special approval for foreign investment in negative list sectors within Hainan FTP"
                    .to_string(),
            regulator_id: "cn-hainan-ftp".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "restricted_sector_investment".to_string(),
                "joint_venture_operation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-hainan-ftp:qflp".to_string(),
            name: "Qualified Foreign Limited Partner (QFLP) License".to_string(),
            description: "License for qualified foreign LPs to invest in domestic PE/VC funds via Hainan"
                .to_string(),
            regulator_id: "cn-hainan-ftp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "pe_fund_investment".to_string(),
                "vc_fund_investment".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-hainan-ftp:qdlp".to_string(),
            name: "Qualified Domestic Limited Partner (QDLP) License".to_string(),
            description: "License for domestic LPs to invest in overseas funds via Hainan FTP"
                .to_string(),
            regulator_id: "cn-hainan-ftp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "overseas_fund_investment".to_string(),
                "cross_border_asset_allocation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── Hangzhou Commerce Bureau ──────────────────────────────────────────────

/// Hangzhou Commerce Bureau regulator profile.
pub fn hangzhou_cb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-hangzhou-cb".to_string(),
        name: "Hangzhou Commerce Bureau".to_string(),
        jurisdiction_id: "cn-hangzhou".to_string(),
        registry_url: Some("https://www.hangzhou.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["ecommerce_registry".to_string()],
    }
}

/// Hangzhou Commerce Bureau license type definitions.
pub fn hangzhou_cb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-hangzhou-cb:cross-border-ecommerce".to_string(),
            name: "Cross-Border E-Commerce Pilot License".to_string(),
            description:
                "License for cross-border e-commerce operations under the Hangzhou CBEC pilot zone"
                    .to_string(),
            regulator_id: "cn-hangzhou-cb".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "cross_border_ecommerce".to_string(),
                "bonded_import".to_string(),
                "direct_mail_import".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-hangzhou-cb:digital-trade".to_string(),
            name: "Digital Trade License".to_string(),
            description: "License for digital trade services in the Hangzhou digital economy zone"
                .to_string(),
            regulator_id: "cn-hangzhou-cb".to_string(),
            category: Some("trade".to_string()),
            permitted_activities: vec![
                "digital_services_trade".to_string(),
                "data_cross_border_transfer".to_string(),
                "digital_content_trade".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── Shenzhen Financial Regulatory Bureau ──────────────────────────────────

/// Shenzhen Financial Regulatory Bureau regulator profile.
pub fn shenzhen_frb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-shenzhen-frb".to_string(),
        name: "Shenzhen Financial Regulatory Bureau".to_string(),
        jurisdiction_id: "cn-shenzhen".to_string(),
        registry_url: Some("https://jr.sz.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["fintech_sandbox_registry".to_string()],
    }
}

/// Shenzhen Financial Regulatory Bureau license type definitions.
pub fn shenzhen_frb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-shenzhen-frb:fintech-sandbox".to_string(),
            name: "Fintech Regulatory Sandbox License".to_string(),
            description:
                "License to operate within the Shenzhen fintech regulatory sandbox program"
                    .to_string(),
            regulator_id: "cn-shenzhen-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fintech_innovation_testing".to_string(),
                "sandbox_product_launch".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(2),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shenzhen-frb:digital-rmb-pilot".to_string(),
            name: "Digital RMB Pilot Operator License".to_string(),
            description:
                "License to participate as an operator in the digital RMB (e-CNY) pilot program"
                    .to_string(),
            regulator_id: "cn-shenzhen-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_rmb_distribution".to_string(),
                "digital_rmb_wallet_services".to_string(),
                "digital_rmb_merchant_acceptance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shenzhen-frb:qianhai-cooperation".to_string(),
            name: "Qianhai Cooperation Zone Financial License".to_string(),
            description:
                "License for financial services within the Qianhai Shenzhen-Hong Kong cooperation zone"
                    .to_string(),
            regulator_id: "cn-shenzhen-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "cross_border_lending".to_string(),
                "cross_border_asset_management".to_string(),
                "rmb_internationalization_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shenzhen-frb:digital-asset-pilot".to_string(),
            name: "Digital Asset Pilot License".to_string(),
            description:
                "Pilot license for digital asset services under Shenzhen EZ innovation framework"
                    .to_string(),
            regulator_id: "cn-shenzhen-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_custody".to_string(),
                "digital_asset_trading_pilot".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(2),
        },
    ]
}

// ── Shanghai FTZ Administration ───────────────────────────────────────────

/// Shanghai FTZ Administration regulator profile.
pub fn shanghai_ftz_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-shanghai-ftz".to_string(),
        name: "Shanghai Free Trade Zone Administration".to_string(),
        jurisdiction_id: "cn-shanghai".to_string(),
        registry_url: Some("https://www.china-shftz.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["ftz_business_registry".to_string()],
    }
}

/// Shanghai FTZ Administration license type definitions.
pub fn shanghai_ftz_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-shanghai-ftz:business-registration".to_string(),
            name: "FTZ Business Registration".to_string(),
            description: "Business registration within the Shanghai Free Trade Zone".to_string(),
            regulator_id: "cn-shanghai-ftz".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "ftz_business_operations".to_string(),
                "international_trade".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shanghai-ftz:wfoe".to_string(),
            name: "Wholly Foreign-Owned Enterprise (WFOE) Registration".to_string(),
            description:
                "Registration of a WFOE within the Shanghai FTZ under simplified procedures"
                    .to_string(),
            regulator_id: "cn-shanghai-ftz".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "foreign_invested_operations".to_string(),
                "profit_repatriation".to_string(),
                "cross_border_settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shanghai-ftz:financial-innovation".to_string(),
            name: "Financial Innovation Pilot License".to_string(),
            description:
                "Pilot license for innovative financial services within the Shanghai FTZ"
                    .to_string(),
            regulator_id: "cn-shanghai-ftz".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "innovative_financial_products".to_string(),
                "cross_border_financial_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── Shanghai Financial Regulatory Bureau ──────────────────────────────────

/// Shanghai Financial Regulatory Bureau regulator profile.
pub fn shanghai_frb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-shanghai-frb".to_string(),
        name: "Shanghai Financial Regulatory Bureau".to_string(),
        jurisdiction_id: "cn-shanghai".to_string(),
        registry_url: Some("https://jrj.sh.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["financial_institution_registry".to_string()],
    }
}

/// Shanghai Financial Regulatory Bureau license type definitions.
pub fn shanghai_frb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-shanghai-frb:local-financial-org".to_string(),
            name: "Local Financial Organization License".to_string(),
            description:
                "License for local financial organizations including small loan and guarantee companies"
                    .to_string(),
            regulator_id: "cn-shanghai-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "small_loan_services".to_string(),
                "financing_guarantee".to_string(),
                "commercial_factoring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-shanghai-frb:fintech-pilot".to_string(),
            name: "Shanghai Fintech Pilot License".to_string(),
            description:
                "License for fintech pilot programs under Shanghai financial regulatory oversight"
                    .to_string(),
            regulator_id: "cn-shanghai-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fintech_innovation_testing".to_string(),
                "regtech_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(2),
        },
    ]
}

// ── Beijing Financial Regulatory Bureau ───────────────────────────────────

/// Beijing Financial Regulatory Bureau regulator profile.
pub fn beijing_frb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "cn-beijing-frb".to_string(),
        name: "Beijing Financial Regulatory Bureau".to_string(),
        jurisdiction_id: "cn-beijing".to_string(),
        registry_url: Some("https://jrj.beijing.gov.cn".to_string()),
        did: None,
        api_capabilities: vec!["fintech_sandbox_registry".to_string()],
    }
}

/// Beijing Financial Regulatory Bureau license type definitions.
pub fn beijing_frb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "cn-beijing-frb:fintech-sandbox".to_string(),
            name: "Beijing Fintech Regulatory Sandbox License".to_string(),
            description:
                "License for fintech innovation under the Beijing regulatory sandbox program"
                    .to_string(),
            regulator_id: "cn-beijing-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fintech_innovation_testing".to_string(),
                "sandbox_product_launch".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(2),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-beijing-frb:local-financial-org".to_string(),
            name: "Beijing Local Financial Organization License".to_string(),
            description:
                "License for local financial organizations operating in Beijing municipality"
                    .to_string(),
            regulator_id: "cn-beijing-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "small_loan_services".to_string(),
                "financing_guarantee".to_string(),
                "pawn_brokerage".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "cn-beijing-frb:digital-asset-pilot".to_string(),
            name: "Beijing Digital Asset Pilot License".to_string(),
            description:
                "Pilot license for digital asset services under Beijing innovation framework"
                    .to_string(),
            regulator_id: "cn-beijing-frb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_custody_pilot".to_string(),
                "nft_marketplace_pilot".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(2),
        },
    ]
}

// ── China Registry Aggregation ────────────────────────────────────────────

/// All China regulatory authorities across national and sub-national levels.
pub fn china_regulators() -> Vec<LicensepackRegulator> {
    vec![
        // National
        pboc_regulator(),
        nfra_regulator(),
        csrc_regulator(),
        safe_regulator(),
        // Hainan
        hainan_ftp_regulator(),
        // Hangzhou
        hangzhou_cb_regulator(),
        // Shenzhen
        shenzhen_frb_regulator(),
        // Shanghai
        shanghai_ftz_regulator(),
        shanghai_frb_regulator(),
        // Beijing
        beijing_frb_regulator(),
    ]
}

/// All China license type definitions across all authorities and zones.
pub fn china_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    // National
    all.extend(pboc_license_types());
    all.extend(nfra_license_types());
    all.extend(csrc_license_types());
    all.extend(safe_license_types());
    // Hainan
    all.extend(hainan_ftp_license_types());
    // Hangzhou
    all.extend(hangzhou_cb_license_types());
    // Shenzhen
    all.extend(shenzhen_frb_license_types());
    // Shanghai
    all.extend(shanghai_ftz_license_types());
    all.extend(shanghai_frb_license_types());
    // Beijing
    all.extend(beijing_frb_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn china_has_ten_regulators() {
        let regs = china_regulators();
        assert_eq!(regs.len(), 10);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"cn-pboc"), "missing PBOC");
        assert!(ids.contains(&"cn-nfra"), "missing NFRA");
        assert!(ids.contains(&"cn-csrc"), "missing CSRC");
        assert!(ids.contains(&"cn-safe"), "missing SAFE");
        assert!(ids.contains(&"cn-hainan-ftp"), "missing Hainan FTP");
        assert!(ids.contains(&"cn-hangzhou-cb"), "missing Hangzhou CB");
        assert!(ids.contains(&"cn-shenzhen-frb"), "missing Shenzhen FRB");
        assert!(ids.contains(&"cn-shanghai-ftz"), "missing Shanghai FTZ");
        assert!(ids.contains(&"cn-shanghai-frb"), "missing Shanghai FRB");
        assert!(ids.contains(&"cn-beijing-frb"), "missing Beijing FRB");
    }

    #[test]
    fn national_regulators_have_cn_jurisdiction() {
        let national_ids = ["cn-pboc", "cn-nfra", "cn-csrc", "cn-safe"];
        for reg in china_regulators() {
            if national_ids.contains(&reg.regulator_id.as_str()) {
                assert_eq!(
                    reg.jurisdiction_id, "cn",
                    "{} should have jurisdiction cn",
                    reg.regulator_id
                );
            }
        }
    }

    #[test]
    fn subnational_regulators_have_correct_jurisdictions() {
        let expected: BTreeMap<&str, &str> = [
            ("cn-hainan-ftp", "cn-hainan"),
            ("cn-hangzhou-cb", "cn-hangzhou"),
            ("cn-shenzhen-frb", "cn-shenzhen"),
            ("cn-shanghai-ftz", "cn-shanghai"),
            ("cn-shanghai-frb", "cn-shanghai"),
            ("cn-beijing-frb", "cn-beijing"),
        ]
        .into_iter()
        .collect();

        for reg in china_regulators() {
            if let Some(expected_jid) = expected.get(reg.regulator_id.as_str()) {
                assert_eq!(
                    reg.jurisdiction_id, *expected_jid,
                    "{} should have jurisdiction {}",
                    reg.regulator_id, expected_jid
                );
            }
        }
    }

    #[test]
    fn china_license_types_cover_all_authorities() {
        let types = china_license_types();
        assert!(
            types.len() >= 28,
            "expected >= 28 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("cn-pboc"), "no PBOC license types");
        assert!(authority_ids.contains("cn-nfra"), "no NFRA license types");
        assert!(authority_ids.contains("cn-csrc"), "no CSRC license types");
        assert!(authority_ids.contains("cn-safe"), "no SAFE license types");
        assert!(authority_ids.contains("cn-hainan-ftp"), "no Hainan FTP license types");
        assert!(authority_ids.contains("cn-hangzhou-cb"), "no Hangzhou CB license types");
        assert!(authority_ids.contains("cn-shenzhen-frb"), "no Shenzhen FRB license types");
        assert!(authority_ids.contains("cn-shanghai-ftz"), "no Shanghai FTZ license types");
        assert!(authority_ids.contains("cn-shanghai-frb"), "no Shanghai FRB license types");
        assert!(authority_ids.contains("cn-beijing-frb"), "no Beijing FRB license types");
    }

    #[test]
    fn pboc_has_payment_and_credit_licenses() {
        let types = pboc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-pboc:payment-institution"));
        assert!(ids.contains(&"cn-pboc:cross-border-payment"));
        assert!(ids.contains(&"cn-pboc:credit-reporting"));
    }

    #[test]
    fn nfra_has_banking_and_insurance_licenses() {
        let types = nfra_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-nfra:commercial-bank"));
        assert!(ids.contains(&"cn-nfra:insurance-company"));
        assert!(ids.contains(&"cn-nfra:trust-company"));
        assert!(ids.contains(&"cn-nfra:consumer-finance"));
    }

    #[test]
    fn csrc_has_securities_and_fund_licenses() {
        let types = csrc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-csrc:securities-company"));
        assert!(ids.contains(&"cn-csrc:fund-management"));
        assert!(ids.contains(&"cn-csrc:futures-company"));
        assert!(ids.contains(&"cn-csrc:investment-advisory"));
    }

    #[test]
    fn safe_has_forex_licenses() {
        let types = safe_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-safe:forex-trading"));
        assert!(ids.contains(&"cn-safe:cross-border-investment"));
    }

    #[test]
    fn hainan_has_ftp_and_qflp_qdlp_licenses() {
        let types = hainan_ftp_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-hainan-ftp:cross-border-trade"));
        assert!(ids.contains(&"cn-hainan-ftp:negative-list-sector"));
        assert!(ids.contains(&"cn-hainan-ftp:qflp"));
        assert!(ids.contains(&"cn-hainan-ftp:qdlp"));
    }

    #[test]
    fn hangzhou_has_ecommerce_and_digital_trade() {
        let types = hangzhou_cb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-hangzhou-cb:cross-border-ecommerce"));
        assert!(ids.contains(&"cn-hangzhou-cb:digital-trade"));
    }

    #[test]
    fn shenzhen_has_fintech_and_digital_rmb_licenses() {
        let types = shenzhen_frb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-shenzhen-frb:fintech-sandbox"));
        assert!(ids.contains(&"cn-shenzhen-frb:digital-rmb-pilot"));
        assert!(ids.contains(&"cn-shenzhen-frb:qianhai-cooperation"));
        assert!(ids.contains(&"cn-shenzhen-frb:digital-asset-pilot"));
    }

    #[test]
    fn shanghai_ftz_has_wfoe_and_financial_innovation() {
        let types = shanghai_ftz_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-shanghai-ftz:business-registration"));
        assert!(ids.contains(&"cn-shanghai-ftz:wfoe"));
        assert!(ids.contains(&"cn-shanghai-ftz:financial-innovation"));
    }

    #[test]
    fn shanghai_frb_has_local_finance_and_fintech() {
        let types = shanghai_frb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-shanghai-frb:local-financial-org"));
        assert!(ids.contains(&"cn-shanghai-frb:fintech-pilot"));
    }

    #[test]
    fn beijing_has_sandbox_and_digital_asset_licenses() {
        let types = beijing_frb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"cn-beijing-frb:fintech-sandbox"));
        assert!(ids.contains(&"cn-beijing-frb:local-financial-org"));
        assert!(ids.contains(&"cn-beijing-frb:digital-asset-pilot"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = china_license_types();
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
        for lt in china_license_types() {
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
        for lt in china_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in china_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in china_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
