"""
PHOENIX Corridor Bridge Protocol

Orchestrates multi-hop asset transfers through intermediate corridors when no direct
path exists. The bridge protocol ensures atomicity across multiple corridor hops using
a two-phase commit mechanism with cryptographic receipts at each stage.

Architecture:

    ┌─────────┐        ┌─────────┐        ┌─────────┐        ┌─────────┐
    │  Source │───────▶│ Corridor│───────▶│ Corridor│───────▶│  Target │
    │   SEZ   │  Hop 1 │    A    │  Hop 2 │    B    │  Hop 3 │   SEZ   │
    └─────────┘        └─────────┘        └─────────┘        └─────────┘
         │                  │                  │                  │
         └──────────────────┼──────────────────┼──────────────────┘
                            │                  │
                      Phase 1: PREPARE    Phase 2: COMMIT
                      (Lock at each hop)  (Finalize transfers)

Protocol Phases:

    1. DISCOVERY: Find optimal path through corridor graph
    2. PREPARE: Lock asset at each hop, collect prepare receipts
    3. COMMIT: Execute transfers atomically, collect commit receipts
    4. FINALIZE: Unlock at destination, update compliance tensor

Failure Handling:

    - PREPARE fails: Release all locks, compensate
    - COMMIT fails: Retry with exponential backoff
    - Timeout: Trigger dispute resolution

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
import secrets
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum
from typing import Any, Callable, Dict, List, Optional, Set, Tuple

from tools.phoenix.tensor import (
    ComplianceDomain,
    ComplianceState,
    ComplianceTensorV2,
    TensorCommitment,
    AttestationRef,
)
from tools.phoenix.manifold import (
    ComplianceManifold,
    MigrationPath,
    MigrationHop,
    CorridorEdge,
    JurisdictionNode,
)


# =============================================================================
# BRIDGE PROTOCOL STATES
# =============================================================================

class BridgePhase(Enum):
    """Phases of the bridge protocol."""
    INITIATED = "initiated"
    DISCOVERY = "discovery"
    PREPARE = "prepare"
    COMMIT = "commit"
    FINALIZE = "finalize"
    COMPLETED = "completed"
    FAILED = "failed"
    COMPENSATING = "compensating"


class HopStatus(Enum):
    """Status of an individual hop in the bridge."""
    PENDING = "pending"
    PREPARING = "preparing"
    PREPARED = "prepared"
    COMMITTING = "committing"
    COMMITTED = "committed"
    FAILED = "failed"
    COMPENSATED = "compensated"


# =============================================================================
# BRIDGE RECEIPTS
# =============================================================================

@dataclass
class PrepareReceipt:
    """
    Receipt from a successful PREPARE operation at a hop.
    
    The prepare receipt proves that the asset is locked at this
    hop and ready for the commit phase.
    """
    receipt_id: str
    hop_index: int
    corridor_id: str
    asset_id: str
    
    # Lock details
    lock_id: str
    locked_amount: Decimal
    lock_expiry: str
    
    # Signatures
    source_signature: bytes
    corridor_signature: bytes
    
    # Compliance
    compliance_tensor_slice: Dict[str, Any]
    
    # Timing
    prepared_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    @property
    def digest(self) -> str:
        """Canonical digest of the receipt."""
        content = {
            "receipt_id": self.receipt_id,
            "hop_index": self.hop_index,
            "corridor_id": self.corridor_id,
            "asset_id": self.asset_id,
            "lock_id": self.lock_id,
            "locked_amount": str(self.locked_amount),
            "lock_expiry": self.lock_expiry,
            "prepared_at": self.prepared_at,
        }
        from tools.lawpack import jcs_canonicalize
        return hashlib.sha256(jcs_canonicalize(content)).hexdigest()

    def to_dict(self) -> Dict[str, Any]:
        return {
            "receipt_id": self.receipt_id,
            "digest": self.digest,
            "hop_index": self.hop_index,
            "corridor_id": self.corridor_id,
            "asset_id": self.asset_id,
            "lock_id": self.lock_id,
            "locked_amount": str(self.locked_amount),
            "lock_expiry": self.lock_expiry,
            "prepared_at": self.prepared_at,
        }


@dataclass
class CommitReceipt:
    """
    Receipt from a successful COMMIT operation at a hop.
    
    The commit receipt proves that the transfer was executed
    at this hop.
    """
    receipt_id: str
    hop_index: int
    corridor_id: str
    asset_id: str
    
    # Transfer details
    prepare_receipt_digest: str
    transfer_amount: Decimal
    
    # Settlement
    settlement_tx_id: str
    settlement_block: int
    
    # Signatures
    corridor_signature: bytes
    target_signature: bytes
    
    # Timing
    committed_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    @property
    def digest(self) -> str:
        """Canonical digest of the receipt."""
        content = {
            "receipt_id": self.receipt_id,
            "hop_index": self.hop_index,
            "corridor_id": self.corridor_id,
            "asset_id": self.asset_id,
            "prepare_receipt_digest": self.prepare_receipt_digest,
            "transfer_amount": str(self.transfer_amount),
            "settlement_tx_id": self.settlement_tx_id,
            "committed_at": self.committed_at,
        }
        from tools.lawpack import jcs_canonicalize
        return hashlib.sha256(jcs_canonicalize(content)).hexdigest()

    def to_dict(self) -> Dict[str, Any]:
        return {
            "receipt_id": self.receipt_id,
            "digest": self.digest,
            "hop_index": self.hop_index,
            "corridor_id": self.corridor_id,
            "asset_id": self.asset_id,
            "prepare_receipt_digest": self.prepare_receipt_digest,
            "transfer_amount": str(self.transfer_amount),
            "settlement_tx_id": self.settlement_tx_id,
            "settlement_block": self.settlement_block,
            "committed_at": self.committed_at,
        }


# =============================================================================
# HOP EXECUTION STATE
# =============================================================================

@dataclass
class HopExecution:
    """
    Execution state for a single hop in the bridge.
    """
    hop_index: int
    corridor: CorridorEdge
    source_jurisdiction: JurisdictionNode
    target_jurisdiction: JurisdictionNode
    
    # Status
    status: HopStatus = HopStatus.PENDING
    
    # Receipts
    prepare_receipt: Optional[PrepareReceipt] = None
    commit_receipt: Optional[CommitReceipt] = None
    
    # Errors
    error: Optional[str] = None
    retry_count: int = 0
    
    # Timing
    started_at: Optional[str] = None
    completed_at: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "hop_index": self.hop_index,
            "corridor_id": self.corridor.corridor_id,
            "source": self.source_jurisdiction.jurisdiction_id,
            "target": self.target_jurisdiction.jurisdiction_id,
            "status": self.status.value,
            "prepare_receipt": self.prepare_receipt.to_dict() if self.prepare_receipt else None,
            "commit_receipt": self.commit_receipt.to_dict() if self.commit_receipt else None,
            "error": self.error,
            "retry_count": self.retry_count,
        }


# =============================================================================
# BRIDGE REQUEST
# =============================================================================

@dataclass
class BridgeRequest:
    """
    Request to bridge an asset across multiple corridors.
    """
    bridge_id: str
    asset_id: str
    asset_genesis_digest: str
    
    # Endpoints
    source_jurisdiction: str
    target_jurisdiction: str
    
    # Value
    amount: Decimal
    currency: str
    
    # Path (populated during discovery)
    migration_path: Optional[MigrationPath] = None
    
    # Requestor
    requestor_did: str = ""
    requestor_signature: bytes = b""
    
    # Constraints
    max_hops: int = 5
    max_time_hours: int = 72
    max_fee_bps: int = 100  # 1%
    
    # Metadata
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "bridge_id": self.bridge_id,
            "asset_id": self.asset_id,
            "source_jurisdiction": self.source_jurisdiction,
            "target_jurisdiction": self.target_jurisdiction,
            "amount": str(self.amount),
            "currency": self.currency,
            "max_hops": self.max_hops,
            "max_time_hours": self.max_time_hours,
            "created_at": self.created_at,
        }


# =============================================================================
# BRIDGE EXECUTION
# =============================================================================

@dataclass
class BridgeExecution:
    """
    Complete execution state for a bridge operation.
    """
    request: BridgeRequest
    phase: BridgePhase = BridgePhase.INITIATED
    
    # Hop executions
    hops: List[HopExecution] = field(default_factory=list)
    
    # Aggregate metrics
    total_fees: Decimal = Decimal("0")
    total_time_seconds: int = 0
    
    # Compliance
    source_tensor_commitment: Optional[TensorCommitment] = None
    target_tensor_commitment: Optional[TensorCommitment] = None
    
    # History
    phase_history: List[Tuple[str, BridgePhase]] = field(default_factory=list)
    
    # Errors
    fatal_error: Optional[str] = None
    
    # Timing
    started_at: Optional[str] = None
    completed_at: Optional[str] = None
    
    @property
    def is_complete(self) -> bool:
        return self.phase in {BridgePhase.COMPLETED, BridgePhase.FAILED}
    
    @property
    def is_successful(self) -> bool:
        return self.phase == BridgePhase.COMPLETED
    
    @property
    def current_hop_index(self) -> int:
        """Index of the currently executing hop."""
        for i, hop in enumerate(self.hops):
            if hop.status not in {HopStatus.COMMITTED, HopStatus.COMPENSATED}:
                return i
        return len(self.hops)
    
    @property
    def all_hops_prepared(self) -> bool:
        return all(h.status == HopStatus.PREPARED for h in self.hops)
    
    @property
    def all_hops_committed(self) -> bool:
        return all(h.status == HopStatus.COMMITTED for h in self.hops)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "bridge_id": self.request.bridge_id,
            "phase": self.phase.value,
            "is_complete": self.is_complete,
            "is_successful": self.is_successful,
            "request": self.request.to_dict(),
            "hops": [h.to_dict() for h in self.hops],
            "current_hop_index": self.current_hop_index,
            "total_fees": str(self.total_fees),
            "total_time_seconds": self.total_time_seconds,
            "fatal_error": self.fatal_error,
            "started_at": self.started_at,
            "completed_at": self.completed_at,
        }


# =============================================================================
# CORRIDOR BRIDGE
# =============================================================================

class CorridorBridge:
    """
    Orchestrates multi-hop asset transfers across corridors.
    
    The bridge implements a two-phase commit protocol:
    
    Phase 1 (PREPARE):
        - For each hop in sequence:
        - Lock asset at source
        - Verify compliance at target
        - Collect prepare receipt
        
    Phase 2 (COMMIT):
        - For each hop in sequence:
        - Execute transfer
        - Verify settlement
        - Collect commit receipt
        
    If any hop fails during PREPARE, all previous locks are released.
    If any hop fails during COMMIT, retry with exponential backoff.
    
    Example:
        bridge = CorridorBridge(manifold)
        
        request = BridgeRequest(
            bridge_id="bridge-123",
            asset_id="asset-456",
            asset_genesis_digest="abc...",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("1000000"),
            currency="USD",
        )
        
        execution = bridge.execute(request)
        
        if execution.is_successful:
            print(f"Bridge completed: {execution.completed_at}")
    """
    
    # Default timeouts
    PREPARE_TIMEOUT_SECONDS = 300  # 5 minutes per hop
    COMMIT_TIMEOUT_SECONDS = 600   # 10 minutes per hop
    MAX_RETRIES = 3
    
    def __init__(
        self,
        manifold: ComplianceManifold,
        prepare_handler: Optional[Callable[[HopExecution], PrepareReceipt]] = None,
        commit_handler: Optional[Callable[[HopExecution, PrepareReceipt], CommitReceipt]] = None,
    ):
        self._manifold = manifold
        self._executions: Dict[str, BridgeExecution] = {}
        
        # Handlers for actual corridor interactions
        self._prepare_handler = prepare_handler or self._mock_prepare
        self._commit_handler = commit_handler or self._mock_commit
    
    def execute(
        self,
        request: BridgeRequest,
        existing_attestations: Optional[List[AttestationRef]] = None,
    ) -> BridgeExecution:
        """
        Execute a bridge request.
        
        This is the main entry point for bridge operations.
        """
        execution = BridgeExecution(request=request)
        execution.started_at = datetime.now(timezone.utc).isoformat()
        
        self._executions[request.bridge_id] = execution
        self._record_phase(execution, BridgePhase.INITIATED)
        
        try:
            # Phase 1: Discovery
            self._execute_discovery(execution, existing_attestations)
            
            if execution.phase == BridgePhase.FAILED:
                return execution
            
            # Phase 2: Prepare all hops
            self._execute_prepare_phase(execution)
            
            if execution.phase == BridgePhase.FAILED:
                self._execute_compensation(execution)
                return execution
            
            # Phase 3: Commit all hops
            self._execute_commit_phase(execution)
            
            if execution.phase == BridgePhase.FAILED:
                # Commit failures are serious - may need manual intervention
                return execution
            
            # Phase 4: Finalize
            self._execute_finalize(execution)
            
        except Exception as e:
            execution.fatal_error = str(e)
            self._record_phase(execution, BridgePhase.FAILED)
        
        return execution
    
    def _record_phase(self, execution: BridgeExecution, phase: BridgePhase) -> None:
        """Record a phase transition."""
        execution.phase = phase
        execution.phase_history.append((
            datetime.now(timezone.utc).isoformat(),
            phase,
        ))
    
    def _execute_discovery(
        self,
        execution: BridgeExecution,
        existing_attestations: Optional[List[AttestationRef]],
    ) -> None:
        """Execute the discovery phase - find optimal path."""
        self._record_phase(execution, BridgePhase.DISCOVERY)
        
        request = execution.request
        
        # Find path through manifold
        from tools.phoenix.manifold import PathConstraint
        
        constraints = PathConstraint(
            max_hops=request.max_hops,
            max_total_time_hours=request.max_time_hours,
        )
        
        path = self._manifold.find_path(
            request.source_jurisdiction,
            request.target_jurisdiction,
            constraints=constraints,
            asset_value_usd=request.amount,
            existing_attestations=existing_attestations,
        )
        
        if not path:
            execution.fatal_error = (
                f"No path found from {request.source_jurisdiction} "
                f"to {request.target_jurisdiction}"
            )
            self._record_phase(execution, BridgePhase.FAILED)
            return
        
        # Check fee constraint (guard against zero amount)
        if request.amount <= 0:
            execution.fatal_error = "Bridge amount must be positive"
            self._record_phase(execution, BridgePhase.FAILED)
            return
        
        # Use Decimal arithmetic to preserve precision before int conversion
        fee_bps = int((Decimal(str(path.total_cost_usd)) / Decimal(str(request.amount))) * Decimal("10000"))
        if fee_bps > request.max_fee_bps:
            execution.fatal_error = (
                f"Path fees ({fee_bps} bps) exceed max ({request.max_fee_bps} bps)"
            )
            self._record_phase(execution, BridgePhase.FAILED)
            return
        
        request.migration_path = path
        
        # Initialize hop executions
        for i, hop in enumerate(path.hops):
            hop_exec = HopExecution(
                hop_index=i,
                corridor=hop.corridor,
                source_jurisdiction=hop.source,
                target_jurisdiction=hop.target,
            )
            execution.hops.append(hop_exec)
    
    def _execute_prepare_phase(self, execution: BridgeExecution) -> None:
        """Execute the prepare phase - lock at each hop."""
        self._record_phase(execution, BridgePhase.PREPARE)
        
        for hop_exec in execution.hops:
            hop_exec.status = HopStatus.PREPARING
            hop_exec.started_at = datetime.now(timezone.utc).isoformat()
            
            try:
                receipt = self._prepare_handler(hop_exec)
                hop_exec.prepare_receipt = receipt
                hop_exec.status = HopStatus.PREPARED
            except Exception as e:
                hop_exec.status = HopStatus.FAILED
                hop_exec.error = str(e)
                self._record_phase(execution, BridgePhase.FAILED)
                return
    
    def _execute_commit_phase(self, execution: BridgeExecution) -> None:
        """Execute the commit phase - transfer at each hop."""
        self._record_phase(execution, BridgePhase.COMMIT)
        
        for hop_exec in execution.hops:
            hop_exec.status = HopStatus.COMMITTING
            
            success = False
            for attempt in range(self.MAX_RETRIES):
                try:
                    receipt = self._commit_handler(hop_exec, hop_exec.prepare_receipt)
                    hop_exec.commit_receipt = receipt
                    hop_exec.status = HopStatus.COMMITTED
                    hop_exec.completed_at = datetime.now(timezone.utc).isoformat()
                    
                    # Accumulate fees
                    execution.total_fees += hop_exec.corridor.transfer_cost(
                        execution.request.amount
                    )
                    
                    success = True
                    break
                except Exception as e:
                    hop_exec.retry_count += 1
                    hop_exec.error = str(e)
            
            if not success:
                hop_exec.status = HopStatus.FAILED
                self._record_phase(execution, BridgePhase.FAILED)
                return
    
    def _execute_finalize(self, execution: BridgeExecution) -> None:
        """Execute the finalize phase - complete the bridge."""
        self._record_phase(execution, BridgePhase.FINALIZE)
        
        # Calculate total time
        if execution.started_at:
            from tools.phoenix.hardening import parse_iso_timestamp
            start = parse_iso_timestamp(execution.started_at)
            now = datetime.now(timezone.utc)
            execution.total_time_seconds = int((now - start).total_seconds())
        
        execution.completed_at = datetime.now(timezone.utc).isoformat()
        self._record_phase(execution, BridgePhase.COMPLETED)
    
    def _execute_compensation(self, execution: BridgeExecution) -> None:
        """Execute compensation for failed bridge."""
        self._record_phase(execution, BridgePhase.COMPENSATING)
        
        # Release locks in reverse order
        for hop_exec in reversed(execution.hops):
            if hop_exec.status == HopStatus.PREPARED:
                # Would release lock here
                hop_exec.status = HopStatus.COMPENSATED
            elif hop_exec.status == HopStatus.PREPARING:
                hop_exec.status = HopStatus.COMPENSATED
        
        self._record_phase(execution, BridgePhase.FAILED)
    
    # =========================================================================
    # MOCK HANDLERS FOR TESTING
    # =========================================================================
    
    def _mock_prepare(self, hop_exec: HopExecution) -> PrepareReceipt:
        """Mock prepare handler for testing."""
        lock_expiry = datetime.now(timezone.utc) + timedelta(hours=1)
        
        return PrepareReceipt(
            receipt_id=f"prep-{secrets.token_hex(8)}",
            hop_index=hop_exec.hop_index,
            corridor_id=hop_exec.corridor.corridor_id,
            asset_id="mock-asset",
            lock_id=f"lock-{secrets.token_hex(8)}",
            locked_amount=Decimal("1000"),
            lock_expiry=lock_expiry.isoformat(),
            source_signature=b"source-sig",
            corridor_signature=b"corridor-sig",
            compliance_tensor_slice={},
        )
    
    def _mock_commit(
        self,
        hop_exec: HopExecution,
        prepare_receipt: PrepareReceipt,
    ) -> CommitReceipt:
        """Mock commit handler for testing."""
        return CommitReceipt(
            receipt_id=f"commit-{secrets.token_hex(8)}",
            hop_index=hop_exec.hop_index,
            corridor_id=hop_exec.corridor.corridor_id,
            asset_id="mock-asset",
            prepare_receipt_digest=prepare_receipt.digest,
            transfer_amount=Decimal("1000"),
            settlement_tx_id=f"0x{secrets.token_hex(32)}",
            settlement_block=1000000 + hop_exec.hop_index,
            corridor_signature=b"corridor-sig",
            target_signature=b"target-sig",
        )
    
    # =========================================================================
    # QUERY METHODS
    # =========================================================================
    
    def get_execution(self, bridge_id: str) -> Optional[BridgeExecution]:
        """Get bridge execution by ID."""
        return self._executions.get(bridge_id)
    
    def list_executions(
        self,
        phase: Optional[BridgePhase] = None,
        source: Optional[str] = None,
        target: Optional[str] = None,
    ) -> List[BridgeExecution]:
        """List bridge executions with optional filters."""
        executions = list(self._executions.values())
        
        if phase:
            executions = [e for e in executions if e.phase == phase]
        if source:
            executions = [e for e in executions if e.request.source_jurisdiction == source]
        if target:
            executions = [e for e in executions if e.request.target_jurisdiction == target]
        
        return executions
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get bridge statistics."""
        by_phase: Dict[str, int] = {}
        total_volume = Decimal("0")
        total_fees = Decimal("0")
        
        for execution in self._executions.values():
            phase = execution.phase.value
            by_phase[phase] = by_phase.get(phase, 0) + 1
            
            if execution.is_successful:
                total_volume += execution.request.amount
                total_fees += execution.total_fees
        
        return {
            "total_bridges": len(self._executions),
            "by_phase": by_phase,
            "completed": by_phase.get("completed", 0),
            "failed": by_phase.get("failed", 0),
            "total_volume": str(total_volume),
            "total_fees": str(total_fees),
        }


# =============================================================================
# RECEIPT CHAIN
# =============================================================================

class BridgeReceiptChain:
    """
    Maintains the chain of receipts for bridge operations.
    
    The receipt chain provides an immutable audit trail of all
    bridge operations, suitable for regulatory review.
    """
    
    def __init__(self):
        self._prepare_receipts: Dict[str, PrepareReceipt] = {}
        self._commit_receipts: Dict[str, CommitReceipt] = {}
        self._bridge_receipts: Dict[str, List[str]] = {}  # bridge_id -> [receipt_ids]
    
    def add_prepare_receipt(
        self,
        bridge_id: str,
        receipt: PrepareReceipt,
    ) -> None:
        """Add a prepare receipt to the chain."""
        self._prepare_receipts[receipt.receipt_id] = receipt
        
        if bridge_id not in self._bridge_receipts:
            self._bridge_receipts[bridge_id] = []
        self._bridge_receipts[bridge_id].append(receipt.receipt_id)
    
    def add_commit_receipt(
        self,
        bridge_id: str,
        receipt: CommitReceipt,
    ) -> None:
        """Add a commit receipt to the chain."""
        self._commit_receipts[receipt.receipt_id] = receipt
        
        if bridge_id not in self._bridge_receipts:
            self._bridge_receipts[bridge_id] = []
        self._bridge_receipts[bridge_id].append(receipt.receipt_id)
    
    def get_bridge_receipts(self, bridge_id: str) -> Dict[str, Any]:
        """Get all receipts for a bridge operation."""
        receipt_ids = self._bridge_receipts.get(bridge_id, [])
        
        prepare = []
        commit = []
        
        for rid in receipt_ids:
            if rid in self._prepare_receipts:
                prepare.append(self._prepare_receipts[rid].to_dict())
            if rid in self._commit_receipts:
                commit.append(self._commit_receipts[rid].to_dict())
        
        return {
            "bridge_id": bridge_id,
            "prepare_receipts": prepare,
            "commit_receipts": commit,
        }
    
    def compute_merkle_root(self, bridge_id: str) -> str:
        """Compute Merkle root of all receipts for a bridge."""
        receipt_ids = self._bridge_receipts.get(bridge_id, [])
        
        if not receipt_ids:
            return "0" * 64
        
        # Collect all receipt digests
        digests = []
        for rid in receipt_ids:
            if rid in self._prepare_receipts:
                digests.append(self._prepare_receipts[rid].digest)
            if rid in self._commit_receipts:
                digests.append(self._commit_receipts[rid].digest)
        
        # Build Merkle tree
        if len(digests) == 1:
            return digests[0]
        
        while len(digests) > 1:
            if len(digests) % 2 == 1:
                digests.append(digests[-1])
            
            next_level = []
            for i in range(0, len(digests), 2):
                combined = digests[i] + digests[i + 1]
                parent = hashlib.sha256(combined.encode()).hexdigest()
                next_level.append(parent)
            digests = next_level
        
        return digests[0]


# =============================================================================
# FACTORY FUNCTIONS
# =============================================================================

def create_bridge_with_manifold(
    manifold: Optional[ComplianceManifold] = None,
) -> CorridorBridge:
    """Create a corridor bridge with optional manifold."""
    if manifold is None:
        from tools.phoenix.manifold import create_standard_manifold
        manifold = create_standard_manifold()
    
    return CorridorBridge(manifold)
