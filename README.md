<div align="center">

# MSEZ Stack

## The Operating System for Special Economic Zones

**v0.4.44 GENESIS**

[![Modules](https://img.shields.io/badge/modules-146%2F146-brightgreen?style=flat-square)]()
[![PHOENIX](https://img.shields.io/badge/PHOENIX-11K%20lines-purple?style=flat-square)]()
[![Tests](https://img.shields.io/badge/tests-294%20passing-success?style=flat-square)]()
[![Bugs Fixed](https://img.shields.io/badge/bugs%20fixed-50%2B-red?style=flat-square)]()
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

Python 3.10+. No external dependencies.

### Installation

```bash
git clone https://github.com/momentum-inc/msez-stack.git
cd msez-stack

# Verify installation
PYTHONPATH=. python -c "from tools.phoenix import __version__; print(f'PHOENIX {__version__}')"
# → PHOENIX 0.4.44

# Run tests
pip install pytest
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

The MSEZ Stack is organized into four layers.

```
┌─────────────────────────────────────────────────────────────────────────────┐
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

**PHOENIX**: Compliance Tensor, ZK Proofs, Compliance Manifold, Migration Protocol, Watcher Economy, L1 Anchor, Corridor Bridge, Smart Asset VM, Security Layer, Hardening Layer.

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

## Repository Structure

```
msez-stack/
├── tools/
│   ├── msez/                  # Zone composition engine
│   │   ├── composition.py     # Multi-jurisdiction composer
│   │   ├── core.py            # Core primitives
│   │   └── schema.py          # Validation
│   ├── phoenix/               # PHOENIX execution layer (11K lines, 14 modules)
│   │   ├── tensor.py          # Compliance Tensor (955 lines)
│   │   ├── vm.py              # Smart Asset VM (1,285 lines)
│   │   ├── zkp.py             # ZK Proofs (766 lines)
│   │   ├── manifold.py        # Compliance Manifold (1,009 lines)
│   │   ├── migration.py       # Migration Protocol (886 lines)
│   │   ├── bridge.py          # Corridor Bridge (822 lines)
│   │   ├── anchor.py          # L1 Anchor (816 lines)
│   │   ├── watcher.py         # Watcher Economy (750 lines)
│   │   ├── security.py        # Security Layer (993 lines)
│   │   ├── hardening.py       # Hardening (744 lines)
│   │   ├── health.py          # Health Checks (400 lines)
│   │   ├── observability.py   # Logging/Tracing (500 lines)
│   │   ├── config.py          # Configuration (492 lines)
│   │   └── cli.py             # CLI Framework (450 lines)
│   ├── lawpack.py             # Legal text management
│   ├── regpack.py             # Regulatory guidance
│   ├── licensepack.py         # License registry
│   ├── arbitration.py         # Dispute resolution
│   └── agentic.py             # Policy automation
├── modules/                   # 146 zone modules (100% complete)
│   ├── legal/                 # Legal infrastructure (60+ jurisdictions)
│   ├── corporate/             # Corporate services (8 modules)
│   ├── licensing/             # Licensing (16 modules)
│   ├── financial/             # Financial infrastructure (13 modules)
│   ├── identity/              # Identity & credentials (6 modules)
│   ├── arbitration/           # Dispute resolution (7 modules)
│   ├── mass-primitives/       # MASS protocol (6 modules)
│   └── ...
├── deploy/
│   ├── aws/terraform/         # AWS infrastructure
│   ├── docker/                # Local development
│   └── helm/                  # Kubernetes charts
├── tests/                     # Test suites
├── schemas/                   # JSON schemas (116)
├── spec/                      # Specifications (25)
└── docs/                      # Documentation
```

---

## Version History

| Version | Codename | Highlights |
|---------|----------|------------|
| **0.4.44** | **GENESIS** | 146/146 modules (100%), 50+ bugs fixed, 294 tests, 27 VM opcodes, production infrastructure |
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
