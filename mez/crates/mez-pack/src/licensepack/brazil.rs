//! # Brazil Regulatory Authority License Mappings
//!
//! Brazil-specific license type definitions covering the four major
//! regulatory authorities:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **BCB** | Banco Central do Brasil | Banking, Payments, VASP |
//! | **CVM** | Comissao de Valores Mobiliarios | Securities, Funds |
//! | **SUSEP** | Superintendencia de Seguros Privados | Insurance |
//! | **DREI** | Department of Business Registration | Corporate |
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries. These definitions provide
//! the Brazil-specific license taxonomy used by the compliance tensor's
//! LICENSING domain evaluation.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── BCB — Banco Central do Brasil ────────────────────────────────────────────

/// BCB regulator profile.
pub fn bcb_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "br-bcb".to_string(),
        name: "Banco Central do Brasil".to_string(),
        jurisdiction_id: "br".to_string(),
        registry_url: Some("https://www.bcb.gov.br".to_string()),
        did: None,
        api_capabilities: vec![
            "institution_registry".to_string(),
            "pix_integration".to_string(),
        ],
    }
}

/// BCB license type definitions.
pub fn bcb_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "br-bcb:commercial-bank".to_string(),
            name: "Commercial Banking License".to_string(),
            description: "Authorization to operate as a commercial bank (banco comercial) in Brazil"
                .to_string(),
            regulator_id: "br-bcb".to_string(),
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
            license_type_id: "br-bcb:investment-bank".to_string(),
            name: "Investment Banking License".to_string(),
            description:
                "Authorization to operate as an investment bank (banco de investimento) in Brazil"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "underwriting".to_string(),
                "advisory".to_string(),
                "proprietary_trading".to_string(),
                "capital_markets".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:development-bank".to_string(),
            name: "Development Banking License".to_string(),
            description: "Authorization to operate as a development bank in Brazil".to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "development_lending".to_string(),
                "project_finance".to_string(),
                "infrastructure_financing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:payment-institution-issuer".to_string(),
            name: "Payment Institution - Issuer".to_string(),
            description:
                "Authorization to operate as an electronic money issuer (instituicao de pagamento emissora)"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "issuing_e_money".to_string(),
                "prepaid_instruments".to_string(),
                "payment_accounts".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:payment-institution-acquirer".to_string(),
            name: "Payment Institution - Acquirer".to_string(),
            description:
                "Authorization to operate as a payment acquirer (instituicao de pagamento credenciadora)"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "merchant_acquisition".to_string(),
                "payment_processing".to_string(),
                "settlement".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:payment-institution-account".to_string(),
            name: "Payment Institution - Payment Account".to_string(),
            description:
                "Authorization to manage payment accounts (instituicao de pagamento iniciadora)"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_accounts".to_string(),
                "payment_initiation".to_string(),
                "pix_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:direct-credit-society".to_string(),
            name: "Direct Credit Society License".to_string(),
            description:
                "Authorization to operate as a direct credit society (sociedade de credito direto)"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_lending".to_string(),
                "peer_to_peer_lending".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-bcb:vasp-authorization".to_string(),
            name: "Virtual Asset Service Provider Authorization".to_string(),
            description:
                "Authorization to operate as a VASP under Brazilian virtual asset regulations"
                    .to_string(),
            regulator_id: "br-bcb".to_string(),
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
    ]
}

// ── CVM — Comissao de Valores Mobiliarios ────────────────────────────────────

/// CVM regulator profile.
pub fn cvm_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "br-cvm".to_string(),
        name: "Comissao de Valores Mobiliarios".to_string(),
        jurisdiction_id: "br".to_string(),
        registry_url: Some("https://www.gov.br/cvm".to_string()),
        did: None,
        api_capabilities: vec!["participant_registry".to_string()],
    }
}

/// CVM license type definitions.
pub fn cvm_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "br-cvm:securities-broker".to_string(),
            name: "Securities Broker Registration".to_string(),
            description: "Registration to operate as a securities broker (corretora) in Brazil"
                .to_string(),
            regulator_id: "br-cvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securities_brokerage".to_string(),
                "order_execution".to_string(),
                "trading".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-cvm:fund-manager".to_string(),
            name: "Investment Fund Manager Registration".to_string(),
            description:
                "Registration to manage investment funds (gestor de fundos) in Brazil"
                    .to_string(),
            regulator_id: "br-cvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "fund_management".to_string(),
                "portfolio_management".to_string(),
                "discretionary_management".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-cvm:securitizer".to_string(),
            name: "Securitizer Registration".to_string(),
            description:
                "Registration to operate as a securitizer (securitizadora) in Brazil"
                    .to_string(),
            regulator_id: "br-cvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "securitization".to_string(),
                "receivables_issuance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-cvm:investment-adviser".to_string(),
            name: "Investment Adviser Registration".to_string(),
            description:
                "Registration to provide investment advisory services in Brazil"
                    .to_string(),
            regulator_id: "br-cvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "investment_advisory".to_string(),
                "financial_planning".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-cvm:crowdfunding-platform".to_string(),
            name: "Crowdfunding Platform Registration".to_string(),
            description:
                "Registration to operate an equity crowdfunding platform under CVM Instruction 588"
                    .to_string(),
            regulator_id: "br-cvm".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "crowdfunding_platform".to_string(),
                "equity_crowdfunding".to_string(),
                "small_business_financing".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: Some(3),
        },
    ]
}

// ── SUSEP — Superintendencia de Seguros Privados ─────────────────────────────

/// SUSEP regulator profile.
pub fn susep_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "br-susep".to_string(),
        name: "Superintendencia de Seguros Privados".to_string(),
        jurisdiction_id: "br".to_string(),
        registry_url: Some("https://www.gov.br/susep".to_string()),
        did: None,
        api_capabilities: vec!["insurer_registry".to_string()],
    }
}

/// SUSEP license type definitions.
pub fn susep_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "br-susep:insurance-company".to_string(),
            name: "Insurance Company Authorization".to_string(),
            description: "Authorization to operate as an insurance company in Brazil".to_string(),
            regulator_id: "br-susep".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "general_insurance".to_string(),
                "life_insurance".to_string(),
                "health_insurance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-susep:reinsurer".to_string(),
            name: "Reinsurer Authorization".to_string(),
            description: "Authorization to operate as a reinsurer in Brazil".to_string(),
            regulator_id: "br-susep".to_string(),
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
            license_type_id: "br-susep:insurance-broker".to_string(),
            name: "Insurance Broker Registration".to_string(),
            description: "Registration to operate as an insurance broker in Brazil".to_string(),
            regulator_id: "br-susep".to_string(),
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
            license_type_id: "br-susep:open-pension-fund".to_string(),
            name: "Open Pension Fund Authorization".to_string(),
            description:
                "Authorization to manage an open supplementary pension fund (EAPC) in Brazil"
                    .to_string(),
            regulator_id: "br-susep".to_string(),
            category: Some("insurance".to_string()),
            permitted_activities: vec![
                "pension_fund_management".to_string(),
                "retirement_products".to_string(),
                "annuities".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── DREI — Department of Business Registration ──────────────────────────────

/// DREI regulator profile.
pub fn drei_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "br-drei".to_string(),
        name: "Department of Business Registration".to_string(),
        jurisdiction_id: "br".to_string(),
        registry_url: Some("https://www.gov.br/economia/drei".to_string()),
        did: None,
        api_capabilities: vec!["company_search".to_string()],
    }
}

/// DREI license type definitions.
pub fn drei_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "br-drei:sa".to_string(),
            name: "Sociedade Anonima (S.A.) Registration".to_string(),
            description: "Registration of a corporation (sociedade anonima) in Brazil".to_string(),
            regulator_id: "br-drei".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "capital_raising".to_string(),
                "securities_issuance".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-drei:ltda".to_string(),
            name: "Sociedade Limitada (Ltda.) Registration".to_string(),
            description:
                "Registration of a limited liability company (sociedade limitada) in Brazil"
                    .to_string(),
            regulator_id: "br-drei".to_string(),
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
            license_type_id: "br-drei:eireli".to_string(),
            name: "EIRELI Registration".to_string(),
            description:
                "Registration of an individual limited liability company (EIRELI) in Brazil"
                    .to_string(),
            regulator_id: "br-drei".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "sole_proprietor_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "br-drei:mei".to_string(),
            name: "Microempreendedor Individual (MEI) Registration".to_string(),
            description:
                "Registration as an individual micro-entrepreneur (MEI) in Brazil".to_string(),
            regulator_id: "br-drei".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "micro_business_operations".to_string(),
                "individual_services".to_string(),
            ],
            requirements: BTreeMap::new(),
            application_fee: BTreeMap::new(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
    ]
}

// ── Brazil Registry Aggregation ──────────────────────────────────────────────

/// All Brazil regulatory authorities.
pub fn brazil_regulators() -> Vec<LicensepackRegulator> {
    vec![
        bcb_regulator(),
        cvm_regulator(),
        susep_regulator(),
        drei_regulator(),
    ]
}

/// All Brazil license type definitions across all authorities.
pub fn brazil_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(bcb_license_types());
    all.extend(cvm_license_types());
    all.extend(susep_license_types());
    all.extend(drei_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brazil_has_four_regulators() {
        let regs = brazil_regulators();
        assert_eq!(regs.len(), 4);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"br-bcb"), "missing BCB");
        assert!(ids.contains(&"br-cvm"), "missing CVM");
        assert!(ids.contains(&"br-susep"), "missing SUSEP");
        assert!(ids.contains(&"br-drei"), "missing DREI");
    }

    #[test]
    fn all_regulators_are_brazil_jurisdiction() {
        for reg in brazil_regulators() {
            assert_eq!(reg.jurisdiction_id, "br", "{} is not br", reg.regulator_id);
        }
    }

    #[test]
    fn brazil_license_types_cover_all_authorities() {
        let types = brazil_license_types();
        assert!(
            types.len() >= 21,
            "expected >= 21 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("br-bcb"), "no BCB license types");
        assert!(authority_ids.contains("br-cvm"), "no CVM license types");
        assert!(authority_ids.contains("br-susep"), "no SUSEP license types");
        assert!(authority_ids.contains("br-drei"), "no DREI license types");
    }

    #[test]
    fn bcb_has_banking_and_payments() {
        let types = bcb_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"br-bcb:commercial-bank"));
        assert!(ids.contains(&"br-bcb:investment-bank"));
        assert!(ids.contains(&"br-bcb:payment-institution-issuer"));
        assert!(ids.contains(&"br-bcb:payment-institution-acquirer"));
        assert!(ids.contains(&"br-bcb:vasp-authorization"));
    }

    #[test]
    fn cvm_has_securities_licenses() {
        let types = cvm_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"br-cvm:securities-broker"));
        assert!(ids.contains(&"br-cvm:fund-manager"));
        assert!(ids.contains(&"br-cvm:securitizer"));
        assert!(ids.contains(&"br-cvm:crowdfunding-platform"));
    }

    #[test]
    fn susep_has_insurance_licenses() {
        let types = susep_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"br-susep:insurance-company"));
        assert!(ids.contains(&"br-susep:reinsurer"));
        assert!(ids.contains(&"br-susep:insurance-broker"));
        assert!(ids.contains(&"br-susep:open-pension-fund"));
    }

    #[test]
    fn drei_has_corporate_registrations() {
        let types = drei_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"br-drei:sa"));
        assert!(ids.contains(&"br-drei:ltda"));
        assert!(ids.contains(&"br-drei:eireli"));
        assert!(ids.contains(&"br-drei:mei"));
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = brazil_license_types();
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
        for lt in brazil_license_types() {
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
        for lt in brazil_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in brazil_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in brazil_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }
}
