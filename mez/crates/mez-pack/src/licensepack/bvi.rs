//! # British Virgin Islands Regulatory Authority License Mappings
//!
//! BVI-specific license type definitions covering the two major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **BVI FSC** | Financial Services Commission | Financial, Insurance, Investment, Digital Assets |
//! | **BVI RCA** | Registry of Corporate Affairs | Corporate Registration |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the BVI-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── BVI FSC — Financial Services Commission ─────────────────────────────────

/// BVI FSC regulator profile.
pub fn fsc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "vg-fsc".to_string(),
        name: "BVI Financial Services Commission".to_string(),
        jurisdiction_id: "vg".to_string(),
        registry_url: Some("https://www.bvifsc.vg".to_string()),
        did: None,
        api_capabilities: vec!["license_search".to_string(), "entity_registry".to_string()],
    }
}

/// BVI FSC license type definitions.
pub fn fsc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        // ── Banking ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:banking-class-i".to_string(),
            name: "Class I Banking License (Offshore)".to_string(),
            description: "Class I general banking license for offshore banking business under the Banks and Trust Companies Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "foreign_exchange".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "20000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:banking-class-ii".to_string(),
            name: "Class II Banking License (Restricted)".to_string(),
            description: "Class II restricted banking license for limited banking operations".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "restricted_deposit_taking".to_string(),
                "restricted_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── Insurance ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:insurance-general".to_string(),
            name: "General Insurance License".to_string(),
            description: "License to carry on general insurance business under the Insurance Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "property_insurance".to_string(),
                "casualty_insurance".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:insurance-long-term".to_string(),
            name: "Long-Term Insurance License".to_string(),
            description: "License to carry on long-term (life) insurance business".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "annuities".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:insurance-manager".to_string(),
            name: "Insurance Manager License".to_string(),
            description: "License to act as an insurance manager in the BVI".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_management".to_string(),
                "captive_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to act as an insurance broker".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "insurance_placement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:insurance-agent".to_string(),
            name: "Insurance Agent License".to_string(),
            description: "License to act as an insurance agent".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_solicitation".to_string(),
                "insurance_sales".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Investment Business ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:fund-manager".to_string(),
            name: "Fund Manager License".to_string(),
            description: "License to manage investment funds under the Securities and Investment Business Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "portfolio_management".to_string(),
                "asset_allocation".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:investment-adviser".to_string(),
            name: "Investment Adviser License".to_string(),
            description: "License to provide investment advisory services".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_planning".to_string(),
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
            license_type_id: "vg-fsc:broker-dealer".to_string(),
            name: "Broker-Dealer License".to_string(),
            description: "License to operate as a broker-dealer in securities".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "securities_dealing".to_string(),
                "trading".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:exchange".to_string(),
            name: "Exchange License".to_string(),
            description: "License to operate a securities or commodities exchange".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "exchange_operations".to_string(),
                "order_matching".to_string(),
                "market_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        // ── Financing & Money Services ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:money-services-business".to_string(),
            name: "Money Services Business License".to_string(),
            description: "License to operate a money services business (MSB) including remittance and money transmission".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_transmission".to_string(),
                "remittance".to_string(),
                "currency_exchange".to_string(),
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
            license_type_id: "vg-fsc:financing-business".to_string(),
            name: "Financing Business License".to_string(),
            description: "License to carry on financing business including lending and credit".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "lending".to_string(),
                "credit_facilities".to_string(),
                "financing".to_string(),
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
        // ── Mutual Funds ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:mutual-fund-professional".to_string(),
            name: "Professional Mutual Fund".to_string(),
            description: "Recognition as a professional mutual fund (minimum initial investment USD 100,000)".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "collective_investment".to_string(),
                "professional_fund_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:mutual-fund-private".to_string(),
            name: "Private Mutual Fund".to_string(),
            description: "Recognition as a private mutual fund (max 50 investors)".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "collective_investment".to_string(),
                "private_fund_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:mutual-fund-public".to_string(),
            name: "Public Mutual Fund".to_string(),
            description: "Registration of a public mutual fund".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "collective_investment".to_string(),
                "public_fund_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "2500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:mutual-fund-recognized-foreign".to_string(),
            name: "Recognized Foreign Mutual Fund".to_string(),
            description: "Recognition of a foreign mutual fund for distribution in the BVI".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "collective_investment".to_string(),
                "foreign_fund_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── Company Management ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:csp".to_string(),
            name: "Company Service Provider License".to_string(),
            description: "License to provide company management and registered agent services".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "company_formation".to_string(),
                "registered_agent_services".to_string(),
                "corporate_administration".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:trust-business".to_string(),
            name: "Trust Business License".to_string(),
            description: "License to carry on trust business under the Banks and Trust Companies Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "trust_administration".to_string(),
                "trust_services".to_string(),
                "fiduciary_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        // ── Credit Union ──
        LicenseTypeDefinition {
            license_type_id: "vg-fsc:credit-union".to_string(),
            name: "Credit Union License".to_string(),
            description: "License to operate a credit union under the Co-operative Societies Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
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
            license_type_id: "vg-fsc:vasp".to_string(),
            name: "Virtual Asset Service Provider (VASP) License".to_string(),
            description: "License to operate as a virtual asset service provider under the Virtual Assets Service Providers Act".to_string(),
            regulator_id: "vg-fsc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_exchange".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_transfer".to_string(),
                "virtual_asset_issuance".to_string(),
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

// ── BVI RCA — Registry of Corporate Affairs ─────────────────────────────────

/// BVI Registry of Corporate Affairs regulator profile.
pub fn rca_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "vg-rca".to_string(),
        name: "BVI Registry of Corporate Affairs".to_string(),
        jurisdiction_id: "vg".to_string(),
        registry_url: Some("https://www.bvi.gov.vg/registry-corporate-affairs".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string()],
    }
}

/// BVI RCA license type definitions (corporate registrations).
pub fn rca_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "vg-rca:business-company".to_string(),
            name: "BVI Business Company (BC) Registration".to_string(),
            description: "Registration of a BVI Business Company under the BVI Business Companies Act".to_string(),
            regulator_id: "vg-rca".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "holding_company".to_string(),
                "trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "450".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "450".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-rca:limited-partnership".to_string(),
            name: "Limited Partnership Registration".to_string(),
            description: "Registration of a limited partnership under the Limited Partnership Act".to_string(),
            regulator_id: "vg-rca".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "partnership_operations".to_string(),
                "investment_holding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "250".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "250".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-rca:segregated-portfolio-company".to_string(),
            name: "Segregated Portfolio Company Registration".to_string(),
            description: "Registration of a segregated portfolio company (SPC) under the BVI Business Companies Act".to_string(),
            regulator_id: "vg-rca".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "segregated_portfolio_operations".to_string(),
                "fund_structuring".to_string(),
                "insurance_structuring".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "650".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "vg-rca:foreign-company".to_string(),
            name: "Foreign Company Registration".to_string(),
            description: "Registration of a foreign company to conduct business in the BVI".to_string(),
            regulator_id: "vg-rca".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "foreign_business_operations".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("USD".to_string(), "450".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "450".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
    ]
}

// ── BVI Registry Aggregation ────────────────────────────────────────────────

/// All BVI regulatory authorities.
pub fn bvi_regulators() -> Vec<LicensepackRegulator> {
    vec![
        fsc_regulator(),
        rca_regulator(),
    ]
}

/// All BVI license type definitions across all authorities.
pub fn bvi_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(fsc_license_types());
    all.extend(rca_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bvi_has_two_regulators() {
        let regs = bvi_regulators();
        assert_eq!(regs.len(), 2);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc"), "missing FSC");
        assert!(ids.contains(&"vg-rca"), "missing RCA");
    }

    #[test]
    fn all_regulators_are_bvi_jurisdiction() {
        for reg in bvi_regulators() {
            assert_eq!(reg.jurisdiction_id, "vg", "{} is not vg", reg.regulator_id);
        }
    }

    #[test]
    fn bvi_license_types_cover_all_authorities() {
        let types = bvi_license_types();
        assert!(
            types.len() >= 25,
            "expected >= 25 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("vg-fsc"), "no FSC license types");
        assert!(authority_ids.contains("vg-rca"), "no RCA license types");
    }

    #[test]
    fn fsc_has_banking_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:banking-class-i"));
        assert!(ids.contains(&"vg-fsc:banking-class-ii"));
    }

    #[test]
    fn fsc_has_insurance_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:insurance-general"));
        assert!(ids.contains(&"vg-fsc:insurance-long-term"));
        assert!(ids.contains(&"vg-fsc:insurance-manager"));
        assert!(ids.contains(&"vg-fsc:insurance-broker"));
        assert!(ids.contains(&"vg-fsc:insurance-agent"));
    }

    #[test]
    fn fsc_has_investment_business_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:fund-manager"));
        assert!(ids.contains(&"vg-fsc:investment-adviser"));
        assert!(ids.contains(&"vg-fsc:broker-dealer"));
        assert!(ids.contains(&"vg-fsc:exchange"));
    }

    #[test]
    fn fsc_has_money_services_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:money-services-business"));
        assert!(ids.contains(&"vg-fsc:financing-business"));
    }

    #[test]
    fn fsc_has_mutual_fund_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:mutual-fund-professional"));
        assert!(ids.contains(&"vg-fsc:mutual-fund-private"));
        assert!(ids.contains(&"vg-fsc:mutual-fund-public"));
        assert!(ids.contains(&"vg-fsc:mutual-fund-recognized-foreign"));
    }

    #[test]
    fn fsc_has_company_management_licenses() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:csp"));
        assert!(ids.contains(&"vg-fsc:trust-business"));
    }

    #[test]
    fn fsc_has_credit_union_license() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:credit-union"));
    }

    #[test]
    fn fsc_has_digital_assets_license() {
        let types = fsc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-fsc:vasp"));
    }

    #[test]
    fn rca_has_corporate_registrations() {
        let types = rca_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"vg-rca:business-company"));
        assert!(ids.contains(&"vg-rca:limited-partnership"));
        assert!(ids.contains(&"vg-rca:segregated-portfolio-company"));
        assert!(ids.contains(&"vg-rca:foreign-company"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = bvi_license_types();
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
        for lt in bvi_license_types() {
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
        for lt in bvi_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in bvi_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in bvi_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
