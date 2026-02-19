//! # Kenya Regulatory Authority License Mappings
//!
//! Kenya-specific license type definitions covering the four major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **CBK** | Central Bank of Kenya | Banking, Payments |
//! | **CMA** | Capital Markets Authority | Securities, Investment |
//! | **IRA** | Insurance Regulatory Authority | Insurance |
//! | **NSE** | Nairobi Securities Exchange | Listed Company Compliance |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Kenya-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── CBK — Central Bank of Kenya ──────────────────────────────────────────────

/// CBK regulator profile.
pub fn cbk_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ke-cbk".to_string(),
        name: "Central Bank of Kenya".to_string(),
        jurisdiction_id: "ke".to_string(),
        registry_url: Some("https://www.centralbank.go.ke".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string(), "license_status".to_string()],
    }
}

/// CBK license type definitions.
pub fn cbk_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "License to operate as a commercial bank under the Banking Act"
                .to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KES".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KES".to_string(), "500000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:microfinance-bank".to_string(),
            name: "Microfinance Bank License".to_string(),
            description:
                "License to operate as a microfinance bank under the Microfinance Act 2006"
                    .to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "micro_lending".to_string(),
                "micro_deposit_taking".to_string(),
                "micro_insurance_agency".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:mortgage-finance".to_string(),
            name: "Mortgage Finance Company License".to_string(),
            description: "License to operate as a mortgage finance company in Kenya".to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "mortgage_lending".to_string(),
                "deposit_taking".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:forex-dealer".to_string(),
            name: "Foreign Exchange Dealer License".to_string(),
            description: "License to operate as a forex bureau in Kenya".to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "foreign_exchange".to_string(),
                "currency_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KES".to_string(), "50000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KES".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:money-remittance".to_string(),
            name: "Money Remittance License".to_string(),
            description: "License to provide money remittance services in Kenya".to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "remittance".to_string(),
                "money_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cbk:payment-service-provider".to_string(),
            name: "Payment Service Provider License".to_string(),
            description:
                "License to provide payment services under the National Payment System Act"
                    .to_string(),
            regulator_id: "ke-cbk".to_string(),
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
            license_type_id: "ke-cbk:digital-credit-provider".to_string(),
            name: "Digital Credit Provider License".to_string(),
            description: "License to provide digital credit services in Kenya under CBK Act"
                .to_string(),
            regulator_id: "ke-cbk".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_lending".to_string(),
                "credit_scoring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
    ]
}

// ── CMA — Capital Markets Authority ──────────────────────────────────────────

/// CMA regulator profile.
pub fn cma_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ke-cma".to_string(),
        name: "Capital Markets Authority".to_string(),
        jurisdiction_id: "ke".to_string(),
        registry_url: Some("https://www.cma.or.ke".to_string()),
        did: None,
        api_capabilities: vec!["licensee_directory".to_string()],
    }
}

/// CMA license type definitions.
pub fn cma_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ke-cma:stockbroker".to_string(),
            name: "Stockbroker License".to_string(),
            description: "License to operate as a stockbroker in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "order_execution".to_string(),
                "trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KES".to_string(), "50000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KES".to_string(), "200000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:dealer".to_string(),
            name: "Securities Dealer License".to_string(),
            description: "License to deal in securities in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "proprietary_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:investment-bank".to_string(),
            name: "Investment Bank License".to_string(),
            description: "License to operate as an investment bank in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "underwriting".to_string(),
                "advisory".to_string(),
                "corporate_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:fund-manager".to_string(),
            name: "Fund Manager License".to_string(),
            description: "License to manage collective investment schemes in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "portfolio_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:investment-adviser".to_string(),
            name: "Investment Adviser License".to_string(),
            description: "License to provide investment advisory services in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_planning".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:credit-rating-agency".to_string(),
            name: "Credit Rating Agency License".to_string(),
            description: "License to operate as a credit rating agency in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "credit_rating".to_string(),
                "rating_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-cma:securities-exchange".to_string(),
            name: "Securities Exchange License".to_string(),
            description: "License to operate a securities exchange in Kenya".to_string(),
            regulator_id: "ke-cma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "exchange_operations".to_string(),
                "listing_services".to_string(),
                "market_surveillance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── IRA — Insurance Regulatory Authority ─────────────────────────────────────

/// IRA regulator profile.
pub fn ira_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ke-ira".to_string(),
        name: "Insurance Regulatory Authority".to_string(),
        jurisdiction_id: "ke".to_string(),
        registry_url: Some("https://www.ira.go.ke".to_string()),
        did: None,
        api_capabilities: vec!["insurer_registry".to_string()],
    }
}

/// IRA license type definitions.
pub fn ira_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ke-ira:general-insurer".to_string(),
            name: "General Insurance Company License".to_string(),
            description: "License to carry on general insurance business in Kenya".to_string(),
            regulator_id: "ke-ira".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "property_insurance".to_string(),
                "motor_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-ira:life-insurer".to_string(),
            name: "Life Insurance Company License".to_string(),
            description: "License to carry on life insurance business in Kenya".to_string(),
            regulator_id: "ke-ira".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "annuities".to_string(),
                "pension_products".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-ira:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to operate as an insurance broker in Kenya".to_string(),
            regulator_id: "ke-ira".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "risk_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-ira:insurance-agent".to_string(),
            name: "Insurance Agent License".to_string(),
            description: "License to operate as an insurance agent in Kenya".to_string(),
            regulator_id: "ke-ira".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_sales".to_string(),
                "policy_servicing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ke-ira:reinsurer".to_string(),
            name: "Reinsurance License".to_string(),
            description: "License to conduct reinsurance business in Kenya".to_string(),
            regulator_id: "ke-ira".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "reinsurance".to_string(),
                "retrocession".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
    ]
}

// ── NSE — Nairobi Securities Exchange ────────────────────────────────────────

/// NSE regulator profile.
pub fn nse_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ke-nse".to_string(),
        name: "Nairobi Securities Exchange".to_string(),
        jurisdiction_id: "ke".to_string(),
        registry_url: Some("https://www.nse.co.ke".to_string()),
        did: None,
        api_capabilities: vec!["listed_company_search".to_string()],
    }
}

/// NSE license type definitions.
pub fn nse_license_types() -> Vec<LicenseTypeDefinition> {
    vec![LicenseTypeDefinition {
        license_type_id: "ke-nse:listed-company".to_string(),
        name: "Listed Company Compliance".to_string(),
        description:
            "Ongoing compliance obligations for companies listed on the Nairobi Securities Exchange"
                .to_string(),
        regulator_id: "ke-nse".to_string(),
        category: Some("corporate".to_string()),
        permitted_activities: vec![
            "public_listing".to_string(),
            "capital_raising".to_string(),
            "securities_issuance".to_string(),
        ],
        requirements: BTreeMap::new(),
        application_fee: BTreeMap::new(),
        annual_fee: [("KES".to_string(), "500000".to_string())]
            .into_iter()
            .collect(),
        validity_period_years: None,
    }]
}

// ── Kenya Registry Aggregation ───────────────────────────────────────────────

/// All Kenya regulatory authorities.
pub fn kenya_regulators() -> Vec<LicensepackRegulator> {
    vec![
        cbk_regulator(),
        cma_regulator(),
        ira_regulator(),
        nse_regulator(),
    ]
}

/// All Kenya license type definitions across all authorities.
pub fn kenya_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(cbk_license_types());
    all.extend(cma_license_types());
    all.extend(ira_license_types());
    all.extend(nse_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kenya_has_four_regulators() {
        let regs = kenya_regulators();
        assert_eq!(regs.len(), 4);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"ke-cbk"), "missing CBK");
        assert!(ids.contains(&"ke-cma"), "missing CMA");
        assert!(ids.contains(&"ke-ira"), "missing IRA");
        assert!(ids.contains(&"ke-nse"), "missing NSE");
    }

    #[test]
    fn all_regulators_are_kenya_jurisdiction() {
        for reg in kenya_regulators() {
            assert_eq!(reg.jurisdiction_id, "ke", "{} is not ke", reg.regulator_id);
        }
    }

    #[test]
    fn kenya_license_types_cover_all_authorities() {
        let types = kenya_license_types();
        assert!(
            types.len() >= 20,
            "expected >= 20 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("ke-cbk"), "no CBK license types");
        assert!(authority_ids.contains("ke-cma"), "no CMA license types");
        assert!(authority_ids.contains("ke-ira"), "no IRA license types");
        assert!(authority_ids.contains("ke-nse"), "no NSE license types");
    }

    #[test]
    fn cbk_has_banking_and_payments() {
        let types = cbk_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ke-cbk:commercial-bank"));
        assert!(ids.contains(&"ke-cbk:microfinance-bank"));
        assert!(ids.contains(&"ke-cbk:payment-service-provider"));
        assert!(ids.contains(&"ke-cbk:digital-credit-provider"));
    }

    #[test]
    fn cma_has_securities_licenses() {
        let types = cma_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ke-cma:stockbroker"));
        assert!(ids.contains(&"ke-cma:fund-manager"));
        assert!(ids.contains(&"ke-cma:investment-adviser"));
        assert!(ids.contains(&"ke-cma:credit-rating-agency"));
    }

    #[test]
    fn ira_has_insurance_licenses() {
        let types = ira_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ke-ira:general-insurer"));
        assert!(ids.contains(&"ke-ira:life-insurer"));
        assert!(ids.contains(&"ke-ira:insurance-broker"));
        assert!(ids.contains(&"ke-ira:reinsurer"));
    }

    #[test]
    fn nse_has_listing_compliance() {
        let types = nse_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ke-nse:listed-company"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = kenya_license_types();
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
        for lt in kenya_license_types() {
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
        for lt in kenya_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in kenya_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in kenya_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
