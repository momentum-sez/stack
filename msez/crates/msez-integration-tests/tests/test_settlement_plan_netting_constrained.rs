//! # Constrained Settlement Plan Netting Test
//!
//! Tests the settlement netting engine for bilateral and multilateral
//! obligation compression. Verifies that netting preserves total obligations
//! while reducing the number and amount of settlement legs, and that
//! edge cases (single obligation, perfectly balanced) are handled correctly.

use msez_corridor::{NettingEngine, NettingError, Obligation};

fn obligation(from: &str, to: &str, amount: i64, currency: &str) -> Obligation {
    Obligation {
        from_party: from.to_string(),
        to_party: to.to_string(),
        amount,
        currency: currency.to_string(),
        corridor_id: None,
        priority: 0,
    }
}

// ---------------------------------------------------------------------------
// 1. Bilateral netting reduces flows
// ---------------------------------------------------------------------------

#[test]
fn bilateral_netting_reduces_flows() {
    let mut engine = NettingEngine::new();
    // A owes B 100 USD, B owes A 60 USD
    engine
        .add_obligation(obligation("A", "B", 100, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("B", "A", 60, "USD"))
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 160);
    assert_eq!(plan.net_total, 40);
    assert_eq!(plan.settlement_legs.len(), 1);
    assert_eq!(plan.settlement_legs[0].from_party, "A");
    assert_eq!(plan.settlement_legs[0].to_party, "B");
    assert_eq!(plan.settlement_legs[0].amount, 40);
    assert!(plan.reduction_percentage > 0.0);
}

// ---------------------------------------------------------------------------
// 2. Multilateral netting compression
// ---------------------------------------------------------------------------

#[test]
fn multilateral_netting_compression() {
    let mut engine = NettingEngine::new();
    // A -> B: 100, B -> C: 80, C -> A: 60
    engine
        .add_obligation(obligation("A", "B", 100, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("B", "C", 80, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("C", "A", 60, "USD"))
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 240);
    assert!(plan.net_total < plan.gross_total);
    // Netting should reduce the total settlement amount
    assert!(plan.reduction_percentage > 0.0);
}

// ---------------------------------------------------------------------------
// 3. Netting preserves total obligations
// ---------------------------------------------------------------------------

#[test]
fn netting_preserves_total_obligations() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(obligation("A", "B", 100, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("B", "C", 150, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("C", "D", 200, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("D", "A", 80, "USD"))
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 530);
    assert_eq!(plan.obligations.len(), 4);

    // Net positions should sum to zero (conservation of obligations)
    let net_sum: i64 = plan.net_positions.iter().map(|p| p.net).sum();
    assert_eq!(net_sum, 0, "net positions must sum to zero");
}

// ---------------------------------------------------------------------------
// 4. Single obligation: no netting possible
// ---------------------------------------------------------------------------

#[test]
fn single_obligation_no_netting() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(obligation("A", "B", 500, "PKR"))
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.gross_total, 500);
    assert_eq!(plan.net_total, 500);
    assert_eq!(plan.settlement_legs.len(), 1);
    // Zero reduction for single obligation
    assert!((plan.reduction_percentage - 0.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// 5. Perfectly balanced obligations net to zero
// ---------------------------------------------------------------------------

#[test]
fn perfectly_balanced_nets_to_zero() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(obligation("A", "B", 100, "USD"))
        .unwrap();
    engine
        .add_obligation(obligation("B", "A", 100, "USD"))
        .unwrap();

    let plan = engine.compute_plan().unwrap();
    assert_eq!(plan.settlement_legs.len(), 0);
    assert_eq!(plan.net_total, 0);
    assert!((plan.reduction_percentage - 100.0).abs() < f64::EPSILON);
}

// ---------------------------------------------------------------------------
// 6. Empty obligations produce error
// ---------------------------------------------------------------------------

#[test]
fn empty_obligations_error() {
    let engine = NettingEngine::new();
    assert!(matches!(
        engine.compute_plan(),
        Err(NettingError::NoObligations)
    ));
}
