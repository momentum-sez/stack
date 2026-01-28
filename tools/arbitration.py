#!/usr/bin/env python3
"""Arbitration Module: Programmatic Dispute Resolution.

This module implements the Arbitration System for:
- Dispute filing with evidence bundles
- Ruling verification and enforcement
- Smart asset state transitions from arbitration orders
- Integration with arbitration institutions (DIFC-LCIA, SIAC, ICC)

Usage:
    from tools.arbitration import ArbitrationManager, DisputeRequest

    manager = ArbitrationManager(institution_id="difc-lcia")
    dispute = manager.create_dispute_request(
        corridor_id="corridor:uae-kaz-trade-01",
        claimant="did:key:...",
        respondent="did:key:...",
        claims=[...],
    )
"""

from __future__ import annotations

import hashlib
import json
import os
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from decimal import Decimal
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

STACK_SPEC_VERSION = "0.4.43"

NAMESPACE_ARBITRATION = uuid.UUID("6ba7b810-9dad-11d1-80b4-00c04fd430ca")

DISPUTE_TYPES = [
    "breach_of_contract",
    "non_conforming_goods",
    "payment_default",
    "delivery_failure",
    "quality_defect",
    "documentary_discrepancy",
    "force_majeure",
    "fraudulent_misrepresentation",
]

# Relief types sought in claims (separate from dispute types)
RELIEF_TYPES = [
    "principal_amount",
    "damages",
    "lost_profits",
    "consequential_damages",
    "specific_performance",
    "declaratory",
]

CLAIM_TYPES = DISPUTE_TYPES  # Alias for spec compliance (Definition 26.4)

RULING_TYPES = [
    "final_award",
    "partial_award",
    "interim_award",
    "emergency_order",
    "consent_award",
]

DISPOSITIONS = [
    "in_favor_of_claimant",
    "in_favor_of_respondent",
    "partially_in_favor_of_claimant",
    "partially_in_favor_of_respondent",
    "dismissed",
    "settled",
]

ORDER_TYPES = [
    "monetary_damages",
    "specific_performance",
    "declaratory",
    "injunction",
    "costs",
    "interest",
]

ENFORCEMENT_METHODS = [
    "smart_asset_state_transition",
    "escrow_release",
    "manual",
    "court_enforcement",
]


# ─────────────────────────────────────────────────────────────────────────────
# Institution Registry
# ─────────────────────────────────────────────────────────────────────────────

ARBITRATION_INSTITUTIONS = {
    "difc-lcia": {
        "name": "DIFC-LCIA Arbitration Centre",
        "jurisdiction_id": "uae-difc",
        "rules_url": "https://www.difc-lcia.org/rules",
        "supported_dispute_types": DISPUTE_TYPES,
        "procedural_options": {
            "emergency_arbitrator": True,
            "expedited_procedure": True,
            "document_only": True,
            "online_hearings": True,
        },
        "fee_schedule": {
            "filing_fee": {"amount": 3000, "currency": "USD"},
            "registration_fee": {"amount": 1500, "currency": "USD"},
        },
        "enforcement_jurisdictions": [
            "uae-difc",
            "uae-adgm",
            "new_york_convention",
        ],
    },
    "siac": {
        "name": "Singapore International Arbitration Centre",
        "jurisdiction_id": "sg",
        "rules_url": "https://www.siac.org.sg/rules",
        "supported_dispute_types": DISPUTE_TYPES,
        "procedural_options": {
            "emergency_arbitrator": True,
            "expedited_procedure": True,
            "document_only": True,
            "online_hearings": True,
        },
        "fee_schedule": {
            "filing_fee": {"amount": 2000, "currency": "SGD"},
            "registration_fee": {"amount": 2140, "currency": "SGD"},
        },
        "enforcement_jurisdictions": [
            "sg",
            "new_york_convention",
        ],
    },
    "icc": {
        "name": "ICC International Court of Arbitration",
        "jurisdiction_id": "fr-paris",
        "rules_url": "https://iccwbo.org/dispute-resolution/dispute-resolution-services/arbitration/rules-procedure/",
        "supported_dispute_types": DISPUTE_TYPES,
        "procedural_options": {
            "emergency_arbitrator": True,
            "expedited_procedure": True,
            "document_only": False,
            "online_hearings": True,
        },
        "fee_schedule": {
            "filing_fee": {"amount": 5000, "currency": "USD"},
            "administrative_fee_basis": "ad_valorem",
        },
        "enforcement_jurisdictions": [
            "new_york_convention",
        ],
    },
    "aifc-iac": {
        "name": "AIFC International Arbitration Centre",
        "jurisdiction_id": "kaz-aifc",
        "rules_url": "https://aifc-iac.kz/rules",
        "supported_dispute_types": DISPUTE_TYPES,
        "procedural_options": {
            "emergency_arbitrator": True,
            "expedited_procedure": True,
            "document_only": True,
            "online_hearings": True,
        },
        "fee_schedule": {
            "filing_fee": {"amount": 2500, "currency": "USD"},
        },
        "enforcement_jurisdictions": [
            "kaz-aifc",
            "new_york_convention",
        ],
    },
}


# ─────────────────────────────────────────────────────────────────────────────
# Data Types
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class Money:
    """Monetary amount with currency."""
    amount: Decimal
    currency: str
    
    def to_dict(self) -> Dict[str, Any]:
        # CRITICAL FIX: Use string representation to preserve Decimal precision
        # Converting Decimal to float causes precision loss (e.g., 0.1 + 0.2 != 0.3)
        return {"amount": str(self.amount), "currency": self.currency}
    
    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "Money":
        return cls(amount=Decimal(str(d["amount"])), currency=d["currency"])
    
    def __add__(self, other: "Money") -> "Money":
        """Add two Money objects with same currency."""
        if self.currency != other.currency:
            raise ValueError(f"Cannot add different currencies: {self.currency} vs {other.currency}")
        return Money(amount=self.amount + other.amount, currency=self.currency)
    
    def __sub__(self, other: "Money") -> "Money":
        """Subtract two Money objects with same currency."""
        if self.currency != other.currency:
            raise ValueError(f"Cannot subtract different currencies: {self.currency} vs {other.currency}")
        return Money(amount=self.amount - other.amount, currency=self.currency)
    
    def __mul__(self, factor: Decimal) -> "Money":
        """Multiply Money by a factor."""
        return Money(amount=self.amount * factor, currency=self.currency)
    
    def __eq__(self, other: object) -> bool:
        """Check equality."""
        if not isinstance(other, Money):
            return False
        return self.amount == other.amount and self.currency == other.currency
    
    def __lt__(self, other: "Money") -> bool:
        """Less than comparison."""
        if self.currency != other.currency:
            raise ValueError(f"Cannot compare different currencies: {self.currency} vs {other.currency}")
        return self.amount < other.amount


@dataclass
class Party:
    """A party in arbitration proceedings."""
    party_id: str  # DID
    legal_name: str
    jurisdiction_id: Optional[str] = None
    email: Optional[str] = None
    address: Optional[str] = None
    legal_representative: Optional[Dict[str, str]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "party_id": self.party_id,
            "legal_name": self.legal_name,
        }
        if self.jurisdiction_id:
            d["jurisdiction_id"] = self.jurisdiction_id
        if self.email or self.address:
            d["contact"] = {}
            if self.email:
                d["contact"]["email"] = self.email
            if self.address:
                d["contact"]["address"] = self.address
        if self.legal_representative:
            d["legal_representative"] = self.legal_representative
        return d


@dataclass
class Claim:
    """A claim in a dispute."""
    claim_id: str
    claim_type: str
    description: str
    amount: Optional[Money] = None
    supporting_receipts: List[Dict[str, Any]] = field(default_factory=list)
    supporting_evidence: List[Dict[str, Any]] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "claim_id": self.claim_id,
            "claim_type": self.claim_type,
            "description": self.description,
        }
        if self.amount:
            d["amount"] = self.amount.to_dict()
        if self.supporting_receipts:
            d["supporting_receipts"] = self.supporting_receipts
        if self.supporting_evidence:
            d["supporting_evidence"] = self.supporting_evidence
        return d


@dataclass
class DisputeRequest:
    """A formal request to initiate arbitration."""
    dispute_id: str
    institution_id: str
    corridor_id: str
    claimant: Party
    respondent: Party
    dispute_type: str
    claims: List[Claim]
    relief_sought: Dict[str, Any] = field(default_factory=dict)
    evidence_bundle_ref: Optional[Dict[str, Any]] = None
    corridor_receipts_refs: List[Dict[str, Any]] = field(default_factory=list)
    governing_law: Optional[str] = None
    seat_of_arbitration: Optional[str] = None
    language: str = "en"
    procedural_preferences: Dict[str, Any] = field(default_factory=dict)
    escrow: Optional[Dict[str, Any]] = None
    created_at: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "type": "MSEZDisputeRequest",
            "stack_spec_version": STACK_SPEC_VERSION,
            "dispute_id": self.dispute_id,
            "institution_id": self.institution_id,
            "corridor_id": self.corridor_id,
            "claimant": self.claimant.to_dict(),
            "respondent": self.respondent.to_dict(),
            "dispute_type": self.dispute_type,
            "claims": [c.to_dict() for c in self.claims],
            "created_at": self.created_at or datetime.now(timezone.utc).isoformat(),
        }
        if self.relief_sought:
            d["relief_sought"] = self.relief_sought
        if self.evidence_bundle_ref:
            d["evidence_bundle_ref"] = self.evidence_bundle_ref
        if self.corridor_receipts_refs:
            d["corridor_receipts_refs"] = self.corridor_receipts_refs
        if self.governing_law:
            d["governing_law"] = self.governing_law
        if self.seat_of_arbitration:
            d["seat_of_arbitration"] = self.seat_of_arbitration
        if self.language != "en":
            d["language"] = self.language
        if self.procedural_preferences:
            d["procedural_preferences"] = self.procedural_preferences
        if self.escrow:
            d["escrow"] = self.escrow
        return d


@dataclass
class Order:
    """An order in an arbitration ruling."""
    order_id: str
    order_type: str
    obligor: str  # DID
    obligee: str  # DID
    amount: Optional[Money] = None
    due_date: Optional[str] = None
    enforcement_method: str = "manual"
    smart_asset_refs: List[Dict[str, Any]] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "order_id": self.order_id,
            "order_type": self.order_type,
            "obligor": self.obligor,
            "obligee": self.obligee,
            "enforcement_method": self.enforcement_method,
        }
        if self.amount:
            d["amount"] = self.amount.to_dict()
        if self.due_date:
            d["due_date"] = self.due_date
        if self.smart_asset_refs:
            d["smart_asset_refs"] = self.smart_asset_refs
        return d


@dataclass
class Ruling:
    """An arbitration ruling/award."""
    ruling_type: str
    disposition: str
    findings: List[Dict[str, Any]] = field(default_factory=list)
    orders: List[Order] = field(default_factory=list)
    interest: Optional[Dict[str, Any]] = None
    costs_allocation: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "ruling_type": self.ruling_type,
            "disposition": self.disposition,
            "findings": self.findings,
            "orders": [o.to_dict() for o in self.orders],
        }
        if self.interest:
            d["interest"] = self.interest
        if self.costs_allocation:
            d["costs_allocation"] = self.costs_allocation
        return d


@dataclass
class ArbitrationRulingVC:
    """A Verifiable Credential for an arbitration ruling."""
    dispute_id: str
    institution_id: str
    case_reference: str
    corridor_id: str
    claimant: Party
    respondent: Party
    ruling: Ruling
    tribunal: Dict[str, Any] = field(default_factory=dict)
    enforcement: Dict[str, Any] = field(default_factory=dict)
    appeal: Dict[str, Any] = field(default_factory=dict)
    full_award_ref: Optional[Dict[str, Any]] = None
    issuer: Optional[str] = None
    issuance_date: Optional[str] = None
    
    def to_vc(self) -> Dict[str, Any]:
        return {
            "@context": ["https://www.w3.org/2018/credentials/v1"],
            "type": ["VerifiableCredential", "MSEZArbitrationRulingCredential"],
            "issuer": self.issuer or f"did:key:z6Mk{self.institution_id}",
            "issuanceDate": self.issuance_date or datetime.now(timezone.utc).isoformat(),
            "credentialSubject": {
                "dispute_id": self.dispute_id,
                "institution_id": self.institution_id,
                "case_reference": self.case_reference,
                "corridor_id": self.corridor_id,
                "tribunal": self.tribunal,
                "parties": {
                    "claimant": self.claimant.to_dict(),
                    "respondent": self.respondent.to_dict(),
                },
                "ruling": self.ruling.to_dict(),
                "enforcement": self.enforcement,
                "appeal": self.appeal,
                "full_award_ref": self.full_award_ref,
            },
        }


@dataclass
class EnforcementReceipt:
    """Receipt of arbitration enforcement action."""
    enforcement_id: str
    ruling_vc_digest: str
    order_id: str
    corridor_id: str
    transition_type: str
    asset_id: Optional[str] = None
    escrow_release: Optional[Dict[str, Any]] = None
    enforcement_timestamp: Optional[str] = None
    receipt_digest: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": "MSEZArbitrationEnforcementReceipt",
            "stack_spec_version": STACK_SPEC_VERSION,
            "enforcement_id": self.enforcement_id,
            "ruling_vc_digest": self.ruling_vc_digest,
            "order_id": self.order_id,
            "corridor_id": self.corridor_id,
            "transition_type": self.transition_type,
            "asset_id": self.asset_id,
            "escrow_release": self.escrow_release,
            "enforcement_timestamp": self.enforcement_timestamp or datetime.now(timezone.utc).isoformat(),
        }


# ─────────────────────────────────────────────────────────────────────────────
# Arbitration Manager
# ─────────────────────────────────────────────────────────────────────────────

class ArbitrationManager:
    """Manage arbitration workflows."""
    
    def __init__(self, institution_id: str):
        if institution_id not in ARBITRATION_INSTITUTIONS:
            raise ValueError(f"Unknown institution: {institution_id}")
        self.institution_id = institution_id
        self.institution = ARBITRATION_INSTITUTIONS[institution_id]
    
    def _deterministic_timestamp(self, offset: int = 0) -> str:
        """Generate deterministic timestamp from SOURCE_DATE_EPOCH."""
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
        if epoch == 0:
            dt = datetime.now(timezone.utc)
        else:
            dt = datetime.fromtimestamp(epoch + offset, tz=timezone.utc)
        return dt.strftime("%Y-%m-%dT%H:%M:%SZ")
    
    def _deterministic_id(self, kind: str, name: str) -> str:
        """Generate deterministic ID."""
        return f"{kind}:{uuid.uuid5(NAMESPACE_ARBITRATION, name)}"
    
    def _canonical_json(self, obj: Any) -> bytes:
        """Produce canonical JSON bytes."""
        return json.dumps(
            obj,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        ).encode("utf-8")
    
    def _compute_digest(self, obj: Any) -> str:
        """Compute SHA256 digest of canonical JSON."""
        return hashlib.sha256(self._canonical_json(obj)).hexdigest()
    
    def create_dispute_request(
        self,
        corridor_id: str,
        claimant: Party,
        respondent: Party,
        dispute_type: str,
        claims: List[Claim],
        relief_sought: Optional[Dict[str, Any]] = None,
        evidence_bundle_ref: Optional[Dict[str, Any]] = None,
        governing_law: Optional[str] = None,
        expedited: bool = False,
    ) -> DisputeRequest:
        """Create a new dispute request."""
        if dispute_type not in self.institution["supported_dispute_types"]:
            raise ValueError(f"Dispute type {dispute_type} not supported by {self.institution_id}")
        
        dispute_id = self._deterministic_id(
            "dispute",
            f"{corridor_id}:{claimant.party_id}:{respondent.party_id}:{self._deterministic_timestamp()}",
        )
        
        procedural_prefs = {}
        if expedited and self.institution["procedural_options"].get("expedited_procedure"):
            procedural_prefs["expedited"] = True
        
        return DisputeRequest(
            dispute_id=dispute_id,
            institution_id=self.institution_id,
            corridor_id=corridor_id,
            claimant=claimant,
            respondent=respondent,
            dispute_type=dispute_type,
            claims=claims,
            relief_sought=relief_sought or {},
            evidence_bundle_ref=evidence_bundle_ref,
            governing_law=governing_law or self.institution["jurisdiction_id"],
            seat_of_arbitration=self.institution["jurisdiction_id"],
            procedural_preferences=procedural_prefs,
            created_at=self._deterministic_timestamp(),
        )
    
    def verify_ruling_vc(self, ruling_vc: Dict[str, Any], verify_signature: bool = True) -> Tuple[bool, List[str]]:
        """
        Verify an arbitration ruling VC.
        
        Args:
            ruling_vc: The ruling verifiable credential to verify
            verify_signature: If True, verify cryptographic signature (default: True)
            
        Returns:
            Tuple of (is_valid, list of error messages)
        """
        errors = []
        
        # Verify cryptographic signature first (most important check)
        if verify_signature:
            try:
                from tools.vc import verify_credential
                results = verify_credential(ruling_vc)
                if not results:
                    errors.append("No proof found in ruling VC")
                elif not any(r.ok for r in results):
                    sig_errors = [r.error for r in results if r.error]
                    errors.append(f"Signature verification failed: {'; '.join(sig_errors)}")
            except Exception as e:
                errors.append(f"Signature verification error: {e}")
        
        # Check type
        if "MSEZArbitrationRulingCredential" not in ruling_vc.get("type", []):
            errors.append("Missing MSEZArbitrationRulingCredential type")
        
        # Check issuer
        issuer = ruling_vc.get("issuer", "")
        if not issuer.startswith("did:"):
            errors.append("Invalid issuer DID")
        
        # Check required fields
        subject = ruling_vc.get("credentialSubject", {})
        required = ["dispute_id", "institution_id", "case_reference", "parties", "ruling"]
        for field in required:
            if field not in subject:
                errors.append(f"Missing required field: {field}")
        
        # Check ruling structure
        ruling = subject.get("ruling", {})
        if ruling.get("ruling_type") not in RULING_TYPES:
            errors.append(f"Invalid ruling_type: {ruling.get('ruling_type')}")
        if ruling.get("disposition") not in DISPOSITIONS:
            errors.append(f"Invalid disposition: {ruling.get('disposition')}")
        
        # Check orders
        for order in ruling.get("orders", []):
            if order.get("order_type") not in ORDER_TYPES:
                errors.append(f"Invalid order_type: {order.get('order_type')}")
            if order.get("enforcement_method") not in ENFORCEMENT_METHODS:
                errors.append(f"Invalid enforcement_method: {order.get('enforcement_method')}")
        
        return len(errors) == 0, errors
    
    def create_enforcement_receipt(
        self,
        ruling_vc: Dict[str, Any],
        order_id: str,
        corridor_id: str,
        transition_type: str,
        asset_id: Optional[str] = None,
    ) -> EnforcementReceipt:
        """Create an enforcement receipt for an arbitration order."""
        ruling_digest = self._compute_digest(ruling_vc)
        
        enforcement_id = self._deterministic_id(
            "enforcement",
            f"{ruling_digest}:{order_id}:{self._deterministic_timestamp()}",
        )
        
        return EnforcementReceipt(
            enforcement_id=enforcement_id,
            ruling_vc_digest=ruling_digest,
            order_id=order_id,
            corridor_id=corridor_id,
            transition_type=transition_type,
            asset_id=asset_id,
            enforcement_timestamp=self._deterministic_timestamp(),
        )
    
    def can_enforce_in_jurisdiction(self, jurisdiction_id: str) -> bool:
        """Check if awards from this institution are enforceable in a jurisdiction."""
        enforceable = self.institution.get("enforcement_jurisdictions", [])
        
        if jurisdiction_id in enforceable:
            return True
        
        # New York Convention covers ~170 signatories
        if "new_york_convention" in enforceable:
            # Simplified check - in production would have full list
            return True
        
        return False
    
    def ruling_to_trigger(self, ruling_vc: Dict[str, Any]) -> Dict[str, Any]:
        """
        Generate an agentic trigger from an arbitration ruling.
        
        This bridges arbitration rulings to the agentic execution system,
        enabling automated enforcement of arbitration orders.
        
        Returns a trigger dict compatible with AgenticTrigger.
        """
        subject = ruling_vc.get("credentialSubject", {})
        ruling = subject.get("ruling", {})
        
        # Extract order details for enforcement
        orders = []
        for order in ruling.get("orders", []):
            orders.append({
                "order_id": order.get("order_id"),
                "order_type": order.get("order_type"),
                "obligor": order.get("obligor"),
                "obligee": order.get("obligee"),
                "amount": order.get("amount"),
                "enforcement_method": order.get("enforcement_method"),
                "smart_asset_refs": order.get("smart_asset_refs", []),
            })
        
        return {
            "trigger_type": "ruling_received",
            "data": {
                "dispute_id": subject.get("dispute_id"),
                "institution_id": subject.get("institution_id"),
                "case_reference": subject.get("case_reference"),
                "corridor_id": subject.get("corridor_id"),
                "ruling_type": ruling.get("ruling_type"),
                "disposition": ruling.get("disposition"),
                "orders": orders,
                "claimant_id": subject.get("parties", {}).get("claimant", {}).get("party_id"),
                "respondent_id": subject.get("parties", {}).get("respondent", {}).get("party_id"),
                "ruling_vc_digest": self._compute_digest(ruling_vc),
                "enforcement": subject.get("enforcement", {}),
                "appeal": subject.get("appeal", {}),
            },
            "timestamp": ruling_vc.get("issuanceDate", self._deterministic_timestamp()),
        }
    
    def create_enforcement_transitions(
        self,
        ruling_vc: Dict[str, Any],
    ) -> List[Dict[str, Any]]:
        """
        Generate transition envelopes for automated enforcement of all orders.
        
        For each order with enforcement_method='smart_asset_state_transition',
        creates the appropriate transition envelope.
        """
        transitions = []
        subject = ruling_vc.get("credentialSubject", {})
        ruling = subject.get("ruling", {})
        
        for order in ruling.get("orders", []):
            if order.get("enforcement_method") != "smart_asset_state_transition":
                continue
            
            for asset_ref in order.get("smart_asset_refs", []):
                asset_id = asset_ref.get("asset_id")
                if not asset_id:
                    continue
                
                # Determine transition type based on order
                order_type = order.get("order_type")
                if order_type == "monetary_damages":
                    transition_kind = "transfer"
                    params = {
                        "from": order.get("obligor"),
                        "to": order.get("obligee"),
                        "amount": order.get("amount", {}).get("amount"),
                        "currency": order.get("amount", {}).get("currency"),
                        "reason": f"arbitration_order:{order.get('order_id')}",
                    }
                elif order_type == "injunction":
                    transition_kind = "halt"
                    params = {
                        "reason": f"arbitration_injunction:{order.get('order_id')}",
                    }
                else:
                    transition_kind = "arbitration_enforce"
                    params = {
                        "order": order,
                    }
                
                transitions.append({
                    "asset_id": asset_id,
                    "transition_kind": transition_kind,
                    "params": params,
                    "order_id": order.get("order_id"),
                    "ruling_vc_digest": self._compute_digest(ruling_vc),
                })
        
        return transitions


# ─────────────────────────────────────────────────────────────────────────────
# Transition Types for Arbitration
# ─────────────────────────────────────────────────────────────────────────────

ARBITRATION_TRANSITION_TYPES = {
    "arbitration.dispute.file.v1": {
        "transition_type_id": "arbitration.dispute.file.v1",
        "title": "File Arbitration Dispute",
        "description": "File a dispute with an arbitration institution",
        "payload_schema_ref": "arbitration.dispute-request.schema.json",
        "required_attestations": ["corridor_participant"],
        "effects": ["creates_dispute", "locks_escrow"],
    },
    "arbitration.dispute.respond.v1": {
        "transition_type_id": "arbitration.dispute.respond.v1",
        "title": "Respond to Arbitration Dispute",
        "description": "Respondent's answer to dispute claims",
        "payload_schema_ref": "arbitration.dispute-response.schema.json",
        "required_attestations": ["corridor_participant"],
        "effects": ["updates_dispute_state"],
    },
    "arbitration.evidence.submit.v1": {
        "transition_type_id": "arbitration.evidence.submit.v1",
        "title": "Submit Arbitration Evidence",
        "description": "Submit evidence bundle to arbitration proceedings",
        "payload_schema_ref": "arbitration.evidence-package.schema.json",
        "required_attestations": ["corridor_participant"],
        "effects": ["appends_evidence"],
    },
    "arbitration.ruling.receive.v1": {
        "transition_type_id": "arbitration.ruling.receive.v1",
        "title": "Receive Arbitration Ruling",
        "description": "Record receipt of arbitration ruling/award",
        "payload_schema_ref": "vc.arbitration-ruling.schema.json",
        "required_attestations": ["arbitration_institution"],
        "effects": ["records_ruling", "starts_appeal_period"],
    },
    "arbitration.ruling.enforce.v1": {
        "transition_type_id": "arbitration.ruling.enforce.v1",
        "title": "Enforce Arbitration Ruling",
        "description": "Execute enforcement of arbitration order",
        "payload_schema_ref": "arbitration.enforcement-receipt.schema.json",
        "required_attestations": ["ruling_exists", "appeal_period_expired"],
        "effects": ["executes_order", "releases_escrow", "transitions_asset"],
    },
    "arbitration.appeal.file.v1": {
        "transition_type_id": "arbitration.appeal.file.v1",
        "title": "File Appeal",
        "description": "File an appeal against arbitration ruling",
        "payload_schema_ref": "arbitration.appeal-request.schema.json",
        "required_attestations": ["ruling_exists", "within_appeal_period"],
        "effects": ["suspends_enforcement"],
    },
    "arbitration.settlement.agree.v1": {
        "transition_type_id": "arbitration.settlement.agree.v1",
        "title": "Agree Settlement",
        "description": "Parties agree to settlement during arbitration",
        "payload_schema_ref": "arbitration.settlement-agreement.schema.json",
        "required_attestations": ["both_parties_sign"],
        "effects": ["closes_dispute", "records_settlement"],
    },
    # Definition 26.7 - Escrow transitions
    "arbitration.escrow.release.v1": {
        "transition_type_id": "arbitration.escrow.release.v1",
        "title": "Release Escrow",
        "description": "Release escrowed funds upon condition satisfaction",
        "payload_schema_ref": "arbitration.escrow.schema.json",
        "required_attestations": ["release_condition_met"],
        "effects": ["releases_funds", "closes_escrow"],
        "params": {
            "escrow_id": "EscrowID",
            "release_condition": "ReleaseCondition",
            "beneficiary": "DID",
            "amount": "u64",
        },
    },
    "arbitration.escrow.forfeit.v1": {
        "transition_type_id": "arbitration.escrow.forfeit.v1",
        "title": "Forfeit Escrow",
        "description": "Forfeit escrowed funds per ruling order",
        "payload_schema_ref": "arbitration.escrow.schema.json",
        "required_attestations": ["ruling_orders_forfeit"],
        "effects": ["forfeits_funds", "closes_escrow"],
        "params": {
            "escrow_id": "EscrowID",
            "forfeit_reason": "ForfeitReason",
            "ruling_vc_ref": "ArtifactRef",
        },
    },
}


# ─────────────────────────────────────────────────────────────────────────────
# Escrow Management (Definition 26.7)
# ─────────────────────────────────────────────────────────────────────────────

ESCROW_TYPES = [
    "filing_fee",
    "security_deposit",
    "award_escrow",
    "appeal_bond",
]

ESCROW_STATUSES = [
    "pending",
    "funded",
    "partially_released",
    "fully_released",
    "forfeited",
]

RELEASE_CONDITIONS = [
    "RulingEnforced",
    "AppealPeriodExpired",
    "SettlementAgreed",
    "DisputeWithdrawn",
    "InstitutionOrder",
]


@dataclass
class EscrowTransaction:
    """A single escrow transaction."""
    transaction_type: str  # deposit, partial_release, full_release, forfeit, refund
    amount: Money
    timestamp: str
    recipient: Optional[str] = None
    reason: Optional[str] = None
    ruling_ref: Optional[Dict[str, Any]] = None
    transaction_proof: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "transaction_type": self.transaction_type,
            "amount": self.amount.to_dict(),
            "timestamp": self.timestamp,
        }
        if self.recipient:
            d["recipient"] = self.recipient
        if self.reason:
            d["reason"] = self.reason
        if self.ruling_ref:
            d["ruling_ref"] = self.ruling_ref
        if self.transaction_proof:
            d["transaction_proof"] = self.transaction_proof
        return d


@dataclass
class ReleaseCondition:
    """Condition for escrow release."""
    condition_type: str
    reference: Optional[Dict[str, Any]] = None
    satisfied: bool = False
    satisfied_at: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "condition_type": self.condition_type,
            "satisfied": self.satisfied,
        }
        if self.reference:
            d["reference"] = self.reference
        if self.satisfied_at:
            d["satisfied_at"] = self.satisfied_at
        return d


@dataclass
class Escrow:
    """Escrow for arbitration proceedings."""
    escrow_id: str
    dispute_id: str
    escrow_type: str
    amount: Money
    status: str = "pending"
    depositor: Optional[str] = None
    beneficiary: Optional[str] = None
    custody_address: Optional[str] = None
    release_conditions: List[ReleaseCondition] = field(default_factory=list)
    transactions: List[EscrowTransaction] = field(default_factory=list)
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    updated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def __post_init__(self):
        if self.escrow_type not in ESCROW_TYPES:
            raise ValueError(f"Invalid escrow_type: {self.escrow_type}")
        if self.status not in ESCROW_STATUSES:
            raise ValueError(f"Invalid escrow status: {self.status}")
    
    def deposit(self, transaction_proof: Optional[str] = None) -> EscrowTransaction:
        """Record a deposit to the escrow."""
        tx = EscrowTransaction(
            transaction_type="deposit",
            amount=self.amount,
            timestamp=datetime.now(timezone.utc).isoformat(),
            transaction_proof=transaction_proof,
        )
        self.transactions.append(tx)
        self.status = "funded"
        self.updated_at = tx.timestamp
        return tx
    
    def release(self, recipient: str, reason: str, 
                ruling_ref: Optional[Dict[str, Any]] = None) -> EscrowTransaction:
        """Release escrow funds to recipient."""
        if self.status not in ["funded", "partially_released"]:
            raise ValueError(f"Cannot release escrow in status: {self.status}")
        
        tx = EscrowTransaction(
            transaction_type="full_release",
            amount=self.amount,
            timestamp=datetime.now(timezone.utc).isoformat(),
            recipient=recipient,
            reason=reason,
            ruling_ref=ruling_ref,
        )
        self.transactions.append(tx)
        self.status = "fully_released"
        self.updated_at = tx.timestamp
        return tx
    
    def forfeit(self, reason: str, ruling_ref: Dict[str, Any]) -> EscrowTransaction:
        """Forfeit escrow funds (e.g., due to adverse ruling)."""
        if self.status != "funded":
            raise ValueError(f"Cannot forfeit escrow in status: {self.status}")
        
        tx = EscrowTransaction(
            transaction_type="forfeit",
            amount=self.amount,
            timestamp=datetime.now(timezone.utc).isoformat(),
            reason=reason,
            ruling_ref=ruling_ref,
        )
        self.transactions.append(tx)
        self.status = "forfeited"
        self.updated_at = tx.timestamp
        return tx
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "escrow_id": self.escrow_id,
            "dispute_id": self.dispute_id,
            "escrow_type": self.escrow_type,
            "amount": self.amount.to_dict(),
            "status": self.status,
            "depositor": self.depositor,
            "beneficiary": self.beneficiary,
            "custody_address": self.custody_address,
            "release_conditions": [rc.to_dict() for rc in self.release_conditions],
            "transactions": [tx.to_dict() for tx in self.transactions],
            "created_at": self.created_at,
            "updated_at": self.updated_at,
        }


# ─────────────────────────────────────────────────────────────────────────────
# Settlement Agreement (Definition 26.7 - DisputeSettle)
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class SettlementPayment:
    """Payment term in a settlement."""
    from_party: str
    to_party: str
    amount: Money
    due_date: str
    payment_method: str = "smart_asset_transfer"
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "from": self.from_party,
            "to": self.to_party,
            "amount": self.amount.to_dict(),
            "due_date": self.due_date,
            "payment_method": self.payment_method,
        }


@dataclass
class SettlementTerms:
    """Terms of a settlement agreement."""
    monetary_terms: Optional[Dict[str, Any]] = None
    asset_transfer_terms: Optional[Dict[str, Any]] = None
    mutual_release: bool = True
    non_admission: bool = True
    specific_performance: List[Dict[str, Any]] = field(default_factory=list)
    confidentiality_clause: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "mutual_release": self.mutual_release,
            "non_admission": self.non_admission,
        }
        if self.monetary_terms:
            d["monetary_terms"] = self.monetary_terms
        if self.asset_transfer_terms:
            d["asset_transfer_terms"] = self.asset_transfer_terms
        if self.specific_performance:
            d["specific_performance"] = self.specific_performance
        if self.confidentiality_clause:
            d["confidentiality_clause"] = self.confidentiality_clause
        return d


@dataclass
class Settlement:
    """Settlement agreement for dispute resolution."""
    settlement_id: str
    dispute_id: str
    parties: List[Dict[str, Any]]
    terms: SettlementTerms
    effective_date: str
    execution_deadline: Optional[str] = None
    institution_acknowledgment: Optional[Dict[str, Any]] = None
    smart_asset_transitions: List[Dict[str, Any]] = field(default_factory=list)
    confidentiality_level: str = "parties_only"
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "settlement_id": self.settlement_id,
            "dispute_id": self.dispute_id,
            "parties": self.parties,
            "terms": self.terms.to_dict(),
            "effective_date": self.effective_date,
            "execution_deadline": self.execution_deadline,
            "institution_acknowledgment": self.institution_acknowledgment,
            "smart_asset_transitions": self.smart_asset_transitions,
            "confidentiality_level": self.confidentiality_level,
        }
    
    @staticmethod
    def create_settlement_id() -> str:
        return f"settlement:{uuid.uuid4().hex}"


# ─────────────────────────────────────────────────────────────────────────────
# Evidence Package (Definition 26.5)
# ─────────────────────────────────────────────────────────────────────────────

EVIDENCE_TYPES = [
    "SmartAssetReceipt",
    "CorridorReceipt",
    "ComplianceEvidence",
    "ExpertReport",
    "WitnessStatement",
    "ContractDocument",
    "CommunicationRecord",
    "PaymentRecord",
    "ShippingDocument",
    "InspectionReport",
]

AUTHENTICITY_TYPES = [
    "CorridorCheckpointInclusion",
    "SmartAssetCheckpointInclusion",
    "NotarizedDocument",
    "ExpertCertification",
    "ChainOfCustody",
]


@dataclass
class AuthenticityAttestation:
    """Attestation proving evidence authenticity."""
    attestation_type: str
    proof_ref: Optional[Dict[str, Any]] = None
    
    def __post_init__(self):
        if self.attestation_type not in AUTHENTICITY_TYPES:
            raise ValueError(f"Invalid attestation_type: {self.attestation_type}")
    
    def to_dict(self) -> Dict[str, Any]:
        d = {"attestation_type": self.attestation_type}
        if self.proof_ref:
            d["proof_ref"] = self.proof_ref
        return d


@dataclass
class EvidenceItem:
    """A single piece of evidence."""
    evidence_id: str
    evidence_type: str
    description: str
    artifact_ref: Dict[str, Any]
    relevance: Optional[str] = None
    authenticity_attestation: Optional[AuthenticityAttestation] = None
    
    def __post_init__(self):
        if self.evidence_type not in EVIDENCE_TYPES:
            raise ValueError(f"Invalid evidence_type: {self.evidence_type}")
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "evidence_id": self.evidence_id,
            "evidence_type": self.evidence_type,
            "description": self.description,
            "artifact_ref": self.artifact_ref,
        }
        if self.relevance:
            d["relevance"] = self.relevance
        if self.authenticity_attestation:
            d["authenticity_attestation"] = self.authenticity_attestation.to_dict()
        return d


@dataclass
class EvidencePackage:
    """Evidence package for arbitration proceedings (Definition 26.5)."""
    evidence_package_id: str
    dispute_id: str
    submitting_party: str
    evidence_items: List[EvidenceItem]
    witness_bundle_ref: Dict[str, Any]
    submission_timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    submission_signature: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "evidence_package_id": self.evidence_package_id,
            "dispute_id": self.dispute_id,
            "submitting_party": self.submitting_party,
            "submission_timestamp": self.submission_timestamp,
            "evidence_items": [item.to_dict() for item in self.evidence_items],
            "witness_bundle_ref": self.witness_bundle_ref,
        }
        if self.submission_signature:
            d["submission_signature"] = self.submission_signature
        return d
    
    @staticmethod
    def create_evidence_package_id() -> str:
        return f"evidence-pkg:{uuid.uuid4().hex}"
    
    @staticmethod
    def create_evidence_id() -> str:
        return f"evidence:{uuid.uuid4().hex}"


# ─────────────────────────────────────────────────────────────────────────────
# Exports
# ─────────────────────────────────────────────────────────────────────────────

__all__ = [
    "STACK_SPEC_VERSION",
    "DISPUTE_TYPES",
    "CLAIM_TYPES",
    "RELIEF_TYPES",
    "RULING_TYPES",
    "DISPOSITIONS",
    "ORDER_TYPES",
    "ENFORCEMENT_METHODS",
    "ESCROW_TYPES",
    "ESCROW_STATUSES",
    "RELEASE_CONDITIONS",
    "EVIDENCE_TYPES",
    "AUTHENTICITY_TYPES",
    "ARBITRATION_INSTITUTIONS",
    "ARBITRATION_TRANSITION_TYPES",
    "Money",
    "Party",
    "Claim",
    "DisputeRequest",
    "Order",
    "Ruling",
    "ArbitrationRulingVC",
    "EnforcementReceipt",
    "EscrowTransaction",
    "ReleaseCondition",
    "Escrow",
    "SettlementPayment",
    "SettlementTerms",
    "Settlement",
    "AuthenticityAttestation",
    "EvidenceItem",
    "EvidencePackage",
    "ArbitrationManager",
]
