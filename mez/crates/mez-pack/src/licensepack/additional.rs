//! # Additional Jurisdiction Regulatory Authority License Mappings
//!
//! Covers remaining jurisdictions referenced on momentum.inc in a single
//! consolidated module:
//!
//! | Jurisdiction | ID | Regulators |
//! |---|---|---|
//! | Portugal | `pt` | BdP, CMVM |
//! | Indonesia | `id` | OJK, BI |
//! | South Africa | `za` | SARB, FSCA |
//! | Egypt | `eg` | CBE, FRA |
//! | Tanzania | `tz` | BOT |
//! | Zanzibar | `tz-zanzibar` | Zanzibar Business Registration |
//! | Ireland | `ie` | CBI, CRO |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the jurisdiction-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ═══════════════════════════════════════════════════════════════════════════════
// PORTUGAL (pt)
// ═══════════════════════════════════════════════════════════════════════════════

// ── BdP — Banco de Portugal ──────────────────────────────────────────────────

/// BdP regulator profile.
pub fn bdp_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pt-bdp".to_string(),
        name: "Banco de Portugal".to_string(),
        jurisdiction_id: "pt".to_string(),
        registry_url: Some("https://www.bportugal.pt".to_string()),
        did: None,
        api_capabilities: vec!["institution_registry".to_string()],
    }
}

/// BdP license type definitions.
pub fn bdp_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pt-bdp:credit-institution".to_string(),
            name: "Credit Institution Authorization".to_string(),
            description: "Authorization to operate as a credit institution in Portugal"
                .to_string(),
            regulator_id: "pt-bdp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-bdp:payment-institution".to_string(),
            name: "Payment Institution Authorization".to_string(),
            description: "Authorization to operate as a payment institution under PSD2 in Portugal"
                .to_string(),
            regulator_id: "pt-bdp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "payment_initiation".to_string(),
                "account_information".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-bdp:e-money-institution".to_string(),
            name: "Electronic Money Institution Authorization".to_string(),
            description: "Authorization to issue electronic money in Portugal".to_string(),
            regulator_id: "pt-bdp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-bdp:vasp".to_string(),
            name: "VASP Registration".to_string(),
            description:
                "Registration as a virtual asset service provider with Banco de Portugal"
                    .to_string(),
            regulator_id: "pt-bdp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_exchange".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "pt-bdp:forex-dealer".to_string(),
            name: "Foreign Exchange Dealer Authorization".to_string(),
            description: "Authorization to provide foreign exchange services in Portugal"
                .to_string(),
            regulator_id: "pt-bdp".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "foreign_exchange".to_string(),
                "currency_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── CMVM — Comissao do Mercado de Valores Mobiliarios ────────────────────────

/// CMVM regulator profile.
pub fn cmvm_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "pt-cmvm".to_string(),
        name: "Comissao do Mercado de Valores Mobiliarios".to_string(),
        jurisdiction_id: "pt".to_string(),
        registry_url: Some("https://www.cmvm.pt".to_string()),
        did: None,
        api_capabilities: vec!["entity_registry".to_string()],
    }
}

/// CMVM license type definitions.
pub fn cmvm_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "pt-cmvm:investment-firm".to_string(),
            name: "Investment Firm Authorization".to_string(),
            description: "Authorization to operate as an investment firm in Portugal under MiFID II"
                .to_string(),
            regulator_id: "pt-cmvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "portfolio_management".to_string(),
                "investment_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-cmvm:fund-manager".to_string(),
            name: "Fund Management Company Authorization".to_string(),
            description: "Authorization to manage investment funds in Portugal".to_string(),
            regulator_id: "pt-cmvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "ucits_management".to_string(),
                "aif_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-cmvm:venture-capital".to_string(),
            name: "Venture Capital Company Registration".to_string(),
            description: "Registration to operate as a venture capital company in Portugal"
                .to_string(),
            regulator_id: "pt-cmvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "venture_capital".to_string(),
                "private_equity".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "pt-cmvm:financial-intermediary".to_string(),
            name: "Financial Intermediary Registration".to_string(),
            description: "Registration as a tied agent or financial intermediary in Portugal"
                .to_string(),
            regulator_id: "pt-cmvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "order_reception".to_string(),
                "investment_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "pt-cmvm:crowdfunding-platform".to_string(),
            name: "Crowdfunding Platform Authorization".to_string(),
            description: "Authorization to operate a crowdfunding platform under ECSPR in Portugal"
                .to_string(),
            regulator_id: "pt-cmvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "crowdfunding_platform".to_string(),
                "equity_crowdfunding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// INDONESIA (id)
// ═══════════════════════════════════════════════════════════════════════════════

// ── OJK — Otoritas Jasa Keuangan ─────────────────────────────────────────────

/// OJK regulator profile.
pub fn ojk_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "id-ojk".to_string(),
        name: "Otoritas Jasa Keuangan".to_string(),
        jurisdiction_id: "id".to_string(),
        registry_url: Some("https://www.ojk.go.id".to_string()),
        did: None,
        api_capabilities: vec!["institution_registry".to_string(), "license_status".to_string()],
    }
}

/// OJK license type definitions.
pub fn ojk_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "id-ojk:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "License to operate as a commercial bank under OJK regulation"
                .to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "id-ojk:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to operate as an insurance company in Indonesia".to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "id-ojk:securities-company".to_string(),
            name: "Securities Company License".to_string(),
            description: "License to operate as a securities company in Indonesia".to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "underwriting".to_string(),
                "investment_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "id-ojk:fintech-lending".to_string(),
            name: "Fintech Lending License".to_string(),
            description:
                "License to operate a peer-to-peer lending platform under OJK regulation"
                    .to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "peer_to_peer_lending".to_string(),
                "digital_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "id-ojk:multifinance".to_string(),
            name: "Multifinance Company License".to_string(),
            description: "License to operate as a multifinance company in Indonesia".to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "consumer_finance".to_string(),
                "leasing".to_string(),
                "factoring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "id-ojk:digital-bank".to_string(),
            name: "Digital Banking License".to_string(),
            description: "License to operate as a digital bank under OJK regulation".to_string(),
            regulator_id: "id-ojk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_banking".to_string(),
                "deposit_taking".to_string(),
                "digital_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── BI — Bank Indonesia ──────────────────────────────────────────────────────

/// BI regulator profile.
pub fn bi_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "id-bi".to_string(),
        name: "Bank Indonesia".to_string(),
        jurisdiction_id: "id".to_string(),
        registry_url: Some("https://www.bi.go.id".to_string()),
        did: None,
        api_capabilities: vec!["payment_system_registry".to_string()],
    }
}

/// BI license type definitions.
pub fn bi_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "id-bi:payment-system-provider".to_string(),
            name: "Payment System Provider License".to_string(),
            description: "License to operate a payment system under Bank Indonesia regulation"
                .to_string(),
            regulator_id: "id-bi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "clearing".to_string(),
                "settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "id-bi:e-money-issuer".to_string(),
            name: "Electronic Money Issuer License".to_string(),
            description: "License to issue electronic money under Bank Indonesia regulation"
                .to_string(),
            regulator_id: "id-bi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "e_money_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "id-bi:payment-gateway".to_string(),
            name: "Payment Gateway License".to_string(),
            description: "License to operate a payment gateway in Indonesia".to_string(),
            regulator_id: "id-bi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_gateway".to_string(),
                "transaction_routing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "id-bi:money-transfer".to_string(),
            name: "Money Transfer Operator License".to_string(),
            description: "License to operate money transfer services in Indonesia".to_string(),
            regulator_id: "id-bi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_transfer".to_string(),
                "remittance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// SOUTH AFRICA (za)
// ═══════════════════════════════════════════════════════════════════════════════

// ── SARB — South African Reserve Bank ────────────────────────────────────────

/// SARB regulator profile.
pub fn sarb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "za-sarb".to_string(),
        name: "South African Reserve Bank".to_string(),
        jurisdiction_id: "za".to_string(),
        registry_url: Some("https://www.resbank.co.za".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string()],
    }
}

/// SARB license type definitions.
pub fn sarb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "za-sarb:banking".to_string(),
            name: "Banking License".to_string(),
            description: "License to conduct banking business under the Banks Act 94 of 1990"
                .to_string(),
            regulator_id: "za-sarb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-sarb:mutual-bank".to_string(),
            name: "Mutual Bank License".to_string(),
            description:
                "License to operate as a mutual bank under the Mutual Banks Act 124 of 1993"
                    .to_string(),
            regulator_id: "za-sarb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "savings_products".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-sarb:payment-system".to_string(),
            name: "Payment System Operator Designation".to_string(),
            description:
                "Designation to operate a payment system under the National Payment System Act"
                    .to_string(),
            regulator_id: "za-sarb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_system_operation".to_string(),
                "clearing".to_string(),
                "settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-sarb:adla".to_string(),
            name: "Authorized Dealer in Foreign Exchange (Limited Authority)".to_string(),
            description:
                "Authorization as an authorized dealer with limited authority for forex transactions"
                    .to_string(),
            regulator_id: "za-sarb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "foreign_exchange".to_string(),
                "remittance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "za-sarb:casp".to_string(),
            name: "Crypto Asset Service Provider Registration".to_string(),
            description:
                "Registration as a crypto asset service provider with the SARB/FSCA"
                    .to_string(),
            regulator_id: "za-sarb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "crypto_asset_exchange".to_string(),
                "crypto_asset_custody".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── FSCA — Financial Sector Conduct Authority ────────────────────────────────

/// FSCA regulator profile.
pub fn fsca_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "za-fsca".to_string(),
        name: "Financial Sector Conduct Authority".to_string(),
        jurisdiction_id: "za".to_string(),
        registry_url: Some("https://www.fsca.co.za".to_string()),
        did: None,
        api_capabilities: vec!["fsp_registry".to_string()],
    }
}

/// FSCA license type definitions.
pub fn fsca_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "za-fsca:fsp".to_string(),
            name: "Financial Services Provider License".to_string(),
            description:
                "License to provide financial services under the FAIS Act 37 of 2002"
                    .to_string(),
            regulator_id: "za-fsca".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "financial_advisory".to_string(),
                "intermediary_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-fsca:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to conduct insurance business under the Insurance Act 18 of 2017"
                .to_string(),
            regulator_id: "za-fsca".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "za-fsca:collective-investment-scheme".to_string(),
            name: "Collective Investment Scheme Manager License".to_string(),
            description:
                "License to manage a collective investment scheme under the CIS Control Act"
                    .to_string(),
            regulator_id: "za-fsca".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "portfolio_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-fsca:market-infrastructure".to_string(),
            name: "Market Infrastructure License".to_string(),
            description:
                "License to operate market infrastructure under the Financial Markets Act"
                    .to_string(),
            regulator_id: "za-fsca".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "exchange_operations".to_string(),
                "clearing_house".to_string(),
                "central_securities_depository".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "za-fsca:retirement-fund".to_string(),
            name: "Retirement Fund Registration".to_string(),
            description: "Registration of a retirement fund under the Pension Funds Act"
                .to_string(),
            regulator_id: "za-fsca".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "pension_fund_management".to_string(),
                "retirement_products".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// EGYPT (eg)
// ═══════════════════════════════════════════════════════════════════════════════

// ── CBE — Central Bank of Egypt ──────────────────────────────────────────────

/// CBE regulator profile.
pub fn cbe_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "eg-cbe".to_string(),
        name: "Central Bank of Egypt".to_string(),
        jurisdiction_id: "eg".to_string(),
        registry_url: Some("https://www.cbe.org.eg".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string()],
    }
}

/// CBE license type definitions.
pub fn cbe_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "eg-cbe:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description:
                "License to operate as a commercial bank under the Central Bank and Banking Law"
                    .to_string(),
            regulator_id: "eg-cbe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "eg-cbe:islamic-bank".to_string(),
            name: "Islamic Banking License".to_string(),
            description:
                "License to operate as an Islamic bank in Egypt under Sharia-compliant principles"
                    .to_string(),
            regulator_id: "eg-cbe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "islamic_deposit_taking".to_string(),
                "islamic_financing".to_string(),
                "murabaha".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "eg-cbe:payment-service-provider".to_string(),
            name: "Payment Service Provider License".to_string(),
            description: "License to provide payment services in Egypt".to_string(),
            regulator_id: "eg-cbe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "mobile_payments".to_string(),
                "e_money".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-cbe:fintech".to_string(),
            name: "Fintech License".to_string(),
            description:
                "License to operate a fintech company under CBE fintech regulations"
                    .to_string(),
            regulator_id: "eg-cbe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_financial_services".to_string(),
                "open_banking".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-cbe:money-exchange".to_string(),
            name: "Money Exchange License".to_string(),
            description: "License to operate a money exchange in Egypt".to_string(),
            regulator_id: "eg-cbe".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "foreign_exchange".to_string(),
                "currency_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
    ]
}

// ── FRA — Financial Regulatory Authority ─────────────────────────────────────

/// FRA regulator profile.
pub fn fra_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "eg-fra".to_string(),
        name: "Financial Regulatory Authority".to_string(),
        jurisdiction_id: "eg".to_string(),
        registry_url: Some("https://www.fra.gov.eg".to_string()),
        did: None,
        api_capabilities: vec!["entity_registry".to_string()],
    }
}

/// FRA license type definitions.
pub fn fra_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "eg-fra:securities-brokerage".to_string(),
            name: "Securities Brokerage License".to_string(),
            description: "License to operate as a securities broker in Egypt".to_string(),
            regulator_id: "eg-fra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "order_execution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-fra:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to conduct non-bank insurance business in Egypt".to_string(),
            regulator_id: "eg-fra".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-fra:microfinance".to_string(),
            name: "Microfinance License".to_string(),
            description: "License to operate as a microfinance institution in Egypt".to_string(),
            regulator_id: "eg-fra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "micro_lending".to_string(),
                "micro_savings".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-fra:leasing".to_string(),
            name: "Financial Leasing License".to_string(),
            description: "License to conduct financial leasing operations in Egypt".to_string(),
            regulator_id: "eg-fra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "financial_leasing".to_string(),
                "equipment_leasing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "eg-fra:factoring".to_string(),
            name: "Factoring License".to_string(),
            description: "License to conduct factoring operations in Egypt".to_string(),
            regulator_id: "eg-fra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "factoring".to_string(),
                "receivables_financing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// TANZANIA / ZANZIBAR (tz, tz-zanzibar)
// ═══════════════════════════════════════════════════════════════════════════════

// ── BOT — Bank of Tanzania ───────────────────────────────────────────────────

/// BOT regulator profile.
pub fn bot_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "tz-bot".to_string(),
        name: "Bank of Tanzania".to_string(),
        jurisdiction_id: "tz".to_string(),
        registry_url: Some("https://www.bot.go.tz".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string()],
    }
}

/// BOT license type definitions.
pub fn bot_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "tz-bot:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "License to operate as a commercial bank in Tanzania".to_string(),
            regulator_id: "tz-bot".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "tz-bot:microfinance".to_string(),
            name: "Microfinance Bank License".to_string(),
            description: "License to operate as a microfinance bank in Tanzania".to_string(),
            regulator_id: "tz-bot".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "micro_lending".to_string(),
                "micro_deposit_taking".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "tz-bot:mobile-money".to_string(),
            name: "Mobile Money Operator License".to_string(),
            description: "License to operate mobile money services in Tanzania".to_string(),
            regulator_id: "tz-bot".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "mobile_payments".to_string(),
                "e_money".to_string(),
                "remittance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "tz-bot:forex-bureau".to_string(),
            name: "Foreign Exchange Bureau License".to_string(),
            description: "License to operate a foreign exchange bureau in Tanzania".to_string(),
            regulator_id: "tz-bot".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "foreign_exchange".to_string(),
                "currency_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "tz-bot:payment-system".to_string(),
            name: "Payment System Operator License".to_string(),
            description: "License to operate a payment system in Tanzania".to_string(),
            regulator_id: "tz-bot".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_system_operation".to_string(),
                "clearing".to_string(),
                "settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
    ]
}

// ── Zanzibar Business Registration ───────────────────────────────────────────

/// Zanzibar Business Registration regulator profile.
pub fn zanzibar_br_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "tz-zanzibar-br".to_string(),
        name: "Zanzibar Business Registration".to_string(),
        jurisdiction_id: "tz-zanzibar".to_string(),
        registry_url: Some("https://www.zanzibar.go.tz".to_string()),
        did: None,
        api_capabilities: vec!["business_registry".to_string()],
    }
}

/// Zanzibar Business Registration license type definitions.
pub fn zanzibar_br_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "tz-zanzibar-br:business-license".to_string(),
            name: "Zanzibar Business License".to_string(),
            description: "General business license for operations in Zanzibar".to_string(),
            regulator_id: "tz-zanzibar-br".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "trade".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "tz-zanzibar-br:company-registration".to_string(),
            name: "Zanzibar Company Registration".to_string(),
            description: "Registration of a company in Zanzibar".to_string(),
            regulator_id: "tz-zanzibar-br".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "corporate_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "tz-zanzibar-br:foreign-investment".to_string(),
            name: "Zanzibar Foreign Investment Certificate".to_string(),
            description: "Certificate for foreign investment operations in Zanzibar".to_string(),
            regulator_id: "tz-zanzibar-br".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "foreign_investment".to_string(),
                "joint_ventures".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "tz-zanzibar-br:free-zone-enterprise".to_string(),
            name: "Zanzibar Free Economic Zone Enterprise License".to_string(),
            description:
                "License to operate within the Zanzibar Free Economic Zone".to_string(),
            regulator_id: "tz-zanzibar-br".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "free_zone_operations".to_string(),
                "export_processing".to_string(),
                "manufacturing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(10),
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// IRELAND (ie)
// ═══════════════════════════════════════════════════════════════════════════════

// ── CBI — Central Bank of Ireland ────────────────────────────────────────────

/// CBI regulator profile.
pub fn cbi_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ie-cbi".to_string(),
        name: "Central Bank of Ireland".to_string(),
        jurisdiction_id: "ie".to_string(),
        registry_url: Some("https://www.centralbank.ie".to_string()),
        did: None,
        api_capabilities: vec![
            "firm_directory".to_string(),
            "register_search".to_string(),
        ],
    }
}

/// CBI license type definitions.
pub fn cbi_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:credit-institution".to_string(),
            name: "Credit Institution Authorization".to_string(),
            description:
                "Authorization to operate as a credit institution (bank) in Ireland under SSM"
                    .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:insurance-undertaking".to_string(),
            name: "Insurance Undertaking Authorization".to_string(),
            description:
                "Authorization to operate as an insurance undertaking under Solvency II"
                    .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
                "reinsurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:investment-firm".to_string(),
            name: "Investment Firm Authorization".to_string(),
            description: "Authorization to operate as an investment firm under MiFID II"
                .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "portfolio_management".to_string(),
                "investment_advisory".to_string(),
                "order_execution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:fund-manager".to_string(),
            name: "Fund Management Company Authorization".to_string(),
            description:
                "Authorization to manage UCITS or AIFs in Ireland".to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "ucits_management".to_string(),
                "aif_management".to_string(),
                "fund_administration".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:payment-institution".to_string(),
            name: "Payment Institution Authorization".to_string(),
            description: "Authorization to operate as a payment institution under PSD2"
                .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "payment_initiation".to_string(),
                "account_information".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:e-money-institution".to_string(),
            name: "Electronic Money Institution Authorization".to_string(),
            description:
                "Authorization to issue electronic money under the E-Money Regulations"
                    .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:vasp".to_string(),
            name: "VASP Registration".to_string(),
            description:
                "Registration as a virtual asset service provider with the Central Bank of Ireland"
                    .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_exchange".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cbi:insurance-intermediary".to_string(),
            name: "Insurance Intermediary Registration".to_string(),
            description: "Registration as an insurance intermediary (broker/agent) in Ireland"
                .to_string(),
            regulator_id: "ie-cbi".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "insurance_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
    ]
}

// ── CRO — Companies Registration Office ──────────────────────────────────────

/// CRO regulator profile.
pub fn cro_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ie-cro".to_string(),
        name: "Companies Registration Office".to_string(),
        jurisdiction_id: "ie".to_string(),
        registry_url: Some("https://www.cro.ie".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string()],
    }
}

/// CRO license type definitions.
pub fn cro_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ie-cro:private-company-limited".to_string(),
            name: "Private Company Limited by Shares (LTD)".to_string(),
            description: "Registration of a private company limited by shares in Ireland"
                .to_string(),
            regulator_id: "ie-cro".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "professional_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cro:designated-activity-company".to_string(),
            name: "Designated Activity Company (DAC)".to_string(),
            description: "Registration of a designated activity company in Ireland".to_string(),
            regulator_id: "ie-cro".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "designated_activities".to_string(),
                "special_purpose_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cro:public-limited-company".to_string(),
            name: "Public Limited Company (PLC)".to_string(),
            description: "Registration of a public limited company in Ireland".to_string(),
            regulator_id: "ie-cro".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "securities_issuance".to_string(),
                "capital_raising".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cro:branch-registration".to_string(),
            name: "Foreign Company Branch Registration".to_string(),
            description:
                "Registration of an external (foreign) company branch in Ireland".to_string(),
            regulator_id: "ie-cro".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "branch_operations".to_string(),
                "representative_office".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ie-cro:icav".to_string(),
            name: "Irish Collective Asset-management Vehicle (ICAV)".to_string(),
            description: "Registration of an ICAV for fund domiciliation in Ireland".to_string(),
            regulator_id: "ie-cro".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "fund_domiciliation".to_string(),
                "collective_investment".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ═══════════════════════════════════════════════════════════════════════════════
// AGGREGATION
// ═══════════════════════════════════════════════════════════════════════════════

/// All additional jurisdiction regulatory authorities.
pub fn additional_regulators() -> Vec<LicensepackRegulator> {
    vec![
        // Portugal
        bdp_regulator(),
        cmvm_regulator(),
        // Indonesia
        ojk_regulator(),
        bi_regulator(),
        // South Africa
        sarb_regulator(),
        fsca_regulator(),
        // Egypt
        cbe_regulator(),
        fra_regulator(),
        // Tanzania / Zanzibar
        bot_regulator(),
        zanzibar_br_regulator(),
        // Ireland
        cbi_regulator(),
        cro_regulator(),
    ]
}

/// All additional jurisdiction license type definitions across all authorities.
pub fn additional_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    // Portugal
    all.extend(bdp_license_types());
    all.extend(cmvm_license_types());
    // Indonesia
    all.extend(ojk_license_types());
    all.extend(bi_license_types());
    // South Africa
    all.extend(sarb_license_types());
    all.extend(fsca_license_types());
    // Egypt
    all.extend(cbe_license_types());
    all.extend(fra_license_types());
    // Tanzania / Zanzibar
    all.extend(bot_license_types());
    all.extend(zanzibar_br_license_types());
    // Ireland
    all.extend(cbi_license_types());
    all.extend(cro_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn additional_has_twelve_regulators() {
        let regs = additional_regulators();
        assert_eq!(regs.len(), 12);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        // Portugal
        assert!(ids.contains(&"pt-bdp"), "missing BdP");
        assert!(ids.contains(&"pt-cmvm"), "missing CMVM");
        // Indonesia
        assert!(ids.contains(&"id-ojk"), "missing OJK");
        assert!(ids.contains(&"id-bi"), "missing BI");
        // South Africa
        assert!(ids.contains(&"za-sarb"), "missing SARB");
        assert!(ids.contains(&"za-fsca"), "missing FSCA");
        // Egypt
        assert!(ids.contains(&"eg-cbe"), "missing CBE");
        assert!(ids.contains(&"eg-fra"), "missing FRA");
        // Tanzania / Zanzibar
        assert!(ids.contains(&"tz-bot"), "missing BOT");
        assert!(ids.contains(&"tz-zanzibar-br"), "missing Zanzibar BR");
        // Ireland
        assert!(ids.contains(&"ie-cbi"), "missing CBI");
        assert!(ids.contains(&"ie-cro"), "missing CRO");
    }

    #[test]
    fn regulators_have_correct_jurisdictions() {
        let regs = additional_regulators();
        for reg in &regs {
            match reg.regulator_id.as_str() {
                "pt-bdp" | "pt-cmvm" => {
                    assert_eq!(reg.jurisdiction_id, "pt", "{} is not pt", reg.regulator_id);
                }
                "id-ojk" | "id-bi" => {
                    assert_eq!(reg.jurisdiction_id, "id", "{} is not id", reg.regulator_id);
                }
                "za-sarb" | "za-fsca" => {
                    assert_eq!(reg.jurisdiction_id, "za", "{} is not za", reg.regulator_id);
                }
                "eg-cbe" | "eg-fra" => {
                    assert_eq!(reg.jurisdiction_id, "eg", "{} is not eg", reg.regulator_id);
                }
                "tz-bot" => {
                    assert_eq!(reg.jurisdiction_id, "tz", "{} is not tz", reg.regulator_id);
                }
                "tz-zanzibar-br" => {
                    assert_eq!(
                        reg.jurisdiction_id, "tz-zanzibar",
                        "{} is not tz-zanzibar",
                        reg.regulator_id
                    );
                }
                "ie-cbi" | "ie-cro" => {
                    assert_eq!(reg.jurisdiction_id, "ie", "{} is not ie", reg.regulator_id);
                }
                _ => panic!("unexpected regulator_id: {}", reg.regulator_id),
            }
        }
    }

    #[test]
    fn additional_license_types_cover_all_authorities() {
        let types = additional_license_types();
        assert!(
            types.len() >= 62,
            "expected >= 62 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("pt-bdp"), "no BdP license types");
        assert!(authority_ids.contains("pt-cmvm"), "no CMVM license types");
        assert!(authority_ids.contains("id-ojk"), "no OJK license types");
        assert!(authority_ids.contains("id-bi"), "no BI license types");
        assert!(authority_ids.contains("za-sarb"), "no SARB license types");
        assert!(authority_ids.contains("za-fsca"), "no FSCA license types");
        assert!(authority_ids.contains("eg-cbe"), "no CBE license types");
        assert!(authority_ids.contains("eg-fra"), "no FRA license types");
        assert!(authority_ids.contains("tz-bot"), "no BOT license types");
        assert!(
            authority_ids.contains("tz-zanzibar-br"),
            "no Zanzibar BR license types"
        );
        assert!(authority_ids.contains("ie-cbi"), "no CBI license types");
        assert!(authority_ids.contains("ie-cro"), "no CRO license types");
    }

    #[test]
    fn portugal_has_banking_and_securities() {
        let bdp = bdp_license_types();
        let bdp_ids: Vec<&str> = bdp.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(bdp_ids.contains(&"pt-bdp:credit-institution"));
        assert!(bdp_ids.contains(&"pt-bdp:vasp"));

        let cmvm = cmvm_license_types();
        let cmvm_ids: Vec<&str> = cmvm.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(cmvm_ids.contains(&"pt-cmvm:investment-firm"));
        assert!(cmvm_ids.contains(&"pt-cmvm:fund-manager"));
    }

    #[test]
    fn indonesia_has_banking_and_payments() {
        let ojk = ojk_license_types();
        let ojk_ids: Vec<&str> = ojk.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ojk_ids.contains(&"id-ojk:commercial-bank"));
        assert!(ojk_ids.contains(&"id-ojk:fintech-lending"));

        let bi = bi_license_types();
        let bi_ids: Vec<&str> = bi.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(bi_ids.contains(&"id-bi:payment-system-provider"));
        assert!(bi_ids.contains(&"id-bi:e-money-issuer"));
    }

    #[test]
    fn south_africa_has_banking_and_financial_services() {
        let sarb = sarb_license_types();
        let sarb_ids: Vec<&str> = sarb.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(sarb_ids.contains(&"za-sarb:banking"));
        assert!(sarb_ids.contains(&"za-sarb:casp"));

        let fsca = fsca_license_types();
        let fsca_ids: Vec<&str> = fsca.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(fsca_ids.contains(&"za-fsca:fsp"));
        assert!(fsca_ids.contains(&"za-fsca:insurance"));
    }

    #[test]
    fn egypt_has_banking_and_non_bank() {
        let cbe = cbe_license_types();
        let cbe_ids: Vec<&str> = cbe.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(cbe_ids.contains(&"eg-cbe:commercial-bank"));
        assert!(cbe_ids.contains(&"eg-cbe:payment-service-provider"));

        let fra = fra_license_types();
        let fra_ids: Vec<&str> = fra.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(fra_ids.contains(&"eg-fra:securities-brokerage"));
        assert!(fra_ids.contains(&"eg-fra:insurance"));
    }

    #[test]
    fn tanzania_has_banking_and_zanzibar() {
        let bot = bot_license_types();
        let bot_ids: Vec<&str> = bot.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(bot_ids.contains(&"tz-bot:commercial-bank"));
        assert!(bot_ids.contains(&"tz-bot:mobile-money"));

        let znz = zanzibar_br_license_types();
        let znz_ids: Vec<&str> = znz.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(znz_ids.contains(&"tz-zanzibar-br:business-license"));
        assert!(znz_ids.contains(&"tz-zanzibar-br:company-registration"));
    }

    #[test]
    fn ireland_has_banking_and_corporate() {
        let cbi = cbi_license_types();
        let cbi_ids: Vec<&str> = cbi.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(cbi_ids.contains(&"ie-cbi:credit-institution"));
        assert!(cbi_ids.contains(&"ie-cbi:vasp"));
        assert!(cbi_ids.contains(&"ie-cbi:e-money-institution"));

        let cro = cro_license_types();
        let cro_ids: Vec<&str> = cro.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(cro_ids.contains(&"ie-cro:private-company-limited"));
        assert!(cro_ids.contains(&"ie-cro:icav"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = additional_license_types();
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
        for lt in additional_license_types() {
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
        for lt in additional_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in additional_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in additional_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
