<div align="center">

# MSEZ Stack

## Smart Asset Operating System

**v0.4.43 PHOENIX ASCENSION**

[![Tests](https://img.shields.io/badge/tests-92%20passing-brightgreen?style=flat-square)]()
[![PHOENIX Modules](https://img.shields.io/badge/PHOENIX-11%20modules-purple?style=flat-square)]()
[![Lines](https://img.shields.io/badge/lines-9,221-blue?style=flat-square)]()
[![Python](https://img.shields.io/badge/python-3.10%2B-blue?style=flat-square)]()

---

**Infrastructure for autonomous Smart Assets across programmable jurisdictions.**

Compliance Tensor · Zero-Knowledge Proofs · Smart Asset VM · Economic Accountability

[**Quick Start →**](#quick-start) · [Architecture](#architecture) · [PHOENIX Modules](#phoenix-modules) · [Examples](#examples)

</div>

---

## Vision

Traditional assets are prisoners of territorial sovereignty—bound to single jurisdictions by manual compliance processes, paper-based audits, and bilateral trust relationships that take months to establish. Cross-border movement requires navigating 195+ incompatible regulatory regimes, each demanding its own documentation, verification, and settlement procedures.

**Smart Assets transcend these limitations.**

A Smart Asset carries its compliance state as an intrinsic property, verified through zero-knowledge proofs, enforced through cryptographic attestations, and settled through decentralized anchor networks. When regulatory conditions change—a license expires, a sanctions list updates, a corridor closes—the asset responds autonomously, migrating to compliant jurisdictions or halting operations as required.

The MSEZ Stack provides the operating system for this new class of assets: a complete infrastructure layer enabling trillion-dollar asset mobility across programmable jurisdictions.

---

## Architecture

The stack is organized into three layers that work together to enable Smart Asset autonomy.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SMART ASSET OPERATING SYSTEM                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  LAYER 3: NETWORK COORDINATION                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   Watcher    │  │   Security   │  │    Audit     │  │  Governance  │   │
│  │   Economy    │  │    Layer     │  │Infrastructure│  │  Framework   │   │
│  │              │  │              │  │              │  │              │   │
│  │  Bonded      │  │  Replay      │  │  Tamper-     │  │  Parameter   │   │
│  │  Attestation │  │  Prevention  │  │  Evident     │  │  Evolution   │   │
│  │  Slashing    │  │  Time Locks  │  │  Hash Chain  │  │  Consensus   │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  LAYER 2: JURISDICTIONAL INFRASTRUCTURE                                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Compliance  │  │  Migration   │  │   Corridor   │  │  L1 Anchor   │   │
│  │   Manifold   │  │   Protocol   │  │    Bridge    │  │   Network    │   │
│  │              │  │              │  │              │  │              │   │
│  │  Path        │  │  Saga-based  │  │  Two-Phase   │  │  Settlement  │   │
│  │  Planning    │  │  State       │  │  Commit      │  │  Finality    │   │
│  │  Dijkstra    │  │  Machine     │  │  Multi-Hop   │  │  Ethereum+L2 │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  LAYER 1: ASSET INTELLIGENCE                                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Compliance  │  │  ZK Proof    │  │  Smart Asset │  │  Hardening   │   │
│  │   Tensor     │  │Infrastructure│  │      VM      │  │    Layer     │   │
│  │              │  │              │  │              │  │              │   │
│  │  4D State    │  │  Groth16     │  │  256-bit     │  │  Validation  │   │
│  │  Lattice     │  │  PLONK       │  │  Stack-based │  │  Thread-safe │   │
│  │  Merkleized  │  │  STARK       │  │  Gas-metered │  │  Atomic Ops  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Layer 1: Asset Intelligence** provides the core computational substrate. The Compliance Tensor represents multi-dimensional compliance state as a sparse 4D structure indexed by (Asset, Jurisdiction, Domain, Time). The ZK Proof Infrastructure enables privacy-preserving verification without disclosing sensitive details. The Smart Asset VM provides deterministic execution with compliance and migration coprocessors.

**Layer 2: Jurisdictional Infrastructure** enables cross-border movement. The Compliance Manifold computes optimal migration paths through the jurisdictional landscape. The Migration Protocol implements saga-based state machines with compensation for failures. The Corridor Bridge orchestrates multi-hop transfers with two-phase commit. The L1 Anchor Network provides settlement finality through Ethereum and L2 checkpoints.

**Layer 3: Network Coordination** ensures economic accountability and security. The Watcher Economy requires bonded collateral for attestations with slashing for misbehavior. The Security Layer implements defense-in-depth against replay attacks, TOCTOU vulnerabilities, and front-running. The Audit Infrastructure maintains tamper-evident forensic trails with hash chain linking.

---

## Quick Start

### Prerequisites

Python 3.10 or higher is required. The stack has no external dependencies beyond the standard library and pytest for testing.

### Installation

```bash
# Clone the repository
git clone https://github.com/momentum-inc/msez-stack.git
cd msez-stack

# Install test dependencies
pip install pytest --break-system-packages

# Run the test suite (92 tests should pass)
PYTHONPATH=. pytest tests/test_phoenix.py -v
```

### Hello World: Compliance Tensor

```python
from tools.phoenix.tensor import (
    ComplianceTensorV2,
    ComplianceDomain,
    ComplianceState,
    AttestationRef,
)
from datetime import datetime, timezone, timedelta
import hashlib

# Create a compliance tensor
tensor = ComplianceTensorV2()

# Create an attestation from a licensed KYC provider
attestation = AttestationRef(
    attestation_id="att-kyc-001",
    attestation_type="kyc_verification",
    issuer_did="did:momentum:licensed-kyc-provider",
    issued_at=datetime.now(timezone.utc).isoformat(),
    expires_at=(datetime.now(timezone.utc) + timedelta(days=365)).isoformat(),
    digest=hashlib.sha256(b"kyc-evidence-bundle").hexdigest(),
)

# Set compliance state
tensor.set(
    asset_id="smart-asset-001",
    jurisdiction_id="uae-difc",
    domain=ComplianceDomain.KYC,
    state=ComplianceState.COMPLIANT,
    attestations=[attestation],
)

# Evaluate compliance
is_compliant, state, issues = tensor.evaluate("smart-asset-001", "uae-difc")
print(f"Compliant: {is_compliant}")  # True

# Generate cryptographic commitment
commitment = tensor.commit()
print(f"Root: {commitment.root[:16]}...")
```

### Hello World: Cross-Jurisdictional Migration

```python
from tools.phoenix.bridge import create_bridge_with_manifold, BridgeRequest
from decimal import Decimal

# Create bridge with UAE-DIFC and KZ-AIFC corridors
bridge = create_bridge_with_manifold()

# Request migration
request = BridgeRequest(
    bridge_id="migration-001",
    asset_id="smart-asset-001",
    asset_genesis_digest="a" * 64,
    source_jurisdiction="uae-difc",
    target_jurisdiction="kz-aifc",
    amount=Decimal("1000000"),
    currency="USD",
)

# Execute with two-phase commit
execution = bridge.execute(request)

if execution.is_successful:
    print(f"Migration completed: {len(execution.hops)} hops, ${execution.total_fees} fees")
```

### Hello World: Smart Asset VM

```python
from tools.phoenix.vm import SmartAssetVM, ExecutionContext, Assembler

# Initialize VM
vm = SmartAssetVM()

# Assemble bytecode
bytecode = Assembler.assemble([
    ('PUSH1', 42),      # Push value
    ('PUSH1', 0),       # Push storage slot
    ('SSTORE',),        # Store
    ('HALT',),          # Stop
])

# Execute
context = ExecutionContext(
    caller="did:momentum:caller",
    origin="did:momentum:origin",
    jurisdiction_id="uae-difc",
)

result = vm.execute(bytecode, context)
print(f"Success: {result.success}, Gas: {result.gas_used}")
```

---

## PHOENIX Modules

The PHOENIX module suite comprises 9,221 lines of production-grade Python across 11 modules.

### Compliance Tensor (955 lines)

`tools/phoenix/tensor.py`

The mathematical core of Smart Asset autonomy. Represents compliance state as a 4-dimensional sparse tensor `C: Asset × Jurisdiction × Domain × Time → State` with lattice algebra semantics.

Key properties include pessimistic composition where `COMPLIANT ∧ PENDING = PENDING`, fail-safe defaults where `UNKNOWN → NON_COMPLIANT`, Merkleized commitments for L1 anchoring, and selective disclosure proofs for privacy-preserving verification.

### Zero-Knowledge Proofs (766 lines)

`tools/phoenix/zkp.py`

Privacy-preserving compliance verification. Supports Groth16, PLONK, and STARK proof systems with a content-addressed circuit registry.

Standard circuits include balance sufficiency proving balance exceeds threshold without revealing amount, sanctions clearance proving non-membership in sanctions set, KYC attestation proving valid KYC from approved issuer, and compliance tensor inclusion proving specific coordinate has claimed state.

### Compliance Manifold (1,009 lines)

`tools/phoenix/manifold.py`

Path planning through the jurisdictional landscape. Models jurisdictions as nodes and corridors as edges, computing optimal migration paths using Dijkstra's algorithm with compliance-aware weights.

Features include attestation gap analysis identifying missing requirements, path cost estimation including fees and time, corridor availability checking, and multi-hop optimization.

### Migration Protocol (886 lines)

`tools/phoenix/migration.py`

Saga-based state machine for cross-jurisdictional transfers. State progression follows INITIATED → COMPLIANCE_CHECK → ATTESTATION_GATHERING → SOURCE_LOCK → TRANSIT → DESTINATION_VERIFICATION → DESTINATION_UNLOCK → COMPLETED with compensation paths for failure recovery at any stage.

### Corridor Bridge (822 lines)

`tools/phoenix/bridge.py`

Orchestrates multi-hop transfers through the two-phase commit protocol. The PREPARE phase locks assets at each hop and collects prepare receipts. The COMMIT phase executes transfers atomically and collects commit receipts. Failure at any point triggers coordinated compensation.

### L1 Anchor (816 lines)

`tools/phoenix/anchor.py`

Settlement finality through Ethereum and L2 checkpointing. Supports Ethereum mainnet with 64-block finality, Arbitrum One with 1-block finality, Base with 1-block finality, and Polygon PoS with 256-block finality. Includes cross-chain verification for defense-in-depth and Merkle inclusion proofs for receipt verification.

### Watcher Economy (750 lines)

`tools/phoenix/watcher.py`

Economic accountability through bonded attestations. Watchers stake collateral proportional to attested transaction volume. Slashing conditions include equivocation at 100% for conflicting attestations, false attestation at 50% for invalid state claims, availability failure at 1% for missed attestations, and collusion at 100% plus permanent ban for coordinated misbehavior.

### Smart Asset VM (1,285 lines)

`tools/phoenix/vm.py`

Stack-based execution environment for deterministic Smart Asset operations. Features a 256-slot stack with 256-bit words, 64KB expandable memory, Merkleized persistent storage, gas metering for DoS prevention, and pre-scanned jump destination validation.

Instruction categories include stack operations (PUSH, POP, DUP, SWAP), arithmetic (ADD, SUB, MUL, DIV, MOD), comparison (EQ, LT, GT, AND, OR), memory (MLOAD, MSTORE), storage (SLOAD, SSTORE), control flow (JUMP, JUMPI, CALL, RETURN), context (CALLER, JURISDICTION, TIMESTAMP), compliance coprocessor (TENSOR_GET, TENSOR_SET, VERIFY_ZK), migration coprocessor (LOCK, UNLOCK, TRANSIT, SETTLE), and cryptography (SHA256, VERIFY_SIG, MERKLE_VERIFY).

### Security Layer (993 lines)

`tools/phoenix/security.py`

Defense-in-depth protection addressing replay attacks through scoped attestations with nonce binding, TOCTOU vulnerabilities through versioned state with compare-and-swap, front-running through time-locked operations with 7-day withdrawal delays, and tamper detection through hash-chained audit logs.

### Hardening Layer (744 lines)

`tools/phoenix/hardening.py`

Production-grade validation and thread safety. Input validators cover strings, digests, addresses, amounts, timestamps, and bytes. Concurrency primitives include ThreadSafeDict, AtomicCounter, and atomic decorators. Economic guards enforce 10x collateral limits for attestations, minimum bond requirements, and whale concentration detection.

---

## Design Principles

Eight core principles guide the architecture.

**Fail-Safe Defaults.** Unknown compliance states default to non-compliant. Missing attestations are treated as absent. Expired credentials invalidate compliance. The system fails closed, never open.

**Cryptographic Integrity.** Every state transition produces verifiable proof. Tensor commitments are Merkle roots. Attestations are content-addressed. Receipts chain cryptographically. Nothing is trusted without verification.

**Atomic Operations.** Migrations either complete fully or compensate entirely. Two-phase commit ensures no partial states. Saga patterns handle distributed failures. The system is always consistent.

**Economic Accountability.** Watchers stake real collateral for attestations. Misbehavior is slashed automatically. Reputation affects future opportunities. Incentives align with honest behavior.

**Privacy by Design.** Zero-knowledge proofs verify without disclosure. Selective tensor slices reveal only necessary state. Range proofs hide exact amounts. Compliance is provable without transparency.

**Defense in Depth.** Multiple layers protect against each threat class. Nonces prevent replay. Versions prevent TOCTOU. Time locks prevent front-running. No single point of failure.

**Zero Trust.** All inputs are untrusted until validated. External data is sanitized. Signatures are verified. Digests are recomputed. Trust is earned, never assumed.

**Deterministic Execution.** VM operations produce identical results across all nodes. No floating point. No randomness. No external state. Consensus is achievable.

---

## Test Suite

The comprehensive test suite validates all PHOENIX components with 92 tests organized into 13 test classes.

```bash
# Run complete suite
PYTHONPATH=. pytest tests/test_phoenix.py -v

# Expected output: 92 passed in ~0.3s
```

Test coverage includes compliance tensor operations and lattice algebra, ZK proof infrastructure and circuit registry, compliance manifold path planning, migration protocol state machine, watcher economy and slashing, L1 anchoring and cross-chain verification, corridor bridge two-phase commit, hardening module validation and concurrency, security module replay prevention and time locks, Smart Asset VM execution and coprocessors, and integrated security scenarios.

---

## Repository Structure

```
momentum-sez-stack-v0.4.43/
├── tools/
│   ├── phoenix/                  # PHOENIX module suite (9,221 lines)
│   │   ├── __init__.py           # Lazy imports and exports
│   │   ├── tensor.py             # Compliance Tensor
│   │   ├── zkp.py                # ZK Proof Infrastructure
│   │   ├── manifold.py           # Compliance Manifold
│   │   ├── migration.py          # Migration Protocol
│   │   ├── bridge.py             # Corridor Bridge
│   │   ├── anchor.py             # L1 Anchor Network
│   │   ├── watcher.py            # Watcher Economy
│   │   ├── vm.py                 # Smart Asset VM
│   │   ├── security.py           # Security Layer
│   │   └── hardening.py          # Hardening Layer
│   ├── msez.py                   # CLI tool
│   ├── agentic.py                # Agentic execution framework
│   ├── regpack.py                # Regulatory pack tools
│   └── arbitration.py            # Dispute resolution
├── tests/
│   └── test_phoenix.py           # PHOENIX test suite (92 tests)
├── schemas/                      # JSON schemas (113 files)
├── docs/                         # Documentation
├── spec/                         # Specification documents
├── CHANGELOG.md                  # Release history
├── VERSION                       # Current version
└── README.md                     # This file
```

---

## Version History

| Version | Codename | Highlights |
|---------|----------|------------|
| **0.4.43** | PHOENIX ASCENSION | Smart Asset VM, Security Layer, 9,221 lines, 92 tests |
| 0.4.42 | Agentic Ascension | Agentic framework, 16 policies, 5 monitors |
| 0.4.41 | Radical Yahoo | Arbitration, RegPack, cryptographic proofs |
| 0.4.40 | — | Trade instruments, settlement netting |

---

## About Momentum

Momentum is a venture fund and studio pioneering programmable institutions—organizations that operate through cryptographic primitives across networks, continents, and markets.

We partner with founders building the rails for durable economies of the next century, with a focus on financial infrastructure, governance, identity, compliance and regulatory primitives, arbitration, settlement and property rights, and rigorous market and protocol design.

---

<div align="center">

**Built by [Momentum](https://momentum.inc)**

[Documentation](./docs/) · [Specification](./spec/) · [Examples](./docs/examples/)

---

*Smart Asset Operating System for programmable jurisdictions.*

Contact: engineering@momentum.inc

</div>
