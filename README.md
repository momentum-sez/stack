<div align="center">

# Momentum EZ Stack

### Deploy Economic Zones as code.

**v0.4.44 GENESIS** · Rust · BUSL-1.1

[![CI](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)]()
[![Rust](https://img.shields.io/badge/rust-1.75+-93450a?style=flat-square)]()
[![Tests](https://img.shields.io/badge/tests-4%2C073-brightgreen?style=flat-square)]()
[![Crates](https://img.shields.io/badge/crates-16-blue?style=flat-square)]()
[![Modules](https://img.shields.io/badge/modules-323-blue?style=flat-square)]()
[![Schemas](https://img.shields.io/badge/schemas-116-blue?style=flat-square)]()

[Getting Started](#getting-started) · [How It Works](#how-it-works) · [Architecture](#architecture) · [CLI](#cli) · [API](#api-server) · [Deployment](#deployment) · [Docs](./docs/)

</div>

---

## What is this

A zone definition file selects jurisdictions, composes regulatory modules, and generates the complete operational substrate for an Economic Zone: entity registry, compliance framework, banking rails, dispute resolution, cross-border corridors — backed by cryptographic proofs and verifiable credentials.

The EZ Stack is the **orchestration layer** above [Mass](https://mass.inc), Momentum's five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent). Mass handles CRUD. The EZ Stack adds compliance intelligence, corridor operations, and jurisdictional composition that primitive APIs cannot express.

```
Zone Admin ──> mez-api ──> Compliance Tensor ──> 20-domain evaluation
                       ──> Corridor Engine    ──> receipt chains, fork resolution
                       ──> Pack Trilogy        ──> lawpacks, regpacks, licensepacks
                       ──> mez-mass-client    ──> Mass APIs (sole authorized gateway)
                       ──> VC Issuance         ──> Ed25519-signed credentials
                       ──> Agentic Engine      ──> autonomous policy evaluation
```

---

## Getting started

```bash
git clone https://github.com/momentum-ez/stack.git && cd stack/mez

cargo build --workspace                     # build all 16 crates
cargo test  --workspace                     # run 4,073 tests
cargo clippy --workspace -- -D warnings     # zero warnings policy

cargo run -p mez-api                        # start API server on :3000
cargo run -p mez-cli -- validate --all-modules  # validate 323 modules
cargo run -p mez-cli -- vc keygen --output keys/ --prefix dev
```

**Prerequisites:** Rust 1.75+, Git. Optional: Docker 24+, kubectl 1.28+, Terraform 1.5+.

See [docs/getting-started.md](./docs/getting-started.md) for the full walkthrough.

---

## How it works

A **zone** is defined by a YAML file that composes jurisdictions and modules:

```yaml
zone_id: momentum.zone.pk-rez
name: "Pakistan Regulatory Economic Zone"

jurisdictions:
  civic: pk
  corporate: pk
  financial: pk-sbp
  digital: pk-secp

corridors:
  - swift-iso20022
  - stablecoin-usdc

arbitration:
  primary: pk-rez-tribunal
```

This selects from 323 modules across 16 families — legal, corporate, regulatory, licensing, identity, financial, capital markets, trade, tax, corridors, governance, arbitration, operations, smart assets, and mass primitives — and generates the complete infrastructure.

Each module is a YAML descriptor validated against 116 JSON Schemas (Draft 2020-12). The lockfile (`stack.lock`) pins every module, artifact, and dependency by SHA-256 digest for reproducible deployments.

---

## Architecture

The EZ Stack owns **orchestration, compliance, and cryptographic state**. Mass owns primitive data.

### What the EZ Stack provides

| Domain | Crate | Capability |
|--------|-------|-----------|
| Compliance | `mez-tensor` | 20-domain compliance evaluation per entity/jurisdiction. Dijkstra-optimized migration paths. Merkle-committed state. |
| Corridors | `mez-corridor` | Cross-border receipt chains (MMR), fork detection and resolution, bilateral netting, SWIFT pacs.008. |
| Pack Trilogy | `mez-pack` | Lawpacks (Akoma Ntoso statutes), regpacks (sanctions, calendars), licensepacks (license registries). |
| State Machines | `mez-state` | Typestate-encoded lifecycles: corridor (6 states), entity (10 stages), migration (8 phases), license (5 states), watcher (4 states). Invalid transitions are compile errors. |
| Credentials | `mez-vc` | W3C Verifiable Credentials with Ed25519 proofs. KYC, sanctions, compliance, corridor attestations. |
| Policy Engine | `mez-agentic` | 20 trigger types, autonomous evaluation, deterministic conflict resolution, append-only audit trail. |
| Arbitration | `mez-arbitration` | 7-phase dispute lifecycle, evidence chain-of-custody, escrow, enforcement via VC-triggered transitions. |
| Zero-Knowledge | `mez-zkp` | Sealed `ProofSystem` trait, 12 circuit types. Phase 1: deterministic mock. Phase 2: Groth16/PLONK (feature-gated). |
| Mass Gateway | `mez-mass-client` | Typed HTTP client for all five Mass primitives. The only authorized path to Mass. |

### Crate map

```
mez/crates/
├── mez-core             Foundation: canonicalization (JCS+MCF), 20 ComplianceDomains,
│                         identifier newtypes, error hierarchy. Zero internal deps.
├── mez-crypto           Ed25519 (zeroize), MMR, CAS, SHA-256.
│                         BBS+ and Poseidon2 behind feature flags.
├── mez-vc               W3C Verifiable Credentials, Ed25519 proofs, registry.
├── mez-state            Typestate machines — invalid transitions don't compile.
├── mez-tensor           Compliance Tensor V2: 20 domains x 5-state lattice,
│                         Dijkstra manifold, Merkle commitments.
├── mez-zkp              Sealed ProofSystem trait, 12 circuits, CDB bridge.
├── mez-pack             Lawpack / Regpack / Licensepack — sanctions checker.
├── mez-corridor         Receipt chain (MMR), fork resolution, netting, SWIFT.
├── mez-agentic          Policy engine: 20 triggers, scheduling, audit trail.
├── mez-arbitration      Dispute lifecycle, evidence, escrow, enforcement.
├── mez-compliance       Jurisdiction config bridge (regpack -> tensor).
├── mez-schema           JSON Schema validation (Draft 2020-12, 116 schemas).
├── mez-mass-client      Typed HTTP client for all 5 Mass API primitives.
├── mez-api              Axum HTTP server — composition root for all crates.
├── mez-cli              CLI: validate, lock, corridor, artifact, vc.
└── mez-integration-tests  113 cross-crate test files.
```

### Type-level safety

| Guarantee | Mechanism |
|-----------|-----------|
| No invalid state transitions | Typestate pattern: each state is a ZST. `Corridor<Draft>` has `.submit()` but no `.halt()`. |
| No serialization divergence | `CanonicalBytes::new()` is the sole path to digest computation. |
| No type confusion | Identifier newtypes: `EntityId`, `CorridorId`, `MigrationId`, `WatcherId`, `DisputeId`. |
| No unauthorized proof backends | `ProofSystem` trait is sealed. Only `mez-zkp` can implement it. |
| No key material leakage | `SigningKey`: `Zeroize` + `ZeroizeOnDrop`. Does not implement `Serialize`. |
| No `unwrap()` in production | All errors via `thiserror`. Exhaustive `match` on all enums. |

---

## CLI

```bash
# Validation
mez validate --all-modules                 # validate all 323 modules
mez validate --all-profiles                # validate all profiles
mez validate path/to/module.yaml           # validate a single module

# Lockfiles
mez lock zone.yaml                         # generate stack.lock
mez lock zone.yaml --check                 # verify existing lockfile

# Corridors
mez corridor create --id PK-AE --jurisdiction-a PK-REZ --jurisdiction-b AE-DIFC
mez corridor activate --id PK-AE --approval-a sig-a.json --approval-b sig-b.json
mez corridor status --id PK-AE

# Verifiable Credentials
mez vc keygen --output keys/ --prefix zone-admin
mez vc sign --key keys/zone-admin.priv.json document.json
mez vc verify --pubkey keys/zone-admin.pub.json document.json --signature abc...

# Artifacts
mez artifact store --type lawpack path/to/archive.zip
mez artifact verify --type schema --digest abc123...
```

---

## API server

```bash
cargo run -p mez-api          # http://localhost:3000
                               # OpenAPI spec at /openapi.json
```

**Middleware:** `TraceLayer` -> `Metrics` -> `Auth` (constant-time bearer) -> `RateLimit` -> Handler

| Route | Domain | Implementation |
|-------|--------|----------------|
| `/v1/entities/*` | Mass Entities | Proxy via `mez-mass-client` |
| `/v1/ownership/*` | Mass Ownership | Proxy via `mez-mass-client` |
| `/v1/fiscal/*` | Mass Fiscal | Proxy via `mez-mass-client` |
| `/v1/identity/*` | Mass Identity | Proxy via `mez-mass-client` |
| `/v1/consent/*` | Mass Consent | Proxy via `mez-mass-client` |
| `/v1/corridors/*` | Corridors | Native: lifecycle, receipts, forks |
| `/v1/settlement/*` | Settlement | Native: netting, SWIFT instructions |
| `/v1/assets/*` | Smart Assets | Native: registry, compliance eval |
| `/v1/credentials/*` | Credentials | Native: VC issuance, verification |
| `/v1/triggers` | Agentic | Native: policy trigger evaluation |
| `/v1/regulator/*` | Regulator | Native: compliance monitoring |
| `/health/liveness` | Health | Always 200 |
| `/health/readiness` | Health | Checks stores, key, locks |

---

## Deployment

### Docker Compose

```bash
cd deploy/docker && docker-compose up -d
# mez-api (port 8080) + PostgreSQL 16 + Prometheus + Grafana
```

### Kubernetes

```bash
kubectl apply -f deploy/k8s/
# 2 replicas, rolling updates, non-root, resource limits, probes
```

### AWS (Terraform)

```bash
cd deploy/aws/terraform && terraform apply -var-file=examples/hybrid-zone.tfvars
# EKS (auto-scaling) + RDS (Multi-AZ) + ElastiCache + S3 + KMS + ALB/TLS
```

---

## Repository layout

```
stack/
├── mez/                   Rust workspace (16 crates, 151K lines)
│   ├── Cargo.toml          Workspace manifest
│   └── crates/             All crate source
├── modules/                323 zone modules across 16 families
├── schemas/                116 JSON Schema files (Draft 2020-12)
├── spec/                   24 normative specification chapters
├── apis/                   OpenAPI 3.x specifications
├── deploy/                 Docker, Kubernetes, Terraform
├── contexts/               Zone composition contexts
├── jurisdictions/           100 zone definitions (US states, UAE free zones, PK, CN, etc.)
├── rulesets/                Regulatory rulesets
├── registries/              Registry data
├── dist/artifacts/          Content-addressed built artifacts
├── governance/              Lifecycle state machines, changelog
└── docs/                   Documentation
```

---

## Documentation

| Document | Path |
|----------|------|
| Getting started | [`docs/getting-started.md`](./docs/getting-started.md) |
| Architecture overview | [`docs/architecture/OVERVIEW.md`](./docs/architecture/OVERVIEW.md) |
| Crate reference | [`docs/architecture/CRATE-REFERENCE.md`](./docs/architecture/CRATE-REFERENCE.md) |
| Mass integration | [`docs/architecture/MASS-INTEGRATION.md`](./docs/architecture/MASS-INTEGRATION.md) |
| Security model | [`docs/architecture/SECURITY-MODEL.md`](./docs/architecture/SECURITY-MODEL.md) |
| Error taxonomy | [`docs/ERRORS.md`](./docs/ERRORS.md) |
| Spec-to-code mapping | [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) |
| Attestation catalog | [`docs/attestations/catalog.md`](./docs/attestations/catalog.md) |
| Deployment roadmap | [`docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md`](./docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md) |

---

## Design principles

| Principle | Enforcement |
|-----------|-------------|
| **Fail closed** | Unknown = `NonCompliant`. Missing attestations invalidate. System fails safe. |
| **Cryptographic integrity** | Every state transition produces proof. Tensor commitments are Merkle roots. Receipts form MMR chains. |
| **Type-level correctness** | Typestate machines. Sealed traits. Identifier newtypes. No `unwrap()`. |
| **Deterministic execution** | No floating point. No randomness in evaluation. `BTreeMap` iteration. `CanonicalBytes` for all digests. |
| **Defense in depth** | Constant-time auth. Zeroize on key drop. Rate limiting. Schema validation at boundary. |

---

<div align="center">

**[Momentum](https://momentum.inc)** · **[Mass](https://mass.inc)**

*Programmable institutions for durable economies.*

</div>
