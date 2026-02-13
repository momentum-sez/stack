"""
PHOENIX Watcher Economy and Accountability

Transforms watchers from passive observers to accountable economic actors. This module
implements the bond, slashing, and reputation infrastructure that ensures watchers
have skin in the game for their attestations.

Economic Model:

    Watchers stake collateral (bonds) proportional to the value they attest.
    Incorrect attestations result in slashing - loss of bonded collateral.
    Reputation accumulates based on attestation accuracy and availability.
    
Slashing Conditions:

    1. EQUIVOCATION: Signing conflicting attestations for same state (100% slash)
    2. AVAILABILITY_FAILURE: Missing required attestations within SLA (1% per incident)
    3. FALSE_ATTESTATION: Attesting to invalid state transitions (50% slash)
    4. COLLUSION: Coordinated false attestation (100% slash + permanent ban)

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum, auto
from typing import Any, Dict, FrozenSet, List, Optional, Set, Tuple


# =============================================================================
# WATCHER IDENTITY
# =============================================================================

@dataclass(frozen=True)
class WatcherId:
    """Unique identifier for a watcher."""
    did: str  # Decentralized identifier
    public_key_hex: str  # Ed25519 public key
    
    def __str__(self) -> str:
        return self.did
    
    @property
    def short_id(self) -> str:
        return self.did[-12:]


# =============================================================================
# WATCHER BOND
# =============================================================================

class BondStatus(Enum):
    """Status of a watcher bond."""
    PENDING = "pending"  # Bond submitted, awaiting confirmation
    ACTIVE = "active"  # Bond confirmed and active
    PARTIALLY_SLASHED = "partially_slashed"  # Some collateral slashed
    FULLY_SLASHED = "fully_slashed"  # All collateral slashed
    WITHDRAWN = "withdrawn"  # Bond withdrawn by watcher
    EXPIRED = "expired"  # Bond validity period ended


@dataclass
class WatcherBond:
    """
    A watcher's collateral bond.
    
    Bonds are posted by watchers to back their attestations.
    The bond amount determines the maximum value the watcher
    can attest to (typically 10x the bond amount).
    """
    bond_id: str
    watcher_id: WatcherId
    
    # Collateral details
    collateral_amount: Decimal
    collateral_currency: str  # "USDC", "ETH", etc.
    collateral_address: str  # On-chain address holding collateral
    
    # Scope
    scope_jurisdictions: FrozenSet[str] = field(default_factory=frozenset)
    scope_asset_classes: FrozenSet[str] = field(default_factory=frozenset)
    max_attestation_value_usd: Decimal = Decimal("0")
    
    # Validity period
    valid_from: str = ""
    valid_until: str = ""
    
    # Status
    status: BondStatus = BondStatus.PENDING
    slashed_amount: Decimal = Decimal("0")
    slash_count: int = 0
    
    # Attestation tracking
    attestation_volume_usd: Decimal = Decimal("0")  # 30-day rolling
    attestation_count: int = 0
    
    # Metadata
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    last_updated: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def __post_init__(self):
        if not self.valid_from:
            self.valid_from = datetime.now(timezone.utc).isoformat()
        if not self.valid_until:
            # Default 1 year validity
            expiry = datetime.now(timezone.utc) + timedelta(days=365)
            self.valid_until = expiry.isoformat()
        if self.max_attestation_value_usd == Decimal("0"):
            # Default 10x collateral
            self.max_attestation_value_usd = self.collateral_amount * Decimal("10")
    
    @property
    def available_collateral(self) -> Decimal:
        """Collateral remaining after slashing."""
        return self.collateral_amount - self.slashed_amount
    
    @property
    def is_active(self) -> bool:
        return self.status == BondStatus.ACTIVE
    
    @property
    def is_valid(self) -> bool:
        """Check if bond is currently valid."""
        if not self.is_active:
            return False
        from tools.phoenix.hardening import parse_iso_timestamp
        now = datetime.now(timezone.utc)
        valid_from = parse_iso_timestamp(self.valid_from)
        valid_until = parse_iso_timestamp(self.valid_until)
        return valid_from <= now <= valid_until
    
    def can_attest(self, value_usd: Decimal) -> bool:
        """Check if watcher can attest to given value."""
        if not self.is_valid:
            return False
        return value_usd <= self.max_attestation_value_usd
    
    def slash(self, amount: Decimal, reason: str) -> Decimal:
        """
        Slash the bond by given amount.
        
        Returns actual amount slashed (may be less if insufficient).
        """
        actual_slash = min(amount, self.available_collateral)
        self.slashed_amount += actual_slash
        self.slash_count += 1
        self.last_updated = datetime.now(timezone.utc).isoformat()
        
        if self.available_collateral <= 0:
            self.status = BondStatus.FULLY_SLASHED
        else:
            self.status = BondStatus.PARTIALLY_SLASHED
        
        return actual_slash
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "bond_id": self.bond_id,
            "watcher_did": self.watcher_id.did,
            "collateral_amount": str(self.collateral_amount),
            "collateral_currency": self.collateral_currency,
            "collateral_address": self.collateral_address,
            "scope_jurisdictions": list(self.scope_jurisdictions),
            "scope_asset_classes": list(self.scope_asset_classes),
            "max_attestation_value_usd": str(self.max_attestation_value_usd),
            "valid_from": self.valid_from,
            "valid_until": self.valid_until,
            "status": self.status.value,
            "available_collateral": str(self.available_collateral),
            "slashed_amount": str(self.slashed_amount),
            "slash_count": self.slash_count,
            "attestation_volume_usd": str(self.attestation_volume_usd),
            "attestation_count": self.attestation_count,
        }


# =============================================================================
# SLASHING
# =============================================================================

class SlashingCondition(Enum):
    """Conditions that trigger slashing."""
    EQUIVOCATION = "equivocation"  # Signed conflicting attestations
    AVAILABILITY_FAILURE = "availability_failure"  # Missed required attestation
    FALSE_ATTESTATION = "false_attestation"  # Attested to invalid state
    COLLUSION = "collusion"  # Coordinated false attestation
    SAFETY_VIOLATION = "safety_violation"  # Violated protocol safety rules
    LIVENESS_VIOLATION = "liveness_violation"  # Violated protocol liveness rules


# Slash percentages by condition
SLASH_PERCENTAGES: Dict[SlashingCondition, Decimal] = {
    SlashingCondition.EQUIVOCATION: Decimal("1.00"),  # 100%
    SlashingCondition.AVAILABILITY_FAILURE: Decimal("0.01"),  # 1%
    SlashingCondition.FALSE_ATTESTATION: Decimal("0.50"),  # 50%
    SlashingCondition.COLLUSION: Decimal("1.00"),  # 100%
    SlashingCondition.SAFETY_VIOLATION: Decimal("0.75"),  # 75%
    SlashingCondition.LIVENESS_VIOLATION: Decimal("0.10"),  # 10%
}


@dataclass
class SlashingEvidence:
    """Evidence supporting a slashing claim."""
    evidence_type: str
    evidence_data: Dict[str, Any]
    evidence_digest: str = ""

    def __post_init__(self):
        if not self.evidence_digest:
            from tools.lawpack import jcs_canonicalize
            self.evidence_digest = hashlib.sha256(jcs_canonicalize(self.evidence_data)).hexdigest()


@dataclass
class EquivocationEvidence:
    """Evidence of watcher equivocation - signing conflicting attestations."""
    attestation_1: Dict[str, Any]
    attestation_2: Dict[str, Any]
    evidence_type: str = field(default="equivocation", init=False)
    evidence_data: Dict[str, Any] = field(default_factory=dict, init=False)
    evidence_digest: str = field(default="", init=False)
    
    def __post_init__(self):
        self.evidence_data = {
            "attestation_1": self.attestation_1,
            "attestation_2": self.attestation_2,
        }
        from tools.lawpack import jcs_canonicalize
        self.evidence_digest = hashlib.sha256(jcs_canonicalize(self.evidence_data)).hexdigest()


@dataclass
class SlashingClaim:
    """
    A claim requesting slashing of a watcher's bond.
    
    Claims go through a challenge period before execution.
    """
    claim_id: str
    watcher_id: WatcherId
    condition: SlashingCondition
    evidence: SlashingEvidence
    
    # Claimant
    claimant_did: str
    claimant_signature: bytes = b""
    
    # Claim details
    claimed_slash_amount: Decimal = Decimal("0")
    claimed_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    # Challenge period
    challenge_deadline: str = ""
    is_challenged: bool = False
    challenge_evidence: Optional[Dict[str, Any]] = None
    
    # Resolution
    is_resolved: bool = False
    resolution: Optional[str] = None  # "executed", "rejected", "partial"
    actual_slash_amount: Decimal = Decimal("0")
    resolved_at: Optional[str] = None
    
    def __post_init__(self):
        if not self.challenge_deadline:
            # 7 day challenge period
            deadline = datetime.now(timezone.utc) + timedelta(days=7)
            self.challenge_deadline = deadline.isoformat()
    
    @property
    def is_past_challenge_period(self) -> bool:
        from tools.phoenix.hardening import parse_iso_timestamp
        now = datetime.now(timezone.utc)
        deadline = parse_iso_timestamp(self.challenge_deadline)
        return now > deadline
    
    @property
    def can_be_executed(self) -> bool:
        """Check if claim can be executed."""
        return (
            not self.is_resolved and
            self.is_past_challenge_period and
            not self.is_challenged
        )
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "claim_id": self.claim_id,
            "watcher_did": self.watcher_id.did,
            "condition": self.condition.value,
            "evidence_digest": self.evidence.evidence_digest,
            "claimant_did": self.claimant_did,
            "claimed_slash_amount": str(self.claimed_slash_amount),
            "claimed_at": self.claimed_at,
            "challenge_deadline": self.challenge_deadline,
            "is_challenged": self.is_challenged,
            "is_resolved": self.is_resolved,
            "resolution": self.resolution,
            "actual_slash_amount": str(self.actual_slash_amount),
        }


# =============================================================================
# REPUTATION
# =============================================================================

@dataclass
class ReputationMetrics:
    """Metrics contributing to watcher reputation."""
    # Availability
    required_attestations: int = 0
    delivered_attestations: int = 0
    on_time_attestations: int = 0
    
    # Accuracy
    challenged_attestations: int = 0
    successful_challenges: int = 0  # Challenges against this watcher that succeeded
    failed_challenges: int = 0  # Challenges that failed (watcher was correct)
    
    # History
    total_attested_value_usd: Decimal = Decimal("0")
    slash_incidents: int = 0
    total_slashed_usd: Decimal = Decimal("0")
    
    # Tenure
    active_since: str = ""
    continuous_active_days: int = 0
    
    @property
    def availability_score(self) -> float:
        """Score from 0-100 based on attestation delivery."""
        if self.required_attestations == 0:
            return 100.0
        delivered_rate = self.delivered_attestations / self.required_attestations
        on_time_rate = self.on_time_attestations / max(self.delivered_attestations, 1)
        return min(100.0, delivered_rate * 50 + on_time_rate * 50)
    
    @property
    def accuracy_score(self) -> float:
        """Score from 0-100 based on attestation accuracy."""
        total_challenges = self.successful_challenges + self.failed_challenges
        if total_challenges == 0:
            return 100.0
        accuracy_rate = self.failed_challenges / total_challenges
        return accuracy_rate * 100
    
    @property
    def tenure_bonus(self) -> float:
        """Bonus points (0-20) based on tenure."""
        if self.continuous_active_days < 30:
            return 0.0
        elif self.continuous_active_days < 90:
            return 5.0
        elif self.continuous_active_days < 180:
            return 10.0
        elif self.continuous_active_days < 365:
            return 15.0
        return 20.0


@dataclass
class WatcherReputation:
    """
    A watcher's reputation score and history.
    
    Reputation is computed from metrics and determines:
    - Which corridors the watcher can participate in
    - Fee tier for the watcher's services
    - Priority in watcher selection
    """
    watcher_id: WatcherId
    metrics: ReputationMetrics
    
    # Computed scores
    overall_score: float = 0.0
    tier: str = "standard"  # "novice", "standard", "trusted", "elite"
    
    # History
    score_history: List[Tuple[str, float]] = field(default_factory=list)
    last_computed: str = ""
    
    def compute_score(self) -> float:
        """Compute overall reputation score."""
        availability = self.metrics.availability_score * 0.4
        accuracy = self.metrics.accuracy_score * 0.5
        tenure = self.metrics.tenure_bonus * 0.1
        
        # Penalty for slashing
        slash_penalty = min(30.0, self.metrics.slash_incidents * 10)
        
        score = availability + accuracy + tenure - slash_penalty
        self.overall_score = max(0.0, min(100.0, score))
        self.last_computed = datetime.now(timezone.utc).isoformat()
        self.score_history.append((self.last_computed, self.overall_score))
        
        # Determine tier
        if self.overall_score >= 95:
            self.tier = "elite"
        elif self.overall_score >= 85:
            self.tier = "trusted"
        elif self.overall_score >= 70:
            self.tier = "standard"
        else:
            self.tier = "novice"
        
        return self.overall_score
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "watcher_did": self.watcher_id.did,
            "overall_score": self.overall_score,
            "tier": self.tier,
            "availability_score": self.metrics.availability_score,
            "accuracy_score": self.metrics.accuracy_score,
            "tenure_bonus": self.metrics.tenure_bonus,
            "slash_incidents": self.metrics.slash_incidents,
            "last_computed": self.last_computed,
        }


# =============================================================================
# WATCHER REGISTRY
# =============================================================================

class WatcherRegistry:
    """
    Registry of watchers, their bonds, and reputation.
    
    The registry manages the lifecycle of watcher participation
    and provides queries for watcher selection.
    """
    
    def __init__(self):
        self._watchers: Dict[str, WatcherId] = {}  # did -> WatcherId
        self._bonds: Dict[str, WatcherBond] = {}  # bond_id -> WatcherBond
        self._watcher_bonds: Dict[str, List[str]] = {}  # did -> [bond_ids]
        self._reputations: Dict[str, WatcherReputation] = {}  # did -> reputation
        self._claims: Dict[str, SlashingClaim] = {}  # claim_id -> claim
        self._banned: Set[str] = set()  # Permanently banned DIDs
    
    def register_watcher(self, watcher_id: WatcherId) -> bool:
        """Register a new watcher."""
        if watcher_id.did in self._banned:
            return False
        
        self._watchers[watcher_id.did] = watcher_id
        self._watcher_bonds[watcher_id.did] = []
        self._reputations[watcher_id.did] = WatcherReputation(
            watcher_id=watcher_id,
            metrics=ReputationMetrics(
                active_since=datetime.now(timezone.utc).isoformat()
            ),
        )
        
        return True
    
    def post_bond(self, bond: WatcherBond) -> bool:
        """Post a new bond for a watcher."""
        if bond.watcher_id.did not in self._watchers:
            return False
        if bond.watcher_id.did in self._banned:
            return False
        
        self._bonds[bond.bond_id] = bond
        self._watcher_bonds[bond.watcher_id.did].append(bond.bond_id)
        
        return True
    
    def activate_bond(self, bond_id: str) -> bool:
        """Activate a pending bond."""
        bond = self._bonds.get(bond_id)
        if not bond or bond.status != BondStatus.PENDING:
            return False
        
        bond.status = BondStatus.ACTIVE
        bond.last_updated = datetime.now(timezone.utc).isoformat()
        return True
    
    def get_watcher(self, did: str) -> Optional[WatcherId]:
        return self._watchers.get(did)
    
    def get_bond(self, bond_id: str) -> Optional[WatcherBond]:
        return self._bonds.get(bond_id)
    
    def get_watcher_bonds(self, did: str) -> List[WatcherBond]:
        """Get all bonds for a watcher."""
        bond_ids = self._watcher_bonds.get(did, [])
        return [self._bonds[bid] for bid in bond_ids if bid in self._bonds]
    
    def get_active_bond(self, did: str) -> Optional[WatcherBond]:
        """Get the active bond for a watcher."""
        for bond in self.get_watcher_bonds(did):
            if bond.is_valid:
                return bond
        return None
    
    def get_reputation(self, did: str) -> Optional[WatcherReputation]:
        return self._reputations.get(did)
    
    def file_slashing_claim(self, claim: SlashingClaim) -> bool:
        """File a new slashing claim."""
        if claim.watcher_id.did not in self._watchers:
            return False
        
        self._claims[claim.claim_id] = claim
        return True
    
    def challenge_claim(
        self,
        claim_id: str,
        challenge_evidence: Dict[str, Any],
    ) -> bool:
        """Challenge a slashing claim."""
        claim = self._claims.get(claim_id)
        if not claim or claim.is_resolved:
            return False
        if claim.is_past_challenge_period:
            return False
        
        claim.is_challenged = True
        claim.challenge_evidence = challenge_evidence
        return True
    
    def execute_claim(self, claim_id: str) -> Optional[Decimal]:
        """
        Execute a slashing claim.
        
        Returns the amount slashed, or None if claim cannot be executed.
        """
        claim = self._claims.get(claim_id)
        if not claim or not claim.can_be_executed:
            return None
        
        # Get watcher's active bond
        bond = self.get_active_bond(claim.watcher_id.did)
        if not bond:
            claim.is_resolved = True
            claim.resolution = "no_bond"
            claim.resolved_at = datetime.now(timezone.utc).isoformat()
            return Decimal("0")
        
        # Calculate slash amount with zero collateral protection
        if bond.collateral_amount <= 0:
            claim.is_resolved = True
            claim.resolution = "zero_collateral"
            claim.actual_slash_amount = Decimal("0")
            claim.resolved_at = datetime.now(timezone.utc).isoformat()
            return Decimal("0")

        slash_percentage = SLASH_PERCENTAGES.get(claim.condition, Decimal("0.10"))
        slash_amount = bond.collateral_amount * slash_percentage
        
        # Execute slash
        actual_slash = bond.slash(slash_amount, claim.condition.value)
        
        # Update claim
        claim.is_resolved = True
        claim.resolution = "executed"
        claim.actual_slash_amount = actual_slash
        claim.resolved_at = datetime.now(timezone.utc).isoformat()
        
        # Update reputation
        reputation = self._reputations.get(claim.watcher_id.did)
        if reputation:
            reputation.metrics.slash_incidents += 1
            reputation.metrics.total_slashed_usd += actual_slash
            reputation.compute_score()
        
        # Check for permanent ban (collusion)
        if claim.condition == SlashingCondition.COLLUSION:
            self._banned.add(claim.watcher_id.did)
        
        return actual_slash
    
    def select_watchers(
        self,
        jurisdiction_id: str,
        min_count: int = 1,
        min_tier: str = "standard",
        min_collateral_usd: Decimal = Decimal("0"),
    ) -> List[WatcherId]:
        """
        Select watchers for a jurisdiction.
        
        Returns watchers sorted by reputation, filtered by requirements.
        """
        tier_order = {"novice": 0, "standard": 1, "trusted": 2, "elite": 3}
        min_tier_rank = tier_order.get(min_tier, 1)
        
        candidates: List[Tuple[float, WatcherId]] = []
        
        for did, watcher in self._watchers.items():
            if did in self._banned:
                continue
            
            bond = self.get_active_bond(did)
            if not bond:
                continue
            
            # Check jurisdiction scope
            if bond.scope_jurisdictions and jurisdiction_id not in bond.scope_jurisdictions:
                continue
            
            # Check collateral
            if bond.available_collateral < min_collateral_usd:
                continue
            
            # Check tier
            reputation = self._reputations.get(did)
            if not reputation:
                continue
            
            tier_rank = tier_order.get(reputation.tier, 0)
            if tier_rank < min_tier_rank:
                continue
            
            candidates.append((reputation.overall_score, watcher))
        
        # Sort by reputation (descending)
        candidates.sort(key=lambda x: x[0], reverse=True)
        
        # Return top watchers (limit to requested count)
        return [w for _, w in candidates[:min(min_count, len(candidates))]]
    
    def record_attestation(
        self,
        watcher_did: str,
        value_usd: Decimal,
        on_time: bool = True,
    ) -> None:
        """Record an attestation by a watcher."""
        bond = self.get_active_bond(watcher_did)
        if bond:
            bond.attestation_count += 1
            bond.attestation_volume_usd += value_usd
            bond.last_updated = datetime.now(timezone.utc).isoformat()
        
        reputation = self._reputations.get(watcher_did)
        if reputation:
            reputation.metrics.required_attestations += 1
            reputation.metrics.delivered_attestations += 1
            if on_time:
                reputation.metrics.on_time_attestations += 1
            reputation.metrics.total_attested_value_usd += value_usd
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get registry statistics."""
        active_watchers = [
            did for did in self._watchers
            if self.get_active_bond(did) is not None
        ]
        
        total_collateral = sum(
            bond.collateral_amount
            for bond in self._bonds.values()
            if bond.is_active
        )
        
        by_tier: Dict[str, int] = {}
        for rep in self._reputations.values():
            by_tier[rep.tier] = by_tier.get(rep.tier, 0) + 1
        
        return {
            "total_watchers": len(self._watchers),
            "active_watchers": len(active_watchers),
            "banned_watchers": len(self._banned),
            "total_bonds": len(self._bonds),
            "total_collateral_usd": str(total_collateral),
            "pending_claims": len([c for c in self._claims.values() if not c.is_resolved]),
            "by_tier": by_tier,
        }
    
    def export_registry(self) -> Dict[str, Any]:
        """Export registry state."""
        return {
            "watchers": {
                did: {
                    "did": watcher.did,
                    "public_key": watcher.public_key_hex,
                    "is_banned": did in self._banned,
                }
                for did, watcher in self._watchers.items()
            },
            "bonds": {
                bid: bond.to_dict()
                for bid, bond in self._bonds.items()
            },
            "reputations": {
                did: rep.to_dict()
                for did, rep in self._reputations.items()
            },
            "statistics": self.get_statistics(),
        }


# =============================================================================
# EQUIVOCATION DETECTION
# =============================================================================

class EquivocationDetector:
    """
    Detects equivocation by watchers.
    
    Equivocation occurs when a watcher signs two different attestations
    for the same state/height.
    """
    
    def __init__(self):
        # Track attestations: (watcher_did, corridor_id, height) -> attestation_digest
        self._attestations: Dict[Tuple[str, str, int], str] = {}
    
    def record_attestation(
        self,
        watcher_did: str,
        corridor_id: str,
        height: int,
        attestation_digest: str,
        attestation_data: Dict[str, Any],
    ) -> Optional[EquivocationEvidence]:
        """
        Record an attestation and check for equivocation.
        
        Returns EquivocationEvidence if equivocation detected.
        """
        key = (watcher_did, corridor_id, height)
        
        if key in self._attestations:
            existing_digest = self._attestations[key]
            if existing_digest != attestation_digest:
                # Equivocation detected!
                return EquivocationEvidence(
                    attestation_1={"digest": existing_digest},
                    attestation_2=attestation_data,
                )
        
        self._attestations[key] = attestation_digest
        return None
    
    def clear_old_attestations(self, older_than_height: int) -> int:
        """Clear attestations below given height."""
        to_remove = [
            key for key in self._attestations
            if key[2] < older_than_height
        ]
        for key in to_remove:
            del self._attestations[key]
        return len(to_remove)
