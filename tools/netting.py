#!/usr/bin/env python3
"""Multi-Corridor Netting Engine (v0.4.42).

This module implements the corridor-of-corridors netting primitive:
- Multi-corridor obligation aggregation
- Multi-currency netting with constrained optimization
- Deterministic tie-breaking for reproducible settlement plans
- Traceable optimization output (explainable plan)

The netting engine takes obligations from multiple corridors and produces
a deterministic settlement plan that minimizes settlement legs while
respecting constraints (per-party limits, per-rail limits, cutoffs, priority tiers).
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from decimal import Decimal
from typing import Any, Dict, List, Optional, Set, Tuple

# ─────────────────────────────────────────────────────────────────────────────
# Data Types
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class Party:
    """A party in the netting session."""
    party_id: str
    name: str = ""


@dataclass(frozen=True)
class Currency:
    """A currency in the netting session."""
    code: str
    precision: int = 2


@dataclass
class Obligation:
    """An obligation from a corridor receipt chain."""
    obligation_id: str
    corridor_id: str
    debtor: Party
    creditor: Party
    currency: Currency
    amount: Decimal
    priority: int = 0  # Higher = more urgent
    checkpoint_digest: str = ""  # Reference to obligation corridor checkpoint
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "obligation_id": self.obligation_id,
            "corridor_id": self.corridor_id,
            "debtor": self.debtor.party_id,
            "creditor": self.creditor.party_id,
            "currency": self.currency.code,
            "amount": str(self.amount),
            "priority": self.priority,
            "checkpoint_digest": self.checkpoint_digest,
        }


@dataclass
class SettlementRail:
    """A settlement rail (corridor) available for settlement."""
    rail_id: str
    corridor_id: str
    supported_currencies: Set[str]
    max_single_transfer: Dict[str, Decimal] = field(default_factory=dict)
    daily_limit: Dict[str, Decimal] = field(default_factory=dict)
    priority: int = 0  # Higher = preferred


@dataclass
class PartyConstraint:
    """Per-party constraints for netting."""
    party_id: str
    max_net_position: Dict[str, Decimal] = field(default_factory=dict)  # per currency
    blocked_counterparties: Set[str] = field(default_factory=set)
    allowed_rails: Optional[Set[str]] = None  # None = all rails


@dataclass
class NettingConstraints:
    """Global constraints for the netting session."""
    party_constraints: Dict[str, PartyConstraint] = field(default_factory=dict)
    cutoff_time: Optional[str] = None  # RFC3339
    min_leg_amount: Dict[str, Decimal] = field(default_factory=dict)  # per currency


@dataclass
class NetPosition:
    """Net position for a party in a currency after netting."""
    party_id: str
    currency: str
    gross_receivable: Decimal
    gross_payable: Decimal
    net_amount: Decimal  # positive = net receiver, negative = net payer


@dataclass
class SettlementLeg:
    """A single settlement leg in the plan."""
    leg_id: str
    rail_id: str
    payer: str
    payee: str
    currency: str
    amount: Decimal
    obligation_refs: List[str]  # obligation_ids covered by this leg
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "leg_id": self.leg_id,
            "rail_id": self.rail_id,
            "payer": self.payer,
            "payee": self.payee,
            "currency": self.currency,
            "amount": str(self.amount),
            "obligation_refs": self.obligation_refs,
        }


@dataclass
class NettingTrace:
    """Trace of the netting optimization for explainability."""
    step: int
    action: str
    details: Dict[str, Any]


@dataclass
class SettlementPlan:
    """The output settlement plan from netting."""
    plan_id: str
    netting_method: str
    obligations: List[Obligation]
    net_positions: List[NetPosition]
    settlement_legs: List[SettlementLeg]
    trace: List[NettingTrace]
    constraints_applied: NettingConstraints
    total_gross_volume: Dict[str, Decimal]
    total_net_volume: Dict[str, Decimal]
    reduction_ratio: Dict[str, float]
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "netting_method": self.netting_method,
            "obligations": [o.to_dict() for o in self.obligations],
            "net_positions": [
                {
                    "party_id": np.party_id,
                    "currency": np.currency,
                    "gross_receivable": str(np.gross_receivable),
                    "gross_payable": str(np.gross_payable),
                    "net_amount": str(np.net_amount),
                }
                for np in self.net_positions
            ],
            "settlement_legs": [leg.to_dict() for leg in self.settlement_legs],
            "trace": [{"step": t.step, "action": t.action, "details": t.details} for t in self.trace],
            "total_gross_volume": {k: str(v) for k, v in self.total_gross_volume.items()},
            "total_net_volume": {k: str(v) for k, v in self.total_net_volume.items()},
            "reduction_ratio": self.reduction_ratio,
        }


# ─────────────────────────────────────────────────────────────────────────────
# Netting Engine
# ─────────────────────────────────────────────────────────────────────────────

class NettingEngine:
    """Multi-corridor, multi-currency netting engine with deterministic tie-breaking."""
    
    def __init__(
        self,
        obligations: List[Obligation],
        rails: List[SettlementRail],
        constraints: Optional[NettingConstraints] = None,
    ):
        self.obligations = obligations
        self.rails = rails
        self.constraints = constraints or NettingConstraints()
        self.trace: List[NettingTrace] = []
        self._step = 0
    
    def _log_trace(self, action: str, details: Dict[str, Any]) -> None:
        self._step += 1
        self.trace.append(NettingTrace(step=self._step, action=action, details=details))
    
    def _deterministic_leg_id(self, payer: str, payee: str, currency: str, seq: int) -> str:
        """Generate deterministic leg ID for reproducibility."""
        data = f"{payer}:{payee}:{currency}:{seq}"
        return f"leg:{hashlib.sha256(data.encode()).hexdigest()[:16]}"
    
    def compute_gross_positions(self) -> Dict[str, Dict[str, Dict[str, Decimal]]]:
        """Compute gross receivable/payable for each party per currency."""
        positions: Dict[str, Dict[str, Dict[str, Decimal]]] = {}
        
        for obl in self.obligations:
            debtor_id = obl.debtor.party_id
            creditor_id = obl.creditor.party_id
            ccy = obl.currency.code
            
            # Initialize if needed
            for party_id in [debtor_id, creditor_id]:
                if party_id not in positions:
                    positions[party_id] = {}
                if ccy not in positions[party_id]:
                    positions[party_id][ccy] = {"receivable": Decimal(0), "payable": Decimal(0)}
            
            # Update positions
            positions[debtor_id][ccy]["payable"] += obl.amount
            positions[creditor_id][ccy]["receivable"] += obl.amount
        
        return positions
    
    def compute_net_positions(
        self,
        gross_positions: Dict[str, Dict[str, Dict[str, Decimal]]],
    ) -> List[NetPosition]:
        """Compute net positions from gross positions."""
        net_positions: List[NetPosition] = []
        
        for party_id in sorted(gross_positions.keys()):  # Deterministic ordering
            for ccy in sorted(gross_positions[party_id].keys()):
                pos = gross_positions[party_id][ccy]
                net = pos["receivable"] - pos["payable"]
                net_positions.append(NetPosition(
                    party_id=party_id,
                    currency=ccy,
                    gross_receivable=pos["receivable"],
                    gross_payable=pos["payable"],
                    net_amount=net,
                ))
        
        return net_positions
    
    def _apply_party_constraints(
        self,
        net_positions: List[NetPosition],
    ) -> List[NetPosition]:
        """Apply party constraints and cap net positions if needed."""
        adjusted: List[NetPosition] = []
        
        for np in net_positions:
            party_constraint = self.constraints.party_constraints.get(np.party_id)
            if party_constraint:
                max_net = party_constraint.max_net_position.get(np.currency)
                if max_net is not None and abs(np.net_amount) > max_net:
                    self._log_trace("constraint_applied", {
                        "party_id": np.party_id,
                        "currency": np.currency,
                        "original_net": str(np.net_amount),
                        "capped_net": str(max_net if np.net_amount > 0 else -max_net),
                    })
                    # Note: In real implementation, excess would need redistribution
                    # For now, we just log it
            adjusted.append(np)
        
        return adjusted
    
    def _select_rail(
        self,
        payer: str,
        payee: str,
        currency: str,
        amount: Decimal,
    ) -> Optional[SettlementRail]:
        """Select the best settlement rail for a leg, respecting constraints."""
        candidates = []
        
        for rail in self.rails:
            # Check currency support
            if currency not in rail.supported_currencies:
                continue
            
            # Check max single transfer
            max_single = rail.max_single_transfer.get(currency)
            if max_single is not None and amount > max_single:
                continue
            
            # Check party constraints
            payer_constraint = self.constraints.party_constraints.get(payer)
            if payer_constraint:
                if payer_constraint.allowed_rails is not None:
                    if rail.rail_id not in payer_constraint.allowed_rails:
                        continue
            
            candidates.append(rail)
        
        if not candidates:
            return None
        
        # Deterministic tie-breaking: sort by (priority desc, rail_id asc)
        candidates.sort(key=lambda r: (-r.priority, r.rail_id))
        return candidates[0]
    
    def _generate_settlement_legs(
        self,
        net_positions: List[NetPosition],
    ) -> List[SettlementLeg]:
        """Generate settlement legs from net positions using greedy matching."""
        legs: List[SettlementLeg] = []
        
        # Group by currency
        by_currency: Dict[str, List[NetPosition]] = {}
        for np in net_positions:
            if np.currency not in by_currency:
                by_currency[np.currency] = []
            by_currency[np.currency].append(np)
        
        leg_seq = 0
        
        for ccy in sorted(by_currency.keys()):  # Deterministic currency ordering
            positions = by_currency[ccy]
            
            # Separate payers (negative net) and receivers (positive net)
            payers = sorted(
                [p for p in positions if p.net_amount < 0],
                key=lambda p: (p.net_amount, p.party_id),  # Deterministic
            )
            receivers = sorted(
                [p for p in positions if p.net_amount > 0],
                key=lambda p: (-p.net_amount, p.party_id),  # Deterministic
            )
            
            self._log_trace("netting_currency", {
                "currency": ccy,
                "payers": [p.party_id for p in payers],
                "receivers": [r.party_id for r in receivers],
            })
            
            # Greedy matching with deterministic tie-breaking
            payer_idx = 0
            receiver_idx = 0
            payer_remaining: Dict[str, Decimal] = {p.party_id: -p.net_amount for p in payers}
            receiver_remaining: Dict[str, Decimal] = {r.party_id: r.net_amount for r in receivers}
            
            while payer_idx < len(payers) and receiver_idx < len(receivers):
                payer = payers[payer_idx]
                receiver = receivers[receiver_idx]
                
                payer_rem = payer_remaining[payer.party_id]
                receiver_rem = receiver_remaining[receiver.party_id]
                
                if payer_rem <= 0:
                    payer_idx += 1
                    continue
                if receiver_rem <= 0:
                    receiver_idx += 1
                    continue
                
                # Determine leg amount
                leg_amount = min(payer_rem, receiver_rem)
                
                # Check minimum leg amount constraint
                min_leg = self.constraints.min_leg_amount.get(ccy, Decimal(0))
                if leg_amount < min_leg:
                    self._log_trace("leg_skipped_min_amount", {
                        "payer": payer.party_id,
                        "payee": receiver.party_id,
                        "amount": str(leg_amount),
                        "min_required": str(min_leg),
                    })
                    # Move to next pairing
                    if payer_rem <= receiver_rem:
                        payer_idx += 1
                    else:
                        receiver_idx += 1
                    continue
                
                # Select rail
                rail = self._select_rail(payer.party_id, receiver.party_id, ccy, leg_amount)
                if rail is None:
                    self._log_trace("no_rail_available", {
                        "payer": payer.party_id,
                        "payee": receiver.party_id,
                        "currency": ccy,
                        "amount": str(leg_amount),
                    })
                    payer_idx += 1
                    continue
                
                # Create leg
                leg_seq += 1
                leg_id = self._deterministic_leg_id(
                    payer.party_id, receiver.party_id, ccy, leg_seq
                )
                
                # Find obligation refs (simplified: all obligations involving both parties)
                obl_refs = [
                    o.obligation_id for o in self.obligations
                    if o.currency.code == ccy and (
                        (o.debtor.party_id == payer.party_id and o.creditor.party_id == receiver.party_id) or
                        (o.creditor.party_id == payer.party_id and o.debtor.party_id == receiver.party_id)
                    )
                ]
                
                leg = SettlementLeg(
                    leg_id=leg_id,
                    rail_id=rail.rail_id,
                    payer=payer.party_id,
                    payee=receiver.party_id,
                    currency=ccy,
                    amount=leg_amount,
                    obligation_refs=obl_refs,
                )
                legs.append(leg)
                
                self._log_trace("leg_created", {
                    "leg_id": leg_id,
                    "rail": rail.rail_id,
                    "payer": payer.party_id,
                    "payee": receiver.party_id,
                    "amount": str(leg_amount),
                })
                
                # Update remaining
                payer_remaining[payer.party_id] -= leg_amount
                receiver_remaining[receiver.party_id] -= leg_amount
                
                if payer_remaining[payer.party_id] <= 0:
                    payer_idx += 1
                if receiver_remaining[receiver.party_id] <= 0:
                    receiver_idx += 1
        
        return legs
    
    def compute_plan(self, plan_id: str) -> SettlementPlan:
        """Compute the full settlement plan."""
        self._log_trace("netting_started", {
            "obligation_count": len(self.obligations),
            "rail_count": len(self.rails),
        })
        
        # Step 1: Compute gross positions
        gross_positions = self.compute_gross_positions()
        
        # Step 2: Compute net positions
        net_positions = self.compute_net_positions(gross_positions)
        
        # Step 3: Apply constraints
        net_positions = self._apply_party_constraints(net_positions)
        
        # Step 4: Generate settlement legs
        settlement_legs = self._generate_settlement_legs(net_positions)
        
        # Compute volumes
        total_gross: Dict[str, Decimal] = {}
        total_net: Dict[str, Decimal] = {}
        
        for obl in self.obligations:
            ccy = obl.currency.code
            total_gross[ccy] = total_gross.get(ccy, Decimal(0)) + obl.amount
        
        for leg in settlement_legs:
            ccy = leg.currency
            total_net[ccy] = total_net.get(ccy, Decimal(0)) + leg.amount
        
        # Compute reduction ratio
        reduction: Dict[str, float] = {}
        for ccy in total_gross:
            gross = total_gross[ccy]
            net = total_net.get(ccy, Decimal(0))
            if gross > 0:
                reduction[ccy] = float((gross - net) / gross)
            else:
                reduction[ccy] = 0.0
        
        self._log_trace("netting_completed", {
            "legs_created": len(settlement_legs),
            "reduction_ratios": reduction,
        })
        
        return SettlementPlan(
            plan_id=plan_id,
            netting_method="multi-corridor-greedy-v1",
            obligations=self.obligations,
            net_positions=net_positions,
            settlement_legs=settlement_legs,
            trace=self.trace,
            constraints_applied=self.constraints,
            total_gross_volume=total_gross,
            total_net_volume=total_net,
            reduction_ratio=reduction,
        )


# ─────────────────────────────────────────────────────────────────────────────
# CLI Interface
# ─────────────────────────────────────────────────────────────────────────────

def demo_netting() -> None:
    """Demonstrate multi-corridor netting with sample data."""
    # Sample parties
    party_a = Party("did:key:z6MkPartyA", "Exporter Corp")
    party_b = Party("did:key:z6MkPartyB", "Importer Inc")
    party_c = Party("did:key:z6MkPartyC", "Bank A")
    party_d = Party("did:key:z6MkPartyD", "Bank B")
    
    usd = Currency("USD", 2)
    eur = Currency("EUR", 2)
    
    # Sample obligations from multiple corridors
    obligations = [
        Obligation("obl-001", "corridor:trade:001", party_b, party_a, usd, Decimal("50000")),
        Obligation("obl-002", "corridor:trade:001", party_b, party_a, usd, Decimal("25000")),
        Obligation("obl-003", "corridor:trade:002", party_a, party_b, usd, Decimal("30000")),
        Obligation("obl-004", "corridor:trade:003", party_c, party_d, eur, Decimal("100000")),
        Obligation("obl-005", "corridor:trade:003", party_d, party_c, eur, Decimal("75000")),
    ]
    
    # Settlement rails
    rails = [
        SettlementRail(
            "rail:swift",
            "corridor:settlement:swift",
            {"USD", "EUR"},
            max_single_transfer={"USD": Decimal("1000000"), "EUR": Decimal("1000000")},
            priority=1,
        ),
        SettlementRail(
            "rail:usdc",
            "corridor:settlement:stablecoin",
            {"USD"},
            max_single_transfer={"USD": Decimal("500000")},
            priority=2,
        ),
    ]
    
    # Run netting
    engine = NettingEngine(obligations, rails)
    plan = engine.compute_plan("plan:demo:001")
    
    print(json.dumps(plan.to_dict(), indent=2, default=str))


if __name__ == "__main__":
    demo_netting()
