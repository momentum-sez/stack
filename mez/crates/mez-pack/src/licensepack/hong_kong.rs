//! # Hong Kong SAR Regulatory Authority License Mappings
//!
//! Hong Kong SAR-specific license type definitions covering the major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **SFC** | Securities and Futures Commission | Securities, Asset Management, VA Trading |
//! | **HKMA** | Hong Kong Monetary Authority | Banking, SVF, MSO |
//! | **IA** | Insurance Authority | Insurance |
//! | **MPFA** | Mandatory Provident Fund Authority | MPF Trustees |
//! | **CR** | Companies Registry | Company Registration, TCSP |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Hong Kong-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── SFC — Securities and Futures Commission ───────────────────────────────

/// SFC regulator profile.
pub fn sfc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hk-sfc".to_string(),
        name: "Securities and Futures Commission".to_string(),
        jurisdiction_id: "hk".to_string(),
        registry_url: Some("https://www.sfc.hk".to_string()),
        did: None,
        api_capabilities: vec![
            "licensed_persons_register".to_string(),
            "public_register_search".to_string(),
        ],
    }
}

/// SFC license type definitions (Types 1-10 + VA trading platform).
pub fn sfc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-1-dealing-securities".to_string(),
            name: "Type 1 — Dealing in Securities".to_string(),
            description: "SFC license for dealing in securities under the Securities and Futures Ordinance"
                .to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_dealing".to_string(),
                "securities_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-2-dealing-futures".to_string(),
            name: "Type 2 — Dealing in Futures Contracts".to_string(),
            description: "SFC license for dealing in futures contracts".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "futures_dealing".to_string(),
                "futures_distribution".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-3-leveraged-forex".to_string(),
            name: "Type 3 — Leveraged Foreign Exchange Trading".to_string(),
            description: "SFC license for leveraged foreign exchange trading".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "leveraged_forex_trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-4-advising-securities".to_string(),
            name: "Type 4 — Advising on Securities".to_string(),
            description: "SFC license for advising on securities".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_advisory".to_string(),
                "research_analysis".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-5-advising-futures".to_string(),
            name: "Type 5 — Advising on Futures Contracts".to_string(),
            description: "SFC license for advising on futures contracts".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "futures_advisory".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-6-corporate-finance".to_string(),
            name: "Type 6 — Advising on Corporate Finance".to_string(),
            description: "SFC license for advising on corporate finance".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "corporate_finance_advisory".to_string(),
                "sponsor_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-7-automated-trading".to_string(),
            name: "Type 7 — Providing Automated Trading Services".to_string(),
            description: "SFC license for providing automated trading services".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "automated_trading_services".to_string(),
                "dark_pool_operation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-8-securities-margin".to_string(),
            name: "Type 8 — Securities Margin Financing".to_string(),
            description: "SFC license for securities margin financing".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_margin_financing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-9-asset-management".to_string(),
            name: "Type 9 — Asset Management".to_string(),
            description: "SFC license for asset management (fund management)".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "discretionary_account_management".to_string(),
                "reit_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:type-10-credit-rating".to_string(),
            name: "Type 10 — Providing Credit Rating Services".to_string(),
            description: "SFC license for providing credit rating services".to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "credit_rating".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "5260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "4650".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-sfc:va-trading-platform".to_string(),
            name: "Virtual Asset Trading Platform License".to_string(),
            description:
                "SFC license to operate a virtual asset trading platform under the AMLO regime"
                    .to_string(),
            regulator_id: "hk-sfc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "virtual_asset_trading".to_string(),
                "virtual_asset_custody".to_string(),
                "virtual_asset_matching".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(1),
        },
    ]
}

// ── HKMA — Hong Kong Monetary Authority ───────────────────────────────────

/// HKMA regulator profile.
pub fn hkma_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hk-hkma".to_string(),
        name: "Hong Kong Monetary Authority".to_string(),
        jurisdiction_id: "hk".to_string(),
        registry_url: Some("https://www.hkma.gov.hk".to_string()),
        did: None,
        api_capabilities: vec![
            "authorized_institutions_register".to_string(),
            "svf_licensee_register".to_string(),
        ],
    }
}

/// HKMA license type definitions.
pub fn hkma_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hk-hkma:licensed-bank".to_string(),
            name: "Licensed Bank".to_string(),
            description: "Authorization as a licensed bank under the Banking Ordinance".to_string(),
            regulator_id: "hk-hkma".to_string(),
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
            license_type_id: "hk-hkma:restricted-license-bank".to_string(),
            name: "Restricted License Bank".to_string(),
            description:
                "Authorization as a restricted license bank (deposits >= HKD 500,000)"
                    .to_string(),
            regulator_id: "hk-hkma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "restricted_deposit_taking".to_string(),
                "lending".to_string(),
                "trade_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-hkma:deposit-taking-company".to_string(),
            name: "Deposit-Taking Company".to_string(),
            description:
                "Authorization as a deposit-taking company under the Banking Ordinance"
                    .to_string(),
            regulator_id: "hk-hkma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "limited_deposit_taking".to_string(),
                "consumer_finance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-hkma:svf".to_string(),
            name: "Stored Value Facility License".to_string(),
            description: "License to issue stored value facilities under the Payment Systems and Stored Value Facilities Ordinance"
                .to_string(),
            regulator_id: "hk-hkma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "stored_value_facility_issuance".to_string(),
                "e_money_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(10),
        },
        LicenseTypeDefinition {
            license_type_id: "hk-hkma:mso".to_string(),
            name: "Money Service Operator License".to_string(),
            description: "License to operate money service under the Anti-Money Laundering and Counter-Terrorist Financing Ordinance"
                .to_string(),
            regulator_id: "hk-hkma".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "money_changing".to_string(),
                "remittance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "2770".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "570".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(2),
        },
    ]
}

// ── IA — Insurance Authority ──────────────────────────────────────────────

/// IA regulator profile.
pub fn ia_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hk-ia".to_string(),
        name: "Insurance Authority".to_string(),
        jurisdiction_id: "hk".to_string(),
        registry_url: Some("https://www.ia.org.hk".to_string()),
        did: None,
        api_capabilities: vec!["insurer_register".to_string(), "intermediary_register".to_string()],
    }
}

/// IA license type definitions.
pub fn ia_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hk-ia:insurer-long-term".to_string(),
            name: "Insurance Company Authorization — Long Term".to_string(),
            description: "Authorization to carry on long term (life) insurance business"
                .to_string(),
            regulator_id: "hk-ia".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "life_insurance".to_string(),
                "annuity_products".to_string(),
                "linked_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-ia:insurer-general".to_string(),
            name: "Insurance Company Authorization — General".to_string(),
            description: "Authorization to carry on general (non-life) insurance business"
                .to_string(),
            regulator_id: "hk-ia".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "property_insurance".to_string(),
                "liability_insurance".to_string(),
                "marine_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-ia:insurer-composite".to_string(),
            name: "Insurance Company Authorization — Composite".to_string(),
            description: "Authorization to carry on both long term and general insurance business"
                .to_string(),
            regulator_id: "hk-ia".to_string(),
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
            license_type_id: "hk-ia:insurance-broker".to_string(),
            name: "Licensed Insurance Broker".to_string(),
            description: "License to act as an insurance broker in Hong Kong".to_string(),
            regulator_id: "hk-ia".to_string(),
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
            license_type_id: "hk-ia:insurance-agent".to_string(),
            name: "Licensed Insurance Agent".to_string(),
            description: "License to act as an insurance agent in Hong Kong".to_string(),
            regulator_id: "hk-ia".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "insurance_agency".to_string(),
                "policy_solicitation".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── MPFA — Mandatory Provident Fund Authority ─────────────────────────────

/// MPFA regulator profile.
pub fn mpfa_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hk-mpfa".to_string(),
        name: "Mandatory Provident Fund Schemes Authority".to_string(),
        jurisdiction_id: "hk".to_string(),
        registry_url: Some("https://www.mpfa.org.hk".to_string()),
        did: None,
        api_capabilities: vec!["trustee_register".to_string()],
    }
}

/// MPFA license type definitions.
pub fn mpfa_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hk-mpfa:mpf-trustee".to_string(),
            name: "MPF Approved Trustee".to_string(),
            description:
                "Approval to act as a trustee of MPF schemes under the MPF Schemes Ordinance"
                    .to_string(),
            regulator_id: "hk-mpfa".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "mpf_trusteeship".to_string(),
                "mpf_scheme_administration".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── CR — Companies Registry ───────────────────────────────────────────────

/// Companies Registry regulator profile.
pub fn cr_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hk-cr".to_string(),
        name: "Companies Registry".to_string(),
        jurisdiction_id: "hk".to_string(),
        registry_url: Some("https://www.cr.gov.hk".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string(), "tcsp_register".to_string()],
    }
}

/// Companies Registry license type definitions.
pub fn cr_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hk-cr:private-limited-company".to_string(),
            name: "Private Limited Company Registration".to_string(),
            description: "Registration of a private company limited by shares under the Companies Ordinance"
                .to_string(),
            regulator_id: "hk-cr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "capital_raising_private".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "1720".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "105".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-cr:public-company".to_string(),
            name: "Public Company Registration".to_string(),
            description: "Registration of a public company under the Companies Ordinance"
                .to_string(),
            regulator_id: "hk-cr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "public_capital_raising".to_string(),
                "shares_listing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "1720".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "105".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-cr:non-hk-company".to_string(),
            name: "Non-Hong Kong Company Registration".to_string(),
            description: "Registration of a non-Hong Kong company establishing a place of business in HK"
                .to_string(),
            regulator_id: "hk-cr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "branch_operations".to_string(),
                "representative_office".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "1720".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "105".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hk-cr:tcsp".to_string(),
            name: "Trust or Company Service Provider License".to_string(),
            description: "License to carry on trust or company service provider business under AMLO"
                .to_string(),
            regulator_id: "hk-cr".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "company_formation".to_string(),
                "registered_office_services".to_string(),
                "directorship_services".to_string(),
                "trust_services".to_string(),
                "nominee_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: [("HKD".to_string(), "2340".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("HKD".to_string(), "2340".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(3),
        },
    ]
}

// ── Hong Kong Registry Aggregation ────────────────────────────────────────

/// All Hong Kong SAR regulatory authorities.
pub fn hong_kong_regulators() -> Vec<LicensepackRegulator> {
    vec![
        sfc_regulator(),
        hkma_regulator(),
        ia_regulator(),
        mpfa_regulator(),
        cr_regulator(),
    ]
}

/// All Hong Kong SAR license type definitions across all authorities.
pub fn hong_kong_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(sfc_license_types());
    all.extend(hkma_license_types());
    all.extend(ia_license_types());
    all.extend(mpfa_license_types());
    all.extend(cr_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hong_kong_has_five_regulators() {
        let regs = hong_kong_regulators();
        assert_eq!(regs.len(), 5);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"hk-sfc"), "missing SFC");
        assert!(ids.contains(&"hk-hkma"), "missing HKMA");
        assert!(ids.contains(&"hk-ia"), "missing IA");
        assert!(ids.contains(&"hk-mpfa"), "missing MPFA");
        assert!(ids.contains(&"hk-cr"), "missing CR");
    }

    #[test]
    fn all_regulators_are_hk_jurisdiction() {
        for reg in hong_kong_regulators() {
            assert_eq!(reg.jurisdiction_id, "hk", "{} is not hk", reg.regulator_id);
        }
    }

    #[test]
    fn hong_kong_license_types_cover_all_authorities() {
        let types = hong_kong_license_types();
        assert!(
            types.len() >= 26,
            "expected >= 26 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("hk-sfc"), "no SFC license types");
        assert!(authority_ids.contains("hk-hkma"), "no HKMA license types");
        assert!(authority_ids.contains("hk-ia"), "no IA license types");
        assert!(authority_ids.contains("hk-mpfa"), "no MPFA license types");
        assert!(authority_ids.contains("hk-cr"), "no CR license types");
    }

    #[test]
    fn sfc_has_types_1_through_10_and_va() {
        let types = sfc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-sfc:type-1-dealing-securities"));
        assert!(ids.contains(&"hk-sfc:type-2-dealing-futures"));
        assert!(ids.contains(&"hk-sfc:type-3-leveraged-forex"));
        assert!(ids.contains(&"hk-sfc:type-4-advising-securities"));
        assert!(ids.contains(&"hk-sfc:type-5-advising-futures"));
        assert!(ids.contains(&"hk-sfc:type-6-corporate-finance"));
        assert!(ids.contains(&"hk-sfc:type-7-automated-trading"));
        assert!(ids.contains(&"hk-sfc:type-8-securities-margin"));
        assert!(ids.contains(&"hk-sfc:type-9-asset-management"));
        assert!(ids.contains(&"hk-sfc:type-10-credit-rating"));
        assert!(ids.contains(&"hk-sfc:va-trading-platform"));
        assert_eq!(types.len(), 11, "SFC should have 11 license types");
    }

    #[test]
    fn hkma_has_banking_svf_and_mso() {
        let types = hkma_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-hkma:licensed-bank"));
        assert!(ids.contains(&"hk-hkma:restricted-license-bank"));
        assert!(ids.contains(&"hk-hkma:deposit-taking-company"));
        assert!(ids.contains(&"hk-hkma:svf"));
        assert!(ids.contains(&"hk-hkma:mso"));
    }

    #[test]
    fn ia_has_insurer_and_intermediary_licenses() {
        let types = ia_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-ia:insurer-long-term"));
        assert!(ids.contains(&"hk-ia:insurer-general"));
        assert!(ids.contains(&"hk-ia:insurer-composite"));
        assert!(ids.contains(&"hk-ia:insurance-broker"));
        assert!(ids.contains(&"hk-ia:insurance-agent"));
    }

    #[test]
    fn mpfa_has_trustee_license() {
        let types = mpfa_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-mpfa:mpf-trustee"));
    }

    #[test]
    fn cr_has_company_and_tcsp_licenses() {
        let types = cr_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-cr:private-limited-company"));
        assert!(ids.contains(&"hk-cr:public-company"));
        assert!(ids.contains(&"hk-cr:non-hk-company"));
        assert!(ids.contains(&"hk-cr:tcsp"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = hong_kong_license_types();
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
        for lt in hong_kong_license_types() {
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
        for lt in hong_kong_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in hong_kong_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in hong_kong_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
