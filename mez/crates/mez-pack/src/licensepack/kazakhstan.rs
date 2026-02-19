//! # Kazakhstan + AIFC + Alatau City Regulatory Authority License Mappings
//!
//! Kazakhstan-specific license type definitions covering onshore regulators,
//! the Astana International Financial Centre (AIFC), and the Alatau City
//! technology zone:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **ARDFM** | Agency for Regulation and Development of Financial Market | Financial |
//! | **NB RK** | National Bank of Kazakhstan | Monetary Policy, Payments |
//! | **AFSA** | Astana Financial Services Authority | Financial Services (AIFC) |
//! | **AIFC Registrar** | AIFC Registrar of Companies | Corporate (AIFC) |
//! | **Alatau City Admin** | Alatau City Administration | Technology, Innovation |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Kazakhstan-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── ARDFM — Agency for Regulation and Development of Financial Market ────────

/// ARDFM regulator profile.
pub fn ardfm_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "kz-ardfm".to_string(),
        name: "Agency for Regulation and Development of Financial Market".to_string(),
        jurisdiction_id: "kz".to_string(),
        registry_url: Some("https://www.gov.kz/memleket/entities/ardfm".to_string()),
        did: None,
        api_capabilities: vec!["license_registry".to_string()],
    }
}

/// ARDFM license type definitions.
pub fn ardfm_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "kz-ardfm:banking".to_string(),
            name: "Banking License".to_string(),
            description: "License to conduct banking operations in the Republic of Kazakhstan"
                .to_string(),
            regulator_id: "kz-ardfm".to_string(),
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
            license_type_id: "kz-ardfm:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to conduct insurance business in Kazakhstan".to_string(),
            regulator_id: "kz-ardfm".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
                "reinsurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-ardfm:securities".to_string(),
            name: "Securities Market License".to_string(),
            description:
                "License to operate on the securities market of Kazakhstan".to_string(),
            regulator_id: "kz-ardfm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "securities_dealing".to_string(),
                "investment_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-ardfm:microfinance".to_string(),
            name: "Microfinance Organization License".to_string(),
            description: "License to operate as a microfinance organization in Kazakhstan"
                .to_string(),
            regulator_id: "kz-ardfm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "micro_lending".to_string(),
                "micro_credit".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-ardfm:payment".to_string(),
            name: "Payment Organization License".to_string(),
            description: "License to provide payment services in Kazakhstan".to_string(),
            regulator_id: "kz-ardfm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_processing".to_string(),
                "payment_services".to_string(),
                "e_money".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── NB RK — National Bank of Kazakhstan ──────────────────────────────────────

/// NB RK regulator profile.
pub fn nbrk_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "kz-nb".to_string(),
        name: "National Bank of Kazakhstan".to_string(),
        jurisdiction_id: "kz".to_string(),
        registry_url: Some("https://www.nationalbank.kz".to_string()),
        did: None,
        api_capabilities: vec!["monetary_data".to_string(), "payment_oversight".to_string()],
    }
}

/// NB RK license type definitions.
pub fn nbrk_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "kz-nb:monetary-policy".to_string(),
            name: "Monetary Policy Participant".to_string(),
            description: "Authorization to participate in National Bank monetary operations"
                .to_string(),
            regulator_id: "kz-nb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "open_market_operations".to_string(),
                "reserve_requirements".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "kz-nb:payment-system-oversight".to_string(),
            name: "Payment System Operator Authorization".to_string(),
            description: "Authorization to operate a payment system under NB RK oversight"
                .to_string(),
            regulator_id: "kz-nb".to_string(),
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
    ]
}

// ── AFSA — Astana Financial Services Authority ───────────────────────────────

/// AFSA regulator profile.
pub fn afsa_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "kz-aifc-afsa".to_string(),
        name: "Astana Financial Services Authority".to_string(),
        jurisdiction_id: "kz-aifc".to_string(),
        registry_url: Some("https://www.afsa.kz".to_string()),
        did: None,
        api_capabilities: vec![
            "firm_directory".to_string(),
            "license_status".to_string(),
        ],
    }
}

/// AFSA license type definitions.
pub fn afsa_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:banking-business".to_string(),
            name: "Banking Business License".to_string(),
            description: "License to conduct banking business within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:insurance".to_string(),
            name: "Insurance License".to_string(),
            description: "License to conduct insurance business within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
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
            license_type_id: "kz-aifc-afsa:islamic-finance".to_string(),
            name: "Islamic Finance License".to_string(),
            description: "License to conduct Islamic finance activities within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
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
            license_type_id: "kz-aifc-afsa:dealing-in-investments".to_string(),
            name: "Dealing in Investments License".to_string(),
            description: "License to deal in investments as principal or agent within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "proprietary_trading".to_string(),
                "order_execution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:managing-investments".to_string(),
            name: "Managing Investments License".to_string(),
            description: "License to manage investments within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "portfolio_management".to_string(),
                "discretionary_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:advising-on-investments".to_string(),
            name: "Advising on Investments License".to_string(),
            description: "License to provide investment advisory services within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
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
            license_type_id: "kz-aifc-afsa:custody".to_string(),
            name: "Custody License".to_string(),
            description: "License to provide custody services within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "custody_services".to_string(),
                "safekeeping".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:fund-administration".to_string(),
            name: "Fund Administration License".to_string(),
            description: "License to provide fund administration services within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
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
            license_type_id: "kz-aifc-afsa:credit-rating".to_string(),
            name: "Credit Rating Agency License".to_string(),
            description: "License to operate as a credit rating agency within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:digital-asset-trading-facility".to_string(),
            name: "Digital Asset Trading Facility License".to_string(),
            description: "License to operate a digital asset trading facility within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_exchange".to_string(),
                "digital_asset_custody".to_string(),
                "order_matching".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:money-services".to_string(),
            name: "Providing Money Services License".to_string(),
            description: "License to provide money services within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_transmission".to_string(),
                "remittance".to_string(),
                "currency_exchange".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:e-money".to_string(),
            name: "E-Money License".to_string(),
            description: "License to issue electronic money within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "e_money_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:crowdfunding-investment".to_string(),
            name: "Investment Crowdfunding License".to_string(),
            description: "License to operate an investment-based crowdfunding platform in AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:crowdfunding-loan".to_string(),
            name: "Loan Crowdfunding License".to_string(),
            description: "License to operate a loan-based crowdfunding platform in AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "crowdfunding_platform".to_string(),
                "peer_to_peer_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:crowdfunding-property".to_string(),
            name: "Property Crowdfunding License".to_string(),
            description: "License to operate a property crowdfunding platform in AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "crowdfunding_platform".to_string(),
                "property_crowdfunding".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:insurance-management".to_string(),
            name: "Insurance Management License".to_string(),
            description: "License to provide insurance management services within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_management".to_string(),
                "claims_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:insurance-broker".to_string(),
            name: "Insurance Broker License".to_string(),
            description: "License to operate as an insurance broker within the AIFC".to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_brokerage".to_string(),
                "risk_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-afsa:private-financing-platform".to_string(),
            name: "Private Financing Platform License".to_string(),
            description: "License to operate a private financing platform within the AIFC"
                .to_string(),
            regulator_id: "kz-aifc-afsa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "private_placement".to_string(),
                "capital_raising".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── AIFC Registrar of Companies ──────────────────────────────────────────────

/// AIFC Registrar of Companies regulator profile.
pub fn aifc_registrar_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "kz-aifc-rc".to_string(),
        name: "AIFC Registrar of Companies".to_string(),
        jurisdiction_id: "kz-aifc".to_string(),
        registry_url: Some("https://services.aifc.kz".to_string()),
        did: None,
        api_capabilities: vec![
            "company_search".to_string(),
            "registration_status".to_string(),
        ],
    }
}

/// AIFC Registrar license type definitions.
pub fn aifc_registrar_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-rc:aifc-company".to_string(),
            name: "AIFC Company Registration".to_string(),
            description: "Registration of a company within the AIFC".to_string(),
            regulator_id: "kz-aifc-rc".to_string(),
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
            license_type_id: "kz-aifc-rc:aifc-partnership".to_string(),
            name: "AIFC Partnership Registration".to_string(),
            description: "Registration of a partnership within the AIFC".to_string(),
            regulator_id: "kz-aifc-rc".to_string(),
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
            license_type_id: "kz-aifc-rc:aifc-foundation".to_string(),
            name: "AIFC Foundation Registration".to_string(),
            description: "Registration of a foundation within the AIFC".to_string(),
            regulator_id: "kz-aifc-rc".to_string(),
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
        LicenseTypeDefinition {
            license_type_id: "kz-aifc-rc:aifc-spv".to_string(),
            name: "AIFC SPV Registration".to_string(),
            description: "Registration of a special purpose vehicle within the AIFC".to_string(),
            regulator_id: "kz-aifc-rc".to_string(),
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
            license_type_id: "kz-aifc-rc:foreign-company".to_string(),
            name: "Foreign Company Registration".to_string(),
            description: "Registration of a foreign company branch within the AIFC".to_string(),
            regulator_id: "kz-aifc-rc".to_string(),
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
    ]
}

// ── Alatau City Administration ───────────────────────────────────────────────

/// Alatau City Administration regulator profile.
pub fn alatau_admin_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "kz-alatau-admin".to_string(),
        name: "Alatau City Administration".to_string(),
        jurisdiction_id: "kz-alatau".to_string(),
        registry_url: Some("https://alataucity.kz".to_string()),
        did: None,
        api_capabilities: vec!["participant_registry".to_string()],
    }
}

/// Alatau City license type definitions.
pub fn alatau_admin_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "kz-alatau-admin:tech-park-participant".to_string(),
            name: "Technology Park Participant".to_string(),
            description: "Registration as a technology park participant in Alatau City"
                .to_string(),
            regulator_id: "kz-alatau-admin".to_string(),
            category: Some("technology".to_string()),
            permitted_activities: vec![
                "technology_development".to_string(),
                "software_development".to_string(),
                "research".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(5),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-alatau-admin:innovation-license".to_string(),
            name: "Innovation License".to_string(),
            description: "License for innovative activities within Alatau City".to_string(),
            regulator_id: "kz-alatau-admin".to_string(),
            category: Some("technology".to_string()),
            permitted_activities: vec![
                "innovation_projects".to_string(),
                "prototyping".to_string(),
                "commercialization".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
        LicenseTypeDefinition {
            license_type_id: "kz-alatau-admin:it-services".to_string(),
            name: "IT Services License".to_string(),
            description: "License to provide IT services within Alatau City".to_string(),
            regulator_id: "kz-alatau-admin".to_string(),
            category: Some("technology".to_string()),
            permitted_activities: vec![
                "it_outsourcing".to_string(),
                "data_processing".to_string(),
                "cloud_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── Kazakhstan Registry Aggregation ──────────────────────────────────────────

/// All Kazakhstan, AIFC, and Alatau City regulatory authorities.
pub fn kazakhstan_regulators() -> Vec<LicensepackRegulator> {
    vec![
        ardfm_regulator(),
        nbrk_regulator(),
        afsa_regulator(),
        aifc_registrar_regulator(),
        alatau_admin_regulator(),
    ]
}

/// All Kazakhstan, AIFC, and Alatau City license type definitions across all authorities.
pub fn kazakhstan_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(ardfm_license_types());
    all.extend(nbrk_license_types());
    all.extend(afsa_license_types());
    all.extend(aifc_registrar_license_types());
    all.extend(alatau_admin_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kazakhstan_has_five_regulators() {
        let regs = kazakhstan_regulators();
        assert_eq!(regs.len(), 5);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"kz-ardfm"), "missing ARDFM");
        assert!(ids.contains(&"kz-nb"), "missing NB RK");
        assert!(ids.contains(&"kz-aifc-afsa"), "missing AFSA");
        assert!(ids.contains(&"kz-aifc-rc"), "missing AIFC Registrar");
        assert!(ids.contains(&"kz-alatau-admin"), "missing Alatau City Admin");
    }

    #[test]
    fn kazakhstan_regulators_have_correct_jurisdictions() {
        let regs = kazakhstan_regulators();
        for reg in &regs {
            match reg.regulator_id.as_str() {
                "kz-ardfm" | "kz-nb" => {
                    assert_eq!(reg.jurisdiction_id, "kz", "{} is not kz", reg.regulator_id);
                }
                "kz-aifc-afsa" | "kz-aifc-rc" => {
                    assert_eq!(
                        reg.jurisdiction_id, "kz-aifc",
                        "{} is not kz-aifc",
                        reg.regulator_id
                    );
                }
                "kz-alatau-admin" => {
                    assert_eq!(
                        reg.jurisdiction_id, "kz-alatau",
                        "{} is not kz-alatau",
                        reg.regulator_id
                    );
                }
                _ => panic!("unexpected regulator_id: {}", reg.regulator_id),
            }
        }
    }

    #[test]
    fn kazakhstan_license_types_cover_all_authorities() {
        let types = kazakhstan_license_types();
        assert!(
            types.len() >= 33,
            "expected >= 33 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("kz-ardfm"), "no ARDFM license types");
        assert!(authority_ids.contains("kz-nb"), "no NB RK license types");
        assert!(
            authority_ids.contains("kz-aifc-afsa"),
            "no AFSA license types"
        );
        assert!(
            authority_ids.contains("kz-aifc-rc"),
            "no AIFC Registrar license types"
        );
        assert!(
            authority_ids.contains("kz-alatau-admin"),
            "no Alatau City license types"
        );
    }

    #[test]
    fn ardfm_has_financial_licenses() {
        let types = ardfm_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"kz-ardfm:banking"));
        assert!(ids.contains(&"kz-ardfm:insurance"));
        assert!(ids.contains(&"kz-ardfm:securities"));
        assert!(ids.contains(&"kz-ardfm:microfinance"));
        assert!(ids.contains(&"kz-ardfm:payment"));
    }

    #[test]
    fn afsa_has_aifc_licenses() {
        let types = afsa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"kz-aifc-afsa:banking-business"));
        assert!(ids.contains(&"kz-aifc-afsa:digital-asset-trading-facility"));
        assert!(ids.contains(&"kz-aifc-afsa:crowdfunding-investment"));
        assert!(ids.contains(&"kz-aifc-afsa:e-money"));
        assert!(ids.contains(&"kz-aifc-afsa:insurance-broker"));
    }

    #[test]
    fn aifc_registrar_has_entity_registrations() {
        let types = aifc_registrar_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"kz-aifc-rc:aifc-company"));
        assert!(ids.contains(&"kz-aifc-rc:aifc-partnership"));
        assert!(ids.contains(&"kz-aifc-rc:aifc-foundation"));
        assert!(ids.contains(&"kz-aifc-rc:aifc-spv"));
        assert!(ids.contains(&"kz-aifc-rc:foreign-company"));
    }

    #[test]
    fn alatau_has_technology_licenses() {
        let types = alatau_admin_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"kz-alatau-admin:tech-park-participant"));
        assert!(ids.contains(&"kz-alatau-admin:innovation-license"));
        assert!(ids.contains(&"kz-alatau-admin:it-services"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = kazakhstan_license_types();
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
        for lt in kazakhstan_license_types() {
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
        for lt in kazakhstan_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in kazakhstan_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in kazakhstan_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
