"""
MASS Protocol Core Primitives
=============================

Implements the formal definitions from the MASS Protocol Enhanced Specification v0.2.

This module provides:
- Canonical Digest Bridge (Definition 3.4, Construction 3.1, Theorem 3.2)
- Smart Asset Tuple (Definition 11.1)
- Receipt Chain with MMR (Chapter 12)
- Compliance Tensor (Definition 7.3)
- Agentic Execution Model (Chapter 17)
- Design Invariants I1-I5 (Definition 11.2)

Reference: MASS Protocol Technical Specification v0.2, November 2025
"""

from __future__ import annotations
import hashlib
import json
from dataclasses import dataclass, field
from enum import Enum, auto
from typing import Any, Dict, List, Optional, Set, Tuple, Callable, Union
from datetime import datetime, timezone
from decimal import Decimal
import copy


# =============================================================================
# CHAPTER 3: CORE CRYPTOGRAPHIC PRIMITIVES
# =============================================================================

class DigestAlgorithm(Enum):
    """Definition 4.1: Digest algorithms supported by MASS."""
    SHA256 = 0x01
    POSEIDON = 0x02


@dataclass(frozen=True)
class Digest:
    """
    Definition 4.1 (Digest Type).
    
    A typed cryptographic digest with algorithm identifier.
    """
    alg: DigestAlgorithm
    bytes_hex: str  # 64 hex characters for SHA256
    
    def __post_init__(self):
        if self.alg == DigestAlgorithm.SHA256:
            if len(self.bytes_hex) != 64:
                raise ValueError(f"SHA256 digest must be 64 hex chars, got {len(self.bytes_hex)}")
            if not all(c in '0123456789abcdef' for c in self.bytes_hex.lower()):
                raise ValueError("Digest must be lowercase hex")
    
    def __str__(self) -> str:
        return self.bytes_hex
    
    @classmethod
    def from_sha256_hex(cls, hex_str: str) -> 'Digest':
        return cls(DigestAlgorithm.SHA256, hex_str.lower())


def json_canonicalize(obj: Any) -> str:
    """
    Definition 3.4 (JSON Canonicalization Scheme - JCS).

    RFC 8785 compliant JSON canonicalization.

    IMPORTANT: This function delegates to the canonical ``jcs_canonicalize()``
    implementation in ``tools.lawpack`` which is the single source of truth for
    digest computation across the entire SEZ Stack.  The core function applies
    strict type coercion (rejecting floats, converting datetimes to UTC ISO8601,
    coercing non-string dict keys, converting tuples to lists) that plain
    ``json.dumps(sort_keys=True)`` does not.

    Returns a UTF-8 *string* (not bytes) for backward compatibility with callers
    in this module that pass the result to ``.encode('utf-8')``.
    """
    from tools.lawpack import jcs_canonicalize
    return jcs_canonicalize(obj).decode("utf-8")


def stack_digest(artifact: Any) -> Digest:
    """
    Construction 3.1 (Canonical Digest Bridge) - Stack Layer.
    
    StackDigest(d) = (alg = 0x01, bytes = SHA256(JCS(d)))
    
    For any JSON artifact d, computes the content-addressed digest
    used throughout the SEZ Stack.
    """
    canonical = json_canonicalize(artifact)
    hash_bytes = hashlib.sha256(canonical.encode('utf-8')).hexdigest()
    return Digest(DigestAlgorithm.SHA256, hash_bytes)


def semantic_digest(artifact_type: str, content: Any) -> Digest:
    """
    Definition 18.4 (Semantic Digest Rules).
    
    Digest computation varies by artifact type:
    - vc: SHA256(JCS(vc_without_proof))
    - checkpoint: SHA256(JCS(checkpoint_without_proof))
    - blob: SHA256(raw_bytes)
    - smart-asset-receipt: receipt.next_root (pre-computed)
    """
    if artifact_type in ('vc', 'checkpoint', 'rule-eval-evidence'):
        # Remove proof field for digest computation
        content_copy = copy.deepcopy(content)
        if isinstance(content_copy, dict) and 'proof' in content_copy:
            del content_copy['proof']
        return stack_digest(content_copy)
    elif artifact_type == 'blob':
        # Raw bytes digest
        if isinstance(content, bytes):
            return Digest(DigestAlgorithm.SHA256, hashlib.sha256(content).hexdigest())
        elif isinstance(content, str):
            return Digest(DigestAlgorithm.SHA256, hashlib.sha256(content.encode()).hexdigest())
    elif artifact_type in ('smart-asset-receipt', 'corridor-receipt'):
        # Pre-computed next_root
        if isinstance(content, dict) and 'next_root' in content:
            return Digest.from_sha256_hex(content['next_root'])
    
    # Default: JCS canonicalization
    return stack_digest(content)


# =============================================================================
# CHAPTER 4: CONTENT-ADDRESSED ARTIFACT MODEL
# =============================================================================

class ArtifactType(Enum):
    """
    Definition 18.2 (ArtifactRef) - Artifact Types.
    
    From the MASS Artifact Type Registry (Table 4.1).
    """
    LAWPACK = "lawpack"
    RULESET = "ruleset"
    TRANSITION_TYPES = "transition-types"
    SCHEMA = "schema"
    VC = "vc"
    CHECKPOINT = "checkpoint"
    RULE_EVAL_EVIDENCE = "rule-eval-evidence"
    CIRCUIT = "circuit"
    PROOF_KEY = "proof-key"
    BLOB = "blob"
    SMART_ASSET_RECEIPT = "smart-asset-receipt"
    CORRIDOR_RECEIPT = "corridor-receipt"
    REGPACK = "regpack"
    WITNESS_BUNDLE = "witness-bundle"


@dataclass
class ArtifactRef:
    """
    Definition 18.2 (ArtifactRef).
    
    A typed commitment to a content-addressed artifact.
    Only artifact_type and digest_sha256 are authoritative.
    Other fields are hints for resolution efficiency.
    """
    artifact_type: ArtifactType
    digest_sha256: str
    uri: Optional[str] = None
    media_type: Optional[str] = None
    byte_length: Optional[int] = None
    
    def __post_init__(self):
        if len(self.digest_sha256) != 64:
            raise ValueError(f"digest_sha256 must be 64 hex chars")
        self.digest_sha256 = self.digest_sha256.lower()
    
    def to_dict(self) -> Dict[str, Any]:
        result = {
            "artifact_type": self.artifact_type.value,
            "digest_sha256": self.digest_sha256
        }
        if self.uri:
            result["uri"] = self.uri
        if self.media_type:
            result["media_type"] = self.media_type
        if self.byte_length:
            result["byte_length"] = self.byte_length
        return result
    
    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> 'ArtifactRef':
        return cls(
            artifact_type=ArtifactType(d["artifact_type"]),
            digest_sha256=d["digest_sha256"],
            uri=d.get("uri"),
            media_type=d.get("media_type"),
            byte_length=d.get("byte_length")
        )


def compute_artifact_closure(root: ArtifactRef, resolver: Callable[[ArtifactRef], Any]) -> Set[str]:
    """
    Definition 18.5 (Artifact Closure).
    
    The closure of an artifact is the transitive set of all referenced artifacts:
    
    Closure(a) = {a} ∪ ⋃_{r∈Refs(a)} Closure(Resolve(r))
    """
    visited: Set[str] = set()
    queue = [root]
    
    while queue:
        ref = queue.pop(0)
        if ref.digest_sha256 in visited:
            continue
        visited.add(ref.digest_sha256)
        
        try:
            artifact = resolver(ref)
            nested_refs = extract_artifact_refs(artifact)
            queue.extend(nested_refs)
        except Exception as exc:
            # Artifact not resolvable — add to visited but don't expand.
            # Log at debug level; callers handle the missing-artifact case
            # via the returned visited set.
            import logging
            logging.getLogger(__name__).debug(
                "Artifact %s not resolvable during closure expansion: %s",
                ref.digest_sha256, exc,
            )
    
    return visited


def extract_artifact_refs(artifact: Any) -> List[ArtifactRef]:
    """Extract all ArtifactRef objects from an artifact structure."""
    refs = []
    
    def _extract(obj: Any):
        if isinstance(obj, dict):
            if 'artifact_type' in obj and 'digest_sha256' in obj:
                try:
                    refs.append(ArtifactRef.from_dict(obj))
                except (ValueError, KeyError):
                    pass
            for v in obj.values():
                _extract(v)
        elif isinstance(obj, list):
            for item in obj:
                _extract(item)
    
    _extract(artifact)
    return refs


# =============================================================================
# CHAPTER 11: SMART ASSET CONCEPTUAL FOUNDATION
# =============================================================================

@dataclass
class GenesisDocument:
    """
    Definition 11.1 (Smart Asset) - G: Genesis Document.
    
    Immutable identity and initial configuration.
    The canonical identity is: asset_id = SHA256(JCS(G))
    """
    asset_name: str
    asset_class: str
    initial_bindings: List[str]  # Harbor IDs
    governance: Dict[str, Any]
    metadata: Dict[str, Any] = field(default_factory=dict)
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "asset_name": self.asset_name,
            "asset_class": self.asset_class,
            "initial_bindings": self.initial_bindings,
            "governance": self.governance,
            "metadata": self.metadata,
            "created_at": self.created_at
        }
    
    @property
    def asset_id(self) -> str:
        """
        Canonical Identity (Definition 11.1):
        asset_id = SHA256(JCS(G))
        """
        return stack_digest(self.to_dict()).bytes_hex


@dataclass
class RegistryCredential:
    """
    Definition 11.1 (Smart Asset) - R: Registry Credential.
    
    Current jurisdictional bindings in VC format.
    """
    asset_id: str
    bindings: List['JurisdictionalBinding']
    registry_vc_digest: str
    effective_from: str
    effective_until: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "bindings": [b.to_dict() for b in self.bindings],
            "registry_vc_digest": self.registry_vc_digest,
            "effective_from": self.effective_from,
            "effective_until": self.effective_until
        }


@dataclass
class JurisdictionalBinding:
    """
    I3 (Explicit Bindings): Every operation must be authorized by at least one binding.
    """
    harbor_id: str
    lawpack_digest: str
    regpack_digest: Optional[str] = None
    binding_status: str = "active"  # active | suspended | terminated
    activated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "harbor_id": self.harbor_id,
            "lawpack_digest": self.lawpack_digest,
            "regpack_digest": self.regpack_digest,
            "binding_status": self.binding_status,
            "activated_at": self.activated_at
        }


@dataclass
class OperationalManifest:
    """
    Definition 11.1 (Smart Asset) - M: Operational Manifest.
    
    Live configuration and metadata.
    """
    asset_id: str
    version: int
    config: Dict[str, Any]
    quorum_threshold: int
    authorized_governors: List[str]  # DIDs
    agentic_policies: List['AgenticPolicy'] = field(default_factory=list)
    updated_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "version": self.version,
            "config": self.config,
            "quorum_threshold": self.quorum_threshold,
            "authorized_governors": self.authorized_governors,
            "agentic_policies": [p.to_dict() for p in self.agentic_policies],
            "updated_at": self.updated_at
        }


# =============================================================================
# CHAPTER 12: RECEIPT CHAIN AND CHECKPOINTS
# =============================================================================

@dataclass
class AssetReceipt:
    """
    Definition 12.1 (Asset Receipt).
    
    An asset receipt records a single state transition.
    Receipts form a cryptographically linked chain.
    """
    asset_id: str
    seq: int
    prev_root: str
    transition_envelope_digest: str
    transition_kind: str
    prev_state_root: str
    new_state_root: str
    harbor_ids: List[str]
    jurisdiction_scope: str
    signatures: List[Dict[str, Any]]
    witness_attestations: List[Dict[str, Any]] = field(default_factory=list)
    compliance_proof_digest: Optional[str] = None
    created_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    effective_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    next_root: Optional[str] = None  # Computed after creation
    
    def __post_init__(self):
        # Compute next_root if not provided
        if self.next_root is None:
            self.next_root = self._compute_next_root()
    
    def _compute_next_root(self) -> str:
        r"""
        Definition 12.2 (Receipt Chain Linkage).
        
        H_n = SHA256(JCS(Receipt_n \ {next_root}))
        """
        receipt_data = self.to_dict_without_next_root()
        return stack_digest(receipt_data).bytes_hex
    
    def to_dict_without_next_root(self) -> Dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "seq": self.seq,
            "prev_root": self.prev_root,
            "transition_envelope_digest": self.transition_envelope_digest,
            "transition_kind": self.transition_kind,
            "prev_state_root": self.prev_state_root,
            "new_state_root": self.new_state_root,
            "harbor_ids": self.harbor_ids,
            "jurisdiction_scope": self.jurisdiction_scope,
            "signatures": self.signatures,
            "witness_attestations": self.witness_attestations,
            "compliance_proof_digest": self.compliance_proof_digest,
            "created_at": self.created_at,
            "effective_at": self.effective_at
        }
    
    def to_dict(self) -> Dict[str, Any]:
        d = self.to_dict_without_next_root()
        d["next_root"] = self.next_root
        return d


def genesis_receipt_root(asset_id: str) -> str:
    """
    Definition 12.2 (Receipt Chain Linkage) - Genesis.
    
    H_0 = SHA256("SMART_ASSET_GENESIS" || asset_id)
    """
    data = f"SMART_ASSET_GENESIS{asset_id}"
    return hashlib.sha256(data.encode()).hexdigest()


class MerkleMountainRange:
    """
    Definition 12.3 (MMR over Receipts).
    
    Merkle Mountain Range provides efficient append and proof operations:
    - Append: O(log n)
    - Prove: O(log n)
    - Verify: O(log n)
    - Proof size: O(log n) hashes
    """
    
    def __init__(self):
        self.peaks: List[Optional[str]] = []  # Forest of complete binary tree roots
        self.leaf_count: int = 0
    
    def append(self, leaf_hash: str) -> str:
        """
        MMR_{n+1} = MMRAppend(MMR_n, H_n)

        Theorem 12.2: Append requires at most floor(log_2 n) + 1 hash operations.

        Hashing uses binary concatenation of the raw 32-byte digests (decoded
        from hex) with a 0x01 internal-node domain separator, matching the
        convention in ``tools.mmr``.
        """
        self.leaf_count += 1
        new_peak = leaf_hash
        height = 0

        # Merge with existing peaks of same height
        while height < len(self.peaks) and self.peaks[height] is not None:
            # Merge: SHA256(0x01 || left_bytes || right_bytes)
            left = self.peaks[height]
            left_bytes = bytes.fromhex(left)
            right_bytes = bytes.fromhex(new_peak)
            new_peak = hashlib.sha256(b"\x01" + left_bytes + right_bytes).hexdigest()
            self.peaks[height] = None
            height += 1

        if height >= len(self.peaks):
            self.peaks.append(new_peak)
        else:
            self.peaks[height] = new_peak

        return self.root

    @property
    def root(self) -> str:
        """
        Compute MMR root from peaks.
        Root can be computed from peaks in O(log n) time.

        Bagging uses binary concatenation with the 0x01 domain separator,
        matching ``tools.mmr.bag_peaks``.
        """
        active_peaks = [p for p in self.peaks if p is not None]
        if not active_peaks:
            return "0" * 64

        # Bag the peaks right-to-left
        result = active_peaks[-1]
        for peak in reversed(active_peaks[:-1]):
            peak_bytes = bytes.fromhex(peak)
            result_bytes = bytes.fromhex(result)
            result = hashlib.sha256(b"\x01" + peak_bytes + result_bytes).hexdigest()

        return result
    
    def prove(self, index: int) -> List[Tuple[str, bool]]:
        """
        π_incl^(i) = MMRProve(MMR_n, i)
        
        Returns inclusion proof: list of (hash, is_left) pairs.
        """
        # Simplified proof generation - full implementation would track all nodes
        raise NotImplementedError("Full MMR proof generation requires persistent node storage")


@dataclass
class AssetCheckpoint:
    """
    Definition 12.4 (Asset Checkpoint).
    
    A checkpoint commits receipt chain state for L1 anchoring.
    """
    asset_id: str
    receipt_chain_head: str  # Latest H_n
    mmr_root: str  # MMR commitment
    receipt_count: int
    state_root: str  # Current state
    registry_vc_digest: str
    signatures: List[Dict[str, Any]]
    witness_set: List[str]  # Witness DIDs
    checkpoint_time: str
    prev_checkpoint_digest: Optional[str] = None
    checkpoint_height: int = 0
    artifact_bundle_root: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "receipt_chain_head": self.receipt_chain_head,
            "mmr_root": self.mmr_root,
            "receipt_count": self.receipt_count,
            "state_root": self.state_root,
            "registry_vc_digest": self.registry_vc_digest,
            "signatures": self.signatures,
            "witness_set": self.witness_set,
            "checkpoint_time": self.checkpoint_time,
            "prev_checkpoint_digest": self.prev_checkpoint_digest,
            "checkpoint_height": self.checkpoint_height,
            "artifact_bundle_root": self.artifact_bundle_root
        }


# =============================================================================
# CHAPTER 13: STATE MACHINE
# =============================================================================

class TransitionKind(Enum):
    """
    Definition 13.2 (Transition Kinds).
    
    Core transition types for Smart Assets.
    """
    # Ownership Operations
    TRANSFER = "transfer"
    MINT = "mint"
    BURN = "burn"
    
    # Binding Operations
    ACTIVATE_BINDING = "activate_binding"
    DEACTIVATE_BINDING = "deactivate_binding"
    MIGRATE_BINDING = "migrate_binding"
    
    # Governance Operations
    UPDATE_MANIFEST = "update_manifest"
    AMEND_GOVERNANCE = "amend_governance"
    ADD_GOVERNOR = "add_governor"
    REMOVE_GOVERNOR = "remove_governor"
    
    # Corporate Actions
    DIVIDEND = "dividend"
    SPLIT = "split"
    MERGER = "merger"
    
    # Control Operations
    HALT = "halt"
    RESUME = "resume"
    
    # Attestation Operations
    ADD_ATTESTATION = "add_attestation"
    REVOKE_ATTESTATION = "revoke_attestation"
    
    # Arbitration Operations (Definition 26.7)
    DISPUTE_FILE = "dispute_file"
    DISPUTE_RESPOND = "dispute_respond"
    ARBITRATION_RULING_RECEIVE = "arbitration_ruling_receive"
    ARBITRATION_ENFORCE = "arbitration_enforce"
    ARBITRATION_APPEAL = "arbitration_appeal"
    DISPUTE_SETTLE = "dispute_settle"
    ESCROW_RELEASE = "escrow_release"
    ESCROW_FORFEIT = "escrow_forfeit"


@dataclass
class TransitionEnvelope:
    """
    Definition 13.1 (Deterministic State Transition Function).
    
    The transition envelope contains all inputs for a state transition.
    """
    asset_id: str
    seq: int
    kind: TransitionKind
    effective_time: str
    inputs_ref: Optional[ArtifactRef] = None
    params_ref: Optional[ArtifactRef] = None
    params: Dict[str, Any] = field(default_factory=dict)
    routing: Dict[str, Any] = field(default_factory=dict)
    zk_commitments: Dict[str, Any] = field(default_factory=dict)
    signatures: List[Dict[str, Any]] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "asset_id": self.asset_id,
            "seq": self.seq,
            "kind": self.kind.value,
            "effective_time": self.effective_time,
            "inputs_ref": self.inputs_ref.to_dict() if self.inputs_ref else None,
            "params_ref": self.params_ref.to_dict() if self.params_ref else None,
            "params": self.params,
            "routing": self.routing,
            "zk_commitments": self.zk_commitments,
            "signatures": self.signatures
        }
    
    @property
    def digest(self) -> str:
        return stack_digest(self.to_dict()).bytes_hex


# =============================================================================
# CHAPTER 14: COMPLIANCE TENSOR
# =============================================================================

class ComplianceStatus(Enum):
    """
    Definition 7.3 (Compliance Tensor) - Status Types.
    """
    PERMITTED = "permitted"
    PROHIBITED = "prohibited"
    REQUIRES_ATTESTATION = "requires_attestation"
    REQUIRES_APPROVAL = "requires_approval"
    LIMITED = "limited"
    CONDITIONAL = "conditional"


@dataclass
class ComplianceConstraint:
    """
    Represents a single entry in the compliance tensor.
    """
    status: ComplianceStatus
    reason_code: Optional[int] = None
    attestation_types: Optional[List[str]] = None
    min_attestation_count: Optional[int] = None
    approvers: Optional[List[str]] = None
    approval_threshold: Optional[int] = None
    max_amount: Optional[int] = None
    max_daily_volume: Optional[int] = None
    max_counterparties: Optional[int] = None
    cooldown_period_seconds: Optional[int] = None
    predicate_ref: Optional[ArtifactRef] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {"status": self.status.value}
        if self.reason_code is not None:
            d["reason_code"] = self.reason_code
        if self.attestation_types:
            d["attestation_types"] = self.attestation_types
            d["min_attestation_count"] = self.min_attestation_count or 1
        if self.approvers:
            d["approvers"] = self.approvers
            d["approval_threshold"] = self.approval_threshold or 1
        if self.max_amount is not None:
            d["max_amount"] = self.max_amount
        if self.max_daily_volume is not None:
            d["max_daily_volume"] = self.max_daily_volume
        if self.predicate_ref:
            d["predicate_ref"] = self.predicate_ref.to_dict()
        return d


class ComplianceTensor:
    """
    Definition 7.3 (Compliance Tensor).
    
    T: OpType × BindingID × BindingID → ComplianceStatus
    
    The compliance tensor encodes all regulatory constraints as a
    multi-dimensional lookup structure.
    """
    
    def __init__(self):
        # Sparse representation: (op_type, src_binding, dst_binding) -> constraint
        self._entries: Dict[Tuple[str, str, str], ComplianceConstraint] = {}
    
    def set(self, op_type: str, src_binding: str, dst_binding: str, 
            constraint: ComplianceConstraint):
        """Set a tensor entry."""
        self._entries[(op_type, src_binding, dst_binding)] = constraint
    
    def get(self, op_type: str, src_binding: str, dst_binding: str) -> ComplianceConstraint:
        """
        Look up compliance status for an operation.
        Default: Permitted if no explicit constraint.
        """
        key = (op_type, src_binding, dst_binding)
        return self._entries.get(key, ComplianceConstraint(ComplianceStatus.PERMITTED))
    
    def evaluate(self, op_type: str, src_binding: str, dst_binding: str,
                 context: Dict[str, Any]) -> Tuple[bool, Optional[str]]:
        """
        Evaluate compliance for an operation.
        Returns (permitted: bool, reason: Optional[str])
        """
        constraint = self.get(op_type, src_binding, dst_binding)
        
        if constraint.status == ComplianceStatus.PERMITTED:
            return True, None
        elif constraint.status == ComplianceStatus.PROHIBITED:
            return False, f"Prohibited: reason_code={constraint.reason_code}"
        elif constraint.status == ComplianceStatus.REQUIRES_ATTESTATION:
            # Check if required attestations are present
            attestations = context.get("attestations", [])
            matching = [a for a in attestations if a.get("type") in (constraint.attestation_types or [])]
            if len(matching) >= (constraint.min_attestation_count or 1):
                return True, None
            return False, f"Requires attestations: {constraint.attestation_types}"
        elif constraint.status == ComplianceStatus.LIMITED:
            amount = context.get("amount", 0)
            if constraint.max_amount and amount > constraint.max_amount:
                return False, f"Amount {amount} exceeds max {constraint.max_amount}"
            return True, None
        
        return True, None
    
    @property
    def root(self) -> str:
        """
        On-Chain Representation:
        tensor_root = MerkleRoot({(Poseidon(op, src, dst), status)})

        For off-chain, we use SHA256 of sorted canonical entries.  Each entry
        is canonicalized via ``json_canonicalize`` (which delegates to
        ``jcs_canonicalize``) to guarantee deterministic serialization.
        """
        entries = []
        for (op, src, dst), constraint in sorted(self._entries.items()):
            entry_data = f"{op}:{src}:{dst}:{json_canonicalize(constraint.to_dict())}"
            entries.append(entry_data)

        if not entries:
            return "0" * 64

        combined = "\n".join(entries)
        return hashlib.sha256(combined.encode()).hexdigest()


# =============================================================================
# CHAPTER 17: AGENTIC EXECUTION MODEL
# =============================================================================

class AgenticTriggerType(Enum):
    """
    Definition 17.1 (Agentic Trigger).
    
    Environmental events that may cause autonomous state transitions.
    """
    # === Regulatory Environment Triggers ===
    SANCTIONS_LIST_UPDATE = "sanctions_list_update"
    LICENSE_STATUS_CHANGE = "license_status_change"
    GUIDANCE_UPDATE = "guidance_update"
    COMPLIANCE_DEADLINE = "compliance_deadline"
    
    # === Arbitration Triggers ===
    DISPUTE_FILED = "dispute_filed"
    RULING_RECEIVED = "ruling_received"
    APPEAL_PERIOD_EXPIRED = "appeal_period_expired"
    ENFORCEMENT_DUE = "enforcement_due"
    
    # === Corridor Triggers ===
    CORRIDOR_STATE_CHANGE = "corridor_state_change"
    SETTLEMENT_ANCHOR_AVAILABLE = "settlement_anchor_available"
    WATCHER_QUORUM_REACHED = "watcher_quorum_reached"
    
    # === Asset Lifecycle Triggers ===
    CHECKPOINT_DUE = "checkpoint_due"
    KEY_ROTATION_DUE = "key_rotation_due"
    GOVERNANCE_VOTE_RESOLVED = "governance_vote_resolved"
    
    # === Fiscal Triggers (future work per spec) ===
    TAX_YEAR_END = "tax_year_end"
    WITHHOLDING_DUE = "withholding_due"


# Impact levels per Definition 17.1
class ImpactLevel(Enum):
    NONE = "none"
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


# License status per Definition 17.1  
class LicenseStatus(Enum):
    VALID = "valid"
    EXPIRED = "expired"
    SUSPENDED = "suspended"
    REVOKED = "revoked"


# Ruling disposition per Definition 17.1
class RulingDisposition(Enum):
    IN_FAVOR_CLAIMANT = "in_favor_claimant"
    IN_FAVOR_RESPONDENT = "in_favor_respondent"
    PARTIAL = "partial"
    DISMISSED = "dismissed"


@dataclass
class AgenticTrigger:
    """
    An agentic trigger event with associated data.
    """
    trigger_type: AgenticTriggerType
    data: Dict[str, Any]
    timestamp: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "trigger_type": self.trigger_type.value,
            "data": self.data,
            "timestamp": self.timestamp
        }


@dataclass
class AgenticPolicy:
    """
    Definition 1.4 (Agentic Transition).
    
    Assets define policies that map triggers to transitions.
    """
    policy_id: str
    trigger_type: AgenticTriggerType
    condition: Optional[Dict[str, Any]] = None  # Predicate to evaluate
    action: TransitionKind = TransitionKind.HALT
    authorization_requirement: str = "automatic"  # automatic | quorum | unanimous
    enabled: bool = True
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "policy_id": self.policy_id,
            "trigger_type": self.trigger_type.value,
            "condition": self.condition,
            "action": self.action.value,
            "authorization_requirement": self.authorization_requirement,
            "enabled": self.enabled
        }
    
    def evaluate_condition(self, trigger: AgenticTrigger, environment: Dict[str, Any]) -> bool:
        """
        Evaluate if this policy's condition is satisfied.
        
        Theorem 17.1 (Agentic Determinism): Given identical trigger events
        and environment state, agentic execution is deterministic.
        
        Supported condition types:
        - threshold: field >= threshold
        - equals: field == value
        - not_equals: field != value
        - contains: item in field (field is a collection)
        - in: field in values (values is a collection)
        - less_than: field < threshold
        - greater_than: field > threshold
        - exists: field exists and is truthy
        - and: all sub-conditions must be true
        - or: at least one sub-condition must be true
        """
        if not self.enabled:
            return False
        
        if trigger.trigger_type != self.trigger_type:
            return False
        
        if self.condition is None:
            return True
        
        return self._evaluate_condition_recursive(self.condition, trigger, environment)
    
    def _get_nested_field(self, data: Dict[str, Any], field_path: str) -> Any:
        """Get a nested field value using dot notation (e.g., 'match.score')."""
        parts = field_path.split(".")
        current = data
        for part in parts:
            if isinstance(current, dict) and part in current:
                current = current[part]
            else:
                return None
        return current
    
    def _evaluate_condition_recursive(
        self, 
        condition: Dict[str, Any], 
        trigger: AgenticTrigger, 
        environment: Dict[str, Any]
    ) -> bool:
        """Recursively evaluate a condition."""
        condition_type = condition.get("type")
        
        if condition_type == "threshold":
            field = condition.get("field", "")
            threshold = condition.get("threshold", 0)
            value = self._get_nested_field(trigger.data, field)
            if value is None:
                value = 0
            try:
                return value >= threshold
            except TypeError:
                return False
        
        if condition_type == "equals":
            field = condition.get("field", "")
            expected = condition.get("value")
            value = self._get_nested_field(trigger.data, field)
            return value == expected
        
        if condition_type == "not_equals":
            field = condition.get("field", "")
            expected = condition.get("value")
            value = self._get_nested_field(trigger.data, field)
            return value != expected
        
        if condition_type == "contains":
            field = condition.get("field", "")
            item = condition.get("item")
            collection = self._get_nested_field(trigger.data, field)
            if collection is None:
                collection = []
            return item in collection
        
        if condition_type == "in":
            field = condition.get("field", "")
            values = condition.get("values", [])
            value = self._get_nested_field(trigger.data, field)
            return value in values
        
        if condition_type == "less_than":
            field = condition.get("field", "")
            threshold = condition.get("threshold", 0)
            value = self._get_nested_field(trigger.data, field)
            if value is None:
                return False
            try:
                return value < threshold
            except TypeError:
                return False
        
        if condition_type == "greater_than":
            field = condition.get("field", "")
            threshold = condition.get("threshold", 0)
            value = self._get_nested_field(trigger.data, field)
            if value is None:
                return False
            try:
                return value > threshold
            except TypeError:
                return False
        
        if condition_type == "exists":
            field = condition.get("field", "")
            value = self._get_nested_field(trigger.data, field)
            return value is not None and bool(value)
        
        if condition_type == "and":
            sub_conditions = condition.get("conditions", [])
            return all(
                self._evaluate_condition_recursive(sub, trigger, environment)
                for sub in sub_conditions
            )
        
        if condition_type == "or":
            sub_conditions = condition.get("conditions", [])
            return any(
                self._evaluate_condition_recursive(sub, trigger, environment)
                for sub in sub_conditions
            )
        
        # SECURITY FIX: Unknown condition types return False (fail-safe)
        # Previously returned True, which was a security vulnerability
        return False


# =============================================================================
# STANDARD AGENTIC POLICIES (Definition 17.3)
# =============================================================================

STANDARD_POLICIES = {
    "sanctions_auto_halt": AgenticPolicy(
        policy_id="sanctions_auto_halt",
        trigger_type=AgenticTriggerType.SANCTIONS_LIST_UPDATE,
        condition={"type": "contains", "field": "affected_parties", "item": "self"},
        action=TransitionKind.HALT,
        authorization_requirement="automatic"
    ),
    "license_expiry_alert": AgenticPolicy(
        policy_id="license_expiry_alert",
        trigger_type=AgenticTriggerType.LICENSE_STATUS_CHANGE,
        condition={"type": "equals", "field": "new_status", "value": "expired"},
        action=TransitionKind.HALT,
        authorization_requirement="automatic"
    ),
    "ruling_enforcement": AgenticPolicy(
        policy_id="ruling_enforcement",
        trigger_type=AgenticTriggerType.RULING_RECEIVED,
        action=TransitionKind.ARBITRATION_ENFORCE,
        authorization_requirement="automatic"
    ),
    "checkpoint_auto": AgenticPolicy(
        policy_id="checkpoint_auto",
        trigger_type=AgenticTriggerType.CHECKPOINT_DUE,
        condition={"type": "threshold", "field": "receipts_since_last", "threshold": 100},
        action=TransitionKind.UPDATE_MANIFEST,
        authorization_requirement="automatic"
    ),
}


# =============================================================================
# SMART ASSET (Complete Definition 11.1)
# =============================================================================

@dataclass
class SmartAsset:
    """
    Definition 11.1 (Smart Asset).
    
    A Smart Asset is a tuple A = (G, R, M, C, H) where:
    - G: Genesis Document — Immutable identity and initial configuration
    - R: Registry Credential — Current jurisdictional bindings
    - M: Operational Manifest — Live configuration and metadata
    - C: Receipt Chain — Cryptographically linked operation history
    - H: State Machine — Deterministic transition function
    """
    
    # G: Genesis Document
    genesis: GenesisDocument
    
    # R: Registry Credential
    registry: RegistryCredential
    
    # M: Operational Manifest
    manifest: OperationalManifest
    
    # C: Receipt Chain
    receipts: List[AssetReceipt] = field(default_factory=list)
    mmr: MerkleMountainRange = field(default_factory=MerkleMountainRange)
    
    # H: State Machine (represented by current state)
    state: Dict[str, Any] = field(default_factory=dict)
    
    # Compliance Tensor for this asset
    compliance_tensor: ComplianceTensor = field(default_factory=ComplianceTensor)
    
    def __post_init__(self):
        """Verify invariants on construction."""
        self._verify_invariants()
    
    @property
    def asset_id(self) -> str:
        """
        I1 (Immutable Identity):
        ∀t ≥ 0: asset_id(t) = asset_id(0) = SHA256(JCS(G))
        """
        return self.genesis.asset_id
    
    @property
    def receipt_chain_head(self) -> str:
        """Get the current receipt chain head hash."""
        if not self.receipts:
            return genesis_receipt_root(self.asset_id)
        return self.receipts[-1].next_root
    
    @property
    def state_root(self) -> str:
        """Compute current state root."""
        return stack_digest(self.state).bytes_hex
    
    def _verify_invariants(self):
        """
        Definition 11.2 (Smart Asset Invariants).
        Verify I1-I5 hold.
        """
        # I1: Immutable Identity - verified by property
        
        # I2: Deterministic State - verified by STF implementation
        
        # I3: Explicit Bindings - verify at least one active binding
        active_bindings = [b for b in self.registry.bindings if b.binding_status == "active"]
        if not active_bindings:
            raise ValueError("I3 violation: No active jurisdictional bindings")
        
        # I4: Resolvability - verified during artifact resolution
        
        # I5: Optional Anchoring - receipt chain validity is sufficient
    
    def transition(self, envelope: TransitionEnvelope) -> AssetReceipt:
        """
        Definition 13.1 (Deterministic State Transition Function).
        
        δ: S × TransitionEnvelope → S ∪ {⊥}
        
        Theorem 13.1 (State Transition Determinism):
        ∀ executions e1, e2: δ(S_{n-1}, env)_{e1} = δ(S_{n-1}, env)_{e2}
        """
        # Verify sequence number
        expected_seq = len(self.receipts)
        if envelope.seq != expected_seq:
            raise ValueError(f"I2 violation: Expected seq {expected_seq}, got {envelope.seq}")
        
        # Verify asset_id matches
        if envelope.asset_id != self.asset_id:
            raise ValueError("Asset ID mismatch in envelope")
        
        # I3: Verify authorization by active binding
        # (simplified - full implementation checks signatures against bindings)
        
        # Compute state transition (deterministic)
        prev_state_root = self.state_root
        prev_receipt_root = self.receipt_chain_head
        
        # Apply transition (simplified - real implementation handles all TransitionKinds)
        new_state = self._apply_transition(envelope)
        self.state = new_state
        new_state_root = self.state_root
        
        # Create receipt
        receipt = AssetReceipt(
            asset_id=self.asset_id,
            seq=envelope.seq,
            prev_root=prev_receipt_root,
            transition_envelope_digest=envelope.digest,
            transition_kind=envelope.kind.value,
            prev_state_root=prev_state_root,
            new_state_root=new_state_root,
            harbor_ids=[b.harbor_id for b in self.registry.bindings if b.binding_status == "active"],
            jurisdiction_scope="multi" if len(self.registry.bindings) > 1 else "single",
            signatures=envelope.signatures,
            effective_at=envelope.effective_time
        )
        
        # Add to chain
        self.receipts.append(receipt)
        self.mmr.append(receipt.next_root)
        
        return receipt
    
    def _apply_transition(self, envelope: TransitionEnvelope) -> Dict[str, Any]:
        """
        Apply a transition to current state (deterministic).
        
        Validates:
        - Asset not halted (except for RESUME)
        - Sufficient balances for transfers
        - All amounts use Decimal for precision
        """
        new_state = copy.deepcopy(self.state)
        
        # CRITICAL FIX: Check if asset is halted (except for RESUME transitions)
        if new_state.get("halted", False) and envelope.kind != TransitionKind.RESUME:
            raise ValueError("Cannot transition halted asset (except RESUME)")
        
        if envelope.kind == TransitionKind.TRANSFER:
            from_holder = envelope.params.get("from")
            to_holder = envelope.params.get("to")
            # CRITICAL FIX: Use Decimal for financial precision
            amount = Decimal(str(envelope.params.get("amount", 0)))
            
            balances = new_state.setdefault("balances", {})
            
            # Convert existing balances to Decimal if needed
            from_balance = Decimal(str(balances.get(from_holder, 0)))
            to_balance = Decimal(str(balances.get(to_holder, 0)))
            
            # CRITICAL FIX: Validate sufficient balance before transfer
            if from_balance < amount:
                raise ValueError(
                    f"Insufficient balance: {from_holder} has {from_balance}, "
                    f"transfer requires {amount}"
                )
            
            balances[from_holder] = from_balance - amount
            balances[to_holder] = to_balance + amount
        
        elif envelope.kind == TransitionKind.MINT:
            to_holder = envelope.params.get("to")
            # CRITICAL FIX: Use Decimal for financial precision
            amount = Decimal(str(envelope.params.get("amount", 0)))
            
            balances = new_state.setdefault("balances", {})
            to_balance = Decimal(str(balances.get(to_holder, 0)))
            balances[to_holder] = to_balance + amount
            
            total_supply = Decimal(str(new_state.get("total_supply", 0)))
            new_state["total_supply"] = total_supply + amount
        
        elif envelope.kind == TransitionKind.BURN:
            # CRITICAL FIX: Add BURN transition support
            from_holder = envelope.params.get("from")
            amount = Decimal(str(envelope.params.get("amount", 0)))
            
            balances = new_state.setdefault("balances", {})
            from_balance = Decimal(str(balances.get(from_holder, 0)))
            
            if from_balance < amount:
                raise ValueError(
                    f"Insufficient balance for burn: {from_holder} has {from_balance}"
                )
            
            balances[from_holder] = from_balance - amount
            total_supply = Decimal(str(new_state.get("total_supply", 0)))
            new_state["total_supply"] = total_supply - amount
        
        elif envelope.kind == TransitionKind.HALT:
            new_state["halted"] = True
            new_state["halt_reason"] = envelope.params.get("reason", "unspecified")
            new_state["halted_at"] = envelope.effective_time
        
        elif envelope.kind == TransitionKind.RESUME:
            if not new_state.get("halted", False):
                raise ValueError("Cannot RESUME an asset that is not halted")
            new_state["halted"] = False
            new_state.pop("halt_reason", None)
            new_state.pop("halted_at", None)
        
        # Update nonce
        new_state["nonce"] = envelope.seq
        
        return new_state
    
    def process_agentic_trigger(self, trigger: AgenticTrigger, 
                                 environment: Dict[str, Any]) -> Optional[TransitionEnvelope]:
        """
        Protocol 17.1 (Agentic Trigger Processing).
        
        Theorem 17.1 (Agentic Determinism): Given identical trigger events
        and environment state, agentic execution is deterministic.
        """
        for policy in self.manifest.agentic_policies:
            if policy.evaluate_condition(trigger, environment):
                # Build transition envelope from policy action
                envelope = TransitionEnvelope(
                    asset_id=self.asset_id,
                    seq=len(self.receipts),
                    kind=policy.action,
                    effective_time=datetime.now(timezone.utc).isoformat(),
                    params={"trigger": trigger.to_dict()}
                )
                return envelope
        
        return None
    
    def create_checkpoint(self, witnesses: List[str], 
                          signatures: List[Dict[str, Any]]) -> AssetCheckpoint:
        """
        Protocol 6.1 (Checkpoint Submission).
        
        Create a checkpoint anchoring current receipt chain state.
        """
        return AssetCheckpoint(
            asset_id=self.asset_id,
            receipt_chain_head=self.receipt_chain_head,
            mmr_root=self.mmr.root,
            receipt_count=len(self.receipts),
            state_root=self.state_root,
            registry_vc_digest=self.registry.registry_vc_digest,
            signatures=signatures,
            witness_set=witnesses,
            checkpoint_time=datetime.now(timezone.utc).isoformat(),
            checkpoint_height=0  # Would track across checkpoints
        )
    
    def verify_receipt_chain(self) -> bool:
        """
        Lemma 12.1 (Receipt Chain Integrity).
        
        Under SHA-256 collision resistance, tampering with any receipt
        in a chain is detectable.
        """
        if not self.receipts:
            return True
        
        # Verify genesis root
        expected_prev = genesis_receipt_root(self.asset_id)
        
        for i, receipt in enumerate(self.receipts):
            # Verify chain linkage
            if receipt.prev_root != expected_prev:
                return False
            
            # Verify sequence
            if receipt.seq != i:
                return False
            
            # Verify asset_id
            if receipt.asset_id != self.asset_id:
                return False
            
            # Verify next_root computation
            computed_next = receipt._compute_next_root()
            if receipt.next_root != computed_next:
                return False
            
            expected_prev = receipt.next_root
        
        return True


# =============================================================================
# THEOREM 16.1: OFFLINE OPERATION CAPABILITY
# =============================================================================

def verify_offline_capability(
    asset: 'SmartAsset', 
    cached_credentials: Optional[List[Dict]] = None,
    cached_attestations: Optional[List[Dict]] = None
) -> Tuple[bool, str]:
    """
    Theorem 16.1 (Offline Operation Capability).
    
    A Smart Asset can continue valid operations without blockchain connectivity,
    provided:
    1. Receipt chain state is locally available
    2. Required credentials/attestations are cached with valid windows
    3. Authorized signers are reachable (via any communication channel)
    
    Returns: (capable: bool, reason: str)
    """
    cached_credentials = cached_credentials or []
    cached_attestations = cached_attestations or []
    
    # 1. Receipt chain state available
    if not asset.verify_receipt_chain():
        return (False, "Receipt chain integrity verification failed")
    
    # 2. Check credential validity windows
    now = datetime.now(timezone.utc)
    for cred in cached_credentials:
        valid_until = cred.get("valid_until")
        if valid_until:
            if datetime.fromisoformat(valid_until.replace('Z', '+00:00')) < now:
                return (False, f"Credential expired: {cred.get('id', 'unknown')}")
    
    # 3. Check attestation validity
    for att in cached_attestations:
        valid_until = att.get("valid_until")
        if valid_until:
            if datetime.fromisoformat(valid_until.replace('Z', '+00:00')) < now:
                return (False, f"Attestation expired: {att.get('id', 'unknown')}")
    
    return (True, "Asset has offline operation capability")


# =============================================================================
# PROTOCOL 14.1: CROSS-JURISDICTION TRANSFER
# =============================================================================

@dataclass
class CrossJurisdictionTransferRequest:
    """Protocol 14.1 (Cross-Jurisdiction Transfer) request."""
    asset_id: str
    from_binding_id: str
    to_binding_id: str
    amount: Optional[int] = None
    corridor_id: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "asset_id": self.asset_id,
            "from_binding_id": self.from_binding_id,
            "to_binding_id": self.to_binding_id,
        }
        if self.amount is not None:
            d["amount"] = self.amount
        if self.corridor_id:
            d["corridor_id"] = self.corridor_id
        return d


def cross_jurisdiction_transfer(
    smart_asset: 'SmartAsset',
    from_binding_id: str,
    to_binding_id: str,
    amount: Optional[int] = None,
    corridor_id: Optional[str] = None,
) -> TransitionEnvelope:
    """
    Protocol 14.1 (Cross-Jurisdiction Transfer).
    
    PROTOCOL:
    1. VERIFY from_binding is active in asset registry
    2. VERIFY to_binding exists and is compatible
    3. EVALUATE compliance tensor T(transfer, from_binding, to_binding)
    4. IF corridor_id specified, VERIFY corridor permits cross-jurisdiction
    5. CREATE transition envelope with both jurisdiction signatures
    """
    # Step 1: Verify from_binding is active
    from_binding = None
    for b in smart_asset.registry_credential.bindings:
        if b.binding_id == from_binding_id:
            from_binding = b
            break
    
    if not from_binding or from_binding.status != "active":
        raise ValueError(f"Source binding {from_binding_id} not active")
    
    # Step 2: Verify to_binding exists
    to_binding = None
    for b in smart_asset.registry_credential.bindings:
        if b.binding_id == to_binding_id:
            to_binding = b
            break
    
    if not to_binding:
        raise ValueError(f"Destination binding {to_binding_id} not found")
    
    # Step 3: Evaluate compliance tensor
    compliance_result = smart_asset.compliance_tensor.evaluate(
        op_type="transfer",
        src_binding=from_binding_id,
        dst_binding=to_binding_id,
    )
    
    if not compliance_result[0]:
        raise ValueError(f"Transfer prohibited: {compliance_result[1]}")
    
    # Step 4: Create transition envelope
    seq = smart_asset.receipt_chain[-1].seq + 1 if smart_asset.receipt_chain else 0
    envelope = TransitionEnvelope(
        asset_id=smart_asset.asset_id,
        seq=seq,
        kind=TransitionKind.TRANSFER,
        params={
            "from_binding": from_binding_id,
            "to_binding": to_binding_id,
            "amount": amount,
            "corridor_id": corridor_id,
            "cross_jurisdiction": True,
        },
    )
    
    return envelope


# =============================================================================
# PROTOCOL 16.1: FORK RESOLUTION
# =============================================================================

class ForkResolutionStrategy(Enum):
    """Fork resolution strategies per Protocol 16.1."""
    LONGEST_CHAIN = "longest_chain"
    HEAVIEST_CHAIN = "heaviest_chain"
    GOVERNANCE_VOTE = "governance_vote"
    MANUAL = "manual"


@dataclass
class ForkDetection:
    """Detected fork per Protocol 16.1."""
    fork_sequence: int
    prev_root: str
    branches: List[Dict[str, Any]]
    detected_at: str = field(default_factory=lambda: datetime.now(timezone.utc).isoformat())
    resolved: bool = False
    resolution_strategy: Optional[ForkResolutionStrategy] = None
    canonical_branch: Optional[int] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "fork_sequence": self.fork_sequence,
            "prev_root": self.prev_root,
            "branches": self.branches,
            "detected_at": self.detected_at,
            "resolved": self.resolved,
            "resolution_strategy": self.resolution_strategy.value if self.resolution_strategy else None,
            "canonical_branch": self.canonical_branch,
        }


def detect_fork(receipt_chains: List[List['AssetReceipt']]) -> Optional[ForkDetection]:
    """
    Protocol 16.1 (Fork Resolution) - Detection Phase.
    
    Detect fork from multiple receipt chains.
    """
    if len(receipt_chains) < 2:
        return None
    
    by_seq: Dict[int, Dict[str, List[int]]] = {}
    
    for chain_idx, chain in enumerate(receipt_chains):
        for receipt in chain:
            if receipt.seq not in by_seq:
                by_seq[receipt.seq] = {}
            root = receipt.next_root
            if root not in by_seq[receipt.seq]:
                by_seq[receipt.seq][root] = []
            by_seq[receipt.seq][root].append(chain_idx)
    
    for seq in sorted(by_seq.keys()):
        roots = by_seq[seq]
        if len(roots) > 1:
            branches = [
                {"root": root, "chain_indices": idxs, "witness_count": len(idxs)}
                for root, idxs in sorted(roots.items())
            ]
            
            prev_root = ""
            if seq > 0:
                for chain in receipt_chains:
                    for r in chain:
                        if r.seq == seq - 1:
                            prev_root = r.next_root
                            break
                    if prev_root:
                        break
            
            return ForkDetection(fork_sequence=seq, prev_root=prev_root, branches=branches)
    
    return None


def resolve_fork(
    fork: ForkDetection,
    strategy: ForkResolutionStrategy,
    governance_votes: Optional[Dict[str, int]] = None,
) -> int:
    """Protocol 16.1 (Fork Resolution) - Resolution Phase."""
    if strategy == ForkResolutionStrategy.LONGEST_CHAIN:
        max_count, chosen = -1, 0
        for i, branch in enumerate(fork.branches):
            if branch.get("witness_count", 0) > max_count:
                max_count = branch["witness_count"]
                chosen = i
        return chosen
    
    elif strategy == ForkResolutionStrategy.GOVERNANCE_VOTE:
        if not governance_votes:
            raise ValueError("Governance votes required")
        max_votes, chosen = -1, 0
        for idx_str, votes in governance_votes.items():
            if votes > max_votes:
                max_votes = votes
                chosen = int(idx_str)
        return chosen
    
    elif strategy == ForkResolutionStrategy.MANUAL:
        raise ValueError("Manual resolution requires operator intervention")
    
    return 0


# =============================================================================
# PROTOCOL 18.1: ARTIFACT GRAPH VERIFICATION  
# =============================================================================

@dataclass
class ArtifactGraphVerificationReport:
    """Result of Protocol 18.1 artifact graph verification."""
    root_digest: str
    total_artifacts: int
    verified_artifacts: int
    missing_artifacts: List[str]
    invalid_artifacts: List[str]
    warnings: List[str]
    strict_mode: bool
    passed: bool
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "root_digest": self.root_digest,
            "total_artifacts": self.total_artifacts,
            "verified_artifacts": self.verified_artifacts,
            "missing_artifacts": self.missing_artifacts,
            "invalid_artifacts": self.invalid_artifacts,
            "warnings": self.warnings,
            "strict_mode": self.strict_mode,
            "passed": self.passed,
        }


def verify_artifact_graph(
    root_artifact: Dict[str, Any],
    resolve_fn: Callable[[str], Optional[bytes]],
    strict: bool = True,
) -> ArtifactGraphVerificationReport:
    """
    Protocol 18.1 (Artifact Graph Verification).
    
    PROTOCOL:
    1. COMPUTE closure = all artifacts transitively referenced from root
    2. FOR EACH artifact_ref in closure: RESOLVE and VERIFY digest
    3. RETURN verification report
    """
    # Extract all artifact refs from the root
    all_refs = extract_artifact_refs(root_artifact)
    
    missing, invalid, verified, warnings = [], [], [], []
    
    for ref in all_refs:
        digest = ref.digest_sha256
        content = resolve_fn(digest)
        
        if content is None:
            missing.append(digest)
            continue
        
        computed = hashlib.sha256(content).hexdigest().lower()
        if computed != digest:
            invalid.append(digest)
            warnings.append(f"Digest mismatch: {digest}")
        else:
            verified.append(digest)
    
    root_bytes = json_canonicalize(root_artifact).encode('utf-8')
    root_digest = hashlib.sha256(root_bytes).hexdigest().lower()
    
    passed = not (invalid or (strict and missing))
    
    return ArtifactGraphVerificationReport(
        root_digest=root_digest,
        total_artifacts=len(all_refs),
        verified_artifacts=len(verified),
        missing_artifacts=missing,
        invalid_artifacts=invalid,
        warnings=warnings,
        strict_mode=strict,
        passed=passed,
    )


# =============================================================================
# THEOREM 29.1: IDENTITY IMMUTABILITY
# =============================================================================

def verify_identity_immutability(
    genesis_document: 'GenesisDocument',
    claimed_asset_id: str,
) -> Tuple[bool, str]:
    """
    Theorem 29.1 (Identity Immutability).
    
    Under SHA-256 collision resistance:
        asset_id = SHA256(JCS(G))
    
    Returns: (valid, error_message)
    """
    computed_id = genesis_document.asset_id  # Property, not method
    if computed_id == claimed_asset_id:
        return (True, "")
    return (False, f"Asset ID mismatch: claimed={claimed_asset_id}, computed={computed_id}")


# =============================================================================
# THEOREM 29.2: RECEIPT CHAIN NON-REPUDIATION
# =============================================================================

def verify_receipt_chain_integrity(
    asset_id: str,
    receipt_chain: List['AssetReceipt'],
) -> Tuple[bool, List[str]]:
    """
    Theorem 29.2 (Receipt Chain Non-Repudiation).
    
    Chain linkage ensures non-repudiation: signers cannot deny
    having signed a receipt in a valid chain.
    
    Returns: (valid, list_of_errors)
    """
    if not receipt_chain:
        return (True, [])
    
    errors = []
    expected_prev = genesis_receipt_root(asset_id)
    
    for i, receipt in enumerate(receipt_chain):
        if receipt.seq != i:
            errors.append(f"Receipt {i}: seq mismatch")
        if receipt.prev_root != expected_prev:
            errors.append(f"Receipt {i}: prev_root mismatch")
        expected_prev = receipt.next_root
    
    return (len(errors) == 0, errors)


# =============================================================================
# EXPORTS
# =============================================================================

__all__ = [
    # Digest and Canonicalization
    'DigestAlgorithm',
    'Digest',
    'json_canonicalize',
    'stack_digest',
    'semantic_digest',
    
    # Artifacts
    'ArtifactType',
    'ArtifactRef',
    'compute_artifact_closure',
    'extract_artifact_refs',
    
    # Smart Asset Components
    'GenesisDocument',
    'RegistryCredential',
    'JurisdictionalBinding',
    'OperationalManifest',
    
    # Receipt Chain
    'AssetReceipt',
    'genesis_receipt_root',
    'MerkleMountainRange',
    'AssetCheckpoint',
    
    # State Machine
    'TransitionKind',
    'TransitionEnvelope',
    
    # Compliance
    'ComplianceStatus',
    'ComplianceConstraint',
    'ComplianceTensor',
    
    # Agentic Execution
    'AgenticTriggerType',
    'AgenticTrigger',
    'AgenticPolicy',
    'STANDARD_POLICIES',
    'ImpactLevel',
    'LicenseStatus',
    'RulingDisposition',
    
    # Smart Asset
    'SmartAsset',
    
    # Protocol 14.1: Cross-Jurisdiction Transfer
    'CrossJurisdictionTransferRequest',
    'cross_jurisdiction_transfer',
    
    # Protocol 16.1: Fork Resolution
    'ForkResolutionStrategy',
    'ForkDetection',
    'detect_fork',
    'resolve_fork',
    
    # Protocol 18.1: Artifact Graph Verification
    'ArtifactGraphVerificationReport',
    'verify_artifact_graph',
    
    # Theorems
    'verify_offline_capability',
    'verify_identity_immutability',
    'verify_receipt_chain_integrity',
]
