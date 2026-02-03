"""
PHOENIX Compliance Tensor

The Compliance Tensor is the mathematical core of Smart Asset autonomy. It represents
the multi-dimensional compliance state of an asset across all bound jurisdictions as
a single, cryptographically committable object.

Mathematical Definition:

    C: AssetID × JurisdictionID × ComplianceDomain × TimeQuantum → ComplianceState

Where:
    - AssetID: SHA256 of genesis document (canonical identity)
    - JurisdictionID: Harbor identifier from jurisdictional binding
    - ComplianceDomain: {AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY}
    - TimeQuantum: Discrete time bucket (block height or timestamp modulo period)
    - ComplianceState: {COMPLIANT, NON_COMPLIANT, PENDING, UNKNOWN, EXEMPT, EXPIRED}

The tensor supports:
    - Incremental updates as attestations arrive
    - Slicing along any dimension
    - Cryptographic commitment generation
    - Zero-knowledge selective disclosure proofs
    - Composition for multi-asset portfolios

Copyright (c) 2026 Momentum. All rights reserved.
"""

from __future__ import annotations

import hashlib
import json
from dataclasses import dataclass, field
from datetime import datetime, timezone
from decimal import Decimal
from enum import Enum, auto
from typing import (
    Any,
    Callable,
    Dict,
    FrozenSet,
    Iterator,
    List,
    Optional,
    Set,
    Tuple,
    Union,
)


# =============================================================================
# COMPLIANCE DOMAINS
# =============================================================================

class ComplianceDomain(Enum):
    """
    Compliance domains represent distinct regulatory categories that may have
    independent requirements across jurisdictions.
    
    Each domain corresponds to a specific regulatory framework:
        AML: Anti-Money Laundering (FATF, local AML acts)
        KYC: Know Your Customer (identity verification requirements)
        SANCTIONS: Sanctions screening (OFAC, EU, UN lists)
        TAX: Tax compliance (withholding, reporting, residency)
        SECURITIES: Securities regulations (accreditation, disclosure)
        CORPORATE: Corporate governance (director duties, filings)
        CUSTODY: Custody requirements (segregation, insurance)
        DATA_PRIVACY: Data protection (GDPR, local privacy laws)
    """
    AML = "aml"
    KYC = "kyc"
    SANCTIONS = "sanctions"
    TAX = "tax"
    SECURITIES = "securities"
    CORPORATE = "corporate"
    CUSTODY = "custody"
    DATA_PRIVACY = "data_privacy"
    
    @classmethod
    def all_domains(cls) -> FrozenSet['ComplianceDomain']:
        """Return all compliance domains."""
        return frozenset(cls)


class ComplianceState(Enum):
    """
    Compliance states follow a strict lattice for composition:
    
        COMPLIANT ⊔ COMPLIANT = COMPLIANT
        COMPLIANT ⊔ PENDING = PENDING
        COMPLIANT ⊔ UNKNOWN = UNKNOWN
        * ⊔ NON_COMPLIANT = NON_COMPLIANT (absorbing)
        
    EXPIRED is a temporal state that transitions to NON_COMPLIANT
    after grace period.
    
    EXEMPT indicates regulatory exemption (e.g., de minimis threshold).
    """
    COMPLIANT = "compliant"
    NON_COMPLIANT = "non_compliant"
    PENDING = "pending"
    UNKNOWN = "unknown"
    EXEMPT = "exempt"
    EXPIRED = "expired"
    
    def __lt__(self, other: 'ComplianceState') -> bool:
        """Lattice ordering: NON_COMPLIANT < EXPIRED < UNKNOWN < PENDING < EXEMPT < COMPLIANT"""
        order = {
            ComplianceState.NON_COMPLIANT: 0,
            ComplianceState.EXPIRED: 1,
            ComplianceState.UNKNOWN: 2,
            ComplianceState.PENDING: 3,
            ComplianceState.EXEMPT: 4,
            ComplianceState.COMPLIANT: 5,
        }
        return order[self] < order[other]
    
    def meet(self, other: 'ComplianceState') -> 'ComplianceState':
        """Lattice meet (greatest lower bound) - pessimistic composition."""
        return self if self < other else other
    
    def join(self, other: 'ComplianceState') -> 'ComplianceState':
        """Lattice join (least upper bound) - optimistic composition."""
        return other if self < other else self
    
    def is_terminal(self) -> bool:
        """Check if this is a terminal (non-transitional) state."""
        return self in {ComplianceState.COMPLIANT, ComplianceState.NON_COMPLIANT, ComplianceState.EXEMPT}


# =============================================================================
# ATTESTATION REFERENCE
# =============================================================================

@dataclass(frozen=True)
class AttestationRef:
    """
    Reference to an attestation that justifies a compliance state.
    
    Attestations are the evidentiary basis for compliance determinations.
    Each tensor cell links to the attestation(s) that established its state.
    """
    attestation_id: str  # Unique identifier (typically VC id)
    attestation_type: str  # Type of attestation (e.g., "kyc_verification")
    issuer_did: str  # DID of attestation issuer
    issued_at: str  # ISO8601 timestamp
    expires_at: Optional[str] = None  # Optional expiry
    digest: str = ""  # SHA256 of attestation content
    
    def is_expired(self, as_of: Optional[datetime] = None) -> bool:
        """Check if attestation has expired."""
        if not self.expires_at:
            return False
        as_of = as_of or datetime.now(timezone.utc)
        from tools.phoenix.hardening import parse_iso_timestamp
        expiry = parse_iso_timestamp(self.expires_at)
        return as_of > expiry
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "attestation_id": self.attestation_id,
            "attestation_type": self.attestation_type,
            "issuer_did": self.issuer_did,
            "issued_at": self.issued_at,
            "expires_at": self.expires_at,
            "digest": self.digest,
        }


# =============================================================================
# TENSOR CELL
# =============================================================================

@dataclass
class TensorCell:
    """
    A single cell in the compliance tensor, containing:
    - The compliance state
    - References to supporting attestations
    - Metadata about the determination
    """
    state: ComplianceState
    attestations: List[AttestationRef] = field(default_factory=list)
    determined_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    reason_code: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "state": self.state.value,
            "attestations": [a.to_dict() for a in self.attestations],
            "determined_at": self.determined_at,
            "reason_code": self.reason_code,
            "metadata": self.metadata,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'TensorCell':
        return cls(
            state=ComplianceState(data["state"]),
            attestations=[
                AttestationRef(**a) for a in data.get("attestations", [])
            ],
            determined_at=data.get("determined_at", ""),
            reason_code=data.get("reason_code"),
            metadata=data.get("metadata", {}),
        )
    
    def is_stale(self, as_of: Optional[datetime] = None) -> bool:
        """Check if any supporting attestation has expired."""
        return any(a.is_expired(as_of) for a in self.attestations)


# =============================================================================
# TENSOR COORDINATES
# =============================================================================

@dataclass(frozen=True)
class TensorCoord:
    """
    Coordinates in the 4-dimensional compliance tensor.

    Immutable to enable use as dictionary key.
    """
    asset_id: str
    jurisdiction_id: str
    domain: ComplianceDomain
    time_quantum: int  # Discrete time bucket

    def to_tuple(self) -> Tuple[str, str, str, int]:
        return (self.asset_id, self.jurisdiction_id, self.domain.value, self.time_quantum)

    def to_dict(self) -> Dict[str, Any]:
        """Serialize to dictionary for storage/transmission."""
        return {
            "asset_id": self.asset_id,
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain.value,
            "time_quantum": self.time_quantum,
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'TensorCoord':
        """Deserialize from dictionary."""
        return cls(
            asset_id=data["asset_id"],
            jurisdiction_id=data["jurisdiction_id"],
            domain=ComplianceDomain(data["domain"]),
            time_quantum=data["time_quantum"],
        )

    def to_key_string(self) -> str:
        """
        Generate a deterministic key string for use as dictionary key.

        Uses base64-encoded hash of full asset_id to ensure uniqueness
        while keeping the key readable.
        """
        asset_hash = hashlib.sha256(self.asset_id.encode()).hexdigest()[:12]
        return f"{asset_hash}:{self.jurisdiction_id}:{self.domain.value}:t{self.time_quantum}"

    def __str__(self) -> str:
        """Human-readable representation (may truncate for display)."""
        # Truncate only for display, use to_dict for serialization
        display_asset = self.asset_id[:16] + "..." if len(self.asset_id) > 16 else self.asset_id
        return f"{display_asset}:{self.jurisdiction_id}:{self.domain.value}:t{self.time_quantum}"


# =============================================================================
# TENSOR SLICE
# =============================================================================

@dataclass
class TensorSlice:
    """
    A slice of the compliance tensor along one or more dimensions.
    
    Slices preserve provenance - a sliced tensor can regenerate proofs
    for the cells it contains.
    """
    cells: Dict[TensorCoord, TensorCell]
    slice_dims: Dict[str, Any]  # Which dimensions were fixed
    parent_commitment: Optional[str] = None  # Commitment of parent tensor
    
    def aggregate_state(self) -> ComplianceState:
        """
        Aggregate compliance state across the slice using lattice meet.
        
        Returns the most pessimistic (lowest) state in the slice.
        """
        if not self.cells:
            return ComplianceState.UNKNOWN
        
        result = ComplianceState.COMPLIANT
        for cell in self.cells.values():
            result = result.meet(cell.state)
        return result
    
    def all_compliant(self) -> bool:
        """Check if all cells in the slice are compliant or exempt."""
        return all(
            cell.state in {ComplianceState.COMPLIANT, ComplianceState.EXEMPT}
            for cell in self.cells.values()
        )
    
    def non_compliant_coords(self) -> List[TensorCoord]:
        """Return coordinates of non-compliant cells."""
        return [
            coord for coord, cell in self.cells.items()
            if cell.state == ComplianceState.NON_COMPLIANT
        ]
    
    def pending_coords(self) -> List[TensorCoord]:
        """Return coordinates of pending cells."""
        return [
            coord for coord, cell in self.cells.items()
            if cell.state == ComplianceState.PENDING
        ]
    
    def to_dict(self) -> Dict[str, Any]:
        # Use list of objects with full coord serialization to preserve data
        # (str(coord) truncates asset_id for display, losing data)
        cells_list = [
            {
                "coord": coord.to_dict(),
                "cell": cell.to_dict(),
            }
            for coord, cell in self.cells.items()
        ]
        return {
            "cells": cells_list,
            "slice_dims": self.slice_dims,
            "parent_commitment": self.parent_commitment,
            "aggregate_state": self.aggregate_state().value,
        }


# =============================================================================
# TENSOR COMMITMENT
# =============================================================================

@dataclass(frozen=True)
class TensorCommitment:
    """
    Cryptographic commitment to the compliance tensor state.
    
    The commitment is a Merkle root over all tensor cells, enabling:
    - Efficient verification of tensor equality
    - Inclusion proofs for specific cells
    - Historical state verification
    """
    root: str  # 32-byte hex digest
    cell_count: int
    timestamp: str
    asset_ids: FrozenSet[str]
    jurisdiction_ids: FrozenSet[str]
    domains: FrozenSet[ComplianceDomain]
    time_range: Tuple[int, int]  # (min_quantum, max_quantum)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "root": self.root,
            "cell_count": self.cell_count,
            "timestamp": self.timestamp,
            "asset_ids": sorted(self.asset_ids),
            "jurisdiction_ids": sorted(self.jurisdiction_ids),
            "domains": sorted(d.value for d in self.domains),
            "time_range": list(self.time_range),
        }
    
    @classmethod
    def empty(cls) -> 'TensorCommitment':
        """Return commitment for empty tensor."""
        return cls(
            root="0" * 64,
            cell_count=0,
            timestamp=datetime.now(timezone.utc).isoformat(),
            asset_ids=frozenset(),
            jurisdiction_ids=frozenset(),
            domains=frozenset(),
            time_range=(0, 0),
        )


# =============================================================================
# COMPLIANCE PROOF
# =============================================================================

@dataclass
class ComplianceProof:
    """
    A proof of compliance for specific tensor coordinates.
    
    The proof contains:
    - The coordinates being proven
    - The claimed compliance state
    - Merkle inclusion proof in the tensor commitment
    - Optional ZK proof for privacy-preserving verification
    """
    coordinates: List[TensorCoord]
    claimed_states: Dict[str, ComplianceState]  # coord_str -> state
    tensor_commitment: TensorCommitment
    merkle_proof: List[str]  # Merkle siblings for inclusion proof
    zk_proof: Optional[bytes] = None  # Optional ZK proof blob
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "coordinates": [str(c) for c in self.coordinates],
            "claimed_states": {k: v.value for k, v in self.claimed_states.items()},
            "tensor_commitment": self.tensor_commitment.to_dict(),
            "merkle_proof": self.merkle_proof,
            "zk_proof": self.zk_proof.hex() if self.zk_proof else None,
        }


# =============================================================================
# COMPLIANCE TENSOR V2
# =============================================================================

class ComplianceTensorV2:
    """
    PHOENIX Compliance Tensor - the mathematical core of Smart Asset autonomy.
    
    This tensor represents the multi-dimensional compliance state of assets
    across jurisdictions, domains, and time. It supports:
    
    - Incremental updates as attestations arrive
    - Slicing along any dimension (asset, jurisdiction, domain, time)
    - Cryptographic commitment generation (Merkle root)
    - Selective disclosure proofs for specific coordinates
    - Composition for multi-asset portfolio compliance
    
    Thread Safety: This implementation is NOT thread-safe. External
    synchronization is required for concurrent access.
    
    Example:
        tensor = ComplianceTensorV2()
        
        # Set compliance state
        tensor.set(
            asset_id="abc123...",
            jurisdiction_id="uae-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            attestations=[kyc_attestation_ref],
        )
        
        # Query compliance
        cell = tensor.get("abc123...", "uae-difc", ComplianceDomain.KYC)
        
        # Generate commitment
        commitment = tensor.commit()
        
        # Create slice for specific jurisdiction
        slice = tensor.slice(jurisdiction_id="uae-difc")
    """
    
    # Time quantum period in seconds (default: 1 day)
    TIME_QUANTUM_PERIOD = 86400
    
    def __init__(self, time_quantum_period: int = TIME_QUANTUM_PERIOD):
        """
        Initialize an empty compliance tensor.
        
        Args:
            time_quantum_period: Period for time quantization in seconds.
                                 Default is 86400 (1 day).
        """
        self._cells: Dict[TensorCoord, TensorCell] = {}
        self._time_quantum_period = time_quantum_period
        self._asset_ids: Set[str] = set()
        self._jurisdiction_ids: Set[str] = set()
        self._domains: Set[ComplianceDomain] = set()
        self._time_quanta: Set[int] = set()
        self._cached_commitment: Optional[TensorCommitment] = None
    
    def _current_quantum(self) -> int:
        """Get the current time quantum."""
        return int(datetime.now(timezone.utc).timestamp() // self._time_quantum_period)
    
    def _invalidate_cache(self) -> None:
        """Invalidate cached commitment on mutation."""
        self._cached_commitment = None
    
    def set(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domain: ComplianceDomain,
        state: ComplianceState,
        attestations: Optional[List[AttestationRef]] = None,
        time_quantum: Optional[int] = None,
        reason_code: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> TensorCoord:
        """
        Set a compliance state in the tensor.
        
        Args:
            asset_id: The Smart Asset identifier
            jurisdiction_id: The jurisdiction/harbor identifier
            domain: The compliance domain
            state: The compliance state to set
            attestations: Optional list of attestations supporting this state
            time_quantum: Optional time quantum (defaults to current)
            reason_code: Optional reason code for the determination
            metadata: Optional additional metadata
            
        Returns:
            The tensor coordinate that was set
        """
        self._invalidate_cache()
        
        tq = time_quantum if time_quantum is not None else self._current_quantum()
        coord = TensorCoord(
            asset_id=asset_id,
            jurisdiction_id=jurisdiction_id,
            domain=domain,
            time_quantum=tq,
        )
        
        cell = TensorCell(
            state=state,
            attestations=attestations or [],
            reason_code=reason_code,
            metadata=metadata or {},
        )
        
        self._cells[coord] = cell
        self._asset_ids.add(asset_id)
        self._jurisdiction_ids.add(jurisdiction_id)
        self._domains.add(domain)
        self._time_quanta.add(tq)
        
        return coord
    
    def get(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domain: ComplianceDomain,
        time_quantum: Optional[int] = None,
    ) -> TensorCell:
        """
        Get the compliance state for given coordinates.
        
        If no explicit state exists, returns UNKNOWN (fail-safe default).
        
        Args:
            asset_id: The Smart Asset identifier
            jurisdiction_id: The jurisdiction/harbor identifier
            domain: The compliance domain
            time_quantum: Optional time quantum (defaults to current)
            
        Returns:
            The tensor cell at the coordinates
        """
        tq = time_quantum if time_quantum is not None else self._current_quantum()
        coord = TensorCoord(
            asset_id=asset_id,
            jurisdiction_id=jurisdiction_id,
            domain=domain,
            time_quantum=tq,
        )
        
        # Fail-safe: unknown state if not explicitly set
        return self._cells.get(coord, TensorCell(state=ComplianceState.UNKNOWN))
    
    def evaluate(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domains: Optional[Set[ComplianceDomain]] = None,
        time_quantum: Optional[int] = None,
    ) -> Tuple[bool, ComplianceState, List[str]]:
        """
        Evaluate overall compliance for an asset in a jurisdiction.
        
        Aggregates compliance across all specified domains (or all domains
        if not specified) using lattice meet (pessimistic composition).
        
        Args:
            asset_id: The Smart Asset identifier
            jurisdiction_id: The jurisdiction/harbor identifier
            domains: Optional set of domains to check (defaults to all)
            time_quantum: Optional time quantum (defaults to current)
            
        Returns:
            Tuple of (is_compliant, aggregate_state, list_of_issues)
        """
        domains_to_check = domains or ComplianceDomain.all_domains()
        tq = time_quantum if time_quantum is not None else self._current_quantum()
        
        aggregate = ComplianceState.COMPLIANT
        issues: List[str] = []
        
        for domain in domains_to_check:
            cell = self.get(asset_id, jurisdiction_id, domain, tq)
            
            # Check for stale attestations
            if cell.is_stale():
                issues.append(f"{domain.value}: attestation expired")
                cell = TensorCell(state=ComplianceState.EXPIRED)
            
            if cell.state == ComplianceState.NON_COMPLIANT:
                issues.append(f"{domain.value}: non-compliant ({cell.reason_code or 'no reason'})")
            elif cell.state == ComplianceState.PENDING:
                issues.append(f"{domain.value}: pending verification")
            elif cell.state == ComplianceState.UNKNOWN:
                issues.append(f"{domain.value}: no attestation")
            elif cell.state == ComplianceState.EXPIRED:
                issues.append(f"{domain.value}: attestation expired")
            
            aggregate = aggregate.meet(cell.state)
        
        is_compliant = aggregate in {ComplianceState.COMPLIANT, ComplianceState.EXEMPT}
        return is_compliant, aggregate, issues
    
    def slice(
        self,
        asset_id: Optional[str] = None,
        jurisdiction_id: Optional[str] = None,
        domain: Optional[ComplianceDomain] = None,
        time_quantum: Optional[int] = None,
        time_range: Optional[Tuple[int, int]] = None,
    ) -> TensorSlice:
        """
        Create a slice of the tensor along specified dimensions.
        
        Any dimension not specified is included in full.
        
        Args:
            asset_id: Optional asset to filter by
            jurisdiction_id: Optional jurisdiction to filter by
            domain: Optional domain to filter by
            time_quantum: Optional specific time quantum
            time_range: Optional (min, max) time quantum range
            
        Returns:
            A TensorSlice containing matching cells
        """
        matching: Dict[TensorCoord, TensorCell] = {}
        
        for coord, cell in self._cells.items():
            # Filter by asset
            if asset_id is not None and coord.asset_id != asset_id:
                continue
            # Filter by jurisdiction
            if jurisdiction_id is not None and coord.jurisdiction_id != jurisdiction_id:
                continue
            # Filter by domain
            if domain is not None and coord.domain != domain:
                continue
            # Filter by time quantum
            if time_quantum is not None and coord.time_quantum != time_quantum:
                continue
            # Filter by time range
            if time_range is not None:
                if coord.time_quantum < time_range[0] or coord.time_quantum > time_range[1]:
                    continue
            
            matching[coord] = cell
        
        slice_dims = {}
        if asset_id is not None:
            slice_dims["asset_id"] = asset_id
        if jurisdiction_id is not None:
            slice_dims["jurisdiction_id"] = jurisdiction_id
        if domain is not None:
            slice_dims["domain"] = domain.value
        if time_quantum is not None:
            slice_dims["time_quantum"] = time_quantum
        if time_range is not None:
            slice_dims["time_range"] = time_range
        
        # Include parent commitment if available
        parent_commitment = self._cached_commitment.root if self._cached_commitment else None
        
        return TensorSlice(
            cells=matching,
            slice_dims=slice_dims,
            parent_commitment=parent_commitment,
        )
    
    def commit(self) -> TensorCommitment:
        """
        Generate a cryptographic commitment to the tensor state.
        
        The commitment is a Merkle root over all tensor cells, sorted by
        coordinate to ensure determinism.
        
        Returns:
            A TensorCommitment capturing the current state
        """
        if self._cached_commitment is not None:
            return self._cached_commitment
        
        if not self._cells:
            return TensorCommitment.empty()
        
        # Sort cells by coordinate for determinism
        sorted_coords = sorted(self._cells.keys(), key=lambda c: c.to_tuple())
        
        # Build Merkle tree over cells
        leaves: List[str] = []
        for coord in sorted_coords:
            cell = self._cells[coord]
            # Hash each cell with ONLY deterministic fields: coord, state, attestation digests
            # Exclude determined_at and metadata which may vary
            deterministic_data = {
                "coord": coord.to_tuple(),
                "state": cell.state.value,
                "attestation_digests": sorted([a.digest for a in cell.attestations]),
                "reason_code": cell.reason_code,
            }
            cell_data = json.dumps(deterministic_data, sort_keys=True, separators=(",", ":"))
            leaf = hashlib.sha256(cell_data.encode()).hexdigest()
            leaves.append(leaf)
        
        # Compute Merkle root
        root = self._merkle_root(leaves)
        
        # Compute time range
        min_tq = min(self._time_quanta) if self._time_quanta else 0
        max_tq = max(self._time_quanta) if self._time_quanta else 0
        
        self._cached_commitment = TensorCommitment(
            root=root,
            cell_count=len(self._cells),
            timestamp=datetime.now(timezone.utc).isoformat(),
            asset_ids=frozenset(self._asset_ids),
            jurisdiction_ids=frozenset(self._jurisdiction_ids),
            domains=frozenset(self._domains),
            time_range=(min_tq, max_tq),
        )
        
        return self._cached_commitment
    
    def _merkle_root(self, leaves: List[str]) -> str:
        """Compute Merkle root from leaf hashes."""
        if not leaves:
            return "0" * 64
        
        if len(leaves) == 1:
            return leaves[0]
        
        # Pad to power of 2
        while len(leaves) & (len(leaves) - 1):
            leaves.append(leaves[-1])
        
        # Build tree bottom-up
        while len(leaves) > 1:
            next_level: List[str] = []
            for i in range(0, len(leaves), 2):
                combined = leaves[i] + leaves[i + 1]
                parent = hashlib.sha256(combined.encode()).hexdigest()
                next_level.append(parent)
            leaves = next_level
        
        return leaves[0]
    
    def prove_compliance(
        self,
        coordinates: List[TensorCoord],
    ) -> ComplianceProof:
        """
        Generate a proof of compliance for specific coordinates.

        The proof includes Merkle inclusion proofs for the specified
        coordinates within the tensor commitment.

        Args:
            coordinates: List of coordinates to prove

        Returns:
            A ComplianceProof object
        """
        commitment = self.commit()

        claimed_states: Dict[str, ComplianceState] = {}
        for coord in coordinates:
            cell = self._cells.get(coord, TensorCell(state=ComplianceState.UNKNOWN))
            claimed_states[str(coord)] = cell.state

        # Generate Merkle proof with sibling hashes
        merkle_proof = self._generate_merkle_proof(coordinates)

        return ComplianceProof(
            coordinates=coordinates,
            claimed_states=claimed_states,
            tensor_commitment=commitment,
            merkle_proof=merkle_proof,
        )

    def _generate_merkle_proof(self, coordinates: List[TensorCoord]) -> List[str]:
        """
        Generate Merkle inclusion proof for the given coordinates.

        Returns a list of sibling hashes needed to verify inclusion.
        """
        if not self._cells:
            return []

        # Build ordered list of leaf hashes
        sorted_coords = sorted(self._cells.keys(), key=lambda c: c.to_tuple())
        leaves: List[str] = []
        coord_indices: Dict[TensorCoord, int] = {}

        for i, coord in enumerate(sorted_coords):
            cell = self._cells[coord]
            # MUST match the leaf format used in commit() exactly
            deterministic_data = {
                "coord": coord.to_tuple(),
                "state": cell.state.value,
                "attestation_digests": sorted([a.digest for a in cell.attestations]),
                "reason_code": cell.reason_code,
            }
            leaf_content = json.dumps(deterministic_data, sort_keys=True, separators=(",", ":"))
            leaf_hash = hashlib.sha256(leaf_content.encode()).hexdigest()
            leaves.append(leaf_hash)
            coord_indices[coord] = i

        # Find indices of coordinates we're proving
        target_indices = set()
        for coord in coordinates:
            if coord in coord_indices:
                target_indices.add(coord_indices[coord])

        if not target_indices:
            return []

        # Pad leaves to power of 2
        original_len = len(leaves)
        while len(leaves) & (len(leaves) - 1):
            leaves.append(leaves[-1])

        # Build tree and collect proof siblings
        proof_siblings: List[str] = []
        current_level = leaves[:]

        while len(current_level) > 1:
            next_level: List[str] = []
            next_target_indices = set()

            for i in range(0, len(current_level), 2):
                left = current_level[i]
                right = current_level[i + 1] if i + 1 < len(current_level) else left

                # If either child is a target, add sibling to proof
                if i in target_indices:
                    proof_siblings.append(right)
                    next_target_indices.add(i // 2)
                elif i + 1 in target_indices:
                    proof_siblings.append(left)
                    next_target_indices.add(i // 2)

                combined = left + right
                parent = hashlib.sha256(combined.encode()).hexdigest()
                next_level.append(parent)

            current_level = next_level
            target_indices = next_target_indices

        return proof_siblings
    
    def merge(self, other: 'ComplianceTensorV2') -> 'ComplianceTensorV2':
        """
        Merge another tensor into this one.
        
        For conflicting coordinates, the cell with the more recent
        determination timestamp wins.
        
        Args:
            other: Another ComplianceTensorV2 to merge
            
        Returns:
            Self (for chaining)
        """
        self._invalidate_cache()
        
        for coord, cell in other._cells.items():
            existing = self._cells.get(coord)
            if existing is None:
                self._cells[coord] = cell
                self._asset_ids.add(coord.asset_id)
                self._jurisdiction_ids.add(coord.jurisdiction_id)
                self._domains.add(coord.domain)
                self._time_quanta.add(coord.time_quantum)
            else:
                # More recent determination wins
                if cell.determined_at > existing.determined_at:
                    self._cells[coord] = cell
        
        return self
    
    def to_dict(self) -> Dict[str, Any]:
        """Serialize tensor to dictionary with full coordinate preservation."""
        # Use JSON-serializable coordinate representation
        cells_list = [
            {
                "coord": coord.to_dict(),
                "cell": cell.to_dict(),
            }
            for coord, cell in self._cells.items()
        ]
        return {
            "cells": cells_list,
            "time_quantum_period": self._time_quantum_period,
            "commitment": self.commit().to_dict(),
        }

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'ComplianceTensorV2':
        """Deserialize tensor from dictionary."""
        tensor = cls(time_quantum_period=data.get("time_quantum_period", cls.TIME_QUANTUM_PERIOD))

        cells_data = data.get("cells", [])

        # Support both new list format and legacy string-key format
        if isinstance(cells_data, list):
            # New format: list of {coord, cell} objects
            for item in cells_data:
                coord = TensorCoord.from_dict(item["coord"])
                cell = TensorCell.from_dict(item["cell"])

                tensor._cells[coord] = cell
                tensor._asset_ids.add(coord.asset_id)
                tensor._jurisdiction_ids.add(coord.jurisdiction_id)
                tensor._domains.add(coord.domain)
                tensor._time_quanta.add(coord.time_quantum)
        else:
            # Legacy format: string keys (best effort parsing)
            for coord_str, cell_data in cells_data.items():
                parts = coord_str.split(":")
                if len(parts) >= 4:
                    asset_id = parts[0]
                    jurisdiction_id = parts[1]
                    domain = ComplianceDomain(parts[2])
                    time_quantum = int(parts[3].replace("t", ""))

                    cell = TensorCell.from_dict(cell_data)

                    coord = TensorCoord(
                        asset_id=asset_id,
                        jurisdiction_id=jurisdiction_id,
                        domain=domain,
                        time_quantum=time_quantum,
                    )

                    tensor._cells[coord] = cell
                    tensor._asset_ids.add(asset_id)
                    tensor._jurisdiction_ids.add(jurisdiction_id)
                    tensor._domains.add(domain)
                    tensor._time_quanta.add(time_quantum)

        return tensor
    
    def __len__(self) -> int:
        """Return number of cells in tensor."""
        return len(self._cells)
    
    def __iter__(self) -> Iterator[Tuple[TensorCoord, TensorCell]]:
        """Iterate over (coordinate, cell) pairs."""
        return iter(self._cells.items())
    
    def __contains__(self, coord: TensorCoord) -> bool:
        """Check if coordinate exists in tensor."""
        return coord in self._cells


# =============================================================================
# TENSOR OPERATIONS
# =============================================================================

def tensor_meet(t1: ComplianceTensorV2, t2: ComplianceTensorV2) -> ComplianceTensorV2:
    """
    Compute the meet (intersection) of two tensors.
    
    For coordinates present in both, uses lattice meet of states.
    For coordinates in only one, includes as-is.
    
    This is useful for computing the joint compliance of two assets
    or two regulatory viewpoints.
    """
    result = ComplianceTensorV2()
    
    all_coords = set(t1._cells.keys()) | set(t2._cells.keys())
    
    for coord in all_coords:
        c1 = t1._cells.get(coord)
        c2 = t2._cells.get(coord)
        
        if c1 is None:
            result._cells[coord] = c2
        elif c2 is None:
            result._cells[coord] = c1
        else:
            # Lattice meet of states
            meet_state = c1.state.meet(c2.state)
            # Combine attestations
            # Combine attestations with deterministic ordering
            combined_attestations = sorted(
                set(c1.attestations) | set(c2.attestations),
                key=lambda a: a.attestation_id
            )
            result._cells[coord] = TensorCell(
                state=meet_state,
                attestations=combined_attestations,
                reason_code=f"meet({c1.reason_code}, {c2.reason_code})",
            )
        
        result._asset_ids.add(coord.asset_id)
        result._jurisdiction_ids.add(coord.jurisdiction_id)
        result._domains.add(coord.domain)
        result._time_quanta.add(coord.time_quantum)
    
    return result


def tensor_join(t1: ComplianceTensorV2, t2: ComplianceTensorV2) -> ComplianceTensorV2:
    """
    Compute the join (union) of two tensors.
    
    For coordinates present in both, uses lattice join of states.
    For coordinates in only one, includes as-is.
    
    This is useful for computing the best-case compliance when
    multiple attestation sources are available.
    """
    result = ComplianceTensorV2()
    
    all_coords = set(t1._cells.keys()) | set(t2._cells.keys())
    
    for coord in all_coords:
        c1 = t1._cells.get(coord)
        c2 = t2._cells.get(coord)
        
        if c1 is None:
            result._cells[coord] = c2
        elif c2 is None:
            result._cells[coord] = c1
        else:
            # Lattice join of states
            join_state = c1.state.join(c2.state)
            # Combine attestations
            # Combine attestations with deterministic ordering
            combined_attestations = sorted(
                set(c1.attestations) | set(c2.attestations),
                key=lambda a: a.attestation_id
            )
            result._cells[coord] = TensorCell(
                state=join_state,
                attestations=combined_attestations,
                reason_code=f"join({c1.reason_code}, {c2.reason_code})",
            )
        
        result._asset_ids.add(coord.asset_id)
        result._jurisdiction_ids.add(coord.jurisdiction_id)
        result._domains.add(coord.domain)
        result._time_quanta.add(coord.time_quantum)
    
    return result


def portfolio_compliance(
    tensors: List[ComplianceTensorV2],
    jurisdiction_id: str,
) -> Tuple[bool, ComplianceState, Dict[str, List[str]]]:
    """
    Evaluate portfolio-level compliance across multiple assets.
    
    Args:
        tensors: List of compliance tensors for individual assets
        jurisdiction_id: The jurisdiction to evaluate
        
    Returns:
        Tuple of (all_compliant, aggregate_state, issues_by_asset)
    """
    aggregate = ComplianceState.COMPLIANT
    issues_by_asset: Dict[str, List[str]] = {}
    
    for tensor in tensors:
        for asset_id in tensor._asset_ids:
            is_compliant, state, issues = tensor.evaluate(asset_id, jurisdiction_id)
            
            aggregate = aggregate.meet(state)
            
            if issues:
                issues_by_asset[asset_id] = issues
    
    all_compliant = aggregate in {ComplianceState.COMPLIANT, ComplianceState.EXEMPT}
    return all_compliant, aggregate, issues_by_asset
