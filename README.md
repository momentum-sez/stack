<div align="center">

# MSEZ Stack

## The Operating System for Special Economic Zones

**v0.4.44 GENESIS**

[![CI](https://img.shields.io/badge/CI-passing-brightgreen?style=flat-square)]()
[![Modules](https://img.shields.io/badge/modules-146%2F146-brightgreen?style=flat-square)]()
[![Rust Tests](https://img.shields.io/badge/rust_tests-2%2C580+-brightgreen?style=flat-square)]()
[![Python Tests](https://img.shields.io/badge/python_tests-294-brightgreen?style=flat-square)]()
[![Coverage](https://img.shields.io/badge/coverage-98%25-brightgreen?style=flat-square)]()
[![Crates](https://img.shields.io/badge/rust_crates-14-blue?style=flat-square)]()
[![Rust LOC](https://img.shields.io/badge/rust-70K_lines-blue?style=flat-square)]()

---

**Deploy a Special Economic Zone the way you deploy cloud infrastructure.**

[Quick Start](#quick-start) · [Architecture](#architecture) · [Rust Workspace](#rust-crate-architecture) · [Deployment](#deployment) · [Documentation](./docs/)

</div>

---

## Why This Exists

Setting up a Special Economic Zone takes **3-7 years** and **$50-200M**.

You need bilateral treaties, regulatory frameworks, banking relationships, corporate registries, dispute resolution, tax treaties, customs procedures, and licensing regimes. Each component requires lawyers, regulators, and months of negotiation.

The MSEZ Stack reduces this to **configuration files**.

```yaml
# zone.yaml — A complete zone definition
zone_id: momentum.zone.nyc-de-adgm
name: "NYC-Delaware-ADGM Hybrid"

jurisdictions:
  civic: us-ny           # New York civil law
  corporate: us-de       # Delaware corporations
  financial: ae-adgm     # ADGM financial services
  digital: ae-adgm       # ADGM digital assets

corridors:
  - swift-iso20022       # Traditional banking
  - stablecoin-usdc      # Crypto settlement

arbitration:
  primary: difc-lcia     # DIFC-LCIA Rules
  ai_enabled: true       # AI-assisted discovery
```

This configuration generates:
- **Legal infrastructure**: Entity registry, land registry, security interests
- **Regulatory framework**: AML/CFT, sanctions screening, data protection
- **Financial rails**: Banking adapters, payment processing, settlement
- **Corporate services**: Formation, governance, beneficial ownership
- **Dispute resolution**: Arbitration, mediation, enforcement

---

## Quick Start

### Prerequisites

- **Rust 1.75+** (for the native crate workspace)
- **Python 3.10+** (for the CLI toolchain and PHOENIX layer)

### Rust Workspace (Primary)

```bash
git clone https://github.com/momentum-sez/stack.git
cd stack/msez

# Build all 14 crates
cargo build --workspace

# Run the full test suite (2,580+ tests)
cargo test --workspace

# Lint with zero warnings
cargo clippy --workspace -- -D warnings

# Generate API documentation
cargo doc --workspace --no-deps --open

# Run the Axum API server (development mode)
cargo run -p msez-api
# listening on 0.0.0.0:3000 — OpenAPI spec at /openapi.json

# Use the Rust CLI
cargo run -p msez-cli -- validate --all-modules
cargo run -p msez-cli -- lock jurisdictions/_starter/zone.yaml --check
```

### Python Toolchain

```bash
cd stack
pip install -r tools/requirements.txt

# Validate the full module set
python -m tools.msez validate --all-modules
python -m tools.msez validate --all-profiles
python -m tools.msez validate --all-zones

# Run the Python test suite (294 tests)
pytest tests/ -q
```

### Your First Smart Asset

A **Smart Asset** is an asset with embedded compliance intelligence. It knows whether it's compliant in any jurisdiction, can identify missing attestations, and can migrate autonomously.

```python
from tools.phoenix.tensor import (
    ComplianceTensorV2,
    ComplianceDomain,
    ComplianceState,
    AttestationRef,
)
from datetime import datetime, timezone, timedelta
import hashlib

# Create compliance tensor — the asset's compliance state across all jurisdictions
tensor = ComplianceTensorV2()

# Create an attestation from a licensed KYC provider
kyc_attestation = AttestationRef(
    attestation_id="att-kyc-001",
    attestation_type="kyc_verification",
    issuer_did="did:momentum:kyc-provider-licensed-adgm",
    issued_at=datetime.now(timezone.utc).isoformat(),
    expires_at=(datetime.now(timezone.utc) + timedelta(days=365)).isoformat(),
    digest=hashlib.sha256(b"kyc-evidence").hexdigest(),
)

# Set compliance state for UAE-ADGM jurisdiction
tensor.set(
    asset_id="asset-001",
    jurisdiction_id="ae-abudhabi-adgm",
    domain=ComplianceDomain.KYC,
    state=ComplianceState.COMPLIANT,
    attestations=[kyc_attestation],
)

# Evaluate: Is this asset compliant in ADGM?
is_compliant, state, issues = tensor.evaluate("asset-001", "ae-abudhabi-adgm")
print(f"Compliant: {is_compliant}")  # True

# Generate cryptographic commitment (Merkle root)
commitment = tensor.commit()
print(f"Tensor root: {commitment.root[:16]}...")  # Anchors to L1
```

---

## Architecture

The system is built in two layers: a **Rust core** providing compile-time safety guarantees and cryptographic correctness, and a **Python PHOENIX layer** implementing the Smart Asset Operating System.

### Rust Crate Architecture

14 crates, 70K lines of Rust. The workspace enforces correctness at the type level:

- **Typestate machines** prevent invalid state transitions at compile time
- **Sealed traits** ensure only authorized proof systems can generate proofs
- **`CanonicalBytes`** is the single path for all digest computation, eliminating serialization divergence
- **`ContentDigest`** carries algorithm tags for forward migration from SHA-256 to Poseidon2

```
msez-core (foundation)
  |
  +-- msez-crypto -----> msez-vc -----> msez-schema
  |                  |
  |                  +-> msez-zkp
  |                  |
  |                  +-> msez-tensor
  |
  +-- msez-state -----> msez-corridor
  |                  |
  |                  +-> msez-arbitration
  |
  +-- msez-pack         msez-agentic
  |
  +-- msez-api (Axum HTTP: 5 primitives + corridors + assets + regulator)
  |
  +-- msez-cli (validates modules, profiles, zones, lockfiles)
  |
  +-- msez-integration-tests (97 cross-crate test suites)
```

| Crate | Purpose | Lines | Tests |
|---|---|---|---|
| `msez-core` | Canonical serialization (JCS), 20 compliance domains, identity newtypes | ~1,700 | 109 |
| `msez-crypto` | Ed25519, MMR, CAS, SHA-256 digest pipeline | ~1,400 | 158 |
| `msez-vc` | W3C Verifiable Credentials with Ed25519 proofs | ~800 | 136 |
| `msez-state` | Typestate machines: Corridor (6), Entity (10), Migration (9), License (5), Watcher (4) | ~1,800 | 131 |
| `msez-tensor` | Compliance tensor (20 domains), Dijkstra manifold, Merkle commitments | ~1,700 | 84 |
| `msez-zkp` | Sealed proof system trait, 12 circuit types (Phase 1: mock backend) | ~1,000 | 36 |
| `msez-pack` | Pack trilogy: Lawpack, Regpack, Licensepack | ~1,000 | 30 |
| `msez-corridor` | Bridge routing, receipt chain (MMR), fork resolution, netting, SWIFT | ~3,000 | 146 |
| `msez-agentic` | Policy engine: 20 triggers, evaluation, scheduling, audit trail | ~2,000 | 165 |
| `msez-arbitration` | Dispute lifecycle (7 phases), evidence, escrow, enforcement | ~1,500 | 21 |
| `msez-schema` | JSON Schema validation (Draft 2020-12), security policy checks | ~700 | 4 |
| `msez-api` | Axum HTTP server: Entities, Ownership, Fiscal, Identity, Consent | ~2,500 | 274 |
| `msez-cli` | Rust CLI with backward-compatible subcommands | ~1,200 | 5 |
| `msez-integration-tests` | 97 cross-crate end-to-end test files | ~8,000 | 282 |

### PHOENIX Execution Layer (18 modules, 14K lines)

```
LAYER 5: INFRASTRUCTURE           LAYER 4: OPERATIONS
  Resilience (circuit breaker)      Health (K8s probes)
  Events (bus + sourcing)           Observability (logging + tracing)
  Cache (LRU + TTL + tiered)        Configuration (YAML + env)
                                    CLI (commands + formats)

LAYER 3: NETWORK COORDINATION     LAYER 2: JURISDICTIONAL INFRA
  Watcher Economy (bonds)           Compliance Manifold (Dijkstra)
  Security (nonces + timelocks)     Migration Protocol (saga)
  Hardening (validation)            Corridor Bridge (2PC)
                                    L1 Anchor (Ethereum/L2)

LAYER 1: ASSET INTELLIGENCE       LAYER 0: KERNEL
  Compliance Tensor (4D sparse)     Phoenix Runtime (orchestration)
  ZK Proofs (Groth16/PLONK/STARK)   Lifecycle management
  Smart Asset VM (60+ opcodes)       Context propagation
```

### API Server

The `msez-api` crate exposes the five programmable primitives as HTTP endpoints:

| Prefix | Primitive | Endpoints |
|---|---|---|
| `/v1/entities/*` | ENTITIES | Organization lifecycle, beneficial ownership |
| `/v1/ownership/*` | OWNERSHIP | Cap table, transfers, share classes |
| `/v1/fiscal/*` | FISCAL | Treasury, payments, withholding, tax reporting |
| `/v1/identity/*` | IDENTITY | KYC/KYB, identity linking, attestations |
| `/v1/consent/*` | CONSENT | Multi-party consent, signing, audit trail |
| `/v1/corridors/*` | Corridors | State channel, receipts, fork resolution |
| `/v1/assets/*` | Smart Assets | Registry, compliance eval, anchor verify |
| `/v1/regulator/*` | Regulator | Query access, compliance reports |

OpenAPI spec auto-generated at `/openapi.json`.

---

## Hybrid Zone Composition

The composition engine enables mixing jurisdictional components from different sources. Each jurisdiction provides modules organized into families (Legal, Corporate, Regulatory, Licensing, Financial, Identity, Tax, Arbitration). When you compose a zone, you select which jurisdiction provides each family:

```python
from tools.msez.composition import compose_zone

zone = compose_zone(
    "my.zone.001",
    "Hybrid Zone",
    civic="us-ny",
    corporate="us-de",
    financial="ae-abudhabi-adgm",
    digital_assets="ae-abudhabi-adgm",
    arbitration="ae-dubai-difc",
)
```

### Available Jurisdictions

| Region | Jurisdictions |
|--------|--------------|
| **UAE** | `ae-abudhabi-adgm`, `ae-dubai-difc`, `ae-dubai-jafza`, `ae-dubai-dafza` |
| **USA** | `us-de` (corporate), `us-ny` (commercial), `us-wy` (digital assets) |
| **Kazakhstan** | `kz-aifc` (common law enclave) |
| **Central America** | `hn-prospera` (charter city) |
| **Caribbean** | `ky-cayman`, `tc-turks` |

---

## Module Coverage

v0.4.44 delivers **146/146 modules (100%)** across 16 families. See [Module Index](./modules/index.yaml) for the complete list.

---

## Deployment

| Target | Command |
|--------|---------|
| **Docker** | `cd deploy/docker && docker-compose up -d` |
| **Kubernetes** | `kubectl apply -f deploy/k8s/` |
| **AWS** | `cd deploy/aws/terraform && terraform apply -var-file=zone.tfvars` |

AWS provisions: EKS (auto-scaling), RDS PostgreSQL (Multi-AZ), ElastiCache Redis, S3, KMS, ALB with TLS.

---

## Design Principles

| Principle | Implementation |
|-----------|---------------|
| **Fail-Safe Defaults** | Unknown = NON_COMPLIANT. Missing attestations invalidate. System fails closed. |
| **Cryptographic Integrity** | Every state transition produces proof. Tensor commitments are Merkle roots. |
| **Atomic Operations** | Migrations complete fully or compensate entirely. Two-phase commit. |
| **Economic Accountability** | Watchers bond collateral. Misbehavior is slashed. |
| **Privacy by Design** | ZK proofs verify without disclosure. Selective tensor slices. |
| **Defense in Depth** | Nonces (replay), versions (TOCTOU), time locks (front-running). |
| **Zero Trust** | All inputs validated. Signatures verified. Digests recomputed. |
| **Deterministic Execution** | No floating point. No randomness. No external state. |

---

## Repository Structure

```
msez-stack/
├── msez/                         # Rust workspace (14 crates, 70K lines)
│   └── crates/                   # core, crypto, vc, state, tensor, zkp,
│                                 # pack, corridor, agentic, arbitration,
│                                 # schema, api, cli, integration-tests
├── tools/                        # Python toolchain
│   ├── msez.py                   # Reference CLI (15K lines)
│   ├── msez/                     # Composition engine
│   └── phoenix/                  # PHOENIX execution layer (14K lines)
├── modules/                      # 146 zone modules across 16 families
├── schemas/                      # 116 JSON schemas (Draft 2020-12)
├── spec/                         # 25 specification chapters
├── tests/                        # 294 Python tests
├── apis/                         # OpenAPI 3.x specs
├── deploy/                       # Docker, Terraform, Kubernetes
├── governance/                   # State machine definitions
└── docs/                         # Architecture, operators, authoring
```

---

## Key References

| Document | Description |
|----------|-------------|
| [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) | PHOENIX Smart Asset OS deep dive |
| [docs/ERRORS.md](./docs/ERRORS.md) | Error taxonomy (RFC 7807) |
| [docs/traceability-matrix.md](./docs/traceability-matrix.md) | Spec chapter to Rust crate mapping |
| [docs/fortification/sez_stack_audit_v2.md](./docs/fortification/sez_stack_audit_v2.md) | Security audit findings |
| [spec/](./spec/) | 25 specification chapters (normative) |

---

## Version History

| Version | Codename | Highlights |
|---------|----------|------------|
| **0.4.44** | **GENESIS** | 14 Rust crates (70K lines), 2,580+ tests at 98% coverage, Axum HTTP API, typestate machines, 50+ bugs fixed |
| 0.4.43 | PHOENIX ASCENSION | Smart Asset VM, Security Layer, 9.2K lines |
| 0.4.42 | Agentic Ascension | Policy automation, 16 policies, 5 monitors |
| 0.4.41 | Radical Yahoo | Arbitration, RegPack, cryptographic proofs |
| 0.4.40 | --- | Trade instruments, settlement netting |

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md).

## License

Proprietary. See [LICENSE](./LICENSE).

---

<div align="center">

**Built by [Momentum](https://momentum.inc)**

*Programmable institutions for durable economies.*

</div>
