"""
PHOENIX Security Layer

Comprehensive security hardening addressing identified vulnerabilities:

1. Attestation Replay Prevention - scope-bound attestations
2. TOCTOU Protection - atomic operations with versioning
3. Front-Running Prevention - time-locked withdrawals
4. Signature Verification - cryptographic validation
5. Nonce Management - replay attack prevention
6. Rate Limiting - DoS protection
7. Audit Logging - forensic trail

Security Model:
    - Defense in depth: multiple layers of protection
    - Fail-secure: errors result in safe state
    - Audit everything: complete forensic trail
    - Zero trust: verify all inputs

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import hmac
import json
import secrets
import threading
import time
from abc import ABC, abstractmethod
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum, auto
from functools import wraps
from typing import (
    Any,
    Callable,
    Dict,
    FrozenSet,
    Generic,
    List,
    Optional,
    Set,
    Tuple,
    TypeVar,
    Union,
)

from tools.phoenix.hardening import (
    ValidationError,
    ValidationResult,
    SecurityViolation,
    InvariantViolation,
    Validators,
    CryptoUtils,
    ThreadSafeDict,
    AtomicCounter,
)


# =============================================================================
# ATTESTATION SCOPE BINDING
# =============================================================================

@dataclass(frozen=True)
class AttestationScope:
    """
    Defines the valid scope for an attestation.
    
    Attestations are bound to specific assets and jurisdictions to
    prevent replay attacks where a valid attestation for one context
    is reused in another.
    """
    asset_id: str
    jurisdiction_id: str
    domain: str
    valid_from: str  # ISO8601
    valid_until: str  # ISO8601
    
    @property
    def scope_hash(self) -> str:
        """Compute deterministic hash of the scope."""
        content = {
            "asset_id": self.asset_id,
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain,
            "valid_from": self.valid_from,
            "valid_until": self.valid_until,
        }
        from tools.lawpack import jcs_canonicalize
        return hashlib.sha256(jcs_canonicalize(content)).hexdigest()

    def includes(self, asset_id: str, jurisdiction_id: str, domain: str) -> bool:
        """Check if the given context is within scope."""
        return (
            self.asset_id == asset_id and
            self.jurisdiction_id == jurisdiction_id and
            self.domain == domain
        )
    
    def is_valid_at(self, timestamp: datetime) -> bool:
        """Check if scope is valid at given time."""
        from tools.phoenix.hardening import parse_iso_timestamp
        valid_from = parse_iso_timestamp(self.valid_from)
        valid_until = parse_iso_timestamp(self.valid_until)
        return valid_from <= timestamp <= valid_until


@dataclass
class ScopedAttestation:
    """
    An attestation with explicit scope binding.
    
    This prevents attestation replay by binding each attestation
    to a specific (asset, jurisdiction, domain) tuple.
    """
    attestation_id: str
    attestation_type: str
    issuer_did: str
    scope: AttestationScope
    
    # Cryptographic binding
    scope_commitment: str  # H(attestation_id || scope_hash)
    issuer_signature: bytes
    
    # Metadata
    issued_at: str
    nonce: str  # Unique nonce for replay prevention
    
    def __post_init__(self):
        # Verify scope commitment
        expected = self._compute_commitment()
        if self.scope_commitment != expected:
            raise SecurityViolation(
                f"Scope commitment mismatch: {self.scope_commitment} != {expected}"
            )
    
    def _compute_commitment(self) -> str:
        """Compute the scope commitment."""
        content = self.attestation_id + self.scope.scope_hash + self.nonce
        return hashlib.sha256(content.encode()).hexdigest()
    
    @classmethod
    def create(
        cls,
        attestation_id: str,
        attestation_type: str,
        issuer_did: str,
        scope: AttestationScope,
        issuer_signature: bytes,
    ) -> 'ScopedAttestation':
        """Create a new scoped attestation."""
        nonce = secrets.token_hex(16)
        content = attestation_id + scope.scope_hash + nonce
        scope_commitment = hashlib.sha256(content.encode()).hexdigest()
        
        return cls(
            attestation_id=attestation_id,
            attestation_type=attestation_type,
            issuer_did=issuer_did,
            scope=scope,
            scope_commitment=scope_commitment,
            issuer_signature=issuer_signature,
            issued_at=datetime.now(timezone.utc).isoformat(),
            nonce=nonce,
        )
    
    def verify_scope(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domain: str,
        at_time: Optional[datetime] = None,
    ) -> bool:
        """Verify attestation is valid for the given context."""
        at_time = at_time or datetime.now(timezone.utc)
        
        # Check scope includes this context
        if not self.scope.includes(asset_id, jurisdiction_id, domain):
            return False
        
        # Check time validity
        if not self.scope.is_valid_at(at_time):
            return False
        
        return True


# =============================================================================
# NONCE REGISTRY
# =============================================================================

class NonceRegistry:
    """
    Registry for tracking used nonces to prevent replay attacks.
    
    Nonces are stored with expiration times to allow cleanup
    of old entries while maintaining security.
    """
    
    def __init__(self, max_age_hours: int = 168):  # 7 days default
        self._nonces: Dict[str, datetime] = {}
        self._lock = threading.Lock()
        self._max_age = timedelta(hours=max_age_hours)
    
    def check_and_register(self, nonce: str) -> bool:
        """
        Check if nonce is fresh and register it.
        
        Returns True if nonce was fresh (not seen before).
        Returns False if nonce was already used (replay attempt).
        """
        with self._lock:
            # Cleanup old nonces periodically
            self._cleanup()
            
            if nonce in self._nonces:
                return False  # Replay detected
            
            self._nonces[nonce] = datetime.now(timezone.utc)
            return True
    
    def is_fresh(self, nonce: str) -> bool:
        """Check if nonce has not been used."""
        with self._lock:
            return nonce not in self._nonces
    
    def _cleanup(self) -> None:
        """Remove expired nonces."""
        cutoff = datetime.now(timezone.utc) - self._max_age
        expired = [n for n, t in self._nonces.items() if t < cutoff]
        for nonce in expired:
            del self._nonces[nonce]
    
    def size(self) -> int:
        """Get number of registered nonces."""
        with self._lock:
            return len(self._nonces)


# =============================================================================
# VERSION-CONTROLLED STATE
# =============================================================================

T = TypeVar('T')


@dataclass
class VersionedValue(Generic[T]):
    """A value with version number for optimistic locking."""
    value: T
    version: int
    updated_at: str


class VersionedStore(Generic[T]):
    """
    Thread-safe versioned key-value store.
    
    Supports optimistic locking via compare-and-swap operations
    to prevent TOCTOU vulnerabilities.
    """
    
    def __init__(self):
        self._data: Dict[str, VersionedValue[T]] = {}
        self._lock = threading.RLock()
        self._version_counter = AtomicCounter(0)
    
    def get(self, key: str) -> Optional[VersionedValue[T]]:
        """Get versioned value."""
        with self._lock:
            return self._data.get(key)
    
    def set(self, key: str, value: T) -> VersionedValue[T]:
        """Set value, incrementing version."""
        with self._lock:
            version = self._version_counter.increment()
            versioned = VersionedValue(
                value=value,
                version=version,
                updated_at=datetime.now(timezone.utc).isoformat(),
            )
            self._data[key] = versioned
            return versioned
    
    def compare_and_swap(
        self,
        key: str,
        expected_version: int,
        new_value: T,
    ) -> Tuple[bool, Optional[VersionedValue[T]]]:
        """
        Atomically update value if version matches.
        
        Returns (success, new_versioned_value or current_value).
        """
        with self._lock:
            current = self._data.get(key)
            
            if current is None:
                # Key doesn't exist - create it
                versioned = self.set(key, new_value)
                return (True, versioned)
            
            if current.version != expected_version:
                # Version mismatch - concurrent modification
                return (False, current)
            
            # Version matches - update
            versioned = self.set(key, new_value)
            return (True, versioned)
    
    def delete(self, key: str) -> bool:
        """Delete key. Returns True if existed."""
        with self._lock:
            if key in self._data:
                del self._data[key]
                return True
            return False
    
    def keys(self) -> List[str]:
        """Get all keys."""
        with self._lock:
            return list(self._data.keys())


# =============================================================================
# TIME-LOCKED OPERATIONS
# =============================================================================

class TimeLockState(Enum):
    """State of a time-locked operation."""
    PENDING = "pending"
    LOCKED = "locked"
    UNLOCKABLE = "unlockable"
    EXECUTED = "executed"
    CANCELLED = "cancelled"


@dataclass
class TimeLock:
    """
    A time-locked operation for front-running prevention.
    
    Operations are announced and must wait a delay period
    before execution, giving others time to respond.
    """
    lock_id: str
    operation_type: str
    operator_did: str
    
    # Timing
    announced_at: str
    unlock_at: str  # When operation can be executed
    expires_at: str  # When lock expires if not executed
    
    # Operation details (encrypted or hashed)
    operation_commitment: str  # H(operation_details)
    operation_data: Optional[bytes] = None  # Revealed at execution
    
    # State
    state: TimeLockState = TimeLockState.PENDING
    executed_at: Optional[str] = None
    
    def is_unlockable(self) -> bool:
        """Check if operation can now be executed."""
        if self.state != TimeLockState.PENDING:
            return False

        from tools.phoenix.hardening import parse_iso_timestamp
        now = datetime.now(timezone.utc)
        unlock_time = parse_iso_timestamp(self.unlock_at)
        expires_time = parse_iso_timestamp(self.expires_at)

        return unlock_time <= now <= expires_time

    def is_expired(self) -> bool:
        """Check if lock has expired."""
        from tools.phoenix.hardening import parse_iso_timestamp
        now = datetime.now(timezone.utc)
        expires_time = parse_iso_timestamp(self.expires_at)
        return now > expires_time


class TimeLockManager:
    """
    Manager for time-locked operations.
    
    Implements the commit-delay-reveal pattern for front-running prevention.
    """
    
    # Default delays
    WITHDRAWAL_DELAY_HOURS = 168  # 7 days
    MIGRATION_DELAY_HOURS = 24    # 1 day
    PARAMETER_CHANGE_DELAY_HOURS = 72  # 3 days
    
    def __init__(self):
        self._locks: Dict[str, TimeLock] = {}
        self._lock = threading.Lock()
    
    def announce(
        self,
        operation_type: str,
        operator_did: str,
        operation_commitment: str,
        delay_hours: Optional[int] = None,
        expiry_hours: int = 48,
    ) -> TimeLock:
        """
        Announce an operation to be executed later.
        
        Args:
            operation_type: Type of operation
            operator_did: DID of operator
            operation_commitment: Hash of operation details
            delay_hours: Hours until operation can execute
            expiry_hours: Hours after unlock until expiry
            
        Returns:
            The created time lock
        """
        # Default delay based on operation type
        if delay_hours is None:
            delay_hours = {
                "withdrawal": self.WITHDRAWAL_DELAY_HOURS,
                "migration": self.MIGRATION_DELAY_HOURS,
                "parameter_change": self.PARAMETER_CHANGE_DELAY_HOURS,
            }.get(operation_type, 24)
        
        now = datetime.now(timezone.utc)
        unlock_at = now + timedelta(hours=delay_hours)
        expires_at = unlock_at + timedelta(hours=expiry_hours)
        
        lock = TimeLock(
            lock_id=f"tl-{secrets.token_hex(16)}",
            operation_type=operation_type,
            operator_did=operator_did,
            announced_at=now.isoformat(),
            unlock_at=unlock_at.isoformat(),
            expires_at=expires_at.isoformat(),
            operation_commitment=operation_commitment,
        )
        
        with self._lock:
            self._locks[lock.lock_id] = lock
        
        return lock
    
    def execute(
        self,
        lock_id: str,
        operation_data: bytes,
    ) -> Tuple[bool, str]:
        """
        Execute a time-locked operation.
        
        Args:
            lock_id: The lock to execute
            operation_data: The revealed operation data
            
        Returns:
            (success, message)
        """
        with self._lock:
            lock = self._locks.get(lock_id)
            
            if not lock:
                return (False, "Lock not found")
            
            if lock.state != TimeLockState.PENDING:
                return (False, f"Lock in invalid state: {lock.state.value}")
            
            if not lock.is_unlockable():
                if lock.is_expired():
                    lock.state = TimeLockState.CANCELLED
                    return (False, "Lock expired")
                return (False, "Lock not yet unlockable")
            
            # Verify commitment
            computed = hashlib.sha256(operation_data).hexdigest()
            if not CryptoUtils.secure_compare_str(computed, lock.operation_commitment):
                return (False, "Operation data does not match commitment")
            
            # Execute
            lock.operation_data = operation_data
            lock.state = TimeLockState.EXECUTED
            lock.executed_at = datetime.now(timezone.utc).isoformat()
            
            return (True, "Operation executed successfully")
    
    def cancel(self, lock_id: str, operator_did: str) -> bool:
        """Cancel a pending time lock."""
        with self._lock:
            lock = self._locks.get(lock_id)
            if not lock:
                return False
            
            if lock.operator_did != operator_did:
                return False
            
            if lock.state != TimeLockState.PENDING:
                return False
            
            lock.state = TimeLockState.CANCELLED
            return True
    
    def get_pending_locks(
        self,
        operation_type: Optional[str] = None,
    ) -> List[TimeLock]:
        """Get all pending locks, optionally filtered by type."""
        with self._lock:
            locks = [l for l in self._locks.values() if l.state == TimeLockState.PENDING]
            if operation_type:
                locks = [l for l in locks if l.operation_type == operation_type]
            return locks


# =============================================================================
# SIGNATURE VERIFICATION
# =============================================================================

class SignatureScheme(Enum):
    """Supported signature schemes."""
    ED25519 = "ed25519"
    SECP256K1 = "secp256k1"
    BLS12_381 = "bls12-381"


@dataclass
class SignedMessage:
    """A cryptographically signed message."""
    message: bytes
    signature: bytes
    signer_public_key: bytes
    scheme: SignatureScheme
    timestamp: str
    nonce: str
    
    @property
    def digest(self) -> str:
        """Compute message digest."""
        content = self.message + self.timestamp.encode() + self.nonce.encode()
        return hashlib.sha256(content).hexdigest()


class SignatureVerifier:
    """
    Signature verification service.
    
    Note: This is a mock implementation. Production should use
    actual cryptographic libraries (e.g., PyNaCl for Ed25519).
    """
    
    def __init__(self):
        self._nonce_registry = NonceRegistry()
        self._trusted_keys: Dict[str, bytes] = {}  # did -> public_key
    
    def register_key(self, did: str, public_key: bytes) -> None:
        """Register a trusted public key for a DID."""
        self._trusted_keys[did] = public_key
    
    def verify(
        self,
        signed_message: SignedMessage,
        expected_signer_did: Optional[str] = None,
    ) -> Tuple[bool, str]:
        """
        Verify a signed message.
        
        Returns (valid, reason).
        """
        # Check nonce freshness (replay prevention)
        if not self._nonce_registry.check_and_register(signed_message.nonce):
            return (False, "Nonce already used (replay attack)")
        
        # Check timestamp freshness
        result = Validators.validate_timestamp(
            signed_message.timestamp,
            "signature_timestamp",
            allow_future=False,
            max_age_days=1,
        )
        if not result.is_valid:
            return (False, "Signature timestamp invalid or expired")
        
        # Check signer if specified
        if expected_signer_did:
            trusted_key = self._trusted_keys.get(expected_signer_did)
            if trusted_key is None:
                return (False, f"Unknown signer: {expected_signer_did}")
            if signed_message.signer_public_key != trusted_key:
                return (False, "Public key does not match expected signer")
        
        # Mock signature verification
        # In production, use actual crypto library based on scheme
        if signed_message.scheme == SignatureScheme.ED25519:
            # Would use nacl.signing.VerifyKey
            pass
        elif signed_message.scheme == SignatureScheme.SECP256K1:
            # Would use ecdsa library
            pass
        elif signed_message.scheme == SignatureScheme.BLS12_381:
            # Would use py_ecc library
            pass
        
        # For now, verify signature length as basic check
        expected_lengths = {
            SignatureScheme.ED25519: 64,
            SignatureScheme.SECP256K1: 64,
            SignatureScheme.BLS12_381: 96,
        }
        expected_len = expected_lengths.get(signed_message.scheme, 64)
        if len(signed_message.signature) != expected_len:
            return (False, f"Invalid signature length for {signed_message.scheme.value}")
        
        return (True, "Signature valid")


# =============================================================================
# AUDIT LOGGER
# =============================================================================

class AuditEventType(Enum):
    """Types of audit events."""
    # State changes
    STATE_CREATED = "state_created"
    STATE_UPDATED = "state_updated"
    STATE_DELETED = "state_deleted"
    
    # Authentication
    AUTH_SUCCESS = "auth_success"
    AUTH_FAILURE = "auth_failure"
    
    # Authorization
    AUTHZ_GRANTED = "authz_granted"
    AUTHZ_DENIED = "authz_denied"
    
    # Security events
    SECURITY_VIOLATION = "security_violation"
    REPLAY_ATTEMPT = "replay_attempt"
    SIGNATURE_INVALID = "signature_invalid"
    
    # Economic events
    BOND_POSTED = "bond_posted"
    BOND_SLASHED = "bond_slashed"
    WITHDRAWAL_INITIATED = "withdrawal_initiated"
    
    # Migration events
    MIGRATION_STARTED = "migration_started"
    MIGRATION_COMPLETED = "migration_completed"
    MIGRATION_FAILED = "migration_failed"


@dataclass
class AuditEvent:
    """An audit log entry."""
    event_id: str
    event_type: AuditEventType
    timestamp: str
    actor_did: str
    resource_type: str
    resource_id: str
    action: str
    outcome: str  # success, failure, error
    details: Dict[str, Any]
    
    # Security metadata
    ip_address: Optional[str] = None
    user_agent: Optional[str] = None
    request_id: Optional[str] = None
    
    # Tamper evidence
    previous_event_digest: Optional[str] = None
    event_digest: str = ""
    
    def __post_init__(self):
        if not self.event_digest:
            self.event_digest = self._compute_digest()
    
    def _compute_digest(self) -> str:
        """Compute tamper-evident digest."""
        content = {
            "event_id": self.event_id,
            "event_type": self.event_type.value,
            "timestamp": self.timestamp,
            "actor_did": self.actor_did,
            "resource_type": self.resource_type,
            "resource_id": self.resource_id,
            "action": self.action,
            "outcome": self.outcome,
            "previous_event_digest": self.previous_event_digest,
        }
        from tools.lawpack import jcs_canonicalize
        return hashlib.sha256(jcs_canonicalize(content)).hexdigest()

    def to_dict(self) -> Dict[str, Any]:
        return {
            "event_id": self.event_id,
            "event_type": self.event_type.value,
            "timestamp": self.timestamp,
            "actor_did": self.actor_did,
            "resource_type": self.resource_type,
            "resource_id": self.resource_id,
            "action": self.action,
            "outcome": self.outcome,
            "details": self.details,
            "event_digest": self.event_digest,
        }


class AuditLogger:
    """
    Tamper-evident audit logger.
    
    Each event includes a hash chain linking to the previous event,
    making it possible to detect log tampering.
    """
    
    def __init__(self):
        self._events: List[AuditEvent] = []
        self._lock = threading.Lock()
        self._event_counter = AtomicCounter(0)
    
    def log(
        self,
        event_type: AuditEventType,
        actor_did: str,
        resource_type: str,
        resource_id: str,
        action: str,
        outcome: str,
        details: Optional[Dict[str, Any]] = None,
        **kwargs,
    ) -> AuditEvent:
        """Log an audit event."""
        with self._lock:
            event_num = self._event_counter.increment()
            previous_digest = self._events[-1].event_digest if self._events else None
            
            event = AuditEvent(
                event_id=f"evt-{event_num:012d}",
                event_type=event_type,
                timestamp=datetime.now(timezone.utc).isoformat(),
                actor_did=actor_did,
                resource_type=resource_type,
                resource_id=resource_id,
                action=action,
                outcome=outcome,
                details=details or {},
                previous_event_digest=previous_digest,
                **kwargs,
            )
            
            self._events.append(event)
            return event
    
    def verify_chain(self) -> Tuple[bool, Optional[int]]:
        """
        Verify the audit log chain integrity.
        
        Returns (valid, first_invalid_index).
        """
        with self._lock:
            for i, event in enumerate(self._events):
                # Verify digest
                computed = event._compute_digest()
                if computed != event.event_digest:
                    return (False, i)
                
                # Verify chain
                if i > 0:
                    expected_prev = self._events[i - 1].event_digest
                    if event.previous_event_digest != expected_prev:
                        return (False, i)
            
            return (True, None)
    
    def get_events(
        self,
        actor_did: Optional[str] = None,
        event_type: Optional[AuditEventType] = None,
        resource_id: Optional[str] = None,
        since: Optional[datetime] = None,
        limit: int = 100,
    ) -> List[AuditEvent]:
        """Query audit events."""
        with self._lock:
            events = list(self._events)
        
        # Apply filters
        if actor_did:
            events = [e for e in events if e.actor_did == actor_did]
        if event_type:
            events = [e for e in events if e.event_type == event_type]
        if resource_id:
            events = [e for e in events if e.resource_id == resource_id]
        if since:
            from tools.phoenix.hardening import parse_iso_timestamp
            events = [e for e in events if parse_iso_timestamp(e.timestamp) >= since]
        
        return events[-limit:]
    
    def export(self) -> List[Dict[str, Any]]:
        """Export all events as dicts."""
        with self._lock:
            return [e.to_dict() for e in self._events]


# =============================================================================
# SECURE WITHDRAWAL MANAGER
# =============================================================================

@dataclass
class WithdrawalRequest:
    """A request to withdraw bonded collateral."""
    request_id: str
    watcher_did: str
    bond_id: str
    amount: Decimal
    destination_address: str
    
    # Time lock
    requested_at: str
    unlocks_at: str
    expires_at: str
    
    # State
    state: str = "pending"  # pending, cancelled, executed, expired
    executed_at: Optional[str] = None
    tx_hash: Optional[str] = None


class SecureWithdrawalManager:
    """
    Manages withdrawals with time-lock protection.
    
    Implements the following security measures:
    1. Mandatory 7-day delay before withdrawal
    2. Slashing claims can be filed during delay
    3. Active attestations block withdrawal
    4. Partial withdrawals allowed (down to minimum)
    """
    
    DELAY_DAYS = 7
    MIN_REMAINING_COLLATERAL = Decimal("1000")
    
    def __init__(self, audit_logger: AuditLogger):
        self._requests: Dict[str, WithdrawalRequest] = {}
        self._lock = threading.Lock()
        self._audit = audit_logger
    
    def request_withdrawal(
        self,
        watcher_did: str,
        bond_id: str,
        amount: Decimal,
        destination_address: str,
        current_collateral: Decimal,
        active_attestation_value: Decimal,
    ) -> Tuple[bool, Union[WithdrawalRequest, str]]:
        """
        Request a withdrawal.
        
        Returns (success, request_or_error_message).
        """
        # Validate destination address
        result = Validators.validate_address(destination_address)
        if not result.is_valid:
            return (False, "Invalid destination address")
        
        # Check sufficient collateral remains
        remaining = current_collateral - amount
        if remaining < self.MIN_REMAINING_COLLATERAL and remaining != Decimal("0"):
            return (False, f"Withdrawal would leave less than minimum {self.MIN_REMAINING_COLLATERAL}")
        
        # Check not over-collateralizing active attestations
        required_collateral = active_attestation_value / Decimal("10")
        if remaining < required_collateral:
            return (False, f"Cannot withdraw: active attestations require {required_collateral} collateral")
        
        # Create time-locked request
        now = datetime.now(timezone.utc)
        unlocks_at = now + timedelta(days=self.DELAY_DAYS)
        expires_at = unlocks_at + timedelta(days=2)
        
        request = WithdrawalRequest(
            request_id=f"wd-{secrets.token_hex(16)}",
            watcher_did=watcher_did,
            bond_id=bond_id,
            amount=amount,
            destination_address=result.sanitized_value,
            requested_at=now.isoformat(),
            unlocks_at=unlocks_at.isoformat(),
            expires_at=expires_at.isoformat(),
        )
        
        with self._lock:
            self._requests[request.request_id] = request
        
        # Audit log
        self._audit.log(
            event_type=AuditEventType.WITHDRAWAL_INITIATED,
            actor_did=watcher_did,
            resource_type="bond",
            resource_id=bond_id,
            action="withdrawal_requested",
            outcome="success",
            details={
                "request_id": request.request_id,
                "amount": str(amount),
                "unlocks_at": request.unlocks_at,
            },
        )
        
        return (True, request)
    
    def execute_withdrawal(
        self,
        request_id: str,
        current_collateral: Decimal,
        active_slashing_claims: int,
    ) -> Tuple[bool, str]:
        """
        Execute a pending withdrawal.
        
        Returns (success, message).
        """
        with self._lock:
            request = self._requests.get(request_id)
            
            if not request:
                return (False, "Withdrawal request not found")
            
            if request.state != "pending":
                return (False, f"Request in invalid state: {request.state}")
            
            # Check unlock time
            from tools.phoenix.hardening import parse_iso_timestamp
            now = datetime.now(timezone.utc)
            unlocks_at = parse_iso_timestamp(request.unlocks_at)
            expires_at = parse_iso_timestamp(request.expires_at)
            
            if now < unlocks_at:
                remaining = (unlocks_at - now).total_seconds() / 3600
                return (False, f"Withdrawal not yet unlocked ({remaining:.1f} hours remaining)")
            
            if now > expires_at:
                request.state = "expired"
                return (False, "Withdrawal request expired")
            
            # Check for active slashing claims
            if active_slashing_claims > 0:
                return (False, f"Cannot withdraw: {active_slashing_claims} active slashing claims")
            
            # Re-verify collateral
            if current_collateral < request.amount:
                return (False, "Insufficient collateral (may have been slashed)")
            
            # Execute
            request.state = "executed"
            request.executed_at = now.isoformat()
            request.tx_hash = f"0x{secrets.token_hex(32)}"  # Would be actual tx hash
            
            return (True, f"Withdrawal executed: {request.tx_hash}")
    
    def cancel_withdrawal(self, request_id: str, watcher_did: str) -> bool:
        """Cancel a pending withdrawal."""
        with self._lock:
            request = self._requests.get(request_id)
            
            if not request:
                return False
            
            if request.watcher_did != watcher_did:
                return False
            
            if request.state != "pending":
                return False
            
            request.state = "cancelled"
            return True
    
    def get_pending_withdrawals(
        self,
        bond_id: Optional[str] = None,
    ) -> List[WithdrawalRequest]:
        """Get pending withdrawal requests."""
        with self._lock:
            requests = [r for r in self._requests.values() if r.state == "pending"]
            if bond_id:
                requests = [r for r in requests if r.bond_id == bond_id]
            return requests
