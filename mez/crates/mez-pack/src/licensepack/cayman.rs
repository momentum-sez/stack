//! # Cayman Islands Regulatory Authority License Mappings
//!
//! Cayman Islands-specific license type definitions covering the three major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **CIMA** | Cayman Islands Monetary Authority | Financial, Insurance, Securities, Money Services |
//! | **ROC** | Registrar of Companies | Corporate Registration |
//! | **DITC** | Department of International Tax Cooperation | Tax Compliance |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Cayman Islands-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── CIMA — Cayman Islands Monetary Authority ────────────────────────────────

/// CIMA regulator profile.
pub fn cima_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ky-cima".to_string(),
        name: "Cayman Islands Monetary Authority".to_string(),
        jurisdiction_id: "ky".to_string(),
        registry_url: Some("https://www.cima.ky".to_string()),
        did: None,
        api_capabilities: vec!["license_search".to_string(), "entity_registry".to_string()],
    }
}

/// CIMA license type definitions.
pub fn cima_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        // ── Banking ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:banking-class-a".to_string(),
            name: "Class A Banking License (Unrestricted)".to_string(),
            description: "Unrestricted banking license to conduct domestic and international banking business under the Banks and Trust Companies Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "foreign_exchange".to_string(),
                "trade_finance".to_string(),
                "domestic_banking".to_string(),
                "international_banking".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "30500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "60000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:banking-class-b".to_string(),
            name: "Class B Banking License (Restricted)".to_string(),
            description: "Restricted banking license for international banking business only, not permitted to take deposits from Cayman residents".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "international_deposit_taking".to_string(),
                "international_lending".to_string(),
                "foreign_exchange".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "12200".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "36600".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── Insurance ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-class-a".to_string(),
            name: "Class A Insurance License (Domestic)".to_string(),
            description: "License to carry on domestic insurance business in the Cayman Islands under the Insurance Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "domestic_insurance".to_string(),
                "general_insurance".to_string(),
                "life_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-class-b".to_string(),
            name: "Class B Insurance License (External/Captive)".to_string(),
            description: "License to carry on external or captive insurance business".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "captive_insurance".to_string(),
                "external_insurance".to_string(),
                "reinsurance_captive".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "9150".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-class-c".to_string(),
            name: "Class C Insurance License (Reinsurer)".to_string(),
            description: "License to carry on reinsurance business".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "reinsurance".to_string(),
                "retrocession".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "9150".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-manager".to_string(),
            name: "Insurance Manager License".to_string(),
            description: "License to act as an insurance manager in the Cayman Islands".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_management".to_string(),
                "captive_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to act as an insurance broker".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "insurance_placement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:insurance-agent".to_string(),
            name: "Insurance Agent License".to_string(),
            description: "License to act as an insurance agent".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_solicitation".to_string(),
                "insurance_sales".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "1525".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "1525".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Securities Investment Business ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:securities".to_string(),
            name: "Securities Investment Business License".to_string(),
            description: "License to carry on securities investment business under the Securities Investment Business Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "securities_brokerage".to_string(),
                "portfolio_management".to_string(),
                "investment_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:fund-administration".to_string(),
            name: "Fund Administration License".to_string(),
            description: "License to provide fund administration services".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_administration".to_string(),
                "nav_calculation".to_string(),
                "investor_servicing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:mutual-fund".to_string(),
            name: "Mutual Fund Registration".to_string(),
            description: "Registration of a mutual fund under the Mutual Funds Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "collective_investment".to_string(),
                "fund_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "4270".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:securities-vasp".to_string(),
            name: "Virtual Asset Service Provider (Securities) License".to_string(),
            description: "License to provide virtual asset services under the Virtual Asset (Service Providers) Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_exchange".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_transfer".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Money Services ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:money-transmission".to_string(),
            name: "Money Transmission License".to_string(),
            description: "License to operate a money transmission or remittance business under the Money Services Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_transmission".to_string(),
                "remittance".to_string(),
                "payment_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:currency-exchange".to_string(),
            name: "Currency Exchange License".to_string(),
            description: "License to operate a currency exchange business".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "currency_exchange".to_string(),
                "foreign_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "3050".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Trust & Corporate ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:trust-license".to_string(),
            name: "Trust License".to_string(),
            description: "License to carry on trust business under the Banks and Trust Companies Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "trust_administration".to_string(),
                "trust_services".to_string(),
                "fiduciary_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "12200".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "12200".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-cima:company-management".to_string(),
            name: "Company Management License".to_string(),
            description: "License to provide company management services under the Companies Management Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "company_formation".to_string(),
                "registered_office_services".to_string(),
                "corporate_administration".to_string(),
                "directorship_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Cooperative Credit Unions ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:cooperative-credit-union".to_string(),
            name: "Cooperative Credit Union License".to_string(),
            description: "License to operate a cooperative credit union under the Cooperative Societies Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "member_deposit_taking".to_string(),
                "member_lending".to_string(),
                "cooperative_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        // ── VASP (standalone) ──
        LicenseTypeDefinition {
            license_type_id: "ky-cima:vasp".to_string(),
            name: "Virtual Asset Service Provider (VASP) License".to_string(),
            description: "Registration as a virtual asset service provider under the Virtual Asset (Service Providers) Act".to_string(),
            regulator_id: "ky-cima".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_exchange".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_transfer".to_string(),
                "virtual_asset_issuance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "6100".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── ROC — Cayman Islands Registrar of Companies ─────────────────────────────

/// Cayman Islands Registrar of Companies regulator profile.
pub fn roc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ky-roc".to_string(),
        name: "Cayman Islands Registrar of Companies".to_string(),
        jurisdiction_id: "ky".to_string(),
        registry_url: Some("https://www.ciregistry.ky".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string()],
    }
}

/// ROC license type definitions (corporate registrations).
pub fn roc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ky-roc:exempt-company".to_string(),
            name: "Exempt Company Registration".to_string(),
            description: "Registration of an exempted company under the Companies Act".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "holding_company".to_string(),
                "international_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "732".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "854".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-roc:ordinary-company".to_string(),
            name: "Ordinary Company Registration".to_string(),
            description: "Registration of an ordinary (resident) company under the Companies Act".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "domestic_business_operations".to_string(),
                "local_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "366".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "427".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-roc:foreign-company".to_string(),
            name: "Foreign Company Registration".to_string(),
            description: "Registration of a foreign company to conduct business in the Cayman Islands".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "foreign_business_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "732".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "854".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-roc:llc".to_string(),
            name: "Limited Liability Company Registration".to_string(),
            description: "Registration of a limited liability company (LLC) under the LLC Act".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "holding_company".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "732".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "854".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-roc:foundation-company".to_string(),
            name: "Foundation Company Registration".to_string(),
            description: "Registration of a foundation company under the Foundation Companies Act".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "foundation_operations".to_string(),
                "charitable_purposes".to_string(),
                "asset_holding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "732".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "854".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-roc:segregated-portfolio-company".to_string(),
            name: "Segregated Portfolio Company Registration".to_string(),
            description: "Registration of a segregated portfolio company (SPC) under the Companies Act".to_string(),
            regulator_id: "ky-roc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "segregated_portfolio_operations".to_string(),
                "fund_structuring".to_string(),
                "insurance_structuring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("KYD".to_string(), "2439".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("KYD".to_string(), "2927".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
    ]
}

// ── DITC — Department of International Tax Cooperation ──────────────────────

/// DITC regulator profile.
pub fn ditc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "ky-ditc".to_string(),
        name: "Department of International Tax Cooperation".to_string(),
        jurisdiction_id: "ky".to_string(),
        registry_url: Some("https://www.ditc.ky".to_string()),
        did: None,
        api_capabilities: vec!["compliance_status".to_string()],
    }
}

/// DITC license type definitions (tax compliance registrations).
pub fn ditc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "ky-ditc:crs-compliance".to_string(),
            name: "CRS Compliance Registration".to_string(),
            description: "Registration for Common Reporting Standard (CRS) compliance under the Tax Information Authority Act".to_string(),
            regulator_id: "ky-ditc".to_string(),
            category: Some("tax_compliance".to_string()),
            permitted_activities: vec![
                "crs_reporting".to_string(),
                "automatic_exchange_of_information".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "ky-ditc:fatca-compliance".to_string(),
            name: "FATCA Compliance Registration".to_string(),
            description: "Registration for Foreign Account Tax Compliance Act (FATCA) compliance under the Cayman-US IGA".to_string(),
            regulator_id: "ky-ditc".to_string(),
            category: Some("tax_compliance".to_string()),
            permitted_activities: vec![
                "fatca_reporting".to_string(),
                "us_tax_information_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Cayman Islands Registry Aggregation ─────────────────────────────────────

/// All Cayman Islands regulatory authorities.
pub fn cayman_regulators() -> Vec<LicensepackRegulator> {
    vec![
        cima_regulator(),
        roc_regulator(),
        ditc_regulator(),
    ]
}

/// All Cayman Islands license type definitions across all authorities.
pub fn cayman_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(cima_license_types());
    all.extend(roc_license_types());
    all.extend(ditc_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cayman_has_three_regulators() {
        let regs = cayman_regulators();
        assert_eq!(regs.len(), 3);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima"), "missing CIMA");
        assert!(ids.contains(&"ky-roc"), "missing ROC");
        assert!(ids.contains(&"ky-ditc"), "missing DITC");
    }

    #[test]
    fn all_regulators_are_cayman_jurisdiction() {
        for reg in cayman_regulators() {
            assert_eq!(reg.jurisdiction_id, "ky", "{} is not ky", reg.regulator_id);
        }
    }

    #[test]
    fn cayman_license_types_cover_all_authorities() {
        let types = cayman_license_types();
        assert!(
            types.len() >= 26,
            "expected >= 26 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("ky-cima"), "no CIMA license types");
        assert!(authority_ids.contains("ky-roc"), "no ROC license types");
        assert!(authority_ids.contains("ky-ditc"), "no DITC license types");
    }

    #[test]
    fn cima_has_banking_licenses() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:banking-class-a"));
        assert!(ids.contains(&"ky-cima:banking-class-b"));
    }

    #[test]
    fn cima_has_insurance_licenses() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:insurance-class-a"));
        assert!(ids.contains(&"ky-cima:insurance-class-b"));
        assert!(ids.contains(&"ky-cima:insurance-class-c"));
        assert!(ids.contains(&"ky-cima:insurance-manager"));
        assert!(ids.contains(&"ky-cima:insurance-broker"));
        assert!(ids.contains(&"ky-cima:insurance-agent"));
    }

    #[test]
    fn cima_has_securities_licenses() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:securities"));
        assert!(ids.contains(&"ky-cima:fund-administration"));
        assert!(ids.contains(&"ky-cima:mutual-fund"));
        assert!(ids.contains(&"ky-cima:securities-vasp"));
    }

    #[test]
    fn cima_has_money_services_licenses() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:money-transmission"));
        assert!(ids.contains(&"ky-cima:currency-exchange"));
    }

    #[test]
    fn cima_has_trust_and_corporate_licenses() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:trust-license"));
        assert!(ids.contains(&"ky-cima:company-management"));
    }

    #[test]
    fn cima_has_credit_union_license() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:cooperative-credit-union"));
    }

    #[test]
    fn cima_has_vasp_license() {
        let types = cima_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima:vasp"));
    }

    #[test]
    fn roc_has_corporate_registrations() {
        let types = roc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-roc:exempt-company"));
        assert!(ids.contains(&"ky-roc:ordinary-company"));
        assert!(ids.contains(&"ky-roc:foreign-company"));
        assert!(ids.contains(&"ky-roc:llc"));
        assert!(ids.contains(&"ky-roc:foundation-company"));
        assert!(ids.contains(&"ky-roc:segregated-portfolio-company"));
    }

    #[test]
    fn ditc_has_tax_compliance_registrations() {
        let types = ditc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-ditc:crs-compliance"));
        assert!(ids.contains(&"ky-ditc:fatca-compliance"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = cayman_license_types();
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
        for lt in cayman_license_types() {
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
        for lt in cayman_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in cayman_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in cayman_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
