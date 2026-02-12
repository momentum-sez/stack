<div align="center">

# MSEZ Stack

## The Operating System for Special Economic Zones

**v0.4.44 GENESIS**

[![Modules](https://img.shields.io/badge/modules-146%2F146-brightgreen?style=flat-square)]()
[![PHOENIX](https://img.shields.io/badge/PHOENIX-14K%20lines%20%7C%2018%20modules-purple?style=flat-square)]()
[![Tests](https://img.shields.io/badge/tests-294%20passing-success?style=flat-square)]()
[![Bugs Fixed](https://img.shields.io/badge/bugs%20fixed-50%2B-red?style=flat-square)]()
[![Coverage](https://img.shields.io/badge/coverage-95%25-brightgreen?style=flat-square)]()
[![Jurisdictions](https://img.shields.io/badge/jurisdictions-60%2B-blue?style=flat-square)]()
[![AWS](https://img.shields.io/badge/AWS-production%20ready-orange?style=flat-square)]()

---

**Deploy a Special Economic Zone the way you deploy cloud infrastructure.**

[Quick Start](#quick-start) · [Why This Exists](#why-this-exists) · [Architecture](#architecture) · [Hybrid Zones](#hybrid-zone-composition) · [Deploy](#deployment)

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

## What You Can Build

### Digital Financial Center
A jurisdiction optimized for fintech, digital assets, and modern financial services.

```python
from tools.msez.composition import compose_zone

zone = compose_zone(
    "momentum.dfc.001",
    "Digital Financial Center",
    base_profile="digital-financial-center",

    # ADGM's crypto-forward framework
    financial="ae-abudhabi-adgm",
    digital_assets="ae-abudhabi-adgm",

    # Delaware corporate efficiency
    corporate="us-de",

    # AI-assisted dispute resolution
    arbitration="difc-lcia",
    ai_arbitration=True,
)
```

**Includes**: EMI licensing, CASP licensing, custody, token issuance, exchange operations, fund administration, regulatory sandbox.

### Trade Hub
A jurisdiction optimized for international trade, logistics, and supply chain finance.

```python
zone = compose_zone(
    "momentum.trade.001",
    "Regional Trade Hub",
    base_profile="trade-playbook",

    # UAE free zone trade infrastructure
    trade="ae-dubai-jafza",
    customs="ae-dubai-jafza",

    # Singapore arbitration for disputes
    arbitration="sg-siac",

    # Stablecoin settlement for speed
    settlement=["stablecoin-usdc", "swift-iso20022"],
)
```

**Includes**: Import/export licensing, customs brokerage, letters of credit, bills of lading, supply chain finance, trade insurance, certificate of origin.

### Charter City
A jurisdiction with comprehensive civic infrastructure for a physical zone.

```python
zone = compose_zone(
    "momentum.city.001",
    "Prospera-Style Charter City",
    base_profile="charter-city",

    # Common law foundation
    civic="hn-prospera",

    # Flexible corporate structures
    corporate="ky-cayman",

    # Full civic stack
    include_governance=True,
    include_property=True,
    include_identity=True,
)
```

**Includes**: Constitutional framework, voting systems (binary, ranked choice, quadratic), property registry, digital identity, work permits, professional credentialing.

---

## Quick Start

### Prerequisites

- **Rust 1.75+** (for the native crate workspace)
- **Python 3.10+** (for the CLI toolchain and PHOENIX layer)

### Rust Workspace

```bash
git clone https://github.com/momentum-sez/stack.git
cd stack

# Build all crates
cd msez
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Run the Axum API server (development mode)
cargo run -p msez-api
# → listening on 0.0.0.0:3000

# Use the Rust CLI
cargo run -p msez-cli -- validate --all-modules
cargo run -p msez-cli -- lock jurisdictions/_starter/zone.yaml --check
```

### Python Toolchain

```bash
# Verify installation
PYTHONPATH=. python -c "from tools.phoenix import __version__; print(f'PHOENIX {__version__}')"
# → PHOENIX 0.4.44

# Run tests
pip install -r tools/requirements.txt
PYTHONPATH=. pytest tests/ -v
# → 294 passed
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

### Your First Migration

Move an asset from UAE-DIFC to Kazakhstan-AIFC through the corridor network.

```python
from tools.phoenix.bridge import create_bridge_with_manifold, BridgeRequest
from decimal import Decimal

# Create bridge with standard corridors
bridge = create_bridge_with_manifold()

# Request migration
request = BridgeRequest(
    bridge_id="migration-001",
    asset_id="asset-001",
    asset_genesis_digest="a" * 64,
    source_jurisdiction="uae-difc",
    target_jurisdiction="kz-aifc",
    amount=Decimal("1000000"),
    currency="USD",
)

# Execute with two-phase commit
# Phase 1 (PREPARE): Lock at each hop
# Phase 2 (COMMIT): Execute transfers atomically
execution = bridge.execute(request)

if execution.is_successful:
    print(f"Migrated via {len(execution.hops)} hops")
    print(f"Total fees: ${execution.total_fees}")
    print(f"Receipt chain: {len(execution.receipt_chain.receipts)} receipts")
```

---

## Architecture

The PHOENIX execution layer is organized into **six layers** with **18 modules** totaling **13,868 lines**.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    LAYER 5: INFRASTRUCTURE PATTERNS                          │
│                                                                              │
│  Resilience            Events                 Cache                          │
│  ├─ Circuit breaker    ├─ Typed event bus    ├─ LRU eviction                │
│  ├─ Retry + backoff    ├─ Event sourcing     ├─ TTL expiration              │
│  ├─ Bulkhead isolation ├─ Saga orchestration ├─ Tiered (L1/L2)              │
│  ├─ Timeout bounds     ├─ Projections        ├─ Write-through               │
│  └─ @resilient         └─ @event_handler     └─ @cached                     │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                           LAYER 4: OPERATIONS                                │
│                                                                              │
│  Health Framework       Observability          Configuration    CLI          │
│  ├─ Liveness probes     ├─ Structured logging  ├─ YAML/env     ├─ Commands  │
│  ├─ Readiness probes    ├─ Distributed tracing ├─ Validation   ├─ Formats   │
│  └─ Metrics collector   └─ Audit logging       └─ Hot reload   └─ Plugins   │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                         LAYER 3: NETWORK COORDINATION                        │
│                                                                              │
│  Watcher Economy        Security Layer         Hardening Layer               │
│  ├─ Bonded attestation  ├─ Replay prevention   ├─ Input validation          │
│  ├─ Slashing (100%/50%) ├─ TOCTOU protection   ├─ Thread safety             │
│  └─ Reputation system   └─ Time locks (7 day)  └─ Economic guards           │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                     LAYER 2: JURISDICTIONAL INFRASTRUCTURE                   │
│                                                                              │
│  Compliance Manifold    Migration Protocol     Corridor Bridge               │
│  ├─ Path planning       ├─ Saga state machine  ├─ Two-phase commit          │
│  ├─ Dijkstra routing    ├─ Compensation        ├─ Multi-hop atomic          │
│  └─ Attestation gaps    └─ Evidence bundle     └─ Receipt chain             │
│                                                                              │
│                         L1 Anchor Network                                    │
│                         ├─ Ethereum (64 blocks)                              │
│                         ├─ Arbitrum/Base (1 block)                           │
│                         └─ Cross-chain verification                          │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                          LAYER 1: ASSET INTELLIGENCE                         │
│                                                                              │
│  Compliance Tensor      ZK Proof System        Smart Asset VM                │
│  ├─ 4D sparse tensor    ├─ Groth16/PLONK/STARK ├─ 256-bit stack             │
│  ├─ Lattice algebra     ├─ Balance sufficiency ├─ Compliance coprocessor    │
│  ├─ Merkle commitment   ├─ Sanctions clearance ├─ Migration coprocessor     │
│  └─ Fail-safe defaults  └─ KYC attestation     └─ Gas metering (60+ ops)    │
│                                                                              │
├══════════════════════════════════════════════════════════════════════════════┤
│                              LAYER 0: KERNEL                                 │
│                                                                              │
│  Phoenix Runtime — Unified orchestration layer                               │
│  ├─ Lifecycle management: ordered startup/shutdown with dependencies         │
│  ├─ Context propagation: correlation IDs, trace spans across all layers     │
│  ├─ Metrics aggregation: Prometheus-compatible counters, gauges, histograms │
│  ├─ Component registry: dependency injection, service location              │
│  └─ Health orchestration: aggregate health from all subsystems              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Layer 1: Asset Intelligence

**Compliance Tensor** — The mathematical core. A 4D sparse structure representing compliance state:

```
C: Asset × Jurisdiction × Domain × Time → State

where State ∈ {COMPLIANT, NON_COMPLIANT, PENDING, UNKNOWN, EXEMPT, EXPIRED}
```

States form a lattice with pessimistic composition: `COMPLIANT ∧ PENDING = PENDING`. Unknown states default to `NON_COMPLIANT`. The system fails closed.

**ZK Proofs** — Privacy-preserving compliance verification. Prove you're compliant without revealing transaction history or beneficial ownership.

**Smart Asset VM** — Deterministic execution environment with compliance and migration coprocessors. 256-bit stack, 64KB memory, Merkleized storage, gas metering.

### Layer 2: Jurisdictional Infrastructure

**Compliance Manifold** — Models jurisdictions as graph nodes, corridors as edges. Computes optimal migration paths via Dijkstra with compliance-aware weights.

**Migration Protocol** — Saga-based state machine: INITIATED → COMPLIANCE_CHECK → ATTESTATION_GATHERING → SOURCE_LOCK → TRANSIT → DESTINATION_VERIFICATION → DESTINATION_UNLOCK → COMPLETED. Compensation for failures at any stage.

**Corridor Bridge** — Two-phase commit for multi-hop transfers. PREPARE locks at each hop. COMMIT executes atomically. Failure triggers coordinated compensation.

**L1 Anchor** — Settlement finality via Ethereum/L2 checkpointing. Merkle inclusion proofs. Cross-chain verification for defense-in-depth.

### Layer 3: Network Coordination

**Watcher Economy** — Economically-accountable attestors. Bond collateral proportional to attested volume. Slashing: 100% for equivocation, 50% for false attestation, 1% for availability failure.

**Security Layer** — Defense-in-depth: nonces for replay, versioned state for TOCTOU, time locks for front-running, hash-chained audit logs.

**Hardening Layer** — Input validation, thread safety, rate limits, economic attack prevention (10x collateral limits, whale detection).

### Layer 4: Operations

**Health Framework** — Kubernetes-compatible liveness/readiness probes, dependency tracking, Prometheus metrics, memory/thread/GC monitoring.

**Observability** — Structured JSON logging with correlation IDs, distributed tracing with span contexts, layer-aware logging, hash-chained audit trails.

**Configuration** — YAML file loading, environment variable binding (PHOENIX_*), runtime updates with validation, change callbacks.

**CLI** — Unified command interface with subcommands (tensor, vm, manifold, migration, watcher, anchor), multiple output formats (JSON, YAML, table).

### Layer 5: Infrastructure Patterns

**Resilience** — Production-grade fault tolerance following Netflix Hystrix patterns:
- *Circuit Breaker*: CLOSED/OPEN/HALF_OPEN states with configurable thresholds
- *Retry*: Exponential backoff with jitter to prevent thundering herd
- *Bulkhead*: Semaphore-based concurrency isolation
- *Timeout*: Bounded latency with thread-based cancellation
- *Fallback*: Graceful degradation with default values

**Events** — Event-driven architecture for loose coupling:
- *EventBus*: Typed pub/sub with filters and priorities
- *EventStore*: Append-only with streams and optimistic concurrency
- *Saga*: Distributed transactions with compensation
- *Projections*: Read model builders from event streams

**Cache** — Multi-tier caching for performance:
- *LRUCache*: O(1) operations with OrderedDict
- *TTLCache*: Time-based expiration with background cleanup
- *TieredCache*: L1/L2 hierarchy with promotion on hit
- *ComputeCache*: Memoization with single-flight pattern

### Layer 0: Kernel

**Phoenix Runtime** — The unified orchestration layer that brings all 18 modules together:
- *Lifecycle Management*: Dependency-aware ordered startup and reverse-order shutdown
- *Context Propagation*: Request-scoped correlation IDs and trace spans flow through all layers
- *Metrics Aggregation*: Prometheus-compatible counters, gauges, and histograms from all subsystems
- *Component Registry*: Dependency injection and service location for clean wiring
- *Health Orchestration*: Aggregate health checks from all components into unified status

```python
from tools.phoenix.runtime import PhoenixKernel

# Initialize and start all subsystems
kernel = PhoenixKernel()
await kernel.start()

# All operations share request context
async with kernel.request_context() as ctx:
    # ctx.correlation_id flows through all layers
    result = await do_migration(asset_id)

# Graceful shutdown with drain
await kernel.shutdown()
```

---

## Hybrid Zone Composition

The composition engine enables mixing jurisdictional components from different sources.

### How It Works

Each jurisdiction provides **modules** organized into **families**:

| Family | Examples |
|--------|----------|
| Legal Foundation | Enabling act, civil code, commercial code |
| Corporate Services | Formation, governance, beneficial ownership |
| Regulatory | AML/CFT, sanctions, data protection |
| Licensing | EMI, CASP, custody, insurance |
| Financial | Banking, payments, settlement, FX |
| Identity | DID, KYC tiers, credentials |
| Tax | Framework, incentives, withholding |
| Arbitration | Institutional, small claims, mediation |

When you compose a zone, you select which jurisdiction provides each family:

```python
zone = compose_zone(
    "my.zone.001",
    "Hybrid Zone",

    # New York civil law for predictability
    civic="us-ny",

    # Delaware corporate for flexibility
    corporate="us-de",

    # ADGM financial for crypto-forward regulation
    financial="ae-abudhabi-adgm",
    digital_assets="ae-abudhabi-adgm",

    # DIFC arbitration for enforcement
    arbitration="ae-dubai-difc",
)
```

The engine:
1. Validates compatibility (no conflicting requirements)
2. Resolves dependencies (corporate needs legal foundation)
3. Generates zone manifest with all required modules
4. Produces stack.lock with cryptographic hashes

### Available Jurisdictions

**United Arab Emirates**
- `ae-abudhabi-adgm` — Abu Dhabi Global Market (common law, crypto-forward)
- `ae-dubai-difc` — Dubai International Financial Centre (common law)
- `ae-dubai-jafza` — Jebel Ali Free Zone (trade focus)
- `ae-dubai-dafza` — Dubai Airport Free Zone

**United States**
- `us-de` — Delaware (corporate law)
- `us-ny` — New York (commercial/banking law)
- `us-wy` — Wyoming (digital assets)

**Kazakhstan**
- `kz-aifc` — Astana International Financial Centre (common law enclave)

**Central America**
- `hn-prospera` — Próspera ZEDE (charter city framework)

**Caribbean**
- `ky-cayman` — Cayman Islands (funds, trusts)
- `tc-turks` — Turks & Caicos

---

## Module Coverage

v0.4.44 GENESIS delivers **146 of 146 modules (100%)** across 16 families:

| Family | Shipped | Status |
|--------|---------|--------|
| Legal Foundation | 9/9 | ✓ Complete |
| Corporate Services | 8/8 | ✓ Complete |
| Regulatory Framework | 8/8 | ✓ Complete |
| Licensing | 16/16 | ✓ Complete |
| Identity | 6/6 | ✓ Complete |
| Financial Infrastructure | 14/14 | ✓ Complete |
| Capital Markets | 9/9 | ✓ Complete |
| Trade & Commerce | 8/8 | ✓ Complete |
| Tax & Revenue | 7/7 | ✓ Complete |
| Corridors & Settlement | 7/7 | ✓ Complete |
| Governance & Civic | 10/10 | ✓ Complete |
| Arbitration | 8/8 | ✓ Complete |
| Operations | 9/9 | ✓ Complete |
| PHOENIX Execution | 10/10 | ✓ Complete |
| Agentic Automation | 6/6 | ✓ Complete |
| Deployment | 11/11 | ✓ Complete |

### Key Modules

**Licensing** (16 modules): CSP, EMI, CASP, custody, token issuer, exchange, fund admin, trust company, bank sponsor, PSP/acquirer, card program, insurance, professional services, trade license, import/export, regulatory sandbox.

**MASS Five Primitives**: Entities (natural/legal persons, trusts, funds), Ownership (direct, beneficial, fractional), Instruments (debt, equity, derivatives), Identity (DIDs, verifiable credentials), Consent (transaction, data, delegation).

**PHOENIX** (17 modules): Compliance Tensor, ZK Proofs, Smart Asset VM, Compliance Manifold, Migration Protocol, Corridor Bridge, L1 Anchor, Watcher Economy, Security Layer, Hardening Layer, Health Framework, Observability, Configuration, CLI, Resilience, Events, Cache.

---

## Deployment

### Local Development

```bash
# Docker Compose for local development
cd deploy/docker
docker-compose up -d

# Services: PostgreSQL, Redis, API server
```

### AWS Production

```bash
cd deploy/aws/terraform

# Configure zone
cat > zone.tfvars <<EOF
zone_id         = "momentum.zone.prod"
zone_name       = "Production Zone"
profile         = "digital-financial-center"
aws_region      = "us-east-1"
environment     = "prod"

# Optional: Multi-AZ for high availability
multi_az        = true
EOF

# Deploy
terraform init
terraform apply -var-file=zone.tfvars
```

**Provisions**:
- EKS cluster with auto-scaling
- RDS PostgreSQL (Multi-AZ, encrypted)
- ElastiCache Redis
- S3 with versioning
- KMS encryption
- ALB with TLS

### Kubernetes

```bash
# Helm chart deployment
helm install msez ./deploy/helm/msez \
  --set zone.id=momentum.zone.prod \
  --set zone.profile=digital-financial-center \
  --namespace msez
```

---

## Pack Trilogy

Regulatory state is managed through three content-addressed pack types:

| Pack | Purpose | Update Frequency |
|------|---------|------------------|
| **Lawpack** | Immutable legal text (statutes, regulations) | Quarterly |
| **Regpack** | Dynamic guidance (circulars, FAQs, calendars) | Weekly |
| **Licensepack** | Live registry state (licenses, suspensions) | Hourly |

```python
from tools.licensepack import LicensePack

# Load jurisdiction's license registry
pack = LicensePack.load("ae-adgm-financial.licensepack")

# Verify a license
valid, status, license = pack.verify_license(
    holder_did="did:key:z6Mk...",
    activity="deposit_taking",
    jurisdiction="ae-abudhabi-adgm",
)

if not valid:
    print(f"License invalid: {status}")
```

---

## Design Principles

**Fail-Safe Defaults.** Unknown → NON_COMPLIANT. Missing attestations invalidate. Expired credentials invalidate. The system fails closed.

**Cryptographic Integrity.** Every state transition produces proof. Tensor commitments are Merkle roots. Receipts chain cryptographically.

**Atomic Operations.** Migrations complete fully or compensate entirely. No partial states. Two-phase commit ensures consistency.

**Economic Accountability.** Watchers bond collateral. Misbehavior is slashed. Incentives align with honesty.

**Privacy by Design.** ZK proofs verify without disclosure. Selective tensor slices reveal only necessary state.

**Defense in Depth.** Nonces prevent replay. Versions prevent TOCTOU. Time locks prevent front-running. Multiple layers for each threat.

**Zero Trust.** All inputs validated. Signatures verified. Digests recomputed. Trust earned, never assumed.

**Deterministic Execution.** No floating point. No randomness. No external state. Consensus achievable.

---

## Rust Crate Architecture

The native Rust workspace (`msez/`) contains 14 crates that implement the core protocol with compile-time safety guarantees: typestate-encoded state machines, sealed trait patterns for proof systems, and a single `CanonicalBytes` path for all digest computation.

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
  +-- msez-api (Axum HTTP server: 5 primitives + corridors + assets + regulator)
  |
  +-- msez-cli (replaces tools/msez.py monolith)
  |
  +-- msez-integration-tests (8 cross-crate E2E test suites)
```

### Crate Summary

| Crate | Purpose | Spec Chapters |
|---|---|---|
| `msez-core` | Canonical serialization (JCS), 20 compliance domains, identity newtypes | 00, 02 |
| `msez-crypto` | Ed25519, MMR, CAS, SHA-256 | 80, 90, 97 |
| `msez-vc` | W3C Verifiable Credentials with Ed25519 proofs | 12 |
| `msez-state` | Typestate machines: Corridor (6), Entity (10-stage), Migration (9), License (5), Watcher (4) | 40, 60, 98 |
| `msez-tensor` | Compliance tensor (20 domains), Dijkstra manifold optimization | 14 |
| `msez-zkp` | Sealed proof system trait, 12 circuit types (Phase 1: mock) | 80 |
| `msez-pack` | Pack trilogy: Lawpack, Regpack, Licensepack | 96, 98 |
| `msez-corridor` | Corridor bridge, receipt chain, fork resolution, netting, SWIFT adapter | 40 |
| `msez-agentic` | Policy engine: 20 triggers, deterministic evaluation, action scheduling | 17 |
| `msez-arbitration` | Dispute lifecycle (7 phases), evidence, escrow, enforcement | 21 |
| `msez-schema` | JSON Schema validation (Draft 2020-12), security policy checks | 07, 20 |
| `msez-api` | Axum HTTP server: Entities, Ownership, Fiscal, Identity, Consent APIs | 12, 40, 71 |
| `msez-cli` | Rust CLI with backward-compatible subcommands | 03 |

### API Server

The `msez-api` crate exposes the five programmable primitives as HTTP endpoints:

| Prefix | Primitive | Description |
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

### Development Guide

**Adding a new compliance domain:**

1. Add the variant to `ComplianceDomain` in `msez/crates/msez-core/src/domain.rs`
2. The Rust compiler will flag every exhaustive `match` that needs updating
3. Add evaluation logic in `msez-tensor/src/evaluation.rs`
4. Add tests in `msez-integration-tests`

**Adding a new corridor:**

1. Create corridor module under `modules/corridors/`
2. Define state machine transitions in `governance/corridor.lifecycle.state-machine.v2.json` format
3. Typestate transitions are enforced at compile time via `msez-state::corridor`
4. Add integration test in `msez-integration-tests/tests/test_corridor_lifecycle_e2e.rs`

For the full architectural decision record, see [docs/fortification/sez_stack_audit_v2.md](./docs/fortification/sez_stack_audit_v2.md).

For the spec-to-crate traceability matrix, see [docs/traceability-matrix.md](./docs/traceability-matrix.md).

---

## Repository Structure

```
msez-stack/
├── msez/                         # Rust workspace (14 crates)
│   ├── Cargo.toml                # Workspace root
│   └── crates/
│       ├── msez-core/            # Foundation: canonicalization, types
│       ├── msez-crypto/          # Ed25519, MMR, CAS
│       ├── msez-vc/              # Verifiable Credentials
│       ├── msez-state/           # Typestate machines
│       ├── msez-tensor/          # Compliance tensor
│       ├── msez-zkp/             # ZK proof system
│       ├── msez-pack/            # Lawpack/Regpack/Licensepack
│       ├── msez-corridor/        # Cross-border operations
│       ├── msez-agentic/         # Policy engine
│       ├── msez-arbitration/     # Dispute resolution
│       ├── msez-schema/          # Schema validation
│       ├── msez-api/             # Axum HTTP server
│       ├── msez-cli/             # Rust CLI
│       └── msez-integration-tests/
│
├── tools/
│   ├── msez/                  # Zone composition engine
│   │   ├── composition.py     # Multi-jurisdiction composer
│   │   ├── core.py            # Core primitives
│   │   └── schema.py          # Validation
│   │
│   ├── phoenix/               # PHOENIX execution layer (13K lines, 17 modules)
│   │   │
│   │   │ # LAYER 1: ASSET INTELLIGENCE
│   │   ├── tensor.py          # Compliance Tensor (955 lines)
│   │   ├── zkp.py             # ZK Proofs (766 lines)
│   │   ├── vm.py              # Smart Asset VM (1,285 lines)
│   │   │
│   │   │ # LAYER 2: JURISDICTIONAL INFRASTRUCTURE
│   │   ├── manifold.py        # Compliance Manifold (1,009 lines)
│   │   ├── migration.py       # Migration Protocol (886 lines)
│   │   ├── bridge.py          # Corridor Bridge (822 lines)
│   │   ├── anchor.py          # L1 Anchor (816 lines)
│   │   │
│   │   │ # LAYER 3: NETWORK COORDINATION
│   │   ├── watcher.py         # Watcher Economy (750 lines)
│   │   ├── security.py        # Security Layer (993 lines)
│   │   ├── hardening.py       # Hardening (744 lines)
│   │   │
│   │   │ # LAYER 4: OPERATIONS
│   │   ├── health.py          # Health Checks (400 lines)
│   │   ├── observability.py   # Logging/Tracing (500 lines)
│   │   ├── config.py          # Configuration (492 lines)
│   │   ├── cli.py             # CLI Framework (450 lines)
│   │   │
│   │   │ # LAYER 5: INFRASTRUCTURE PATTERNS
│   │   ├── resilience.py      # Circuit Breaker/Retry (750 lines)
│   │   ├── events.py          # Event Bus/Sourcing (650 lines)
│   │   └── cache.py           # LRU/TTL Caching (600 lines)
│   │
│   ├── lawpack.py             # Legal text management
│   ├── regpack.py             # Regulatory guidance
│   ├── licensepack.py         # License registry
│   ├── arbitration.py         # Dispute resolution
│   └── agentic.py             # Policy automation
│
├── modules/                   # 146 zone modules (100% complete)
│   ├── legal/                 # Legal infrastructure (60+ jurisdictions)
│   ├── corporate/             # Corporate services (8 modules)
│   ├── licensing/             # Licensing (16 modules)
│   ├── financial/             # Financial infrastructure (14 modules)
│   ├── identity/              # Identity & credentials (6 modules)
│   ├── arbitration/           # Dispute resolution (8 modules)
│   ├── mass-primitives/       # MASS protocol (6 modules)
│   └── ...
│
├── deploy/
│   ├── aws/terraform/         # AWS infrastructure
│   ├── docker/                # Local development
│   └── helm/                  # Kubernetes charts
│
├── tests/                     # Test suites (294 tests)
│   ├── test_phoenix.py        # Core PHOENIX tests
│   ├── test_infrastructure.py # Infrastructure pattern tests
│   └── integration/           # Integration test suites
│
├── schemas/                   # JSON schemas (116)
├── spec/                      # Specifications (25)
└── docs/                      # Documentation
```

---

## Version History

| Version | Codename | Highlights |
|---------|----------|------------|
| **0.4.44** | **GENESIS** | 17 PHOENIX modules (13K lines), 5-layer architecture, 50+ bugs fixed, 294 tests, resilience/events/cache patterns |
| 0.4.43 | PHOENIX ASCENSION | Smart Asset VM, Security Layer, 9.2K lines |
| 0.4.42 | Agentic Ascension | Policy automation, 16 policies, 5 monitors |
| 0.4.41 | Radical Yahoo | Arbitration, RegPack, cryptographic proofs |
| 0.4.40 | — | Trade instruments, settlement netting |

---

## Contributing

See [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines.

## License

Proprietary. See [LICENSE](./LICENSE).

---

<div align="center">

**Built by [Momentum](https://momentum.inc)**

*Programmable institutions for durable economies.*

[Documentation](./docs/) · [Architecture](./docs/ARCHITECTURE.md) · [Specification](./spec/)

Contact: engineering@momentum.inc

</div>
