//! Cayman Islands-specific regpack content — real regulatory data.
//!
//! Provides Cayman Islands-specific regulatory content:
//!   - Regulator profiles (CIMA, Registrar of Companies, FRA)
//!   - Sanctions entries (AML Regulations, UNSC consolidated lists via UK/Crown dependency)
//!   - Compliance deadlines (CIMA licensing, CIMA quarterly, FRA STR, Registrar annual return)
//!   - Reporting requirements (STR/SAR, annual audited statements, CIMA prudential returns)

use super::*;

// ── Regulator Profiles ──────────────────────────────────────────────────

/// Cayman Islands Monetary Authority — primary financial regulator.
pub fn cima_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "banking".to_string(),
        vec![
            "banks_and_trust_companies".to_string(),
            "cooperative_societies".to_string(),
            "building_societies".to_string(),
        ],
    );
    scope.insert(
        "insurance".to_string(),
        vec![
            "domestic_insurance".to_string(),
            "captive_insurance".to_string(),
            "insurance_managers".to_string(),
        ],
    );
    scope.insert(
        "investments".to_string(),
        vec![
            "mutual_funds".to_string(),
            "securities_investment_business".to_string(),
            "fund_administrators".to_string(),
        ],
    );
    scope.insert(
        "money_services".to_string(),
        vec![
            "money_services_business".to_string(),
            "virtual_asset_service_providers".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.cima.ky".to_string());
    contact.insert(
        "address".to_string(),
        "80 Shedden Road, George Town, Grand Cayman, Cayman Islands".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("entity_search".to_string(), true);
    api.insert("regulatory_filings".to_string(), true);
    api.insert("licensing_portal".to_string(), true);

    RegulatorProfile {
        regulator_id: "ky-cima".to_string(),
        name: "Cayman Islands Monetary Authority".to_string(),
        jurisdiction_id: "ky".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "America/Cayman".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Registrar of Companies — corporate registry for the Cayman Islands.
pub fn registrar_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "corporate".to_string(),
        vec![
            "company_registration".to_string(),
            "exempted_limited_partnerships".to_string(),
            "limited_liability_companies".to_string(),
            "foreign_company_registration".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert(
        "website".to_string(),
        "https://www.ciregistry.ky".to_string(),
    );
    contact.insert(
        "address".to_string(),
        "Government Administration Building, George Town, Grand Cayman, Cayman Islands"
            .to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("company_search".to_string(), true);
    api.insert("filing_status".to_string(), true);
    api.insert("online_registration".to_string(), true);

    RegulatorProfile {
        regulator_id: "ky-registrar".to_string(),
        name: "Registrar of Companies".to_string(),
        jurisdiction_id: "ky".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "America/Cayman".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Financial Reporting Authority — Cayman Islands' financial intelligence unit (FIU).
pub fn fra_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "suspicious_activity_reports".to_string(),
            "suspicious_transaction_reports".to_string(),
            "targeted_financial_sanctions".to_string(),
            "mutual_legal_assistance".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert(
        "website".to_string(),
        "https://www.fra.gov.ky".to_string(),
    );
    contact.insert(
        "address".to_string(),
        "Government Administration Building, George Town, Grand Cayman, Cayman Islands"
            .to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("sar_reporting".to_string(), true);
    api.insert("sanctions_query".to_string(), true);

    RegulatorProfile {
        regulator_id: "ky-fra".to_string(),
        name: "Financial Reporting Authority".to_string(),
        jurisdiction_id: "ky".to_string(),
        parent_authority: Some("ky-cima".to_string()),
        scope,
        contact,
        api_capabilities: api,
        timezone: "America/Cayman".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// All Cayman Islands regulatory authorities relevant to regpack domains.
pub fn cayman_regulators() -> Vec<RegulatorProfile> {
    vec![
        cima_regulator(),
        registrar_regulator(),
        fra_regulator(),
    ]
}

// ── Sanctions Entries ────────────────────────────────────────────────────
//
// Representative entries from Cayman Islands' sanctions regime.
// Sources: Anti-Money Laundering Regulations (as revised),
//          UNSC Consolidated Sanctions List (applicable via UK/Crown dependency),
//          UK HM Treasury Consolidated List (extended to Crown Dependencies).
//
// NOTE: These are publicly available designations. Real deployment
// must pull from live UNSC XML feed and UK HM Treasury consolidated list.

/// Representative Cayman Islands sanctions entries for regpack content.
pub fn cayman_sanctions_entries() -> Vec<SanctionsEntry> {
    vec![
        SanctionsEntry {
            entry_id: "ky-aml-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "uk_hmt_consolidated".to_string(),
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
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2001-10-15".to_string()),
            remarks: Some("UNSC QDe.004; applicable via UK/Crown dependency extension".to_string()),
        },
        SanctionsEntry {
            entry_id: "ky-aml-002".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1989".to_string(),
                "uk_hmt_consolidated".to_string(),
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
                "unsc_2253".to_string(),
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2014-05-29".to_string()),
            remarks: Some("UNSC; applicable via UK/Crown dependency extension".to_string()),
        },
        SanctionsEntry {
            entry_id: "ky-aml-003".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "uk_hmt_consolidated".to_string(),
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
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2001-01-25".to_string()),
            remarks: Some("UNSC TAe.001; applicable via UK/Crown dependency extension".to_string()),
        },
        SanctionsEntry {
            entry_id: "ky-aml-004".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "uk_hmt_consolidated".to_string(),
            ],
            primary_name: "Boko Haram".to_string(),
            aliases: vec![
                btree_alias("Jama'atu Ahlis Sunna Lidda'awati wal-Jihad"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Nigeria")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_2083".to_string(),
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2014-05-22".to_string()),
            remarks: Some("UNSC QDe.138; applicable via UK/Crown dependency extension".to_string()),
        },
        SanctionsEntry {
            entry_id: "ky-aml-005".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "unsc_1267".to_string(),
                "uk_hmt_consolidated".to_string(),
            ],
            primary_name: "Al-Shabaab".to_string(),
            aliases: vec![
                btree_alias("Harakat Shabaab al-Mujahidin"),
                btree_alias("Hizbul Shabaab"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Somalia")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "unsc_751".to_string(),
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2012-02-29".to_string()),
            remarks: Some("UNSC SOe.001; applicable via UK/Crown dependency extension".to_string()),
        },
        SanctionsEntry {
            entry_id: "ky-aml-006".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "uk_hmt_consolidated".to_string(),
            ],
            primary_name: "Hezbollah Military Wing".to_string(),
            aliases: vec![
                btree_alias("Hizballah Military Wing"),
                btree_alias("Hizbollah Military Wing"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Lebanon")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "uk_counter_terrorism".to_string(),
                "cayman_aml_regulations".to_string(),
            ],
            listing_date: Some("2019-03-01".to_string()),
            remarks: Some("UK HM Treasury designation; extended to Cayman Islands".to_string()),
        },
    ]
}

/// Build a sanctions snapshot from Cayman Islands entries.
pub fn cayman_sanctions_snapshot() -> SanctionsSnapshot {
    let entries = cayman_sanctions_entries();

    let mut counts = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
    }

    let mut sources = BTreeMap::new();
    sources.insert(
        "unsc_consolidated".to_string(),
        serde_json::json!({
            "name": "UNSC Consolidated Sanctions List",
            "url": "https://www.un.org/securitycouncil/sanctions/information",
            "authority": "United Nations Security Council",
            "legal_basis": "Applicable to Cayman Islands via UK/Crown dependency"
        }),
    );
    sources.insert(
        "uk_hmt_consolidated".to_string(),
        serde_json::json!({
            "name": "UK HM Treasury Consolidated List",
            "url": "https://www.gov.uk/government/publications/financial-sanctions-consolidated-list-of-targets",
            "authority": "HM Treasury, United Kingdom",
            "legal_basis": "Extended to Cayman Islands as Crown dependency"
        }),
    );
    sources.insert(
        "cayman_aml_regulations".to_string(),
        serde_json::json!({
            "name": "Anti-Money Laundering Regulations (as revised)",
            "url": "https://www.cima.ky/anti-money-laundering",
            "authority": "Cayman Islands Government / CIMA",
            "legal_basis": "Proceeds of Crime Act (as revised), Anti-Money Laundering Regulations"
        }),
    );

    SanctionsSnapshot {
        snapshot_id: "ky-sanctions-2026Q1".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        sources,
        consolidated_counts: counts,
        delta_from_previous: None,
    }
}

// ── Compliance Deadlines ────────────────────────────────────────────────

/// Cayman Islands compliance deadlines.
pub fn cayman_compliance_deadlines() -> Vec<ComplianceDeadline> {
    vec![
        // CIMA Annual Licensing Fee
        ComplianceDeadline {
            deadline_id: "ky-cima-annual-license-fee".to_string(),
            regulator_id: "ky-cima".to_string(),
            deadline_type: "payment".to_string(),
            description: "Annual CIMA licensing fee — all regulated entities (due 15 January)"
                .to_string(),
            due_date: "2026-01-15".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "ky-cima:bank-and-trust".to_string(),
                "ky-cima:insurance".to_string(),
                "ky-cima:mutual-fund".to_string(),
                "ky-cima:securities-investment".to_string(),
                "ky-cima:money-services".to_string(),
                "ky-cima:vasp".to_string(),
            ],
        },
        // CIMA Quarterly Regulatory Filing
        ComplianceDeadline {
            deadline_id: "ky-cima-quarterly-filing".to_string(),
            regulator_id: "ky-cima".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Quarterly prudential/regulatory filing — banks and trust companies (within 30 days of quarter-end)"
                    .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ky-cima:bank-and-trust".to_string(),
                "ky-cima:insurance".to_string(),
            ],
        },
        // FRA STR Reporting
        ComplianceDeadline {
            deadline_id: "ky-fra-str-ongoing".to_string(),
            regulator_id: "ky-fra".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Suspicious Activity/Transaction Report — as soon as practicable upon suspicion (Proceeds of Crime Act s.136)"
                    .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ky-cima:bank-and-trust".to_string(),
                "ky-cima:insurance".to_string(),
                "ky-cima:mutual-fund".to_string(),
                "ky-cima:securities-investment".to_string(),
                "ky-cima:money-services".to_string(),
                "ky-cima:vasp".to_string(),
            ],
        },
        // Registrar Annual Return
        ComplianceDeadline {
            deadline_id: "ky-registrar-annual-return".to_string(),
            regulator_id: "ky-registrar".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual return — exempted companies (due 31 March, Companies Act s.166)"
                    .to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "ky-registrar:exempted-company".to_string(),
                "ky-registrar:exempted-limited-partnership".to_string(),
                "ky-registrar:llc".to_string(),
            ],
        },
        // CIMA Annual Audited Financial Statements
        ComplianceDeadline {
            deadline_id: "ky-cima-annual-audited".to_string(),
            regulator_id: "ky-cima".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Annual audited financial statements — banks and trust companies (within 6 months of FY-end)"
                    .to_string(),
            due_date: "2026-06-30".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "ky-cima:bank-and-trust".to_string(),
                "ky-cima:securities-investment".to_string(),
            ],
        },
        // CIMA Annual Compliance Return
        ComplianceDeadline {
            deadline_id: "ky-cima-compliance-return".to_string(),
            regulator_id: "ky-cima".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual compliance return — CIMA-regulated entities (AML/CFT compliance confirmation)"
                    .to_string(),
            due_date: "2026-03-31".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "ky-cima:bank-and-trust".to_string(),
                "ky-cima:insurance".to_string(),
                "ky-cima:mutual-fund".to_string(),
                "ky-cima:securities-investment".to_string(),
                "ky-cima:money-services".to_string(),
                "ky-cima:vasp".to_string(),
            ],
        },
    ]
}

// ── Reporting Requirements ───────────────────────────────────────────────

/// Cayman Islands reporting requirements across regulators.
pub fn cayman_reporting_requirements() -> Vec<ReportingRequirement> {
    vec![
        ReportingRequirement {
            report_type_id: "ky-fra-str".to_string(),
            name: "Suspicious Activity Report (SAR/STR)".to_string(),
            regulator_id: "ky-fra".to_string(),
            applicable_to: vec![
                "bank_and_trust_company".to_string(),
                "insurance_company".to_string(),
                "mutual_fund".to_string(),
                "securities_investment_business".to_string(),
                "money_services_business".to_string(),
                "vasp".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_suspicion".to_string(), "as_soon_as_practicable".to_string());
                inner.insert("submission_system".to_string(), "FRA Portal".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("FRA electronic filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.fra.gov.ky/reporting"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Proceeds of Crime Act (as revised) s.136: criminal offence, imprisonment up to 2 years or fine"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ky-cima-annual-audited-statements".to_string(),
            name: "Annual Audited Financial Statements".to_string(),
            regulator_id: "ky-cima".to_string(),
            applicable_to: vec![
                "bank_and_trust_company".to_string(),
                "securities_investment_business".to_string(),
                "insurance_company".to_string(),
            ],
            frequency: "annual".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("months_after_fy_end".to_string(), "6".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("CIMA regulatory filing system"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.cima.ky/regulatory-filings"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("CIMA Administrative Fines: up to CI$1,000,000 per offence"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ky-cima-prudential-return".to_string(),
            name: "CIMA Prudential Return".to_string(),
            regulator_id: "ky-cima".to_string(),
            applicable_to: vec![
                "bank_and_trust_company".to_string(),
                "insurance_company".to_string(),
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
                s.insert("format".to_string(), serde_json::json!("CIMA regulatory filing system"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.cima.ky/regulatory-filings"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("CIMA Administrative Fines: escalating daily penalty"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "ky-registrar-annual-return".to_string(),
            name: "Annual Return to Registrar".to_string(),
            regulator_id: "ky-registrar".to_string(),
            applicable_to: vec![
                "exempted_company".to_string(),
                "exempted_limited_partnership".to_string(),
                "llc".to_string(),
            ],
            frequency: "annual".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("due_date".to_string(), "31 March".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("Registrar online filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://www.ciregistry.ky/online-services"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Companies Act (as revised) s.166: late filing surcharge plus potential striking off"),
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

/// Build a complete Cayman Islands regpack with all content.
///
/// Assembles regulators, sanctions, deadlines, and reporting requirements
/// into a content-addressed regpack for the `ky` jurisdiction.
#[allow(clippy::type_complexity)]
pub fn build_cayman_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot, Vec<ComplianceDeadline>, Vec<ReportingRequirement>)> {
    let regulators = cayman_regulators();
    let sanctions_snapshot = cayman_sanctions_snapshot();
    let deadlines = cayman_compliance_deadlines();
    let reporting = cayman_reporting_requirements();

    let mut includes = BTreeMap::new();
    includes.insert(
        "regulators".to_string(),
        serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
    );
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(cayman_sanctions_entries().len()),
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
        regpack_id: "regpack:ky:financial:2026Q1".to_string(),
        jurisdiction_id: "ky".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "unsc_consolidated",
                "name": "UNSC Consolidated Sanctions List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "uk_hmt_consolidated",
                "name": "UK HM Treasury Consolidated List",
                "authority": "HM Treasury, United Kingdom"
            }),
            serde_json::json!({
                "source_id": "cayman_aml_regulations",
                "name": "Anti-Money Laundering Regulations (as revised)",
                "authority": "Cayman Islands Government / CIMA"
            }),
            serde_json::json!({
                "source_id": "proceeds_of_crime_act",
                "name": "Proceeds of Crime Act (as revised)",
                "authority": "Cayman Islands Government"
            }),
            serde_json::json!({
                "source_id": "companies_act",
                "name": "Companies Act (as revised)",
                "authority": "Cayman Islands Government"
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
        jurisdiction: JurisdictionId::new("ky".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Cayman Islands Financial Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting))
}

/// Build a sanctions-domain-specific Cayman Islands regpack.
///
/// Produces a regpack focused on the `sanctions` compliance domain,
/// containing UNSC consolidated list entries and UK HM Treasury
/// designations applicable to the Cayman Islands as a UK Crown
/// dependency. Separate from the `financial` domain regpack which
/// includes broader regulatory data (regulators, compliance deadlines,
/// reporting requirements).
///
/// The sanctions regpack is content-addressed independently so that
/// sanctions-list-only updates can be pushed without rebuilding the
/// full financial regpack.
pub fn build_cayman_sanctions_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
    let sanctions_snapshot = cayman_sanctions_snapshot();

    let mut includes = BTreeMap::new();
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(cayman_sanctions_entries().len()),
    );
    includes.insert(
        "source_lists".to_string(),
        serde_json::json!(["unsc_consolidated", "uk_hmt_consolidated", "cayman_aml_regulations"]),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:ky:sanctions:2026Q1".to_string(),
        jurisdiction_id: "ky".to_string(),
        domain: "sanctions".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "unsc_consolidated",
                "name": "UNSC Consolidated Sanctions List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "uk_hmt_consolidated",
                "name": "UK HM Treasury Consolidated List",
                "authority": "HM Treasury, United Kingdom"
            }),
            serde_json::json!({
                "source_id": "cayman_aml_regulations",
                "name": "Anti-Money Laundering Regulations (as revised)",
                "authority": "Cayman Islands Government / CIMA"
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
        jurisdiction: JurisdictionId::new("ky".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Cayman Islands Sanctions Regulatory Pack — 2026 Q1".to_string(),
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
    fn cayman_has_three_regulators() {
        let regs = cayman_regulators();
        assert_eq!(regs.len(), 3);
        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"ky-cima"));
        assert!(ids.contains(&"ky-registrar"));
        assert!(ids.contains(&"ky-fra"));
    }

    #[test]
    fn all_regulators_timezone() {
        for reg in cayman_regulators() {
            assert_eq!(
                reg.timezone, "America/Cayman",
                "{} wrong tz",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn all_regulators_are_cayman_jurisdiction() {
        for reg in cayman_regulators() {
            assert_eq!(reg.jurisdiction_id, "ky", "{} wrong jid", reg.regulator_id);
        }
    }

    #[test]
    fn fra_parent_is_cima() {
        let fra = fra_regulator();
        assert_eq!(fra.parent_authority, Some("ky-cima".to_string()));
    }

    #[test]
    fn sanctions_have_sources() {
        for entry in cayman_sanctions_entries() {
            assert!(
                !entry.source_lists.is_empty(),
                "{} has no source_lists",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_entries_all_have_programs() {
        for entry in cayman_sanctions_entries() {
            assert!(
                !entry.programs.is_empty(),
                "{} has no programs",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_snapshot_has_sources() {
        let snap = cayman_sanctions_snapshot();
        assert!(snap.sources.contains_key("unsc_consolidated"));
        assert!(snap.sources.contains_key("uk_hmt_consolidated"));
        assert!(snap.sources.contains_key("cayman_aml_regulations"));
    }

    #[test]
    fn compliance_deadlines_cover_all_regulators() {
        let deadlines = cayman_compliance_deadlines();
        let regulator_ids: std::collections::HashSet<&str> =
            deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
        assert!(regulator_ids.contains("ky-cima"), "missing CIMA deadlines");
        assert!(regulator_ids.contains("ky-fra"), "missing FRA deadlines");
        assert!(regulator_ids.contains("ky-registrar"), "missing Registrar deadlines");
    }

    #[test]
    fn compliance_deadlines_have_unique_ids() {
        let deadlines = cayman_compliance_deadlines();
        let mut ids = std::collections::HashSet::new();
        for dl in &deadlines {
            assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
        }
    }

    #[test]
    fn reporting_requirements_cover_key_reports() {
        let reqs = cayman_reporting_requirements();
        let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
        assert!(ids.contains(&"ky-fra-str"), "missing STR/SAR");
        assert!(ids.contains(&"ky-cima-annual-audited-statements"), "missing annual audited");
        assert!(ids.contains(&"ky-cima-prudential-return"), "missing prudential return");
        assert!(ids.contains(&"ky-registrar-annual-return"), "missing registrar annual");
    }

    #[test]
    fn build_regpack_produces_digest() {
        let (regpack, metadata, snap, deadlines, reporting) =
            build_cayman_regpack().expect("build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "ky");
        assert!(regpack.digest.is_some(), "regpack should have digest");
        assert_eq!(metadata.jurisdiction_id, "ky");
        assert!(!snap.consolidated_counts.is_empty());
        assert!(!deadlines.is_empty());
        assert!(!reporting.is_empty());

        // Determinism check
        let (rp2, ..) = build_cayman_regpack().unwrap();
        assert_eq!(
            regpack.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "regpack digest must be deterministic"
        );
    }

    #[test]
    fn build_sanctions_regpack_produces_digest() {
        let (regpack, metadata, snap) =
            build_cayman_sanctions_regpack().expect("sanctions build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "ky");
        assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
        assert_eq!(metadata.domain, "sanctions");
        assert_eq!(metadata.jurisdiction_id, "ky");
        assert!(!snap.consolidated_counts.is_empty());

        // Determinism check
        let (rp2, ..) = build_cayman_sanctions_regpack().unwrap();
        assert_eq!(
            regpack.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "sanctions regpack digest must be deterministic"
        );
    }

    #[test]
    fn sanctions_regpack_digest_differs_from_financial() {
        let (financial, ..) = build_cayman_regpack().unwrap();
        let (sanctions, ..) = build_cayman_sanctions_regpack().unwrap();
        assert_ne!(
            financial.digest.as_ref().unwrap().to_hex(),
            sanctions.digest.as_ref().unwrap().to_hex(),
            "financial and sanctions regpack digests must differ"
        );
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in cayman_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, de.regulator_id);
            assert_eq!(reg.name, de.name);
            assert_eq!(reg.timezone, de.timezone);
        }
    }
}
