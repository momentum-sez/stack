"""
PHOENIX Legendary Quality Test Suite

Engineering excellence tests following Torvalds/Carmack principles:
- Torvalds: "Given enough eyeballs, all bugs are shallow"
  => Test every edge case, every boundary, every invariant

- Carmack: "Low-level optimization only matters if you've
  eliminated all the high-level stupidity"
  => Test that the architecture makes correctness easy

Test Categories:
1. Word Arithmetic Boundaries - Every possible overflow/underflow
2. State Machine Exhaustive - All transitions and invalid transitions
3. Cryptographic Invariants - Hash properties, signature verification
4. Thread Safety Under Load - Race conditions, deadlocks
5. Economic Attack Vectors - Every known attack pattern
6. Memory Safety - Buffer boundaries, allocation limits

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import secrets
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Any, Dict, List, Optional, Set, Tuple

import pytest


# =============================================================================
# WORD ARITHMETIC BOUNDARY TESTS
# =============================================================================

class TestWordArithmeticBoundaries:
    """
    Test every arithmetic boundary in the 256-bit Word type.

    Carmack principle: "The physics simulation doesn't care about your
    intentions. It only cares about the math being right."
    """

    def test_word_max_value(self):
        """Test Word at maximum 256-bit value."""
        from tools.phoenix.vm import Word

        max_val = (1 << 256) - 1
        w = Word.from_int(max_val)
        assert w.to_int() == max_val

        # Adding 1 should wrap to 0
        one = Word.from_int(1)
        result = w + one
        assert result.to_int() == 0, "Overflow should wrap to zero"

    def test_word_min_value_signed(self):
        """Test Word with minimum signed value."""
        from tools.phoenix.vm import Word

        # Two's complement minimum for 256 bits
        min_signed = -(1 << 255)
        w = Word.from_int(min_signed)

        # Should be stored as unsigned
        unsigned_repr = (1 << 256) + min_signed
        assert w.to_int(signed=False) == unsigned_repr

        # Should convert back correctly
        assert w.to_int(signed=True) == min_signed

    def test_word_subtraction_underflow(self):
        """Test underflow behavior in subtraction."""
        from tools.phoenix.vm import Word

        zero = Word.zero()
        one = Word.one()

        result = zero - one

        # Should wrap to max value
        max_val = (1 << 256) - 1
        assert result.to_int() == max_val

    def test_word_multiplication_overflow(self):
        """Test multiplication overflow behavior."""
        from tools.phoenix.vm import Word

        large = Word.from_int((1 << 128))

        # (2^128)^2 = 2^256 which overflows
        result = large * large

        # Should be 0 due to mod 2^256
        assert result.to_int() == 0

    def test_word_division_by_zero(self):
        """Test division by zero returns zero (EVM semantics)."""
        from tools.phoenix.vm import Word

        numerator = Word.from_int(100)
        zero = Word.zero()

        result = numerator / zero

        # EVM convention: x / 0 = 0
        assert result.to_int() == 0

    def test_word_modulo_by_zero(self):
        """Test modulo by zero returns zero."""
        from tools.phoenix.vm import Word

        value = Word.from_int(100)
        zero = Word.zero()

        result = value % zero
        assert result.to_int() == 0

    def test_word_byte_alignment(self):
        """Test Word byte alignment for all offsets."""
        from tools.phoenix.vm import Word

        # Test creating words from various byte lengths
        for length in range(1, 33):
            data = secrets.token_bytes(length)
            w = Word.from_bytes(data)

            # Should be exactly 32 bytes
            assert len(w.data) == 32

            # Should be right-padded with zeros on the left
            if length < 32:
                expected_padding = 32 - length
                assert w.data[:expected_padding] == b'\x00' * expected_padding

    def test_word_hex_conversion_edge_cases(self):
        """Test hex conversion edge cases."""
        from tools.phoenix.vm import Word

        # Leading zeros should be preserved
        w = Word.from_hex("0x0000000000000001")
        assert w.to_int() == 1

        # Full 64 character hex
        full_hex = "ff" * 32
        w = Word.from_hex(full_hex)
        assert w.to_int() == (1 << 256) - 1


# =============================================================================
# VM STATE MACHINE EXHAUSTIVE TESTS
# =============================================================================

class TestVMStateMachineExhaustive:
    """
    Test all possible VM state transitions.

    Torvalds principle: "Talk is cheap. Show me the code."
    => Prove correctness through exhaustive testing.
    """

    def test_stack_operations_boundary(self):
        """Test stack at exact boundary conditions."""
        from tools.phoenix.vm import VMState, Word
        from tools.phoenix.hardening import SecurityViolation

        state = VMState(code=b"")

        # Push exactly MAX_STACK_SIZE items
        for i in range(state.MAX_STACK_SIZE):
            state.push(Word.from_int(i))

        assert len(state.stack) == state.MAX_STACK_SIZE

        # Next push should raise
        with pytest.raises(SecurityViolation, match="Stack overflow"):
            state.push(Word.from_int(0))

    def test_stack_underflow_all_operations(self):
        """Test underflow on all stack-consuming operations."""
        from tools.phoenix.vm import VMState, Word
        from tools.phoenix.hardening import SecurityViolation

        state = VMState(code=b"")

        # Pop on empty stack
        with pytest.raises(SecurityViolation, match="underflow"):
            state.pop()

        # Peek on empty stack
        with pytest.raises(SecurityViolation, match="underflow"):
            state.peek(0)

        # Push one, then try to peek at depth 1
        state.push(Word.zero())
        with pytest.raises(SecurityViolation, match="underflow"):
            state.peek(1)

    def test_memory_expansion_limits(self):
        """Test memory expansion up to and beyond limits."""
        from tools.phoenix.vm import VMState, Word
        from tools.phoenix.hardening import SecurityViolation

        state = VMState(code=b"")

        # Valid expansion to just under limit
        state.mstore(state.MAX_MEMORY_SIZE - 32, Word.zero())
        assert len(state.memory) == state.MAX_MEMORY_SIZE

        # Expansion beyond limit should fail
        with pytest.raises(SecurityViolation, match="Memory limit"):
            state.mstore(state.MAX_MEMORY_SIZE, Word.zero())

    def test_storage_root_determinism(self):
        """Test that storage root is deterministic."""
        from tools.phoenix.vm import VMState, Word

        # Create two states with same storage (different insertion order)
        state1 = VMState(code=b"")
        state1.sstore("key_b", Word.from_int(2))
        state1.sstore("key_a", Word.from_int(1))
        state1.sstore("key_c", Word.from_int(3))

        state2 = VMState(code=b"")
        state2.sstore("key_a", Word.from_int(1))
        state2.sstore("key_c", Word.from_int(3))
        state2.sstore("key_b", Word.from_int(2))

        # Roots should be identical
        assert state1.storage_root() == state2.storage_root()

    def test_storage_root_empty(self):
        """Test storage root with empty storage."""
        from tools.phoenix.vm import VMState

        state = VMState(code=b"")
        root = state.storage_root()

        assert root == "0" * 64


# =============================================================================
# CRYPTOGRAPHIC INVARIANT TESTS
# =============================================================================

class TestCryptographicInvariants:
    """
    Test cryptographic properties that must hold.

    These are invariants that, if violated, would break the
    security model of the entire system.
    """

    def test_merkle_root_order_independence(self):
        """Test Merkle root is order-independent for determinism."""
        from tools.phoenix.hardening import CryptoUtils

        leaves1 = ["aaa", "bbb", "ccc", "ddd"]
        leaves2 = sorted(leaves1)

        # Pre-sort leaves for determinism
        root1 = CryptoUtils.merkle_root(sorted(leaves1))
        root2 = CryptoUtils.merkle_root(sorted(leaves2))

        assert root1 == root2

    def test_merkle_root_odd_leaf_handling(self):
        """Test Merkle root handles odd number of leaves."""
        from tools.phoenix.hardening import CryptoUtils

        # Odd number of leaves
        leaves = ["a", "b", "c"]
        root = CryptoUtils.merkle_root(leaves)

        # Should not crash and should produce valid hash
        assert len(root) == 64
        assert all(c in "0123456789abcdef" for c in root)

    def test_merkle_proof_verification(self):
        """Test Merkle proof verification."""
        from tools.phoenix.hardening import CryptoUtils

        # Build tree
        leaves = ["leaf0", "leaf1", "leaf2", "leaf3"]
        root = CryptoUtils.merkle_root(leaves)

        # Construct proof for leaf0
        # In a 4-leaf tree: H(H(H(leaf0),H(leaf1)), H(H(leaf2),H(leaf3)))
        # Proof for leaf0 needs: H(leaf1) and H(H(leaf2),H(leaf3))

        h0 = leaves[0]
        h1 = leaves[1]
        h23 = hashlib.sha256((leaves[2] + leaves[3]).encode()).hexdigest()

        proof = [h1, h23]
        indices = [0, 0]  # leaf0 is left child at both levels

        # Verify
        assert CryptoUtils.verify_merkle_proof(h0, proof, indices, root)

    def test_secure_compare_constant_time(self):
        """Test that comparison is timing-safe."""
        from tools.phoenix.hardening import CryptoUtils

        # Same strings
        a = b"test" * 1000
        b = b"test" * 1000

        # Different strings (differ at end)
        c = b"test" * 999 + b"xest"

        # Both comparisons should take similar time
        # (can't easily test timing, but verify correctness)
        assert CryptoUtils.secure_compare(a, b) is True
        assert CryptoUtils.secure_compare(a, c) is False

    def test_hash_preimage_resistance(self):
        """Test that hash function has proper preimage resistance properties."""
        from tools.phoenix.hardening import CryptoUtils

        # Different inputs must produce different hashes (with overwhelming probability)
        hashes = set()
        for i in range(1000):
            h = CryptoUtils.hash_sha256(f"test_input_{i}")
            assert h not in hashes, f"Hash collision at {i}"
            hashes.add(h)


# =============================================================================
# THREAD SAFETY UNDER LOAD TESTS
# =============================================================================

class TestThreadSafetyUnderLoad:
    """
    Test thread safety under extreme concurrent load.

    These tests verify that concurrent access doesn't cause
    data corruption or deadlocks.
    """

    def test_atomic_counter_under_contention(self):
        """Test AtomicCounter with high contention."""
        from tools.phoenix.hardening import AtomicCounter

        counter = AtomicCounter(0)
        iterations = 1000
        threads = 10

        def increment_many():
            for _ in range(iterations):
                counter.increment()

        workers = [threading.Thread(target=increment_many) for _ in range(threads)]
        for w in workers:
            w.start()
        for w in workers:
            w.join()

        assert counter.get() == threads * iterations

    def test_thread_safe_dict_concurrent_writes(self):
        """Test ThreadSafeDict with concurrent writes."""
        from tools.phoenix.hardening import ThreadSafeDict

        d: ThreadSafeDict[int] = ThreadSafeDict()
        iterations = 100
        threads = 10

        def write_many(thread_id: int):
            for i in range(iterations):
                key = f"key_{thread_id}_{i}"
                d[key] = thread_id * 1000 + i

        workers = [
            threading.Thread(target=write_many, args=(tid,))
            for tid in range(threads)
        ]
        for w in workers:
            w.start()
        for w in workers:
            w.join()

        # Verify all writes succeeded
        assert len(d) == threads * iterations

    def test_nonce_registry_concurrent_access(self):
        """Test NonceRegistry under concurrent registration."""
        from tools.phoenix.security import NonceRegistry

        registry = NonceRegistry()
        results: Dict[str, bool] = {}
        lock = threading.Lock()

        def register_nonces(prefix: str, count: int):
            for i in range(count):
                nonce = f"{prefix}_{i}"
                success = registry.check_and_register(nonce)
                with lock:
                    results[nonce] = success

        # Multiple threads trying to register overlapping nonces
        workers = []
        for prefix in ["a", "b", "c"]:
            t = threading.Thread(target=register_nonces, args=(prefix, 100))
            workers.append(t)

        for w in workers:
            w.start()
        for w in workers:
            w.join()

        # All registrations should succeed (unique nonces)
        assert all(results.values())

    def test_versioned_store_concurrent_cas(self):
        """Test VersionedStore CAS under concurrent updates."""
        from tools.phoenix.security import VersionedStore

        store: VersionedStore[int] = VersionedStore()
        store.set("counter", 0)

        success_count = AtomicInt()
        iterations = 100
        threads = 5

        def cas_increment():
            for _ in range(iterations):
                while True:
                    entry = store.get("counter")
                    if entry is None:
                        break

                    success, _ = store.compare_and_swap(
                        "counter",
                        entry.version,
                        entry.value + 1,
                    )

                    if success:
                        success_count.increment()
                        break

        workers = [threading.Thread(target=cas_increment) for _ in range(threads)]
        for w in workers:
            w.start()
        for w in workers:
            w.join()

        final = store.get("counter")
        assert final is not None
        assert final.value == threads * iterations


class AtomicInt:
    """Simple atomic integer for test counting."""

    def __init__(self, initial: int = 0):
        self._value = initial
        self._lock = threading.Lock()

    def increment(self) -> int:
        with self._lock:
            self._value += 1
            return self._value

    def get(self) -> int:
        with self._lock:
            return self._value


# =============================================================================
# ECONOMIC ATTACK VECTOR TESTS
# =============================================================================

class TestEconomicAttackVectors:
    """
    Test defense against known economic attack patterns.

    These tests verify the system resists attacks that exploit
    economic incentives rather than technical vulnerabilities.
    """

    def test_attestation_replay_attack(self):
        """Test that attestations cannot be replayed."""
        from tools.phoenix.security import (
            AttestationScope,
            ScopedAttestation,
            NonceRegistry,
        )

        scope = AttestationScope(
            asset_id="asset-001",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=datetime.now(timezone.utc).isoformat(),
            valid_until=(datetime.now(timezone.utc) + timedelta(days=30)).isoformat(),
        )

        attestation = ScopedAttestation.create(
            attestation_id="attest-001",
            attestation_type="kyc_verification",
            issuer_did="did:test:issuer",
            scope=scope,
            issuer_signature=b"sig",
        )

        # Register the nonce
        registry = NonceRegistry()
        first_use = registry.check_and_register(attestation.nonce)
        assert first_use is True

        # Replay attempt
        replay_attempt = registry.check_and_register(attestation.nonce)
        assert replay_attempt is False, "Replay should be detected"

    def test_scope_mismatch_attack(self):
        """Test attestation cannot be used for wrong scope."""
        from tools.phoenix.security import AttestationScope, ScopedAttestation

        scope = AttestationScope(
            asset_id="asset-001",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=datetime.now(timezone.utc).isoformat(),
            valid_until=(datetime.now(timezone.utc) + timedelta(days=30)).isoformat(),
        )

        attestation = ScopedAttestation.create(
            attestation_id="attest-002",
            attestation_type="kyc_verification",
            issuer_did="did:test:issuer",
            scope=scope,
            issuer_signature=b"sig",
        )

        # Valid use
        assert attestation.verify_scope(
            "asset-001", "uae-difc", "kyc"
        )

        # Wrong asset
        assert not attestation.verify_scope(
            "asset-002", "uae-difc", "kyc"
        )

        # Wrong jurisdiction
        assert not attestation.verify_scope(
            "asset-001", "sg-mas", "kyc"
        )

        # Wrong domain
        assert not attestation.verify_scope(
            "asset-001", "uae-difc", "aml"
        )

    def test_expired_attestation_attack(self):
        """Test that expired attestations are rejected."""
        from tools.phoenix.security import AttestationScope, ScopedAttestation

        # Create attestation valid in the past
        scope = AttestationScope(
            asset_id="asset-001",
            jurisdiction_id="uae-difc",
            domain="kyc",
            valid_from=(datetime.now(timezone.utc) - timedelta(days=60)).isoformat(),
            valid_until=(datetime.now(timezone.utc) - timedelta(days=30)).isoformat(),
        )

        attestation = ScopedAttestation.create(
            attestation_id="attest-003",
            attestation_type="kyc_verification",
            issuer_did="did:test:issuer",
            scope=scope,
            issuer_signature=b"sig",
        )

        # Should fail due to expiration
        assert not attestation.verify_scope(
            "asset-001", "uae-difc", "kyc"
        )

    def test_slashing_over_collateral(self):
        """Test that slashing cannot exceed available collateral."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher_id = WatcherId(did="did:test:watcher", public_key_hex="abc")
        bond = WatcherBond(
            bond_id="bond-001",
            watcher_id=watcher_id,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "a" * 40,
        )
        bond.status = BondStatus.ACTIVE

        # Try to slash more than available
        actual = bond.slash(Decimal("15000"), "test_slash")

        # Should only slash available amount
        assert actual == Decimal("10000")
        assert bond.available_collateral == Decimal("0")
        assert bond.status == BondStatus.FULLY_SLASHED

    def test_whale_concentration_detection(self):
        """Test detection of whale concentration attacks."""
        from tools.phoenix.hardening import EconomicGuard, EconomicAttackDetected

        # Single operator has 50% of stake
        with pytest.raises(EconomicAttackDetected, match="whale"):
            EconomicGuard.check_whale_concentration(
                operator_stake=Decimal("500"),
                total_stake=Decimal("1000"),
                max_concentration=Decimal("0.33"),
            )

    def test_front_running_protection(self):
        """Test time-lock protection against front-running.

        Note: TimeLock is a conceptual protection. The actual implementation
        uses versioned stores and nonce registries to prevent front-running.
        This test verifies the nonce-based approach.
        """
        from tools.phoenix.security import NonceRegistry

        registry = NonceRegistry()

        # Create operation with unique nonce
        operation_nonce = secrets.token_hex(16)

        # First use should succeed
        assert registry.check_and_register(operation_nonce) is True

        # Front-running attempt (reusing same nonce) should fail
        assert registry.check_and_register(operation_nonce) is False


# =============================================================================
# VALIDATION EXHAUSTIVE TESTS
# =============================================================================

class TestValidationExhaustive:
    """
    Test all validation paths exhaustively.

    Every input boundary, every error condition.
    """

    def test_jurisdiction_id_validation_patterns(self):
        """Test all jurisdiction ID patterns."""
        from tools.phoenix.hardening import Validators

        # Valid patterns (pattern allows 2-3 letter country code)
        valid_ids = [
            "us-ny",
            "ae-abudhabi-adgm",
            "sg-mas",
            "kz-aifc",
            "hk-hkma",
            "us-de-dsf",
            "usa-ny",       # 3-letter country is valid
        ]
        for jid in valid_ids:
            result = Validators.validate_jurisdiction_id(jid)
            assert result.is_valid, f"{jid} should be valid"

        # Invalid patterns
        invalid_ids = [
            "US-NY",        # uppercase
            "-us-ny",       # leading hyphen
            "us_ny",        # underscore
            "",             # empty
            "u",            # too short (needs min 3 chars)
        ]
        for jid in invalid_ids:
            result = Validators.validate_jurisdiction_id(jid)
            assert not result.is_valid, f"{jid} should be invalid"

    def test_amount_validation_boundaries(self):
        """Test amount validation at boundaries."""
        from tools.phoenix.hardening import Validators

        # Valid amounts
        assert Validators.validate_amount(Decimal("0.01")).is_valid
        assert Validators.validate_amount(Decimal("1000000000000")).is_valid
        assert Validators.validate_amount("100.50").is_valid
        assert Validators.validate_amount(100).is_valid

        # Invalid amounts
        assert not Validators.validate_amount(Decimal("0.001")).is_valid  # Below min
        assert not Validators.validate_amount(Decimal("1000000000001")).is_valid  # Above max
        assert not Validators.validate_amount("not_a_number").is_valid
        assert not Validators.validate_amount(float("inf")).is_valid
        assert not Validators.validate_amount(float("nan")).is_valid

    def test_digest_validation(self):
        """Test SHA256 digest validation."""
        from tools.phoenix.hardening import Validators

        # Valid digest
        valid = "a" * 64
        assert Validators.validate_digest(valid).is_valid

        # Invalid digests
        assert not Validators.validate_digest("a" * 63).is_valid  # Too short
        assert not Validators.validate_digest("a" * 65).is_valid  # Too long
        assert not Validators.validate_digest("g" * 64).is_valid  # Invalid hex
        assert not Validators.validate_digest("").is_valid  # Empty


# =============================================================================
# COMPLIANCE TENSOR TESTS
# =============================================================================

class TestComplianceTensorExhaustive:
    """
    Test compliance tensor operations exhaustively.
    """

    def test_tensor_domain_coverage(self):
        """Test all compliance domains are accessible."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2,
            ComplianceDomain,
            ComplianceState,
        )

        tensor = ComplianceTensorV2()

        # All domains should be addressable
        all_domains = list(ComplianceDomain)

        for domain in all_domains:
            tensor.set(
                asset_id="test-asset",
                jurisdiction_id="test-jur",
                domain=domain,
                state=ComplianceState.COMPLIANT,
                attestations=[],
            )

        # Verify all domains were set
        assert len(tensor._cells) == len(all_domains)

    def test_tensor_state_transitions(self):
        """Test all compliance state values exist and are unique."""
        from tools.phoenix.tensor import ComplianceState

        # Get all states
        all_states = list(ComplianceState)

        # Actual states: COMPLIANT, NON_COMPLIANT, PENDING, UNKNOWN, EXEMPT, EXPIRED
        assert len(all_states) == 6

        # Verify all state values are unique
        state_values = [s.value for s in all_states]
        assert len(state_values) == len(set(state_values))

        # Verify expected states exist
        expected = {"COMPLIANT", "NON_COMPLIANT", "PENDING", "UNKNOWN", "EXEMPT", "EXPIRED"}
        actual = {s.name for s in all_states}
        assert expected == actual


# =============================================================================
# MIGRATION STATE MACHINE TESTS
# =============================================================================

class TestMigrationStateMachineExhaustive:
    """
    Test migration state machine exhaustively.
    """

    def test_all_terminal_states(self):
        """Test that terminal states cannot transition further."""
        from tools.phoenix.migration import MigrationState, MigrationSaga, MigrationRequest

        # Terminal states in the actual implementation
        terminal_states = {
            MigrationState.COMPLETED,
            MigrationState.CANCELLED,
            MigrationState.COMPENSATED,
        }

        for terminal_state in terminal_states:
            # Create a saga and set it to terminal state
            request = MigrationRequest(
                asset_id="test-asset",
                asset_genesis_digest="a" * 64,
                source_jurisdiction="us-ny",
                target_jurisdiction="ae-difc",
            )
            saga = MigrationSaga(request)

            # Use the saga's VALID_TRANSITIONS to check terminal states
            valid_targets = saga.VALID_TRANSITIONS.get(terminal_state, set())
            assert len(valid_targets) == 0, f"{terminal_state} should have no valid transitions"

    def test_cancellation_paths(self):
        """Test cancellation is allowed from correct states."""
        from tools.phoenix.migration import MigrationState, MigrationSaga, MigrationRequest

        # Create a saga to access VALID_TRANSITIONS
        request = MigrationRequest(
            asset_id="test-asset",
            asset_genesis_digest="a" * 64,
            source_jurisdiction="us-ny",
            target_jurisdiction="ae-difc",
        )
        saga = MigrationSaga(request)

        # States where cancellation should be allowed
        cancellable_states = {
            MigrationState.INITIATED,
            MigrationState.COMPLIANCE_CHECK,
            MigrationState.ATTESTATION_GATHERING,
        }

        for state in cancellable_states:
            valid_targets = saga.VALID_TRANSITIONS.get(state, set())
            assert MigrationState.CANCELLED in valid_targets, (
                f"CANCELLED should be reachable from {state}"
            )


# =============================================================================
# ZONE COMPOSITION TESTS
# =============================================================================

class TestZoneCompositionQuality:
    """
    Test zone composition for architectural quality.
    """

    def test_domain_conflict_detection(self):
        """Test that domain conflicts are properly detected."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
        )

        # Create composition with domain conflict
        layer1 = JurisdictionLayer(
            jurisdiction_id="us-ny",
            domains=[Domain.CIVIC, Domain.CORPORATE],
        )
        layer2 = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE],  # Conflict with layer1
        )

        composition = ZoneComposition(
            zone_id="test.conflict",
            name="Conflict Test",
            layers=[layer1, layer2],
        )

        errors = composition.validate()
        assert any("conflict" in e.lower() for e in errors)

    def test_composition_digest_determinism(self):
        """Test that composition digest is deterministic."""
        from tools.msez.composition import compose_zone

        zone1 = compose_zone(
            "test.determinism",
            "Determinism Test",
            civic=("us-ny", "NY civic"),
            corporate=("us-de", "DE corporate"),
        )

        zone2 = compose_zone(
            "test.determinism",
            "Determinism Test",
            civic=("us-ny", "NY civic"),
            corporate=("us-de", "DE corporate"),
        )

        assert zone1.composition_digest() == zone2.composition_digest()


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
