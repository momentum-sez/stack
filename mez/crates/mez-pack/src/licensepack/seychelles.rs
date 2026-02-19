//! # Seychelles Regulatory Authority License Mappings
//!
//! Seychelles-specific license type definitions covering the three major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **FSA** | Financial Services Authority of Seychelles | Securities, Insurance, Corporate, Digital Assets |
//! | **CBS** | Central Bank of Seychelles | Banking, Payments, Remittance |
//! | **SRC** | Seychelles Revenue Commission | Business & Tax Registration |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Seychelles-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── FSA — Financial Services Authority of Seychelles ────────────────────────

/// FSA regulator profile.
pub fn fsa_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sc-fsa".to_string(),
        name: "Financial Services Authority of Seychelles".to_string(),
        jurisdiction_id: "sc".to_string(),
        registry_url: Some("https://www.fsaseychelles.sc".to_string()),
        did: None,
        api_capabilities: vec!["license_search".to_string(), "entity_registry".to_string()],
    }
}

/// FSA license type definitions.
pub fn fsa_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        // ── Securities ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:securities-dealer".to_string(),
            name: "Securities Dealer License".to_string(),
            description: "License to deal in securities under the Securities Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "securities_brokerage".to_string(),
                "trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:investment-adviser".to_string(),
            name: "Investment Adviser License".to_string(),
            description: "License to provide investment advisory services".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_planning".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "2000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "2000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:fund-management".to_string(),
            name: "Fund Management License".to_string(),
            description: "License to manage investment funds under the Mutual Fund and Hedge Fund Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "portfolio_management".to_string(),
                "asset_allocation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Insurance ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:insurance-domestic".to_string(),
            name: "Domestic Insurance License".to_string(),
            description: "License to carry on domestic insurance business under the Insurance Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "domestic_insurance".to_string(),
                "general_insurance".to_string(),
                "life_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:insurance-captive".to_string(),
            name: "Captive Insurance License".to_string(),
            description: "License to carry on captive insurance business".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "captive_insurance".to_string(),
                "self_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "3000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to act as an insurance broker".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "insurance_placement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:insurance-agent".to_string(),
            name: "Insurance Agent License".to_string(),
            description: "License to act as an insurance agent".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_solicitation".to_string(),
                "insurance_sales".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Corporate / IBC / GBC ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:ibc".to_string(),
            name: "International Business Company (IBC) Registration".to_string(),
            description: "Registration of an International Business Company (legacy, now Global Business Company) under the IBC Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "international_trading".to_string(),
                "holding_company".to_string(),
                "investment_holding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:gbc".to_string(),
            name: "Global Business Company (GBC) Registration".to_string(),
            description: "Registration of a Global Business Company under the Companies (Special Licenses) Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "international_trading".to_string(),
                "holding_company".to_string(),
                "investment_holding".to_string(),
                "headquarters_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── CSP ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:csp".to_string(),
            name: "Corporate Service Provider License".to_string(),
            description: "License to provide corporate services including company formation and registered agent services".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "company_formation".to_string(),
                "registered_agent_services".to_string(),
                "corporate_administration".to_string(),
                "nominee_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "2000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "2000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Money Changing ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:money-changing".to_string(),
            name: "Money Changing License".to_string(),
            description: "License to operate a money changing (bureau de change) business under FSA oversight".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "currency_exchange".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Credit Union ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:credit-union".to_string(),
            name: "Credit Union License".to_string(),
            description: "License to operate a credit union under the Credit Union Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "member_deposit_taking".to_string(),
                "member_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        // ── Digital Assets ──
        LicenseTypeDefinition {
            license_type_id: "sc-fsa:digital-asset-service-provider".to_string(),
            name: "Digital Asset Service Provider License".to_string(),
            description: "License to operate as a digital asset service provider under the Digital Asset Business Act".to_string(),
            regulator_id: "sc-fsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_exchange".to_string(),
                "digital_asset_custody".to_string(),
                "digital_asset_transfer".to_string(),
                "digital_asset_issuance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── CBS — Central Bank of Seychelles ────────────────────────────────────────

/// CBS regulator profile.
pub fn cbs_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sc-cbs".to_string(),
        name: "Central Bank of Seychelles".to_string(),
        jurisdiction_id: "sc".to_string(),
        registry_url: Some("https://www.cbs.sc".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string()],
    }
}

/// CBS license type definitions.
pub fn cbs_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "sc-cbs:banking-domestic".to_string(),
            name: "Domestic Banking License".to_string(),
            description: "License to carry on domestic banking business under the Financial Institutions Act".to_string(),
            regulator_id: "sc-cbs".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
                "domestic_banking".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "100000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sc-cbs:banking-offshore".to_string(),
            name: "Offshore Banking License".to_string(),
            description: "License to carry on offshore banking business".to_string(),
            regulator_id: "sc-cbs".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "international_deposit_taking".to_string(),
                "international_lending".to_string(),
                "foreign_exchange".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sc-cbs:money-value-transfer".to_string(),
            name: "Money Value Transfer (Remittance) License".to_string(),
            description: "License to operate a money value transfer (remittance) service".to_string(),
            regulator_id: "sc-cbs".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "remittance".to_string(),
                "money_transfer".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-cbs:bureau-de-change".to_string(),
            name: "Bureau de Change License".to_string(),
            description: "License to operate a bureau de change (foreign currency exchange)".to_string(),
            regulator_id: "sc-cbs".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "currency_exchange".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "sc-cbs:payment-service-provider".to_string(),
            name: "Payment Service Provider License".to_string(),
            description: "License to operate as a payment service provider under the National Payment System Act".to_string(),
            regulator_id: "sc-cbs".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "payment_services".to_string(),
                "e_money_issuance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("SCR".to_string(), "25000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── SRC — Seychelles Revenue Commission ─────────────────────────────────────

/// SRC regulator profile.
pub fn src_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "sc-src".to_string(),
        name: "Seychelles Revenue Commission".to_string(),
        jurisdiction_id: "sc".to_string(),
        registry_url: Some("https://www.src.gov.sc".to_string()),
        did: None,
        api_capabilities: vec!["registration_status".to_string()],
    }
}

/// SRC license type definitions (business and tax registrations).
pub fn src_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "sc-src:business-registration".to_string(),
            name: "Business Registration".to_string(),
            description: "Registration of a business entity with the Seychelles Revenue Commission".to_string(),
            regulator_id: "sc-src".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "local_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("SCR".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "sc-src:tax-registration".to_string(),
            name: "Tax Registration".to_string(),
            description: "Tax registration for VAT and business tax purposes with the Seychelles Revenue Commission".to_string(),
            regulator_id: "sc-src".to_string(),
            category: Some("tax_compliance".to_string()),
            permitted_activities: vec![
                "tax_reporting".to_string(),
                "vat_collection".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Seychelles Registry Aggregation ─────────────────────────────────────────

/// All Seychelles regulatory authorities.
pub fn seychelles_regulators() -> Vec<LicensepackRegulator> {
    vec![
        fsa_regulator(),
        cbs_regulator(),
        src_regulator(),
    ]
}

/// All Seychelles license type definitions across all authorities.
pub fn seychelles_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(fsa_license_types());
    all.extend(cbs_license_types());
    all.extend(src_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seychelles_has_three_regulators() {
        let regs = seychelles_regulators();
        assert_eq!(regs.len(), 3);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa"), "missing FSA");
        assert!(ids.contains(&"sc-cbs"), "missing CBS");
        assert!(ids.contains(&"sc-src"), "missing SRC");
    }

    #[test]
    fn all_regulators_are_seychelles_jurisdiction() {
        for reg in seychelles_regulators() {
            assert_eq!(reg.jurisdiction_id, "sc", "{} is not sc", reg.regulator_id);
        }
    }

    #[test]
    fn seychelles_license_types_cover_all_authorities() {
        let types = seychelles_license_types();
        assert!(
            types.len() >= 20,
            "expected >= 20 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("sc-fsa"), "no FSA license types");
        assert!(authority_ids.contains("sc-cbs"), "no CBS license types");
        assert!(authority_ids.contains("sc-src"), "no SRC license types");
    }

    #[test]
    fn fsa_has_securities_licenses() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:securities-dealer"));
        assert!(ids.contains(&"sc-fsa:investment-adviser"));
        assert!(ids.contains(&"sc-fsa:fund-management"));
    }

    #[test]
    fn fsa_has_insurance_licenses() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:insurance-domestic"));
        assert!(ids.contains(&"sc-fsa:insurance-captive"));
        assert!(ids.contains(&"sc-fsa:insurance-broker"));
        assert!(ids.contains(&"sc-fsa:insurance-agent"));
    }

    #[test]
    fn fsa_has_corporate_licenses() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:ibc"));
        assert!(ids.contains(&"sc-fsa:gbc"));
        assert!(ids.contains(&"sc-fsa:csp"));
    }

    #[test]
    fn fsa_has_money_changing_license() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:money-changing"));
    }

    #[test]
    fn fsa_has_credit_union_license() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:credit-union"));
    }

    #[test]
    fn fsa_has_digital_asset_license() {
        let types = fsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-fsa:digital-asset-service-provider"));
    }

    #[test]
    fn cbs_has_banking_licenses() {
        let types = cbs_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-cbs:banking-domestic"));
        assert!(ids.contains(&"sc-cbs:banking-offshore"));
    }

    #[test]
    fn cbs_has_money_transfer_and_exchange_licenses() {
        let types = cbs_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-cbs:money-value-transfer"));
        assert!(ids.contains(&"sc-cbs:bureau-de-change"));
    }

    #[test]
    fn cbs_has_payment_service_provider_license() {
        let types = cbs_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-cbs:payment-service-provider"));
    }

    #[test]
    fn src_has_registration_types() {
        let types = src_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"sc-src:business-registration"));
        assert!(ids.contains(&"sc-src:tax-registration"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = seychelles_license_types();
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
        for lt in seychelles_license_types() {
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
        for lt in seychelles_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in seychelles_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in seychelles_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
