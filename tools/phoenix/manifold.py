"""
PHOENIX Compliance Manifold

The Compliance Manifold extends the tensor concept to continuous compliance evaluation
across the jurisdictional landscape. When a Smart Asset considers migration between
jurisdictions, the Manifold computes the "compliance distance" — the set of attestations,
verifications, and state transitions required to maintain continuous compliance.

Key Concepts:

    Compliance Graph: Directed graph where nodes are jurisdictions and edges represent
    corridor agreements with compliance requirements.
    
    Compliance Distance: The "cost" of migrating between two jurisdictions, measured
    in required attestations, verification time, and fees.
    
    Migration Path: A sequence of jurisdictions and corridors that maintains compliance
    throughout the journey.
    
    Attestation Gap: The set of missing attestations blocking a migration.

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import heapq
import json
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum
from typing import (
    Any,
    Callable,
    Dict,
    FrozenSet,
    List,
    Optional,
    Set,
    Tuple,
)

from tools.phoenix.tensor import (
    ComplianceDomain,
    ComplianceState,
    ComplianceTensorV2,
    AttestationRef,
)


# =============================================================================
# ATTESTATION REQUIREMENTS
# =============================================================================

class AttestationType(Enum):
    """Types of attestations that may be required for migration."""
    KYC_VERIFICATION = "kyc_verification"
    KYB_VERIFICATION = "kyb_verification"
    AML_SCREENING = "aml_screening"
    SANCTIONS_CHECK = "sanctions_check"
    SOURCE_OF_FUNDS = "source_of_funds"
    TAX_RESIDENCY = "tax_residency"
    ACCREDITED_INVESTOR = "accredited_investor"
    QUALIFIED_PURCHASER = "qualified_purchaser"
    PROFESSIONAL_INVESTOR = "professional_investor"
    CUSTODY_VERIFICATION = "custody_verification"
    INSURANCE_COVERAGE = "insurance_coverage"
    REGULATORY_LICENSE = "regulatory_license"


@dataclass
class AttestationRequirement:
    """
    A specific attestation required for compliance.
    
    Requirements specify what attestation is needed, from whom,
    and with what validity period.
    """
    attestation_type: AttestationType
    domain: ComplianceDomain
    
    # Issuer constraints
    approved_issuers: FrozenSet[str] = field(default_factory=frozenset)  # DIDs
    issuer_jurisdiction: Optional[str] = None
    min_issuer_tier: int = 1
    
    # Validity constraints
    max_age_days: int = 365
    must_be_current: bool = True
    
    # Cost estimates
    estimated_cost_usd: Decimal = Decimal("0")
    estimated_time_hours: int = 24
    
    # Priority
    is_mandatory: bool = True
    can_be_waived: bool = False
    waiver_conditions: Optional[str] = None
    
    def is_satisfied_by(self, attestation: AttestationRef, as_of: Optional[datetime] = None) -> bool:
        """Check if an attestation satisfies this requirement."""
        as_of = as_of or datetime.now(timezone.utc)
        
        # Type match
        if attestation.attestation_type != self.attestation_type.value:
            return False
        
        # Issuer check
        if self.approved_issuers and attestation.issuer_did not in self.approved_issuers:
            return False
        
        # Expiry check
        if attestation.is_expired(as_of):
            return False
        
        # Age check
        from tools.phoenix.hardening import parse_iso_timestamp
        issued = parse_iso_timestamp(attestation.issued_at)
        age = as_of - issued
        if age.days > self.max_age_days:
            return False
        
        return True
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "attestation_type": self.attestation_type.value,
            "domain": self.domain.value,
            "approved_issuers": list(self.approved_issuers),
            "issuer_jurisdiction": self.issuer_jurisdiction,
            "min_issuer_tier": self.min_issuer_tier,
            "max_age_days": self.max_age_days,
            "must_be_current": self.must_be_current,
            "estimated_cost_usd": str(self.estimated_cost_usd),
            "estimated_time_hours": self.estimated_time_hours,
            "is_mandatory": self.is_mandatory,
        }


# =============================================================================
# PATH CONSTRAINTS
# =============================================================================

@dataclass
class PathConstraint:
    """
    Constraints on migration paths.
    
    Constraints limit which paths are acceptable based on
    cost, time, jurisdictional preferences, and compliance requirements.
    """
    # Cost limits
    max_total_cost_usd: Optional[Decimal] = None
    max_per_hop_cost_usd: Optional[Decimal] = None
    
    # Time limits
    max_total_time_hours: Optional[int] = None
    max_per_hop_time_hours: Optional[int] = None
    deadline: Optional[datetime] = None
    
    # Jurisdictional preferences
    required_jurisdictions: FrozenSet[str] = field(default_factory=frozenset)
    excluded_jurisdictions: FrozenSet[str] = field(default_factory=frozenset)
    preferred_jurisdictions: FrozenSet[str] = field(default_factory=frozenset)
    
    # Compliance requirements
    required_domains: FrozenSet[ComplianceDomain] = field(
        default_factory=lambda: frozenset(ComplianceDomain)
    )
    min_compliance_state: ComplianceState = ComplianceState.COMPLIANT
    
    # Path structure
    max_hops: int = 5
    allow_loops: bool = False
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "max_total_cost_usd": str(self.max_total_cost_usd) if self.max_total_cost_usd else None,
            "max_total_time_hours": self.max_total_time_hours,
            "deadline": self.deadline.isoformat() if self.deadline else None,
            "required_jurisdictions": list(self.required_jurisdictions),
            "excluded_jurisdictions": list(self.excluded_jurisdictions),
            "max_hops": self.max_hops,
        }


# =============================================================================
# JURISDICTION AND CORRIDOR
# =============================================================================

@dataclass
class JurisdictionNode:
    """
    A jurisdiction in the compliance graph.
    
    Each jurisdiction has specific compliance requirements for
    assets to be held or operated within it.
    """
    jurisdiction_id: str
    name: str
    country_code: str
    
    # Regulatory framework
    supported_asset_classes: FrozenSet[str] = field(default_factory=frozenset)
    regulatory_framework: str = ""
    
    # Compliance requirements for entry
    entry_requirements: List[AttestationRequirement] = field(default_factory=list)
    
    # Compliance requirements for ongoing operation
    ongoing_requirements: List[AttestationRequirement] = field(default_factory=list)
    
    # Capabilities
    supports_custody: bool = True
    supports_trading: bool = True
    supports_settlement: bool = True
    
    # Costs
    entry_fee_usd: Decimal = Decimal("0")
    annual_fee_usd: Decimal = Decimal("0")
    
    # Status
    is_active: bool = True
    activation_date: Optional[str] = None
    
    def total_entry_cost(self) -> Decimal:
        """Calculate total cost to enter jurisdiction."""
        attestation_costs = sum(
            r.estimated_cost_usd for r in self.entry_requirements
        )
        return self.entry_fee_usd + attestation_costs
    
    def total_entry_time_hours(self) -> int:
        """Calculate total time to enter jurisdiction."""
        # Requirements can be parallelized, take max
        if not self.entry_requirements:
            return 0
        return max(r.estimated_time_hours for r in self.entry_requirements)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "jurisdiction_id": self.jurisdiction_id,
            "name": self.name,
            "country_code": self.country_code,
            "supported_asset_classes": list(self.supported_asset_classes),
            "entry_requirements": [r.to_dict() for r in self.entry_requirements],
            "entry_fee_usd": str(self.entry_fee_usd),
            "is_active": self.is_active,
        }


@dataclass
class CorridorEdge:
    """
    A corridor connecting two jurisdictions.
    
    Corridors enable asset migration between jurisdictions and
    specify the compliance requirements for the transfer.
    """
    corridor_id: str
    source_jurisdiction: str
    target_jurisdiction: str
    
    # Corridor status
    is_active: bool = True
    is_bidirectional: bool = True
    
    # Transfer requirements
    transfer_requirements: List[AttestationRequirement] = field(default_factory=list)
    
    # Supported operations
    supported_asset_classes: FrozenSet[str] = field(default_factory=frozenset)
    max_transfer_value_usd: Optional[Decimal] = None
    
    # Costs
    transfer_fee_bps: int = 0  # Basis points
    flat_fee_usd: Decimal = Decimal("0")
    
    # Timing
    estimated_transfer_hours: int = 24
    settlement_finality_hours: int = 48
    
    # Compliance
    requires_continuous_compliance: bool = True
    compliance_check_interval_hours: int = 24
    
    def transfer_cost(self, value_usd: Decimal) -> Decimal:
        """Calculate transfer cost for given value."""
        bps_cost = value_usd * Decimal(self.transfer_fee_bps) / Decimal("10000")
        return self.flat_fee_usd + bps_cost
    
    def total_attestation_cost(self) -> Decimal:
        return sum(r.estimated_cost_usd for r in self.transfer_requirements)
    
    def total_time_hours(self) -> int:
        return self.estimated_transfer_hours + self.settlement_finality_hours
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "corridor_id": self.corridor_id,
            "source_jurisdiction": self.source_jurisdiction,
            "target_jurisdiction": self.target_jurisdiction,
            "is_active": self.is_active,
            "is_bidirectional": self.is_bidirectional,
            "transfer_requirements": [r.to_dict() for r in self.transfer_requirements],
            "transfer_fee_bps": self.transfer_fee_bps,
            "flat_fee_usd": str(self.flat_fee_usd),
            "estimated_transfer_hours": self.estimated_transfer_hours,
        }


# =============================================================================
# MIGRATION PATH
# =============================================================================

@dataclass
class MigrationHop:
    """A single hop in a migration path."""
    corridor: CorridorEdge
    source: JurisdictionNode
    target: JurisdictionNode
    required_attestations: List[AttestationRequirement]
    
    # Computed costs
    cost_usd: Decimal = Decimal("0")
    time_hours: int = 0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "corridor_id": self.corridor.corridor_id,
            "source": self.source.jurisdiction_id,
            "target": self.target.jurisdiction_id,
            "required_attestations": [r.to_dict() for r in self.required_attestations],
            "cost_usd": str(self.cost_usd),
            "time_hours": self.time_hours,
        }


@dataclass
class MigrationPath:
    """
    A complete migration path from source to target jurisdiction.
    
    The path includes all intermediate hops, attestation requirements,
    and cost/time estimates.
    """
    source_jurisdiction: str
    target_jurisdiction: str
    hops: List[MigrationHop]
    
    # Aggregated metrics
    total_cost_usd: Decimal = Decimal("0")
    total_time_hours: int = 0
    
    # All required attestations (deduplicated)
    all_requirements: List[AttestationRequirement] = field(default_factory=list)
    
    # Path metadata
    computed_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    path_id: str = ""
    
    def __post_init__(self):
        if not self.path_id:
            # Generate deterministic path ID
            content = f"{self.source_jurisdiction}:{self.target_jurisdiction}:" + \
                      ":".join(h.corridor.corridor_id for h in self.hops)
            self.path_id = hashlib.sha256(content.encode()).hexdigest()[:16]
    
    @property
    def hop_count(self) -> int:
        return len(self.hops)
    
    @property
    def jurisdictions(self) -> List[str]:
        """List all jurisdictions in path order."""
        if not self.hops:
            return [self.source_jurisdiction, self.target_jurisdiction]
        
        result = [self.hops[0].source.jurisdiction_id]
        for hop in self.hops:
            result.append(hop.target.jurisdiction_id)
        return result
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "path_id": self.path_id,
            "source_jurisdiction": self.source_jurisdiction,
            "target_jurisdiction": self.target_jurisdiction,
            "hops": [h.to_dict() for h in self.hops],
            "total_cost_usd": str(self.total_cost_usd),
            "total_time_hours": self.total_time_hours,
            "hop_count": self.hop_count,
            "jurisdictions": self.jurisdictions,
            "all_requirements": [r.to_dict() for r in self.all_requirements],
            "computed_at": self.computed_at,
        }


# =============================================================================
# ATTESTATION GAP ANALYSIS
# =============================================================================

@dataclass
class AttestationGap:
    """
    Analysis of attestations needed vs. available for a migration.
    """
    required: List[AttestationRequirement]
    available: List[AttestationRef]
    missing: List[AttestationRequirement]
    expired: List[Tuple[AttestationRequirement, AttestationRef]]
    
    @property
    def is_satisfied(self) -> bool:
        return len(self.missing) == 0 and len(self.expired) == 0
    
    @property
    def missing_count(self) -> int:
        return len(self.missing) + len(self.expired)
    
    def estimated_resolution_cost(self) -> Decimal:
        """Estimate cost to resolve all gaps."""
        missing_cost = sum(r.estimated_cost_usd for r in self.missing)
        # Renewals typically cost less
        renewal_cost = sum(
            r.estimated_cost_usd * Decimal("0.5")
            for r, _ in self.expired
        )
        return missing_cost + renewal_cost
    
    def estimated_resolution_time_hours(self) -> int:
        """Estimate time to resolve all gaps."""
        if not self.missing and not self.expired:
            return 0
        
        # Can parallelize, take max
        times = [r.estimated_time_hours for r in self.missing]
        times.extend(r.estimated_time_hours // 2 for r, _ in self.expired)
        return max(times) if times else 0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "is_satisfied": self.is_satisfied,
            "missing_count": self.missing_count,
            "required_count": len(self.required),
            "available_count": len(self.available),
            "missing": [r.to_dict() for r in self.missing],
            "expired": [
                {"requirement": r.to_dict(), "attestation": a.to_dict()}
                for r, a in self.expired
            ],
            "estimated_resolution_cost_usd": str(self.estimated_resolution_cost()),
            "estimated_resolution_time_hours": self.estimated_resolution_time_hours(),
        }


# =============================================================================
# COMPLIANCE MANIFOLD
# =============================================================================

class ComplianceManifold:
    """
    The Compliance Manifold - path planning through jurisdictional landscape.
    
    The manifold computes optimal migration paths between jurisdictions
    while satisfying compliance constraints. It uses Dijkstra's algorithm
    with compliance-aware edge weights.
    
    Example:
        manifold = ComplianceManifold()
        manifold.add_jurisdiction(uae_difc)
        manifold.add_jurisdiction(kz_aifc)
        manifold.add_corridor(difc_aifc_corridor)
        
        path = manifold.find_path(
            source="uae-difc",
            target="kz-aifc",
            constraints=PathConstraint(max_total_cost_usd=Decimal("10000")),
        )
    """
    
    def __init__(self):
        self._jurisdictions: Dict[str, JurisdictionNode] = {}
        self._corridors: Dict[str, CorridorEdge] = {}
        self._adjacency: Dict[str, List[str]] = {}  # jurisdiction -> [corridor_ids]
    
    def add_jurisdiction(self, jurisdiction: JurisdictionNode) -> None:
        """Add a jurisdiction to the manifold."""
        self._jurisdictions[jurisdiction.jurisdiction_id] = jurisdiction
        if jurisdiction.jurisdiction_id not in self._adjacency:
            self._adjacency[jurisdiction.jurisdiction_id] = []
    
    def add_corridor(self, corridor: CorridorEdge) -> None:
        """Add a corridor to the manifold."""
        self._corridors[corridor.corridor_id] = corridor
        
        # Update adjacency
        if corridor.source_jurisdiction not in self._adjacency:
            self._adjacency[corridor.source_jurisdiction] = []
        self._adjacency[corridor.source_jurisdiction].append(corridor.corridor_id)
        
        # Handle bidirectional
        if corridor.is_bidirectional:
            if corridor.target_jurisdiction not in self._adjacency:
                self._adjacency[corridor.target_jurisdiction] = []
            self._adjacency[corridor.target_jurisdiction].append(corridor.corridor_id)
    
    def get_jurisdiction(self, jurisdiction_id: str) -> Optional[JurisdictionNode]:
        return self._jurisdictions.get(jurisdiction_id)
    
    def get_corridor(self, corridor_id: str) -> Optional[CorridorEdge]:
        return self._corridors.get(corridor_id)
    
    def list_jurisdictions(self) -> List[JurisdictionNode]:
        return list(self._jurisdictions.values())
    
    def list_corridors(self) -> List[CorridorEdge]:
        return list(self._corridors.values())
    
    def find_path(
        self,
        source: str,
        target: str,
        constraints: Optional[PathConstraint] = None,
        asset_value_usd: Decimal = Decimal("0"),
        existing_attestations: Optional[List[AttestationRef]] = None,
    ) -> Optional[MigrationPath]:
        """
        Find the optimal migration path from source to target.
        
        Uses Dijkstra's algorithm with compliance-aware weights.
        
        Args:
            source: Source jurisdiction ID
            target: Target jurisdiction ID
            constraints: Optional path constraints
            asset_value_usd: Asset value for fee calculation
            existing_attestations: Attestations the asset already has
            
        Returns:
            The optimal MigrationPath, or None if no path exists
        """
        if source not in self._jurisdictions or target not in self._jurisdictions:
            return None
        
        constraints = constraints or PathConstraint()
        existing_attestations = existing_attestations or []
        
        # Check excluded jurisdictions
        if source in constraints.excluded_jurisdictions:
            return None
        if target in constraints.excluded_jurisdictions:
            return None
        
        # Dijkstra's algorithm
        distances: Dict[str, Decimal] = {j: Decimal("Infinity") for j in self._jurisdictions}
        distances[source] = Decimal("0")
        
        times: Dict[str, int] = {j: 999999 for j in self._jurisdictions}
        times[source] = 0
        
        previous: Dict[str, Optional[Tuple[str, str]]] = {j: None for j in self._jurisdictions}
        
        # Priority queue: (distance, time, jurisdiction_id)
        pq: List[Tuple[Decimal, int, str]] = [(Decimal("0"), 0, source)]
        visited: Set[str] = set()
        
        while pq:
            current_dist, current_time, current = heapq.heappop(pq)
            
            if current in visited:
                continue
            visited.add(current)
            
            if current == target:
                break
            
            # Check hop limit
            hop_count = self._count_hops(previous, current)
            if hop_count >= constraints.max_hops:
                continue
            
            # Explore neighbors
            for corridor_id in self._adjacency.get(current, []):
                corridor = self._corridors[corridor_id]
                
                # Determine target of this edge
                if corridor.source_jurisdiction == current:
                    neighbor = corridor.target_jurisdiction
                elif corridor.is_bidirectional and corridor.target_jurisdiction == current:
                    neighbor = corridor.source_jurisdiction
                else:
                    continue
                
                # Skip excluded
                if neighbor in constraints.excluded_jurisdictions:
                    continue
                
                # Skip inactive
                if not corridor.is_active:
                    continue
                if not self._jurisdictions[neighbor].is_active:
                    continue
                
                # Skip if no loops and already visited
                if not constraints.allow_loops and neighbor in visited:
                    continue
                
                # Calculate edge cost
                edge_cost = self._calculate_edge_cost(
                    corridor, asset_value_usd, existing_attestations
                )
                edge_time = corridor.total_time_hours()
                
                # Check per-hop constraints
                if constraints.max_per_hop_cost_usd:
                    if edge_cost > constraints.max_per_hop_cost_usd:
                        continue
                if constraints.max_per_hop_time_hours:
                    if edge_time > constraints.max_per_hop_time_hours:
                        continue
                
                new_dist = current_dist + edge_cost
                new_time = current_time + edge_time
                
                # Check total constraints
                if constraints.max_total_cost_usd:
                    if new_dist > constraints.max_total_cost_usd:
                        continue
                if constraints.max_total_time_hours:
                    if new_time > constraints.max_total_time_hours:
                        continue
                
                if new_dist < distances[neighbor]:
                    distances[neighbor] = new_dist
                    times[neighbor] = new_time
                    previous[neighbor] = (current, corridor_id)
                    heapq.heappush(pq, (new_dist, new_time, neighbor))
        
        # Reconstruct path
        if distances[target] == Decimal("Infinity"):
            return None
        
        return self._reconstruct_path(
            source, target, previous, asset_value_usd, existing_attestations
        )
    
    def _count_hops(
        self,
        previous: Dict[str, Optional[Tuple[str, str]]],
        current: str,
    ) -> int:
        """Count hops from source to current."""
        count = 0
        while previous.get(current):
            prev, _ = previous[current]
            current = prev
            count += 1
        return count
    
    def _calculate_edge_cost(
        self,
        corridor: CorridorEdge,
        asset_value_usd: Decimal,
        existing_attestations: List[AttestationRef],
    ) -> Decimal:
        """Calculate total cost for traversing a corridor edge."""
        # Transfer fee
        transfer_cost = corridor.transfer_cost(asset_value_usd)
        
        # Attestation costs (only for missing attestations)
        attestation_cost = Decimal("0")
        for req in corridor.transfer_requirements:
            if not any(req.is_satisfied_by(a) for a in existing_attestations):
                attestation_cost += req.estimated_cost_usd
        
        # Target jurisdiction entry cost
        target = self._jurisdictions.get(corridor.target_jurisdiction)
        entry_cost = target.entry_fee_usd if target else Decimal("0")
        
        return transfer_cost + attestation_cost + entry_cost
    
    def _reconstruct_path(
        self,
        source: str,
        target: str,
        previous: Dict[str, Optional[Tuple[str, str]]],
        asset_value_usd: Decimal,
        existing_attestations: List[AttestationRef],
    ) -> MigrationPath:
        """Reconstruct the path from source to target."""
        hops: List[MigrationHop] = []
        all_requirements: List[AttestationRequirement] = []
        seen_requirements: Set[str] = set()
        
        total_cost = Decimal("0")
        total_time = 0
        
        current = target
        path_segments: List[Tuple[str, str]] = []
        
        while previous.get(current):
            prev, corridor_id = previous[current]
            path_segments.append((prev, corridor_id))
            current = prev
        
        path_segments.reverse()
        
        for prev_jurisdiction, corridor_id in path_segments:
            corridor = self._corridors[corridor_id]
            source_node = self._jurisdictions[prev_jurisdiction]
            
            # Determine target of this hop
            if corridor.source_jurisdiction == prev_jurisdiction:
                target_id = corridor.target_jurisdiction
            else:
                target_id = corridor.source_jurisdiction
            
            target_node = self._jurisdictions[target_id]
            
            # Collect requirements
            hop_requirements = list(corridor.transfer_requirements)
            hop_requirements.extend(target_node.entry_requirements)
            
            for req in hop_requirements:
                req_key = f"{req.attestation_type.value}:{req.domain.value}"
                if req_key not in seen_requirements:
                    seen_requirements.add(req_key)
                    all_requirements.append(req)
            
            # Calculate hop cost
            hop_cost = corridor.transfer_cost(asset_value_usd)
            hop_cost += corridor.total_attestation_cost()
            hop_cost += target_node.entry_fee_usd
            
            hop_time = corridor.total_time_hours()
            
            hop = MigrationHop(
                corridor=corridor,
                source=source_node,
                target=target_node,
                required_attestations=hop_requirements,
                cost_usd=hop_cost,
                time_hours=hop_time,
            )
            
            hops.append(hop)
            total_cost += hop_cost
            total_time += hop_time
        
        return MigrationPath(
            source_jurisdiction=source,
            target_jurisdiction=target,
            hops=hops,
            total_cost_usd=total_cost,
            total_time_hours=total_time,
            all_requirements=all_requirements,
        )
    
    def analyze_attestation_gap(
        self,
        path: MigrationPath,
        available_attestations: List[AttestationRef],
        as_of: Optional[datetime] = None,
    ) -> AttestationGap:
        """
        Analyze the gap between required and available attestations.
        
        Args:
            path: The migration path to analyze
            available_attestations: Attestations the asset has
            as_of: Time to check attestation validity against
            
        Returns:
            AttestationGap analysis
        """
        as_of = as_of or datetime.now(timezone.utc)
        
        required = path.all_requirements
        missing: List[AttestationRequirement] = []
        expired: List[Tuple[AttestationRequirement, AttestationRef]] = []
        
        for req in required:
            satisfying = None
            for att in available_attestations:
                if req.is_satisfied_by(att, as_of):
                    satisfying = att
                    break
            
            if satisfying is None:
                # Check if we have an expired version
                for att in available_attestations:
                    if att.attestation_type == req.attestation_type.value:
                        if att.is_expired(as_of):
                            expired.append((req, att))
                            break
                else:
                    missing.append(req)
        
        return AttestationGap(
            required=required,
            available=available_attestations,
            missing=missing,
            expired=expired,
        )
    
    def find_all_paths(
        self,
        source: str,
        target: str,
        constraints: Optional[PathConstraint] = None,
        max_paths: int = 5,
    ) -> List[MigrationPath]:
        """
        Find multiple alternative paths.
        
        Uses k-shortest paths algorithm to find alternatives.
        """
        paths: List[MigrationPath] = []
        constraints = constraints or PathConstraint()
        
        # Find primary path
        primary = self.find_path(source, target, constraints)
        if primary:
            paths.append(primary)
        
        # Find alternatives by excluding corridors from previous paths
        excluded_corridors: Set[str] = set()
        
        for _ in range(max_paths - 1):
            if not paths:
                break
            
            # Exclude corridors from last found path
            last_path = paths[-1]
            for hop in last_path.hops:
                excluded_corridors.add(hop.corridor.corridor_id)
            
            # Try to find alternative
            alt = self._find_path_excluding(
                source, target, constraints, excluded_corridors
            )
            if alt and alt.path_id not in {p.path_id for p in paths}:
                paths.append(alt)
        
        # Sort by cost
        paths.sort(key=lambda p: p.total_cost_usd)
        return paths[:max_paths]
    
    def _find_path_excluding(
        self,
        source: str,
        target: str,
        constraints: PathConstraint,
        excluded_corridors: Set[str],
    ) -> Optional[MigrationPath]:
        """Find path excluding certain corridors."""
        # Temporarily remove excluded corridors
        removed = {}
        for cid in excluded_corridors:
            if cid in self._corridors:
                removed[cid] = self._corridors[cid]
                del self._corridors[cid]
        
        # Rebuild adjacency
        old_adjacency = self._adjacency.copy()
        for jid in self._adjacency:
            self._adjacency[jid] = [
                cid for cid in self._adjacency[jid]
                if cid not in excluded_corridors
            ]
        
        try:
            return self.find_path(source, target, constraints)
        finally:
            # Restore
            self._corridors.update(removed)
            self._adjacency = old_adjacency
    
    def compliance_distance(
        self,
        source: str,
        target: str,
        constraints: Optional[PathConstraint] = None,
    ) -> Optional[Dict[str, Any]]:
        """
        Compute the compliance distance between two jurisdictions.
        
        Returns a summary of the minimum requirements to migrate.
        """
        path = self.find_path(source, target, constraints)
        if not path:
            return None
        
        return {
            "source": source,
            "target": target,
            "hop_count": path.hop_count,
            "total_cost_usd": str(path.total_cost_usd),
            "total_time_hours": path.total_time_hours,
            "attestation_count": len(path.all_requirements),
            "path_id": path.path_id,
        }
    
    def export_graph(self) -> Dict[str, Any]:
        """Export the compliance graph for visualization."""
        return {
            "jurisdictions": [j.to_dict() for j in self._jurisdictions.values()],
            "corridors": [c.to_dict() for c in self._corridors.values()],
            "adjacency": {k: list(v) for k, v in self._adjacency.items()},
        }


# =============================================================================
# STANDARD JURISDICTIONS AND CORRIDORS
# =============================================================================

def create_uae_difc_jurisdiction() -> JurisdictionNode:
    """Create UAE-DIFC jurisdiction node."""
    return JurisdictionNode(
        jurisdiction_id="uae-difc",
        name="Dubai International Financial Centre",
        country_code="AE",
        supported_asset_classes=frozenset({"securities", "commodities", "digital_assets"}),
        regulatory_framework="DFSA",
        entry_requirements=[
            AttestationRequirement(
                attestation_type=AttestationType.KYC_VERIFICATION,
                domain=ComplianceDomain.KYC,
                max_age_days=365,
                estimated_cost_usd=Decimal("500"),
                estimated_time_hours=24,
            ),
            AttestationRequirement(
                attestation_type=AttestationType.AML_SCREENING,
                domain=ComplianceDomain.AML,
                max_age_days=30,
                estimated_cost_usd=Decimal("100"),
                estimated_time_hours=2,
            ),
        ],
        entry_fee_usd=Decimal("1000"),
        annual_fee_usd=Decimal("5000"),
    )


def create_kz_aifc_jurisdiction() -> JurisdictionNode:
    """Create Kazakhstan-AIFC jurisdiction node."""
    return JurisdictionNode(
        jurisdiction_id="kz-aifc",
        name="Astana International Financial Centre",
        country_code="KZ",
        supported_asset_classes=frozenset({"securities", "digital_assets", "islamic_finance"}),
        regulatory_framework="AFSA",
        entry_requirements=[
            AttestationRequirement(
                attestation_type=AttestationType.KYC_VERIFICATION,
                domain=ComplianceDomain.KYC,
                max_age_days=365,
                estimated_cost_usd=Decimal("300"),
                estimated_time_hours=48,
            ),
            AttestationRequirement(
                attestation_type=AttestationType.SANCTIONS_CHECK,
                domain=ComplianceDomain.SANCTIONS,
                max_age_days=7,
                estimated_cost_usd=Decimal("50"),
                estimated_time_hours=1,
            ),
        ],
        entry_fee_usd=Decimal("500"),
        annual_fee_usd=Decimal("2000"),
    )


def create_difc_aifc_corridor() -> CorridorEdge:
    """Create corridor between DIFC and AIFC."""
    return CorridorEdge(
        corridor_id="corridor-difc-aifc",
        source_jurisdiction="uae-difc",
        target_jurisdiction="kz-aifc",
        is_bidirectional=True,
        transfer_requirements=[
            AttestationRequirement(
                attestation_type=AttestationType.SOURCE_OF_FUNDS,
                domain=ComplianceDomain.AML,
                max_age_days=90,
                estimated_cost_usd=Decimal("200"),
                estimated_time_hours=24,
            ),
        ],
        transfer_fee_bps=10,  # 0.1%
        flat_fee_usd=Decimal("100"),
        estimated_transfer_hours=24,
        settlement_finality_hours=48,
    )


def create_standard_manifold() -> ComplianceManifold:
    """Create a manifold with standard jurisdictions and corridors."""
    manifold = ComplianceManifold()
    
    manifold.add_jurisdiction(create_uae_difc_jurisdiction())
    manifold.add_jurisdiction(create_kz_aifc_jurisdiction())
    manifold.add_corridor(create_difc_aifc_corridor())
    
    return manifold
