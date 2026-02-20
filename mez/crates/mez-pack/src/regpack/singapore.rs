//! Singapore-specific regpack content — real regulatory data.
//!
//! Provides Singapore-specific regulatory content for sovereign deployment:
//!   - Regulator profiles (MAS, ACRA, STRO)
//!   - Sanctions entries (UNSC consolidated + MAS-specific designations)
//!   - Compliance deadlines (MAS quarterly prudential, ACRA annual, STRO reporting)
//!   - Reporting requirements (STR, CTR, MAS prudential returns, ACRA annual returns)

use super::*;

// ── Regulator Profiles ──────────────────────────────────────────────────

/// Monetary Authority of Singapore — central bank, financial regulator, and
/// monetary authority.
pub fn mas_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "banking".to_string(),
        vec![
            "full_banks".to_string(),
            "wholesale_banks".to_string(),
            "merchant_banks".to_string(),
            "finance_companies".to_string(),
        ],
    );
    scope.insert(
        "capital_markets".to_string(),
        vec![
            "securities".to_string(),
            "futures".to_string(),
            "fund_management".to_string(),
            "reit_management".to_string(),
        ],
    );
    scope.insert(
        "insurance".to_string(),
        vec![
            "direct_insurers".to_string(),
            "reinsurers".to_string(),
            "insurance_brokers".to_string(),
        ],
    );
    scope.insert(
        "payments".to_string(),
        vec![
            "major_payment_institution".to_string(),
            "standard_payment_institution".to_string(),
            "money_changing".to_string(),
        ],
    );
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "targeted_financial_sanctions".to_string(),
            "aml_cft_supervision".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.mas.gov.sg".to_string());
    contact.insert(
        "address".to_string(),
        "10 Shenton Way, MAS Building, Singapore 079117".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("fi_registry".to_string(), true);
    api.insert("sanctions_list".to_string(), true);
    api.insert("exchange_rate_feed".to_string(), true);
    api.insert("masnet_reporting".to_string(), true);

    RegulatorProfile {
        regulator_id: "sg-mas".to_string(),
        name: "Monetary Authority of Singapore".to_string(),
        jurisdiction_id: "sg".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Singapore".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Accounting and Corporate Regulatory Authority — corporate registration
/// and governance regulator.
pub fn acra_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "corporate".to_string(),
        vec![
            "company_registration".to_string(),
            "business_registration".to_string(),
            "llp_registration".to_string(),
            "corporate_governance".to_string(),
        ],
    );
    scope.insert(
        "accounting".to_string(),
        vec![
            "public_accountants".to_string(),
            "accounting_entities".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert(
        "website".to_string(),
        "https://www.acra.gov.sg".to_string(),
    );
    contact.insert(
        "address".to_string(),
        "55 Newton Road, #03-01, Revenue House, Singapore 307987".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("bizfile_portal".to_string(), true);
    api.insert("company_search".to_string(), true);
    api.insert("annual_filing".to_string(), true);

    RegulatorProfile {
        regulator_id: "sg-acra".to_string(),
        name: "Accounting and Corporate Regulatory Authority".to_string(),
        jurisdiction_id: "sg".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Singapore".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Suspicious Transaction Reporting Office — Singapore's financial
/// intelligence unit under the Singapore Police Force.
pub fn stro_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "suspicious_transaction_reports".to_string(),
            "cash_transaction_reports".to_string(),
            "cross_border_cash_reports".to_string(),
            "financial_intelligence".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert(
        "website".to_string(),
        "https://www.police.gov.sg/stro".to_string(),
    );
    contact.insert(
        "address".to_string(),
        "Police Cantonment Complex, 391 New Bridge Road, Singapore 088762".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("sonar_reporting".to_string(), true);
    api.insert("str_submission".to_string(), true);

    RegulatorProfile {
        regulator_id: "sg-stro".to_string(),
        name: "Suspicious Transaction Reporting Office".to_string(),
        jurisdiction_id: "sg".to_string(),
        parent_authority: Some("sg-mas".to_string()),
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Singapore".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// All Singapore regulatory authorities relevant to regpack domains.
pub fn singapore_regulators() -> Vec<RegulatorProfile> {
    vec![mas_regulator(), acra_regulator(), stro_regulator()]
}

// ── Sanctions Entries ────────────────────────────────────────────────────
//
// Representative entries from Singapore's targeted sanctions regime.
// Sources: UNSC Consolidated Sanctions List,
//          MAS Targeted Financial Sanctions (Terrorism Financing)
//          (Suppression of Financing of Terrorism) Regulations,
//          MAS (Monetary Authority of Singapore) (Sanctions and Freezing
//          of Assets of Persons — DPRK) Regulations.
//
// NOTE: These are publicly available, gazette-notified designations.
// Real deployment must pull from live MAS notices and UNSC XML feed.

/// Representative Singapore sanctions entries for regpack content.
pub fn singapore_sanctions_entries() -> Vec<SanctionsEntry> {
    vec![
        SanctionsEntry {
            entry_id: "sg-mas-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "mas_tfs".to_string(),
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
                "mas_tfs_terrorism".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2001-10-15".to_string()),
            remarks: Some("UNSC QDe.004; MAS TFS Regulations".to_string()),
        },
        SanctionsEntry {
            entry_id: "sg-mas-002".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "mas_tfs".to_string(),
                "unsc_1989".to_string(),
            ],
            primary_name: "Islamic State / Daesh".to_string(),
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
                "mas_tfs_terrorism".to_string(),
                "unsc_2253".to_string(),
            ],
            listing_date: Some("2015-07-01".to_string()),
            remarks: Some("UNSC; MAS TFS Regulations".to_string()),
        },
        SanctionsEntry {
            entry_id: "sg-mas-003".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "mas_tfs".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Jemaah Islamiyah".to_string(),
            aliases: vec![
                btree_alias("JI"),
                btree_alias("Jemaah Islamiah"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Southeast Asia")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "mas_tfs_terrorism".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2002-10-25".to_string()),
            remarks: Some("UNSC QDe.092; MAS TFS Regulations — regional threat".to_string()),
        },
        SanctionsEntry {
            entry_id: "sg-mas-004".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "mas_dprk_sanctions".to_string(),
                "unsc_1718".to_string(),
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
                "mas_dprk_sanctions".to_string(),
                "unsc_1718".to_string(),
            ],
            listing_date: Some("2009-04-24".to_string()),
            remarks: Some("UNSC; MAS DPRK Sanctions Regulations — arms proliferation".to_string()),
        },
        SanctionsEntry {
            entry_id: "sg-mas-005".to_string(),
            entry_type: "individual".to_string(),
            source_lists: vec![
                "mas_tfs".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Hambali".to_string(),
            aliases: vec![
                btree_alias("Riduan Isamuddin"),
                btree_alias("Encep Nurjaman"),
            ],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec!["Indonesian".to_string()],
            date_of_birth: Some("1964-04-04".to_string()),
            programs: vec![
                "mas_tfs_terrorism".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2003-01-28".to_string()),
            remarks: Some("UNSC QDi.070; JI operations chief".to_string()),
        },
        SanctionsEntry {
            entry_id: "sg-mas-006".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "mas_dprk_sanctions".to_string(),
                "unsc_1718".to_string(),
            ],
            primary_name: "Reconnaissance General Bureau".to_string(),
            aliases: vec![
                btree_alias("RGB"),
                btree_alias("Chongch'alch'ong"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Pyongyang, DPRK")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "mas_dprk_sanctions".to_string(),
                "unsc_1718".to_string(),
            ],
            listing_date: Some("2013-03-07".to_string()),
            remarks: Some("UNSC; MAS DPRK Sanctions Regulations — intelligence entity".to_string()),
        },
    ]
}

/// Build a sanctions snapshot from Singapore entries.
pub fn singapore_sanctions_snapshot() -> SanctionsSnapshot {
    let entries = singapore_sanctions_entries();

    let mut counts = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
    }

    let mut sources = BTreeMap::new();
    sources.insert(
        "mas_tfs".to_string(),
        serde_json::json!({
            "name": "MAS Targeted Financial Sanctions — Terrorism (Suppression of Financing) Regulations",
            "url": "https://www.mas.gov.sg/regulation/regulations/mas-regulations-on-targeted-financial-sanctions",
            "authority": "Monetary Authority of Singapore",
            "legal_basis": "Terrorism (Suppression of Financing) Act (Cap 325)"
        }),
    );
    sources.insert(
        "mas_dprk_sanctions".to_string(),
        serde_json::json!({
            "name": "MAS (Sanctions and Freezing of Assets of Persons — DPRK) Regulations",
            "url": "https://www.mas.gov.sg/regulation/regulations/mas-dprk-regulations",
            "authority": "Monetary Authority of Singapore",
            "legal_basis": "Monetary Authority of Singapore Act (Cap 186)"
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
    sources.insert(
        "unsc_1718".to_string(),
        serde_json::json!({
            "name": "UNSC 1718 DPRK Sanctions Committee List",
            "url": "https://www.un.org/securitycouncil/sanctions/1718",
            "authority": "United Nations Security Council"
        }),
    );

    SanctionsSnapshot {
        snapshot_id: "sg-sanctions-2026Q1".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        sources,
        consolidated_counts: counts,
        delta_from_previous: None,
    }
}

// ── Compliance Deadlines ────────────────────────────────────────────────

/// Singapore compliance deadlines for FY 2026.
pub fn singapore_compliance_deadlines() -> Vec<ComplianceDeadline> {
    vec![
        // MAS Quarterly Prudential
        ComplianceDeadline {
            deadline_id: "sg-mas-quarterly-prudential".to_string(),
            regulator_id: "sg-mas".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly prudential return — banks (MAS Notice 610, within 14 days of quarter-end)".to_string(),
            due_date: "2026-04-14".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:full-bank".to_string(),
                "sg-mas:wholesale-bank".to_string(),
                "sg-mas:merchant-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "sg-mas-annual-audited".to_string(),
            regulator_id: "sg-mas".to_string(),
            deadline_type: "report".to_string(),
            description: "Annual audited financial statements — banks (within 5 months of FY-end per MAS Notice 610)".to_string(),
            due_date: "2026-05-31".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:full-bank".to_string(),
                "sg-mas:wholesale-bank".to_string(),
                "sg-mas:merchant-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "sg-mas-car-quarterly".to_string(),
            regulator_id: "sg-mas".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly Capital Adequacy Ratio return — banks (MAS Notice 637)".to_string(),
            due_date: "2026-04-14".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:full-bank".to_string(),
                "sg-mas:wholesale-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "sg-mas-pi-annual".to_string(),
            regulator_id: "sg-mas".to_string(),
            deadline_type: "report".to_string(),
            description: "Annual compliance report — payment institutions (Payment Services Act 2019)".to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 14,
            applicable_license_types: vec![
                "sg-mas:major-payment-institution".to_string(),
                "sg-mas:standard-payment-institution".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "sg-mas-cmsl-annual".to_string(),
            regulator_id: "sg-mas".to_string(),
            deadline_type: "report".to_string(),
            description: "Annual compliance report — CMS licence holders (Securities and Futures Act)".to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:cms-licence".to_string(),
                "sg-mas:fund-management".to_string(),
            ],
        },
        // ACRA Annual Filing
        ComplianceDeadline {
            deadline_id: "sg-acra-annual-return".to_string(),
            regulator_id: "sg-acra".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual return — companies (within 30 days of AGM, Companies Act s.197)".to_string(),
            due_date: "2026-07-30".to_string(),
            grace_period_days: 14,
            applicable_license_types: vec!["sg-acra:company-registration".to_string()],
        },
        ComplianceDeadline {
            deadline_id: "sg-acra-financial-statements".to_string(),
            regulator_id: "sg-acra".to_string(),
            deadline_type: "filing".to_string(),
            description: "Audited financial statements — companies (within 6 months of FY-end, Companies Act s.175)".to_string(),
            due_date: "2026-06-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-acra:company-registration".to_string(),
            ],
        },
        // STRO Reporting
        ComplianceDeadline {
            deadline_id: "sg-stro-str-ongoing".to_string(),
            regulator_id: "sg-stro".to_string(),
            deadline_type: "report".to_string(),
            description: "Suspicious Transaction Report — as soon as practicable (CDSA s.39, TSOFA s.8)".to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:full-bank".to_string(),
                "sg-mas:wholesale-bank".to_string(),
                "sg-mas:major-payment-institution".to_string(),
                "sg-mas:standard-payment-institution".to_string(),
                "sg-mas:cms-licence".to_string(),
                "sg-mas:insurance".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "sg-stro-ctr-ongoing".to_string(),
            regulator_id: "sg-stro".to_string(),
            deadline_type: "report".to_string(),
            description: "Cash Transaction Report — for cash transactions >= SGD 20,000 (within 15 days)".to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "sg-mas:full-bank".to_string(),
                "sg-mas:wholesale-bank".to_string(),
                "sg-mas:money-changing".to_string(),
            ],
        },
    ]
}

// ── Reporting Requirements ───────────────────────────────────────────────

/// Singapore reporting requirements across regulators.
pub fn singapore_reporting_requirements() -> Vec<ReportingRequirement> {
    vec![
        ReportingRequirement {
            report_type_id: "sg-stro-str".to_string(),
            name: "Suspicious Transaction Report (STR)".to_string(),
            regulator_id: "sg-stro".to_string(),
            applicable_to: vec![
                "full_bank".to_string(),
                "wholesale_bank".to_string(),
                "merchant_bank".to_string(),
                "major_payment_institution".to_string(),
                "standard_payment_institution".to_string(),
                "cms_licence_holder".to_string(),
                "insurer".to_string(),
                "money_changer".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("timing".to_string(), "as_soon_as_practicable".to_string());
                inner.insert("submission_system".to_string(), "SONAR".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("SONAR electronic filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://sonar.spf.gov.sg"),
                );
                s.insert(
                    "legal_basis".to_string(),
                    serde_json::json!("Corruption, Drug Trafficking and Other Serious Crimes (Confiscation of Benefits) Act (CDSA) s.39; Terrorism (Suppression of Financing) Act (TSOFA) s.8"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("CDSA s.39: fine up to SGD 20,000 or imprisonment up to 2 years"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "sg-stro-ctr".to_string(),
            name: "Cash Transaction Report (CTR)".to_string(),
            regulator_id: "sg-stro".to_string(),
            applicable_to: vec![
                "full_bank".to_string(),
                "wholesale_bank".to_string(),
                "money_changer".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_transaction".to_string(), "15".to_string());
                inner.insert("threshold_sgd".to_string(), "20000".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("SONAR electronic filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://sonar.spf.gov.sg"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("MAS Notice — regulatory action for non-compliance"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "sg-mas-prudential-quarterly".to_string(),
            name: "Quarterly Prudential Return".to_string(),
            regulator_id: "sg-mas".to_string(),
            applicable_to: vec![
                "full_bank".to_string(),
                "wholesale_bank".to_string(),
                "merchant_bank".to_string(),
            ],
            frequency: "quarterly".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_after_quarter_end".to_string(), "14".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("MASNet electronic submission"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("MASNet — MAS electronic submission gateway"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Banking Act s.71: fine up to SGD 100,000 or imprisonment up to 3 years"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "sg-acra-annual-return".to_string(),
            name: "Annual Return".to_string(),
            regulator_id: "sg-acra".to_string(),
            applicable_to: vec![
                "company".to_string(),
                "llp".to_string(),
            ],
            frequency: "annual".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_after_agm".to_string(), "30".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("BizFile+ electronic filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.bizfile.gov.sg"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Companies Act s.197: fine up to SGD 5,000; additional SGD 50/day for continuing offence"),
                );
                p
            },
        },
    ]
}

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

// ── Full Regpack Builder ─────────────────────────────────────────────────

/// Build a complete Singapore regpack with all content.
///
/// Assembles regulators, sanctions, deadlines, and reporting requirements
/// into a content-addressed regpack for the `sg` jurisdiction.
#[allow(clippy::type_complexity)]
pub fn build_singapore_regpack() -> PackResult<(
    Regpack,
    RegPackMetadata,
    SanctionsSnapshot,
    Vec<ComplianceDeadline>,
    Vec<ReportingRequirement>,
)> {
    let regulators = singapore_regulators();
    let sanctions_snapshot = singapore_sanctions_snapshot();
    let deadlines = singapore_compliance_deadlines();
    let reporting = singapore_reporting_requirements();

    let mut includes = BTreeMap::new();
    includes.insert(
        "regulators".to_string(),
        serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
    );
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(singapore_sanctions_entries().len()),
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
        regpack_id: "regpack:sg:financial:2026Q1".to_string(),
        jurisdiction_id: "sg".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "mas_tfs",
                "name": "MAS Targeted Financial Sanctions Regulations",
                "authority": "Monetary Authority of Singapore"
            }),
            serde_json::json!({
                "source_id": "mas_dprk_sanctions",
                "name": "MAS (Sanctions — DPRK) Regulations",
                "authority": "Monetary Authority of Singapore"
            }),
            serde_json::json!({
                "source_id": "unsc_1267",
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "unsc_1718",
                "name": "UNSC 1718 DPRK Sanctions Committee List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "banking_act",
                "name": "Banking Act (Cap 19)",
                "authority": "Government of Singapore"
            }),
            serde_json::json!({
                "source_id": "companies_act",
                "name": "Companies Act (Cap 50)",
                "authority": "Government of Singapore"
            }),
            serde_json::json!({
                "source_id": "psa_2019",
                "name": "Payment Services Act 2019",
                "authority": "Government of Singapore"
            }),
            serde_json::json!({
                "source_id": "cdsa",
                "name": "Corruption, Drug Trafficking and Other Serious Crimes (Confiscation of Benefits) Act",
                "authority": "Government of Singapore"
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
        jurisdiction: JurisdictionId::new("sg".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Singapore Financial Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting))
}

/// Build a sanctions-domain-specific Singapore regpack.
///
/// Produces a regpack focused on the `sanctions` compliance domain,
/// containing the MAS targeted financial sanctions designations and UNSC
/// consolidated list entries. Separate from the `financial` domain
/// regpack which includes broader regulatory data (regulators,
/// compliance deadlines, reporting requirements).
///
/// The sanctions regpack is content-addressed independently so that
/// sanctions-list-only updates can be pushed without rebuilding the
/// full financial regpack.
pub fn build_singapore_sanctions_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
    let sanctions_snapshot = singapore_sanctions_snapshot();

    let mut includes = BTreeMap::new();
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(singapore_sanctions_entries().len()),
    );
    includes.insert(
        "source_lists".to_string(),
        serde_json::json!(["mas_tfs", "mas_dprk_sanctions", "unsc_1267", "unsc_1718"]),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:sg:sanctions:2026Q1".to_string(),
        jurisdiction_id: "sg".to_string(),
        domain: "sanctions".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "mas_tfs",
                "name": "MAS Targeted Financial Sanctions Regulations",
                "authority": "Monetary Authority of Singapore"
            }),
            serde_json::json!({
                "source_id": "mas_dprk_sanctions",
                "name": "MAS (Sanctions — DPRK) Regulations",
                "authority": "Monetary Authority of Singapore"
            }),
            serde_json::json!({
                "source_id": "unsc_1267",
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "unsc_1718",
                "name": "UNSC 1718 DPRK Sanctions Committee List",
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
        None,  // No regulators — sanctions-only domain
        None,  // No deadlines — sanctions-only domain
    )?;

    let regpack = Regpack {
        jurisdiction: JurisdictionId::new("sg".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Singapore Sanctions Regulatory Pack — 2026 Q1".to_string(),
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
    fn singapore_has_three_regulators() {
        let regs = singapore_regulators();
        assert_eq!(regs.len(), 3);
        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"sg-mas"));
        assert!(ids.contains(&"sg-acra"));
        assert!(ids.contains(&"sg-stro"));
    }

    #[test]
    fn all_regulators_timezone() {
        for reg in singapore_regulators() {
            assert_eq!(
                reg.timezone, "Asia/Singapore",
                "{} wrong tz",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn sanctions_have_sources() {
        for entry in singapore_sanctions_entries() {
            assert!(
                !entry.source_lists.is_empty(),
                "{} has no source_lists",
                entry.entry_id
            );
        }
    }

    #[test]
    fn build_regpack_produces_digest() {
        let (regpack, metadata, snap, deadlines, reporting) =
            build_singapore_regpack().expect("build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "sg");
        assert!(regpack.digest.is_some(), "regpack should have digest");
        assert_eq!(metadata.jurisdiction_id, "sg");
        assert!(!snap.consolidated_counts.is_empty());
        assert!(!deadlines.is_empty());
        assert!(!reporting.is_empty());

        // Deterministic
        let (rp2, ..) = build_singapore_regpack().unwrap();
        assert_eq!(
            regpack.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "regpack digest must be deterministic"
        );
    }

    #[test]
    fn build_sanctions_regpack_produces_digest() {
        let (regpack, metadata, snap) =
            build_singapore_sanctions_regpack().expect("sanctions build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "sg");
        assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
        assert_eq!(metadata.domain, "sanctions");
        assert_eq!(metadata.jurisdiction_id, "sg");
        assert!(!snap.consolidated_counts.is_empty());

        // Deterministic
        let (rp2, ..) = build_singapore_sanctions_regpack().unwrap();
        assert_eq!(
            regpack.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "sanctions regpack digest must be deterministic"
        );
    }

    #[test]
    fn all_regulators_are_singapore_jurisdiction() {
        for reg in singapore_regulators() {
            assert_eq!(reg.jurisdiction_id, "sg", "{} wrong jid", reg.regulator_id);
        }
    }

    #[test]
    fn stro_parent_is_mas() {
        let stro = stro_regulator();
        assert_eq!(stro.parent_authority, Some("sg-mas".to_string()));
    }

    #[test]
    fn sanctions_entries_all_have_programs() {
        for entry in singapore_sanctions_entries() {
            assert!(
                !entry.programs.is_empty(),
                "{} has no programs",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_snapshot_has_sources() {
        let snap = singapore_sanctions_snapshot();
        assert!(snap.sources.contains_key("mas_tfs"));
        assert!(snap.sources.contains_key("unsc_1267"));
    }

    #[test]
    fn compliance_deadlines_cover_all_regulators() {
        let deadlines = singapore_compliance_deadlines();
        let regulator_ids: std::collections::HashSet<&str> =
            deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
        assert!(regulator_ids.contains("sg-mas"), "missing MAS deadlines");
        assert!(regulator_ids.contains("sg-acra"), "missing ACRA deadlines");
        assert!(regulator_ids.contains("sg-stro"), "missing STRO deadlines");
    }

    #[test]
    fn compliance_deadlines_have_unique_ids() {
        let deadlines = singapore_compliance_deadlines();
        let mut ids = std::collections::HashSet::new();
        for dl in &deadlines {
            assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
        }
    }

    #[test]
    fn reporting_requirements_cover_key_reports() {
        let reqs = singapore_reporting_requirements();
        let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
        assert!(ids.contains(&"sg-stro-str"), "missing STR");
        assert!(ids.contains(&"sg-stro-ctr"), "missing CTR");
        assert!(ids.contains(&"sg-mas-prudential-quarterly"), "missing prudential");
        assert!(ids.contains(&"sg-acra-annual-return"), "missing ACRA annual");
    }

    #[test]
    fn sanctions_regpack_digest_differs_from_financial() {
        let (financial, ..) = build_singapore_regpack().unwrap();
        let (sanctions, ..) = build_singapore_sanctions_regpack().unwrap();
        assert_ne!(
            financial.digest.as_ref().unwrap().to_hex(),
            sanctions.digest.as_ref().unwrap().to_hex(),
            "financial and sanctions regpack digests must differ"
        );
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in singapore_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, de.regulator_id);
            assert_eq!(reg.jurisdiction_id, de.jurisdiction_id);
        }
    }
}
