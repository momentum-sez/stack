//! # Tax Collection Pipeline — End-to-End Integration Tests (B-003 / P1-009)
//!
//! Validates the full tax collection pipeline as described in CLAUDE.md Section II:
//!
//! ```text
//! Transaction (Mass fiscal event)
//!   -> Tax Event classification (TaxEvent)
//!   -> Withholding computation (WithholdingEngine)
//!   -> Report generation (TaxReport for FBR IRIS)
//! ```
//!
//! These tests exercise the real `msez-agentic::tax` module across all three
//! pipeline stages, verifying:
//!
//! - Correct withholding rate selection based on filer status (filer vs non-filer)
//! - Correct withholding amount computation with fixed-precision arithmetic
//! - Pakistan ITO 2001 statutory section mapping for all event types
//! - Multi-event report aggregation with line item grouping
//! - Deterministic computation (Theorem 17.1 analog for tax pipeline)
//! - Pipeline behavior for unknown jurisdictions (zero withholding)
//! - Threshold-based rule filtering (salary threshold at PKR 50,000)
//! - Custom rule loading for non-Pakistan jurisdictions (UAE VAT)
//! - Full pipeline orchestration via `TaxPipeline`
//! - Cross-crate composition: agentic policy engine trigger taxonomy
//!   alongside tax pipeline event taxonomy
//!
//! ## What is NOT tested here (requires Mass API integration)
//!
//! - Actual HTTP calls to `treasury-info.api.mass.inc` for withholding execution
//! - Persistence of tax events to Postgres (requires database)
//! - FBR IRIS submission (requires external service)
//! - Agentic trigger-to-tax-event bridge (would need `PolicyEngine` -> `TaxPipeline`
//!   composition, which is currently done in `msez-api` routes, not in a composable
//!   library function — a composable `process_fiscal_trigger()` function in
//!   `msez-agentic` would close this gap)

use msez_agentic::tax::{
    self, format_amount, generate_report, pakistan_standard_rules, parse_amount, FilerStatus,
    ReportParams, ReportStatus, TaxCategory, TaxEvent, TaxEventType, TaxPipeline,
    WithholdingEngine, WithholdingRule,
};
use uuid::Uuid;

// ===========================================================================
// Stage 1: Tax Event Classification
// ===========================================================================

/// Validates that every TaxEventType maps to its correct default TaxCategory
/// under Pakistani tax law. This ensures the classification taxonomy is
/// consistent: income tax events map to IncomeTax, supply events to SalesTax,
/// and import/export events to CustomsDuty.
#[test]
fn event_type_default_category_mapping_is_consistent() {
    let expected: Vec<(TaxEventType, TaxCategory)> = vec![
        (TaxEventType::PaymentForGoods, TaxCategory::IncomeTax),
        (TaxEventType::PaymentForServices, TaxCategory::IncomeTax),
        (TaxEventType::SalaryPayment, TaxCategory::IncomeTax),
        (TaxEventType::ProfitOnDebt, TaxCategory::IncomeTax),
        (TaxEventType::DividendDistribution, TaxCategory::IncomeTax),
        (TaxEventType::RentPayment, TaxCategory::IncomeTax),
        (TaxEventType::CashWithdrawal, TaxCategory::IncomeTax),
        (TaxEventType::SaleToUnregistered, TaxCategory::IncomeTax),
        (TaxEventType::CrossBorderPayment, TaxCategory::IncomeTax),
        (TaxEventType::CapitalGainDisposal, TaxCategory::IncomeTax),
        (TaxEventType::ImportOfGoods, TaxCategory::CustomsDuty),
        (TaxEventType::ExportOfGoods, TaxCategory::CustomsDuty),
        (TaxEventType::SupplyOfGoods, TaxCategory::SalesTax),
        (TaxEventType::SupplyOfServices, TaxCategory::SalesTax),
        (TaxEventType::FormationFee, TaxCategory::IncomeTax),
        (TaxEventType::AnnualFilingFee, TaxCategory::IncomeTax),
    ];

    for (event_type, expected_category) in &expected {
        assert_eq!(
            event_type.default_category(),
            *expected_category,
            "TaxEventType::{} should default to TaxCategory::{}, got {:?}",
            event_type.as_str(),
            expected_category.as_str(),
            event_type.default_category()
        );
    }

    // Verify all 16 event types are covered — guards against new variants
    // being added without updating this test.
    assert_eq!(
        expected.len(),
        TaxEventType::all().len(),
        "test must cover all TaxEventType variants"
    );
}

/// Validates that TaxEvent::new correctly auto-assigns the tax category
/// from the event type's default, and that builder methods set fields
/// without corrupting others.
#[test]
fn tax_event_builder_preserves_invariants() {
    let entity_id = Uuid::new_v4();
    let counterparty_id = Uuid::new_v4();
    let mass_tx_id = Uuid::new_v4();
    let mass_pay_id = Uuid::new_v4();

    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "250000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("9876543", FilerStatus::Filer)
    .with_statutory_section("ITO 2001 Section 153(1)(a)")
    .with_counterparty(counterparty_id)
    .with_mass_transaction(mass_tx_id)
    .with_mass_payment(mass_pay_id)
    .with_metadata(serde_json::json!({"invoice_number": "INV-2025-001"}));

    // Auto-assigned category from event type.
    assert_eq!(event.tax_category, TaxCategory::IncomeTax);
    // All builder fields set correctly.
    assert_eq!(event.entity_id, entity_id);
    assert_eq!(event.filer_status, FilerStatus::Filer);
    assert_eq!(event.ntn.as_deref(), Some("9876543"));
    assert_eq!(
        event.statutory_section.as_deref(),
        Some("ITO 2001 Section 153(1)(a)")
    );
    assert_eq!(event.counterparty_entity_id, Some(counterparty_id));
    assert_eq!(event.mass_transaction_id, Some(mass_tx_id));
    assert_eq!(event.mass_payment_id, Some(mass_pay_id));
    assert_eq!(event.metadata["invoice_number"], "INV-2025-001");
    assert_eq!(event.gross_amount, "250000");
    assert_eq!(event.currency, "PKR");
    assert_eq!(event.jurisdiction_id, "PK");
    assert_eq!(event.tax_year, "2025-2026");
}

/// Validates that with_tax_category overrides the auto-assigned category.
/// This is needed when an event has multi-category implications (e.g.,
/// an import triggers both customs duty and sales tax).
#[test]
fn tax_event_category_override() {
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::ImportOfGoods,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    );
    // Default category for ImportOfGoods is CustomsDuty.
    assert_eq!(event.tax_category, TaxCategory::CustomsDuty);

    // Override to SalesTax for the sales tax component of the import.
    let overridden = event.with_tax_category(TaxCategory::SalesTax);
    assert_eq!(overridden.tax_category, TaxCategory::SalesTax);
    // Event type should be unchanged.
    assert_eq!(overridden.event_type, TaxEventType::ImportOfGoods);
}

/// Validates that TaxEvent defaults to NonFiler when no NTN is provided.
/// Under Pakistani tax law, unregistered entities pay double withholding rates.
#[test]
fn tax_event_defaults_to_nonfiler() {
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    );
    assert_eq!(event.filer_status, FilerStatus::NonFiler);
    assert!(event.ntn.is_none());
}

// ===========================================================================
// Stage 2: Withholding Computation
// ===========================================================================

/// Validates Pakistan's standard withholding rules are loaded correctly.
/// The rule set must cover all major ITO 2001 sections and the Sales Tax Act.
#[test]
fn pakistan_standard_rules_cover_major_sections() {
    let rules = pakistan_standard_rules();

    // Verify we have a substantial rule set (currently 14 standard rules).
    assert!(
        rules.len() >= 10,
        "Pakistan standard rules should have at least 10 rules, got {}",
        rules.len()
    );

    // Verify key statutory sections are represented.
    let sections: Vec<&str> = rules.iter().map(|r| r.statutory_section.as_str()).collect();

    assert!(
        sections.iter().any(|s| s.contains("153")),
        "must include Section 153 (goods/services WHT)"
    );
    assert!(
        sections.iter().any(|s| s.contains("149")),
        "must include Section 149 (salary WHT)"
    );
    assert!(
        sections.iter().any(|s| s.contains("151")),
        "must include Section 151 (profit on debt)"
    );
    assert!(
        sections.iter().any(|s| s.contains("150")),
        "must include Section 150 (dividends)"
    );
    assert!(
        sections.iter().any(|s| s.contains("155")),
        "must include Section 155 (rent)"
    );
    assert!(
        sections.iter().any(|s| s.contains("152")),
        "must include Section 152 (cross-border)"
    );
    assert!(
        sections.iter().any(|s| s.contains("Sales Tax")),
        "must include Sales Tax Act rules"
    );

    // Verify rule IDs follow the naming convention: PAK-{statute}-{section}-{detail}.
    for rule in &rules {
        assert!(
            rule.rule_id.starts_with("PAK-"),
            "Pakistan rule ID should start with 'PAK-', got: {}",
            rule.rule_id
        );
    }
}

/// Validates the core withholding computation: payment for goods to a filer
/// at 4.5% under ITO 2001 Section 153(1)(a).
///
/// PKR 100,000 gross * 4.5% = PKR 4,500 withholding, PKR 95,500 net.
#[test]
fn withholding_goods_filer_section_153() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let entity_id = Uuid::new_v4();
    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    assert_eq!(results.len(), 1, "exactly one rule should match");

    let r = &results[0];
    assert_eq!(r.entity_id, entity_id);
    assert_eq!(r.event_id, event.event_id);
    assert_eq!(r.rate_percent, "4.5");
    assert_eq!(r.withholding_amount, "4500.00");
    assert_eq!(r.net_amount, "95500.00");
    assert_eq!(r.statutory_section, "ITO 2001 Section 153(1)(a)");
    assert!(!r.is_final_tax, "goods WHT under S153 is adjustable, not final");
    assert_eq!(r.currency, "PKR");
    assert_eq!(r.tax_category, TaxCategory::IncomeTax);
}

/// Validates the non-filer penalty: payment for goods to a non-filer
/// at 9.0% (double the filer rate) under ITO 2001 Section 153(1)(a).
///
/// This differential rate is Pakistan's mechanism to incentivize
/// tax registration on the Active Taxpayer List (ATL).
#[test]
fn withholding_goods_nonfiler_double_rate() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    );
    // Default is NonFiler.

    let results = engine.compute(&event);
    assert_eq!(results.len(), 1);

    let r = &results[0];
    assert_eq!(r.rate_percent, "9.0");
    assert_eq!(r.withholding_amount, "9000.00");
    assert_eq!(r.net_amount, "91000.00");

    // Non-filer rate should be exactly double the filer rate.
    let filer_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let filer_results = engine.compute(&filer_event);
    let filer_rate: f64 = filer_results[0].rate_percent.parse().expect("valid rate");
    let nonfiler_rate: f64 = r.rate_percent.parse().expect("valid rate");
    assert!(
        (nonfiler_rate - filer_rate * 2.0).abs() < f64::EPSILON,
        "non-filer rate ({nonfiler_rate}%) should be double filer rate ({filer_rate}%)"
    );
}

/// Validates withholding on services (ITO 2001 Section 153(1)(b)):
/// filer at 8.0%, non-filer at 16.0%. Both adjustable (not final tax).
#[test]
fn withholding_services_filer_vs_nonfiler() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let filer_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForServices,
        "PK",
        "200000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("7654321", FilerStatus::Filer);

    let nonfiler_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForServices,
        "PK",
        "200000",
        "PKR",
        "2025-2026",
    );

    let filer_results = engine.compute(&filer_event);
    let nonfiler_results = engine.compute(&nonfiler_event);

    assert_eq!(filer_results.len(), 1);
    assert_eq!(nonfiler_results.len(), 1);

    // Filer: 200,000 * 8% = 16,000
    assert_eq!(filer_results[0].rate_percent, "8.0");
    assert_eq!(filer_results[0].withholding_amount, "16000.00");
    assert_eq!(filer_results[0].net_amount, "184000.00");

    // Non-filer: 200,000 * 16% = 32,000
    assert_eq!(nonfiler_results[0].rate_percent, "16.0");
    assert_eq!(nonfiler_results[0].withholding_amount, "32000.00");
    assert_eq!(nonfiler_results[0].net_amount, "168000.00");

    // Non-filer withholding is exactly double filer withholding.
    let filer_wht = parse_amount(&filer_results[0].withholding_amount).expect("valid amount");
    let nonfiler_wht = parse_amount(&nonfiler_results[0].withholding_amount).expect("valid amount");
    assert_eq!(
        nonfiler_wht,
        filer_wht * 2,
        "non-filer WHT should be exactly double filer WHT"
    );
}

/// Validates profit on debt withholding (ITO 2001 Section 151):
/// filer at 15.0%, non-filer at 30.0%. Both are final tax (no further liability).
#[test]
fn withholding_profit_on_debt_is_final_tax() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let filer_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::ProfitOnDebt,
        "PK",
        "1000000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let nonfiler_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::ProfitOnDebt,
        "PK",
        "1000000",
        "PKR",
        "2025-2026",
    );

    let filer_results = engine.compute(&filer_event);
    let nonfiler_results = engine.compute(&nonfiler_event);

    // Filer: 1,000,000 * 15% = 150,000
    assert_eq!(filer_results[0].rate_percent, "15.0");
    assert_eq!(filer_results[0].withholding_amount, "150000.00");
    assert!(
        filer_results[0].is_final_tax,
        "profit on debt WHT is final tax for filer"
    );

    // Non-filer: 1,000,000 * 30% = 300,000
    assert_eq!(nonfiler_results[0].rate_percent, "30.0");
    assert_eq!(nonfiler_results[0].withholding_amount, "300000.00");
    assert!(
        nonfiler_results[0].is_final_tax,
        "profit on debt WHT is final tax for non-filer"
    );
}

/// Validates dividend withholding (ITO 2001 Section 150):
/// filer at 15.0%, non-filer at 30.0%. Both are final tax.
#[test]
fn withholding_dividend_rates_and_final_tax() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let filer_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::DividendDistribution,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let nonfiler_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::DividendDistribution,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    );

    let filer_results = engine.compute(&filer_event);
    let nonfiler_results = engine.compute(&nonfiler_event);

    assert_eq!(filer_results[0].rate_percent, "15.0");
    assert_eq!(filer_results[0].withholding_amount, "75000.00");
    assert!(filer_results[0].is_final_tax);

    assert_eq!(nonfiler_results[0].rate_percent, "30.0");
    assert_eq!(nonfiler_results[0].withholding_amount, "150000.00");
    assert!(nonfiler_results[0].is_final_tax);
}

/// Validates cross-border payment withholding (ITO 2001 Section 152):
/// 20.0% for all filer statuses (residents paying non-residents). Final tax.
#[test]
fn withholding_crossborder_uniform_rate() {
    let engine = WithholdingEngine::with_pakistan_rules();

    for filer_status in &[FilerStatus::Filer, FilerStatus::NonFiler, FilerStatus::LateFiler] {
        let mut event = TaxEvent::new(
            Uuid::new_v4(),
            TaxEventType::CrossBorderPayment,
            "PK",
            "1000000",
            "PKR",
            "2025-2026",
        );
        event.filer_status = *filer_status;
        if *filer_status == FilerStatus::Filer {
            event = event.with_ntn("1234567", FilerStatus::Filer);
        }

        let results = engine.compute(&event);
        assert_eq!(
            results.len(),
            1,
            "cross-border should match for filer_status={filer_status}"
        );
        assert_eq!(
            results[0].rate_percent, "20.0",
            "cross-border rate should be 20% for all filer statuses"
        );
        assert_eq!(results[0].withholding_amount, "200000.00");
        assert_eq!(results[0].statutory_section, "ITO 2001 Section 152");
        assert!(results[0].is_final_tax);
    }
}

/// Validates salary withholding (ITO 2001 Section 149): only applies
/// above the PKR 5,000,000 threshold (encoded as "5000000" in the rule).
///
/// This tests the threshold-based rule filtering: salaries below the
/// threshold produce zero withholding.
#[test]
fn withholding_salary_threshold_filtering() {
    let engine = WithholdingEngine::with_pakistan_rules();

    // Below threshold: PKR 4,000,000 (below 5,000,000 threshold).
    let below_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SalaryPayment,
        "PK",
        "4000000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let below_results = engine.compute(&below_event);
    assert!(
        below_results.is_empty(),
        "salary below threshold should produce no withholding"
    );

    // Above threshold: PKR 6,000,000.
    let above_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SalaryPayment,
        "PK",
        "6000000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let above_results = engine.compute(&above_event);
    assert_eq!(
        above_results.len(),
        1,
        "salary above threshold should produce withholding"
    );
    assert_eq!(above_results[0].rate_percent, "5.0");
    assert_eq!(above_results[0].statutory_section, "ITO 2001 Section 149");
    // 6,000,000 * 5% = 300,000
    assert_eq!(above_results[0].withholding_amount, "300000.00");
    assert_eq!(above_results[0].net_amount, "5700000.00");
}

/// Validates rent payment withholding (ITO 2001 Section 155):
/// filer at 15%, non-filer at 30%. Both are adjustable (not final tax).
#[test]
fn withholding_rent_adjustable_not_final() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let filer_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::RentPayment,
        "PK",
        "120000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let nonfiler_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::RentPayment,
        "PK",
        "120000",
        "PKR",
        "2025-2026",
    );

    let filer_results = engine.compute(&filer_event);
    let nonfiler_results = engine.compute(&nonfiler_event);

    assert_eq!(filer_results.len(), 1);
    assert_eq!(filer_results[0].rate_percent, "15.0");
    // 120,000 * 15% = 18,000
    assert_eq!(filer_results[0].withholding_amount, "18000.00");
    assert!(
        !filer_results[0].is_final_tax,
        "rent WHT under S155 should be adjustable, not final"
    );

    assert_eq!(nonfiler_results.len(), 1);
    assert_eq!(nonfiler_results[0].rate_percent, "30.0");
    // 120,000 * 30% = 36,000
    assert_eq!(nonfiler_results[0].withholding_amount, "36000.00");
    assert!(!nonfiler_results[0].is_final_tax);
}

/// Validates sales tax computation (Sales Tax Act 1990 Section 3):
/// 18% standard rate on supply of goods and services, applicable to
/// all filer statuses. Sales tax is final (no adjustability).
#[test]
fn withholding_sales_tax_standard_rate() {
    let engine = WithholdingEngine::with_pakistan_rules();

    // Supply of goods.
    let goods_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    );

    let goods_results = engine.compute(&goods_event);
    assert_eq!(goods_results.len(), 1);
    assert_eq!(goods_results[0].rate_percent, "18.0");
    // 100,000 * 18% = 18,000
    assert_eq!(goods_results[0].withholding_amount, "18000.00");
    assert_eq!(goods_results[0].net_amount, "82000.00");
    assert_eq!(goods_results[0].tax_category, TaxCategory::SalesTax);
    assert_eq!(
        goods_results[0].statutory_section,
        "Sales Tax Act 1990 Section 3"
    );
    assert!(goods_results[0].is_final_tax);

    // Supply of services.
    let services_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfServices,
        "PK",
        "75000",
        "PKR",
        "2025-2026",
    );

    let services_results = engine.compute(&services_event);
    assert_eq!(services_results.len(), 1);
    assert_eq!(services_results[0].rate_percent, "18.0");
    // 75,000 * 18% = 13,500
    assert_eq!(services_results[0].withholding_amount, "13500.00");
    assert_eq!(services_results[0].net_amount, "61500.00");
    assert_eq!(services_results[0].tax_category, TaxCategory::SalesTax);
}

/// Validates that the engine returns empty results when no rules are loaded
/// for the event's jurisdiction. This is correct behavior — not an error.
#[test]
fn withholding_unknown_jurisdiction_yields_zero() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "AE",
        "100000",
        "AED",
        "2025",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    assert!(
        results.is_empty(),
        "no rules loaded for AE jurisdiction, should yield zero withholding"
    );
}

/// Validates that unparseable gross amounts result in zero withholding
/// (graceful degradation, not a panic).
#[test]
fn withholding_unparseable_amount_yields_zero() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "not-a-number",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    assert!(
        results.is_empty(),
        "unparseable amount should yield zero withholding, not panic"
    );
}

/// Validates withholding computation is deterministic: identical inputs
/// always produce identical outputs. This is critical for audit
/// reproducibility and is the tax pipeline analog of Theorem 17.1.
#[test]
fn withholding_computation_is_deterministic() {
    let engine = WithholdingEngine::with_pakistan_rules();
    let entity_id = Uuid::new_v4();

    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "123456.78",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let baseline = engine.compute(&event);
    assert!(!baseline.is_empty(), "should produce at least one result");

    // Run 10 times, verify identical outputs each time.
    for i in 0..10 {
        let result = engine.compute(&event);
        assert_eq!(
            result.len(),
            baseline.len(),
            "iteration {i}: result count differs"
        );
        for (a, b) in result.iter().zip(baseline.iter()) {
            assert_eq!(a.rule_id, b.rule_id, "iteration {i}: rule_id differs");
            assert_eq!(
                a.withholding_amount, b.withholding_amount,
                "iteration {i}: withholding_amount differs"
            );
            assert_eq!(
                a.net_amount, b.net_amount,
                "iteration {i}: net_amount differs"
            );
            assert_eq!(
                a.rate_percent, b.rate_percent,
                "iteration {i}: rate_percent differs"
            );
        }
    }
}

/// Validates that LateFiler status is treated the same as NonFiler for
/// all Pakistan rules where both appear in applicable_filer_status.
#[test]
fn late_filer_matches_nonfiler_rules() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let late_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::LateFiler);

    let nonfiler_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    );

    let late_results = engine.compute(&late_event);
    let nonfiler_results = engine.compute(&nonfiler_event);

    assert_eq!(late_results.len(), 1);
    assert_eq!(nonfiler_results.len(), 1);
    assert_eq!(
        late_results[0].rate_percent, nonfiler_results[0].rate_percent,
        "late filer should get the same rate as non-filer"
    );
    assert_eq!(
        late_results[0].withholding_amount, nonfiler_results[0].withholding_amount,
        "late filer should get the same withholding as non-filer"
    );
}

/// Validates that custom rules can be loaded for non-Pakistan jurisdictions.
/// Tests UAE VAT at 5% as an example of the engine's jurisdiction-agnostic
/// design.
#[test]
fn custom_jurisdiction_rules_uae_vat() {
    let mut engine = WithholdingEngine::new();
    assert_eq!(engine.rule_count(), 0);

    engine.load_rules(
        "AE",
        vec![WithholdingRule {
            rule_id: "AE-VAT-5".into(),
            applicable_event_types: vec![
                TaxEventType::SupplyOfGoods,
                TaxEventType::SupplyOfServices,
            ],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::NonFiler,
                FilerStatus::LateFiler,
            ],
            tax_category: TaxCategory::SalesTax,
            rate_percent: "5.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "UAE VAT Federal Decree-Law No. 8 of 2017".into(),
            description: "UAE Standard VAT Rate".into(),
            effective_from: "2018-01-01".into(),
            effective_until: None,
            is_final_tax: true,
        }],
    );

    assert_eq!(engine.rule_count(), 1);

    // Supply of goods in UAE.
    let goods_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfGoods,
        "AE",
        "100000",
        "AED",
        "2025",
    )
    .with_tax_category(TaxCategory::SalesTax);

    let goods_results = engine.compute(&goods_event);
    assert_eq!(goods_results.len(), 1);
    assert_eq!(goods_results[0].rate_percent, "5.0");
    assert_eq!(goods_results[0].withholding_amount, "5000.00");
    assert_eq!(goods_results[0].net_amount, "95000.00");
    assert_eq!(goods_results[0].currency, "AED");

    // Supply of services in UAE (same rule applies).
    let services_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfServices,
        "AE",
        "50000",
        "AED",
        "2025",
    )
    .with_tax_category(TaxCategory::SalesTax);

    let services_results = engine.compute(&services_event);
    assert_eq!(services_results.len(), 1);
    assert_eq!(services_results[0].withholding_amount, "2500.00");
}

/// Validates that loading rules for a jurisdiction replaces any previously
/// loaded rules (not appends). This ensures rule updates from new SROs
/// cleanly override old rules.
#[test]
fn load_rules_replaces_existing() {
    let mut engine = WithholdingEngine::new();

    engine.load_rules(
        "PK",
        vec![WithholdingRule {
            rule_id: "OLD-RULE".into(),
            applicable_event_types: vec![TaxEventType::PaymentForGoods],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "10.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "OLD".into(),
            description: "old rule".into(),
            effective_from: "2020-01-01".into(),
            effective_until: None,
            is_final_tax: false,
        }],
    );

    assert_eq!(engine.rule_count(), 1);
    assert_eq!(engine.rules_for_jurisdiction("PK")[0].rule_id, "OLD-RULE");

    // Replace with a new rule.
    engine.load_rules(
        "PK",
        vec![WithholdingRule {
            rule_id: "NEW-RULE".into(),
            applicable_event_types: vec![TaxEventType::PaymentForGoods],
            applicable_filer_status: vec![FilerStatus::Filer],
            tax_category: TaxCategory::IncomeTax,
            rate_percent: "5.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "NEW".into(),
            description: "new rule".into(),
            effective_from: "2024-07-01".into(),
            effective_until: None,
            is_final_tax: false,
        }],
    );

    assert_eq!(engine.rule_count(), 1, "old rule should be replaced");
    assert_eq!(engine.rules_for_jurisdiction("PK")[0].rule_id, "NEW-RULE");
}

/// Validates that multiple matching rules both fire and results are sorted
/// by rule_id for determinism.
#[test]
fn multiple_matching_rules_accumulate_sorted() {
    let mut engine = WithholdingEngine::new();

    engine.load_rules(
        "PK",
        vec![
            WithholdingRule {
                rule_id: "TEST-RULE-B".into(),
                applicable_event_types: vec![TaxEventType::PaymentForGoods],
                applicable_filer_status: vec![FilerStatus::Filer],
                tax_category: TaxCategory::IncomeTax,
                rate_percent: "2.0".into(),
                threshold_min: "0".into(),
                threshold_max: None,
                statutory_section: "Test Section B".into(),
                description: "Test rule B".into(),
                effective_from: "2024-07-01".into(),
                effective_until: None,
                is_final_tax: false,
            },
            WithholdingRule {
                rule_id: "TEST-RULE-A".into(),
                applicable_event_types: vec![TaxEventType::PaymentForGoods],
                applicable_filer_status: vec![FilerStatus::Filer],
                tax_category: TaxCategory::IncomeTax,
                rate_percent: "4.5".into(),
                threshold_min: "0".into(),
                threshold_max: None,
                statutory_section: "Test Section A".into(),
                description: "Test rule A".into(),
                effective_from: "2024-07-01".into(),
                effective_until: None,
                is_final_tax: false,
            },
        ],
    );

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);

    // Both rules should match.
    assert_eq!(results.len(), 2);

    // Results must be sorted by rule_id (A before B) regardless of insertion order.
    assert_eq!(results[0].rule_id, "TEST-RULE-A");
    assert_eq!(results[1].rule_id, "TEST-RULE-B");

    // Rule A: 100,000 * 4.5% = 4,500.
    assert_eq!(results[0].withholding_amount, "4500.00");
    // Rule B: 100,000 * 2.0% = 2,000.
    assert_eq!(results[1].withholding_amount, "2000.00");
}

/// Validates small decimal PKR amounts are handled with correct precision.
#[test]
fn withholding_small_decimal_amount() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "1000.50",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].currency, "PKR");

    // 1000.50 in cents = 100050. * 450 bps / 10000 = 4502 cents = 45.02
    assert_eq!(results[0].withholding_amount, "45.02");
    assert_eq!(results[0].net_amount, "955.48");

    // Verify invariant: gross = withholding + net.
    let gross = parse_amount(&results[0].gross_amount).expect("valid gross");
    let wht = parse_amount(&results[0].withholding_amount).expect("valid wht");
    let net = parse_amount(&results[0].net_amount).expect("valid net");
    assert_eq!(gross, wht + net, "gross must equal withholding + net");
}

// ===========================================================================
// Stage 3: Report Generation
// ===========================================================================

/// Validates end-to-end: multiple tax events of different types are processed
/// through the pipeline and aggregated into a single report with correct
/// line items grouped by statutory section.
#[test]
fn report_aggregates_multiple_event_types() {
    let entity_id = Uuid::new_v4();
    let engine = WithholdingEngine::with_pakistan_rules();

    // Event 1: Payment for goods (S153, 4.5% filer)
    let event1 = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    // Event 2: Payment for services (S153, 8.0% filer)
    let event2 = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForServices,
        "PK",
        "200000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    // Event 3: Dividend distribution (S150, 15.0% filer)
    let event3 = TaxEvent::new(
        entity_id,
        TaxEventType::DividendDistribution,
        "PK",
        "300000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let mut all_results = engine.compute(&event1);
    all_results.extend(engine.compute(&event2));
    all_results.extend(engine.compute(&event3));

    assert_eq!(all_results.len(), 3, "three events should produce three results");

    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-07-01".into(),
            period_end: "2025-07-31".into(),
            report_type: "monthly_withholding".into(),
        },
        &all_results,
    );

    assert_eq!(report.entity_id, entity_id);
    assert_eq!(report.ntn.as_deref(), Some("1234567"));
    assert_eq!(report.jurisdiction_id, "PK");
    assert_eq!(report.tax_year, "2025-2026");
    assert_eq!(report.period_start, "2025-07-01");
    assert_eq!(report.period_end, "2025-07-31");
    assert_eq!(report.report_type, "monthly_withholding");
    assert_eq!(report.event_count, 3);
    assert_eq!(report.currency, "PKR");
    assert_eq!(report.status, ReportStatus::Generated);
    assert!(report.submitted_at.is_none());
    assert!(report.authority_reference.is_none());

    // Verify line items group by statutory section (3 distinct sections).
    assert_eq!(
        report.line_items.len(),
        3,
        "three distinct sections should produce three line items"
    );

    // Total withholding: 4,500 + 16,000 + 45,000 = 65,500
    assert_eq!(report.total_withholding, "65500.00");
    // Total gross: 100,000 + 200,000 + 300,000 = 600,000
    assert_eq!(report.total_gross, "600000.00");
}

/// Validates that multiple events under the same statutory section are
/// aggregated into a single line item in the report.
#[test]
fn report_aggregates_same_section_events() {
    let entity_id = Uuid::new_v4();
    let engine = WithholdingEngine::with_pakistan_rules();

    // Two separate goods payments, both under S153(1)(a).
    let event1 = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let event2 = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "200000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let mut all_results = engine.compute(&event1);
    all_results.extend(engine.compute(&event2));

    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-08-01".into(),
            period_end: "2025-08-31".into(),
            report_type: "monthly_withholding".into(),
        },
        &all_results,
    );

    assert_eq!(report.event_count, 2);
    // Both events are under the same statutory section, so one line item.
    assert_eq!(
        report.line_items.len(),
        1,
        "same section should aggregate into one line item"
    );
    assert_eq!(report.line_items[0].event_count, 2);
    // Gross: 100,000 + 200,000 = 300,000
    assert_eq!(report.line_items[0].total_gross, "300000.00");
    // Withholding: 4,500 + 9,000 = 13,500
    assert_eq!(report.line_items[0].total_withholding, "13500.00");
    assert_eq!(
        report.line_items[0].statutory_section,
        "ITO 2001 Section 153(1)(a)"
    );
    assert_eq!(report.line_items[0].tax_category, TaxCategory::IncomeTax);
}

/// Validates that an empty set of withholding results produces a valid
/// report with zero totals. This handles the edge case of a reporting
/// period with no taxable activity.
#[test]
fn report_empty_results_produces_zero_report() {
    let entity_id = Uuid::new_v4();

    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: None,
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-09-01".into(),
            period_end: "2025-09-30".into(),
            report_type: "monthly_withholding".into(),
        },
        &[],
    );

    assert_eq!(report.entity_id, entity_id);
    assert_eq!(report.event_count, 0);
    assert_eq!(report.total_gross, "0.00");
    assert_eq!(report.total_withholding, "0.00");
    assert!(report.line_items.is_empty());
    assert_eq!(report.status, ReportStatus::Generated);
    // Report should still have a valid UUID.
    assert_ne!(report.report_id, Uuid::nil());
}

/// Validates report line items include per-line totals that sum correctly
/// to the report-level totals.
#[test]
fn report_line_item_totals_sum_to_report_totals() {
    let entity_id = Uuid::new_v4();
    let engine = WithholdingEngine::with_pakistan_rules();

    let events = vec![
        TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForGoods,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
        TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForServices,
            "PK",
            "200000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
        TaxEvent::new(
            entity_id,
            TaxEventType::SupplyOfGoods,
            "PK",
            "300000",
            "PKR",
            "2025-2026",
        ),
    ];

    let mut all_results = Vec::new();
    for event in &events {
        all_results.extend(engine.compute(event));
    }

    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-07-01".into(),
            period_end: "2025-07-31".into(),
            report_type: "monthly_withholding".into(),
        },
        &all_results,
    );

    let report_total_gross = parse_amount(&report.total_gross).expect("valid total gross");
    let report_total_wht = parse_amount(&report.total_withholding).expect("valid total wht");

    let line_gross_sum: i64 = report
        .line_items
        .iter()
        .map(|li| parse_amount(&li.total_gross).expect("valid line gross"))
        .sum();
    let line_wht_sum: i64 = report
        .line_items
        .iter()
        .map(|li| parse_amount(&li.total_withholding).expect("valid line wht"))
        .sum();

    assert_eq!(
        report_total_gross, line_gross_sum,
        "report total gross must equal sum of line item grosses"
    );
    assert_eq!(
        report_total_wht, line_wht_sum,
        "report total withholding must equal sum of line item withholdings"
    );
}

// ===========================================================================
// Full Pipeline (TaxPipeline orchestrator)
// ===========================================================================

/// Validates the full pipeline orchestration: TaxPipeline::pakistan() creates
/// a pipeline with Pakistan rules pre-loaded and process_event routes
/// through the withholding engine correctly.
#[test]
fn pipeline_pakistan_full_flow() {
    let pipeline = TaxPipeline::pakistan();
    let entity_id = Uuid::new_v4();

    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = pipeline.process_event(&event);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].rate_percent, "4.5");
    // 500,000 * 4.5% = 22,500
    assert_eq!(results[0].withholding_amount, "22500.00");
    assert_eq!(results[0].net_amount, "477500.00");

    // Now generate a report from these results.
    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-07-01".into(),
            period_end: "2025-12-31".into(),
            report_type: "semi_annual_withholding".into(),
        },
        &results,
    );

    assert_eq!(report.event_count, 1);
    assert_eq!(report.total_withholding, "22500.00");
    assert_eq!(report.total_gross, "500000.00");
    assert_eq!(report.report_type, "semi_annual_withholding");
}

/// Validates the default pipeline (no rules) processes events safely
/// and returns empty results.
#[test]
fn pipeline_default_no_rules_yields_empty() {
    let pipeline = TaxPipeline::default();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = pipeline.process_event(&event);
    assert!(
        results.is_empty(),
        "default pipeline has no rules, should yield empty results"
    );
}

/// Validates the complete end-to-end pipeline: multiple events of different
/// types flow through classification, withholding, and report generation.
/// This is the integration test that proves the P1-009 pipeline operates
/// as a coherent unit.
///
/// Simulates a full month of economic activity for a Pakistani entity:
/// goods payments, service payments, dividends, rent, and supply of goods.
#[test]
fn end_to_end_multi_event_pipeline_to_report() {
    let pipeline = TaxPipeline::pakistan();
    let entity_id = Uuid::new_v4();

    // Simulate a month of economic activity for a single entity.
    let events = vec![
        // 1. Paid a supplier for goods (filer) -- S153, 4.5%
        TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForGoods,
            "PK",
            "500000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
        // 2. Paid a consultant for services (filer) -- S153, 8%
        TaxEvent::new(
            entity_id,
            TaxEventType::PaymentForServices,
            "PK",
            "300000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
        // 3. Distributed dividends (filer) -- S150, 15%
        TaxEvent::new(
            entity_id,
            TaxEventType::DividendDistribution,
            "PK",
            "1000000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
        // 4. Paid rent (non-filer landlord) -- S155, 30%
        TaxEvent::new(
            entity_id,
            TaxEventType::RentPayment,
            "PK",
            "100000",
            "PKR",
            "2025-2026",
        ),
        // 5. Supply of goods (sales tax) -- STA 1990, 18%
        TaxEvent::new(
            entity_id,
            TaxEventType::SupplyOfGoods,
            "PK",
            "200000",
            "PKR",
            "2025-2026",
        )
        .with_ntn("1234567", FilerStatus::Filer),
    ];

    // Process all events through the pipeline.
    let mut all_results = Vec::new();
    for event in &events {
        let results = pipeline.process_event(event);
        all_results.extend(results);
    }

    // Expected: 5 events -> 5 withholding results (one rule per event).
    assert_eq!(all_results.len(), 5, "each event should produce one result");

    // Generate the monthly report.
    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-10-01".into(),
            period_end: "2025-10-31".into(),
            report_type: "monthly_withholding".into(),
        },
        &all_results,
    );

    assert_eq!(report.event_count, 5);
    assert_eq!(report.jurisdiction_id, "PK");
    assert_eq!(report.tax_year, "2025-2026");

    // Verify line items cover distinct sections.
    // S153(1)(a), S153(1)(b), S150, S155, STA1990-S3 = 5 distinct sections.
    assert_eq!(
        report.line_items.len(),
        5,
        "should have 5 distinct line items, got {}",
        report.line_items.len()
    );

    // Verify total gross: 500k + 300k + 1M + 100k + 200k = 2.1M
    assert_eq!(report.total_gross, "2100000.00");

    // Verify total withholding:
    // Goods:    500,000 * 4.5%  = 22,500
    // Services: 300,000 * 8%    = 24,000
    // Dividend: 1,000,000 * 15% = 150,000
    // Rent:     100,000 * 30%   = 30,000  (non-filer rate)
    // Sales:    200,000 * 18%   = 36,000
    // Total:    262,500
    assert_eq!(report.total_withholding, "262500.00");

    // Verify report is in Generated status (not yet submitted).
    assert_eq!(report.status, ReportStatus::Generated);

    // Verify each line item by statutory section.
    for item in &report.line_items {
        match item.statutory_section.as_str() {
            "ITO 2001 Section 153(1)(a)" => {
                assert_eq!(item.total_withholding, "22500.00");
                assert_eq!(item.total_gross, "500000.00");
                assert_eq!(item.event_count, 1);
            }
            "ITO 2001 Section 153(1)(b)" => {
                assert_eq!(item.total_withholding, "24000.00");
                assert_eq!(item.total_gross, "300000.00");
                assert_eq!(item.event_count, 1);
            }
            "ITO 2001 Section 150" => {
                assert_eq!(item.total_withholding, "150000.00");
                assert_eq!(item.total_gross, "1000000.00");
                assert_eq!(item.event_count, 1);
            }
            "ITO 2001 Section 155" => {
                assert_eq!(item.total_withholding, "30000.00");
                assert_eq!(item.total_gross, "100000.00");
                assert_eq!(item.event_count, 1);
            }
            "Sales Tax Act 1990 Section 3" => {
                assert_eq!(item.total_withholding, "36000.00");
                assert_eq!(item.total_gross, "200000.00");
                assert_eq!(item.event_count, 1);
                assert_eq!(item.tax_category, TaxCategory::SalesTax);
            }
            other => panic!("Unexpected statutory section in report: {other}"),
        }
    }
}

// ===========================================================================
// Fixed-Precision Arithmetic
// ===========================================================================

/// Validates parse_amount and format_amount roundtrip for edge cases.
/// These utilities underpin all withholding computations and must handle
/// decimal precision correctly.
#[test]
fn amount_parsing_edge_cases() {
    // Whole numbers (no decimal) are treated as major units, converted to cents.
    assert_eq!(parse_amount("0"), Some(0));
    assert_eq!(parse_amount("1"), Some(100));
    assert_eq!(parse_amount("999999999"), Some(99_999_999_900));

    // Decimals.
    assert_eq!(parse_amount("0.01"), Some(1));
    assert_eq!(parse_amount("0.10"), Some(10));
    assert_eq!(parse_amount("0.99"), Some(99));
    assert_eq!(parse_amount("1.50"), Some(150));
    assert_eq!(parse_amount("100000.00"), Some(10_000_000));

    // Single decimal digit is padded.
    assert_eq!(parse_amount("1.5"), Some(150));

    // More than 2 decimal digits are truncated to 2.
    assert_eq!(parse_amount("1.999"), Some(199));

    // Whitespace is trimmed.
    assert_eq!(parse_amount("  500  "), Some(50_000));

    // Invalid strings return None.
    assert_eq!(parse_amount(""), None);
    assert_eq!(parse_amount("   "), None);
    assert_eq!(parse_amount("abc"), None);
}

/// Validates format_amount produces consistently formatted output with
/// exactly 2 decimal places.
#[test]
fn amount_formatting_consistency() {
    assert_eq!(format_amount(0), "0.00");
    assert_eq!(format_amount(1), "0.01");
    assert_eq!(format_amount(10), "0.10");
    assert_eq!(format_amount(99), "0.99");
    assert_eq!(format_amount(100), "1.00");
    assert_eq!(format_amount(10_000_000), "100000.00");
    assert_eq!(format_amount(123_456_789), "1234567.89");
    assert_eq!(format_amount(-500), "-5.00");
    assert_eq!(format_amount(-1), "-0.01");
}

/// Validates that parse_amount and format_amount are inverse operations
/// for well-formed inputs.
#[test]
fn amount_parse_format_roundtrip() {
    let test_values = [0, 1, 50, 99, 100, 12345, 1_000_000, 99_999_999];

    for cents in test_values {
        let formatted = format_amount(cents);
        let parsed = parse_amount(&formatted);
        assert_eq!(
            parsed,
            Some(cents),
            "roundtrip failed for {cents}: formatted as \"{formatted}\", parsed back as {parsed:?}"
        );
    }
}

/// Validates that withholding arithmetic preserves the invariant:
/// gross = withholding + net, for every computed result across all
/// event types and filer statuses.
#[test]
fn withholding_gross_equals_withholding_plus_net() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let test_cases: Vec<(TaxEventType, FilerStatus, &str)> = vec![
        (TaxEventType::PaymentForGoods, FilerStatus::Filer, "100000"),
        (
            TaxEventType::PaymentForGoods,
            FilerStatus::NonFiler,
            "100000",
        ),
        (
            TaxEventType::PaymentForServices,
            FilerStatus::Filer,
            "777777",
        ),
        (
            TaxEventType::DividendDistribution,
            FilerStatus::Filer,
            "1000000",
        ),
        (TaxEventType::RentPayment, FilerStatus::NonFiler, "250000"),
        (
            TaxEventType::CrossBorderPayment,
            FilerStatus::Filer,
            "5000000",
        ),
        (TaxEventType::SupplyOfGoods, FilerStatus::Filer, "333333"),
        (
            TaxEventType::ProfitOnDebt,
            FilerStatus::NonFiler,
            "999999.99",
        ),
    ];

    for (event_type, filer_status, amount) in &test_cases {
        let mut event = TaxEvent::new(
            Uuid::new_v4(),
            *event_type,
            "PK",
            *amount,
            "PKR",
            "2025-2026",
        );
        event.filer_status = *filer_status;
        if *filer_status == FilerStatus::Filer {
            event = event.with_ntn("1234567", FilerStatus::Filer);
        }

        let results = engine.compute(&event);
        for r in &results {
            let gross_cents = parse_amount(&r.gross_amount).expect("valid gross");
            let wht_cents = parse_amount(&r.withholding_amount).expect("valid wht");
            let net_cents = parse_amount(&r.net_amount).expect("valid net");

            assert_eq!(
                gross_cents,
                wht_cents + net_cents,
                "invariant violated for {:?}/{:?}/{}: gross({}) != wht({}) + net({})",
                event_type,
                filer_status,
                amount,
                r.gross_amount,
                r.withholding_amount,
                r.net_amount
            );
        }
    }
}

// ===========================================================================
// Cross-Crate Composition: Agentic Policy Engine + Tax Pipeline
// ===========================================================================

/// Validates that the tax pipeline's TaxEventType taxonomy and the agentic
/// policy engine's TriggerType taxonomy are distinct but independently
/// complete. The policy engine generates triggers; the tax pipeline classifies
/// events. This test verifies both taxonomies maintain their expected sizes
/// and string representations.
#[test]
fn tax_and_agentic_taxonomies_are_independently_complete() {
    use msez_agentic::policy::TriggerType;

    // Tax event types: 16 variants covering all Pakistani tax statutes.
    let tax_types = TaxEventType::all();
    assert_eq!(
        tax_types.len(),
        16,
        "TaxEventType should have 16 variants"
    );

    // Every tax event type has a non-empty string representation.
    for t in tax_types {
        let s = t.as_str();
        assert!(
            !s.is_empty(),
            "TaxEventType::{:?} has empty as_str()",
            t
        );
    }

    // Tax categories: 7 variants.
    let categories = TaxCategory::all();
    assert_eq!(categories.len(), 7, "TaxCategory should have 7 variants");

    // TriggerType::WithholdingDue is the bridge point for tax event
    // generation from fiscal triggers in the agentic policy engine.
    let fiscal_trigger = TriggerType::WithholdingDue;
    assert_eq!(
        fiscal_trigger.as_str(),
        "withholding_due",
        "withholding trigger should exist for tax pipeline bridge"
    );

    // TriggerType::TaxYearEnd triggers annual compliance evaluation.
    let annual_trigger = TriggerType::TaxYearEnd;
    assert_eq!(
        annual_trigger.as_str(),
        "tax_year_end",
        "tax year end trigger should exist for annual compliance"
    );
}

/// Validates that the policy engine can process a WithholdingDue trigger
/// (the bridge between fiscal events and tax pipeline) without panicking,
/// and that re-processing the same trigger type produces deterministic results.
#[test]
fn policy_engine_fiscal_trigger_deterministic() {
    use msez_agentic::evaluation::PolicyEngine;
    use msez_agentic::policy::{Trigger, TriggerType};

    let mut engine = PolicyEngine::with_extended_policies();

    let trigger = Trigger::new(
        TriggerType::WithholdingDue,
        serde_json::json!({
            "amount": "1000000",
            "currency": "PKR",
            "entity_id": Uuid::new_v4().to_string(),
            "tax_year": "2025-2026"
        }),
    );

    let first_actions = engine.process_trigger(&trigger, "entity:pk-corp-001", Some("PK"));

    // Process the same trigger type again.
    let second_trigger = Trigger::new(
        TriggerType::WithholdingDue,
        serde_json::json!({
            "amount": "1000000",
            "currency": "PKR",
            "entity_id": Uuid::new_v4().to_string(),
            "tax_year": "2025-2026"
        }),
    );
    let second_actions = engine.process_trigger(&second_trigger, "entity:pk-corp-001", Some("PK"));

    // Determinism: same trigger type on same asset produces consistent action count.
    assert_eq!(
        first_actions.len(),
        second_actions.len(),
        "policy engine must produce deterministic results for identical trigger types"
    );
}

// ===========================================================================
// Multi-jurisdiction Pipeline
// ===========================================================================

/// Validates that a single WithholdingEngine can hold rules for multiple
/// jurisdictions simultaneously and route events to the correct rule set.
/// This tests the multi-jurisdiction design needed for corridor operations
/// (e.g., PAK<->UAE corridor).
#[test]
fn multi_jurisdiction_engine_routes_correctly() {
    let mut engine = WithholdingEngine::new();

    // Load Pakistan rules.
    engine.load_rules("PK", pakistan_standard_rules());

    // Load UAE VAT rule.
    engine.load_rules(
        "AE",
        vec![WithholdingRule {
            rule_id: "AE-VAT-5".into(),
            applicable_event_types: vec![TaxEventType::SupplyOfGoods],
            applicable_filer_status: vec![
                FilerStatus::Filer,
                FilerStatus::NonFiler,
                FilerStatus::LateFiler,
            ],
            tax_category: TaxCategory::SalesTax,
            rate_percent: "5.0".into(),
            threshold_min: "0".into(),
            threshold_max: None,
            statutory_section: "UAE VAT".into(),
            description: "UAE VAT".into(),
            effective_from: "2018-01-01".into(),
            effective_until: None,
            is_final_tax: true,
        }],
    );

    // PK event should get Pakistan rates.
    let pk_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let pk_results = engine.compute(&pk_event);
    assert_eq!(pk_results.len(), 1);
    assert_eq!(pk_results[0].rate_percent, "4.5");

    // AE event should get UAE rates.
    let ae_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfGoods,
        "AE",
        "100000",
        "AED",
        "2025",
    )
    .with_tax_category(TaxCategory::SalesTax);

    let ae_results = engine.compute(&ae_event);
    assert_eq!(ae_results.len(), 1);
    assert_eq!(ae_results[0].rate_percent, "5.0");
    assert_eq!(ae_results[0].currency, "AED");

    // Unknown jurisdiction should get nothing.
    let gb_event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "GB",
        "100000",
        "GBP",
        "2025-2026",
    );

    let gb_results = engine.compute(&gb_event);
    assert!(gb_results.is_empty());
}

// ===========================================================================
// Serde Fidelity
// ===========================================================================

/// Validates that TaxEvent survives JSON serialization roundtrip without
/// data loss. This is critical for the FBR IRIS reporting pipeline where
/// tax events are serialized for transmission and audit persistence.
#[test]
fn tax_event_serde_roundtrip_preserves_all_fields() {
    let entity_id = Uuid::new_v4();
    let tx_id = Uuid::new_v4();
    let pay_id = Uuid::new_v4();
    let counter_id = Uuid::new_v4();

    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForServices,
        "PK",
        "999999.99",
        "PKR",
        "2025-2026",
    )
    .with_ntn("7654321", FilerStatus::Filer)
    .with_statutory_section("ITO 2001 Section 153(1)(b)")
    .with_mass_transaction(tx_id)
    .with_mass_payment(pay_id)
    .with_counterparty(counter_id)
    .with_metadata(serde_json::json!({"source": "test"}));

    let json = serde_json::to_string(&event).expect("serialize");
    let deserialized: TaxEvent = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.entity_id, entity_id);
    assert_eq!(deserialized.event_type, TaxEventType::PaymentForServices);
    assert_eq!(deserialized.tax_category, TaxCategory::IncomeTax);
    assert_eq!(deserialized.jurisdiction_id, "PK");
    assert_eq!(deserialized.gross_amount, "999999.99");
    assert_eq!(deserialized.currency, "PKR");
    assert_eq!(deserialized.tax_year, "2025-2026");
    assert_eq!(deserialized.ntn.as_deref(), Some("7654321"));
    assert_eq!(deserialized.filer_status, FilerStatus::Filer);
    assert_eq!(
        deserialized.statutory_section.as_deref(),
        Some("ITO 2001 Section 153(1)(b)")
    );
    assert_eq!(deserialized.mass_transaction_id, Some(tx_id));
    assert_eq!(deserialized.mass_payment_id, Some(pay_id));
    assert_eq!(deserialized.counterparty_entity_id, Some(counter_id));
    assert_eq!(deserialized.metadata["source"], "test");
}

/// Validates that WithholdingResult survives JSON roundtrip. These results
/// are serialized in API responses and stored for audit.
#[test]
fn withholding_result_serde_roundtrip() {
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    assert_eq!(results.len(), 1);

    let json = serde_json::to_string(&results[0]).expect("serialize");
    let deserialized: tax::WithholdingResult = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.rule_id, results[0].rule_id);
    assert_eq!(deserialized.rate_percent, results[0].rate_percent);
    assert_eq!(
        deserialized.withholding_amount,
        results[0].withholding_amount
    );
    assert_eq!(deserialized.net_amount, results[0].net_amount);
    assert_eq!(deserialized.tax_category, results[0].tax_category);
    assert_eq!(
        deserialized.statutory_section,
        results[0].statutory_section
    );
    assert_eq!(deserialized.is_final_tax, results[0].is_final_tax);
}

/// Validates that TaxReport survives JSON roundtrip with line items intact.
#[test]
fn tax_report_serde_roundtrip() {
    let entity_id = Uuid::new_v4();
    let engine = WithholdingEngine::with_pakistan_rules();

    let event = TaxEvent::new(
        entity_id,
        TaxEventType::DividendDistribution,
        "PK",
        "1000000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);

    let results = engine.compute(&event);
    let report = generate_report(
        &ReportParams {
            entity_id,
            ntn: Some("1234567".into()),
            jurisdiction_id: "PK".into(),
            tax_year: "2025-2026".into(),
            period_start: "2025-07-01".into(),
            period_end: "2025-07-31".into(),
            report_type: "monthly_withholding".into(),
        },
        &results,
    );

    let json = serde_json::to_string(&report).expect("serialize");
    let deserialized: tax::TaxReport = serde_json::from_str(&json).expect("deserialize");

    assert_eq!(deserialized.entity_id, report.entity_id);
    assert_eq!(deserialized.report_id, report.report_id);
    assert_eq!(deserialized.total_gross, report.total_gross);
    assert_eq!(deserialized.total_withholding, report.total_withholding);
    assert_eq!(deserialized.event_count, report.event_count);
    assert_eq!(deserialized.line_items.len(), report.line_items.len());
    assert_eq!(deserialized.status, report.status);
    assert_eq!(deserialized.jurisdiction_id, report.jurisdiction_id);
    assert_eq!(deserialized.tax_year, report.tax_year);
}

/// Validates that ReportStatus enum variants serialize to expected snake_case
/// strings and roundtrip correctly through JSON.
#[test]
fn report_status_serde_values() {
    let cases = [
        (ReportStatus::Generated, "\"generated\""),
        (ReportStatus::Submitted, "\"submitted\""),
        (ReportStatus::Acknowledged, "\"acknowledged\""),
        (ReportStatus::Rejected, "\"rejected\""),
        (ReportStatus::Accepted, "\"accepted\""),
    ];

    for (status, expected_json) in &cases {
        let json = serde_json::to_string(status).expect("serialize");
        assert_eq!(
            &json, expected_json,
            "ReportStatus::{:?} should serialize to {expected_json}",
            status
        );

        let deserialized: ReportStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(
            *status, deserialized,
            "ReportStatus roundtrip failed for {expected_json}"
        );
    }
}
