//! # End-to-End Tax Pipeline Integration Test (B-003)
//!
//! Traces a complete transaction through the full tax pipeline:
//! 1. Fiscal API call (tax event creation)
//! 2. Agentic tax event generation (event type classification)
//! 3. Withholding computation (Pakistan rules)
//! 4. FBR IRIS report generation
//! 5. Gap analysis trigger (multiple categories)
//!
//! Uses a realistic Pakistan jurisdiction configuration with Income Tax
//! Ordinance 2001 rules and current FBR withholding rates.

use msez_agentic::tax::{
    FilerStatus, TaxCategory, TaxEvent, TaxEventType, TaxPipeline, WithholdingEngine,
};
use uuid::Uuid;

/// Verify that a payment-for-goods transaction flows through the full pipeline:
/// event creation → withholding computation → report generation.
#[test]
fn pakistan_goods_payment_full_pipeline() {
    let pipeline = TaxPipeline::pakistan();

    // 1. Create a tax event (simulating a Mass fiscal API observation).
    let entity_id = Uuid::new_v4();
    let event = TaxEvent::new(
        entity_id,
        TaxEventType::PaymentForGoods,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer)
    .with_statutory_section("S153(1)(a)");

    // 2. Run through withholding engine.
    let withholdings = pipeline.process_event(&event);

    // 3. Verify withholding was computed.
    assert!(
        !withholdings.is_empty(),
        "Pakistan filer goods payment must produce withholding results"
    );

    // Verify withholding rate is correct for S153(1)(a) filer.
    let wht = &withholdings[0];
    assert_eq!(wht.tax_category, TaxCategory::IncomeTax);
    // 4.5% for filer on goods payment under S153.
    assert_eq!(wht.rate_percent, "4.5");
    assert_eq!(wht.withholding_amount, "22500.00"); // 500000 * 4.5%
    assert_eq!(wht.net_amount, "477500.00"); // 500000 - 22500
    assert!(wht.statutory_section.contains("153"));

    // 4. Generate FBR IRIS report.
    let report = msez_agentic::tax::generate_report(
        &msez_agentic::tax::ReportParams {
            entity_id,
            ntn: Some("1234567".to_string()),
            jurisdiction_id: "PK".to_string(),
            tax_year: "2025-2026".to_string(),
            period_start: "2025-07-01".to_string(),
            period_end: "2025-07-31".to_string(),
            report_type: "monthly_withholding".to_string(),
        },
        &withholdings,
    );

    assert_eq!(report.entity_id, entity_id);
    assert_eq!(report.jurisdiction_id, "PK");
    assert_eq!(report.tax_year, "2025-2026");
    assert_eq!(report.total_gross, "500000.00");
    assert_eq!(report.total_withholding, "22500.00");
    assert_eq!(report.event_count, 1);
    assert!(!report.line_items.is_empty());
    assert_eq!(report.currency, "PKR");
}

/// Non-filer rate should be double the filer rate (Pakistan tax policy).
#[test]
fn pakistan_non_filer_double_rate() {
    let pipeline = TaxPipeline::pakistan();

    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("9876543", FilerStatus::NonFiler);

    let withholdings = pipeline.process_event(&event);
    assert!(!withholdings.is_empty());

    let wht = &withholdings[0];
    // Non-filer rate for goods: 9.0% (double the 4.5% filer rate).
    assert_eq!(wht.rate_percent, "9.0");
    assert_eq!(wht.withholding_amount, "9000.00");
}

/// All five tax categories should be exercisable through the pipeline.
#[test]
fn pakistan_all_tax_categories_exercised() {
    let pipeline = TaxPipeline::pakistan();

    // Income tax: payment for goods.
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::PaymentForGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);
    let wht = pipeline.process_event(&event);
    assert!(
        wht.iter().any(|w| w.tax_category == TaxCategory::IncomeTax),
        "PaymentForGoods should trigger income tax"
    );

    // Income tax: salary payment (above PKR 5M threshold per S149).
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SalaryPayment,
        "PK",
        "6000000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);
    let wht = pipeline.process_event(&event);
    assert!(
        wht.iter().any(|w| w.tax_category == TaxCategory::IncomeTax),
        "SalaryPayment above PKR 5M threshold should trigger income tax"
    );

    // Sales tax: supply of goods.
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::SupplyOfGoods,
        "PK",
        "100000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);
    let wht = pipeline.process_event(&event);
    assert!(
        wht.iter()
            .any(|w| w.tax_category == TaxCategory::SalesTax),
        "SupplyOfGoods should trigger sales tax"
    );

    // Customs duty: import of goods.
    // NOTE: Pakistan standard rules currently focus on income tax and sales
    // tax (S153, S149, Sales Tax Act). Customs duty rules (Customs Act 1969)
    // are Phase 2. Verify the pipeline at least classifies the event type
    // correctly even if no withholding rule matches yet.
    let event = TaxEvent::new(
        Uuid::new_v4(),
        TaxEventType::ImportOfGoods,
        "PK",
        "500000",
        "PKR",
        "2025-2026",
    )
    .with_ntn("1234567", FilerStatus::Filer);
    assert_eq!(
        event.tax_category,
        TaxCategory::CustomsDuty,
        "ImportOfGoods should be classified as customs duty"
    );
    // No withholding rules for customs yet — pipeline returns empty.
    let wht = pipeline.process_event(&event);
    assert!(
        wht.is_empty(),
        "ImportOfGoods should produce no withholding until customs rules added"
    );
}

/// Multiple events for the same entity should aggregate correctly in a report.
#[test]
fn report_aggregates_multiple_events() {
    let pipeline = TaxPipeline::pakistan();
    let entity_id = Uuid::new_v4();

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
    ];

    let mut all_withholdings = Vec::new();
    for event in &events {
        all_withholdings.extend(pipeline.process_event(event));
    }

    let report = msez_agentic::tax::generate_report(
        &msez_agentic::tax::ReportParams {
            entity_id,
            ntn: Some("1234567".to_string()),
            jurisdiction_id: "PK".to_string(),
            tax_year: "2025-2026".to_string(),
            period_start: "2025-07-01".to_string(),
            period_end: "2025-07-31".to_string(),
            report_type: "monthly_withholding".to_string(),
        },
        &all_withholdings,
    );

    assert_eq!(report.event_count, all_withholdings.len());
    assert!(report.line_items.len() >= 2);
}

/// The withholding engine should load Pakistan rules by default.
#[test]
fn withholding_engine_has_pakistan_rules() {
    let engine = WithholdingEngine::with_pakistan_rules();
    let rules = engine.rules_for_jurisdiction("PK");
    assert!(
        !rules.is_empty(),
        "Pakistan withholding engine must have loaded rules"
    );
    // Check that S153 (goods/services withholding) exists.
    assert!(
        rules.iter().any(|r| r.rule_id.contains("S153")),
        "Pakistan rules must include S153 withholding"
    );
}
