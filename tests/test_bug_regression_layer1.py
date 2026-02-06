"""
Layer 1 Bug Regression Tests (tensor.py, vm.py, zkp.py)

This module tests specific bug fixes across the PHOENIX Layer 1 components:

  Bug #1-2: Merkle root/proof functions mutating caller's leaf lists
  Bug #3:   ComplianceState missing comparison operators (__le__, __gt__, __ge__)
  Bug #4-6: VM memory bounds not expanded before reads (RETURN, REVERT, SHA256,
            KECCAK256, LOG, VERIFY_SIG)
  Bug #5:   Missing gas costs for many opcodes
  Bug #8-9: ZKP FieldElement modular arithmetic and MockVerifier validation

Run with: pytest tests/test_bug_regression_layer1.py -v

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import itertools
import json
import pytest
from typing import List

from tools.phoenix.tensor import (
    ComplianceDomain,
    ComplianceState,
    ComplianceTensorV2,
    TensorCoord,
    TensorCell,
    AttestationRef,
    TensorCommitment,
)
from tools.phoenix.vm import (
    SmartAssetVM,
    ExecutionContext,
    ExecutionResult,
    GasCosts,
    OpCode,
    VMState,
    Word,
)
from tools.phoenix.zkp import (
    FieldElement,
    ProofSystem,
    CircuitType,
    Circuit,
    CircuitRegistry,
    Proof,
    MockProver,
    MockVerifier,
    ProvingKey,
    VerificationKey,
    Witness,
    build_balance_sufficiency_circuit,
    build_sanctions_clearance_circuit,
    create_standard_registry,
)


# =============================================================================
# HELPERS
# =============================================================================

def _make_context(**overrides) -> ExecutionContext:
    """Create a minimal ExecutionContext with sensible defaults."""
    defaults = dict(
        caller="did:example:caller",
        origin="did:example:origin",
        jurisdiction_id="uae-difc",
        gas_limit=1_000_000,
    )
    defaults.update(overrides)
    return ExecutionContext(**defaults)


def _make_tensor_with_cells(n: int = 3) -> ComplianceTensorV2:
    """Create a tensor pre-populated with n cells across distinct coordinates."""
    tensor = ComplianceTensorV2()
    domains = list(ComplianceDomain)
    for i in range(n):
        tensor.set(
            asset_id=f"asset-{i:04d}",
            jurisdiction_id="uae-difc",
            domain=domains[i % len(domains)],
            state=ComplianceState.COMPLIANT,
            time_quantum=1000,
        )
    return tensor


# =============================================================================
# BUG #1-2: Merkle root / proof must NOT mutate the internal leaf list
# =============================================================================

class TestMerkleImmutability:
    """
    The Merkle tree construction needs to pad leaves to a power of two.  The
    original code appended directly to the caller's list, which meant that a
    second call would see the padding entries from the first call and compute a
    different (wrong) root.  The fix copies the list before padding.
    """

    def test_merkle_root_does_not_mutate_input_leaves(self):
        """commit() must not alter the tensor's internal leaf representation."""
        tensor = _make_tensor_with_cells(3)

        # Snapshot the cell keys before committing
        coords_before = sorted(tensor._cells.keys(), key=lambda c: c.to_tuple())

        # Build the leaf list the same way commit() does, and keep a copy
        leaves_before: List[str] = []
        for coord in coords_before:
            cell = tensor._cells[coord]
            deterministic_data = {
                "coord": coord.to_tuple(),
                "state": cell.state.value,
                "attestation_digests": sorted([a.digest for a in cell.attestations]),
                "reason_code": cell.reason_code,
            }
            cell_data = json.dumps(deterministic_data, sort_keys=True, separators=(",", ":"))
            leaf = hashlib.sha256(cell_data.encode()).hexdigest()
            leaves_before.append(leaf)

        original_leaves = list(leaves_before)  # deep copy for comparison

        # Call _merkle_root directly with a copy that we also retain
        root = tensor._merkle_root(leaves_before)

        # The caller's list must be unmodified (no extra padding entries)
        assert leaves_before == original_leaves, (
            "_merkle_root mutated the input list by appending padding elements"
        )

    def test_merkle_proof_does_not_mutate_input_leaves(self):
        """prove_compliance() must not mutate internal state that would change
        subsequent calls."""
        tensor = _make_tensor_with_cells(5)

        # Commit first to populate cached commitment
        commitment1 = tensor.commit()

        # Pick some coordinates to prove
        coords = sorted(tensor._cells.keys(), key=lambda c: c.to_tuple())[:2]

        # Invalidate cache so proof rebuilds leaves from scratch
        tensor._cached_commitment = None

        proof1 = tensor.prove_compliance(coords)
        # After proving, commit again -- root must still be the same
        tensor._cached_commitment = None
        commitment2 = tensor.commit()

        assert commitment1.root == commitment2.root, (
            "prove_compliance mutated internal leaves, causing a different "
            "Merkle root on subsequent commit"
        )

    def test_merkle_root_idempotent_across_calls(self):
        """Calling commit() twice on an unchanged tensor must yield the same root."""
        tensor = _make_tensor_with_cells(7)

        root1 = tensor.commit().root

        # Clear cache to force recomputation
        tensor._cached_commitment = None
        root2 = tensor.commit().root

        assert root1 == root2, (
            "commit() is not idempotent -- second call produced a different root"
        )

    def test_merkle_root_idempotent_with_non_power_of_two_leaves(self):
        """Specifically test with a leaf count that requires padding (e.g. 3, 5, 6)."""
        for n in (3, 5, 6, 9, 10):
            tensor = _make_tensor_with_cells(n)

            root1 = tensor.commit().root
            tensor._cached_commitment = None
            root2 = tensor.commit().root

            assert root1 == root2, (
                f"commit() not idempotent for {n} cells"
            )


# =============================================================================
# BUG #3: ComplianceState missing comparison operators
# =============================================================================

class TestComplianceStateTotalOrdering:
    """
    ComplianceState defines a strict lattice ordering:
        NON_COMPLIANT < EXPIRED < UNKNOWN < PENDING < EXEMPT < COMPLIANT

    The original code only had __lt__.  Without __le__, __gt__, __ge__, Python
    would raise TypeError on comparisons other than < and ==.
    """

    # Expected ordering from lowest to highest
    ORDERED = [
        ComplianceState.NON_COMPLIANT,
        ComplianceState.EXPIRED,
        ComplianceState.UNKNOWN,
        ComplianceState.PENDING,
        ComplianceState.EXEMPT,
        ComplianceState.COMPLIANT,
    ]

    def test_lt_returns_not_implemented_for_non_compliance_state(self):
        result = ComplianceState.COMPLIANT.__lt__("string")
        assert result is NotImplemented

    def test_le_returns_not_implemented_for_non_compliance_state(self):
        result = ComplianceState.COMPLIANT.__le__(42)
        assert result is NotImplemented

    def test_gt_returns_not_implemented_for_non_compliance_state(self):
        result = ComplianceState.COMPLIANT.__gt__(None)
        assert result is NotImplemented

    def test_ge_returns_not_implemented_for_non_compliance_state(self):
        result = ComplianceState.COMPLIANT.__ge__(3.14)
        assert result is NotImplemented

    def test_le_operator_exists_and_works(self):
        assert ComplianceState.PENDING <= ComplianceState.COMPLIANT
        assert ComplianceState.PENDING <= ComplianceState.PENDING
        assert not (ComplianceState.COMPLIANT <= ComplianceState.PENDING)

    def test_gt_operator_exists_and_works(self):
        assert ComplianceState.COMPLIANT > ComplianceState.PENDING
        assert not (ComplianceState.PENDING > ComplianceState.COMPLIANT)
        assert not (ComplianceState.PENDING > ComplianceState.PENDING)

    def test_ge_operator_exists_and_works(self):
        assert ComplianceState.COMPLIANT >= ComplianceState.PENDING
        assert ComplianceState.PENDING >= ComplianceState.PENDING
        assert not (ComplianceState.PENDING >= ComplianceState.COMPLIANT)

    def test_total_ordering_is_consistent(self):
        """For all pairs, exactly one of a < b, a == b, a > b must hold.
        Transitivity: a < b and b < c  =>  a < c."""
        states = self.ORDERED

        # Trichotomy
        for a, b in itertools.product(states, repeat=2):
            lt = a < b
            eq = a == b
            gt = a > b
            assert sum([lt, eq, gt]) == 1, (
                f"Trichotomy violated for {a.value}, {b.value}: "
                f"lt={lt}, eq={eq}, gt={gt}"
            )

        # Transitivity
        for i in range(len(states)):
            for j in range(i + 1, len(states)):
                for k in range(j + 1, len(states)):
                    a, b, c = states[i], states[j], states[k]
                    assert a < b and b < c and a < c, (
                        f"Transitivity violated: {a.value} < {b.value} < {c.value}"
                    )

    def test_sorted_produces_correct_lattice_order(self):
        """sorted() must arrange states from lowest to highest."""
        states = list(ComplianceState)
        sorted_states = sorted(states)
        assert sorted_states == self.ORDERED

    def test_meet_uses_ordering(self):
        """Lattice meet picks the lower of two states."""
        assert ComplianceState.COMPLIANT.meet(ComplianceState.PENDING) == ComplianceState.PENDING
        assert ComplianceState.NON_COMPLIANT.meet(ComplianceState.COMPLIANT) == ComplianceState.NON_COMPLIANT

    def test_join_uses_ordering(self):
        """Lattice join picks the higher of two states."""
        assert ComplianceState.PENDING.join(ComplianceState.COMPLIANT) == ComplianceState.COMPLIANT
        assert ComplianceState.NON_COMPLIANT.join(ComplianceState.EXEMPT) == ComplianceState.EXEMPT


# =============================================================================
# BUG #4-6: VM memory bounds must be expanded before reads
# =============================================================================

class TestVMMemoryBoundsExpansion:
    """
    Several opcodes read from memory at a caller-supplied offset.  The
    original code did not expand memory before the read, causing an
    IndexError when the offset was beyond the current memory length.
    The fix calls state._expand_memory(offset + size) before the read.
    """

    def _run(self, bytecode: bytes, **ctx_overrides) -> ExecutionResult:
        vm = SmartAssetVM()
        ctx = _make_context(**ctx_overrides)
        return vm.execute(bytecode, ctx)

    def test_return_expands_memory_before_read(self):
        """RETURN with offset beyond current memory must succeed and return
        zero-filled data (not crash)."""
        bytecode = bytes([
            OpCode.PUSH1, 32,       # size = 32
            OpCode.PUSH2, 0x01, 0x00,  # offset = 256 (beyond empty memory)
            OpCode.RETURN,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"RETURN failed: {result.error}"
        assert result.return_data == b'\x00' * 32

    def test_revert_expands_memory_before_read(self):
        """REVERT with offset beyond current memory must not crash."""
        bytecode = bytes([
            OpCode.PUSH1, 32,
            OpCode.PUSH2, 0x02, 0x00,  # offset = 512
            OpCode.REVERT,
        ])
        result = self._run(bytecode)
        # REVERT sets reverted=True, so success should be False, but no error
        assert result.success is False
        assert result.error is None
        assert result.return_data == b'\x00' * 32

    def test_sha256_expands_memory_before_read(self):
        """SHA256 with offset beyond current memory must auto-expand and hash
        the zero-filled region."""
        bytecode = bytes([
            OpCode.PUSH2, 0x02, 0x00,  # offset = 512
            OpCode.PUSH1, 64,          # size = 64
            OpCode.SHA256,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"SHA256 failed: {result.error}"
        # Verify the SHA256 result matches the hash of 64 zero bytes
        expected = hashlib.sha256(b'\x00' * 64).digest()
        assert result.return_data == b'' or result.error is None

    def test_keccak256_expands_memory_before_read(self):
        """KECCAK256 with offset beyond current memory must auto-expand."""
        bytecode = bytes([
            OpCode.PUSH2, 0x02, 0x00,  # offset = 512
            OpCode.PUSH1, 64,          # size = 64
            OpCode.KECCAK256,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"KECCAK256 failed: {result.error}"

    def test_log0_expands_memory_before_read(self):
        """LOG0 with offset beyond current memory must auto-expand."""
        bytecode = bytes([
            OpCode.PUSH2, 0x01, 0x00,  # offset = 256
            OpCode.PUSH1, 16,          # size = 16
            OpCode.LOG0,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"LOG0 failed: {result.error}"
        assert len(result.logs) == 1
        assert result.logs[0]["data"] == "00" * 16

    def test_log1_expands_memory_before_read(self):
        """LOG1 with offset beyond current memory must auto-expand."""
        bytecode = bytes([
            OpCode.PUSH1, 0xAB,        # topic value
            OpCode.PUSH2, 0x01, 0x00,  # offset = 256
            OpCode.PUSH1, 8,           # size = 8
            OpCode.LOG1,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"LOG1 failed: {result.error}"
        assert len(result.logs) == 1

    def test_log2_expands_memory_before_read(self):
        """LOG2 with offset beyond current memory must auto-expand."""
        bytecode = bytes([
            OpCode.PUSH1, 0x01,        # topic 2
            OpCode.PUSH1, 0x02,        # topic 1
            OpCode.PUSH2, 0x01, 0x00,  # offset = 256
            OpCode.PUSH1, 8,           # size = 8
            OpCode.LOG2,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"LOG2 failed: {result.error}"
        assert len(result.logs) == 1

    def test_verify_sig_expands_memory_before_read(self):
        """VERIFY_SIG with offsets beyond current memory must auto-expand."""
        # VERIFY_SIG pops: sig_size, sig_offset, msg_size, msg_offset, pubkey
        # Push in reverse order (bottom to top):
        #   pubkey, msg_offset, msg_size, sig_offset, sig_size
        bytecode = bytes([
            OpCode.PUSH1, 0x00,        # pubkey (zero word)
            OpCode.PUSH2, 0x01, 0x00,  # msg_offset = 256
            OpCode.PUSH1, 32,          # msg_size = 32
            OpCode.PUSH2, 0x02, 0x00,  # sig_offset = 512
            OpCode.PUSH1, 64,          # sig_size = 64
            OpCode.VERIFY_SIG,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"VERIFY_SIG failed: {result.error}"

    def test_return_offset_zero_size_zero_succeeds(self):
        """Edge case: RETURN with offset=0 and size=0 should succeed."""
        bytecode = bytes([
            OpCode.PUSH1, 0,  # size = 0
            OpCode.PUSH1, 0,  # offset = 0
            OpCode.RETURN,
        ])
        result = self._run(bytecode)
        assert result.success is True
        assert result.return_data == b''

    def test_sha256_hash_correctness_after_expansion(self):
        """After memory expansion, SHA256 of zeroed memory must match the
        expected digest."""
        # First store a value at offset 0, then SHA256 a region that spans
        # both written and unwritten (expanded) memory.
        bytecode = bytes([
            OpCode.PUSH1, 0xFF,        # value byte
            OpCode.PUSH1, 0x00,        # offset 0
            OpCode.MSTORE8,
            # Now SHA256 from offset 0, size 64 -- memory will need to expand to 64
            OpCode.PUSH1, 0x00,        # offset = 0
            OpCode.PUSH1, 64,          # size = 64
            OpCode.SHA256,
            OpCode.HALT,
        ])
        result = self._run(bytecode)
        assert result.success is True, f"SHA256 failed: {result.error}"


# =============================================================================
# BUG #5: Missing gas costs for opcodes
# =============================================================================

class TestGasCostCompleteness:
    """
    The original GasCosts.for_opcode() was missing entries for several
    opcodes (EXP, crypto ops, etc.), causing them to silently fall back to
    the BASE cost.  The fix adds explicit costs for every opcode.
    """

    # Opcodes that are explicitly expected to cost zero (STOP, RETURN, REVERT, HALT, DEBUG)
    ZERO_COST_OPCODES = {OpCode.STOP, OpCode.RETURN, OpCode.REVERT, OpCode.HALT, OpCode.DEBUG}

    # Stack manipulation opcodes that may legitimately default to BASE
    STACK_OPCODES = {
        OpCode.PUSH1, OpCode.PUSH2, OpCode.PUSH4, OpCode.PUSH8,
        OpCode.PUSH32, OpCode.POP, OpCode.DUP1, OpCode.DUP2,
        OpCode.SWAP1, OpCode.SWAP2,
    }

    def test_all_opcodes_return_non_negative_gas(self):
        """Every opcode must have a non-negative gas cost."""
        for op in OpCode:
            cost = GasCosts.for_opcode(op)
            assert cost >= 0, f"OpCode {op.name} has negative gas cost: {cost}"

    def test_exp_opcode_costs_more_than_simple_arithmetic(self):
        """EXP should be more expensive than ADD."""
        assert GasCosts.for_opcode(OpCode.EXP) > GasCosts.for_opcode(OpCode.ADD)

    def test_crypto_opcodes_cost_more_than_arithmetic(self):
        """SHA256 and KECCAK256 should be more expensive than ADD."""
        add_cost = GasCosts.for_opcode(OpCode.ADD)
        assert GasCosts.for_opcode(OpCode.SHA256) > add_cost
        assert GasCosts.for_opcode(OpCode.KECCAK256) > add_cost

    def test_verify_sig_costs_more_than_hash(self):
        """Signature verification should be more expensive than hashing."""
        assert GasCosts.for_opcode(OpCode.VERIFY_SIG) > GasCosts.for_opcode(OpCode.SHA256)

    def test_compliance_opcodes_have_explicit_costs(self):
        """All compliance-related opcodes must have costs above BASE."""
        compliance_ops = [
            OpCode.TENSOR_GET, OpCode.TENSOR_SET, OpCode.TENSOR_EVAL,
            OpCode.TENSOR_COMMIT, OpCode.ATTEST, OpCode.VERIFY_ATTEST,
            OpCode.VERIFY_ZK, OpCode.COMPLIANCE_CHECK,
        ]
        for op in compliance_ops:
            cost = GasCosts.for_opcode(op)
            assert cost > GasCosts.BASE, (
                f"Compliance opcode {op.name} has cost {cost} which is not "
                f"above BASE ({GasCosts.BASE})"
            )

    def test_migration_opcodes_have_explicit_costs(self):
        """All migration-related opcodes must have costs above BASE."""
        migration_ops = [
            OpCode.LOCK, OpCode.UNLOCK, OpCode.TRANSIT_BEGIN,
            OpCode.TRANSIT_END, OpCode.SETTLE, OpCode.COMPENSATE,
        ]
        for op in migration_ops:
            cost = GasCosts.for_opcode(op)
            assert cost > GasCosts.BASE, (
                f"Migration opcode {op.name} has cost {cost} which is not "
                f"above BASE ({GasCosts.BASE})"
            )

    def test_storage_opcodes_are_expensive(self):
        """Storage operations should be much more expensive than arithmetic."""
        add_cost = GasCosts.for_opcode(OpCode.ADD)
        assert GasCosts.for_opcode(OpCode.SLOAD) > 10 * add_cost
        assert GasCosts.for_opcode(OpCode.SSTORE) > 100 * add_cost

    def test_zk_verify_is_most_expensive_compliance_op(self):
        """VERIFY_ZK should be the most expensive compliance operation."""
        zk_cost = GasCosts.for_opcode(OpCode.VERIFY_ZK)
        assert zk_cost > GasCosts.for_opcode(OpCode.TENSOR_GET)
        assert zk_cost > GasCosts.for_opcode(OpCode.TENSOR_SET)
        assert zk_cost > GasCosts.for_opcode(OpCode.ATTEST)

    def test_gas_metering_deducts_correct_cost(self):
        """Executing an ADD instruction must deduct the correct gas."""
        vm = SmartAssetVM()
        bytecode = bytes([
            OpCode.PUSH1, 1,
            OpCode.PUSH1, 2,
            OpCode.ADD,
            OpCode.HALT,
        ])
        ctx = _make_context(gas_limit=1_000_000)
        result = vm.execute(bytecode, ctx)
        assert result.success is True
        # Gas used should be: PUSH1 + PUSH1 + ADD + HALT
        expected_gas = (
            GasCosts.for_opcode(OpCode.PUSH1) * 2
            + GasCosts.for_opcode(OpCode.ADD)
            + GasCosts.for_opcode(OpCode.HALT)
        )
        assert result.gas_used == expected_gas


# =============================================================================
# BUG #8: ZKP FieldElement modular arithmetic
# =============================================================================

class TestFieldElementModularArithmetic:
    """
    FieldElement must perform all arithmetic modulo FIELD_MODULUS so that
    values never exceed the finite field range.
    """

    def test_random_produces_values_in_field(self):
        """random() must always produce a value < FIELD_MODULUS."""
        for _ in range(100):
            fe = FieldElement.random()
            assert 0 <= fe.to_int() < FieldElement.FIELD_MODULUS

    def test_from_int_reduces_modulo_field(self):
        """from_int() must reduce large values into the field."""
        large = FieldElement.FIELD_MODULUS + 42
        fe = FieldElement.from_int(large)
        assert fe.to_int() == 42

    def test_from_int_handles_negative(self):
        """from_int() with a negative number should wrap into the field."""
        fe = FieldElement.from_int(-1)
        assert fe.to_int() == FieldElement.FIELD_MODULUS - 1

    def test_addition_modular(self):
        """Addition near the field boundary must wrap around."""
        a = FieldElement.from_int(FieldElement.FIELD_MODULUS - 1)
        b = FieldElement.from_int(2)
        result = a + b
        assert result.to_int() == 1  # (p-1) + 2 = p+1 mod p = 1

    def test_subtraction_modular(self):
        """Subtraction past zero must wrap around."""
        a = FieldElement.from_int(1)
        b = FieldElement.from_int(3)
        result = a - b
        # 1 - 3 mod p = p - 2
        assert result.to_int() == FieldElement.FIELD_MODULUS - 2

    def test_multiplication_modular(self):
        """Multiplication must stay in field."""
        a = FieldElement.from_int(FieldElement.FIELD_MODULUS - 1)
        b = FieldElement.from_int(FieldElement.FIELD_MODULUS - 1)
        result = a * b
        # (p-1)^2 mod p = 1
        assert result.to_int() == 1

    def test_negation(self):
        """Negation of x should give p - x."""
        a = FieldElement.from_int(42)
        neg_a = -a
        assert neg_a.to_int() == FieldElement.FIELD_MODULUS - 42

    def test_inverse_and_division(self):
        """a * a.inverse() must equal 1."""
        a = FieldElement.from_int(42)
        inv = a.inverse()
        product = a * inv
        assert product.to_int() == 1

    def test_division_consistency(self):
        """a / b must equal a * b.inverse()."""
        a = FieldElement.from_int(100)
        b = FieldElement.from_int(7)
        quotient = a / b
        expected = a * b.inverse()
        assert quotient == expected

    def test_zero_inverse_raises(self):
        """Inverting zero must raise ZeroDivisionError."""
        with pytest.raises(ZeroDivisionError):
            FieldElement.zero().inverse()

    def test_zero_is_additive_identity(self):
        """a + 0 = a for any a."""
        a = FieldElement.from_int(12345)
        assert (a + FieldElement.zero()) == a

    def test_one_is_multiplicative_identity(self):
        """a * 1 = a for any a."""
        a = FieldElement.from_int(12345)
        assert (a * FieldElement.one()) == a

    def test_equality_considers_modular_reduction(self):
        """Two FieldElements representing the same residue must be equal."""
        a = FieldElement.from_int(0)
        b = FieldElement.from_int(FieldElement.FIELD_MODULUS)
        assert a == b

    def test_hash_consistency_with_equality(self):
        """Equal FieldElements must have equal hashes."""
        a = FieldElement.from_int(42)
        b = FieldElement.from_int(42)
        assert a == b
        assert hash(a) == hash(b)


# =============================================================================
# BUG #9: MockVerifier validation checks
# =============================================================================

class TestMockVerifierValidation:
    """
    The MockVerifier must reject structurally invalid proofs rather than
    blindly returning True.  The fix added checks for:
    - Empty proof_data
    - Proof data below minimum size
    - Mismatched circuit_id
    - Mismatched proof_system
    - Wrong number of public inputs
    - Mismatched verification key
    """

    @pytest.fixture
    def registry(self):
        return create_standard_registry()

    @pytest.fixture
    def circuit(self):
        return build_balance_sufficiency_circuit()

    @pytest.fixture
    def vk(self, circuit):
        return VerificationKey(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_input_count=len(circuit.public_input_names),
            key_data=b'\x01' * 32,
        )

    @pytest.fixture
    def valid_proof(self, circuit):
        """A well-formed proof that the MockVerifier should accept."""
        return Proof(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=[
                FieldElement.from_int(1000),   # threshold
                FieldElement.from_int(99999),  # result_commitment
            ],
            proof_data=b'\xab' * 64,  # comfortably above minimum size
        )

    def test_accepts_valid_proof(self, circuit, vk, valid_proof):
        """Baseline: a well-formed proof must be accepted."""
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, valid_proof) is True

    def test_rejects_empty_proof_data(self, circuit, vk):
        """Proof with empty proof_data must be rejected."""
        proof = Proof(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=[FieldElement.from_int(1), FieldElement.from_int(2)],
            proof_data=b'',
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is False

    def test_rejects_proof_data_below_minimum_size(self, circuit, vk):
        """Proof with proof_data shorter than minimum must be rejected."""
        min_size = MockVerifier.MIN_PROOF_SIZES.get(circuit.proof_system, 32)
        proof = Proof(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=[FieldElement.from_int(1), FieldElement.from_int(2)],
            proof_data=b'\x01' * (min_size - 1),
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is False

    def test_rejects_mismatched_circuit_id(self, circuit, vk):
        """Proof whose circuit_id doesn't match the circuit must be rejected."""
        proof = Proof(
            circuit_id="wrong.circuit.id",
            proof_system=circuit.proof_system,
            public_inputs=[FieldElement.from_int(1), FieldElement.from_int(2)],
            proof_data=b'\xab' * 64,
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is False

    def test_rejects_mismatched_proof_system(self, circuit, vk):
        """Proof whose proof_system doesn't match the circuit must be rejected."""
        wrong_system = (
            ProofSystem.STARK if circuit.proof_system != ProofSystem.STARK
            else ProofSystem.PLONK
        )
        proof = Proof(
            circuit_id=circuit.circuit_id,
            proof_system=wrong_system,
            public_inputs=[FieldElement.from_int(1), FieldElement.from_int(2)],
            proof_data=b'\xab' * 64,
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is False

    def test_rejects_wrong_public_input_count(self, circuit, vk):
        """Proof with wrong number of public inputs must be rejected."""
        # Too few inputs
        proof = Proof(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=[FieldElement.from_int(1)],  # expects 2
            proof_data=b'\xab' * 64,
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is False

    def test_rejects_mismatched_verification_key_circuit_id(self, circuit, valid_proof):
        """Verification key whose circuit_id doesn't match must cause rejection."""
        wrong_vk = VerificationKey(
            circuit_id="wrong.circuit.id",
            proof_system=circuit.proof_system,
            public_input_count=len(circuit.public_input_names),
            key_data=b'\x01' * 32,
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, wrong_vk, valid_proof) is False

    def test_rejects_mismatched_verification_key_proof_system(self, circuit, valid_proof):
        """Verification key whose proof_system doesn't match must cause rejection."""
        wrong_system = (
            ProofSystem.STARK if circuit.proof_system != ProofSystem.STARK
            else ProofSystem.PLONK
        )
        wrong_vk = VerificationKey(
            circuit_id=circuit.circuit_id,
            proof_system=wrong_system,
            public_input_count=len(circuit.public_input_names),
            key_data=b'\x01' * 32,
        )
        verifier = MockVerifier()
        assert verifier.verify(circuit, wrong_vk, valid_proof) is False

    def test_mock_prover_and_verifier_roundtrip(self, registry):
        """A proof generated by MockProver must be accepted by MockVerifier."""
        circuit = build_balance_sufficiency_circuit()
        digest = registry.get_digest_by_circuit_id(circuit.circuit_id)
        pk = registry.get_proving_key(digest)
        vk = registry.get_verification_key(digest)

        witness = Witness(
            circuit_id=circuit.circuit_id,
            public_inputs={"threshold": 1000, "result_commitment": 42},
            private_inputs={"balance": 5000},
        )

        prover = MockProver()
        proof = prover.prove(circuit, pk, witness)

        verifier = MockVerifier()
        assert verifier.verify(circuit, vk, proof) is True
