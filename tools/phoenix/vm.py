"""
PHOENIX Smart Asset Virtual Machine (SAVM)

A stack-based execution environment for Smart Asset operations across the
decentralized MSEZ network. The VM provides deterministic execution of
compliance operations, migration protocols, and inter-jurisdictional transactions.

Architecture:

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                     SMART ASSET VIRTUAL MACHINE                          │
    │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
    │  │    STACK    │  │    MEMORY   │  │   STORAGE   │  │    WORLD    │    │
    │  │  (256 slots)│  │  (64KB max) │  │  (Merkleized)│  │   STATE     │    │
    │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘    │
    │                                                                          │
    │  ┌─────────────────────────────────────────────────────────────────┐   │
    │  │                    INSTRUCTION DECODER                            │   │
    │  │  Arithmetic | Stack | Memory | Storage | Control | Compliance    │   │
    │  └─────────────────────────────────────────────────────────────────┘   │
    │                                                                          │
    │  ┌─────────────────────────────────────────────────────────────────┐   │
    │  │                    COMPLIANCE COPROCESSOR                         │   │
    │  │  Tensor Ops | ZK Verify | Attestation Check | Migration FSM      │   │
    │  └─────────────────────────────────────────────────────────────────┘   │
    └─────────────────────────────────────────────────────────────────────────┘

Instruction Categories:

    0x00-0x0F: Stack Operations (PUSH, POP, DUP, SWAP)
    0x10-0x1F: Arithmetic (ADD, SUB, MUL, DIV, MOD)
    0x20-0x2F: Comparison (EQ, LT, GT, AND, OR, NOT)
    0x30-0x3F: Memory (MLOAD, MSTORE, MSIZE)
    0x40-0x4F: Storage (SLOAD, SSTORE, SDELETE)
    0x50-0x5F: Control Flow (JUMP, JUMPI, CALL, RETURN, REVERT)
    0x60-0x6F: Context (CALLER, ORIGIN, JURISDICTION, TIMESTAMP)
    0x70-0x7F: Compliance (TENSOR_GET, TENSOR_SET, ATTEST, VERIFY_ZK)
    0x80-0x8F: Migration (LOCK, UNLOCK, TRANSIT, SETTLE)
    0x90-0x9F: Crypto (HASH, VERIFY_SIG, MERKLE_VERIFY)
    0xF0-0xFF: System (HALT, LOG, DEBUG)

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
import struct
from dataclasses import dataclass, field
from datetime import datetime, timezone
from decimal import Decimal
from enum import IntEnum, auto
from typing import (
    Any,
    Callable,
    Dict,
    List,
    Optional,
    Set,
    Tuple,
    Union,
)

from tools.phoenix.hardening import (
    ValidationError,
    InvariantViolation,
    SecurityViolation,
    Validators,
    CryptoUtils,
    ThreadSafeDict,
    AtomicCounter,
)


# =============================================================================
# OPCODES
# =============================================================================

class OpCode(IntEnum):
    """Smart Asset VM opcodes."""
    
    # Stack Operations (0x00-0x0F)
    STOP = 0x00
    PUSH1 = 0x01
    PUSH2 = 0x02
    PUSH4 = 0x03
    PUSH8 = 0x04
    PUSH32 = 0x05
    POP = 0x06
    DUP1 = 0x07
    DUP2 = 0x08
    SWAP1 = 0x09
    SWAP2 = 0x0A
    
    # Arithmetic (0x10-0x1F)
    ADD = 0x10
    SUB = 0x11
    MUL = 0x12
    DIV = 0x13
    MOD = 0x14
    EXP = 0x15
    NEG = 0x16
    ABS = 0x17
    
    # Comparison (0x20-0x2F)
    EQ = 0x20
    NE = 0x21
    LT = 0x22
    GT = 0x23
    LE = 0x24
    GE = 0x25
    AND = 0x26
    OR = 0x27
    NOT = 0x28
    XOR = 0x29
    
    # Memory (0x30-0x3F)
    MLOAD = 0x30
    MSTORE = 0x31
    MSTORE8 = 0x32
    MSIZE = 0x33
    MCOPY = 0x34
    
    # Storage (0x40-0x4F)
    SLOAD = 0x40
    SSTORE = 0x41
    SDELETE = 0x42
    SHAS = 0x43
    
    # Control Flow (0x50-0x5F)
    JUMP = 0x50
    JUMPI = 0x51
    JUMPDEST = 0x52
    CALL = 0x53
    RETURN = 0x54
    REVERT = 0x55
    ASSERT = 0x56
    
    # Context (0x60-0x6F)
    CALLER = 0x60
    ORIGIN = 0x61
    JURISDICTION = 0x62
    TIMESTAMP = 0x63
    BLOCK_HEIGHT = 0x64
    ASSET_ID = 0x65
    GAS = 0x66
    GASPRICE = 0x67
    
    # Compliance (0x70-0x7F)
    TENSOR_GET = 0x70
    TENSOR_SET = 0x71
    TENSOR_EVAL = 0x72
    TENSOR_COMMIT = 0x73
    ATTEST = 0x74
    VERIFY_ATTEST = 0x75
    VERIFY_ZK = 0x76
    COMPLIANCE_CHECK = 0x77
    
    # Migration (0x80-0x8F)
    LOCK = 0x80
    UNLOCK = 0x81
    TRANSIT_BEGIN = 0x82
    TRANSIT_END = 0x83
    SETTLE = 0x84
    COMPENSATE = 0x85
    MIGRATION_STATE = 0x86
    
    # Crypto (0x90-0x9F)
    SHA256 = 0x90
    KECCAK256 = 0x91
    VERIFY_SIG = 0x92
    MERKLE_ROOT = 0x93
    MERKLE_VERIFY = 0x94
    
    # System (0xF0-0xFF)
    HALT = 0xF0
    LOG0 = 0xF1
    LOG1 = 0xF2
    LOG2 = 0xF3
    DEBUG = 0xFE
    INVALID = 0xFF


# =============================================================================
# VM WORD TYPE
# =============================================================================

@dataclass(frozen=True)
class Word:
    """
    256-bit word - the fundamental unit of the SAVM.
    
    Represented internally as bytes for efficiency.
    """
    data: bytes
    
    def __post_init__(self):
        if len(self.data) != 32:
            raise ValueError(f"Word must be exactly 32 bytes, got {len(self.data)}")
    
    @classmethod
    def from_int(cls, value: int) -> 'Word':
        """Create word from integer (big-endian)."""
        # Handle negative numbers with two's complement
        if value < 0:
            value = (1 << 256) + value
        value = value % (1 << 256)
        return cls(value.to_bytes(32, 'big'))
    
    @classmethod
    def from_bytes(cls, data: bytes) -> 'Word':
        """Create word from bytes (right-padded)."""
        if len(data) > 32:
            data = data[:32]
        return cls(data.rjust(32, b'\x00'))
    
    @classmethod
    def from_hex(cls, hex_str: str) -> 'Word':
        """Create word from hex string."""
        if hex_str.startswith('0x'):
            hex_str = hex_str[2:]
        data = bytes.fromhex(hex_str.zfill(64))
        return cls(data)
    
    @classmethod
    def zero(cls) -> 'Word':
        """Create zero word."""
        return cls(b'\x00' * 32)
    
    @classmethod
    def one(cls) -> 'Word':
        """Create word with value 1."""
        return cls.from_int(1)
    
    def to_int(self, signed: bool = False) -> int:
        """Convert to integer."""
        value = int.from_bytes(self.data, 'big')
        if signed and value >= (1 << 255):
            value -= (1 << 256)
        return value
    
    def to_hex(self) -> str:
        """Convert to hex string."""
        return '0x' + self.data.hex()
    
    def __add__(self, other: 'Word') -> 'Word':
        return Word.from_int((self.to_int() + other.to_int()) % (1 << 256))
    
    def __sub__(self, other: 'Word') -> 'Word':
        return Word.from_int((self.to_int() - other.to_int()) % (1 << 256))
    
    def __mul__(self, other: 'Word') -> 'Word':
        return Word.from_int((self.to_int() * other.to_int()) % (1 << 256))
    
    def __truediv__(self, other: 'Word') -> 'Word':
        if other.to_int() == 0:
            return Word.zero()
        return Word.from_int(self.to_int() // other.to_int())
    
    def __mod__(self, other: 'Word') -> 'Word':
        if other.to_int() == 0:
            return Word.zero()
        return Word.from_int(self.to_int() % other.to_int())
    
    def __eq__(self, other: object) -> bool:
        if isinstance(other, Word):
            return self.data == other.data
        return False
    
    def __hash__(self) -> int:
        return hash(self.data)
    
    def __bool__(self) -> bool:
        return self.to_int() != 0


# =============================================================================
# EXECUTION CONTEXT
# =============================================================================

@dataclass
class ExecutionContext:
    """
    Context for VM execution.
    
    Contains information about the current execution environment.
    """
    # Caller information
    caller: str  # DID of caller
    origin: str  # DID of original transaction sender
    
    # Jurisdiction context
    jurisdiction_id: str
    corridor_id: Optional[str] = None
    
    # Asset context
    asset_id: str = ""
    asset_genesis_digest: str = ""
    
    # Time context
    timestamp: int = 0  # Unix timestamp
    block_height: int = 0
    
    # Gas context
    gas_limit: int = 1000000
    gas_price: int = 1
    
    # Value transfer
    value: int = 0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "caller": self.caller,
            "origin": self.origin,
            "jurisdiction_id": self.jurisdiction_id,
            "corridor_id": self.corridor_id,
            "asset_id": self.asset_id,
            "timestamp": self.timestamp,
            "block_height": self.block_height,
            "gas_limit": self.gas_limit,
        }


# =============================================================================
# VM STATE
# =============================================================================

@dataclass
class VMState:
    """
    Complete state of the SAVM.
    """
    # Program
    code: bytes
    pc: int = 0  # Program counter
    
    # Stack (max 256 words)
    stack: List[Word] = field(default_factory=list)
    
    # Memory (expandable, max 64KB)
    memory: bytearray = field(default_factory=bytearray)
    
    # Storage (persistent, Merkleized)
    storage: Dict[str, Word] = field(default_factory=dict)
    
    # Execution state
    gas_used: int = 0
    halted: bool = False
    reverted: bool = False
    return_data: bytes = b''
    
    # Logs
    logs: List[Dict[str, Any]] = field(default_factory=list)
    
    # Constants
    MAX_STACK_SIZE = 256
    MAX_MEMORY_SIZE = 65536  # 64KB
    
    def push(self, word: Word) -> None:
        """Push word onto stack."""
        if len(self.stack) >= self.MAX_STACK_SIZE:
            raise SecurityViolation("Stack overflow")
        self.stack.append(word)
    
    def pop(self) -> Word:
        """Pop word from stack."""
        if not self.stack:
            raise SecurityViolation("Stack underflow")
        return self.stack.pop()
    
    def peek(self, depth: int = 0) -> Word:
        """Peek at stack without popping."""
        if depth >= len(self.stack):
            raise SecurityViolation(f"Stack underflow at depth {depth}")
        return self.stack[-(depth + 1)]
    
    def dup(self, depth: int) -> None:
        """Duplicate stack item at depth."""
        word = self.peek(depth)
        self.push(word)
    
    def swap(self, depth: int) -> None:
        """Swap top of stack with item at depth."""
        if depth >= len(self.stack):
            raise SecurityViolation(f"Stack underflow for swap at depth {depth}")
        self.stack[-1], self.stack[-(depth + 1)] = \
            self.stack[-(depth + 1)], self.stack[-1]
    
    def mload(self, offset: int) -> Word:
        """Load 32 bytes from memory."""
        self._expand_memory(offset + 32)
        return Word(bytes(self.memory[offset:offset + 32]))
    
    def mstore(self, offset: int, word: Word) -> None:
        """Store 32 bytes to memory."""
        self._expand_memory(offset + 32)
        self.memory[offset:offset + 32] = word.data
    
    def mstore8(self, offset: int, value: int) -> None:
        """Store single byte to memory."""
        self._expand_memory(offset + 1)
        self.memory[offset] = value & 0xFF
    
    def _expand_memory(self, size: int) -> None:
        """Expand memory if needed."""
        if size > self.MAX_MEMORY_SIZE:
            raise SecurityViolation(f"Memory limit exceeded: {size} > {self.MAX_MEMORY_SIZE}")
        if len(self.memory) < size:
            self.memory.extend(b'\x00' * (size - len(self.memory)))
    
    def sload(self, key: str) -> Word:
        """Load from storage."""
        return self.storage.get(key, Word.zero())
    
    def sstore(self, key: str, value: Word) -> None:
        """Store to storage."""
        self.storage[key] = value
    
    def storage_root(self) -> str:
        """Compute Merkle root of storage."""
        if not self.storage:
            return "0" * 64
        
        leaves = []
        for key in sorted(self.storage.keys()):
            value = self.storage[key]
            leaf_data = key.encode() + value.data
            leaf = hashlib.sha256(leaf_data).hexdigest()
            leaves.append(leaf)
        
        return CryptoUtils.merkle_root(leaves)


# =============================================================================
# GAS COSTS
# =============================================================================

class GasCosts:
    """Gas costs for VM operations."""
    
    # Base costs
    ZERO = 0
    BASE = 2
    VERY_LOW = 3
    LOW = 5
    MID = 8
    HIGH = 10
    
    # Memory costs
    MEMORY_WORD = 3
    MEMORY_EXPANSION = 3
    
    # Storage costs
    SLOAD = 200
    SSTORE_SET = 20000
    SSTORE_RESET = 5000
    SSTORE_REFUND = 15000
    
    # Control flow
    JUMP = 8
    JUMPI = 10
    CALL = 700
    
    # Compliance operations (expensive)
    TENSOR_READ = 100
    TENSOR_WRITE = 500
    ZK_VERIFY = 10000
    ATTEST = 1000
    
    # Migration operations
    LOCK = 5000
    UNLOCK = 2000
    TRANSIT = 10000
    SETTLE = 20000
    
    # Crypto
    SHA256_BASE = 60
    SHA256_WORD = 12
    SIG_VERIFY = 3000
    
    @classmethod
    def for_opcode(cls, opcode: OpCode) -> int:
        """Get gas cost for opcode."""
        costs = {
            OpCode.STOP: cls.ZERO,
            OpCode.ADD: cls.VERY_LOW,
            OpCode.SUB: cls.VERY_LOW,
            OpCode.MUL: cls.LOW,
            OpCode.DIV: cls.LOW,
            OpCode.MOD: cls.LOW,
            OpCode.EQ: cls.VERY_LOW,
            OpCode.LT: cls.VERY_LOW,
            OpCode.GT: cls.VERY_LOW,
            OpCode.AND: cls.VERY_LOW,
            OpCode.OR: cls.VERY_LOW,
            OpCode.NOT: cls.VERY_LOW,
            OpCode.MLOAD: cls.VERY_LOW,
            OpCode.MSTORE: cls.VERY_LOW,
            OpCode.SLOAD: cls.SLOAD,
            OpCode.SSTORE: cls.SSTORE_SET,
            OpCode.JUMP: cls.JUMP,
            OpCode.JUMPI: cls.JUMPI,
            OpCode.TENSOR_GET: cls.TENSOR_READ,
            OpCode.TENSOR_SET: cls.TENSOR_WRITE,
            OpCode.VERIFY_ZK: cls.ZK_VERIFY,
            OpCode.LOCK: cls.LOCK,
            OpCode.UNLOCK: cls.UNLOCK,
            OpCode.TRANSIT_BEGIN: cls.TRANSIT,
            OpCode.SETTLE: cls.SETTLE,
            OpCode.SHA256: cls.SHA256_BASE,
            OpCode.VERIFY_SIG: cls.SIG_VERIFY,
        }
        return costs.get(opcode, cls.BASE)


# =============================================================================
# EXECUTION RESULT
# =============================================================================

@dataclass
class ExecutionResult:
    """Result of VM execution."""
    success: bool
    return_data: bytes
    gas_used: int
    logs: List[Dict[str, Any]]
    storage_root: str
    error: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "success": self.success,
            "return_data": self.return_data.hex(),
            "gas_used": self.gas_used,
            "logs": self.logs,
            "storage_root": self.storage_root,
            "error": self.error,
        }


# =============================================================================
# COMPLIANCE COPROCESSOR
# =============================================================================

class ComplianceCoprocessor:
    """
    Coprocessor for compliance operations.
    
    Handles tensor operations, ZK verification, and attestation checks.
    """
    
    def __init__(self):
        from tools.phoenix.tensor import ComplianceTensorV2, ComplianceDomain, ComplianceState
        self._tensor = ComplianceTensorV2()
        self._ComplianceDomain = ComplianceDomain
        self._ComplianceState = ComplianceState
    
    def tensor_get(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domain_code: int,
    ) -> Tuple[int, bool]:
        """
        Get compliance state from tensor.
        
        Returns (state_code, has_expired_attestations).
        """
        try:
            domain = self._ComplianceDomain(domain_code)
        except ValueError:
            return (0, False)  # UNKNOWN
        
        cell = self._tensor.get(asset_id, jurisdiction_id, domain)
        
        # Check for expired attestations using is_stale method
        has_expired = cell.is_stale() if cell.attestations else False
        
        return (cell.state.value, has_expired)
    
    def tensor_set(
        self,
        asset_id: str,
        jurisdiction_id: str,
        domain_code: int,
        state_code: int,
    ) -> bool:
        """Set compliance state in tensor."""
        try:
            domain = self._ComplianceDomain(domain_code)
            state = self._ComplianceState(state_code)
        except ValueError:
            return False
        
        self._tensor.set(asset_id, jurisdiction_id, domain, state)
        return True
    
    def tensor_evaluate(
        self,
        asset_id: str,
        jurisdiction_id: str,
    ) -> Tuple[bool, int, List[str]]:
        """
        Evaluate compliance for asset in jurisdiction.
        
        Returns (is_compliant, aggregate_state_code, issues).
        """
        is_compliant, state, issues = self._tensor.evaluate(
            asset_id, jurisdiction_id
        )
        return (is_compliant, state.value, issues)
    
    def tensor_commit(self) -> str:
        """Get tensor commitment root."""
        return self._tensor.commit().root
    
    def verify_zk_proof(
        self,
        circuit_id: str,
        public_inputs: List[bytes],
        proof_data: bytes,
    ) -> bool:
        """Verify a ZK proof."""
        from tools.phoenix.zkp import (
            create_standard_registry,
            MockVerifier,
            Proof,
            ProofSystem,
            FieldElement,
        )
        
        registry = create_standard_registry()
        circuit = registry.get_circuit(circuit_id)
        if not circuit:
            return False
        
        vk = registry.get_verification_key(circuit_id)
        if not vk:
            return False
        
        # Parse public inputs
        field_elements = [
            FieldElement(inp.hex()) for inp in public_inputs
        ]
        
        proof = Proof(
            circuit_id=circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=field_elements,
            proof_data=proof_data,
        )
        
        verifier = MockVerifier()
        return verifier.verify(circuit, vk, proof)


# =============================================================================
# MIGRATION COPROCESSOR
# =============================================================================

class MigrationCoprocessor:
    """
    Coprocessor for migration operations.
    
    Handles locks, unlocks, transit, and settlement.
    """
    
    def __init__(self):
        self._locks: Dict[str, Dict[str, Any]] = {}
        self._transits: Dict[str, Dict[str, Any]] = {}
    
    def lock(
        self,
        asset_id: str,
        jurisdiction_id: str,
        amount: int,
        lock_duration_seconds: int,
    ) -> Optional[str]:
        """
        Lock asset for migration.
        
        Returns lock_id if successful.
        """
        lock_id = CryptoUtils.secure_random_hex(16)
        
        now = datetime.now(timezone.utc)
        expiry = now.timestamp() + lock_duration_seconds
        
        self._locks[lock_id] = {
            "asset_id": asset_id,
            "jurisdiction_id": jurisdiction_id,
            "amount": amount,
            "locked_at": now.isoformat(),
            "expires_at": datetime.fromtimestamp(expiry, timezone.utc).isoformat(),
            "status": "active",
        }
        
        return lock_id
    
    def unlock(self, lock_id: str) -> bool:
        """Unlock a locked asset."""
        if lock_id not in self._locks:
            return False
        
        lock = self._locks[lock_id]
        if lock["status"] != "active":
            return False
        
        lock["status"] = "released"
        return True
    
    def transit_begin(
        self,
        lock_id: str,
        target_jurisdiction: str,
    ) -> Optional[str]:
        """Begin transit from locked asset."""
        if lock_id not in self._locks:
            return None
        
        lock = self._locks[lock_id]
        if lock["status"] != "active":
            return None
        
        transit_id = CryptoUtils.secure_random_hex(16)
        
        self._transits[transit_id] = {
            "lock_id": lock_id,
            "source_jurisdiction": lock["jurisdiction_id"],
            "target_jurisdiction": target_jurisdiction,
            "asset_id": lock["asset_id"],
            "amount": lock["amount"],
            "started_at": datetime.now(timezone.utc).isoformat(),
            "status": "in_transit",
        }
        
        lock["status"] = "in_transit"
        
        return transit_id
    
    def transit_end(self, transit_id: str) -> bool:
        """Complete transit."""
        if transit_id not in self._transits:
            return False
        
        transit = self._transits[transit_id]
        if transit["status"] != "in_transit":
            return False
        
        transit["status"] = "arrived"
        transit["completed_at"] = datetime.now(timezone.utc).isoformat()
        
        return True
    
    def settle(self, transit_id: str) -> bool:
        """Settle a completed transit."""
        if transit_id not in self._transits:
            return False
        
        transit = self._transits[transit_id]
        if transit["status"] != "arrived":
            return False
        
        # Mark settled
        transit["status"] = "settled"
        transit["settled_at"] = datetime.now(timezone.utc).isoformat()
        
        # Release lock
        lock_id = transit["lock_id"]
        if lock_id in self._locks:
            self._locks[lock_id]["status"] = "settled"
        
        return True
    
    def compensate(self, transit_id: str) -> bool:
        """Compensate a failed transit."""
        if transit_id not in self._transits:
            return False
        
        transit = self._transits[transit_id]
        if transit["status"] == "settled":
            return False
        
        transit["status"] = "compensated"
        
        # Release lock
        lock_id = transit["lock_id"]
        if lock_id in self._locks:
            self._locks[lock_id]["status"] = "compensated"
        
        return True


# =============================================================================
# SMART ASSET VM
# =============================================================================

class SmartAssetVM:
    """
    The Smart Asset Virtual Machine.
    
    Executes bytecode in a deterministic, gas-metered environment
    with compliance and migration coprocessors.
    
    Example:
        vm = SmartAssetVM()
        
        # Compile or use raw bytecode
        bytecode = bytes([
            OpCode.PUSH1, 0x42,  # Push 0x42
            OpCode.PUSH1, 0x00,  # Push 0x00 (storage key)
            OpCode.SSTORE,       # Store
            OpCode.HALT,
        ])
        
        context = ExecutionContext(
            caller="did:example:caller",
            origin="did:example:origin",
            jurisdiction_id="uae-difc",
        )
        
        result = vm.execute(bytecode, context)
    """
    
    def __init__(self):
        self._compliance = ComplianceCoprocessor()
        self._migration = MigrationCoprocessor()
        self._valid_jump_dests: Set[int] = set()
    
    def execute(
        self,
        code: bytes,
        context: ExecutionContext,
        initial_storage: Optional[Dict[str, Word]] = None,
    ) -> ExecutionResult:
        """
        Execute bytecode.
        
        Args:
            code: The bytecode to execute
            context: Execution context
            initial_storage: Optional initial storage state
            
        Returns:
            ExecutionResult with success/failure and data
        """
        # Initialize state
        state = VMState(
            code=code,
            storage=initial_storage or {},
        )
        
        # Pre-scan for valid jump destinations
        self._valid_jump_dests = self._scan_jump_dests(code)
        
        # Main execution loop
        try:
            while not state.halted and state.pc < len(code):
                # Fetch opcode
                opcode_byte = code[state.pc]
                try:
                    opcode = OpCode(opcode_byte)
                except ValueError:
                    opcode = OpCode.INVALID
                
                # Check gas
                gas_cost = GasCosts.for_opcode(opcode)
                if state.gas_used + gas_cost > context.gas_limit:
                    raise SecurityViolation("Out of gas")
                state.gas_used += gas_cost
                
                # Execute instruction
                self._execute_instruction(opcode, state, context)
                
                # Advance PC (unless jump)
                if opcode not in {OpCode.JUMP, OpCode.JUMPI, OpCode.CALL, OpCode.RETURN, OpCode.REVERT}:
                    state.pc += 1 + self._immediate_size(opcode)
            
            return ExecutionResult(
                success=not state.reverted,
                return_data=state.return_data,
                gas_used=state.gas_used,
                logs=state.logs,
                storage_root=state.storage_root(),
            )
            
        except Exception as e:
            return ExecutionResult(
                success=False,
                return_data=b'',
                gas_used=state.gas_used,
                logs=state.logs,
                storage_root=state.storage_root(),
                error=str(e),
            )
    
    def _scan_jump_dests(self, code: bytes) -> Set[int]:
        """Scan bytecode for valid jump destinations."""
        dests = set()
        pc = 0
        while pc < len(code):
            opcode_byte = code[pc]
            try:
                opcode = OpCode(opcode_byte)
            except ValueError:
                pc += 1
                continue
            
            if opcode == OpCode.JUMPDEST:
                dests.add(pc)
            
            pc += 1 + self._immediate_size(opcode)
        
        return dests
    
    def _immediate_size(self, opcode: OpCode) -> int:
        """Get size of immediate data for opcode."""
        sizes = {
            OpCode.PUSH1: 1,
            OpCode.PUSH2: 2,
            OpCode.PUSH4: 4,
            OpCode.PUSH8: 8,
            OpCode.PUSH32: 32,
        }
        return sizes.get(opcode, 0)
    
    def _read_immediate(self, state: VMState, size: int) -> bytes:
        """Read immediate data from code."""
        start = state.pc + 1
        end = start + size
        if end > len(state.code):
            raise SecurityViolation("Code overflow reading immediate")
        return state.code[start:end]
    
    def _execute_instruction(
        self,
        opcode: OpCode,
        state: VMState,
        context: ExecutionContext,
    ) -> None:
        """Execute a single instruction."""
        
        # Stack operations
        if opcode == OpCode.STOP:
            state.halted = True
            
        elif opcode == OpCode.PUSH1:
            data = self._read_immediate(state, 1)
            state.push(Word.from_bytes(data))
            
        elif opcode == OpCode.PUSH2:
            data = self._read_immediate(state, 2)
            state.push(Word.from_bytes(data))
            
        elif opcode == OpCode.PUSH4:
            data = self._read_immediate(state, 4)
            state.push(Word.from_bytes(data))
            
        elif opcode == OpCode.PUSH8:
            data = self._read_immediate(state, 8)
            state.push(Word.from_bytes(data))
            
        elif opcode == OpCode.PUSH32:
            data = self._read_immediate(state, 32)
            state.push(Word.from_bytes(data))
            
        elif opcode == OpCode.POP:
            state.pop()
            
        elif opcode == OpCode.DUP1:
            state.dup(0)
            
        elif opcode == OpCode.DUP2:
            state.dup(1)
            
        elif opcode == OpCode.SWAP1:
            state.swap(1)
            
        elif opcode == OpCode.SWAP2:
            state.swap(2)
        
        # Arithmetic
        elif opcode == OpCode.ADD:
            b, a = state.pop(), state.pop()
            state.push(a + b)
            
        elif opcode == OpCode.SUB:
            b, a = state.pop(), state.pop()
            state.push(a - b)
            
        elif opcode == OpCode.MUL:
            b, a = state.pop(), state.pop()
            state.push(a * b)
            
        elif opcode == OpCode.DIV:
            b, a = state.pop(), state.pop()
            state.push(a / b)
            
        elif opcode == OpCode.MOD:
            b, a = state.pop(), state.pop()
            state.push(a % b)
        
        # Comparison
        elif opcode == OpCode.EQ:
            b, a = state.pop(), state.pop()
            state.push(Word.one() if a == b else Word.zero())
            
        elif opcode == OpCode.LT:
            b, a = state.pop(), state.pop()
            state.push(Word.one() if a.to_int() < b.to_int() else Word.zero())
            
        elif opcode == OpCode.GT:
            b, a = state.pop(), state.pop()
            state.push(Word.one() if a.to_int() > b.to_int() else Word.zero())
            
        elif opcode == OpCode.AND:
            b, a = state.pop(), state.pop()
            result = bytes(x & y for x, y in zip(a.data, b.data))
            state.push(Word(result))
            
        elif opcode == OpCode.OR:
            b, a = state.pop(), state.pop()
            result = bytes(x | y for x, y in zip(a.data, b.data))
            state.push(Word(result))
            
        elif opcode == OpCode.NOT:
            a = state.pop()
            state.push(Word.one() if not a else Word.zero())
        
        # Memory
        elif opcode == OpCode.MLOAD:
            offset = state.pop().to_int()
            state.push(state.mload(offset))
            
        elif opcode == OpCode.MSTORE:
            offset = state.pop().to_int()
            value = state.pop()
            state.mstore(offset, value)
            
        elif opcode == OpCode.MSTORE8:
            offset = state.pop().to_int()
            value = state.pop().to_int() & 0xFF
            state.mstore8(offset, value)
            
        elif opcode == OpCode.MSIZE:
            state.push(Word.from_int(len(state.memory)))
        
        # Storage
        elif opcode == OpCode.SLOAD:
            key = state.pop().to_hex()
            state.push(state.sload(key))
            
        elif opcode == OpCode.SSTORE:
            key = state.pop().to_hex()
            value = state.pop()
            state.sstore(key, value)
        
        # Control flow
        elif opcode == OpCode.JUMP:
            dest = state.pop().to_int()
            if dest not in self._valid_jump_dests:
                raise SecurityViolation(f"Invalid jump destination: {dest}")
            state.pc = dest
            
        elif opcode == OpCode.JUMPI:
            dest = state.pop().to_int()
            condition = state.pop()
            if condition:
                if dest not in self._valid_jump_dests:
                    raise SecurityViolation(f"Invalid jump destination: {dest}")
                state.pc = dest
            else:
                state.pc += 1
                
        elif opcode == OpCode.JUMPDEST:
            pass  # No-op marker
            
        elif opcode == OpCode.RETURN:
            offset = state.pop().to_int()
            size = state.pop().to_int()
            state.return_data = bytes(state.memory[offset:offset + size])
            state.halted = True
            
        elif opcode == OpCode.REVERT:
            offset = state.pop().to_int()
            size = state.pop().to_int()
            state.return_data = bytes(state.memory[offset:offset + size])
            state.reverted = True
            state.halted = True
        
        # Context
        elif opcode == OpCode.CALLER:
            state.push(Word.from_bytes(context.caller.encode()[:32]))
            
        elif opcode == OpCode.ORIGIN:
            state.push(Word.from_bytes(context.origin.encode()[:32]))
            
        elif opcode == OpCode.JURISDICTION:
            state.push(Word.from_bytes(context.jurisdiction_id.encode()[:32]))
            
        elif opcode == OpCode.TIMESTAMP:
            state.push(Word.from_int(context.timestamp))
            
        elif opcode == OpCode.BLOCK_HEIGHT:
            state.push(Word.from_int(context.block_height))
            
        elif opcode == OpCode.ASSET_ID:
            state.push(Word.from_bytes(context.asset_id.encode()[:32]))
            
        elif opcode == OpCode.GAS:
            remaining = context.gas_limit - state.gas_used
            state.push(Word.from_int(remaining))
        
        # Compliance coprocessor
        elif opcode == OpCode.TENSOR_GET:
            domain_code = state.pop().to_int()
            jurisdiction = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            asset = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            
            state_code, has_expired = self._compliance.tensor_get(
                asset, jurisdiction, domain_code
            )
            state.push(Word.from_int(state_code))
            state.push(Word.from_int(1 if has_expired else 0))
            
        elif opcode == OpCode.TENSOR_SET:
            state_code = state.pop().to_int()
            domain_code = state.pop().to_int()
            jurisdiction = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            asset = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            
            success = self._compliance.tensor_set(
                asset, jurisdiction, domain_code, state_code
            )
            state.push(Word.from_int(1 if success else 0))
            
        elif opcode == OpCode.TENSOR_COMMIT:
            root = self._compliance.tensor_commit()
            state.push(Word.from_hex(root))
        
        # Migration coprocessor
        elif opcode == OpCode.LOCK:
            duration = state.pop().to_int()
            amount = state.pop().to_int()
            jurisdiction = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            asset = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            
            lock_id = self._migration.lock(asset, jurisdiction, amount, duration)
            if lock_id:
                state.push(Word.from_bytes(lock_id.encode()[:32]))
            else:
                state.push(Word.zero())
                
        elif opcode == OpCode.UNLOCK:
            lock_id = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            success = self._migration.unlock(lock_id)
            state.push(Word.from_int(1 if success else 0))
            
        elif opcode == OpCode.TRANSIT_BEGIN:
            target = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            lock_id = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            
            transit_id = self._migration.transit_begin(lock_id, target)
            if transit_id:
                state.push(Word.from_bytes(transit_id.encode()[:32]))
            else:
                state.push(Word.zero())
                
        elif opcode == OpCode.SETTLE:
            transit_id = state.pop().data.decode('utf-8', errors='ignore').rstrip('\x00')
            success = self._migration.settle(transit_id)
            state.push(Word.from_int(1 if success else 0))
        
        # Crypto
        elif opcode == OpCode.SHA256:
            size = state.pop().to_int()
            offset = state.pop().to_int()
            data = bytes(state.memory[offset:offset + size])
            digest = hashlib.sha256(data).digest()
            state.push(Word(digest))
        
        # System
        elif opcode == OpCode.HALT:
            state.halted = True
            
        elif opcode in {OpCode.LOG0, OpCode.LOG1, OpCode.LOG2}:
            size = state.pop().to_int()
            offset = state.pop().to_int()
            data = bytes(state.memory[offset:offset + size])
            
            topics = []
            topic_count = opcode.value - OpCode.LOG0.value
            for _ in range(topic_count):
                topics.append(state.pop().to_hex())
            
            state.logs.append({
                "data": data.hex(),
                "topics": topics,
            })
            
        elif opcode == OpCode.INVALID:
            raise SecurityViolation("Invalid opcode")
            
        else:
            raise SecurityViolation(f"Unimplemented opcode: {opcode}")


# =============================================================================
# BYTECODE ASSEMBLER
# =============================================================================

class Assembler:
    """Simple assembler for SAVM bytecode."""
    
    @staticmethod
    def assemble(instructions: List[Tuple[str, ...]]) -> bytes:
        """
        Assemble instructions to bytecode.
        
        Example:
            code = Assembler.assemble([
                ("PUSH1", 0x42),
                ("PUSH1", 0x00),
                ("SSTORE",),
                ("HALT",),
            ])
        """
        bytecode = bytearray()
        
        for instr in instructions:
            mnemonic = instr[0].upper()
            opcode = OpCode[mnemonic]
            bytecode.append(opcode.value)
            
            # Handle immediates
            if mnemonic.startswith("PUSH"):
                size = int(mnemonic[4:]) if mnemonic != "PUSH" else 1
                if len(instr) > 1:
                    value = instr[1]
                    if isinstance(value, int):
                        bytecode.extend(value.to_bytes(size, 'big'))
                    elif isinstance(value, bytes):
                        bytecode.extend(value[:size].rjust(size, b'\x00'))
        
        return bytes(bytecode)
    
    @staticmethod
    def disassemble(bytecode: bytes) -> List[Tuple[int, str, Any]]:
        """
        Disassemble bytecode to instructions.
        
        Returns list of (offset, mnemonic, immediate_value).
        """
        instructions = []
        pc = 0
        
        while pc < len(bytecode):
            offset = pc
            opcode_byte = bytecode[pc]
            
            try:
                opcode = OpCode(opcode_byte)
                mnemonic = opcode.name
            except ValueError:
                mnemonic = f"UNKNOWN(0x{opcode_byte:02x})"
                instructions.append((offset, mnemonic, None))
                pc += 1
                continue
            
            # Read immediate if present
            immediate = None
            imm_size = 0
            if mnemonic.startswith("PUSH"):
                imm_size = int(mnemonic[4:]) if mnemonic != "PUSH" else 1
                if pc + 1 + imm_size <= len(bytecode):
                    immediate = int.from_bytes(
                        bytecode[pc + 1:pc + 1 + imm_size], 'big'
                    )
            
            instructions.append((offset, mnemonic, immediate))
            pc += 1 + imm_size
        
        return instructions
