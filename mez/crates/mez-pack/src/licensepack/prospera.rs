//! # Prospera ZEDE (Honduras) Regulatory Authority License Mappings
//!
//! Prospera-specific license type definitions covering the charter city's
//! unique insurance-based regulatory model:
//!
//! | Authority | Full Name | Domain |
//! |-----------|-----------|--------|
//! | **Prospera RC** | Prospera Regulatory Commission | E-Residency, Business Registration, Financial Services, Insurance-Based Regulation |
//! | **Prospera CR** | Prospera Civil Registry | Physical and E-Resident Registration |
//! | **CNBS** | Honduras National Banking Commission | Traditional Banking (parent authority) |
//!
//! ## Insurance-Based Regulatory Model
//!
//! Prospera uses a unique charter city model where, instead of prescriptive
//! regulatory rules, businesses carry **regulatory liability insurance**.
//! This replaces traditional licensing with a market-based compliance
//! mechanism: insurers assess risk and price coverage, creating incentive
//! alignment without prescriptive rule-making.
//!
//! Key features:
//! - Modular financial services licensing (payments, lending, custody, exchange)
//! - Regulatory liability insurance in lieu of traditional prescriptive licensing
//! - E-Residency as a first-class registration category
//! - Arbitration-first dispute resolution
//!
//! Each authority is represented as a [`LicensepackRegulator`] with its
//! associated [`LicenseTypeDefinition`] entries.

use std::collections::BTreeMap;

use super::license::{LicenseTypeDefinition, LicensepackRegulator};

// ── Prospera Regulatory Commission ────────────────────────────────────────

/// Prospera Regulatory Commission regulator profile.
pub fn prospera_rc_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hn-prospera-rc".to_string(),
        name: "Prospera Regulatory Commission".to_string(),
        jurisdiction_id: "hn-prospera".to_string(),
        registry_url: Some("https://www.prospera.hn".to_string()),
        did: None,
        api_capabilities: vec![
            "entity_registry".to_string(),
            "license_status".to_string(),
            "e_residency_verification".to_string(),
        ],
    }
}

/// Prospera Regulatory Commission license type definitions.
pub fn prospera_rc_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:e-residency".to_string(),
            name: "Prospera E-Residency".to_string(),
            description:
                "Digital residency registration granting access to Prospera's business and legal framework"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "digital_identity".to_string(),
                "remote_business_formation".to_string(),
                "digital_contract_execution".to_string(),
            ],
            requirements: [
                ("identity_verification".to_string(), serde_json::json!(true)),
                ("minimum_age".to_string(), serde_json::json!(18)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "130".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "260".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:prospera-llc".to_string(),
            name: "Prospera LLC Registration".to_string(),
            description:
                "Registration of a limited liability company under Prospera's business entity framework"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "contract_execution".to_string(),
                "asset_holding".to_string(),
            ],
            requirements: [
                ("e_residency_or_physical_residency".to_string(), serde_json::json!(true)),
                ("registered_agent_required".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:prospera-corporation".to_string(),
            name: "Prospera Corporation Registration".to_string(),
            description:
                "Registration of a corporation under Prospera's business entity framework"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "business_operations".to_string(),
                "capital_raising".to_string(),
                "share_issuance".to_string(),
            ],
            requirements: [
                ("e_residency_or_physical_residency".to_string(), serde_json::json!(true)),
                ("registered_agent_required".to_string(), serde_json::json!(true)),
                ("minimum_directors".to_string(), serde_json::json!(1)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:regulated-business-insurance".to_string(),
            name: "Regulated Business Insurance License".to_string(),
            description:
                "Insurance-based regulatory license: business carries regulatory liability insurance in lieu of prescriptive licensing"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "regulated_business_operations".to_string(),
                "consumer_facing_services".to_string(),
            ],
            requirements: [
                ("regulatory_liability_insurance_required".to_string(), serde_json::json!(true)),
                ("insurance_coverage_minimum_usd".to_string(), serde_json::json!(100000)),
                ("approved_insurer_required".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "250".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "500".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:financial-services-modular".to_string(),
            name: "Modular Financial Services License".to_string(),
            description:
                "Modular financial services license with insurance-based compliance — select from payments, lending, custody, exchange modules"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "payment_services".to_string(),
                "lending".to_string(),
                "digital_asset_custody".to_string(),
                "digital_asset_exchange".to_string(),
            ],
            requirements: [
                ("regulatory_liability_insurance_required".to_string(), serde_json::json!(true)),
                ("insurance_coverage_minimum_usd".to_string(), serde_json::json!(500000)),
                ("aml_cft_program_required".to_string(), serde_json::json!(true)),
                ("approved_insurer_required".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "2000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:professional-services".to_string(),
            name: "Professional Services Registration".to_string(),
            description:
                "Registration for professional services providers (legal, accounting, consulting) in Prospera"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "legal_services".to_string(),
                "accounting_services".to_string(),
                "consulting_services".to_string(),
            ],
            requirements: [
                ("regulatory_liability_insurance_required".to_string(), serde_json::json!(true)),
                ("professional_qualification_verification".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "200".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "400".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:arbitration-provider".to_string(),
            name: "Arbitration Service Provider Registration".to_string(),
            description:
                "Registration to provide arbitration and dispute resolution services within Prospera"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("professional".to_string()),
            permitted_activities: vec![
                "commercial_arbitration".to_string(),
                "mediation".to_string(),
                "dispute_resolution".to_string(),
            ],
            requirements: [
                ("regulatory_liability_insurance_required".to_string(), serde_json::json!(true)),
                ("arbitrator_qualification_required".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "500".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "1000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-rc:digital-asset-service-provider".to_string(),
            name: "Digital Asset Service Provider License".to_string(),
            description:
                "License for digital asset services (custody, exchange, issuance) under Prospera's insurance-based framework"
                    .to_string(),
            regulator_id: "hn-prospera-rc".to_string(),
            category: Some("financial".to_string()),
            permitted_activities: vec![
                "digital_asset_custody".to_string(),
                "digital_asset_exchange".to_string(),
                "digital_asset_issuance".to_string(),
                "digital_asset_transfer".to_string(),
            ],
            requirements: [
                ("regulatory_liability_insurance_required".to_string(), serde_json::json!(true)),
                ("insurance_coverage_minimum_usd".to_string(), serde_json::json!(1000000)),
                ("aml_cft_program_required".to_string(), serde_json::json!(true)),
                ("cybersecurity_audit_required".to_string(), serde_json::json!(true)),
                ("approved_insurer_required".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "5000".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "10000".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── Prospera Civil Registry ───────────────────────────────────────────────

/// Prospera Civil Registry regulator profile.
pub fn prospera_cr_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hn-prospera-cr".to_string(),
        name: "Prospera Civil Registry".to_string(),
        jurisdiction_id: "hn-prospera".to_string(),
        registry_url: Some("https://www.prospera.hn/registry".to_string()),
        did: None,
        api_capabilities: vec!["resident_verification".to_string()],
    }
}

/// Prospera Civil Registry license type definitions.
pub fn prospera_cr_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-cr:physical-resident".to_string(),
            name: "Physical Resident Registration".to_string(),
            description: "Registration as a physical resident of Prospera ZEDE".to_string(),
            regulator_id: "hn-prospera-cr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "physical_residency".to_string(),
                "local_business_formation".to_string(),
                "property_ownership".to_string(),
            ],
            requirements: [
                ("identity_verification".to_string(), serde_json::json!(true)),
                ("background_check".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "260".to_string())]
                .into_iter()
                .collect(),
            annual_fee: BTreeMap::new(),
            validity_period_years: None,
        },
        LicenseTypeDefinition {
            license_type_id: "hn-prospera-cr:e-resident".to_string(),
            name: "E-Resident Registration".to_string(),
            description:
                "Registration as an e-resident of Prospera for digital access to the jurisdiction"
                    .to_string(),
            regulator_id: "hn-prospera-cr".to_string(),
            category: Some("corporate".to_string()),
            permitted_activities: vec![
                "digital_residency".to_string(),
                "remote_business_access".to_string(),
                "digital_contract_execution".to_string(),
            ],
            requirements: [
                ("identity_verification".to_string(), serde_json::json!(true)),
            ]
            .into_iter()
            .collect(),
            application_fee: [("USD".to_string(), "130".to_string())]
                .into_iter()
                .collect(),
            annual_fee: [("USD".to_string(), "260".to_string())]
                .into_iter()
                .collect(),
            validity_period_years: Some(1),
        },
    ]
}

// ── CNBS — Honduras National Banking Commission ───────────────────────────

/// CNBS (Comision Nacional de Bancos y Seguros) regulator profile.
pub fn cnbs_regulator() -> LicensepackRegulator {
    LicensepackRegulator {
        regulator_id: "hn-cnbs".to_string(),
        name: "Honduras National Banking and Insurance Commission".to_string(),
        jurisdiction_id: "hn-prospera".to_string(),
        registry_url: Some("https://www.cnbs.gob.hn".to_string()),
        did: None,
        api_capabilities: vec!["bank_registry".to_string()],
    }
}

/// CNBS license type definitions (for traditional banking operating in Prospera).
pub fn cnbs_license_types() -> Vec<LicenseTypeDefinition> {
    vec![
        LicenseTypeDefinition {
            license_type_id: "hn-cnbs:banking-license".to_string(),
            name: "Honduras Banking License".to_string(),
            description:
                "Traditional banking license issued by CNBS for banks operating within Prospera ZEDE"
                    .to_string(),
            regulator_id: "hn-cnbs".to_string(),
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
    ]
}

// ── Prospera Registry Aggregation ─────────────────────────────────────────

/// All Prospera ZEDE regulatory authorities.
pub fn prospera_regulators() -> Vec<LicensepackRegulator> {
    vec![
        prospera_rc_regulator(),
        prospera_cr_regulator(),
        cnbs_regulator(),
    ]
}

/// All Prospera ZEDE license type definitions across all authorities.
pub fn prospera_license_types() -> Vec<LicenseTypeDefinition> {
    let mut all = Vec::new();
    all.extend(prospera_rc_license_types());
    all.extend(prospera_cr_license_types());
    all.extend(cnbs_license_types());
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prospera_has_three_regulators() {
        let regs = prospera_regulators();
        assert_eq!(regs.len(), 3);

        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"hn-prospera-rc"), "missing Prospera RC");
        assert!(ids.contains(&"hn-prospera-cr"), "missing Prospera CR");
        assert!(ids.contains(&"hn-cnbs"), "missing CNBS");
    }

    #[test]
    fn all_regulators_are_hn_prospera_jurisdiction() {
        for reg in prospera_regulators() {
            assert_eq!(
                reg.jurisdiction_id, "hn-prospera",
                "{} is not hn-prospera",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn prospera_license_types_cover_all_authorities() {
        let types = prospera_license_types();
        assert!(
            types.len() >= 11,
            "expected >= 11 license types, got {}",
            types.len()
        );

        let mut authority_ids: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for lt in &types {
            authority_ids.insert(&lt.regulator_id);
        }
        assert!(authority_ids.contains("hn-prospera-rc"), "no Prospera RC license types");
        assert!(authority_ids.contains("hn-prospera-cr"), "no Prospera CR license types");
        assert!(authority_ids.contains("hn-cnbs"), "no CNBS license types");
    }

    #[test]
    fn prospera_rc_has_entity_and_financial_licenses() {
        let types = prospera_rc_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hn-prospera-rc:e-residency"));
        assert!(ids.contains(&"hn-prospera-rc:prospera-llc"));
        assert!(ids.contains(&"hn-prospera-rc:prospera-corporation"));
        assert!(ids.contains(&"hn-prospera-rc:regulated-business-insurance"));
        assert!(ids.contains(&"hn-prospera-rc:financial-services-modular"));
        assert!(ids.contains(&"hn-prospera-rc:professional-services"));
        assert!(ids.contains(&"hn-prospera-rc:arbitration-provider"));
        assert!(ids.contains(&"hn-prospera-rc:digital-asset-service-provider"));
    }

    #[test]
    fn prospera_cr_has_resident_registrations() {
        let types = prospera_cr_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hn-prospera-cr:physical-resident"));
        assert!(ids.contains(&"hn-prospera-cr:e-resident"));
    }

    #[test]
    fn cnbs_has_banking_license() {
        let types = cnbs_license_types();
        let ids: Vec<&str> = types.iter().map(|t| t.license_type_id.as_str()).collect();
        assert!(ids.contains(&"hn-cnbs:banking-license"));
    }

    #[test]
    fn insurance_based_licenses_require_liability_insurance() {
        let insurance_license_ids = [
            "hn-prospera-rc:regulated-business-insurance",
            "hn-prospera-rc:financial-services-modular",
            "hn-prospera-rc:professional-services",
            "hn-prospera-rc:arbitration-provider",
            "hn-prospera-rc:digital-asset-service-provider",
        ];

        for lt in prospera_rc_license_types() {
            if insurance_license_ids.contains(&lt.license_type_id.as_str()) {
                assert!(
                    lt.requirements
                        .contains_key("regulatory_liability_insurance_required"),
                    "{} should require regulatory_liability_insurance_required",
                    lt.license_type_id
                );
                let val = &lt.requirements["regulatory_liability_insurance_required"];
                assert_eq!(
                    val,
                    &serde_json::json!(true),
                    "{} regulatory_liability_insurance_required should be true",
                    lt.license_type_id
                );
            }
        }
    }

    #[test]
    fn financial_licenses_require_insurance_coverage_minimum() {
        let financial_ids = [
            "hn-prospera-rc:regulated-business-insurance",
            "hn-prospera-rc:financial-services-modular",
            "hn-prospera-rc:digital-asset-service-provider",
        ];

        for lt in prospera_rc_license_types() {
            if financial_ids.contains(&lt.license_type_id.as_str()) {
                assert!(
                    lt.requirements
                        .contains_key("insurance_coverage_minimum_usd"),
                    "{} should specify insurance_coverage_minimum_usd",
                    lt.license_type_id
                );
                let min = lt.requirements["insurance_coverage_minimum_usd"]
                    .as_u64()
                    .expect("insurance_coverage_minimum_usd should be a number");
                assert!(
                    min >= 100000,
                    "{} insurance_coverage_minimum_usd should be >= 100000, got {}",
                    lt.license_type_id,
                    min
                );
            }
        }
    }

    #[test]
    fn digital_asset_provider_has_highest_insurance_minimum() {
        let types = prospera_rc_license_types();
        let dasp = types
            .iter()
            .find(|t| t.license_type_id == "hn-prospera-rc:digital-asset-service-provider")
            .expect("missing digital-asset-service-provider");

        let min = dasp.requirements["insurance_coverage_minimum_usd"]
            .as_u64()
            .expect("should be a number");
        assert_eq!(min, 1000000, "digital-asset-service-provider should require $1M coverage");
    }

    #[test]
    fn all_license_types_have_unique_ids() {
        let types = prospera_license_types();
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
        for lt in prospera_license_types() {
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
        for lt in prospera_license_types() {
            assert!(
                !lt.permitted_activities.is_empty(),
                "no permitted_activities for {}",
                lt.license_type_id
            );
        }
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in prospera_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let deserialized: LicensepackRegulator =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, deserialized.regulator_id);
            assert_eq!(reg.name, deserialized.name);
        }
    }

    #[test]
    fn license_type_serialization_roundtrip() {
        for lt in prospera_license_types() {
            let json = serde_json::to_string(&lt).expect("serialize");
            let deserialized: LicenseTypeDefinition =
                serde_json::from_str(&json).expect("deserialize");
            assert_eq!(lt.license_type_id, deserialized.license_type_id);
            assert_eq!(lt.name, deserialized.name);
            assert_eq!(lt.regulator_id, deserialized.regulator_id);
        }
    }

    #[test]
    fn e_residency_has_identity_requirement() {
        let types = prospera_rc_license_types();
        let eresidency = types
            .iter()
            .find(|t| t.license_type_id == "hn-prospera-rc:e-residency")
            .expect("missing e-residency");
        assert!(
            eresidency.requirements.contains_key("identity_verification"),
            "e-residency should require identity_verification"
        );
    }

    #[test]
    fn modular_financial_services_requires_aml() {
        let types = prospera_rc_license_types();
        let modular = types
            .iter()
            .find(|t| t.license_type_id == "hn-prospera-rc:financial-services-modular")
            .expect("missing financial-services-modular");
        assert!(
            modular.requirements.contains_key("aml_cft_program_required"),
            "financial-services-modular should require AML/CFT program"
        );
    }
}
