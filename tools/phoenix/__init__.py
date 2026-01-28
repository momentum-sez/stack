"""
PHOENIX — Smart Asset Operating System

The foundational infrastructure enabling autonomous Smart Assets to operate
across programmable jurisdictions through cryptographic primitives.

PHOENIX transforms traditional assets into Smart Assets—entities with embedded
compliance intelligence that can autonomously migrate between jurisdictions
based on service quality, regulatory efficiency, and economic incentives.

Architecture
────────────

    ┌─────────────────────────────────────────────────────────────────────────┐
    │                     SMART ASSET OPERATING SYSTEM                         │
    │                                                                          │
    │  LAYER 3: NETWORK COORDINATION                                          │
    │    watcher.py     Bonded attestations with slashing for misbehavior     │
    │    security.py    Defense-in-depth: replay, TOCTOU, front-running       │
    │    hardening.py   Validation, thread safety, economic guards            │
    │                                                                          │
    │  LAYER 2: JURISDICTIONAL INFRASTRUCTURE                                 │
    │    manifold.py    Path planning through jurisdictional landscape        │
    │    migration.py   Saga-based cross-jurisdictional state machine         │
    │    bridge.py      Two-phase commit multi-hop corridor transfers         │
    │    anchor.py      Settlement finality via Ethereum and L2 networks      │
    │                                                                          │
    │  LAYER 1: ASSET INTELLIGENCE                                            │
    │    tensor.py      4D compliance tensor with lattice algebra             │
    │    zkp.py         Zero-knowledge proof circuits and verification        │
    │    vm.py          Stack-based VM with compliance coprocessors           │
    │                                                                          │
    └─────────────────────────────────────────────────────────────────────────┘

Core Concepts
─────────────

    Smart Asset: An asset with embedded compliance intelligence represented as
    a 4D tensor. The asset can evaluate its own compliance in any jurisdiction,
    identify attestation gaps for migration paths, and execute autonomous
    transfers through corridor networks.

    Compliance Tensor: Mathematical structure C: Asset × Jurisdiction × Domain × Time → State
    with lattice algebra semantics. States compose pessimistically (COMPLIANT ∧ PENDING = PENDING)
    and UNKNOWN defaults to NON_COMPLIANT for fail-safe behavior.

    Corridor: Bilateral agreement between jurisdictions enabling asset movement.
    Each corridor specifies entry requirements, fee schedules, settlement
    mechanisms, and watcher quorum thresholds.

    Watcher: Economically-accountable attestor who stakes collateral proportional
    to attested transaction volume. Slashing conditions enforce honest behavior:
    100% for equivocation, 50% for false attestation, 1% for availability failure.

Design Principles
─────────────────

    Fail-Safe Defaults: Unknown compliance states default to non-compliant.
    Missing attestations invalidate compliance. The system fails closed.

    Cryptographic Integrity: Every state transition produces verifiable proof.
    Tensor commitments are Merkle roots. Attestations are content-addressed.
    Receipts chain cryptographically.

    Atomic Operations: Migrations complete fully or compensate entirely.
    Two-phase commit ensures no partial states. Saga patterns handle failures.

    Economic Accountability: Watchers stake real collateral. Misbehavior is
    slashed automatically. Incentives align with honest behavior.

    Privacy by Design: ZK proofs verify without disclosure. Selective tensor
    slices reveal only necessary state. Range proofs hide exact amounts.

Module Index
────────────

    tensor.py       Compliance Tensor            955 lines
    zkp.py          ZK Proof Infrastructure      766 lines
    manifold.py     Compliance Manifold        1,009 lines
    migration.py    Migration Protocol           886 lines
    bridge.py       Corridor Bridge              822 lines
    anchor.py       L1 Anchor Network            816 lines
    watcher.py      Watcher Economy              750 lines
    vm.py           Smart Asset VM             1,285 lines
    security.py     Security Layer               993 lines
    hardening.py    Hardening Layer              744 lines

    Total: 9,221 lines across 11 modules with 92 tests

Copyright © 2026 Momentum. All rights reserved.
Contact: engineering@momentum.inc
"""

__version__ = "0.4.43"
__codename__ = "PHOENIX ASCENSION"

# Lazy imports to avoid circular dependencies
def __getattr__(name):
    """Lazy import PHOENIX modules on first access."""
    
    # Tensor exports
    if name in ("ComplianceDomain", "ComplianceState", "ComplianceTensorV2", 
                "TensorSlice", "TensorCommitment", "ComplianceProof", "AttestationRef",
                "TensorCoord", "TensorCell", "tensor_meet", "tensor_join"):
        from tools.phoenix import tensor
        return getattr(tensor, name)
    
    # ZK exports
    if name in ("ProofSystem", "Circuit", "CircuitRegistry", "Witness", "Proof",
                "VerificationKey", "ProvingKey", "CircuitType", "MockProver",
                "MockVerifier", "create_standard_registry"):
        from tools.phoenix import zkp
        return getattr(zkp, name)
    
    # Manifold exports
    if name in ("ComplianceManifold", "MigrationPath", "AttestationRequirement",
                "PathConstraint", "JurisdictionNode", "CorridorEdge", "AttestationGap",
                "AttestationType", "MigrationHop", "create_standard_manifold"):
        from tools.phoenix import manifold
        return getattr(manifold, name)
    
    # Migration exports
    if name in ("MigrationSaga", "MigrationState", "MigrationRequest",
                "MigrationEvidence", "CompensationAction", "MigrationOrchestrator",
                "StateTransition", "LockEvidence", "TransitProof", "VerificationResult"):
        from tools.phoenix import migration
        return getattr(migration, name)
    
    # Watcher exports
    if name in ("WatcherBond", "SlashingCondition", "SlashingClaim",
                "WatcherReputation", "WatcherRegistry", "WatcherId", "BondStatus",
                "ReputationMetrics", "EquivocationDetector", "SlashingEvidence"):
        from tools.phoenix import watcher
        return getattr(watcher, name)
    
    # Anchor exports
    if name in ("Chain", "AnchorStatus", "AnchorManager", "AnchorRecord",
                "CorridorCheckpoint", "InclusionProof", "MockChainAdapter",
                "CrossChainVerifier", "CrossChainVerification",
                "create_mock_anchor_manager"):
        from tools.phoenix import anchor
        return getattr(anchor, name)
    
    # Bridge exports
    if name in ("CorridorBridge", "BridgePhase", "BridgeRequest", "BridgeExecution",
                "HopExecution", "HopStatus", "PrepareReceipt", "CommitReceipt",
                "BridgeReceiptChain", "create_bridge_with_manifold"):
        from tools.phoenix import bridge
        return getattr(bridge, name)
    
    # Hardening exports
    if name in ("ValidationError", "ValidationErrors", "SecurityViolation",
                "InvariantViolation", "EconomicAttackDetected", "ValidationResult",
                "Validators", "CryptoUtils", "ThreadSafeDict", "AtomicCounter",
                "InvariantChecker", "EconomicGuard", "RateLimiter", "RateLimitConfig"):
        from tools.phoenix import hardening
        return getattr(hardening, name)
    
    # Security exports
    if name in ("AttestationScope", "ScopedAttestation", "NonceRegistry",
                "VersionedValue", "VersionedStore", "TimeLock", "TimeLockState",
                "TimeLockManager", "SignatureScheme", "SignedMessage",
                "SignatureVerifier", "AuditEventType", "AuditEvent", "AuditLogger",
                "WithdrawalRequest", "SecureWithdrawalManager"):
        from tools.phoenix import security
        return getattr(security, name)
    
    # VM exports
    if name in ("OpCode", "Word", "ExecutionContext", "VMState", "GasCosts",
                "ExecutionResult", "ComplianceCoprocessor", "MigrationCoprocessor",
                "SmartAssetVM", "Assembler"):
        from tools.phoenix import vm
        return getattr(vm, name)
    
    raise AttributeError(f"module 'phoenix' has no attribute '{name}'")

__all__ = [
    # Version info
    "__version__",
    "__codename__",
    # Tensor
    "ComplianceDomain",
    "ComplianceState", 
    "ComplianceTensorV2",
    "TensorSlice",
    "TensorCommitment",
    "ComplianceProof",
    # ZK
    "ProofSystem",
    "Circuit",
    "CircuitRegistry",
    "Witness",
    "Proof",
    "VerificationKey",
    "ProvingKey",
    # Manifold
    "ComplianceManifold",
    "MigrationPath",
    "AttestationRequirement",
    "PathConstraint",
    # Migration
    "MigrationSaga",
    "MigrationState",
    "MigrationRequest",
    "MigrationEvidence",
    "CompensationAction",
    # Watcher
    "WatcherBond",
    "SlashingCondition",
    "SlashingClaim",
    "WatcherReputation",
    "WatcherRegistry",
]
