"""
PHOENIX Validation and Hardening Module

This module provides comprehensive validation, security hardening, and defensive
programming utilities for the PHOENIX infrastructure. It addresses:

1. Input validation with sanitization
2. Cryptographic verification utilities
3. Thread-safety primitives
4. State machine invariant enforcement
5. Economic attack prevention
6. Rate limiting and DoS protection

Security Model:
    - All inputs are untrusted until validated
    - All cryptographic operations use constant-time comparisons
    - All state mutations are atomic or compensated
    - All economic operations are bounded

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import hmac
import re
import secrets
import threading
from contextlib import contextmanager
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal, InvalidOperation
from enum import Enum
from functools import wraps
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
    TypeVar,
    Union,
)


# =============================================================================
# VALIDATION ERROR TYPES
# =============================================================================

class ValidationError(Exception):
    """Base exception for validation failures."""
    
    def __init__(self, field: str, message: str, value: Any = None):
        self.field = field
        self.message = message
        self.value = value
        super().__init__(f"{field}: {message}")


class ValidationErrors(Exception):
    """Collection of validation errors."""
    
    def __init__(self, errors: List[ValidationError]):
        self.errors = errors
        messages = "; ".join(f"{e.field}: {e.message}" for e in errors)
        super().__init__(f"Validation failed: {messages}")


class SecurityViolation(Exception):
    """Security constraint violated."""
    pass


class InvariantViolation(Exception):
    """State machine invariant violated."""
    pass


class EconomicAttackDetected(Exception):
    """Potential economic attack detected."""
    pass


# =============================================================================
# VALIDATION RESULT
# =============================================================================

@dataclass
class ValidationResult:
    """Result of a validation operation."""
    is_valid: bool
    errors: List[ValidationError] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    sanitized_value: Any = None
    
    def raise_if_invalid(self) -> None:
        """Raise ValidationErrors if validation failed."""
        if not self.is_valid:
            raise ValidationErrors(self.errors)
    
    @classmethod
    def success(cls, sanitized_value: Any = None) -> 'ValidationResult':
        return cls(is_valid=True, sanitized_value=sanitized_value)
    
    @classmethod
    def failure(cls, errors: List[ValidationError]) -> 'ValidationResult':
        return cls(is_valid=False, errors=errors)


# =============================================================================
# TIMESTAMP UTILITIES
# =============================================================================

def parse_iso_timestamp(timestamp: str) -> datetime:
    """
    Parse ISO 8601 timestamp with robust handling of all common formats.

    Handles:
    - "2026-01-01T00:00:00Z" (Zulu time)
    - "2026-01-01T00:00:00+00:00" (explicit offset)
    - "2026-01-01T00:00:00.123Z" (with milliseconds)
    - "2026-01-01T00:00:00.123456Z" (with microseconds)
    - "2026-01-01T00:00:00" (no timezone - assumes UTC)

    Args:
        timestamp: ISO 8601 formatted timestamp string

    Returns:
        datetime object with timezone info (UTC if not specified)

    Raises:
        ValueError: If timestamp cannot be parsed
    """
    if not timestamp:
        raise ValueError("Empty timestamp")

    # Normalize: replace Z with +00:00 for fromisoformat compatibility
    normalized = timestamp.replace("Z", "+00:00")

    try:
        dt = datetime.fromisoformat(normalized)
    except ValueError:
        # Try without timezone (assume UTC)
        try:
            # Remove any trailing timezone info that might be malformed
            base = timestamp.split("+")[0].split("-")[0:3]
            base_str = "-".join(base[:3])
            if "T" in timestamp:
                time_part = timestamp.split("T")[1].split("+")[0].split("Z")[0]
                base_str += "T" + time_part
            dt = datetime.fromisoformat(base_str)
            dt = dt.replace(tzinfo=timezone.utc)
        except (ValueError, IndexError) as e:
            raise ValueError(f"Cannot parse timestamp: {timestamp}") from e

    # Ensure timezone awareness (default to UTC)
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)

    return dt


# =============================================================================
# INPUT VALIDATORS
# =============================================================================

class Validators:
    """Collection of input validators."""
    
    # Patterns
    DID_PATTERN = re.compile(r'^did:[a-z0-9]+:[a-zA-Z0-9._-]+$')
    HEX64_PATTERN = re.compile(r'^[a-f0-9]{64}$')
    HEX40_PATTERN = re.compile(r'^0x[a-f0-9]{40}$')
    HEX_PATTERN = re.compile(r'^[a-f0-9]+$')
    JURISDICTION_PATTERN = re.compile(r'^[a-z]{2,3}-[a-z0-9-]+$')
    ASSET_ID_PATTERN = re.compile(r'^[a-zA-Z0-9_-]{1,128}$')
    
    # Limits
    MAX_STRING_LENGTH = 4096
    MAX_METADATA_SIZE = 65536
    MAX_ATTESTATIONS_PER_CELL = 100
    MAX_HOPS = 10
    MAX_AMOUNT_USD = Decimal("1000000000000")  # 1 trillion
    MIN_AMOUNT_USD = Decimal("0.01")
    
    @classmethod
    def validate_string(
        cls,
        value: Any,
        field_name: str,
        min_length: int = 1,
        max_length: int = None,
        pattern: Optional[re.Pattern] = None,
        allowed_chars: Optional[str] = None,
    ) -> ValidationResult:
        """Validate a string value."""
        max_length = max_length or cls.MAX_STRING_LENGTH
        errors = []
        
        if not isinstance(value, str):
            errors.append(ValidationError(field_name, f"Expected string, got {type(value).__name__}", value))
            return ValidationResult.failure(errors)
        
        # Reject null bytes — they indicate injection attempts
        if '\x00' in value:
            errors.append(ValidationError(field_name, "Contains null bytes", value))
            return ValidationResult.failure(errors)

        # Sanitize: strip whitespace
        sanitized = value.strip()

        if len(sanitized) < min_length:
            errors.append(ValidationError(field_name, f"Too short (min {min_length} chars)", value))
        
        if len(sanitized) > max_length:
            errors.append(ValidationError(field_name, f"Too long (max {max_length} chars)", value))
            sanitized = sanitized[:max_length]
        
        if pattern and not pattern.match(sanitized):
            errors.append(ValidationError(field_name, f"Does not match required pattern", value))
        
        if allowed_chars:
            invalid_chars = set(sanitized) - set(allowed_chars)
            if invalid_chars:
                errors.append(ValidationError(
                    field_name,
                    f"Contains invalid characters: {invalid_chars}",
                    value
                ))
        
        if errors:
            return ValidationResult.failure(errors)
        return ValidationResult.success(sanitized)
    
    @classmethod
    def validate_asset_id(cls, value: Any) -> ValidationResult:
        """Validate an asset ID."""
        return cls.validate_string(
            value, "asset_id",
            min_length=1, max_length=128,
            pattern=cls.ASSET_ID_PATTERN
        )
    
    @classmethod
    def validate_jurisdiction_id(cls, value: Any) -> ValidationResult:
        """Validate a jurisdiction ID."""
        return cls.validate_string(
            value, "jurisdiction_id",
            min_length=3, max_length=64,
            pattern=cls.JURISDICTION_PATTERN
        )
    
    @classmethod
    def validate_did(cls, value: Any, field_name: str = "did") -> ValidationResult:
        """Validate a DID."""
        return cls.validate_string(
            value, field_name,
            min_length=8, max_length=256,
            pattern=cls.DID_PATTERN
        )
    
    @classmethod
    def validate_digest(cls, value: Any, field_name: str = "digest") -> ValidationResult:
        """Validate a SHA256 digest (64 lowercase hex chars)."""
        result = cls.validate_string(value, field_name, min_length=64, max_length=64)
        if not result.is_valid:
            return result

        # Digests must already be lowercase — reject uppercase
        if not cls.HEX64_PATTERN.match(result.sanitized_value):
            return ValidationResult.failure([
                ValidationError(field_name, "Must be 64 lowercase hex characters", value)
            ])

        return ValidationResult.success(result.sanitized_value)
    
    @classmethod
    def validate_address(cls, value: Any, field_name: str = "address") -> ValidationResult:
        """Validate an Ethereum address."""
        result = cls.validate_string(value, field_name, min_length=42, max_length=42)
        if not result.is_valid:
            return result
        
        lower = result.sanitized_value.lower()
        if not cls.HEX40_PATTERN.match(lower):
            return ValidationResult.failure([
                ValidationError(field_name, "Must be valid Ethereum address (0x + 40 hex)", value)
            ])
        
        return ValidationResult.success(lower)
    
    @classmethod
    def validate_amount(
        cls,
        value: Any,
        field_name: str = "amount",
        min_value: Optional[Decimal] = None,
        max_value: Optional[Decimal] = None,
    ) -> ValidationResult:
        """Validate a monetary amount."""
        min_value = min_value if min_value is not None else cls.MIN_AMOUNT_USD
        max_value = max_value if max_value is not None else cls.MAX_AMOUNT_USD
        errors = []
        
        try:
            if isinstance(value, str):
                amount = Decimal(value)
            elif isinstance(value, (int, float)):
                amount = Decimal(str(value))
            elif isinstance(value, Decimal):
                amount = value
            else:
                errors.append(ValidationError(field_name, f"Cannot convert {type(value).__name__} to Decimal", value))
                return ValidationResult.failure(errors)
        except InvalidOperation:
            errors.append(ValidationError(field_name, "Invalid decimal value", value))
            return ValidationResult.failure(errors)
        
        # Check for NaN, Inf
        if not amount.is_finite():
            errors.append(ValidationError(field_name, "Must be a finite number", value))
            return ValidationResult.failure(errors)
        
        if amount < min_value:
            errors.append(ValidationError(field_name, f"Below minimum ({min_value})", value))
        
        if amount > max_value:
            errors.append(ValidationError(field_name, f"Exceeds maximum ({max_value})", value))
        
        if errors:
            return ValidationResult.failure(errors)
        
        return ValidationResult.success(amount)
    
    @classmethod
    def validate_timestamp(
        cls,
        value: Any,
        field_name: str = "timestamp",
        allow_future: bool = True,
        max_age_days: Optional[int] = None,
    ) -> ValidationResult:
        """Validate an ISO8601 timestamp."""
        errors = []
        
        if isinstance(value, datetime):
            dt = value
        elif isinstance(value, str):
            try:
                dt = parse_iso_timestamp(value)
            except ValueError:
                errors.append(ValidationError(field_name, "Invalid ISO8601 timestamp", value))
                return ValidationResult.failure(errors)
        else:
            errors.append(ValidationError(field_name, f"Expected datetime or string, got {type(value).__name__}", value))
            return ValidationResult.failure(errors)
        
        now = datetime.now(timezone.utc)
        
        if not allow_future and dt > now + timedelta(seconds=60):  # 60s clock skew allowance
            errors.append(ValidationError(field_name, "Timestamp is in the future", value))
        
        if max_age_days is not None:
            min_time = now - timedelta(days=max_age_days)
            if dt < min_time:
                errors.append(ValidationError(field_name, f"Timestamp too old (max {max_age_days} days)", value))
        
        if errors:
            return ValidationResult.failure(errors)
        
        return ValidationResult.success(dt)
    
    @classmethod
    def validate_bytes(
        cls,
        value: Any,
        field_name: str,
        min_length: int = 0,
        max_length: int = 65536,
    ) -> ValidationResult:
        """Validate bytes."""
        errors = []
        
        if isinstance(value, str):
            # Try hex decoding
            try:
                value = bytes.fromhex(value)
            except ValueError:
                errors.append(ValidationError(field_name, "Invalid hex string", value))
                return ValidationResult.failure(errors)
        
        if not isinstance(value, bytes):
            errors.append(ValidationError(field_name, f"Expected bytes, got {type(value).__name__}", value))
            return ValidationResult.failure(errors)
        
        if len(value) < min_length:
            errors.append(ValidationError(field_name, f"Too short (min {min_length} bytes)", value))
        
        if len(value) > max_length:
            errors.append(ValidationError(field_name, f"Too long (max {max_length} bytes)", value))
        
        if errors:
            return ValidationResult.failure(errors)
        
        return ValidationResult.success(value)


# =============================================================================
# CRYPTOGRAPHIC UTILITIES
# =============================================================================

class CryptoUtils:
    """Cryptographic utility functions with security hardening."""
    
    @staticmethod
    def secure_compare(a: bytes, b: bytes) -> bool:
        """Constant-time comparison to prevent timing attacks."""
        return hmac.compare_digest(a, b)
    
    @staticmethod
    def secure_compare_str(a: str, b: str) -> bool:
        """Constant-time string comparison."""
        return hmac.compare_digest(a.encode(), b.encode())
    
    @staticmethod
    def secure_random_hex(n_bytes: int = 32) -> str:
        """Generate cryptographically secure random hex string."""
        return secrets.token_hex(n_bytes)
    
    @staticmethod
    def secure_random_bytes(n_bytes: int = 32) -> bytes:
        """Generate cryptographically secure random bytes."""
        return secrets.token_bytes(n_bytes)
    
    @staticmethod
    def hash_sha256(data: Union[str, bytes]) -> str:
        """Compute SHA256 hash."""
        if isinstance(data, str):
            data = data.encode('utf-8')
        return hashlib.sha256(data).hexdigest()
    
    @staticmethod
    def merkle_root(leaves: List[str]) -> str:
        """
        Compute Merkle root with proper handling of odd leaf counts.
        
        Uses the convention of duplicating the last leaf when odd.
        """
        if not leaves:
            return "0" * 64
        
        if len(leaves) == 1:
            return leaves[0]
        
        # Ensure even number by duplicating last
        current_level = list(leaves)
        
        while len(current_level) > 1:
            if len(current_level) % 2 == 1:
                current_level.append(current_level[-1])
            
            next_level = []
            for i in range(0, len(current_level), 2):
                combined = current_level[i] + current_level[i + 1]
                parent = hashlib.sha256(combined.encode()).hexdigest()
                next_level.append(parent)
            
            current_level = next_level
        
        return current_level[0]
    
    @staticmethod
    def compute_merkle_root(
        leaf: str,
        proof: List[str],
        indices: List[int],
    ) -> str:
        """Walk a Merkle proof and return the computed root hash."""
        current = leaf
        for sibling, index in zip(proof, indices):
            if index == 0:  # Current is left
                combined = current + sibling
            else:  # Current is right
                combined = sibling + current
            current = hashlib.sha256(combined.encode()).hexdigest()
        return current

    @staticmethod
    def verify_merkle_proof(
        leaf: str,
        proof: List[str],
        indices: List[int],
        root: Optional[str] = None,
    ) -> Union[bool, str]:
        """Verify a Merkle inclusion proof.

        When *root* is provided, returns ``True``/``False`` indicating
        whether the proof matches (original API).  When *root* is omitted,
        returns the computed root hash so the caller can compare it.
        """
        if len(proof) != len(indices):
            if root is not None:
                return False
            return ""

        computed = CryptoUtils.compute_merkle_root(leaf, proof, indices)

        if root is not None:
            return CryptoUtils.secure_compare_str(computed, root)
        return computed


# =============================================================================
# THREAD SAFETY
# =============================================================================

T = TypeVar('T')


class ThreadSafeDict(Dict[str, T]):
    """Thread-safe dictionary wrapper.

    All read, write, and iteration operations are protected by an RLock.

    IMPORTANT: keys(), values(), and items() return snapshot lists, not live
    views. This is intentional — live views would defeat the purpose of the
    lock because they lazily iterate the underlying dict outside the critical
    section.  Callers that need a consistent snapshot across multiple keys
    should use the ``transaction()`` context manager.
    """

    def __init__(self, *args, **kwargs):
        super().__init__(*args, **kwargs)
        self._lock = threading.RLock()

    def __getitem__(self, key: str) -> T:
        with self._lock:
            return super().__getitem__(key)

    def __setitem__(self, key: str, value: T) -> None:
        with self._lock:
            super().__setitem__(key, value)

    def __delitem__(self, key: str) -> None:
        with self._lock:
            super().__delitem__(key)

    def __contains__(self, key: object) -> bool:
        with self._lock:
            return super().__contains__(key)

    def __iter__(self) -> Iterator[str]:
        with self._lock:
            return iter(list(super().keys()))

    def __len__(self) -> int:
        with self._lock:
            return super().__len__()

    def get(self, key: str, default: T = None) -> Optional[T]:
        with self._lock:
            return super().get(key, default)

    def pop(self, key: str, *args) -> T:
        with self._lock:
            return super().pop(key, *args)

    def setdefault(self, key: str, default: T = None) -> T:
        with self._lock:
            return super().setdefault(key, default)

    def update(self, *args, **kwargs) -> None:
        with self._lock:
            super().update(*args, **kwargs)

    def keys(self) -> List[str]:
        with self._lock:
            return list(super().keys())

    def values(self) -> List[T]:
        with self._lock:
            return list(super().values())

    def items(self) -> List[Tuple[str, T]]:
        with self._lock:
            return list(super().items())

    def copy(self) -> Dict[str, T]:
        with self._lock:
            return dict(super().items())

    @contextmanager
    def transaction(self):
        """Context manager for atomic multi-operation transactions."""
        with self._lock:
            yield self


class AtomicCounter:
    """Thread-safe counter."""
    
    def __init__(self, initial: int = 0):
        self._value = initial
        self._lock = threading.Lock()
    
    def increment(self, delta: int = 1) -> int:
        """Atomically increment and return new value."""
        with self._lock:
            self._value += delta
            return self._value
    
    def decrement(self, delta: int = 1) -> int:
        """Atomically decrement and return new value."""
        with self._lock:
            self._value -= delta
            return self._value
    
    @property
    def value(self) -> int:
        """Get current value (property access)."""
        with self._lock:
            return self._value

    def get(self) -> int:
        """Get current value."""
        with self._lock:
            return self._value

    def compare_and_set(self, expected: int, new_value: int) -> bool:
        """Atomically set value if it equals expected."""
        with self._lock:
            if self._value == expected:
                self._value = new_value
                return True
            return False


# =============================================================================
# STATE MACHINE INVARIANTS
# =============================================================================

class InvariantChecker:
    """Enforces state machine invariants."""

    def __init__(self) -> None:
        self._state_machines: Dict[str, Dict[str, List[str]]] = {}

    def register_state_machine(
        self,
        name: str,
        transitions: Dict[str, List[str]],
    ) -> None:
        """Register a named state machine with its valid transitions."""
        self._state_machines[name] = transitions

    def can_transition(self, name: str, from_state: str, to_state: str) -> bool:
        """Check whether a transition is valid for a registered state machine."""
        machine = self._state_machines.get(name)
        if machine is None:
            return False
        valid_targets = machine.get(from_state, [])
        return to_state in valid_targets

    @staticmethod
    def check_state_transition(
        current_state: Enum,
        target_state: Enum,
        valid_transitions: Dict[Enum, Set[Enum]],
    ) -> None:
        """Verify state transition is valid."""
        valid_targets = valid_transitions.get(current_state, set())
        if target_state not in valid_targets:
            raise InvariantViolation(
                f"Invalid state transition: {current_state.value} -> {target_state.value}. "
                f"Valid targets: {[s.value for s in valid_targets]}"
            )
    
    @staticmethod
    def check_monotonic_increase(
        field_name: str,
        old_value: int,
        new_value: int,
    ) -> None:
        """Ensure value only increases."""
        if new_value < old_value:
            raise InvariantViolation(
                f"{field_name} must be monotonically increasing: "
                f"cannot go from {old_value} to {new_value}"
            )
    
    @staticmethod
    def check_non_negative(field_name: str, value: Decimal) -> None:
        """Ensure value is non-negative."""
        if value < 0:
            raise InvariantViolation(f"{field_name} cannot be negative: {value}")
    
    @staticmethod
    def check_balance_sufficient(
        available: Decimal,
        required: Decimal,
        field_name: str = "balance",
    ) -> None:
        """Ensure sufficient balance for operation."""
        if available < required:
            raise InvariantViolation(
                f"Insufficient {field_name}: have {available}, need {required}"
            )


# =============================================================================
# ECONOMIC ATTACK PREVENTION
# =============================================================================

class EconomicGuard:
    """Guards against economic attacks.

    Can be used as a class (classmethods raise on violation) or as an
    instance with configurable thresholds and boolean return values.
    """

    # Thresholds (class-level defaults)
    MAX_ATTESTATION_VALUE_MULTIPLE = Decimal("10")  # Max 10x collateral
    MIN_BOND_COLLATERAL_USD = Decimal("1000")
    MAX_SLASH_RATE_PER_EPOCH = Decimal("0.5")  # Max 50% slash per epoch

    def __init__(
        self,
        max_attestation_value_multiple: Optional[Decimal] = None,
        min_collateral_usd: Optional[Decimal] = None,
        max_slash_rate_per_epoch: Optional[Decimal] = None,
        max_stake_concentration: Optional[Decimal] = None,
    ) -> None:
        self._max_attestation_value_multiple = (
            Decimal(str(max_attestation_value_multiple))
            if max_attestation_value_multiple is not None
            else self.MAX_ATTESTATION_VALUE_MULTIPLE
        )
        self._min_collateral_usd = (
            min_collateral_usd if min_collateral_usd is not None
            else self.MIN_BOND_COLLATERAL_USD
        )
        self._max_slash_rate = (
            max_slash_rate_per_epoch if max_slash_rate_per_epoch is not None
            else self.MAX_SLASH_RATE_PER_EPOCH
        )
        self._max_stake_concentration = (
            max_stake_concentration if max_stake_concentration is not None
            else Decimal("0.33")
        )

    # -- Instance methods returning bool --------------------------------

    def validate_attestation_value(
        self, attestation_value: Decimal, collateral: Decimal
    ) -> bool:
        """Return True if attestation value is within collateral limits."""
        max_value = collateral * self._max_attestation_value_multiple
        return attestation_value <= max_value

    def check_concentration(
        self, stake: Decimal, total_stake: Decimal
    ) -> bool:
        """Return True if stake concentration is within threshold."""
        if total_stake == 0:
            return True
        return (stake / total_stake) <= self._max_stake_concentration

    def validate_slash(
        self, slash_amount: Decimal, watcher_stake: Decimal
    ) -> bool:
        """Return True if slash amount is within per-epoch limits."""
        if watcher_stake == 0:
            return True
        return (slash_amount / watcher_stake) <= self._max_slash_rate

    # -- Class methods (original API — raise on violation) ---------------

    @classmethod
    def check_attestation_limit(
        cls,
        bond_collateral: Decimal,
        attestation_value: Decimal,
    ) -> None:
        """Ensure attestation value within bond limits."""
        max_value = bond_collateral * cls.MAX_ATTESTATION_VALUE_MULTIPLE
        if attestation_value > max_value:
            raise EconomicAttackDetected(
                f"Attestation value {attestation_value} exceeds limit {max_value} "
                f"for bond collateral {bond_collateral}"
            )

    @classmethod
    def check_minimum_collateral(cls, collateral: Decimal) -> None:
        """Ensure minimum collateral requirements met."""
        if collateral < cls.MIN_BOND_COLLATERAL_USD:
            raise EconomicAttackDetected(
                f"Collateral {collateral} below minimum {cls.MIN_BOND_COLLATERAL_USD}"
            )

    @classmethod
    def check_slash_rate(
        cls,
        total_slashed: Decimal,
        total_collateral: Decimal,
        epoch_start: datetime,
    ) -> None:
        """Check slash rate within acceptable bounds."""
        if total_collateral == 0:
            return

        slash_rate = total_slashed / total_collateral
        if slash_rate > cls.MAX_SLASH_RATE_PER_EPOCH:
            raise EconomicAttackDetected(
                f"Slash rate {slash_rate} exceeds epoch maximum {cls.MAX_SLASH_RATE_PER_EPOCH}. "
                f"Possible coordinated attack."
            )

    @staticmethod
    def check_whale_concentration(
        operator_stake: Decimal,
        total_stake: Decimal,
        max_concentration: Decimal = Decimal("0.33"),
    ) -> None:
        """Check for whale concentration risk."""
        if total_stake == 0:
            return

        concentration = operator_stake / total_stake
        if concentration > max_concentration:
            raise EconomicAttackDetected(
                f"Stake concentration {concentration} exceeds threshold {max_concentration}. "
                f"Possible whale attack."
            )


# =============================================================================
# RATE LIMITING
# =============================================================================

@dataclass
class RateLimitConfig:
    """Configuration for rate limiting."""
    requests_per_minute: int = 60
    requests_per_hour: int = 1000
    burst_size: int = 10


class _PerClientBucket:
    """Internal per-client token bucket."""

    def __init__(self, rate: int, window: float) -> None:
        self._rate = rate
        self._window = window
        self._timestamps: List[float] = []

    def allow(self) -> bool:
        """Return True if the request is within the rate limit."""
        import time as _time
        now = _time.monotonic()
        # Discard timestamps outside the current window
        cutoff = now - self._window
        self._timestamps = [t for t in self._timestamps if t > cutoff]
        if len(self._timestamps) < self._rate:
            self._timestamps.append(now)
            return True
        return False


class RateLimiter:
    """Token bucket rate limiter.

    Supports two construction styles:
      - ``RateLimiter(config=RateLimitConfig(...))``   (original API)
      - ``RateLimiter(rate=10, per_seconds=1)``        (simple per-client API)
    """

    def __init__(
        self,
        config: Optional[RateLimitConfig] = None,
        *,
        rate: Optional[int] = None,
        per_seconds: Optional[float] = None,
    ):
        if rate is not None:
            # Simple per-client mode
            self._rate = rate
            self._window = float(per_seconds) if per_seconds else 1.0
            self._clients: Dict[str, _PerClientBucket] = {}
            self._client_lock = threading.Lock()
            self._simple_mode = True
            # Also initialise legacy fields so ``acquire`` still works
            self.config = RateLimitConfig(
                requests_per_minute=int(rate * (60.0 / self._window)),
                burst_size=rate,
            )
        elif config is not None:
            self.config = config
            self._simple_mode = False
        else:
            self.config = RateLimitConfig()
            self._simple_mode = False

        if not self._simple_mode:
            self._tokens = float(self.config.burst_size)
            self._last_update = datetime.now(timezone.utc)
            self._lock = threading.Lock()
            self._refill_rate = self.config.requests_per_minute / 60.0

    # -- Simple per-client API ------------------------------------------

    def allow(self, client_id: str) -> bool:
        """Check whether *client_id* is within its rate limit."""
        with self._client_lock:
            bucket = self._clients.get(client_id)
            if bucket is None:
                bucket = _PerClientBucket(self._rate, self._window)
                self._clients[client_id] = bucket
            return bucket.allow()

    # -- Original API ---------------------------------------------------

    def acquire(self, tokens: int = 1) -> bool:
        """Try to acquire tokens. Returns True if successful."""
        with self._lock:
            now = datetime.now(timezone.utc)
            elapsed = (now - self._last_update).total_seconds()
            self._last_update = now

            # Refill tokens
            self._tokens = min(
                self.config.burst_size,
                self._tokens + elapsed * self._refill_rate
            )

            if self._tokens >= tokens:
                self._tokens -= tokens
                return True
            return False

    def wait_and_acquire(self, tokens: int = 1, max_wait_seconds: float = 10.0) -> bool:
        """Wait for tokens to become available."""
        start = datetime.now(timezone.utc)
        while True:
            if self.acquire(tokens):
                return True

            elapsed = (datetime.now(timezone.utc) - start).total_seconds()
            if elapsed >= max_wait_seconds:
                return False

            # Sleep a bit
            import time
            time.sleep(0.1)


# =============================================================================
# DECORATOR UTILITIES
# =============================================================================

def validated(validator_func: Callable) -> Callable:
    """Decorator to validate function arguments."""
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            result = validator_func(*args, **kwargs)
            if isinstance(result, ValidationResult) and not result.is_valid:
                result.raise_if_invalid()
            return func(*args, **kwargs)
        return wrapper
    return decorator


def rate_limited(limiter: RateLimiter) -> Callable:
    """Decorator to rate limit function calls."""
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            if not limiter.acquire():
                raise SecurityViolation("Rate limit exceeded")
            return func(*args, **kwargs)
        return wrapper
    return decorator


def atomic(lock: threading.Lock) -> Callable:
    """Decorator to make function atomic."""
    def decorator(func: Callable) -> Callable:
        @wraps(func)
        def wrapper(*args, **kwargs):
            with lock:
                return func(*args, **kwargs)
        return wrapper
    return decorator
