//! # UAE Regpack Content Module
//!
//! Regulatory content for the United Arab Emirates jurisdiction, covering:
//! - CBUAE (Central Bank of the UAE)
//! - SCA (Securities and Commodities Authority)
//! - ADGM FSRA (Abu Dhabi Global Market Financial Services Regulatory Authority)
//! - DIFC DFSA (Dubai International Financial Centre — Dubai Financial Services Authority)
//! - UAE FIU (Financial Intelligence Unit)
//!
//! Sources: Federal Decree-Law No. 14/2018 (AML/CFT), CBUAE regulations,
//! SCA Board Decisions, ADGM FSMR 2015, DIFC Regulatory Law 2004,
//! UNSC consolidated list, OFAC SDN list.

use super::*;

// -- Helpers -----------------------------------------------------------------

fn btree_alias(name: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("name".to_string(), name.to_string());
    m
}

fn btree_address(addr: &str) -> BTreeMap<String, String> {
    let mut m = BTreeMap::new();
    m.insert("address".to_string(), addr.to_string());
    m
}

// -- Regulator Profiles ------------------------------------------------------

/// Central Bank of the UAE — banking, payments, insurance supervision, and AML/CFT.
pub fn cbuae_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "banking".to_string(),
        vec![
            "commercial_banks".to_string(),
            "islamic_banks".to_string(),
            "exchange_houses".to_string(),
            "finance_companies".to_string(),
        ],
    );
    scope.insert(
        "payments".to_string(),
        vec![
            "retail_payment_services".to_string(),
            "stored_value_facilities".to_string(),
            "digital_payment_tokens".to_string(),
        ],
    );
    scope.insert(
        "insurance".to_string(),
        vec![
            "insurance_companies".to_string(),
            "insurance_brokers".to_string(),
            "takaful_operators".to_string(),
        ],
    );
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "designated_non_financial_businesses".to_string(),
            "virtual_asset_service_providers".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.centralbank.ae".to_string());
    contact.insert(
        "address".to_string(),
        "King Abdullah Bin Abdulaziz Al Saud Street, Abu Dhabi, UAE".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("licensed_entities_registry".to_string(), true);
    api.insert("exchange_rate_feed".to_string(), true);
    api.insert("payment_systems_oversight".to_string(), true);
    api.insert("aml_cft_reporting".to_string(), true);

    RegulatorProfile {
        regulator_id: "ae-cbuae".to_string(),
        name: "Central Bank of the UAE".to_string(),
        jurisdiction_id: "ae".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Dubai".to_string(),
        business_days: vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ],
    }
}

/// Securities and Commodities Authority — federal capital markets regulator.
pub fn sca_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "capital_markets".to_string(),
        vec![
            "securities_exchanges".to_string(),
            "brokerage_firms".to_string(),
            "investment_funds".to_string(),
            "clearing_houses".to_string(),
        ],
    );
    scope.insert(
        "commodities".to_string(),
        vec![
            "commodity_exchanges".to_string(),
            "commodity_brokers".to_string(),
        ],
    );
    scope.insert(
        "corporate_governance".to_string(),
        vec![
            "listed_companies".to_string(),
            "public_joint_stock_companies".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.sca.gov.ae".to_string());
    contact.insert(
        "address".to_string(),
        "Al Bateen Area, Abu Dhabi, UAE".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("licensed_entities_search".to_string(), true);
    api.insert("disclosure_filings".to_string(), true);
    api.insert("fund_registry".to_string(), true);

    RegulatorProfile {
        regulator_id: "ae-sca".to_string(),
        name: "Securities and Commodities Authority".to_string(),
        jurisdiction_id: "ae".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Dubai".to_string(),
        business_days: vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ],
    }
}

/// ADGM FSRA — Abu Dhabi Global Market Financial Services Regulatory Authority.
pub fn adgm_fsra_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "financial_services".to_string(),
        vec![
            "banks".to_string(),
            "asset_management".to_string(),
            "insurance_intermediaries".to_string(),
            "digital_securities".to_string(),
            "virtual_assets".to_string(),
        ],
    );
    scope.insert(
        "market_infrastructure".to_string(),
        vec![
            "recognised_investment_exchanges".to_string(),
            "recognised_clearing_houses".to_string(),
            "multilateral_trading_facilities".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.adgm.com/fsra".to_string());
    contact.insert(
        "address".to_string(),
        "Al Maryah Island, Abu Dhabi, UAE".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("financial_services_register".to_string(), true);
    api.insert("regulatory_filings".to_string(), true);
    api.insert("virtual_asset_framework".to_string(), true);

    RegulatorProfile {
        regulator_id: "ae-adgm-fsra".to_string(),
        name: "ADGM Financial Services Regulatory Authority".to_string(),
        jurisdiction_id: "ae-abudhabi-adgm".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Dubai".to_string(),
        business_days: vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ],
    }
}

/// DIFC DFSA — Dubai International Financial Centre — Dubai Financial Services Authority.
pub fn difc_dfsa_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "financial_services".to_string(),
        vec![
            "banking".to_string(),
            "insurance".to_string(),
            "asset_management".to_string(),
            "securities".to_string(),
            "crowdfunding".to_string(),
            "innovation_testing_licence".to_string(),
        ],
    );
    scope.insert(
        "ancillary_services".to_string(),
        vec![
            "audit".to_string(),
            "legal".to_string(),
            "credit_rating_agencies".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.dfsa.ae".to_string());
    contact.insert(
        "address".to_string(),
        "Level 13, The Gate, DIFC, Dubai, UAE".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("public_register".to_string(), true);
    api.insert("regulatory_filings".to_string(), true);
    api.insert("complaints_portal".to_string(), true);

    RegulatorProfile {
        regulator_id: "ae-difc-dfsa".to_string(),
        name: "Dubai Financial Services Authority".to_string(),
        jurisdiction_id: "ae-dubai-difc".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Dubai".to_string(),
        business_days: vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ],
    }
}

/// UAE Financial Intelligence Unit — AML/CFT reporting and analysis.
pub fn uae_fiu_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "suspicious_transaction_reports".to_string(),
            "suspicious_activity_reports".to_string(),
            "targeted_financial_sanctions".to_string(),
            "proliferation_financing".to_string(),
            "mutual_legal_assistance".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.uaefiu.gov.ae".to_string());
    contact.insert(
        "address".to_string(),
        "CBUAE Building, Abu Dhabi, UAE".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("goaml_reporting".to_string(), true);
    api.insert("sanctions_screening".to_string(), true);

    RegulatorProfile {
        regulator_id: "ae-fiu".to_string(),
        name: "UAE Financial Intelligence Unit".to_string(),
        jurisdiction_id: "ae".to_string(),
        parent_authority: Some("ae-cbuae".to_string()),
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Dubai".to_string(),
        business_days: vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ],
    }
}

/// All UAE regulatory authorities relevant to regpack domains.
pub fn uae_regulators() -> Vec<RegulatorProfile> {
    vec![
        cbuae_regulator(),
        sca_regulator(),
        adgm_fsra_regulator(),
        difc_dfsa_regulator(),
        uae_fiu_regulator(),
    ]
}

// -- Sanctions Entries -------------------------------------------------------
//
// Representative entries from UAE's sanctions regime.
// Sources: UAE Local Terrorist List (Cabinet Decision No. 83/2014),
//          OFAC SDN List, UNSC 1267/1989/2253 Consolidated List.
//
// NOTE: These are publicly available, gazette-notified designations.
// Real deployment must pull from live UAE Executive Office feed,
// OFAC SDN XML, and UNSC XML.

/// Representative UAE sanctions entries for regpack content.
pub fn uae_sanctions_entries() -> Vec<SanctionsEntry> {
    vec![
        SanctionsEntry {
            entry_id: "ae-local-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "uae_local_terrorist_list".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Al-Qaeda in the Arabian Peninsula".to_string(),
            aliases: vec![
                btree_alias("AQAP"),
                btree_alias("Tanzim Qaidat al-Jihad fi Jazirat al-Arab"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "uae_cabinet_decision_83_2014".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2014-11-15".to_string()),
            remarks: Some("UAE Cabinet Decision No. 83/2014; UNSC QDe.129".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-local-002".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "uae_local_terrorist_list".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Islamic State of Iraq and the Levant".to_string(),
            aliases: vec![
                btree_alias("ISIL"),
                btree_alias("ISIS"),
                btree_alias("Daesh"),
                btree_alias("Islamic State"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "uae_cabinet_decision_83_2014".to_string(),
                "unsc_2253".to_string(),
            ],
            listing_date: Some("2014-11-15".to_string()),
            remarks: Some("UAE Cabinet Decision No. 83/2014; UNSC".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-local-003".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["uae_local_terrorist_list".to_string()],
            primary_name: "Al-Islah (UAE)".to_string(),
            aliases: vec![
                btree_alias("Reform and Social Guidance Association"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("United Arab Emirates")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["uae_cabinet_decision_83_2014".to_string()],
            listing_date: Some("2014-11-15".to_string()),
            remarks: Some("UAE Cabinet Decision No. 83/2014".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-local-004".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "uae_local_terrorist_list".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Al-Qaeda".to_string(),
            aliases: vec![
                btree_alias("Al-Qaida"),
                btree_alias("The Base"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "uae_cabinet_decision_83_2014".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2014-11-15".to_string()),
            remarks: Some("UAE Cabinet Decision No. 83/2014; UNSC QDe.004".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-ofac-001".to_string(),
            entry_type: "entity".to_string(),
            source_lists: vec!["ofac_sdn".to_string()],
            primary_name: "Hizballah".to_string(),
            aliases: vec![
                btree_alias("Hezbollah"),
                btree_alias("Party of God"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Lebanon")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "ofac_sdgt".to_string(),
                "ofac_lebanon".to_string(),
            ],
            listing_date: Some("1997-10-08".to_string()),
            remarks: Some("OFAC SDN; Specially Designated Global Terrorist".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-unsc-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "uae_local_terrorist_list".to_string(),
            ],
            primary_name: "Taliban".to_string(),
            aliases: vec![
                btree_alias("Islamic Emirate of Afghanistan"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Afghanistan")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_1988".to_string(),
                "uae_cabinet_decision_83_2014".to_string(),
            ],
            listing_date: Some("1999-10-15".to_string()),
            remarks: Some("UNSC 1988 List; UAE Cabinet Decision No. 83/2014".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-local-005".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["uae_local_terrorist_list".to_string()],
            primary_name: "Ansar Bait al-Maqdis".to_string(),
            aliases: vec![
                btree_alias("Ansar Bayt al-Maqdis"),
                btree_alias("Supporters of Jerusalem"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Sinai Peninsula, Egypt")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["uae_cabinet_decision_83_2014".to_string()],
            listing_date: Some("2014-11-15".to_string()),
            remarks: Some("UAE Cabinet Decision No. 83/2014".to_string()),
        },
        SanctionsEntry {
            entry_id: "ae-ofac-002".to_string(),
            entry_type: "entity".to_string(),
            source_lists: vec!["ofac_sdn".to_string()],
            primary_name: "Islamic Revolutionary Guard Corps".to_string(),
            aliases: vec![
                btree_alias("IRGC"),
                btree_alias("Pasdaran"),
                btree_alias("Sepah"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Iran")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "ofac_iran".to_string(),
                "ofac_irgc".to_string(),
            ],
            listing_date: Some("2007-10-25".to_string()),
            remarks: Some("OFAC SDN; Iran-related sanctions".to_string()),
        },
    ]
}

/// Build a sanctions snapshot from UAE entries.
pub fn uae_sanctions_snapshot() -> SanctionsSnapshot {
    let entries = uae_sanctions_entries();

    let mut counts = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
    }

    let mut sources = BTreeMap::new();
    sources.insert(
        "uae_local_terrorist_list".to_string(),
        serde_json::json!({
            "name": "UAE Local Terrorist List — Cabinet Decision No. 83/2014",
            "url": "https://www.uaeiec.gov.ae/en/un-sanctions",
            "authority": "UAE Cabinet / Executive Office of AML/CFT",
            "legal_basis": "Federal Decree-Law No. 7/2014 on Combating Terrorism Offences"
        }),
    );
    sources.insert(
        "ofac_sdn".to_string(),
        serde_json::json!({
            "name": "OFAC Specially Designated Nationals and Blocked Persons List",
            "url": "https://sanctionssearch.ofac.treas.gov/",
            "authority": "U.S. Department of the Treasury"
        }),
    );
    sources.insert(
        "unsc_1267".to_string(),
        serde_json::json!({
            "name": "UNSC 1267/1989/2253 Consolidated List",
            "url": "https://www.un.org/securitycouncil/sanctions/1267",
            "authority": "United Nations Security Council"
        }),
    );

    SanctionsSnapshot {
        snapshot_id: "ae-sanctions-2026Q1".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        sources,
        consolidated_counts: counts,
        delta_from_previous: None,
    }
}

// -- Compliance Deadlines ----------------------------------------------------

/// UAE compliance deadlines for FY 2026.
pub fn uae_compliance_deadlines() -> Vec<ComplianceDeadline> {
    vec![
        // CBUAE Quarterly Prudential
        ComplianceDeadline {
            deadline_id: "ae-cbuae-quarterly-prudential".to_string(),
            regulator_id: "ae-cbuae".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly prudential return — banks (within 30 days of quarter-end)"
                .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-cbuae:commercial-bank".to_string(),
                "ae-cbuae:islamic-bank".to_string(),
                "ae-cbuae:finance-company".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "ae-cbuae-annual-audited".to_string(),
            regulator_id: "ae-cbuae".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Annual audited financial statements — banks (within 3 months of FY-end)"
                    .to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "ae-cbuae:commercial-bank".to_string(),
                "ae-cbuae:islamic-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "ae-cbuae-car-monthly".to_string(),
            regulator_id: "ae-cbuae".to_string(),
            deadline_type: "report".to_string(),
            description: "Monthly capital adequacy ratio report — banks".to_string(),
            due_date: "2026-02-28".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-cbuae:commercial-bank".to_string(),
                "ae-cbuae:islamic-bank".to_string(),
            ],
        },
        // DFSA Annual Filing
        ComplianceDeadline {
            deadline_id: "ae-dfsa-annual-filing".to_string(),
            regulator_id: "ae-difc-dfsa".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual regulatory return — DFSA authorised firms (within 4 months of FY-end)"
                    .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-difc-dfsa:category-1".to_string(),
                "ae-difc-dfsa:category-2".to_string(),
                "ae-difc-dfsa:category-3a".to_string(),
                "ae-difc-dfsa:category-3b".to_string(),
                "ae-difc-dfsa:category-3c".to_string(),
                "ae-difc-dfsa:category-4".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "ae-dfsa-prudential-quarterly".to_string(),
            regulator_id: "ae-difc-dfsa".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Quarterly prudential information return — DFSA authorised firms"
                    .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 15,
            applicable_license_types: vec![
                "ae-difc-dfsa:category-1".to_string(),
                "ae-difc-dfsa:category-2".to_string(),
            ],
        },
        // ADGM Annual Return
        ComplianceDeadline {
            deadline_id: "ae-adgm-annual-return".to_string(),
            regulator_id: "ae-adgm-fsra".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual regulatory return — ADGM authorised persons (within 4 months of FY-end)"
                    .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-adgm-fsra:financial-services-permission".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "ae-adgm-prudential-quarterly".to_string(),
            regulator_id: "ae-adgm-fsra".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Quarterly prudential return — ADGM authorised persons"
                    .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 15,
            applicable_license_types: vec![
                "ae-adgm-fsra:financial-services-permission".to_string(),
            ],
        },
        // SCA Annual Report
        ComplianceDeadline {
            deadline_id: "ae-sca-annual-report".to_string(),
            regulator_id: "ae-sca".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual report — listed companies and licensed entities (within 3 months of FY-end)"
                    .to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-sca:brokerage-firm".to_string(),
                "ae-sca:investment-fund".to_string(),
                "ae-sca:listed-company".to_string(),
            ],
        },
        // FIU STR Reporting
        ComplianceDeadline {
            deadline_id: "ae-fiu-str-ongoing".to_string(),
            regulator_id: "ae-fiu".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Suspicious Transaction Report — within 30 days of suspicion (Federal Decree-Law No. 20/2018 Art. 15)"
                    .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-cbuae:commercial-bank".to_string(),
                "ae-cbuae:islamic-bank".to_string(),
                "ae-cbuae:exchange-house".to_string(),
                "ae-sca:brokerage-firm".to_string(),
                "ae-difc-dfsa:category-1".to_string(),
                "ae-adgm-fsra:financial-services-permission".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "ae-fiu-sar-ongoing".to_string(),
            regulator_id: "ae-fiu".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Suspicious Activity Report — within 30 days of suspicion (Federal Decree-Law No. 20/2018)"
                    .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ae-cbuae:commercial-bank".to_string(),
                "ae-cbuae:islamic-bank".to_string(),
                "ae-cbuae:exchange-house".to_string(),
            ],
        },
    ]
}

// -- Reporting Requirements --------------------------------------------------

/// UAE reporting requirements across regulators.
pub fn uae_reporting_requirements() -> Vec<ReportingRequirement> {
    vec![
        ReportingRequirement {
            report_type_id: "ae-fiu-str".to_string(),
            name: "Suspicious Transaction Report (STR)".to_string(),
            regulator_id: "ae-fiu".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "islamic_bank".to_string(),
                "exchange_house".to_string(),
                "finance_company".to_string(),
                "brokerage_firm".to_string(),
                "insurance_company".to_string(),
                "dnfbp".to_string(),
                "vasp".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_suspicion".to_string(), "30".to_string());
                inner.insert("submission_system".to_string(), "goAML".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("goAML XML"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://goaml.uaefiu.gov.ae"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Federal Decree-Law No. 20/2018 Art. 25: imprisonment and/or fine AED 50,000 to AED 5,000,000"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ae-fiu-sar".to_string(),
            name: "Suspicious Activity Report (SAR)".to_string(),
            regulator_id: "ae-fiu".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "islamic_bank".to_string(),
                "exchange_house".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_suspicion".to_string(), "30".to_string());
                inner.insert("submission_system".to_string(), "goAML".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("goAML XML"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://goaml.uaefiu.gov.ae"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Federal Decree-Law No. 20/2018 Art. 25: fine AED 50,000 to AED 5,000,000"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ae-cbuae-ctr".to_string(),
            name: "Currency Transaction Report (CTR)".to_string(),
            regulator_id: "ae-cbuae".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "islamic_bank".to_string(),
                "exchange_house".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_transaction".to_string(), "30".to_string());
                inner.insert("threshold_aed".to_string(), "55000".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("CBUAE prescribed format"));
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("CBUAE administrative penalty per Notice No. 74/2019"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ae-cbuae-prudential-quarterly".to_string(),
            name: "Quarterly Prudential Return".to_string(),
            regulator_id: "ae-cbuae".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "islamic_bank".to_string(),
                "finance_company".to_string(),
            ],
            frequency: "quarterly".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_after_quarter_end".to_string(), "30".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("CBUAE XBRL / Reporting Portal"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("CBUAE Banking Supervision Department"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("CBUAE administrative sanctions per Decretal Federal Law No. 14/2018"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ae-dfsa-annual-return".to_string(),
            name: "Annual Regulatory Return".to_string(),
            regulator_id: "ae-difc-dfsa".to_string(),
            applicable_to: vec![
                "dfsa_authorised_firm".to_string(),
            ],
            frequency: "annual".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("months_after_fy_end".to_string(), "4".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("DFSA e-Portal"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://eportal.dfsa.ae"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("DFSA Regulatory Law 2004 Art. 90: fine up to USD 100,000"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ae-sca-annual-report".to_string(),
            name: "Annual Report — Listed Entities".to_string(),
            regulator_id: "ae-sca".to_string(),
            applicable_to: vec![
                "listed_company".to_string(),
                "brokerage_firm".to_string(),
                "investment_fund".to_string(),
            ],
            frequency: "annual".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("months_after_fy_end".to_string(), "3".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("SCA e-Services Portal"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://eservices.sca.gov.ae"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("SCA Board Decision: administrative fine per violation"),
                );
                p
            },
        },
    ]
}

// -- Full Regpack Builder ----------------------------------------------------

/// Build a complete UAE regpack with all content.
///
/// Assembles regulators, sanctions, deadlines, and reporting requirements
/// into a content-addressed regpack for the `ae` jurisdiction.
#[allow(clippy::type_complexity)]
pub fn build_uae_regpack(
) -> PackResult<(
    Regpack,
    RegPackMetadata,
    SanctionsSnapshot,
    Vec<ComplianceDeadline>,
    Vec<ReportingRequirement>,
)> {
    let regulators = uae_regulators();
    let sanctions_snapshot = uae_sanctions_snapshot();
    let deadlines = uae_compliance_deadlines();
    let reporting = uae_reporting_requirements();

    let mut includes = BTreeMap::new();
    includes.insert(
        "regulators".to_string(),
        serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
    );
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(uae_sanctions_entries().len()),
    );
    includes.insert(
        "compliance_deadlines".to_string(),
        serde_json::json!(deadlines.len()),
    );
    includes.insert(
        "reporting_requirements".to_string(),
        serde_json::json!(reporting.len()),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:ae:financial:2026Q1".to_string(),
        jurisdiction_id: "ae".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "uae_local_terrorist_list",
                "name": "UAE Local Terrorist List — Cabinet Decision No. 83/2014",
                "authority": "UAE Cabinet / Executive Office of AML/CFT"
            }),
            serde_json::json!({
                "source_id": "ofac_sdn",
                "name": "OFAC Specially Designated Nationals and Blocked Persons List",
                "authority": "U.S. Department of the Treasury"
            }),
            serde_json::json!({
                "source_id": "unsc_1267",
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "cbuae_regulations",
                "name": "CBUAE Regulations and Standards",
                "authority": "Central Bank of the UAE"
            }),
            serde_json::json!({
                "source_id": "federal_decree_law_20_2018",
                "name": "Federal Decree-Law No. 20/2018 on Anti-Money Laundering",
                "authority": "UAE Government"
            }),
            serde_json::json!({
                "source_id": "dfsa_rulebook",
                "name": "DFSA Rulebook",
                "authority": "Dubai Financial Services Authority"
            }),
        ],
        includes,
        previous_regpack_digest: None,
        created_at: Some("2026-01-15T00:00:00Z".to_string()),
        expires_at: Some("2026-04-15T00:00:00Z".to_string()),
        digest_sha256: None,
    };

    let digest = compute_regpack_digest(
        &metadata,
        Some(&sanctions_snapshot),
        Some(&regulators),
        Some(&deadlines),
    )?;

    let regpack = Regpack {
        jurisdiction: JurisdictionId::new("ae".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "UAE Financial Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting))
}

/// Build a sanctions-domain-specific UAE regpack.
///
/// Produces a regpack focused on the `sanctions` compliance domain,
/// containing the UAE Local Terrorist List, OFAC SDN, and UNSC 1267
/// consolidated list entries. Separate from the `financial` domain
/// regpack which includes broader regulatory data (regulators,
/// compliance deadlines, reporting requirements).
///
/// The sanctions regpack is content-addressed independently so that
/// sanctions-list-only updates can be pushed without rebuilding the
/// full financial regpack.
pub fn build_uae_sanctions_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
    let sanctions_snapshot = uae_sanctions_snapshot();

    let mut includes = BTreeMap::new();
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(uae_sanctions_entries().len()),
    );
    includes.insert(
        "source_lists".to_string(),
        serde_json::json!(["uae_local_terrorist_list", "ofac_sdn", "unsc_1267"]),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:ae:sanctions:2026Q1".to_string(),
        jurisdiction_id: "ae".to_string(),
        domain: "sanctions".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "uae_local_terrorist_list",
                "name": "UAE Local Terrorist List — Cabinet Decision No. 83/2014",
                "authority": "UAE Cabinet / Executive Office of AML/CFT"
            }),
            serde_json::json!({
                "source_id": "ofac_sdn",
                "name": "OFAC Specially Designated Nationals and Blocked Persons List",
                "authority": "U.S. Department of the Treasury"
            }),
            serde_json::json!({
                "source_id": "unsc_1267",
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "authority": "United Nations Security Council"
            }),
        ],
        includes,
        previous_regpack_digest: None,
        created_at: Some("2026-01-15T00:00:00Z".to_string()),
        expires_at: Some("2026-04-15T00:00:00Z".to_string()),
        digest_sha256: None,
    };

    let digest = compute_regpack_digest(
        &metadata,
        Some(&sanctions_snapshot),
        None, // No regulators — sanctions-only domain
        None, // No deadlines — sanctions-only domain
    )?;

    let regpack = Regpack {
        jurisdiction: JurisdictionId::new("ae".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "UAE Sanctions Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot))
}

// -- Tests -------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uae_has_five_regulators() {
        let regs = uae_regulators();
        assert_eq!(regs.len(), 5);
        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"ae-cbuae"));
        assert!(ids.contains(&"ae-sca"));
        assert!(ids.contains(&"ae-adgm-fsra"));
        assert!(ids.contains(&"ae-difc-dfsa"));
        assert!(ids.contains(&"ae-fiu"));
    }

    #[test]
    fn all_regulators_have_timezone() {
        for reg in uae_regulators() {
            assert_eq!(
                reg.timezone, "Asia/Dubai",
                "{} has wrong timezone",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn fiu_parent_is_cbuae() {
        let fiu = uae_fiu_regulator();
        assert_eq!(fiu.parent_authority, Some("ae-cbuae".to_string()));
    }

    #[test]
    fn adgm_fsra_jurisdiction_is_adgm() {
        let fsra = adgm_fsra_regulator();
        assert_eq!(fsra.jurisdiction_id, "ae-abudhabi-adgm");
    }

    #[test]
    fn difc_dfsa_jurisdiction_is_difc() {
        let dfsa = difc_dfsa_regulator();
        assert_eq!(dfsa.jurisdiction_id, "ae-dubai-difc");
    }

    #[test]
    fn business_days_are_sunday_through_thursday() {
        let expected = vec![
            "sunday".to_string(),
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
        ];
        for reg in uae_regulators() {
            assert_eq!(
                reg.business_days, expected,
                "{} has wrong business days",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn sanctions_entries_all_have_source_lists() {
        for entry in uae_sanctions_entries() {
            assert!(
                !entry.source_lists.is_empty(),
                "{} has no source_lists",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_entries_all_have_programs() {
        for entry in uae_sanctions_entries() {
            assert!(
                !entry.programs.is_empty(),
                "{} has no programs",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_snapshot_has_sources() {
        let snap = uae_sanctions_snapshot();
        assert!(snap.sources.contains_key("uae_local_terrorist_list"));
        assert!(snap.sources.contains_key("ofac_sdn"));
        assert!(snap.sources.contains_key("unsc_1267"));
    }

    #[test]
    fn compliance_deadlines_cover_all_regulators() {
        let deadlines = uae_compliance_deadlines();
        let regulator_ids: std::collections::HashSet<&str> =
            deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
        assert!(regulator_ids.contains("ae-cbuae"), "missing CBUAE deadlines");
        assert!(regulator_ids.contains("ae-difc-dfsa"), "missing DFSA deadlines");
        assert!(regulator_ids.contains("ae-adgm-fsra"), "missing ADGM deadlines");
        assert!(regulator_ids.contains("ae-sca"), "missing SCA deadlines");
        assert!(regulator_ids.contains("ae-fiu"), "missing FIU deadlines");
    }

    #[test]
    fn compliance_deadlines_have_unique_ids() {
        let deadlines = uae_compliance_deadlines();
        let mut ids = std::collections::HashSet::new();
        for dl in &deadlines {
            assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
        }
    }

    #[test]
    fn reporting_requirements_cover_key_reports() {
        let reqs = uae_reporting_requirements();
        let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
        assert!(ids.contains(&"ae-fiu-str"), "missing STR");
        assert!(ids.contains(&"ae-fiu-sar"), "missing SAR");
        assert!(ids.contains(&"ae-cbuae-ctr"), "missing CTR");
        assert!(ids.contains(&"ae-cbuae-prudential-quarterly"), "missing prudential");
        assert!(ids.contains(&"ae-dfsa-annual-return"), "missing DFSA annual");
        assert!(ids.contains(&"ae-sca-annual-report"), "missing SCA annual");
    }

    #[test]
    fn build_uae_regpack_produces_digest() {
        let (regpack, metadata, snap, deadlines, reporting) =
            build_uae_regpack().expect("build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "ae");
        assert!(regpack.digest.is_some(), "regpack should have digest");
        assert_eq!(metadata.jurisdiction_id, "ae");
        assert_eq!(metadata.domain, "financial");
        assert!(!snap.consolidated_counts.is_empty());
        assert!(!deadlines.is_empty());
        assert!(!reporting.is_empty());
    }

    #[test]
    fn build_uae_regpack_is_deterministic() {
        let (rp1, ..) = build_uae_regpack().unwrap();
        let (rp2, ..) = build_uae_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "regpack digest must be deterministic"
        );
    }

    #[test]
    fn build_uae_sanctions_regpack_produces_digest() {
        let (regpack, metadata, snap) =
            build_uae_sanctions_regpack().expect("sanctions build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "ae");
        assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
        assert_eq!(metadata.domain, "sanctions");
        assert_eq!(metadata.jurisdiction_id, "ae");
        assert!(!snap.consolidated_counts.is_empty());
    }

    #[test]
    fn build_uae_sanctions_regpack_is_deterministic() {
        let (rp1, ..) = build_uae_sanctions_regpack().unwrap();
        let (rp2, ..) = build_uae_sanctions_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "sanctions regpack digest must be deterministic"
        );
    }

    #[test]
    fn sanctions_regpack_digest_differs_from_financial() {
        let (financial, ..) = build_uae_regpack().unwrap();
        let (sanctions, ..) = build_uae_sanctions_regpack().unwrap();
        assert_ne!(
            financial.digest.as_ref().unwrap().to_hex(),
            sanctions.digest.as_ref().unwrap().to_hex(),
            "financial and sanctions regpack digests must differ"
        );
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in uae_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, de.regulator_id);
            assert_eq!(reg.name, de.name);
            assert_eq!(reg.timezone, de.timezone);
        }
    }
}
