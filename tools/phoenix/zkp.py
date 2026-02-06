"""
PHOENIX Zero-Knowledge Proof Infrastructure

Privacy-preserving compliance verification without disclosure. This module provides
the cryptographic infrastructure for generating and verifying ZK proofs that
demonstrate compliance properties without revealing sensitive transaction details.

Supported Proof Systems:
    - Groth16: Succinct proofs, trusted setup required
    - PLONK: Universal trusted setup, larger proofs
    - STARK: No trusted setup, post-quantum secure, largest proofs

Circuit Types:
    - Balance Sufficiency: Prove balance >= threshold
    - Sanctions Clearance: Prove entity not on sanctions list
    - KYC Attestation: Prove valid KYC without revealing details
    - Tax Compliance: Prove tax obligations satisfied
    - Ownership Chain: Prove valid chain of title

The implementation follows a modular architecture where circuits are registered
in a content-addressed registry, and proofs reference their circuit by digest.

Copyright (c) 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

from __future__ import annotations

import hashlib
import json
import secrets
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any, Dict, List, Optional, Tuple, Generic, TypeVar, Protocol


# =============================================================================
# PROOF SYSTEMS
# =============================================================================

class ProofSystem(Enum):
    """
    Supported zero-knowledge proof systems.
    
    Selection criteria:
        - GROTH16: Smallest proofs (~200 bytes), fastest verification, trusted setup
        - PLONK: Universal setup, moderate proof size (~500 bytes)
        - STARK: No trusted setup, post-quantum, large proofs (~50KB+)
    """
    GROTH16 = "groth16"
    PLONK = "plonk"
    STARK = "stark"
    
    def requires_trusted_setup(self) -> bool:
        return self in {ProofSystem.GROTH16, ProofSystem.PLONK}
    
    def is_post_quantum(self) -> bool:
        return self == ProofSystem.STARK


class CircuitType(Enum):
    """
    Pre-defined circuit types for common compliance proofs.
    """
    BALANCE_SUFFICIENCY = "balance_sufficiency"
    SANCTIONS_CLEARANCE = "sanctions_clearance"
    KYC_ATTESTATION = "kyc_attestation"
    TAX_COMPLIANCE = "tax_compliance"
    OWNERSHIP_CHAIN = "ownership_chain"
    COMPLIANCE_TENSOR_INCLUSION = "compliance_tensor_inclusion"
    ATTESTATION_VALIDITY = "attestation_validity"
    THRESHOLD_SIGNATURE = "threshold_signature"
    MERKLE_MEMBERSHIP = "merkle_membership"
    RANGE_PROOF = "range_proof"


# =============================================================================
# CRYPTOGRAPHIC PRIMITIVES
# =============================================================================

@dataclass(frozen=True)
class FieldElement:
    """
    Element of the scalar field for the proof system.

    For BN254/BLS12-381: 254-bit field elements
    Represented as hex string for serialization.

    All arithmetic operations are performed modulo the BN254 scalar field prime
    to ensure values remain valid field elements.
    """
    value: str  # Hex representation

    # BN254 scalar field order (also known as Fr)
    FIELD_MODULUS: int = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001

    def __post_init__(self):
        if not all(c in '0123456789abcdef' for c in self.value.lower()):
            raise ValueError("Field element must be hex string")

    @classmethod
    def zero(cls) -> 'FieldElement':
        return cls("0" * 64)

    @classmethod
    def one(cls) -> 'FieldElement':
        return cls("0" * 63 + "1")

    @classmethod
    def random(cls) -> 'FieldElement':
        val = int.from_bytes(secrets.token_bytes(32), 'big') % cls.FIELD_MODULUS
        return cls(format(val, '064x'))

    @classmethod
    def from_int(cls, n: int) -> 'FieldElement':
        """Create a field element from an integer, reducing modulo FIELD_MODULUS."""
        val = n % cls.FIELD_MODULUS
        return cls(format(val, '064x'))

    def to_int(self) -> int:
        """Convert to integer."""
        return int(self.value, 16)

    def to_bytes(self) -> bytes:
        return bytes.fromhex(self.value)

    def __add__(self, other: 'FieldElement') -> 'FieldElement':
        result = (self.to_int() + other.to_int()) % self.FIELD_MODULUS
        return FieldElement(format(result, '064x'))

    def __sub__(self, other: 'FieldElement') -> 'FieldElement':
        result = (self.to_int() - other.to_int()) % self.FIELD_MODULUS
        return FieldElement(format(result, '064x'))

    def __mul__(self, other: 'FieldElement') -> 'FieldElement':
        result = (self.to_int() * other.to_int()) % self.FIELD_MODULUS
        return FieldElement(format(result, '064x'))

    def __neg__(self) -> 'FieldElement':
        result = (self.FIELD_MODULUS - self.to_int()) % self.FIELD_MODULUS
        return FieldElement(format(result, '064x'))

    def inverse(self) -> 'FieldElement':
        """Compute modular multiplicative inverse using Fermat's little theorem."""
        val = self.to_int()
        if val == 0:
            raise ZeroDivisionError("Cannot invert zero field element")
        result = pow(val, self.FIELD_MODULUS - 2, self.FIELD_MODULUS)
        return FieldElement(format(result, '064x'))

    def __truediv__(self, other: 'FieldElement') -> 'FieldElement':
        return self * other.inverse()

    def __eq__(self, other: object) -> bool:
        if isinstance(other, FieldElement):
            return self.to_int() % self.FIELD_MODULUS == other.to_int() % self.FIELD_MODULUS
        return False

    def __hash__(self) -> int:
        return hash(self.to_int() % self.FIELD_MODULUS)


@dataclass(frozen=True)
class G1Point:
    """Point on the G1 curve (for pairing-based systems)."""
    x: FieldElement
    y: FieldElement
    
    @classmethod
    def generator(cls) -> 'G1Point':
        # Placeholder - actual generator depends on curve
        return cls(FieldElement.one(), FieldElement.one())


@dataclass(frozen=True)
class G2Point:
    """Point on the G2 curve (for pairing-based systems)."""
    x: Tuple[FieldElement, FieldElement]
    y: Tuple[FieldElement, FieldElement]


# =============================================================================
# KEYS
# =============================================================================

@dataclass
class ProvingKey:
    """
    Proving key for a specific circuit.
    
    Contains the structured reference string (SRS) elements needed
    to generate proofs for the circuit.
    """
    circuit_id: str
    proof_system: ProofSystem
    constraint_count: int
    public_input_count: int
    key_data: bytes  # Serialized proving key
    digest: str = ""  # SHA256 of key_data
    
    def __post_init__(self):
        if not self.digest:
            self.digest = hashlib.sha256(self.key_data).hexdigest()

    def to_dict(self) -> Dict[str, Any]:
        return {
            "circuit_id": self.circuit_id,
            "proof_system": self.proof_system.value,
            "constraint_count": self.constraint_count,
            "public_input_count": self.public_input_count,
            "key_digest": self.digest,
        }


@dataclass
class VerificationKey:
    """
    Verification key for a specific circuit.

    Contains the elements needed to verify proofs for the circuit.
    Much smaller than the proving key.
    """
    circuit_id: str
    proof_system: ProofSystem
    public_input_count: int
    key_data: bytes  # Serialized verification key
    digest: str = ""

    def __post_init__(self):
        if not self.digest:
            self.digest = hashlib.sha256(self.key_data).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "circuit_id": self.circuit_id,
            "proof_system": self.proof_system.value,
            "public_input_count": self.public_input_count,
            "key_digest": self.digest,
        }


# =============================================================================
# WITNESS AND PROOF
# =============================================================================

@dataclass
class Witness:
    """
    Private witness data for proof generation.
    
    The witness contains all private inputs needed to satisfy
    the circuit constraints. This data is never revealed.
    """
    circuit_id: str
    private_inputs: Dict[str, Any]
    public_inputs: Dict[str, Any]
    
    def to_field_elements(self) -> List[FieldElement]:
        """Convert witness to field elements for the prover."""
        elements: List[FieldElement] = []
        
        # Public inputs first
        for key in sorted(self.public_inputs.keys()):
            value = self.public_inputs[key]
            elements.append(self._to_field_element(value))
        
        # Then private inputs
        for key in sorted(self.private_inputs.keys()):
            value = self.private_inputs[key]
            elements.append(self._to_field_element(value))
        
        return elements
    
    def _to_field_element(self, value: Any) -> FieldElement:
        """Convert a value to a field element."""
        from decimal import Decimal as DecimalType

        if isinstance(value, bool):
            # Check bool before int since bool is subclass of int
            return FieldElement.one() if value else FieldElement.zero()
        elif isinstance(value, int):
            # Handle negative integers with two's complement in 256-bit field
            if value < 0:
                # Two's complement: convert to positive representation in field
                value = (1 << 256) + value
            # Ensure value fits in 256 bits
            value = value % (1 << 256)
            # Format as 64-character hex (256 bits = 32 bytes = 64 hex chars)
            hex_str = format(value, '064x')
            return FieldElement(hex_str)
        elif isinstance(value, DecimalType):
            # Convert Decimal to integer (scale by 10^18 for precision)
            # This is a common pattern for fixed-point arithmetic
            scaled = int(value * DecimalType("1000000000000000000"))  # 10^18
            if scaled < 0:
                scaled = (1 << 256) + scaled
            scaled = scaled % (1 << 256)
            hex_str = format(scaled, '064x')
            return FieldElement(hex_str)
        elif isinstance(value, float):
            # Convert float to Decimal then to field element
            return self._to_field_element(DecimalType(str(value)))
        elif isinstance(value, str):
            # Hash string to field element
            h = hashlib.sha256(value.encode()).hexdigest()
            return FieldElement(h)
        elif isinstance(value, bytes):
            h = hashlib.sha256(value).hexdigest()
            return FieldElement(h)
        else:
            raise ValueError(f"Cannot convert {type(value)} to field element")


@dataclass
class Proof:
    """
    A zero-knowledge proof.
    
    The proof demonstrates that the prover knows a valid witness
    satisfying the circuit constraints, without revealing the witness.
    """
    circuit_id: str
    proof_system: ProofSystem
    public_inputs: List[FieldElement]
    proof_data: bytes
    generated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    @property
    def digest(self) -> str:
        """Content-addressed identifier for the proof."""
        content = {
            "circuit_id": self.circuit_id,
            "proof_system": self.proof_system.value,
            "public_inputs": [p.value for p in self.public_inputs],
            "proof_data": self.proof_data.hex(),
        }
        canonical = json.dumps(content, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "circuit_id": self.circuit_id,
            "proof_system": self.proof_system.value,
            "public_inputs": [p.value for p in self.public_inputs],
            "proof_data": self.proof_data.hex(),
            "generated_at": self.generated_at,
            "digest": self.digest,
        }
    
    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> 'Proof':
        return cls(
            circuit_id=data["circuit_id"],
            proof_system=ProofSystem(data["proof_system"]),
            public_inputs=[FieldElement(p) for p in data["public_inputs"]],
            proof_data=bytes.fromhex(data["proof_data"]),
            generated_at=data.get("generated_at", ""),
        )


# =============================================================================
# CIRCUIT DEFINITION
# =============================================================================

@dataclass
class CircuitConstraint:
    """
    A single R1CS constraint: a * b = c
    
    Where a, b, c are linear combinations of variables.
    """
    a_coefficients: Dict[int, FieldElement]  # variable_index -> coefficient
    b_coefficients: Dict[int, FieldElement]
    c_coefficients: Dict[int, FieldElement]


@dataclass
class Circuit:
    """
    A zero-knowledge circuit definition.
    
    Circuits are specified as R1CS (Rank-1 Constraint System) for
    SNARK-based systems, or as AIR (Algebraic Intermediate Representation)
    for STARKs.
    """
    circuit_id: str
    circuit_type: CircuitType
    proof_system: ProofSystem
    
    # Circuit structure
    public_input_names: List[str]
    private_input_names: List[str]
    constraint_count: int
    
    # Constraints (for R1CS)
    constraints: List[CircuitConstraint] = field(default_factory=list)
    
    # Metadata
    description: str = ""
    version: str = "1.0.0"
    audits: List[Dict[str, str]] = field(default_factory=list)
    
    @property
    def digest(self) -> str:
        """Content-addressed identifier for the circuit."""
        content = {
            "circuit_id": self.circuit_id,
            "circuit_type": self.circuit_type.value,
            "proof_system": self.proof_system.value,
            "public_input_names": self.public_input_names,
            "private_input_names": self.private_input_names,
            "constraint_count": self.constraint_count,
            "version": self.version,
        }
        canonical = json.dumps(content, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "circuit_id": self.circuit_id,
            "circuit_type": self.circuit_type.value,
            "proof_system": self.proof_system.value,
            "public_input_names": self.public_input_names,
            "private_input_names": self.private_input_names,
            "constraint_count": self.constraint_count,
            "description": self.description,
            "version": self.version,
            "audits": self.audits,
            "digest": self.digest,
        }


# =============================================================================
# CIRCUIT REGISTRY
# =============================================================================

class CircuitRegistry:
    """
    Content-addressed registry of ZK circuits.
    
    Circuits are identified by their digest, enabling:
    - Deterministic references across systems
    - Version control and upgrade paths
    - Audit trail for circuit changes
    """
    
    def __init__(self):
        self._circuits: Dict[str, Circuit] = {}  # digest -> Circuit
        self._circuit_id_to_digest: Dict[str, str] = {}  # circuit_id -> digest
        self._proving_keys: Dict[str, ProvingKey] = {}
        self._verification_keys: Dict[str, VerificationKey] = {}
    
    def register(
        self,
        circuit: Circuit,
        proving_key: Optional[ProvingKey] = None,
        verification_key: Optional[VerificationKey] = None,
    ) -> str:
        """
        Register a circuit and optionally its keys.
        
        Returns the circuit digest.
        """
        digest = circuit.digest
        self._circuits[digest] = circuit
        self._circuit_id_to_digest[circuit.circuit_id] = digest

        if proving_key:
            self._proving_keys[digest] = proving_key
        if verification_key:
            self._verification_keys[digest] = verification_key

        return digest
    
    def get_circuit(self, digest: str) -> Optional[Circuit]:
        """Retrieve a circuit by digest."""
        return self._circuits.get(digest)

    def get_circuit_by_id(self, circuit_id: str) -> Optional[Circuit]:
        """Retrieve a circuit by its circuit_id (human-readable name)."""
        digest = self._circuit_id_to_digest.get(circuit_id)
        if digest:
            return self._circuits.get(digest)
        return None

    def get_digest_by_circuit_id(self, circuit_id: str) -> Optional[str]:
        """Get the digest for a circuit by its circuit_id."""
        return self._circuit_id_to_digest.get(circuit_id)
    
    def get_proving_key(self, circuit_digest: str) -> Optional[ProvingKey]:
        """Retrieve proving key for a circuit."""
        return self._proving_keys.get(circuit_digest)
    
    def get_verification_key(self, circuit_digest: str) -> Optional[VerificationKey]:
        """Retrieve verification key for a circuit."""
        return self._verification_keys.get(circuit_digest)
    
    def list_circuits(
        self,
        circuit_type: Optional[CircuitType] = None,
        proof_system: Optional[ProofSystem] = None,
    ) -> List[Circuit]:
        """List circuits, optionally filtered."""
        circuits = list(self._circuits.values())
        
        if circuit_type:
            circuits = [c for c in circuits if c.circuit_type == circuit_type]
        if proof_system:
            circuits = [c for c in circuits if c.proof_system == proof_system]
        
        return circuits
    
    def export_registry(self) -> Dict[str, Any]:
        """Export registry to serializable format."""
        return {
            "circuits": {
                digest: circuit.to_dict()
                for digest, circuit in self._circuits.items()
            },
            "verification_keys": {
                digest: key.to_dict()
                for digest, key in self._verification_keys.items()
            },
        }


# =============================================================================
# PROVER AND VERIFIER INTERFACES
# =============================================================================

class Prover(Protocol):
    """Protocol for ZK proof generation."""
    
    def prove(
        self,
        circuit: Circuit,
        proving_key: ProvingKey,
        witness: Witness,
    ) -> Proof:
        """Generate a proof for the given witness."""
        ...


class Verifier(Protocol):
    """Protocol for ZK proof verification."""
    
    def verify(
        self,
        circuit: Circuit,
        verification_key: VerificationKey,
        proof: Proof,
    ) -> bool:
        """Verify a proof against the circuit and public inputs."""
        ...


# =============================================================================
# MOCK IMPLEMENTATIONS (for testing without actual ZK backend)
# =============================================================================

class MockProver:
    """
    Mock prover for testing.
    
    Generates deterministic mock proofs based on witness hash.
    NOT CRYPTOGRAPHICALLY SECURE - for testing only.
    """
    
    def prove(
        self,
        circuit: Circuit,
        proving_key: ProvingKey,
        witness: Witness,
    ) -> Proof:
        # Create deterministic mock proof from witness
        witness_hash = hashlib.sha256(
            json.dumps(witness.private_inputs, sort_keys=True).encode()
        ).digest()
        
        public_inputs = []
        for name in circuit.public_input_names:
            if name not in witness.public_inputs:
                raise ValueError(
                    f"Missing required public input '{name}' in witness. "
                    f"Circuit requires: {circuit.public_input_names}, "
                    f"witness provides: {list(witness.public_inputs.keys())}"
                )
            public_inputs.append(witness._to_field_element(witness.public_inputs[name]))
        
        # Mock proof is hash of witness || circuit_id
        mock_proof_data = hashlib.sha256(
            witness_hash + circuit.circuit_id.encode()
        ).digest()
        
        return Proof(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_inputs=public_inputs,
            proof_data=mock_proof_data,
        )


class MockVerifier:
    """
    Mock verifier for testing.
    
    Always returns True for well-formed proofs.
    NOT CRYPTOGRAPHICALLY SECURE - for testing only.
    """
    
    # Minimum proof sizes by proof system (in bytes)
    MIN_PROOF_SIZES = {
        ProofSystem.GROTH16: 32,
        ProofSystem.PLONK: 32,
        ProofSystem.STARK: 32,
    }

    def verify(
        self,
        circuit: Circuit,
        verification_key: VerificationKey,
        proof: Proof,
    ) -> bool:
        # Basic structure checks
        if proof.circuit_id != circuit.circuit_id:
            return False
        if proof.proof_system != circuit.proof_system:
            return False
        if len(proof.public_inputs) != len(circuit.public_input_names):
            return False
        if len(proof.proof_data) == 0:
            return False

        # Verify proof_data meets minimum size for the proof system
        min_size = self.MIN_PROOF_SIZES.get(proof.proof_system, 32)
        if len(proof.proof_data) < min_size:
            return False

        # Verify all public inputs are well-formed (non-empty field elements)
        for pi in proof.public_inputs:
            if not isinstance(pi, FieldElement):
                return False
            if not pi.value or len(pi.value) == 0:
                return False

        # Verify verification key matches the circuit
        if verification_key.circuit_id != circuit.circuit_id:
            return False
        if verification_key.proof_system != circuit.proof_system:
            return False

        return True


# =============================================================================
# COMPLIANCE PROOF BUILDERS
# =============================================================================

def build_balance_sufficiency_circuit(
    threshold_public: bool = True,
) -> Circuit:
    """
    Build a balance sufficiency circuit.
    
    Proves: balance >= threshold
    
    Public inputs: threshold (optional), result_hash
    Private inputs: balance
    """
    public_inputs = ["result_commitment"]
    if threshold_public:
        public_inputs.insert(0, "threshold")
    
    return Circuit(
        circuit_id="zk.balance_sufficiency.v1",
        circuit_type=CircuitType.BALANCE_SUFFICIENCY,
        proof_system=ProofSystem.GROTH16,
        public_input_names=public_inputs,
        private_input_names=["balance"],
        constraint_count=256,  # Approximate for range check
        description="Proves balance >= threshold without revealing balance",
        version="1.0.0",
    )


def build_sanctions_clearance_circuit() -> Circuit:
    """
    Build a sanctions clearance circuit.
    
    Proves: entity_hash NOT IN sanctions_merkle_tree
    
    Uses Merkle non-membership proof.
    """
    return Circuit(
        circuit_id="zk.sanctions_clearance.v1",
        circuit_type=CircuitType.SANCTIONS_CLEARANCE,
        proof_system=ProofSystem.GROTH16,
        public_input_names=["sanctions_root", "verification_timestamp"],
        private_input_names=["entity_hash", "merkle_proof", "merkle_path"],
        constraint_count=2048,
        description="Proves entity not on sanctions list via Merkle non-membership",
        version="1.0.0",
    )


def build_kyc_attestation_circuit() -> Circuit:
    """
    Build a KYC attestation circuit.
    
    Proves: Valid KYC attestation exists from approved issuer
    """
    return Circuit(
        circuit_id="zk.kyc_attestation.v1",
        circuit_type=CircuitType.KYC_ATTESTATION,
        proof_system=ProofSystem.GROTH16,
        public_input_names=[
            "approved_issuers_root",
            "min_kyc_level",
            "verification_timestamp",
        ],
        private_input_names=[
            "attestation_hash",
            "issuer_signature",
            "issuer_pubkey",
            "kyc_level",
            "issuer_merkle_proof",
        ],
        constraint_count=4096,
        description="Proves valid KYC attestation without revealing details",
        version="1.0.0",
    )


def build_compliance_tensor_inclusion_circuit() -> Circuit:
    """
    Build a compliance tensor inclusion circuit.
    
    Proves: Specific coordinate in tensor has claimed state
    """
    return Circuit(
        circuit_id="zk.compliance_tensor_inclusion.v1",
        circuit_type=CircuitType.COMPLIANCE_TENSOR_INCLUSION,
        proof_system=ProofSystem.GROTH16,
        public_input_names=[
            "tensor_commitment",
            "claimed_state",
        ],
        private_input_names=[
            "asset_id",
            "jurisdiction_id",
            "domain",
            "time_quantum",
            "merkle_proof",
        ],
        constraint_count=2048,
        description="Proves compliance state at specific tensor coordinate",
        version="1.0.0",
    )


# =============================================================================
# PROOF AGGREGATION
# =============================================================================

@dataclass
class AggregatedProof:
    """
    Aggregated proof combining multiple individual proofs.
    
    Enables batch verification with sublinear verification cost.
    """
    individual_proofs: List[Proof]
    aggregation_proof: bytes
    
    @property
    def circuit_ids(self) -> List[str]:
        return [p.circuit_id for p in self.individual_proofs]
    
    @property
    def digest(self) -> str:
        content = {
            "individual_digests": [p.digest for p in self.individual_proofs],
            "aggregation_proof": self.aggregation_proof.hex(),
        }
        canonical = json.dumps(content, sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(canonical.encode()).hexdigest()


class ProofAggregator:
    """
    Aggregates multiple proofs into a single succinct proof.
    
    Uses recursive proof composition for SNARK-based systems.
    """
    
    def aggregate(self, proofs: List[Proof]) -> AggregatedProof:
        """
        Aggregate multiple proofs.
        
        The aggregated proof proves: "I know valid proofs for all these statements"
        """
        if not proofs:
            raise ValueError("Cannot aggregate empty proof list")
        
        # Mock aggregation - real implementation would use recursive SNARKs
        combined = b"".join(p.proof_data for p in proofs)
        aggregation_proof = hashlib.sha256(combined).digest()
        
        return AggregatedProof(
            individual_proofs=proofs,
            aggregation_proof=aggregation_proof,
        )
    
    def verify_aggregated(
        self,
        aggregated: AggregatedProof,
        registry: CircuitRegistry,
    ) -> bool:
        """
        Verify an aggregated proof.

        Returns True iff all individual proofs are valid.
        """
        verifier = MockVerifier()

        for proof in aggregated.individual_proofs:
            # Look up circuit by circuit_id (human-readable name), not digest
            circuit = registry.get_circuit_by_id(proof.circuit_id)
            if not circuit:
                return False

            # Get the circuit digest to look up verification key
            digest = registry.get_digest_by_circuit_id(proof.circuit_id)
            if not digest:
                return False

            vk = registry.get_verification_key(digest)
            if not vk:
                return False

            if not verifier.verify(circuit, vk, proof):
                return False

        return True


# =============================================================================
# STANDARD CIRCUIT REGISTRY
# =============================================================================

def create_standard_registry() -> CircuitRegistry:
    """
    Create a registry with standard compliance circuits.
    """
    registry = CircuitRegistry()
    
    # Register standard circuits
    circuits = [
        build_balance_sufficiency_circuit(),
        build_sanctions_clearance_circuit(),
        build_kyc_attestation_circuit(),
        build_compliance_tensor_inclusion_circuit(),
    ]
    
    for circuit in circuits:
        # Create mock keys for testing
        pk = ProvingKey(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            constraint_count=circuit.constraint_count,
            public_input_count=len(circuit.public_input_names),
            key_data=hashlib.sha256(circuit.circuit_id.encode()).digest(),
        )
        
        vk = VerificationKey(
            circuit_id=circuit.circuit_id,
            proof_system=circuit.proof_system,
            public_input_count=len(circuit.public_input_names),
            key_data=hashlib.sha256(f"{circuit.circuit_id}_vk".encode()).digest(),
        )
        
        registry.register(circuit, pk, vk)
    
    return registry
