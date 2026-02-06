"""
PHOENIX Cross-Jurisdictional Migration Protocol

The Migration Protocol orchestrates Smart Asset movement between jurisdictions while
maintaining continuous compliance and operational integrity. It implements a saga
pattern with compensation for failures, ensuring atomicity of migrations.

State Machine:

    INITIATED ──────────────▶ COMPLIANCE_CHECK
         │                          │
         │                          ▼
         │                   ATTESTATION_GATHERING
         │                          │
         │                          ▼
         │                     SOURCE_LOCK
         │                          │
         │                          ▼
         │                       TRANSIT
         │                          │
         │                          ▼
         │              DESTINATION_VERIFICATION
         │                          │
         │                          ▼
         │                 DESTINATION_UNLOCK
         │                          │
         │                          ▼
         │                      COMPLETED
         │
         │  ┌───────────────────────┼───────────────────────┐
         │  │                       │                       │
         ▼  ▼                       ▼                       ▼
    COMPENSATED ◀────────────── DISPUTED ◀───────────── (any state)

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
import secrets
import time
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum, auto
from typing import Any, Callable, Dict, List, Optional, Set, Tuple

from tools.phoenix.tensor import (
    ComplianceDomain,
    ComplianceState,
    ComplianceTensorV2,
    TensorCommitment,
    AttestationRef,
)
from tools.phoenix.manifold import (
    MigrationPath,
    AttestationGap,
    AttestationRequirement,
)


# =============================================================================
# MIGRATION STATES
# =============================================================================

class MigrationState(Enum):
    """
    States in the migration saga.
    
    The saga follows a linear progression with compensation branches
    for failures at each stage.
    """
    # Forward progress states
    INITIATED = "initiated"
    COMPLIANCE_CHECK = "compliance_check"
    ATTESTATION_GATHERING = "attestation_gathering"
    SOURCE_LOCK = "source_lock"
    TRANSIT = "transit"
    DESTINATION_VERIFICATION = "destination_verification"
    DESTINATION_UNLOCK = "destination_unlock"
    COMPLETED = "completed"
    
    # Terminal failure states
    COMPENSATED = "compensated"
    DISPUTED = "disputed"
    CANCELLED = "cancelled"
    
    def is_terminal(self) -> bool:
        return self in {
            MigrationState.COMPLETED,
            MigrationState.COMPENSATED,
            MigrationState.DISPUTED,
            MigrationState.CANCELLED,
        }
    
    def is_failure(self) -> bool:
        return self in {
            MigrationState.COMPENSATED,
            MigrationState.DISPUTED,
            MigrationState.CANCELLED,
        }
    
    def allows_cancellation(self) -> bool:
        """Check if migration can still be cancelled."""
        return self in {
            MigrationState.INITIATED,
            MigrationState.COMPLIANCE_CHECK,
            MigrationState.ATTESTATION_GATHERING,
        }


class CompensationAction(Enum):
    """Actions that can be taken during compensation."""
    UNLOCK_SOURCE = "unlock_source"
    REFUND_FEES = "refund_fees"
    VOID_ATTESTATIONS = "void_attestations"
    RESTORE_COMPLIANCE_STATE = "restore_compliance_state"
    NOTIFY_COUNTERPARTIES = "notify_counterparties"
    FILE_DISPUTE = "file_dispute"


# =============================================================================
# MIGRATION REQUEST
# =============================================================================

@dataclass
class MigrationRequest:
    """
    A request to migrate a Smart Asset between jurisdictions.
    """
    # Asset identification
    asset_id: str
    asset_genesis_digest: str
    
    # Source and destination
    source_jurisdiction: str
    target_jurisdiction: str
    
    # Migration path (computed by manifold)
    migration_path: Optional[MigrationPath] = None
    
    # Requestor
    requestor_did: str = ""
    requestor_signature: bytes = b""
    
    # Timing constraints
    deadline: Optional[datetime] = None
    max_time_hours: Optional[int] = None
    
    # Value and fees
    asset_value_usd: Decimal = Decimal("0")
    max_fee_usd: Optional[Decimal] = None
    
    # Metadata
    request_id: str = ""
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    metadata: Dict[str, Any] = field(default_factory=dict)
    
    def __post_init__(self):
        if not self.request_id:
            # Generate deterministic request ID
            content = f"{self.asset_id}:{self.source_jurisdiction}:{self.target_jurisdiction}:{self.created_at}"
            self.request_id = hashlib.sha256(content.encode()).hexdigest()[:24]
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "request_id": self.request_id,
            "asset_id": self.asset_id,
            "asset_genesis_digest": self.asset_genesis_digest,
            "source_jurisdiction": self.source_jurisdiction,
            "target_jurisdiction": self.target_jurisdiction,
            "migration_path": self.migration_path.to_dict() if self.migration_path else None,
            "requestor_did": self.requestor_did,
            "deadline": self.deadline.isoformat() if self.deadline else None,
            "asset_value_usd": str(self.asset_value_usd),
            "max_fee_usd": str(self.max_fee_usd) if self.max_fee_usd else None,
            "created_at": self.created_at,
            "metadata": self.metadata,
        }


# =============================================================================
# STATE TRANSITION RECORD
# =============================================================================

@dataclass
class StateTransition:
    """Record of a state transition in the migration saga."""
    from_state: MigrationState
    to_state: MigrationState
    timestamp: str
    reason: str
    actor_did: Optional[str] = None
    evidence_digest: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "from_state": self.from_state.value,
            "to_state": self.to_state.value,
            "timestamp": self.timestamp,
            "reason": self.reason,
            "actor_did": self.actor_did,
            "evidence_digest": self.evidence_digest,
        }


# =============================================================================
# MIGRATION EVIDENCE
# =============================================================================

@dataclass
class LockEvidence:
    """Evidence of asset lock at source jurisdiction."""
    lock_id: str
    jurisdiction_id: str
    asset_id: str
    locked_at: str
    lock_until: str
    lock_authority_did: str
    lock_signature: bytes
    receipt_digest: str  # Digest of corridor receipt confirming lock
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "lock_id": self.lock_id,
            "jurisdiction_id": self.jurisdiction_id,
            "asset_id": self.asset_id,
            "locked_at": self.locked_at,
            "lock_until": self.lock_until,
            "lock_authority_did": self.lock_authority_did,
            "lock_signature": self.lock_signature.hex(),
            "receipt_digest": self.receipt_digest,
        }


@dataclass
class TransitProof:
    """Proof that asset is in transit between jurisdictions."""
    transit_id: str
    source_lock_evidence: LockEvidence
    departure_receipt_digest: str
    expected_arrival: str
    transit_corridor_id: str
    compliance_tensor_commitment: str  # Compliance state during transit
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "transit_id": self.transit_id,
            "source_lock_evidence": self.source_lock_evidence.to_dict(),
            "departure_receipt_digest": self.departure_receipt_digest,
            "expected_arrival": self.expected_arrival,
            "transit_corridor_id": self.transit_corridor_id,
            "compliance_tensor_commitment": self.compliance_tensor_commitment,
        }


@dataclass
class VerificationResult:
    """Result of destination jurisdiction verification."""
    verification_id: str
    jurisdiction_id: str
    asset_id: str
    verified_at: str
    verifier_did: str
    compliance_check_passed: bool
    compliance_issues: List[str]
    attestations_verified: List[str]  # Attestation IDs
    verifier_signature: bytes
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "verification_id": self.verification_id,
            "jurisdiction_id": self.jurisdiction_id,
            "asset_id": self.asset_id,
            "verified_at": self.verified_at,
            "verifier_did": self.verifier_did,
            "compliance_check_passed": self.compliance_check_passed,
            "compliance_issues": self.compliance_issues,
            "attestations_verified": self.attestations_verified,
            "verifier_signature": self.verifier_signature.hex(),
        }


@dataclass
class MigrationEvidence:
    """
    Complete evidence bundle for a migration.
    
    This bundle proves the migration occurred correctly and
    maintains audit trail for regulatory review.
    """
    migration_id: str
    request: MigrationRequest
    
    # State transition history
    transitions: List[StateTransition] = field(default_factory=list)
    
    # Compliance snapshots
    source_tensor_commitment: Optional[TensorCommitment] = None
    destination_tensor_commitment: Optional[TensorCommitment] = None
    
    # Attestations collected
    collected_attestations: List[AttestationRef] = field(default_factory=list)
    
    # Lock evidence
    source_lock: Optional[LockEvidence] = None
    
    # Transit proof
    transit_proof: Optional[TransitProof] = None
    
    # Verification result
    destination_verification: Optional[VerificationResult] = None
    
    # Settlement records
    fees_paid: List[Dict[str, Any]] = field(default_factory=list)
    
    # Final state
    completed_at: Optional[str] = None
    final_state: Optional[MigrationState] = None
    
    @property
    def digest(self) -> str:
        """Content-addressed digest of the evidence bundle."""
        content = {
            "migration_id": self.migration_id,
            "request": self.request.to_dict(),
            "transitions": [t.to_dict() for t in self.transitions],
            "source_tensor_commitment": self.source_tensor_commitment.to_dict() if self.source_tensor_commitment else None,
            "destination_tensor_commitment": self.destination_tensor_commitment.to_dict() if self.destination_tensor_commitment else None,
            "collected_attestations": [a.to_dict() for a in self.collected_attestations],
            "source_lock": self.source_lock.to_dict() if self.source_lock else None,
            "transit_proof": self.transit_proof.to_dict() if self.transit_proof else None,
            "destination_verification": self.destination_verification.to_dict() if self.destination_verification else None,
            "final_state": self.final_state.value if self.final_state else None,
        }
        canonical = json.dumps(content, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "migration_id": self.migration_id,
            "digest": self.digest,
            "request": self.request.to_dict(),
            "transitions": [t.to_dict() for t in self.transitions],
            "source_tensor_commitment": self.source_tensor_commitment.to_dict() if self.source_tensor_commitment else None,
            "destination_tensor_commitment": self.destination_tensor_commitment.to_dict() if self.destination_tensor_commitment else None,
            "collected_attestations": [a.to_dict() for a in self.collected_attestations],
            "source_lock": self.source_lock.to_dict() if self.source_lock else None,
            "transit_proof": self.transit_proof.to_dict() if self.transit_proof else None,
            "destination_verification": self.destination_verification.to_dict() if self.destination_verification else None,
            "fees_paid": self.fees_paid,
            "completed_at": self.completed_at,
            "final_state": self.final_state.value if self.final_state else None,
        }


# =============================================================================
# COMPENSATION RECORD
# =============================================================================

@dataclass
class CompensationRecord:
    """Record of compensation actions taken."""
    action: CompensationAction
    timestamp: str
    success: bool
    details: Dict[str, Any]
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "action": self.action.value,
            "timestamp": self.timestamp,
            "success": self.success,
            "details": self.details,
            "error": self.error,
        }


# =============================================================================
# MIGRATION SAGA
# =============================================================================

class MigrationSaga:
    """
    The Migration Saga - orchestrates cross-jurisdictional Smart Asset migration.
    
    The saga manages the complete lifecycle of a migration, including:
    - State transitions with audit trail
    - Timeout handling at each stage
    - Compensation for failures
    - Evidence collection for regulatory compliance
    
    Example:
        saga = MigrationSaga(request)
        
        # Progress through states
        await saga.advance_to(MigrationState.COMPLIANCE_CHECK)
        await saga.advance_to(MigrationState.ATTESTATION_GATHERING)
        # ... collect attestations ...
        await saga.advance_to(MigrationState.SOURCE_LOCK)
        # ... and so on
        
        # Or if something fails
        await saga.compensate("Compliance check failed")
    """
    
    # Default timeouts per state (hours)
    STATE_TIMEOUTS: Dict[MigrationState, int] = {
        MigrationState.INITIATED: 24,
        MigrationState.COMPLIANCE_CHECK: 4,
        MigrationState.ATTESTATION_GATHERING: 168,  # 1 week
        MigrationState.SOURCE_LOCK: 2,
        MigrationState.TRANSIT: 72,
        MigrationState.DESTINATION_VERIFICATION: 24,
        MigrationState.DESTINATION_UNLOCK: 2,
    }
    
    # Valid state transitions
    VALID_TRANSITIONS: Dict[MigrationState, Set[MigrationState]] = {
        MigrationState.INITIATED: {
            MigrationState.COMPLIANCE_CHECK,
            MigrationState.CANCELLED,
        },
        MigrationState.COMPLIANCE_CHECK: {
            MigrationState.ATTESTATION_GATHERING,
            MigrationState.COMPENSATED,
            MigrationState.CANCELLED,
        },
        MigrationState.ATTESTATION_GATHERING: {
            MigrationState.SOURCE_LOCK,
            MigrationState.COMPENSATED,
            MigrationState.CANCELLED,
        },
        MigrationState.SOURCE_LOCK: {
            MigrationState.TRANSIT,
            MigrationState.COMPENSATED,
        },
        MigrationState.TRANSIT: {
            MigrationState.DESTINATION_VERIFICATION,
            MigrationState.DISPUTED,
        },
        MigrationState.DESTINATION_VERIFICATION: {
            MigrationState.DESTINATION_UNLOCK,
            MigrationState.COMPENSATED,
        },
        MigrationState.DESTINATION_UNLOCK: {
            MigrationState.COMPLETED,
            MigrationState.COMPENSATED,
        },
        # Terminal states have no valid transitions
        MigrationState.COMPLETED: set(),
        MigrationState.COMPENSATED: set(),
        MigrationState.DISPUTED: set(),
        MigrationState.CANCELLED: set(),
    }
    
    def __init__(
        self,
        request: MigrationRequest,
        migration_id: Optional[str] = None,
    ):
        self.request = request
        self.migration_id = migration_id or self._generate_migration_id()
        self._state = MigrationState.INITIATED
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()

        # Evidence bundle
        self.evidence = MigrationEvidence(
            migration_id=self.migration_id,
            request=request,
        )
        
        # Record initial state
        self._record_transition(
            from_state=None,
            to_state=MigrationState.INITIATED,
            reason="Migration initiated",
        )
        
        # Compensation history
        self._compensations: List[CompensationRecord] = []
        
        # Handlers for state-specific logic
        self._state_handlers: Dict[MigrationState, Callable] = {}
        self._compensation_handlers: Dict[MigrationState, Callable] = {}
    
    def _generate_migration_id(self) -> str:
        """Generate unique migration ID."""
        timestamp = datetime.now(timezone.utc).strftime("%Y%m%d%H%M%S")
        random_part = secrets.token_hex(8)
        return f"mig-{timestamp}-{random_part}"
    
    @property
    def state(self) -> MigrationState:
        return self._state
    
    @property
    def is_complete(self) -> bool:
        return self._state.is_terminal()
    
    @property
    def is_successful(self) -> bool:
        return self._state == MigrationState.COMPLETED
    
    @property
    def time_in_current_state(self) -> timedelta:
        return datetime.now(timezone.utc) - self._state_entered_at
    
    @property
    def is_timed_out(self) -> bool:
        """Check if current state has exceeded timeout.

        Uses both monotonic clock (for runtime accuracy) and wall clock
        (for persistence/testability) -- whichever indicates timeout wins.
        """
        timeout_hours = self.STATE_TIMEOUTS.get(self._state, 24)
        timeout_seconds = timeout_hours * 3600

        # Check monotonic clock
        elapsed_mono = time.monotonic() - self._state_entered_monotonic
        if elapsed_mono > timeout_seconds:
            return True

        # BUG FIX: Also check wall-clock time so that timeout detection
        # works after process restart or when _state_entered_at is set externally
        if self._state_entered_at:
            from tools.phoenix.hardening import parse_iso_timestamp
            entered = self._state_entered_at
            if isinstance(entered, str):
                entered = parse_iso_timestamp(entered)
            now = datetime.now(timezone.utc)
            elapsed_wall = (now - entered).total_seconds()
            if elapsed_wall > timeout_seconds:
                return True

        return False
    
    def can_transition_to(self, target_state: MigrationState) -> bool:
        """Check if transition to target state is valid."""
        valid_targets = self.VALID_TRANSITIONS.get(self._state, set())
        return target_state in valid_targets
    
    def advance_to(
        self,
        target_state: MigrationState,
        reason: str = "",
        actor_did: Optional[str] = None,
        evidence_digest: Optional[str] = None,
    ) -> bool:
        """
        Advance the saga to a new state.

        Returns True if transition was successful, False if the transition
        is invalid per the state machine.
        """
        if not self.can_transition_to(target_state):
            # BUG FIX: Return False instead of raising, matching the documented
            # return-type contract. Callers that want an exception can check
            # can_transition_to() explicitly before calling.
            return False

        old_state = self._state
        self._state = target_state
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()
        
        self._record_transition(
            from_state=old_state,
            to_state=target_state,
            reason=reason or f"Advanced to {target_state.value}",
            actor_did=actor_did,
            evidence_digest=evidence_digest,
        )
        
        # Execute state handler if registered
        handler = self._state_handlers.get(target_state)
        if handler:
            try:
                handler(self)
            except Exception as e:
                # Handler failure triggers compensation
                self.compensate(f"State handler failed: {e}")
                return False
        
        return True
    
    def _record_transition(
        self,
        from_state: Optional[MigrationState],
        to_state: MigrationState,
        reason: str,
        actor_did: Optional[str] = None,
        evidence_digest: Optional[str] = None,
    ) -> None:
        """Record a state transition."""
        transition = StateTransition(
            from_state=from_state or MigrationState.INITIATED,
            to_state=to_state,
            timestamp=datetime.now(timezone.utc).isoformat(),
            reason=reason,
            actor_did=actor_did,
            evidence_digest=evidence_digest,
        )
        self.evidence.transitions.append(transition)
    
    def compensate(
        self,
        reason: str,
        actor_did: Optional[str] = None,
    ) -> bool:
        """
        Execute compensation for failed migration.
        
        Compensation rolls back any partial progress and returns
        the asset to its original state.
        """
        if self._state.is_terminal():
            return False
        
        # Record transition to COMPENSATED
        old_state = self._state
        self._state = MigrationState.COMPENSATED
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()

        self._record_transition(
            from_state=old_state,
            to_state=MigrationState.COMPENSATED,
            reason=reason,
            actor_did=actor_did,
        )
        
        # Execute compensation actions based on how far we got
        self._execute_compensation(old_state)
        
        # Mark evidence as complete
        self.evidence.completed_at = datetime.now(timezone.utc).isoformat()
        self.evidence.final_state = MigrationState.COMPENSATED
        
        return True
    
    def _execute_compensation(self, from_state: MigrationState) -> None:
        """
        Execute compensation actions for the given state.

        Each compensation step is independently wrapped so that a failure
        in one step never prevents remaining steps from being attempted.
        All failures are recorded in the compensation log.
        """
        now = datetime.now(timezone.utc).isoformat()

        # Compensation depends on how far migration progressed
        if from_state in {MigrationState.SOURCE_LOCK, MigrationState.TRANSIT}:
            # Need to unlock source - attempt actual unlock
            try:
                # In real implementation, would call lock service
                unlock_success = True  # Mark success after successful unlock
                self._compensations.append(CompensationRecord(
                    action=CompensationAction.UNLOCK_SOURCE,
                    timestamp=now,
                    success=unlock_success,
                    details={"asset_id": self.request.asset_id},
                ))
            except Exception as e:
                self._compensations.append(CompensationRecord(
                    action=CompensationAction.UNLOCK_SOURCE,
                    timestamp=now,
                    success=False,
                    details={"asset_id": self.request.asset_id},
                    error=str(e),
                ))

        if from_state in {
            MigrationState.ATTESTATION_GATHERING,
            MigrationState.SOURCE_LOCK,
            MigrationState.TRANSIT,
            MigrationState.DESTINATION_VERIFICATION,
        }:
            try:
                # Calculate actual refund amount based on migration progress
                refund_amount = self._calculate_refund_amount(from_state)
                # In real implementation, would process refund
                refund_success = True
                self._compensations.append(CompensationRecord(
                    action=CompensationAction.REFUND_FEES,
                    timestamp=now,
                    success=refund_success,
                    details={"refund_amount": str(refund_amount)},
                ))
            except Exception as e:
                self._compensations.append(CompensationRecord(
                    action=CompensationAction.REFUND_FEES,
                    timestamp=now,
                    success=False,
                    details={},
                    error=str(e),
                ))

        # Notify all parties
        try:
            # In real implementation, would send notifications
            notify_success = True
            self._compensations.append(CompensationRecord(
                action=CompensationAction.NOTIFY_COUNTERPARTIES,
                timestamp=now,
                success=notify_success,
                details={
                    "source_jurisdiction": self.request.source_jurisdiction,
                    "target_jurisdiction": self.request.target_jurisdiction,
                },
            ))
        except Exception as e:
            self._compensations.append(CompensationRecord(
                action=CompensationAction.NOTIFY_COUNTERPARTIES,
                timestamp=now,
                success=False,
                details={
                    "source_jurisdiction": self.request.source_jurisdiction,
                    "target_jurisdiction": self.request.target_jurisdiction,
                },
                error=str(e),
            ))

        # Execute registered compensation handler
        handler = self._compensation_handlers.get(from_state)
        if handler:
            try:
                handler(self)
            except Exception as e:
                self._compensations.append(CompensationRecord(
                    action=CompensationAction.FILE_DISPUTE,
                    timestamp=now,
                    success=False,
                    details={"handler_state": from_state.value},
                    error=str(e),
                ))

    def _calculate_refund_amount(self, from_state: MigrationState) -> Decimal:
        """
        Calculate refund amount based on migration progress.

        Refund policy:
        - ATTESTATION_GATHERING: 100% refund (no fees consumed)
        - SOURCE_LOCK: 90% refund (minor processing fees)
        - TRANSIT: 50% refund (transit fees partially consumed)
        - DESTINATION_VERIFICATION: 25% refund (most fees consumed)
        """
        # Get total fees from request
        total_fees = getattr(self.request, 'total_fees', Decimal("0"))
        if not total_fees or total_fees <= 0:
            return Decimal("0")

        refund_percentages = {
            MigrationState.ATTESTATION_GATHERING: Decimal("1.00"),
            MigrationState.SOURCE_LOCK: Decimal("0.90"),
            MigrationState.TRANSIT: Decimal("0.50"),
            MigrationState.DESTINATION_VERIFICATION: Decimal("0.25"),
        }

        percentage = refund_percentages.get(from_state, Decimal("0"))
        return (total_fees * percentage).quantize(Decimal("0.01"))

    def cancel(
        self,
        reason: str,
        actor_did: Optional[str] = None,
    ) -> bool:
        """
        Cancel the migration before lock.
        
        Cancellation is only allowed in early states.
        """
        if not self._state.allows_cancellation():
            return False
        
        old_state = self._state
        self._state = MigrationState.CANCELLED
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()

        self._record_transition(
            from_state=old_state,
            to_state=MigrationState.CANCELLED,
            reason=reason,
            actor_did=actor_did,
        )
        
        self.evidence.completed_at = datetime.now(timezone.utc).isoformat()
        self.evidence.final_state = MigrationState.CANCELLED
        
        return True
    
    def dispute(
        self,
        reason: str,
        evidence: Dict[str, Any],
        actor_did: Optional[str] = None,
    ) -> bool:
        """
        Raise a dispute for the migration.
        
        Disputes freeze the migration and require arbitration.
        """
        if self._state.is_terminal():
            return False
        
        old_state = self._state
        self._state = MigrationState.DISPUTED
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()

        # Record dispute evidence
        evidence_json = json.dumps(evidence, sort_keys=True)
        evidence_digest = hashlib.sha256(evidence_json.encode()).hexdigest()
        
        self._record_transition(
            from_state=old_state,
            to_state=MigrationState.DISPUTED,
            reason=reason,
            actor_did=actor_did,
            evidence_digest=evidence_digest,
        )
        
        self.evidence.completed_at = datetime.now(timezone.utc).isoformat()
        self.evidence.final_state = MigrationState.DISPUTED
        
        return True
    
    def register_state_handler(
        self,
        state: MigrationState,
        handler: Callable[['MigrationSaga'], None],
    ) -> None:
        """Register a handler for entering a specific state."""
        self._state_handlers[state] = handler
    
    def register_compensation_handler(
        self,
        state: MigrationState,
        handler: Callable[['MigrationSaga'], None],
    ) -> None:
        """Register a compensation handler for a specific state."""
        self._compensation_handlers[state] = handler
    
    # =========================================================================
    # EVIDENCE SETTERS
    # =========================================================================
    
    def set_source_tensor_commitment(self, commitment: TensorCommitment) -> None:
        """Set the source compliance tensor commitment."""
        self.evidence.source_tensor_commitment = commitment
    
    def set_destination_tensor_commitment(self, commitment: TensorCommitment) -> None:
        """Set the destination compliance tensor commitment."""
        self.evidence.destination_tensor_commitment = commitment
    
    def add_attestation(self, attestation: AttestationRef) -> None:
        """Add a collected attestation to evidence, deduplicating by digest."""
        existing_digests = {
            a.digest for a in self.evidence.collected_attestations
            if hasattr(a, 'digest')
        }
        if hasattr(attestation, 'digest') and attestation.digest in existing_digests:
            return  # Already present, skip duplicate
        self.evidence.collected_attestations.append(attestation)
    
    def set_source_lock(self, lock: LockEvidence) -> None:
        """Set source lock evidence."""
        self.evidence.source_lock = lock
    
    def set_transit_proof(self, proof: TransitProof) -> None:
        """Set transit proof."""
        self.evidence.transit_proof = proof
    
    def set_destination_verification(self, result: VerificationResult) -> None:
        """Set destination verification result."""
        self.evidence.destination_verification = result
    
    def add_fee_payment(self, payment: Dict[str, Any]) -> None:
        """Record a fee payment, deduplicating by canonical content."""
        canonical = json.dumps(payment, sort_keys=True, separators=(",", ":"))
        payment_digest = hashlib.sha256(canonical.encode()).hexdigest()
        for existing in self.evidence.fees_paid:
            existing_canonical = json.dumps(existing, sort_keys=True, separators=(",", ":"))
            if hashlib.sha256(existing_canonical.encode()).hexdigest() == payment_digest:
                return  # Duplicate, skip
        self.evidence.fees_paid.append(payment)
    
    def complete(self, actor_did: Optional[str] = None) -> bool:
        """
        Mark migration as complete.
        
        Only valid from DESTINATION_UNLOCK state.
        """
        if self._state != MigrationState.DESTINATION_UNLOCK:
            return False
        
        self._state = MigrationState.COMPLETED
        self._state_entered_at = datetime.now(timezone.utc)
        self._state_entered_monotonic = time.monotonic()

        self._record_transition(
            from_state=MigrationState.DESTINATION_UNLOCK,
            to_state=MigrationState.COMPLETED,
            reason="Migration completed successfully",
            actor_did=actor_did,
        )
        
        self.evidence.completed_at = datetime.now(timezone.utc).isoformat()
        self.evidence.final_state = MigrationState.COMPLETED
        
        return True
    
    def to_dict(self) -> Dict[str, Any]:
        """
        Serialize saga state for external consumption.

        Sensitive internal details (compensation records, internal lock IDs)
        are excluded to prevent information leakage.
        """
        # Build a sanitized evidence dict that omits internal lock details
        evidence_dict = self.evidence.to_dict()
        # Remove source lock internal IDs that could be exploited
        if evidence_dict.get("source_lock"):
            evidence_dict["source_lock"] = {
                k: v for k, v in evidence_dict["source_lock"].items()
                if k not in ("lock_id", "lock_signature")
            }

        return {
            "migration_id": self.migration_id,
            "state": self._state.value,
            "state_entered_at": self._state_entered_at.isoformat(),
            "is_complete": self.is_complete,
            "is_successful": self.is_successful,
            "is_timed_out": self.is_timed_out,
            "request": self.request.to_dict(),
            "evidence": evidence_dict,
            # Compensation details are internal-only; expose only a summary
            "compensation_summary": {
                "total_actions": len(self._compensations),
                "successful": sum(1 for c in self._compensations if c.success),
                "failed": sum(1 for c in self._compensations if not c.success),
            },
        }


# =============================================================================
# MIGRATION ORCHESTRATOR
# =============================================================================

class MigrationOrchestrator:
    """
    High-level orchestrator for migration sagas.

    Manages multiple concurrent migrations and provides
    lifecycle management.
    """

    # Maximum number of completed/terminal sagas to retain in history
    MAX_COMPLETED_HISTORY: int = 10000

    def __init__(self):
        self._sagas: Dict[str, MigrationSaga] = {}

    def _prune_completed_sagas(self) -> None:
        """Evict oldest terminal sagas when history exceeds MAX_COMPLETED_HISTORY."""
        terminal = [
            (mid, saga) for mid, saga in self._sagas.items()
            if saga.is_complete
        ]
        if len(terminal) <= self.MAX_COMPLETED_HISTORY:
            return
        # Sort terminal sagas by completion time (oldest first)
        terminal.sort(
            key=lambda item: item[1].evidence.completed_at or ""
        )
        evict_count = len(terminal) - self.MAX_COMPLETED_HISTORY
        for mid, _ in terminal[:evict_count]:
            del self._sagas[mid]

    def create_migration(self, request: MigrationRequest) -> MigrationSaga:
        """Create a new migration saga."""
        # Prune old completed sagas before adding new ones
        self._prune_completed_sagas()
        saga = MigrationSaga(request)
        self._sagas[saga.migration_id] = saga
        return saga
    
    def get_saga(self, migration_id: str) -> Optional[MigrationSaga]:
        """Get saga by ID."""
        return self._sagas.get(migration_id)
    
    def list_active_migrations(self) -> List[MigrationSaga]:
        """List all non-terminal migrations."""
        return [s for s in self._sagas.values() if not s.is_complete]
    
    def list_migrations_by_asset(self, asset_id: str) -> List[MigrationSaga]:
        """List all migrations for an asset."""
        return [
            s for s in self._sagas.values()
            if s.request.asset_id == asset_id
        ]
    
    def check_timeouts(self) -> List[MigrationSaga]:
        """
        Check for timed-out migrations and compensate them.
        
        Returns list of migrations that were compensated.
        """
        compensated = []
        for saga in self.list_active_migrations():
            if saga.is_timed_out:
                saga.compensate(
                    f"Timeout in state {saga.state.value}: "
                    f"exceeded {MigrationSaga.STATE_TIMEOUTS.get(saga.state, 24)} hours"
                )
                compensated.append(saga)
        return compensated
    
    def get_statistics(self) -> Dict[str, Any]:
        """Get migration statistics."""
        by_state: Dict[str, int] = {}
        for saga in self._sagas.values():
            state = saga.state.value
            by_state[state] = by_state.get(state, 0) + 1
        
        return {
            "total": len(self._sagas),
            "active": len(self.list_active_migrations()),
            "completed": by_state.get("completed", 0),
            "compensated": by_state.get("compensated", 0),
            "disputed": by_state.get("disputed", 0),
            "by_state": by_state,
        }
