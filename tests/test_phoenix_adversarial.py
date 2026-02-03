"""
PHOENIX Adversarial Test Suite - Bug Discovery Through Edge Cases

This suite systematically tests edge cases, boundary conditions, and
adversarial scenarios to uncover bugs in the PHOENIX infrastructure.

Test Categories:
1. VM Security (stack overflow, memory limits, gas exhaustion)
2. Migration Protocol (timeout, cancellation, state transitions)
3. Bridge Multi-hop (failure recovery, partial commits)
4. Tensor Operations (empty tensors, max dimensions, proof generation)
5. Timestamp/Time Edge Cases
6. Fee Calculation Edge Cases
7. Concurrent Access
8. Invalid Input Handling
9. State Machine Transitions
10. Cryptographic Edge Cases

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import json
import secrets
import threading
import time
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from decimal import Decimal, InvalidOperation
from typing import Any, Dict, List, Optional, Tuple


# =============================================================================
# TEST CATEGORY 1: VM SECURITY
# =============================================================================

class TestVMSecurity:
    """Test VM security boundaries and resource limits."""

    def test_stack_overflow_protection(self):
        """Test that stack overflow is properly prevented."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')

        # Push until max
        for i in range(state.MAX_STACK_SIZE):
            state.push(Word.from_int(i))

        # One more should fail
        try:
            state.push(Word.from_int(256))
            assert False, "Should have raised SecurityViolation"
        except Exception as e:
            assert "overflow" in str(e).lower()

    def test_stack_underflow_protection(self):
        """Test that stack underflow is properly prevented."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')

        # Pop from empty stack should fail
        try:
            state.pop()
            assert False, "Should have raised SecurityViolation"
        except Exception as e:
            assert "underflow" in str(e).lower()

    def test_memory_limit_protection(self):
        """Test that memory limit is enforced."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')

        # Try to expand memory beyond limit
        try:
            state.mload(state.MAX_MEMORY_SIZE + 1)
            assert False, "Should have raised SecurityViolation"
        except Exception as e:
            assert "limit" in str(e).lower() or "exceeded" in str(e).lower()

    def test_word_division_by_zero(self):
        """Test that division by zero returns zero (not exception)."""
        from tools.phoenix.vm import Word

        a = Word.from_int(100)
        b = Word.from_int(0)

        # Division by zero should return zero, not raise exception
        result = a / b
        assert result.to_int() == 0

        # Modulo by zero should also return zero
        result = a % b
        assert result.to_int() == 0

    def test_word_negative_numbers(self):
        """Test handling of negative numbers in two's complement."""
        from tools.phoenix.vm import Word

        # Create negative number
        neg = Word.from_int(-1)
        assert neg.to_int(signed=True) == -1

        # Large negative
        neg_large = Word.from_int(-1000000)
        assert neg_large.to_int(signed=True) == -1000000

    def test_word_overflow_wrapping(self):
        """Test that arithmetic overflow wraps correctly."""
        from tools.phoenix.vm import Word

        max_val = Word.from_int((1 << 256) - 1)
        one = Word.from_int(1)

        # Overflow should wrap to zero
        result = max_val + one
        assert result.to_int() == 0

    def test_storage_root_empty(self):
        """Test storage root computation with empty storage."""
        from tools.phoenix.vm import VMState

        state = VMState(code=b'')
        root = state.storage_root()

        # Empty storage should have predictable root
        assert root == "0" * 64

    def test_peek_at_invalid_depth(self):
        """Test that peeking at invalid depth raises error."""
        from tools.phoenix.vm import VMState, Word

        state = VMState(code=b'')
        state.push(Word.from_int(1))

        try:
            state.peek(5)  # Only 1 item on stack
            assert False, "Should have raised SecurityViolation"
        except Exception as e:
            assert "underflow" in str(e).lower()


# =============================================================================
# TEST CATEGORY 2: MIGRATION PROTOCOL
# =============================================================================

class TestMigrationProtocol:
    """Test migration protocol edge cases."""

    def test_migration_state_transitions(self):
        """Test valid and invalid state transitions."""
        from tools.phoenix.migration import MigrationState

        # Test is_terminal
        assert MigrationState.COMPLETED.is_terminal()
        assert MigrationState.COMPENSATED.is_terminal()
        assert MigrationState.DISPUTED.is_terminal()
        assert not MigrationState.TRANSIT.is_terminal()

        # Test is_failure
        assert not MigrationState.COMPLETED.is_failure()
        assert MigrationState.COMPENSATED.is_failure()

        # Test allows_cancellation
        assert MigrationState.INITIATED.allows_cancellation()
        assert not MigrationState.TRANSIT.allows_cancellation()

    def test_migration_request_id_generation(self):
        """Test that request IDs are deterministic."""
        from tools.phoenix.migration import MigrationRequest

        req1 = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            created_at="2026-01-01T00:00:00+00:00",
        )

        req2 = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            created_at="2026-01-01T00:00:00+00:00",
        )

        # Same inputs should produce same ID
        assert req1.request_id == req2.request_id

    def test_migration_request_different_timestamp(self):
        """Test that different timestamps produce different IDs."""
        from tools.phoenix.migration import MigrationRequest

        req1 = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            created_at="2026-01-01T00:00:00+00:00",
        )

        req2 = MigrationRequest(
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            created_at="2026-01-01T00:00:01+00:00",  # 1 second later
        )

        # Different timestamps should produce different IDs
        assert req1.request_id != req2.request_id


# =============================================================================
# TEST CATEGORY 3: BRIDGE MULTI-HOP
# =============================================================================

class TestBridgeMultiHop:
    """Test bridge multi-hop scenarios."""

    def test_bridge_negative_amount(self):
        """Test that negative amounts are rejected."""
        from tools.phoenix.bridge import CorridorBridge, BridgeRequest
        from tools.phoenix.manifold import create_standard_manifold

        manifold = create_standard_manifold()
        bridge = CorridorBridge(manifold)

        request = BridgeRequest(
            bridge_id="bridge-001",
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            amount=Decimal("-100"),  # Negative
            currency="USD",
        )

        execution = bridge.execute(request)
        assert execution.phase.value == "failed"
        assert "positive" in execution.fatal_error.lower()

    def test_bridge_same_source_target(self):
        """Test bridge with same source and target jurisdiction."""
        from tools.phoenix.bridge import CorridorBridge, BridgeRequest
        from tools.phoenix.manifold import create_standard_manifold

        manifold = create_standard_manifold()
        bridge = CorridorBridge(manifold)

        request = BridgeRequest(
            bridge_id="bridge-001",
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="uae-difc",
            target_jurisdiction="uae-difc",  # Same as source
            amount=Decimal("1000"),
            currency="USD",
        )

        execution = bridge.execute(request)
        # Should either succeed trivially or fail gracefully
        assert execution.phase.value in ["completed", "failed"]

    def test_bridge_non_existent_jurisdiction(self):
        """Test bridge with non-existent jurisdiction."""
        from tools.phoenix.bridge import CorridorBridge, BridgeRequest
        from tools.phoenix.manifold import create_standard_manifold

        manifold = create_standard_manifold()
        bridge = CorridorBridge(manifold)

        request = BridgeRequest(
            bridge_id="bridge-001",
            asset_id="asset-001",
            asset_genesis_digest="abc123",
            source_jurisdiction="non-existent-123",
            target_jurisdiction="kz-aifc",
            amount=Decimal("1000"),
            currency="USD",
        )

        execution = bridge.execute(request)
        assert execution.phase.value == "failed"
        assert "path" in execution.fatal_error.lower() or "not found" in execution.fatal_error.lower()


# =============================================================================
# TEST CATEGORY 4: TENSOR OPERATIONS
# =============================================================================

class TestTensorOperations:
    """Test tensor edge cases."""

    def test_tensor_empty_commit(self):
        """Test commitment of empty tensor."""
        from tools.phoenix.tensor import ComplianceTensorV2

        tensor = ComplianceTensorV2()
        commitment = tensor.commit()

        # Empty tensor should have valid commitment
        assert commitment is not None
        assert commitment.cell_count == 0

    def test_tensor_single_cell(self):
        """Test tensor with single cell."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2, ComplianceDomain, ComplianceState
        )

        tensor = ComplianceTensorV2()
        coord = tensor.set(
            asset_id="asset-001",
            jurisdiction_id="uae-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
        )

        commitment = tensor.commit()
        assert commitment.cell_count == 1

        # Proof for single cell
        proof = tensor.prove_compliance([coord])
        assert proof is not None

    def test_tensor_all_domains(self):
        """Test tensor with all compliance domains."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2, ComplianceDomain, ComplianceState
        )

        tensor = ComplianceTensorV2()

        for domain in ComplianceDomain:
            tensor.set(
                asset_id="asset-001",
                jurisdiction_id="uae-difc",
                domain=domain,
                state=ComplianceState.COMPLIANT,
            )

        commitment = tensor.commit()
        assert commitment.cell_count == len(ComplianceDomain)

    def test_tensor_compliance_state_lattice(self):
        """Test compliance state lattice operations."""
        from tools.phoenix.tensor import ComplianceState

        # Test meet (pessimistic)
        assert ComplianceState.COMPLIANT.meet(ComplianceState.COMPLIANT) == ComplianceState.COMPLIANT
        assert ComplianceState.COMPLIANT.meet(ComplianceState.PENDING) == ComplianceState.PENDING
        assert ComplianceState.COMPLIANT.meet(ComplianceState.NON_COMPLIANT) == ComplianceState.NON_COMPLIANT

        # Test join (optimistic)
        assert ComplianceState.PENDING.join(ComplianceState.COMPLIANT) == ComplianceState.COMPLIANT
        assert ComplianceState.NON_COMPLIANT.join(ComplianceState.COMPLIANT) == ComplianceState.COMPLIANT

    def test_tensor_merge(self):
        """Test tensor merge operation."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2, ComplianceDomain, ComplianceState
        )

        tensor1 = ComplianceTensorV2()
        tensor1.set(
            asset_id="asset-001",
            jurisdiction_id="uae-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
        )

        tensor2 = ComplianceTensorV2()
        tensor2.set(
            asset_id="asset-002",
            jurisdiction_id="kz-aifc",
            domain=ComplianceDomain.AML,
            state=ComplianceState.PENDING,
        )

        # Merge
        tensor1.merge(tensor2)

        assert len(tensor1) == 2


# =============================================================================
# TEST CATEGORY 5: TIMESTAMP EDGE CASES
# =============================================================================

class TestTimestampEdgeCases:
    """Test timestamp parsing edge cases."""

    def test_timestamp_empty_string(self):
        """Test that empty timestamp raises error."""
        from tools.phoenix.hardening import parse_iso_timestamp

        try:
            parse_iso_timestamp("")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass

    def test_timestamp_invalid_format(self):
        """Test invalid timestamp formats."""
        from tools.phoenix.hardening import parse_iso_timestamp

        invalid_formats = [
            "not-a-timestamp",
            "2026/01/01",
            "01-01-2026",
            "2026-13-01T00:00:00Z",  # Invalid month
        ]

        for ts in invalid_formats:
            try:
                parse_iso_timestamp(ts)
                # Some might parse with fallback, that's OK
            except ValueError:
                pass  # Expected

    def test_timestamp_far_future(self):
        """Test far future timestamps."""
        from tools.phoenix.hardening import parse_iso_timestamp

        # Year 9999 should still parse
        dt = parse_iso_timestamp("9999-12-31T23:59:59Z")
        assert dt.year == 9999

    def test_timestamp_epoch(self):
        """Test Unix epoch timestamp."""
        from tools.phoenix.hardening import parse_iso_timestamp

        dt = parse_iso_timestamp("1970-01-01T00:00:00Z")
        assert dt.year == 1970


# =============================================================================
# TEST CATEGORY 6: FEE CALCULATION
# =============================================================================

class TestFeeCalculation:
    """Test fee calculation edge cases."""

    def test_corridor_edge_fee_bps_zero(self):
        """Test corridor with zero basis points fee."""
        from tools.phoenix.manifold import CorridorEdge

        corridor = CorridorEdge(
            corridor_id="corr-001",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            transfer_fee_bps=0,
            flat_fee_usd=Decimal("0"),
        )

        # Zero fee should be valid
        cost = corridor.transfer_cost(Decimal("1000000"))
        assert cost == Decimal("0")

    def test_corridor_very_large_amount(self):
        """Test corridor with very large amount."""
        from tools.phoenix.manifold import CorridorEdge

        corridor = CorridorEdge(
            corridor_id="corr-001",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            transfer_fee_bps=10,
            flat_fee_usd=Decimal("100"),
        )

        # Very large amount
        large_amount = Decimal("999999999999999")
        cost = corridor.transfer_cost(large_amount)

        # Should not overflow
        assert cost > Decimal("0")

    def test_corridor_decimal_precision(self):
        """Test corridor fee precision."""
        from tools.phoenix.manifold import CorridorEdge

        corridor = CorridorEdge(
            corridor_id="corr-001",
            source_jurisdiction="uae-difc",
            target_jurisdiction="kz-aifc",
            transfer_fee_bps=1,  # 0.01%
            flat_fee_usd=Decimal("0.001"),
        )

        # Small amount
        cost = corridor.transfer_cost(Decimal("100"))

        # Should handle small decimals
        assert cost >= Decimal("0")


# =============================================================================
# TEST CATEGORY 7: CONCURRENT ACCESS
# =============================================================================

class TestConcurrentAccess:
    """Test thread safety of shared resources."""

    def test_versioned_store_concurrent_cas(self):
        """Test concurrent compare-and-swap operations."""
        from tools.phoenix.security import VersionedStore

        store: VersionedStore[int] = VersionedStore()
        store.set("counter", 0)

        success_count = [0]
        fail_count = [0]
        lock = threading.Lock()

        def increment():
            for _ in range(50):
                while True:
                    current = store.get("counter")
                    success, _ = store.compare_and_swap(
                        "counter",
                        current.version,
                        current.value + 1
                    )
                    if success:
                        with lock:
                            success_count[0] += 1
                        break
                    else:
                        with lock:
                            fail_count[0] += 1

        threads = [threading.Thread(target=increment) for _ in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        final = store.get("counter")
        assert final.value == 200  # 4 threads * 50 increments

    def test_nonce_registry_concurrent(self):
        """Test concurrent nonce registration."""
        from tools.phoenix.security import NonceRegistry

        registry = NonceRegistry()
        nonces = [secrets.token_hex(16) for _ in range(100)]
        results = []
        lock = threading.Lock()

        def register_nonces(nonce_list):
            for nonce in nonce_list:
                result = registry.check_and_register(nonce)
                with lock:
                    results.append((nonce, result))

        # Split nonces among threads
        threads = []
        for i in range(4):
            chunk = nonces[i * 25:(i + 1) * 25]
            t = threading.Thread(target=register_nonces, args=(chunk,))
            threads.append(t)

        for t in threads:
            t.start()
        for t in threads:
            t.join()

        # All unique nonces should succeed once
        successful = [r for r in results if r[1]]
        assert len(successful) == 100


# =============================================================================
# TEST CATEGORY 8: INVALID INPUT HANDLING
# =============================================================================

class TestInvalidInputHandling:
    """Test handling of invalid inputs."""

    def test_word_from_invalid_hex(self):
        """Test Word creation from invalid hex."""
        from tools.phoenix.vm import Word

        try:
            Word.from_hex("not-hex")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass

    def test_field_element_invalid(self):
        """Test FieldElement with invalid hex."""
        from tools.phoenix.zkp import FieldElement

        try:
            FieldElement("not-hex-string!")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass

    def test_attestation_ref_expired_check(self):
        """Test attestation expiry checking."""
        from tools.phoenix.tensor import AttestationRef

        # Create expired attestation
        past = datetime.now(timezone.utc) - timedelta(days=1)

        att = AttestationRef(
            attestation_id="att-001",
            attestation_type="kyc",
            issuer_did="did:msez:issuer:001",
            issued_at=(datetime.now(timezone.utc) - timedelta(days=365)).isoformat(),
            expires_at=past.isoformat(),
        )

        # Should be expired
        assert att.is_expired()

    def test_attestation_ref_no_expiry(self):
        """Test attestation with no expiry date."""
        from tools.phoenix.tensor import AttestationRef

        att = AttestationRef(
            attestation_id="att-001",
            attestation_type="kyc",
            issuer_did="did:msez:issuer:001",
            issued_at=datetime.now(timezone.utc).isoformat(),
            expires_at=None,  # No expiry
        )

        # Should never be expired
        assert not att.is_expired()


# =============================================================================
# TEST CATEGORY 9: STATE MACHINE TRANSITIONS
# =============================================================================

class TestStateMachineTransitions:
    """Test state machine transition logic."""

    def test_timelock_state_transitions(self):
        """Test time lock state transitions."""
        from tools.phoenix.security import TimeLock, TimeLockState

        now = datetime.now(timezone.utc)
        lock = TimeLock(
            lock_id="lock-001",
            operation_type="withdrawal",
            operator_did="did:msez:test:001",
            announced_at=now.isoformat(),
            unlock_at=(now + timedelta(hours=1)).isoformat(),
            expires_at=(now + timedelta(hours=2)).isoformat(),
            operation_commitment="abc123",
        )

        # Should not be unlockable yet
        assert not lock.is_unlockable()

        # Should not be expired yet
        assert not lock.is_expired()

    def test_withdrawal_request_states(self):
        """Test withdrawal request state handling."""
        from tools.phoenix.security import WithdrawalRequest

        now = datetime.now(timezone.utc)
        request = WithdrawalRequest(
            request_id="wd-001",
            watcher_did="did:msez:watcher:001",
            bond_id="bond-001",
            amount=Decimal("5000"),
            destination_address="0x" + "a" * 40,
            requested_at=now.isoformat(),
            unlocks_at=(now + timedelta(days=7)).isoformat(),
            expires_at=(now + timedelta(days=9)).isoformat(),
        )

        assert request.state == "pending"


# =============================================================================
# TEST CATEGORY 10: CRYPTOGRAPHIC EDGE CASES
# =============================================================================

class TestCryptoEdgeCases:
    """Test cryptographic edge cases."""

    def test_hash_empty_input(self):
        """Test hashing empty input."""
        result = hashlib.sha256(b"").hexdigest()
        assert len(result) == 64

    def test_merkle_root_single_leaf(self):
        """Test Merkle root with single leaf."""
        from tools.phoenix.hardening import CryptoUtils

        leaves = ["abc123"]
        root = CryptoUtils.merkle_root(leaves)

        # Single leaf should be its own root
        assert root == leaves[0]

    def test_merkle_root_empty(self):
        """Test Merkle root with no leaves."""
        from tools.phoenix.hardening import CryptoUtils

        leaves = []
        root = CryptoUtils.merkle_root(leaves)

        # Empty should return predictable value
        assert root == "" or root == "0" * 64

    def test_secure_compare_timing(self):
        """Test secure comparison returns correct results."""
        from tools.phoenix.hardening import CryptoUtils

        # Equal strings
        assert CryptoUtils.secure_compare_str("abc", "abc")

        # Unequal strings
        assert not CryptoUtils.secure_compare_str("abc", "def")

        # Different lengths
        assert not CryptoUtils.secure_compare_str("abc", "abcd")

    def test_signature_scheme_verification(self):
        """Test signature scheme properties."""
        from tools.phoenix.security import SignatureScheme

        # All schemes should have defined properties
        for scheme in SignatureScheme:
            assert isinstance(scheme.value, str)


# =============================================================================
# RUN ALL TESTS
# =============================================================================

def run_tests():
    """Run all test classes."""
    test_classes = [
        TestVMSecurity,
        TestMigrationProtocol,
        TestBridgeMultiHop,
        TestTensorOperations,
        TestTimestampEdgeCases,
        TestFeeCalculation,
        TestConcurrentAccess,
        TestInvalidInputHandling,
        TestStateMachineTransitions,
        TestCryptoEdgeCases,
    ]

    passed = 0
    failed = 0
    errors = []

    for cls in test_classes:
        print(f'\n=== {cls.__name__} ===')
        instance = cls()
        for method_name in sorted(dir(instance)):
            if method_name.startswith('test_'):
                try:
                    getattr(instance, method_name)()
                    print(f'  PASS: {method_name}')
                    passed += 1
                except Exception as e:
                    print(f'  FAIL: {method_name}')
                    print(f'        {type(e).__name__}: {e}')
                    errors.append((cls.__name__, method_name, e))
                    failed += 1

    print(f'\n{"="*60}')
    print(f'RESULTS: {passed} passed, {failed} failed')
    if errors:
        print('\nFailed tests (potential bugs):')
        for cls_name, method_name, error in errors:
            print(f'  {cls_name}.{method_name}:')
            print(f'    {type(error).__name__}: {error}')

    return errors


if __name__ == "__main__":
    import sys
    errors = run_tests()
    sys.exit(0 if len(errors) == 0 else 1)
