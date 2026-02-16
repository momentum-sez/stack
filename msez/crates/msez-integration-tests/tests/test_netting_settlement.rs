//! # Corridor Netting Integration Tests (S-014)
//!
//! Tests the settlement netting engine across corridor scenarios:
//! - Bilateral netting between two parties
//! - Multilateral netting across three or more parties
//! - Deterministic ordering for byte-level reproducibility
//! - Edge cases: single obligation, zero-sum netting

use msez_corridor::netting::{NettingEngine, Obligation};

/// Bilateral netting: two parties with mutual obligations.
#[test]
fn bilateral_netting_reduces_to_net_position() {
    let mut engine = NettingEngine::new();

    // Party A owes Party B $1000
    engine
        .add_obligation(Obligation {
            from_party: "party-a".to_string(),
            to_party: "party-b".to_string(),
            amount: 100_000, // $1000 in cents
            currency: "USD".to_string(),
            corridor_id: Some("PAK-UAE".to_string()),
            priority: 0,
        })
        .expect("valid obligation");

    // Party B owes Party A $600
    engine
        .add_obligation(Obligation {
            from_party: "party-b".to_string(),
            to_party: "party-a".to_string(),
            amount: 60_000, // $600 in cents
            currency: "USD".to_string(),
            corridor_id: Some("PAK-UAE".to_string()),
            priority: 0,
        })
        .expect("valid obligation");

    let result = engine.compute_plan().expect("netting succeeds");

    // Net position: A owes B $400.
    assert!(
        result.gross_total > result.net_total,
        "netting should reduce total"
    );

    // Should produce exactly one settlement leg.
    assert_eq!(
        result.settlement_legs.len(),
        1,
        "bilateral netting should produce one leg"
    );

    let leg = &result.settlement_legs[0];
    assert_eq!(leg.amount, 40_000); // $400
    assert_eq!(leg.currency, "USD");
}

/// Multilateral netting: three parties in a cycle.
#[test]
fn multilateral_netting_three_party_cycle() {
    let mut engine = NettingEngine::new();

    // A → B: $1000
    engine
        .add_obligation(Obligation {
            from_party: "party-a".to_string(),
            to_party: "party-b".to_string(),
            amount: 100_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .expect("valid");

    // B → C: $800
    engine
        .add_obligation(Obligation {
            from_party: "party-b".to_string(),
            to_party: "party-c".to_string(),
            amount: 80_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .expect("valid");

    // C → A: $500
    engine
        .add_obligation(Obligation {
            from_party: "party-c".to_string(),
            to_party: "party-a".to_string(),
            amount: 50_000,
            currency: "USD".to_string(),
            corridor_id: None,
            priority: 0,
        })
        .expect("valid");

    let result = engine.compute_plan().expect("netting succeeds");

    // Gross total: 1000 + 800 + 500 = 2300.
    assert_eq!(result.gross_total, 230_000);

    // Net should be less than gross.
    assert!(result.net_total < result.gross_total);

    // All positions should be settled.
    let positions = &result.net_positions;
    assert!(!positions.is_empty());
}

/// Deterministic output: same inputs always produce same settlement plan.
#[test]
fn netting_is_deterministic() {
    let make_engine = || {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(Obligation {
                from_party: "alpha".to_string(),
                to_party: "beta".to_string(),
                amount: 50_000,
                currency: "USD".to_string(),
                corridor_id: Some("C1".to_string()),
                priority: 0,
            })
            .expect("valid");
        engine
            .add_obligation(Obligation {
                from_party: "beta".to_string(),
                to_party: "alpha".to_string(),
                amount: 30_000,
                currency: "USD".to_string(),
                corridor_id: Some("C1".to_string()),
                priority: 0,
            })
            .expect("valid");
        engine
    };

    let result1 = make_engine().compute_plan().expect("netting");
    let result2 = make_engine().compute_plan().expect("netting");

    // Same structure.
    assert_eq!(result1.settlement_legs.len(), result2.settlement_legs.len());
    assert_eq!(result1.net_total, result2.net_total);
    assert_eq!(result1.gross_total, result2.gross_total);

    // Byte-identical JSON serialization (deterministic ordering).
    let json1 = serde_json::to_string(&result1).expect("serialize 1");
    let json2 = serde_json::to_string(&result2).expect("serialize 2");
    assert_eq!(json1, json2, "netting results must be deterministic");
}

/// Self-referencing obligation is rejected.
#[test]
fn self_referencing_obligation_rejected() {
    let mut engine = NettingEngine::new();
    let result = engine.add_obligation(Obligation {
        from_party: "party-a".to_string(),
        to_party: "party-a".to_string(),
        amount: 10_000,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    assert!(result.is_err());
}

/// Zero-amount obligation is rejected.
#[test]
fn zero_amount_obligation_rejected() {
    let mut engine = NettingEngine::new();
    let result = engine.add_obligation(Obligation {
        from_party: "party-a".to_string(),
        to_party: "party-b".to_string(),
        amount: 0,
        currency: "USD".to_string(),
        corridor_id: None,
        priority: 0,
    });
    assert!(result.is_err());
}

/// Empty engine produces an error.
#[test]
fn empty_engine_errors() {
    let engine = NettingEngine::new();
    assert!(engine.compute_plan().is_err());
}

/// Single obligation produces a single settlement leg equal to the obligation.
#[test]
fn single_obligation_passes_through() {
    let mut engine = NettingEngine::new();
    engine
        .add_obligation(Obligation {
            from_party: "party-a".to_string(),
            to_party: "party-b".to_string(),
            amount: 75_000,
            currency: "PKR".to_string(),
            corridor_id: Some("PAK-KSA".to_string()),
            priority: 0,
        })
        .expect("valid");

    let result = engine.compute_plan().expect("netting");
    assert_eq!(result.settlement_legs.len(), 1);
    assert_eq!(result.settlement_legs[0].amount, 75_000);
    assert_eq!(result.gross_total, result.net_total);
}
