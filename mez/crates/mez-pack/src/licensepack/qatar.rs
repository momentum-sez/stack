//! # Qatar + QFC Regulatory Authority License Mappings
//!
//! Qatar-specific license type definitions covering the onshore regulators
//! and the Qatar Financial Centre (QFC) free zone:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **QCB** | Qatar Central Bank | Banking, Insurance, Payments |
//! | **QFMA** | Qatar Financial Markets Authority | Securities, Investment |
//! | **QFCRA** | QFC Regulatory Authority | Financial Services (QFC) |
//! | **QFC Authority** | QFC Authority (Business Registration) | Corporate (QFC) |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Qatar-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── QCB — Qatar Central Bank ─────────────────────────────────────────────────

/// QCB regulator profile.
pub fn qcb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "qa-qcb".to_string(),
        name: "Qatar Central Bank".to_string(),
        jurisdiction_id: "qa".to_string(),
        registry_url: Some("https://www.qcb.gov.qa".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string(), "license_status".to_string()],
    }
}

/// QCB license type definitions.
pub fn qcb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "License to operate as a commercial bank in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "foreign_exchange".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:islamic-bank".to_string(),
            name: "Islamic Banking License".to_string(),
            description: "License to operate as an Islamic bank under Sharia-compliant principles"
                .to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "islamic_deposit_taking".to_string(),
                "islamic_financing".to_string(),
                "murabaha".to_string(),
                "ijara".to_string(),
                "sukuk".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:investment-bank".to_string(),
            name: "Investment Banking License".to_string(),
            description: "License to operate as an investment bank in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "underwriting".to_string(),
                "advisory".to_string(),
                "capital_markets".to_string(),
                "proprietary_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:specialized-bank".to_string(),
            name: "Specialized Banking License".to_string(),
            description: "License to operate a specialized banking institution in Qatar"
                .to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "specialized_lending".to_string(),
                "development_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:insurer".to_string(),
            name: "Insurance Company License".to_string(),
            description: "License to operate as an insurance company in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
                "underwriting".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:reinsurer".to_string(),
            name: "Reinsurance License".to_string(),
            description: "License to operate as a reinsurer in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to operate as an insurance broker in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
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
            license_type_id: "qa-qcb:insurance-agent".to_string(),
            name: "Insurance Agent License".to_string(),
            description: "License to operate as an insurance agent in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
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
            license_type_id: "qa-qcb:payment-service-provider".to_string(),
            name: "Payment Service Provider License".to_string(),
            description: "License to provide payment services in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "payment_initiation".to_string(),
                "e_money".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qcb:money-exchange".to_string(),
            name: "Money Exchange License".to_string(),
            description: "License to operate a money exchange business in Qatar".to_string(),
            regulator_id: "qa-qcb".to_string(),
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
    ]
}

// ── QFMA — Qatar Financial Markets Authority ─────────────────────────────────

/// QFMA regulator profile.
pub fn qfma_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "qa-qfma".to_string(),
        name: "Qatar Financial Markets Authority".to_string(),
        jurisdiction_id: "qa".to_string(),
        registry_url: Some("https://www.qfma.org.qa".to_string()),
        did: None,
        api_capabilities: vec!["license_query".to_string()],
    }
}

/// QFMA license type definitions.
pub fn qfma_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "qa-qfma:securities-broker".to_string(),
            name: "Securities Broker License".to_string(),
            description: "License to operate as a securities broker on the Qatar Stock Exchange"
                .to_string(),
            regulator_id: "qa-qfma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "order_execution".to_string(),
                "trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfma:investment-manager".to_string(),
            name: "Investment Manager License".to_string(),
            description: "License to manage investment portfolios in Qatar".to_string(),
            regulator_id: "qa-qfma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "portfolio_management".to_string(),
                "investment_advisory".to_string(),
                "discretionary_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfma:fund-administrator".to_string(),
            name: "Fund Administrator License".to_string(),
            description: "License to provide fund administration services in Qatar".to_string(),
            regulator_id: "qa-qfma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_administration".to_string(),
                "nav_calculation".to_string(),
                "investor_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfma:market-making".to_string(),
            name: "Market Making License".to_string(),
            description: "License to operate as a market maker on the Qatar Stock Exchange"
                .to_string(),
            regulator_id: "qa-qfma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "market_making".to_string(),
                "liquidity_provision".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfma:custodian".to_string(),
            name: "Custodian License".to_string(),
            description: "License to provide custody services for securities in Qatar".to_string(),
            regulator_id: "qa-qfma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "custody_services".to_string(),
                "safekeeping".to_string(),
                "settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── QFCRA — QFC Regulatory Authority ─────────────────────────────────────────

/// QFCRA regulator profile.
pub fn qfcra_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "qa-qfc-ra".to_string(),
        name: "QFC Regulatory Authority".to_string(),
        jurisdiction_id: "qa-qfc".to_string(),
        registry_url: Some("https://www.qfcra.com".to_string()),
        did: None,
        api_capabilities: vec!["firm_directory".to_string(), "license_status".to_string()],
    }
}

/// QFCRA license type definitions.
pub fn qfcra_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:banking-business".to_string(),
            name: "Banking Business License".to_string(),
            description: "License to conduct banking business within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:insurance-general".to_string(),
            name: "General Insurance Business License".to_string(),
            description: "License to conduct general insurance business within the QFC"
                .to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "property_insurance".to_string(),
                "casualty_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:insurance-long-term".to_string(),
            name: "Long-Term Insurance Business License".to_string(),
            description: "License to conduct long-term (life) insurance business within the QFC"
                .to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
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
            license_type_id: "qa-qfc-ra:insurance-mediation".to_string(),
            name: "Insurance Mediation License".to_string(),
            description: "License to conduct insurance mediation within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:investment-management".to_string(),
            name: "Investment Management License".to_string(),
            description: "License to manage investments within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "portfolio_management".to_string(),
                "discretionary_management".to_string(),
                "fund_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:advisory-services".to_string(),
            name: "Advisory Services License".to_string(),
            description: "License to provide financial advisory services within the QFC"
                .to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_planning".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:fund-administration".to_string(),
            name: "Fund Administration License".to_string(),
            description: "License to provide fund administration services within the QFC"
                .to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_administration".to_string(),
                "nav_calculation".to_string(),
                "transfer_agency".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:trust-services".to_string(),
            name: "Trust Services License".to_string(),
            description: "License to provide trust services within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "trust_administration".to_string(),
                "fiduciary_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:money-transmission".to_string(),
            name: "Money Transmission License".to_string(),
            description: "License to provide money transmission services within the QFC"
                .to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_transmission".to_string(),
                "remittance".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:digital-asset-services".to_string(),
            name: "Digital Asset Services License".to_string(),
            description: "License to provide digital asset services within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_exchange".to_string(),
                "digital_asset_custody".to_string(),
                "digital_asset_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-ra:credit-rating-agency".to_string(),
            name: "Credit Rating Agency License".to_string(),
            description: "License to operate as a credit rating agency within the QFC".to_string(),
            regulator_id: "qa-qfc-ra".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "credit_rating".to_string(),
                "rating_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── QFC Authority — Business Registration ────────────────────────────────────

/// QFC Authority (Business Registration) regulator profile.
pub fn qfc_auth_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "qa-qfc-auth".to_string(),
        name: "QFC Authority (Business Registration)".to_string(),
        jurisdiction_id: "qa-qfc".to_string(),
        registry_url: Some("https://www.qfc.qa".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string(), "registration_status".to_string()],
    }
}

/// QFC Authority license type definitions.
pub fn qfc_auth_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-auth:qfc-company".to_string(),
            name: "QFC Company Registration".to_string(),
            description: "Registration of a company within the Qatar Financial Centre".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "financial_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-auth:qfc-llc".to_string(),
            name: "QFC LLC Registration".to_string(),
            description: "Registration of a limited liability company within the QFC".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
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
            license_type_id: "qa-qfc-auth:qfc-branch".to_string(),
            name: "QFC Branch Registration".to_string(),
            description: "Registration of a foreign company branch within the QFC".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
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
            license_type_id: "qa-qfc-auth:qfc-partnership".to_string(),
            name: "QFC Partnership Registration".to_string(),
            description: "Registration of a partnership within the QFC".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "partnership_operations".to_string(),
                "professional_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-auth:qfc-spv".to_string(),
            name: "QFC SPV Registration".to_string(),
            description: "Registration of a special purpose vehicle within the QFC".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "securitization".to_string(),
                "asset_holding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "qa-qfc-auth:qfc-foundation".to_string(),
            name: "QFC Foundation Registration".to_string(),
            description: "Registration of a foundation within the QFC".to_string(),
            regulator_id: "qa-qfc-auth".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "asset_holding".to_string(),
                "wealth_preservation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Qatar Registry Aggregation ───────────────────────────────────────────────

/// All Qatar and QFC regulatory authorities.
pub fn qatar_regulators() -> Vec<LicensepackRegulator> {
    vec![
        qcb_regulator(),
        qfma_regulator(),
        qfcra_regulator(),
        qfc_auth_regulator(),
    ]
}

/// All Qatar and QFC license type definitions across all authorities.
pub fn qatar_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(qcb_license_types());
    all.extend(qfma_license_types());
    all.extend(qfcra_license_types());
    all.extend(qfc_auth_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qatar_has_four_regulators() {
        let regs = qatar_regulators();
        assert_eq!(regs.len(), 4);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"qa-qcb"), "missing QCB");
        assert!(ids.contains(&"qa-qfma"), "missing QFMA");
        assert!(ids.contains(&"qa-qfc-ra"), "missing QFCRA");
        assert!(ids.contains(&"qa-qfc-auth"), "missing QFC Authority");
    }

    #[test]
    fn qatar_onshore_regulators_have_qa_jurisdiction() {
        let regs = qatar_regulators();
        for reg in &regs {
            match reg.regulator_id.as_str() {
                "qa-qcb" | "qa-qfma" => {
                    assert_eq!(reg.jurisdiction_id, "qa", "{} is not qa", reg.regulator_id);
                }
                "qa-qfc-ra" | "qa-qfc-auth" => {
                    assert_eq!(
                        reg.jurisdiction_id, "qa-qfc",
                        "{} is not qa-qfc",
                        reg.regulator_id
                    );
                }
                _ => panic!("unexpected regulator_id: {}", reg.regulator_id),
            }
        }
    }

    #[test]
    fn qatar_license_types_cover_all_authorities() {
        let types = qatar_license_types();
        assert!(
            types.len() >= 32,
            "expected >= 32 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("qa-qcb"), "no QCB license types");
        assert!(authority_ids.contains("qa-qfma"), "no QFMA license types");
        assert!(
            authority_ids.contains("qa-qfc-ra"),
            "no QFCRA license types"
        );
        assert!(
            authority_ids.contains("qa-qfc-auth"),
            "no QFC Authority license types"
        );
    }

    #[test]
    fn qcb_has_banking_and_insurance() {
        let types = qcb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"qa-qcb:commercial-bank"));
        assert!(ids.contains(&"qa-qcb:islamic-bank"));
        assert!(ids.contains(&"qa-qcb:insurer"));
        assert!(ids.contains(&"qa-qcb:payment-service-provider"));
        assert!(ids.contains(&"qa-qcb:money-exchange"));
    }

    #[test]
    fn qfma_has_securities_licenses() {
        let types = qfma_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"qa-qfma:securities-broker"));
        assert!(ids.contains(&"qa-qfma:investment-manager"));
        assert!(ids.contains(&"qa-qfma:custodian"));
    }

    #[test]
    fn qfcra_has_financial_centre_licenses() {
        let types = qfcra_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"qa-qfc-ra:banking-business"));
        assert!(ids.contains(&"qa-qfc-ra:insurance-general"));
        assert!(ids.contains(&"qa-qfc-ra:investment-management"));
        assert!(ids.contains(&"qa-qfc-ra:digital-asset-services"));
        assert!(ids.contains(&"qa-qfc-ra:trust-services"));
    }

    #[test]
    fn qfc_auth_has_entity_registrations() {
        let types = qfc_auth_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"qa-qfc-auth:qfc-company"));
        assert!(ids.contains(&"qa-qfc-auth:qfc-llc"));
        assert!(ids.contains(&"qa-qfc-auth:qfc-branch"));
        assert!(ids.contains(&"qa-qfc-auth:qfc-spv"));
        assert!(ids.contains(&"qa-qfc-auth:qfc-foundation"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = qatar_license_types();
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
        for lt in qatar_license_types() {
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
        for lt in qatar_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in qatar_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in qatar_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
