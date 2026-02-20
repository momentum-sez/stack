//! Pakistan-specific regpack content — real regulatory data.
//!
//! Provides Pakistan-specific regulatory content required by P0-PACK-001:
//!   - Regulator profiles (SBP, SECP, FMU, FBR, NACTA)
//!   - Sanctions entries (NACTA Proscribed Persons, UNSC 1267 consolidated)
//!   - Compliance deadlines (FBR filing, SBP prudential, SECP annual)
//!   - Reporting requirements (CTR, STR, prudential returns)
//!   - Withholding tax rate schedule (ITO 2001)

use super::*;

// ── Withholding Tax Rates (Income Tax Ordinance 2001) ───────────────────

/// A withholding tax rate entry from the Income Tax Ordinance 2001.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WithholdingTaxRate {
    /// Section reference (e.g., "149" for salary, "151" for profit on debt).
    pub section: String,
    /// Human-readable description.
    pub description: String,
    /// Applicable to filer or non-filer.
    pub taxpayer_status: String,
    /// Rate as string decimal (e.g., "0.15" for 15%).
    pub rate: String,
    /// Threshold amount (PKR) below which WHT does not apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_pkr: Option<String>,
    /// Effective date.
    pub effective_date: String,
    /// Legal basis.
    pub legal_basis: String,
}

/// Pakistan withholding tax rate schedule per ITO 2001 (FY 2025-26).
pub fn pakistan_wht_rates() -> Vec<WithholdingTaxRate> {
    vec![
        WithholdingTaxRate {
            section: "149".to_string(),
            description: "Salary income".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "varies".to_string(),
            threshold_pkr: Some("600000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.149".to_string(),
        },
        WithholdingTaxRate {
            section: "151(1)(a)".to_string(),
            description: "Profit on debt — bank deposits (filer)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.15".to_string(),
            threshold_pkr: None,
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.151(1)(a)".to_string(),
        },
        WithholdingTaxRate {
            section: "151(1)(a)".to_string(),
            description: "Profit on debt — bank deposits (non-filer)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.30".to_string(),
            threshold_pkr: None,
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.151(1)(a)".to_string(),
        },
        WithholdingTaxRate {
            section: "152(1)".to_string(),
            description: "Payments to non-residents — royalties/fees".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.15".to_string(),
            threshold_pkr: None,
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.152(1)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(a)".to_string(),
            description: "Sale of goods (filer, company)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.04".to_string(),
            threshold_pkr: Some("75000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(a)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(a)".to_string(),
            description: "Sale of goods (non-filer, company)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.08".to_string(),
            threshold_pkr: Some("75000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(a)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(b)".to_string(),
            description: "Rendering of services (filer, company)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.08".to_string(),
            threshold_pkr: Some("30000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(b)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(b)".to_string(),
            description: "Rendering of services (non-filer, company)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.16".to_string(),
            threshold_pkr: Some("30000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(b)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(c)".to_string(),
            description: "Contracts — execution of contracts (filer)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.07".to_string(),
            threshold_pkr: Some("75000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(c)".to_string(),
        },
        WithholdingTaxRate {
            section: "153(1)(c)".to_string(),
            description: "Contracts — execution of contracts (non-filer)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.14".to_string(),
            threshold_pkr: Some("75000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.153(1)(c)".to_string(),
        },
        WithholdingTaxRate {
            section: "155".to_string(),
            description: "Income from property — rent (filer)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.15".to_string(),
            threshold_pkr: Some("300000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.155".to_string(),
        },
        WithholdingTaxRate {
            section: "156A".to_string(),
            description: "Prizes and winnings (filer)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.15".to_string(),
            threshold_pkr: None,
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.156A".to_string(),
        },
        WithholdingTaxRate {
            section: "156A".to_string(),
            description: "Prizes and winnings (non-filer)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.30".to_string(),
            threshold_pkr: None,
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.156A".to_string(),
        },
        WithholdingTaxRate {
            section: "231A".to_string(),
            description: "Cash withdrawal from bank (filer)".to_string(),
            taxpayer_status: "filer".to_string(),
            rate: "0.003".to_string(),
            threshold_pkr: Some("50000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.231A".to_string(),
        },
        WithholdingTaxRate {
            section: "231A".to_string(),
            description: "Cash withdrawal from bank (non-filer)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.006".to_string(),
            threshold_pkr: Some("50000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.231A".to_string(),
        },
        WithholdingTaxRate {
            section: "236P".to_string(),
            description: "Banking transactions exceeding Rs 50k (non-filer)".to_string(),
            taxpayer_status: "non-filer".to_string(),
            rate: "0.006".to_string(),
            threshold_pkr: Some("50000".to_string()),
            effective_date: "2025-07-01".to_string(),
            legal_basis: "Income Tax Ordinance 2001, s.236P".to_string(),
        },
    ]
}

// ── Regulator Profiles ──────────────────────────────────────────────────

/// State Bank of Pakistan — central bank and banking regulator.
pub fn sbp_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "banking".to_string(),
        vec![
            "commercial_banks".to_string(),
            "microfinance_banks".to_string(),
            "development_finance_institutions".to_string(),
        ],
    );
    scope.insert(
        "payments".to_string(),
        vec![
            "emi".to_string(),
            "psp".to_string(),
            "raast".to_string(),
            "rtgs".to_string(),
        ],
    );
    scope.insert(
        "foreign_exchange".to_string(),
        vec!["exchange_companies".to_string(), "authorized_dealers".to_string()],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.sbp.org.pk".to_string());
    contact.insert(
        "address".to_string(),
        "I.I. Chundrigar Road, Karachi 74000, Pakistan".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("bank_registry".to_string(), true);
    api.insert("raast_integration".to_string(), true);
    api.insert("credit_bureau_query".to_string(), true);
    api.insert("forex_rate_feed".to_string(), true);

    RegulatorProfile {
        regulator_id: "pk-sbp".to_string(),
        name: "State Bank of Pakistan".to_string(),
        jurisdiction_id: "pk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Karachi".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Securities and Exchange Commission of Pakistan — corporate and capital markets regulator.
pub fn secp_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "corporate".to_string(),
        vec![
            "company_registration".to_string(),
            "corporate_governance".to_string(),
        ],
    );
    scope.insert(
        "capital_markets".to_string(),
        vec![
            "securities_brokers".to_string(),
            "mutual_funds".to_string(),
            "nbfcs".to_string(),
        ],
    );
    scope.insert(
        "insurance".to_string(),
        vec!["life_insurance".to_string(), "general_insurance".to_string()],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.secp.gov.pk".to_string());
    contact.insert(
        "address".to_string(),
        "NIC Building, Jinnah Avenue, Islamabad, Pakistan".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("company_search".to_string(), true);
    api.insert("filing_status".to_string(), true);
    api.insert("eservices_portal".to_string(), true);

    RegulatorProfile {
        regulator_id: "pk-secp".to_string(),
        name: "Securities and Exchange Commission of Pakistan".to_string(),
        jurisdiction_id: "pk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Karachi".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Financial Monitoring Unit — Pakistan's AML/CFT financial intelligence unit.
pub fn fmu_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "aml_cft".to_string(),
        vec![
            "suspicious_transaction_reports".to_string(),
            "currency_transaction_reports".to_string(),
            "targeted_financial_sanctions".to_string(),
            "mutual_legal_assistance".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.fmu.gov.pk".to_string());
    contact.insert(
        "address".to_string(),
        "State Bank of Pakistan Building, Islamabad, Pakistan".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("goaml_reporting".to_string(), true);
    api.insert("sanctions_query".to_string(), true);

    RegulatorProfile {
        regulator_id: "pk-fmu".to_string(),
        name: "Financial Monitoring Unit".to_string(),
        jurisdiction_id: "pk".to_string(),
        parent_authority: Some("pk-sbp".to_string()),
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Karachi".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// Federal Board of Revenue — Pakistan's tax authority.
pub fn fbr_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "tax".to_string(),
        vec![
            "income_tax".to_string(),
            "sales_tax".to_string(),
            "federal_excise".to_string(),
            "customs_duty".to_string(),
            "withholding_tax".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://www.fbr.gov.pk".to_string());
    contact.insert(
        "address".to_string(),
        "Constitution Avenue, Islamabad, Pakistan".to_string(),
    );

    let mut api = BTreeMap::new();
    api.insert("iris_portal".to_string(), true);
    api.insert("ntn_verification".to_string(), true);
    api.insert("active_taxpayer_list".to_string(), true);
    api.insert("e_filing".to_string(), true);

    RegulatorProfile {
        regulator_id: "pk-fbr".to_string(),
        name: "Federal Board of Revenue".to_string(),
        jurisdiction_id: "pk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: api,
        timezone: "Asia/Karachi".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// NACTA — National Counter Terrorism Authority (proscription list).
pub fn nacta_regulator() -> RegulatorProfile {
    let mut scope = BTreeMap::new();
    scope.insert(
        "sanctions".to_string(),
        vec![
            "proscribed_organizations".to_string(),
            "designated_persons".to_string(),
            "unsc_1267_implementation".to_string(),
        ],
    );

    let mut contact = BTreeMap::new();
    contact.insert("website".to_string(), "https://nacta.gov.pk".to_string());
    contact.insert(
        "address".to_string(),
        "Sector G-5, Islamabad, Pakistan".to_string(),
    );

    RegulatorProfile {
        regulator_id: "pk-nacta".to_string(),
        name: "National Counter Terrorism Authority".to_string(),
        jurisdiction_id: "pk".to_string(),
        parent_authority: None,
        scope,
        contact,
        api_capabilities: BTreeMap::new(),
        timezone: "Asia/Karachi".to_string(),
        business_days: vec![
            "monday".to_string(),
            "tuesday".to_string(),
            "wednesday".to_string(),
            "thursday".to_string(),
            "friday".to_string(),
        ],
    }
}

/// All Pakistan regulatory authorities relevant to regpack domains.
pub fn pakistan_regulators() -> Vec<RegulatorProfile> {
    vec![
        sbp_regulator(),
        secp_regulator(),
        fmu_regulator(),
        fbr_regulator(),
        nacta_regulator(),
    ]
}

// ── Sanctions Entries ────────────────────────────────────────────────────
//
// Representative entries from Pakistan's proscription regime.
// Sources: NACTA First Schedule (Anti-Terrorism Act 1997),
//          UNSC 1267/1989/2253 Consolidated Sanctions List.
//
// NOTE: These are publicly available, gazette-notified designations.
// Real deployment must pull from live NACTA gazette and UNSC XML feed.

/// Representative Pakistan sanctions entries for regpack content.
pub fn pakistan_sanctions_entries() -> Vec<SanctionsEntry> {
    vec![
        SanctionsEntry {
            entry_id: "pk-nacta-001".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "nacta_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Lashkar-e-Taiba".to_string(),
            aliases: vec![
                btree_alias("Jamaat-ud-Dawa"),
                btree_alias("Falah-i-Insaniyat Foundation"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Muridke, Punjab, Pakistan")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "ata_1997_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2002-01-14".to_string()),
            remarks: Some("UNSC QDe.118; ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-002".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "nacta_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            primary_name: "Jaish-e-Mohammed".to_string(),
            aliases: vec![
                btree_alias("Jaish-i-Mohammed"),
                btree_alias("Khuddam ul-Islam"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Bahawalpur, Punjab, Pakistan")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "ata_1997_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2001-10-17".to_string()),
            remarks: Some("UNSC QDe.019; ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-003".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["nacta_first_schedule".to_string()],
            primary_name: "Tehrik-i-Taliban Pakistan".to_string(),
            aliases: vec![btree_alias("TTP")],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["ata_1997_first_schedule".to_string()],
            listing_date: Some("2008-08-25".to_string()),
            remarks: Some("ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-004".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "nacta_first_schedule".to_string(),
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
                "ata_1997_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2001-10-15".to_string()),
            remarks: Some("UNSC QDe.004; ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-005".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec![
                "nacta_first_schedule".to_string(),
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
                "ata_1997_first_schedule".to_string(),
                "unsc_2253".to_string(),
            ],
            listing_date: Some("2015-07-01".to_string()),
            remarks: Some("UNSC; ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-006".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["nacta_first_schedule".to_string()],
            primary_name: "Sipah-e-Sahaba Pakistan".to_string(),
            aliases: vec![
                btree_alias("SSP"),
                btree_alias("Ahle Sunnat Wal Jamaat"),
            ],
            identifiers: vec![],
            addresses: vec![btree_address("Jhang, Punjab, Pakistan")],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["ata_1997_first_schedule".to_string()],
            listing_date: Some("2002-01-14".to_string()),
            remarks: Some("ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-007".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["nacta_first_schedule".to_string()],
            primary_name: "Lashkar-e-Jhangvi".to_string(),
            aliases: vec![btree_alias("LeJ")],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec![
                "ata_1997_first_schedule".to_string(),
                "unsc_1267".to_string(),
            ],
            listing_date: Some("2001-08-14".to_string()),
            remarks: Some("UNSC QDe.096; ATA 1997 First Schedule".to_string()),
        },
        SanctionsEntry {
            entry_id: "pk-nacta-008".to_string(),
            entry_type: "organization".to_string(),
            source_lists: vec!["nacta_first_schedule".to_string()],
            primary_name: "Balochistan Liberation Army".to_string(),
            aliases: vec![btree_alias("BLA")],
            identifiers: vec![],
            addresses: vec![],
            nationalities: vec![],
            date_of_birth: None,
            programs: vec!["ata_1997_first_schedule".to_string()],
            listing_date: Some("2006-04-07".to_string()),
            remarks: Some("ATA 1997 First Schedule".to_string()),
        },
    ]
}

/// Build a sanctions snapshot from Pakistan entries.
pub fn pakistan_sanctions_snapshot() -> SanctionsSnapshot {
    let entries = pakistan_sanctions_entries();

    let mut counts = BTreeMap::new();
    for entry in &entries {
        *counts.entry(entry.entry_type.clone()).or_insert(0i64) += 1;
    }

    let mut sources = BTreeMap::new();
    sources.insert(
        "nacta_first_schedule".to_string(),
        serde_json::json!({
            "name": "NACTA First Schedule — Anti-Terrorism Act 1997",
            "url": "https://nacta.gov.pk/proscribed-organizations/",
            "authority": "Government of Pakistan",
            "legal_basis": "Anti-Terrorism Act 1997, First Schedule"
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
        snapshot_id: "pk-sanctions-2026Q1".to_string(),
        snapshot_timestamp: "2026-01-15T00:00:00Z".to_string(),
        sources,
        consolidated_counts: counts,
        delta_from_previous: None,
    }
}

// ── Compliance Deadlines ────────────────────────────────────────────────

/// Pakistan compliance deadlines for FY 2025-26.
pub fn pakistan_compliance_deadlines() -> Vec<ComplianceDeadline> {
    vec![
        // FBR Income Tax
        ComplianceDeadline {
            deadline_id: "pk-fbr-it-annual-company".to_string(),
            regulator_id: "pk-fbr".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual income tax return — companies (FY 2025-26)".to_string(),
            due_date: "2026-12-31".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-secp:company-registration".to_string(),
                "pk-sbp:commercial-bank".to_string(),
                "pk-sbp:microfinance-bank".to_string(),
                "pk-sbp:emi".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-fbr-it-annual-individual".to_string(),
            regulator_id: "pk-fbr".to_string(),
            deadline_type: "filing".to_string(),
            description: "Annual income tax return — individuals/AOPs (FY 2025-26)".to_string(),
            due_date: "2026-09-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![],
        },
        ComplianceDeadline {
            deadline_id: "pk-fbr-wht-monthly".to_string(),
            regulator_id: "pk-fbr".to_string(),
            deadline_type: "payment".to_string(),
            description: "Monthly withholding tax statement (15th of following month)".to_string(),
            due_date: "2026-02-15".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-sbp:commercial-bank".to_string(),
                "pk-secp:company-registration".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-fbr-sales-tax-monthly".to_string(),
            regulator_id: "pk-fbr".to_string(),
            deadline_type: "filing".to_string(),
            description: "Monthly sales tax return (18th of following month)".to_string(),
            due_date: "2026-02-18".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![],
        },
        // SBP Prudential Returns
        ComplianceDeadline {
            deadline_id: "pk-sbp-quarterly-prudential".to_string(),
            regulator_id: "pk-sbp".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly prudential return — banks (within 30 days of quarter-end)"
                .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-sbp:commercial-bank".to_string(),
                "pk-sbp:microfinance-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-sbp-annual-audited".to_string(),
            regulator_id: "pk-sbp".to_string(),
            deadline_type: "report".to_string(),
            description: "Annual audited financial statements — banks (within 4 months of FY-end)"
                .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec![
                "pk-sbp:commercial-bank".to_string(),
                "pk-sbp:microfinance-bank".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-sbp-emi-quarterly".to_string(),
            regulator_id: "pk-sbp".to_string(),
            deadline_type: "report".to_string(),
            description: "Quarterly EMI compliance report — float safeguarding, transaction volume"
                .to_string(),
            due_date: "2026-04-30".to_string(),
            grace_period_days: 15,
            applicable_license_types: vec!["pk-sbp:emi".to_string()],
        },
        ComplianceDeadline {
            deadline_id: "pk-sbp-forex-monthly".to_string(),
            regulator_id: "pk-sbp".to_string(),
            deadline_type: "report".to_string(),
            description: "Monthly foreign exchange position report — exchange companies"
                .to_string(),
            due_date: "2026-02-10".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec!["pk-sbp:exchange-company".to_string()],
        },
        // SECP Annual Filings
        ComplianceDeadline {
            deadline_id: "pk-secp-annual-return".to_string(),
            regulator_id: "pk-secp".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Annual return (Form A) — within 30 days of AGM (Companies Act 2017 s.130)"
                    .to_string(),
            due_date: "2026-10-30".to_string(),
            grace_period_days: 30,
            applicable_license_types: vec!["pk-secp:company-registration".to_string()],
        },
        ComplianceDeadline {
            deadline_id: "pk-secp-financial-statements".to_string(),
            regulator_id: "pk-secp".to_string(),
            deadline_type: "filing".to_string(),
            description:
                "Audited financial statements — within 4 months of FY-end (s.233 Companies Act 2017)"
                    .to_string(),
            due_date: "2026-10-30".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-secp:company-registration".to_string(),
                "pk-secp:nbfc".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-secp-broker-net-capital".to_string(),
            regulator_id: "pk-secp".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Monthly net capital balance certificate — securities brokers"
                    .to_string(),
            due_date: "2026-02-15".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec!["pk-secp:securities-broker".to_string()],
        },
        // FMU AML/CFT
        ComplianceDeadline {
            deadline_id: "pk-fmu-str-ongoing".to_string(),
            regulator_id: "pk-fmu".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Suspicious Transaction Report — within 7 days of suspicion (AML Act 2010 s.7)"
                    .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-sbp:commercial-bank".to_string(),
                "pk-sbp:microfinance-bank".to_string(),
                "pk-sbp:emi".to_string(),
                "pk-sbp:exchange-company".to_string(),
                "pk-secp:securities-broker".to_string(),
            ],
        },
        ComplianceDeadline {
            deadline_id: "pk-fmu-ctr-15days".to_string(),
            regulator_id: "pk-fmu".to_string(),
            deadline_type: "report".to_string(),
            description:
                "Currency Transaction Report — within 15 days for transactions >= PKR 2M (AML Act 2010 s.7)"
                    .to_string(),
            due_date: "ongoing".to_string(),
            grace_period_days: 0,
            applicable_license_types: vec![
                "pk-sbp:commercial-bank".to_string(),
                "pk-sbp:exchange-company".to_string(),
            ],
        },
    ]
}

// ── Reporting Requirements ───────────────────────────────────────────────

/// Pakistan reporting requirements across regulators.
pub fn pakistan_reporting_requirements() -> Vec<ReportingRequirement> {
    vec![
        ReportingRequirement {
            report_type_id: "pk-fmu-str".to_string(),
            name: "Suspicious Transaction Report (STR)".to_string(),
            regulator_id: "pk-fmu".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "microfinance_bank".to_string(),
                "emi".to_string(),
                "exchange_company".to_string(),
                "securities_broker".to_string(),
                "nbfc".to_string(),
                "insurance_company".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_detection".to_string(), "7".to_string());
                inner.insert("submission_system".to_string(), "goAML".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("goAML XML"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://goaml.fmu.gov.pk"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("AML Act 2010 s.16: imprisonment up to 5 years or fine up to PKR 10M"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "pk-fmu-ctr".to_string(),
            name: "Currency Transaction Report (CTR)".to_string(),
            regulator_id: "pk-fmu".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "exchange_company".to_string(),
            ],
            frequency: "event_driven".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("days_from_transaction".to_string(), "15".to_string());
                inner.insert("threshold_pkr".to_string(), "2000000".to_string());
                d.insert("trigger".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("goAML XML"));
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("AML Act 2010 s.16: fine up to PKR 5M"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "pk-sbp-prudential-quarterly".to_string(),
            name: "Quarterly Prudential Return".to_string(),
            regulator_id: "pk-sbp".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "microfinance_bank".to_string(),
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
                s.insert("format".to_string(), serde_json::json!("SBP XBRL / Excel"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("SBP Banking Surveillance Department"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("BCO 1962 s.46: penalty per day of default"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "pk-fbr-wht-statement".to_string(),
            name: "Monthly Withholding Tax Statement".to_string(),
            regulator_id: "pk-fbr".to_string(),
            applicable_to: vec![
                "commercial_bank".to_string(),
                "company".to_string(),
                "aop".to_string(),
            ],
            frequency: "monthly".to_string(),
            deadlines: {
                let mut d = BTreeMap::new();
                let mut inner = BTreeMap::new();
                inner.insert("day_of_following_month".to_string(), "15".to_string());
                d.insert("standard".to_string(), inner);
                d
            },
            submission: {
                let mut s = BTreeMap::new();
                s.insert("format".to_string(), serde_json::json!("FBR IRIS e-filing"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://iris.fbr.gov.pk"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("ITO 2001 s.182: PKR 2,500 per day of default"),
                );
                p
            },
        },
        ReportingRequirement {
            report_type_id: "pk-secp-annual-return".to_string(),
            name: "Annual Return (Form A/B)".to_string(),
            regulator_id: "pk-secp".to_string(),
            applicable_to: vec![
                "company".to_string(),
                "nbfc".to_string(),
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
                s.insert("format".to_string(), serde_json::json!("SECP eServices"));
                s.insert(
                    "portal".to_string(),
                    serde_json::json!("https://eservices.secp.gov.pk"),
                );
                s
            },
            late_penalty: {
                let mut p = BTreeMap::new();
                p.insert(
                    "penalty".to_string(),
                    serde_json::json!("Companies Act 2017 s.130: PKR 100 per day up to 2 years"),
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

/// Build a complete Pakistan regpack with all content.
///
/// Assembles regulators, sanctions, deadlines, and reporting requirements
/// into a content-addressed regpack for the `pk` jurisdiction.
#[allow(clippy::type_complexity)]
pub fn build_pakistan_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot, Vec<ComplianceDeadline>, Vec<ReportingRequirement>, Vec<WithholdingTaxRate>)> {
    let regulators = pakistan_regulators();
    let sanctions_snapshot = pakistan_sanctions_snapshot();
    let deadlines = pakistan_compliance_deadlines();
    let reporting = pakistan_reporting_requirements();
    let wht_rates = pakistan_wht_rates();

    let mut includes = BTreeMap::new();
    includes.insert(
        "regulators".to_string(),
        serde_json::json!(regulators.iter().map(|r| &r.regulator_id).collect::<Vec<_>>()),
    );
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(pakistan_sanctions_entries().len()),
    );
    includes.insert(
        "compliance_deadlines".to_string(),
        serde_json::json!(deadlines.len()),
    );
    includes.insert(
        "reporting_requirements".to_string(),
        serde_json::json!(reporting.len()),
    );
    includes.insert(
        "wht_rates".to_string(),
        serde_json::json!(wht_rates.len()),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:pk:financial:2026Q1".to_string(),
        jurisdiction_id: "pk".to_string(),
        domain: "financial".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "nacta_gazette",
                "name": "NACTA Proscribed Organizations Gazette",
                "authority": "Government of Pakistan"
            }),
            serde_json::json!({
                "source_id": "unsc_1267",
                "name": "UNSC 1267/1989/2253 Consolidated List",
                "authority": "United Nations Security Council"
            }),
            serde_json::json!({
                "source_id": "fbr_ito_2001",
                "name": "Income Tax Ordinance 2001 (as amended)",
                "authority": "Federal Board of Revenue"
            }),
            serde_json::json!({
                "source_id": "aml_act_2010",
                "name": "Anti-Money Laundering Act 2010",
                "authority": "Government of Pakistan"
            }),
            serde_json::json!({
                "source_id": "companies_act_2017",
                "name": "Companies Act 2017",
                "authority": "Government of Pakistan / SECP"
            }),
            serde_json::json!({
                "source_id": "bco_1962",
                "name": "Banking Companies Ordinance 1962",
                "authority": "State Bank of Pakistan"
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
        jurisdiction: JurisdictionId::new("pk".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Pakistan Financial Regulatory Pack — 2026 Q1".to_string(),
        version: REGPACK_VERSION.to_string(),
        digest: Some(
            ContentDigest::from_hex(&digest)
                .map_err(|e| PackError::Validation(format!("digest error: {e}")))?,
        ),
        metadata: Some(metadata.clone()),
    };

    Ok((regpack, metadata, sanctions_snapshot, deadlines, reporting, wht_rates))
}

/// Build a sanctions-domain-specific Pakistan regpack.
///
/// Produces a regpack focused on the `sanctions` compliance domain,
/// containing the NACTA proscribed organizations gazette and UNSC 1267
/// consolidated list entries. Separate from the `financial` domain
/// regpack which includes broader regulatory data (WHT rates, regulators,
/// compliance deadlines, reporting requirements).
///
/// The sanctions regpack is content-addressed independently so that
/// sanctions-list-only updates can be pushed without rebuilding the
/// full financial regpack.
pub fn build_pakistan_sanctions_regpack() -> PackResult<(Regpack, RegPackMetadata, SanctionsSnapshot)> {
    let sanctions_snapshot = pakistan_sanctions_snapshot();

    let mut includes = BTreeMap::new();
    includes.insert(
        "sanctions_entries".to_string(),
        serde_json::json!(pakistan_sanctions_entries().len()),
    );
    includes.insert(
        "source_lists".to_string(),
        serde_json::json!(["nacta_gazette", "unsc_1267"]),
    );

    let metadata = RegPackMetadata {
        regpack_id: "regpack:pk:sanctions:2026Q1".to_string(),
        jurisdiction_id: "pk".to_string(),
        domain: "sanctions".to_string(),
        as_of_date: "2026-01-15".to_string(),
        snapshot_type: "quarterly".to_string(),
        sources: vec![
            serde_json::json!({
                "source_id": "nacta_gazette",
                "name": "NACTA Proscribed Organizations Gazette",
                "authority": "Government of Pakistan"
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
        None,  // No regulators — sanctions-only domain
        None,  // No deadlines — sanctions-only domain
    )?;

    let regpack = Regpack {
        jurisdiction: JurisdictionId::new("pk".to_string())
            .map_err(|e| PackError::Validation(format!("invalid jurisdiction: {e}")))?,
        name: "Pakistan Sanctions Regulatory Pack — 2026 Q1".to_string(),
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
    fn pakistan_has_five_regulators() {
        let regs = pakistan_regulators();
        assert_eq!(regs.len(), 5);
        let ids: Vec<&str> = regs.iter().map(|r| r.regulator_id.as_str()).collect();
        assert!(ids.contains(&"pk-sbp"));
        assert!(ids.contains(&"pk-secp"));
        assert!(ids.contains(&"pk-fmu"));
        assert!(ids.contains(&"pk-fbr"));
        assert!(ids.contains(&"pk-nacta"));
    }

    #[test]
    fn all_regulators_are_pakistan_jurisdiction() {
        for reg in pakistan_regulators() {
            assert_eq!(reg.jurisdiction_id, "pk", "{} wrong jid", reg.regulator_id);
        }
    }

    #[test]
    fn all_regulators_have_asia_karachi_timezone() {
        for reg in pakistan_regulators() {
            assert_eq!(
                reg.timezone, "Asia/Karachi",
                "{} wrong tz",
                reg.regulator_id
            );
        }
    }

    #[test]
    fn fmu_parent_is_sbp() {
        let fmu = fmu_regulator();
        assert_eq!(fmu.parent_authority, Some("pk-sbp".to_string()));
    }

    #[test]
    fn sanctions_entries_all_have_source_lists() {
        for entry in pakistan_sanctions_entries() {
            assert!(
                !entry.source_lists.is_empty(),
                "{} has no source_lists",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_entries_all_have_programs() {
        for entry in pakistan_sanctions_entries() {
            assert!(
                !entry.programs.is_empty(),
                "{} has no programs",
                entry.entry_id
            );
        }
    }

    #[test]
    fn sanctions_snapshot_has_sources() {
        let snap = pakistan_sanctions_snapshot();
        assert!(snap.sources.contains_key("nacta_first_schedule"));
        assert!(snap.sources.contains_key("unsc_1267"));
    }

    #[test]
    fn sanctions_checker_finds_exact_match() {
        let entries = pakistan_sanctions_entries();
        let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
        let result = checker.check_entity("Al-Qaeda", None, 0.7);
        assert!(result.matched, "Al-Qaeda should match");
        assert_eq!(result.match_score, 1.0);
    }

    #[test]
    fn sanctions_checker_finds_alias() {
        let entries = pakistan_sanctions_entries();
        let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
        let result = checker.check_entity("Jamaat-ud-Dawa", None, 0.7);
        assert!(result.matched, "Alias Jamaat-ud-Dawa should match");
    }

    #[test]
    fn sanctions_checker_rejects_clean_entity() {
        let entries = pakistan_sanctions_entries();
        let checker = SanctionsChecker::new(entries, "pk-sanctions-2026Q1".to_string());
        let result = checker.check_entity("Habib Bank Limited", None, 0.8);
        assert!(!result.matched, "legitimate bank should not match");
    }

    #[test]
    fn compliance_deadlines_cover_all_regulators() {
        let deadlines = pakistan_compliance_deadlines();
        let regulator_ids: std::collections::HashSet<&str> =
            deadlines.iter().map(|d| d.regulator_id.as_str()).collect();
        assert!(regulator_ids.contains("pk-fbr"), "missing FBR deadlines");
        assert!(regulator_ids.contains("pk-sbp"), "missing SBP deadlines");
        assert!(regulator_ids.contains("pk-secp"), "missing SECP deadlines");
        assert!(regulator_ids.contains("pk-fmu"), "missing FMU deadlines");
    }

    #[test]
    fn compliance_deadlines_have_unique_ids() {
        let deadlines = pakistan_compliance_deadlines();
        let mut ids = std::collections::HashSet::new();
        for dl in &deadlines {
            assert!(ids.insert(&dl.deadline_id), "duplicate: {}", dl.deadline_id);
        }
    }

    #[test]
    fn reporting_requirements_cover_key_reports() {
        let reqs = pakistan_reporting_requirements();
        let ids: Vec<&str> = reqs.iter().map(|r| r.report_type_id.as_str()).collect();
        assert!(ids.contains(&"pk-fmu-str"), "missing STR");
        assert!(ids.contains(&"pk-fmu-ctr"), "missing CTR");
        assert!(ids.contains(&"pk-sbp-prudential-quarterly"), "missing prudential");
        assert!(ids.contains(&"pk-fbr-wht-statement"), "missing WHT statement");
        assert!(ids.contains(&"pk-secp-annual-return"), "missing SECP annual");
    }

    #[test]
    fn wht_rates_cover_key_sections() {
        let rates = pakistan_wht_rates();
        assert!(rates.len() >= 12, "expected >= 12 WHT rates, got {}", rates.len());
        let sections: Vec<&str> = rates.iter().map(|r| r.section.as_str()).collect();
        assert!(sections.contains(&"149"), "missing salary s.149");
        assert!(sections.contains(&"151(1)(a)"), "missing profit on debt s.151");
        assert!(sections.contains(&"153(1)(a)"), "missing goods s.153(1)(a)");
        assert!(sections.contains(&"153(1)(b)"), "missing services s.153(1)(b)");
        assert!(sections.contains(&"231A"), "missing cash withdrawal s.231A");
    }

    #[test]
    fn wht_rates_distinguish_filer_nonfiler() {
        let rates = pakistan_wht_rates();
        let filer_count = rates.iter().filter(|r| r.taxpayer_status == "filer").count();
        let nonfiler_count = rates
            .iter()
            .filter(|r| r.taxpayer_status == "non-filer")
            .count();
        assert!(filer_count > 0, "no filer rates");
        assert!(nonfiler_count > 0, "no non-filer rates");
    }

    #[test]
    fn build_pakistan_regpack_succeeds() {
        let (regpack, metadata, snap, deadlines, reporting, wht) =
            build_pakistan_regpack().expect("build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "pk");
        assert!(regpack.digest.is_some(), "regpack should have digest");
        assert_eq!(metadata.jurisdiction_id, "pk");
        assert!(!snap.consolidated_counts.is_empty());
        assert!(!deadlines.is_empty());
        assert!(!reporting.is_empty());
        assert!(!wht.is_empty());
    }

    #[test]
    fn build_pakistan_regpack_is_deterministic() {
        let (rp1, ..) = build_pakistan_regpack().unwrap();
        let (rp2, ..) = build_pakistan_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "regpack digest must be deterministic"
        );
    }

    #[test]
    fn build_pakistan_sanctions_regpack_succeeds() {
        let (regpack, metadata, snap) =
            build_pakistan_sanctions_regpack().expect("sanctions build should succeed");
        assert_eq!(regpack.jurisdiction.as_str(), "pk");
        assert!(regpack.digest.is_some(), "sanctions regpack should have digest");
        assert_eq!(metadata.domain, "sanctions");
        assert_eq!(metadata.jurisdiction_id, "pk");
        assert!(!snap.consolidated_counts.is_empty());
    }

    #[test]
    fn build_pakistan_sanctions_regpack_is_deterministic() {
        let (rp1, ..) = build_pakistan_sanctions_regpack().unwrap();
        let (rp2, ..) = build_pakistan_sanctions_regpack().unwrap();
        assert_eq!(
            rp1.digest.as_ref().unwrap().to_hex(),
            rp2.digest.as_ref().unwrap().to_hex(),
            "sanctions regpack digest must be deterministic"
        );
    }

    #[test]
    fn sanctions_regpack_digest_differs_from_financial() {
        let (financial, ..) = build_pakistan_regpack().unwrap();
        let (sanctions, ..) = build_pakistan_sanctions_regpack().unwrap();
        assert_ne!(
            financial.digest.as_ref().unwrap().to_hex(),
            sanctions.digest.as_ref().unwrap().to_hex(),
            "financial and sanctions regpack digests must differ"
        );
    }

    #[test]
    fn regulator_serialization_roundtrip() {
        for reg in pakistan_regulators() {
            let json = serde_json::to_string(&reg).expect("serialize");
            let de: RegulatorProfile = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reg.regulator_id, de.regulator_id);
            assert_eq!(reg.name, de.name);
            assert_eq!(reg.timezone, de.timezone);
        }
    }
}
