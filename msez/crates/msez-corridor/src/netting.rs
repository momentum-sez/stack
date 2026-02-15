//! # Settlement Netting Engine
//!
//! Compresses bilateral and multilateral obligations into net settlement
//! positions, minimizing the number and total value of settlement legs.
//!
//! ## Design
//!
//! The netting engine operates over a set of [`Obligation`]s between
//! [`Party`] pairs. Each obligation has an amount, currency, and corridor
//! reference. The engine computes:
//!
//! 1. **Gross positions** — total payables and receivables per party per currency.
//! 2. **Net positions** — payables offset against receivables.
//! 3. **Settlement legs** — the minimal set of payments to settle all net positions.
//!
//! ## Determinism
//!
//! All computations use deterministic ordering (sorted party IDs, sorted
//! currencies) to ensure byte-level reproducibility across runs. This is
//! critical for settlement plan verification by multiple independent parties.
//!
//! ## Spec Reference
//!
//! Port of `tools/netting.py` `NettingEngine` class (559 lines).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from netting operations.
#[derive(Error, Debug)]
pub enum NettingError {
    /// No obligations to net.
    #[error("no obligations provided")]
    NoObligations,

    /// Obligation amount is non-positive.
    #[error("obligation amount must be positive: got {amount} for {from_party} -> {to_party}")]
    InvalidAmount {
        /// The from party.
        from_party: String,
        /// The to party.
        to_party: String,
        /// The invalid amount.
        amount: i64,
    },

    /// Duplicate obligation detected.
    #[error("duplicate obligation: {from_party} -> {to_party} for {amount} {currency} (corridor: {corridor_id})")]
    DuplicateObligation {
        /// The from party.
        from_party: String,
        /// The to party.
        to_party: String,
        /// The amount.
        amount: i64,
        /// The currency.
        currency: String,
        /// The corridor ID (or "none").
        corridor_id: String,
    },

    /// Netting is infeasible under the given constraints.
    #[error("netting infeasible: {reason}")]
    Infeasible {
        /// Reason for infeasibility.
        reason: String,
    },

    /// Obligation party identifiers are invalid (empty or self-referencing).
    #[error("invalid obligation parties: {reason}")]
    InvalidParties {
        /// Reason for rejection.
        reason: String,
    },

    /// Currency code is invalid (empty).
    #[error("invalid currency code: must be non-empty")]
    InvalidCurrency,

    /// Arithmetic overflow during settlement computation.
    #[error(
        "arithmetic overflow computing settlement totals — obligation amounts exceed i64 range"
    )]
    ArithmeticOverflow,
}

/// A party in the settlement netting system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Party {
    /// Unique party identifier.
    pub id: String,
    /// Human-readable party name.
    pub name: String,
}

/// A currency in the netting system.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Currency {
    /// ISO 4217 currency code (e.g., "USD", "PKR", "AED").
    pub code: String,
    /// Decimal precision for this currency.
    pub precision: u8,
}

/// An obligation between two parties.
///
/// Represents a directed payment obligation: `from_party` owes
/// `amount` in `currency` to `to_party`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Obligation {
    /// The party that owes.
    pub from_party: String,
    /// The party that is owed.
    pub to_party: String,
    /// Amount in the smallest currency unit (e.g., cents for USD).
    pub amount: i64,
    /// Currency code.
    pub currency: String,
    /// Corridor reference (for audit trail).
    pub corridor_id: Option<String>,
    /// Priority (higher = settled first). Default 0.
    pub priority: i32,
}

/// A computed net position for a party in a specific currency.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetPosition {
    /// Party identifier.
    pub party_id: String,
    /// Currency code.
    pub currency: String,
    /// Total receivable amount.
    pub receivable: i64,
    /// Total payable amount.
    pub payable: i64,
    /// Net amount (receivable - payable). Positive = net receiver.
    pub net: i64,
}

/// A settlement leg — a single payment in the settlement plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettlementLeg {
    /// Paying party.
    pub from_party: String,
    /// Receiving party.
    pub to_party: String,
    /// Settlement amount.
    pub amount: i64,
    /// Currency code.
    pub currency: String,
}

/// A complete settlement plan produced by the netting engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementPlan {
    /// The original obligations that were netted.
    pub obligations: Vec<Obligation>,
    /// Computed net positions per party per currency.
    pub net_positions: Vec<NetPosition>,
    /// Settlement legs (minimal payment set).
    pub settlement_legs: Vec<SettlementLeg>,
    /// Total gross obligation amount (sum of all obligation amounts).
    pub gross_total: i64,
    /// Total net settlement amount (sum of all settlement leg amounts).
    pub net_total: i64,
    /// Netting efficiency as a percentage (0.0 to 100.0).
    pub reduction_percentage: f64,
}

/// The settlement netting engine.
///
/// Computes optimal netting of bilateral obligations into minimal
/// settlement legs. Supports multi-currency, multi-party, multi-corridor
/// obligation sets.
///
/// ## Determinism
///
/// All internal operations use BTreeMap/BTreeSet for deterministic
/// ordering. Party IDs and currency codes are sorted lexicographically.
/// Two runs with the same input produce byte-identical output.
///
/// ## Duplicate Detection
///
/// Obligations are deduplicated by (from_party, to_party, amount, currency,
/// corridor_id) to prevent double-settlement. Adding a duplicate obligation
/// returns [`NettingError::DuplicateObligation`].
#[derive(Debug, Default)]
pub struct NettingEngine {
    obligations: Vec<Obligation>,
    /// Tracks seen obligation keys for O(1) duplicate detection.
    seen: BTreeSet<(String, String, i64, String, String)>,
}

impl NettingEngine {
    /// Create a new empty netting engine.
    pub fn new() -> Self {
        Self {
            obligations: Vec::new(),
            seen: BTreeSet::new(),
        }
    }

    /// Add an obligation to the netting set.
    ///
    /// Rejects duplicate obligations (same from_party, to_party, amount,
    /// currency, corridor_id) to prevent double-settlement.
    pub fn add_obligation(&mut self, obligation: Obligation) -> Result<(), NettingError> {
        if obligation.amount <= 0 {
            return Err(NettingError::InvalidAmount {
                from_party: obligation.from_party.clone(),
                to_party: obligation.to_party.clone(),
                amount: obligation.amount,
            });
        }

        // BUG-020: Reject self-obligations (from_party == to_party).
        if obligation.from_party == obligation.to_party {
            return Err(NettingError::InvalidParties {
                reason: format!(
                    "from_party and to_party are identical: \"{}\"",
                    obligation.from_party
                ),
            });
        }

        // BUG-021: Reject empty party IDs.
        if obligation.from_party.trim().is_empty() || obligation.to_party.trim().is_empty() {
            return Err(NettingError::InvalidParties {
                reason: "party identifiers must be non-empty".to_string(),
            });
        }

        // BUG-022: Reject empty currency codes.
        if obligation.currency.trim().is_empty() {
            return Err(NettingError::InvalidCurrency);
        }

        let key = (
            obligation.from_party.clone(),
            obligation.to_party.clone(),
            obligation.amount,
            obligation.currency.clone(),
            obligation.corridor_id.clone().unwrap_or_default(),
        );
        if !self.seen.insert(key) {
            return Err(NettingError::DuplicateObligation {
                from_party: obligation.from_party,
                to_party: obligation.to_party,
                amount: obligation.amount,
                currency: obligation.currency,
                corridor_id: obligation.corridor_id.unwrap_or_else(|| "none".to_string()),
            });
        }

        self.obligations.push(obligation);
        Ok(())
    }

    /// Return the number of obligations.
    pub fn obligation_count(&self) -> usize {
        self.obligations.len()
    }

    /// Compute gross positions: total payables and receivables per party per currency.
    pub fn compute_gross_positions(&self) -> BTreeMap<(String, String), (i64, i64)> {
        let mut positions: BTreeMap<(String, String), (i64, i64)> = BTreeMap::new();

        for ob in &self.obligations {
            // from_party: payable increases
            let from_key = (ob.from_party.clone(), ob.currency.clone());
            let from_entry = positions.entry(from_key).or_insert((0, 0));
            from_entry.1 += ob.amount;

            // to_party: receivable increases
            let to_key = (ob.to_party.clone(), ob.currency.clone());
            let to_entry = positions.entry(to_key).or_insert((0, 0));
            to_entry.0 += ob.amount;
        }

        positions
    }

    /// Compute net positions: receivables offset against payables.
    pub fn compute_net_positions(&self) -> Vec<NetPosition> {
        let gross = self.compute_gross_positions();
        gross
            .into_iter()
            .map(
                |((party_id, currency), (receivable, payable))| NetPosition {
                    party_id,
                    currency,
                    receivable,
                    payable,
                    net: receivable - payable,
                },
            )
            .collect()
    }

    /// Generate settlement legs using greedy matching of payers and receivers.
    ///
    /// For each currency, sorts parties into payers (net < 0) and receivers
    /// (net > 0), then greedily matches them to produce minimal settlement legs.
    ///
    /// ## Determinism
    ///
    /// Payers and receivers are sorted by party ID for deterministic output.
    fn generate_settlement_legs(net_positions: &[NetPosition]) -> Vec<SettlementLeg> {
        let mut currencies: BTreeSet<String> = BTreeSet::new();
        for np in net_positions {
            currencies.insert(np.currency.clone());
        }

        let mut legs = Vec::new();

        for currency in &currencies {
            let mut payers: Vec<(String, i64)> = Vec::new();
            let mut receivers: Vec<(String, i64)> = Vec::new();

            for np in net_positions {
                if np.currency != *currency {
                    continue;
                }
                if np.net < 0 {
                    payers.push((np.party_id.clone(), -np.net));
                } else if np.net > 0 {
                    receivers.push((np.party_id.clone(), np.net));
                }
            }

            // Sort for deterministic ordering.
            payers.sort_by(|a, b| a.0.cmp(&b.0));
            receivers.sort_by(|a, b| a.0.cmp(&b.0));

            let mut pi = 0;
            let mut ri = 0;

            while pi < payers.len() && ri < receivers.len() {
                let settle_amount = payers[pi].1.min(receivers[ri].1);
                if settle_amount > 0 {
                    legs.push(SettlementLeg {
                        from_party: payers[pi].0.clone(),
                        to_party: receivers[ri].0.clone(),
                        amount: settle_amount,
                        currency: currency.clone(),
                    });
                }

                payers[pi].1 -= settle_amount;
                receivers[ri].1 -= settle_amount;

                if payers[pi].1 == 0 {
                    pi += 1;
                }
                if ri < receivers.len() && receivers[ri].1 == 0 {
                    ri += 1;
                }
            }
        }

        legs
    }

    /// Compute the complete settlement plan.
    ///
    /// Validates obligations, computes net positions, generates settlement
    /// legs, and calculates netting efficiency metrics.
    pub fn compute_plan(&self) -> Result<SettlementPlan, NettingError> {
        if self.obligations.is_empty() {
            return Err(NettingError::NoObligations);
        }

        let net_positions = self.compute_net_positions();
        let settlement_legs = Self::generate_settlement_legs(&net_positions);

        // BUG-018 fix: use checked arithmetic to prevent silent i64 overflow.
        let gross_total: i64 = self
            .obligations
            .iter()
            .try_fold(0i64, |acc, o| acc.checked_add(o.amount))
            .ok_or(NettingError::ArithmeticOverflow)?;
        let net_total: i64 = settlement_legs
            .iter()
            .try_fold(0i64, |acc, l| acc.checked_add(l.amount))
            .ok_or(NettingError::ArithmeticOverflow)?;

        let reduction_percentage = if gross_total > 0 {
            (1.0 - (net_total as f64 / gross_total as f64)) * 100.0
        } else {
            0.0
        };

        Ok(SettlementPlan {
            obligations: self.obligations.clone(),
            net_positions,
            settlement_legs,
            gross_total,
            net_total,
            reduction_percentage,
        })
    }

    /// Clear all obligations and start fresh.
    pub fn clear(&mut self) {
        self.obligations.clear();
        self.seen.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn bilateral_netting() {
        let mut engine = NettingEngine::new();
        // A owes B 100 USD
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        // B owes A 60 USD
        engine
            .add_obligation(obligation("B", "A", 60, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(plan.gross_total, 160);
        // Net: A owes B 40 USD (100 - 60)
        assert_eq!(plan.settlement_legs.len(), 1);
        assert_eq!(plan.settlement_legs[0].from_party, "A");
        assert_eq!(plan.settlement_legs[0].to_party, "B");
        assert_eq!(plan.settlement_legs[0].amount, 40);
        assert_eq!(plan.net_total, 40);
        assert!(plan.reduction_percentage > 0.0);
    }

    #[test]
    fn multilateral_netting() {
        let mut engine = NettingEngine::new();
        // A -> B: 100 USD
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        // B -> C: 80 USD
        engine
            .add_obligation(obligation("B", "C", 80, "USD"))
            .unwrap();
        // C -> A: 60 USD
        engine
            .add_obligation(obligation("C", "A", 60, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(plan.gross_total, 240);
        // Net positions:
        // A: receivable=60, payable=100, net=-40 (payer)
        // B: receivable=100, payable=80, net=+20 (receiver)
        // C: receivable=80, payable=60, net=+20 (receiver)
        assert!(plan.net_total < plan.gross_total);
    }

    #[test]
    fn multi_currency_netting() {
        let mut engine = NettingEngine::new();
        // USD obligations
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 60, "USD"))
            .unwrap();
        // PKR obligations
        engine
            .add_obligation(obligation("A", "B", 50000, "PKR"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 30000, "PKR"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        // USD and PKR legs should be separate
        let usd_legs: Vec<_> = plan
            .settlement_legs
            .iter()
            .filter(|l| l.currency == "USD")
            .collect();
        let pkr_legs: Vec<_> = plan
            .settlement_legs
            .iter()
            .filter(|l| l.currency == "PKR")
            .collect();

        assert_eq!(usd_legs.len(), 1);
        assert_eq!(usd_legs[0].amount, 40);
        assert_eq!(pkr_legs.len(), 1);
        assert_eq!(pkr_legs[0].amount, 20000);
    }

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

    #[test]
    fn empty_obligations_error() {
        let engine = NettingEngine::new();
        assert!(matches!(
            engine.compute_plan(),
            Err(NettingError::NoObligations)
        ));
    }

    #[test]
    fn negative_amount_rejected() {
        let mut engine = NettingEngine::new();
        let result = engine.add_obligation(obligation("A", "B", -100, "USD"));
        assert!(matches!(result, Err(NettingError::InvalidAmount { .. })));
    }

    #[test]
    fn zero_amount_rejected() {
        let mut engine = NettingEngine::new();
        let result = engine.add_obligation(obligation("A", "B", 0, "USD"));
        assert!(matches!(result, Err(NettingError::InvalidAmount { .. })));
    }

    #[test]
    fn deterministic_output() {
        let build_engine = || {
            let mut engine = NettingEngine::new();
            engine
                .add_obligation(obligation("C", "A", 300, "USD"))
                .unwrap();
            engine
                .add_obligation(obligation("A", "B", 100, "USD"))
                .unwrap();
            engine
                .add_obligation(obligation("B", "C", 200, "USD"))
                .unwrap();
            engine
        };

        let plan1 = build_engine().compute_plan().unwrap();
        let plan2 = build_engine().compute_plan().unwrap();

        let json1 = serde_json::to_string(&plan1.settlement_legs).unwrap();
        let json2 = serde_json::to_string(&plan2.settlement_legs).unwrap();
        assert_eq!(json1, json2);
    }

    #[test]
    fn four_party_netting() {
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
        assert!(plan.net_total < plan.gross_total);
        // Every party should appear in net positions
        let parties: BTreeSet<String> = plan
            .net_positions
            .iter()
            .map(|np| np.party_id.clone())
            .collect();
        assert!(parties.contains("A"));
        assert!(parties.contains("B"));
        assert!(parties.contains("C"));
        assert!(parties.contains("D"));
    }

    #[test]
    fn clear_resets_engine() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 1);
        engine.clear();
        assert_eq!(engine.obligation_count(), 0);
        assert!(matches!(
            engine.compute_plan(),
            Err(NettingError::NoObligations)
        ));
    }

    // ── Additional coverage tests ────────────────────────────────────

    #[test]
    fn single_obligation_no_netting() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 500, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(plan.gross_total, 500);
        assert_eq!(plan.net_total, 500);
        assert_eq!(plan.settlement_legs.len(), 1);
        assert!((plan.reduction_percentage - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn obligation_count_tracks_additions() {
        let mut engine = NettingEngine::new();
        assert_eq!(engine.obligation_count(), 0);
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 1);
        engine
            .add_obligation(obligation("B", "C", 200, "USD"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 2);
        engine
            .add_obligation(obligation("C", "A", 150, "PKR"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 3);
    }

    #[test]
    fn gross_positions_computed_correctly() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 60, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("A", "B", 50, "USD"))
            .unwrap();

        let positions = engine.compute_gross_positions();
        // A-USD: receivable=60, payable=150
        let a_pos = positions
            .get(&("A".to_string(), "USD".to_string()))
            .unwrap();
        assert_eq!(a_pos.0, 60); // receivable
        assert_eq!(a_pos.1, 150); // payable

        // B-USD: receivable=150, payable=60
        let b_pos = positions
            .get(&("B".to_string(), "USD".to_string()))
            .unwrap();
        assert_eq!(b_pos.0, 150); // receivable
        assert_eq!(b_pos.1, 60); // payable
    }

    #[test]
    fn net_positions_computed_correctly() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 60, "USD"))
            .unwrap();

        let positions = engine.compute_net_positions();
        let a_pos = positions.iter().find(|p| p.party_id == "A").unwrap();
        assert_eq!(a_pos.receivable, 60);
        assert_eq!(a_pos.payable, 100);
        assert_eq!(a_pos.net, -40);

        let b_pos = positions.iter().find(|p| p.party_id == "B").unwrap();
        assert_eq!(b_pos.receivable, 100);
        assert_eq!(b_pos.payable, 60);
        assert_eq!(b_pos.net, 40);
    }

    #[test]
    fn obligation_with_corridor_id() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(Obligation {
                from_party: "A".to_string(),
                to_party: "B".to_string(),
                amount: 100,
                currency: "USD".to_string(),
                corridor_id: Some("corridor-pk-ae-001".to_string()),
                priority: 5,
            })
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(
            plan.obligations[0].corridor_id.as_deref(),
            Some("corridor-pk-ae-001")
        );
        assert_eq!(plan.obligations[0].priority, 5);
    }

    #[test]
    fn netting_error_display_no_obligations() {
        let err = NettingError::NoObligations;
        assert_eq!(format!("{err}"), "no obligations provided");
    }

    #[test]
    fn netting_error_display_invalid_amount() {
        let err = NettingError::InvalidAmount {
            from_party: "A".to_string(),
            to_party: "B".to_string(),
            amount: -100,
        };
        let msg = format!("{err}");
        assert!(msg.contains("-100"));
        assert!(msg.contains("A"));
        assert!(msg.contains("B"));
    }

    #[test]
    fn netting_error_display_infeasible() {
        let err = NettingError::Infeasible {
            reason: "insufficient liquidity".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("insufficient liquidity"));
    }

    #[test]
    fn settlement_plan_serialization_roundtrip() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 40, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        let json_str = serde_json::to_string(&plan).unwrap();
        let back: SettlementPlan = serde_json::from_str(&json_str).unwrap();

        assert_eq!(back.gross_total, plan.gross_total);
        assert_eq!(back.net_total, plan.net_total);
        assert_eq!(back.settlement_legs.len(), plan.settlement_legs.len());
    }

    #[test]
    fn default_netting_engine() {
        let engine = NettingEngine::default();
        assert_eq!(engine.obligation_count(), 0);
    }

    #[test]
    fn three_currency_netting() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("A", "B", 5000, "PKR"))
            .unwrap();
        engine
            .add_obligation(obligation("A", "B", 200, "AED"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 50, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 3000, "PKR"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "A", 100, "AED"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        let currencies: std::collections::BTreeSet<String> = plan
            .settlement_legs
            .iter()
            .map(|l| l.currency.clone())
            .collect();
        assert_eq!(currencies.len(), 3);
        assert!(currencies.contains("USD"));
        assert!(currencies.contains("PKR"));
        assert!(currencies.contains("AED"));
    }

    #[test]
    fn duplicate_obligation_rejected() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        let result = engine.add_obligation(obligation("A", "B", 100, "USD"));
        assert!(
            matches!(result, Err(NettingError::DuplicateObligation { .. })),
            "duplicate obligation should be rejected"
        );
        assert_eq!(engine.obligation_count(), 1);
    }

    #[test]
    fn different_amounts_not_duplicate() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        // Same parties, different amount → not a duplicate
        engine
            .add_obligation(obligation("A", "B", 200, "USD"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 2);
    }

    #[test]
    fn different_currencies_not_duplicate() {
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("A", "B", 100, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("A", "B", 100, "PKR"))
            .unwrap();
        assert_eq!(engine.obligation_count(), 2);
    }

    #[test]
    fn self_netting_obligation_nets_to_zero() {
        // If A owes B and B owes A the same amount in the same currency,
        // they should net to zero
        let mut engine = NettingEngine::new();
        engine
            .add_obligation(obligation("X", "Y", 1000, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("Y", "X", 1000, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(plan.settlement_legs.len(), 0);
        assert_eq!(plan.net_total, 0);
    }

    #[test]
    fn large_multilateral_netting() {
        let mut engine = NettingEngine::new();
        // Five-party circular obligation chain
        engine
            .add_obligation(obligation("A", "B", 1000, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("B", "C", 800, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("C", "D", 600, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("D", "E", 400, "USD"))
            .unwrap();
        engine
            .add_obligation(obligation("E", "A", 200, "USD"))
            .unwrap();

        let plan = engine.compute_plan().unwrap();
        assert_eq!(plan.gross_total, 3000);
        assert!(plan.net_total < plan.gross_total);
        assert!(plan.reduction_percentage > 0.0);
    }
}
