//! Hong Kong regpack content — real regulatory data.
//!
//! Provides Hong Kong-specific regulatory content:
//!   - Regulator profiles (HKMA, SFC, IA, JFIU)
//!   - Sanctions entries (UN Sanctions Ordinance Cap.537, UNSC consolidated list)
//!   - Compliance deadlines (HKMA quarterly prudential, SFC annual filing, JFIU STR)
//!   - Reporting requirements (STR, CTR, HKMA prudential returns, SFC annual returns)

use super::*;

// ── Helpers ──────────────────────────────────────────────────────────────

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

// ── Regulator Profiles ──────────────────────────────────────────────────

/// Hong Kong Monetary Authority — central banking institution and banking regulator.
pub fn hkma_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "banking".to_string(),
        vec![
            "licensed_banks".to_string(),
            "restricted_licence_banks".to_string(),
            "deposit_taking_companies".to_string(),
        ],
    );
    scope.insert(
        "payments".to_string(),
        vec![
            "stored_value_facilities".to_string(),
            "retail_payment_systems".to_string(),
            "faster_payment_system".to_string(),
        ],
    );
    scope.insert(
        "monetary_stability".to_string(),
        vec![
            "linked_exchange_rate".to_string(),
            "monetary_base_management".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.hkma.gov.hk".to_string());
    contact.insert(
        "address".to_string(),
        "55th Floor, Two International Finance Centre, 8 Finance Street, Central, Hong Kong"
            .to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("bank_registry".to_string(), true);
    api.insert("svf_registry".to_string(), true);
    api.insert("open_api_framework".to_string(), true);
    api.insert("prudential_returns".to_string(), true);

    RegulatorProfile {
        regulator_id: "hk-hkma".to_string(),
        name: "Hong Kong Monetary Authority".to_string(),
        jurisdiction_id: "hk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Hong_Kong".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Securities and Futures Commission — securities and futures markets regulator.
pub fn sfc_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "securities".to_string(),
        vec![
            "dealing_in_securities".to_string(),
            "advising_on_securities".to_string(),
            "securities_margin_financing".to_string(),
        ],
    );
    scope.insert(
        "futures".to_string(),
        vec![
            "dealing_in_futures_contracts".to_string(),
            "leveraged_foreign_exchange".to_string(),
        ],
    );
    scope.insert(
        "asset_management".to_string(),
        vec![
            "asset_management".to_string(),
            "fund_management".to_string(),
            "advising_on_corporate_finance".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.sfc.hk".to_string());
    contact.insert(
        "address".to_string(),
        "54th Floor, One Island East, 18 Westlands Road, Quarry Bay, Hong Kong".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("licensed_persons_register".to_string(), true);
    api.insert("investment_products_register".to_string(), true);
    api.insert("wings_filing".to_string(), true);

    RegulatorProfile {
        regulator_id: "hk-sfc".to_string(),
        name: "Securities and Futures Commission".to_string(),
        jurisdiction_id: "hk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Hong_Kong".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Insurance Authority — insurance industry regulator.
pub fn ia_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "insurance".to_string(),
        vec![
            "authorized_insurers".to_string(),
            "insurance_intermediaries".to_string(),
            "captive_insurers".to_string(),
        ],
    );
    scope.insert(
        "policyholder_protection".to_string(),
        vec![
            "conduct_regulation".to_string(),
            "prudential_supervision".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.ia.org.hk".to_string());
    contact.insert(
        "address".to_string(),
        "19th Floor, 41 Heung Yip Road, Wong Chuk Hang, Hong Kong".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("insurer_register".to_string(), true);
    api.insert("intermediary_register".to_string(), true);

    RegulatorProfile {
        regulator_id: "hk-ia".to_string(),
        name: "Insurance Authority".to_string(),
        jurisdiction_id: "hk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Hong_Kong".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Joint Financial Intelligence Unit — Hong Kong's financial intelligence unit for AML/CFT.
pub fn jfiu_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "suspicious_transaction_reports".to_string(),
            "large_cash_transaction_reports".to_string(),
            "targeted_financial_sanctions".to_string(),
            "cross_border_movement_reports".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert(
        "website".to_string(),
        "https://www.jfiu.gov.hk".to_string(),
    );
    contact.insert(
        "address".to_string(),
        "Police Headquarters, Arsenal House, 1 Arsenal Street, Wan Chai, Hong Kong".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("str_filing".to_string(), true);
    api.insert("sanctions_query".to_string(), true);

    RegulatorProfile {
        regulator_id: "hk-jfiu".to_string(),
        name: "Joint Financial Intelligence Unit".to_string(),
        jurisdiction_id: "hk".to_string(),
        parent_authority: Some("hk-hkma".to_string()),
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Hong_Kong".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// All Hong Kong regulatory authorities relevant to regpack domains.
pub fn hong_kong_regulators() -> Vec<RegulatorProfile> {
    vec![
        hkma_regulator(),
        sfc_regulator(),
        ia_regulator(),
        jfiu_regulator(),
    ]
}

// ── Sanctions Entries ────────────────────────────────────────────────────
//
// Representative entries from Hong Kong's UN sanctions regime.
// Sources: United Nations Sanctions Ordinance (Cap.537),
//          UNSC Consolidated Sanctions List.
//
// NOTE: These are publicly available, gazette-notified designations
// implemented via the United Nations Sanctions (DPRK) Regulation,
// United Nations Sanctions (Taliban) Regulation, etc.
// Real deployment must pull from live UNSC XML feed and HK Gazette notices.

/// Representative Hong Kong sanctions entries for regpack content.
pub fn hong_kong_sanctions_entries() -> Vec<SanctionsEntry> {
    vec![
        SanctionsEntry {
            entry_id: "hk-uns-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1718_dprk".to_string(),
                "hk_cap537_dprk".to_string(),
            ],
            primary_name: "Korea Mining Development Trading Corporation".to_string(),
            aliases: vec![
                btree_alias("KOMID"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Pyongyang, DPRK")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_1718".to_string(),
                "cap537_dprk_regulation".to_string(),
            ],
            listing_date: Some("2009-04-24".to_string()),
            remarks: Some("UNSC KPe.001; UN Sanctions (DPRK) Regulation (Cap.537AK)".to_string()),
        },
        SanctionsEntry {
            entry_id: "hk-uns-002".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1718_dprk".to_string(),
                "hk_cap537_dprk".to_string(),
            ],
            primary_name: "Korea Ryonbong General Corporation".to_string(),
            aliases: vec![
                btree_alias("Lyongaksan General Trading Corporation"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Pot'onggang District, Pyongyang, DPRK")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_1718".to_string(),
                "cap537_dprk_regulation".to_string(),
            ],
            listing_date: Some("2009-04-24".to_string()),
            remarks: Some("UNSC KPe.002; UN Sanctions (DPRK) Regulation (Cap.537AK)".to_string()),
        },
        SanctionsEntry {
            entry_id: "hk-uns-003".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "hk_cap537_taliban".to_string(),
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
                "unsc_1267".to_string(),
                "cap537_taliban_regulation".to_string(),
            ],
            listing_date: Some("2001-10-15".to_string()),
            remarks: Some("UNSC QDe.004; UN Sanctions (Taliban) Regulation (Cap.537AB)".to_string()),
        },
        SanctionsEntry {
            entry_id: "hk-uns-004".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "hk_cap537_isil".to_string(),
            ],
            primary_name: "Islamic State in Iraq and the Levant".to_string(),
            aliases: vec![
                btree_alias("ISIL"),
                btree_alias("ISIS"),
                btree_alias("Daesh"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_2253".to_string(),
                "cap537_isil_regulation".to_string(),
            ],
            listing_date: Some("2014-05-30".to_string()),
            remarks: Some("UNSC; UN Sanctions (ISIL and Al-Qaida) Regulation (Cap.537BA)".to_string()),
        },
        SanctionsEntry {
            entry_id: "hk-uns-005".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1718_dprk".to_string(),
                "hk_cap537_dprk".to_string(),
            ],
            primary_name: "Ocean Maritime Management Company".to_string(),
            aliases: vec![
                btree_alias("OMM"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Pyongyang, DPRK")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_1718".to_string(),
                "cap537_dprk_regulation".to_string(),
            ],
            listing_date: Some("2014-07-28".to_string()),
            remarks: Some("UNSC KPe.029; UN Sanctions (DPRK) Regulation (Cap.537AK)".to_string()),
        },
        SanctionsEntry {
            entry_id: "hk-uns-006".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec![
                "unsc_1988_taliban".to_string(),
                "hk_cap537_taliban".to_string(),
            ],
            primary_name: "Abdul Kabir".to_string(),
            aliases: vec![
                btree_alias("A. Kabir"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec!["AF".to_string()],
            date_of_birth: Some("1958-01-01".to_string()),
            programs: vec![
                "unsc_1988".to_string(),
                "cap537_taliban_regulation".to_string(),
            ],
            listing_date: Some("2001-01-25".to_string()),
            remarks: Some("UNSC TAi.024; UN Sanctions (Taliban) Regulation (Cap.537AB)".to_string()),
        },
    ]
}

/// Build a sanctions snapshot from Hong Kong UN sanctions entries.
pub fn hong_kong_sanctions_snapshot() -> SanctionsSnapshot {
    let entries = hong_kong_sanctions_entries();

    let mut counts = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
    }

    let mut sources = BTreeMap::new();
    sources.insert(
        "hk_cap537".to_string(),
        serde_json::json!({
            "name": "United Nations Sanctions Ordinance (Cap.537)",
            "url": "https://www.elegislation.gov.hk/hk/cap537",
            "authority": "Government of the Hong Kong SAR",
            "legal_basis": "United Nations Sanctions Ordinance (Cap.537)"
        }),
    );
    sources.insert(
        "unsc_consolidated".to_string(),
        serde_json::json!({
            "name": "UNSC Consolidated Sanctions List",
            "url": "https://www.un.org/securitycouncil/sanctions/un-sc-consolidated-list",
            "authority": "United Nations Security Council"
        }),
    );

    SanctionsSnapshot {
        snapshot_id: "hk-sanctions-2026Q1".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        sources,
        consolidated_counts: counts,
        delta_from_previous: None,
    }
}

// ── Compliance Deadlines ────────────────────────────────────────────────

/// Hong Kong compliance deadlines for 2026.
pub fn hong_kong_compliance_deadlines() -> Vec<ComplianceDeadline> {
    vec![
        // HKMA Prudential Returns (Quarterly)
        ComplianceDeadline {
            deadline_id: "hk-hkma-quarterly-prudential".to_string(),
            regulator_id: "hk-hkma".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly prudential return — authorized institutions (within 21 business days of quarter-end)"
                .to_string(),
            due_date: "2026-04-28".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-hkma:licensed-bank".to_string(),
                "hk-hkma:restricted-licence-bank".to_string(),
                "hk-hkma:deposit-taking-company".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "hk-hkma-capital-adequacy-quarterly".to_string(),
            regulator_id: "hk-hkma".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly capital adequacy ratio return — Banking (Capital) Rules"
                .to_string(),
            due_date: "2026-04-28".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-hkma:licensed-bank".to_string(),
                "hk-hkma:restricted-licence-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "hk-hkma-liquidity-monthly".to_string(),
            regulator_id: "hk-hkma".to_string(),
            deadline_type: "report".to_string(),
            description: "Monthly liquidity return — Banking (Liquidity) Rules"
                .to_string(),
            due_date: "2026-02-28".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-hkma:licensed-bank".to_string(),
                "hk-hkma:restricted-licence-bank".to_string(),
            ],
        },
        // SFC Annual Filing
        ComplianceDeadline {
            deadline_id: "hk-sfc-annual-return".to_string(),
            regulator_id: "hk-sfc".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual return — licensed corporations (within 4 months of FY-end, Securities and Futures (Accounts and Audit) Rules)"
                .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "hk-sfc:type-1-dealing-securities".to_string(),
                "hk-sfc:type-4-advising-securities".to_string(),
                "hk-sfc:type-9-asset-management".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "hk-sfc-financial-resources-monthly".to_string(),
            regulator_id: "hk-sfc".to_string(),
            deadline_type: "report".to_string(),
            description: "Monthly financial resources return — Securities and Futures (Financial Resources) Rules"
                .to_string(),
            due_date: "2026-02-21".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-sfc:type-1-dealing-securities".to_string(),
                "hk-sfc:type-2-dealing-futures".to_string(),
                "hk-sfc:type-3-leveraged-fx".to_string(),
            ],
        },
        // JFIU STR Reporting
        ComplianceDeadline {
            deadline_id: "hk-jfiu-str-ongoing".to_string(),
            regulator_id: "hk-jfiu".to_string(),
            deadline_type: "report".to_string(),
            description: "Suspicious Transaction Report — as soon as reasonably practicable (Drug Trafficking (Recovery of Proceeds) Ordinance Cap.405 s.25A; Organized and Serious Crimes Ordinance Cap.455 s.25A)"
                .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-hkma:licensed-bank".to_string(),
                "hk-hkma:restricted-licence-bank".to_string(),
                "hk-hkma:deposit-taking-company".to_string(),
                "hk-sfc:type-1-dealing-securities".to_string(),
                "hk-sfc:type-9-asset-management".to_string(),
                "hk-ia:authorized-insurer".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "hk-jfiu-ctr-ongoing".to_string(),
            regulator_id: "hk-jfiu".to_string(),
            deadline_type: "report".to_string(),
            description: "Cash Transaction Report — large cash transactions >= HKD 120,000 (AMLO Cap.615 s.5)"
                .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-hkma:licensed-bank".to_string(),
                "hk-sfc:type-1-dealing-securities".to_string(),
            ],
        },
        // IA Annual Filing
        ComplianceDeadline {
            deadline_id: "hk-ia-annual-return".to_string(),
            regulator_id: "hk-ia".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual return — authorized insurers (within 6 months of FY-end, Insurance Ordinance Cap.41 s.18)"
                .to_string(),
            due_date: "2026-06-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "hk-ia:authorized-insurer".to_string(),
            ],
        },
    ]
}

// ── Reporting Requirements ───────────────────────────────────────────────

/// Hong Kong reporting requirements across regulators.
pub fn hong_kong_reporting_requirements() -> Vec<ReportingRequirement> {
    vec![
        ReportingRequirement {
            report_type_id: "hk-jfiu-str".to_string(),
            name: "Suspicious Transaction Report (STR)".to_string(),
            regulator_id: "hk-jfiu".to_string(),
            applicable_to: vec![
                "licensed_bank".to_string(),
                "restricted_licence_bank".to_string(),
                "deposit_taking_company".to_string(),
                "licensed_corporation".to_string(),
                "authorized_insurer".to_string(),
                "money_service_operator".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("timing".to_string(), "as_soon_as_reasonably_practicable".to_string());
                inner.insert("submission_system".to_string(), "JFIU_STR_Online".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("JFIU STR Online System"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.jfiu.gov.hk"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("OSCO Cap.455 s.25A: fine up to HKD 50,000 and imprisonment up to 3 months"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "hk-jfiu-ctr".to_string(),
            name: "Cash Transaction Report (CTR)".to_string(),
            regulator_id: "hk-jfiu".to_string(),
            applicable_to: vec![
                "licensed_bank".to_string(),
                "money_service_operator".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("timing".to_string(), "within_reporting_period".to_string());
                inner.insert("threshold_hkd".to_string(), "120000".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("JFIU reporting form"));
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("AMLO Cap.615: fine up to HKD 50,000 and imprisonment up to 3 months"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "hk-hkma-prudential-quarterly".to_string(),
            name: "Quarterly Prudential Return".to_string(),
            regulator_id: "hk-hkma".to_string(),
            applicable_to: vec![
                "licensed_bank".to_string(),
                "restricted_licence_bank".to_string(),
                "deposit_taking_company".to_string(),
            ],
            frequency: "quarterly".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("business_days_after_quarter_end".to_string(), "21".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("HKMA Returns Submission System"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("HKMA Banking Supervision Department"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Banking Ordinance Cap.155 s.132: fine up to HKD 500,000"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "hk-sfc-annual-return".to_string(),
            name: "Annual Audited Accounts and Annual Return".to_string(),
            regulator_id: "hk-sfc".to_string(),
            applicable_to: vec![
                "licensed_corporation".to_string(),
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
                s.insert("format".to_string(), serde_json::json!("SFC WINGS e-filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://wings.sfc.hk"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("SFO Cap.571 s.156: fine up to HKD 100,000 and imprisonment up to 2 years"),
                );
                p
            },
        },
    ]
}

// ── Full Regpack Builder ─────────────────────────────────────────────────

/// Build a complete Hong Kong regpack with all content.
///
/// Assembles regulators, sanctions, deadlines, and reporting requirements
/// into a content-addressed regpack for the `hk` jurisdiction.
#[allow(clippy::type_complexity)]
pub fn build_hong_kong_regpack(
) -> PackResult<(
    Regpack,
    RegPackMetadata,
    SanctionsSnapshot,
    Vec<ComplianceDeadline>,
    Vec<ReportingRequirement>,
)> {
    let regulators = hong_kong_regulators();
    let sanctions_snapshot = hong_kong_sanctions_snapshot();
    let deadlines = hong_kong_compliance_deadlines();
    let reporting = hong_kong_reporting_requirements();

    let mut includes = BTreeMap::new();
    includes.insert(
        "regulators".to_string(),
        serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
    );
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(hong_kong_sanctions_entries().len()),
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
        regpack_id: "regpack:hk:financial:2026Q1".to_string(),
        jurisdiction_id: "hk".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "hk_cap537",
                "name": "United Nations Sanctions Ordinance (Cap.537)",
                "authority": "Government of the Hong Kong SAR"
            }),
            serde_json::json!({
                "source_id": "unsc_consolidated",
                "name": "UNSC Consolidated Sanctions List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "banking_ordinance_cap155",
                "name": "Banking Ordinance (Cap.155)",
                "authority": "Government of the Hong Kong SAR"
            }),
            serde_json::json!({
                "source_id": "sfo_cap571",
                "name": "Securities and Futures Ordinance (Cap.571)",
                "authority": "Government of the Hong Kong SAR"
            }),
            serde_json::json!({
                "source_id": "amlo_cap615",
                "name": "Anti-Money Laundering and Counter-Terrorist Financing Ordinance (Cap.615)",
                "authority": "Government of the Hong Kong SAR"
            }),
            serde_json::json!({
                "source_id": "insurance_ordinance_cap41",
                "name": "Insurance Ordinance (Cap.41)",
                "authority": "Government of the Hong Kong SAR"
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
        jurisdiction: JurisdictionId::new("hk".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Hong Kong Financial Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting))
}

/// Build a sanctions-domain-specific Hong Kong regpack.
///
/// Produces a regpack focused on the `sanctions` compliance domain,
/// containing UN Sanctions Ordinance (Cap.537) entries and UNSC
/// consolidated list entries. Separate from the `financial` domain
/// regpack which includes broader regulatory data (regulators,
/// compliance deadlines, reporting requirements).
///
/// The sanctions regpack is content-addressed independently so that
/// sanctions-list-only updates can be pushed without rebuilding the
/// full financial regpack.
pub fn build_hong_kong_sanctions_regpack(
) -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
    let sanctions_snapshot = hong_kong_sanctions_snapshot();

    let mut includes = BTreeMap::new();
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(hong_kong_sanctions_entries().len()),
    );
    includes.insert(
        "source_lists".to_string(),
        serde_json::json!(["hk_cap537", "unsc_consolidated"]),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:hk:sanctions:2026Q1".to_string(),
        jurisdiction_id: "hk".to_string(),
        domain: "sanctions".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "hk_cap537",
                "name": "United Nations Sanctions Ordinance (Cap.537)",
                "authority": "Government of the Hong Kong SAR"
            }),
            serde_json::json!({
                "source_id": "unsc_consolidated",
                "name": "UNSC Consolidated Sanctions List",
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
        jurisdiction: JurisdictionId::new("hk".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Hong Kong Sanctions Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot))
}

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hong_kong_has_four_regulators() {
        let regs = hong_kong_regulators();
        assert_eq!(regs.len(), 4);
        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"hk-hkma"));
        assert!(ids.contains(&"hk-sfc"));
        assert!(ids.contains(&"hk-ia"));
        assert!(ids.contains(&"hk-jfiu"));
    }

    #[test]
    fn all_regulators_timezone() {
        for reg in hong_kong_regulators() {
            assert_eq!(
                reg.timezone, "Asia/Hong_Kong",
                "{} wrong timezone",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn all_regulators_are_hk_jurisdiction() {
        for reg in hong_kong_regulators() {
            assert_eq!(reg.jurisdiction_id, "hk", "{} wrong jid", reg.regulator_id);
        }
    }

    #[test]
    fn jfiu_parent_is_hkma() {
        let jfiu = jfiu_regulator();
        assert_eq!(jfiu.parent_authority, Some("hk-hkma".to_string()));
    }

    #[test]
    fn sanctions_have_sources() {
        let snap = hong_kong_sanctions_snapshot();
        assert!(snap.sources.contains_key("hk_cap537"));
        assert!(snap.sources.contains_key("unsc_consolidated"));
    }

    #[test]
    fn sanctions_entries_all_have_source_lists() {
        for entry in hong_kong_sanctions_entries() {
            assert!(
                !entry.source_lists.is_empty(),
                "{} has no source_lists",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_entries_all_have_programs() {
        for entry in hong_kong_sanctions_entries() {
            assert!(
                !entry.programs.is_empty(),
                "{} has no programs",
                entry.entry_id
            );
        }
    }

    #[test]
    fn compliance_deadlines_cover_key_regulators() {
        let deadlines = hong_kong_compliance_deadlines();
        let regulator_ids: std::collections::HashSet<&str> =
            deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
        assert!(regulator_ids.contains("hk-hkma"), "missing HKMA deadlines");
        assert!(regulator_ids.contains("hk-sfc"), "missing SFC deadlines");
        assert!(regulator_ids.contains("hk-jfiu"), "missing JFIU deadlines");
        assert!(regulator_ids.contains("hk-ia"), "missing IA deadlines");
    }

    #[test]
    fn compliance_deadlines_have_unique_ids() {
        let deadlines = hong_kong_compliance_deadlines();
        let mut ids = std::collections::HashSet::new();
        for dl in &deadlines {
            assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
        }
    }

    #[test]
    fn reporting_requirements_cover_key_reports() {
        let reqs = hong_kong_reporting_requirements();
        let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
        assert!(ids.contains(&"hk-jfiu-str"), "missing STR");
        assert!(ids.contains(&"hk-jfiu-ctr"), "missing CTR");
        assert!(ids.contains(&"hk-hkma-prudential-quarterly"), "missing HKMA prudential");
        assert!(ids.contains(&"hk-sfc-annual-return"), "missing SFC annual return");
    }

    #[test]
    fn build_regpack_produces_digest() {
        let (regpack, metadata, snap, deadlines, reporting) =
            build_hong_kong_regpack().expect("build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "hk");
        assert!(regpack.digest.is_some(), "regpack should have digest");
        assert_eq!(metadata.jurisdiction_id, "hk");
        assert!(!snap.consolidated_counts.is_empty());
        assert!(!deadlines.is_empty());
        assert!(!reporting.is_empty());
    }

    #[test]
    fn build_regpack_is_deterministic() {
        let (rp1, ..) = build_hong_kong_regpack().unwrap();
        let (rp2, ..) = build_hong_kong_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "regpack digest must be deterministic"
        );
    }

    #[test]
    fn build_sanctions_regpack_produces_digest() {
        let (regpack, metadata, snap) =
            build_hong_kong_sanctions_regpack().expect("sanctions build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "hk");
        assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
        assert_eq!(metadata.domain, "sanctions");
        assert_eq!(metadata.jurisdiction_id, "hk");
        assert!(!snap.consolidated_counts.is_empty());
    }

    #[test]
    fn build_sanctions_regpack_is_deterministic() {
        let (rp1, ..) = build_hong_kong_sanctions_regpack().unwrap();
        let (rp2, ..) = build_hong_kong_sanctions_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "sanctions regpack digest must be deterministic"
        );
    }

    #[test]
    fn sanctions_regpack_digest_differs_from_financial() {
        let (financial, ..) = build_hong_kong_regpack().unwrap();
        let (sanctions, ..) = build_hong_kong_sanctions_regpack().unwrap();
        assert_ne!(
            financial.digest.as_ref().unwrap().to_hex(),
            sanctions.digest.as_ref().unwrap().to_hex(),
            "financial and sanctions regpack digests must differ"
        );
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in hong_kong_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, de.regulator_id);
            assert_eq!(reg.name, de.name);
            assert_eq!(reg.timezone, de.timezone);
        }
    }
}
