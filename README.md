<div align="center">

# Momentum EZ Stack

### The AWS of Economic Zones.

Deploy jurisdictional infrastructure as code — compliance, corridors, credentials, and capital flows — in a single Rust binary.

**v0.4.44 GENESIS** · BUSL-1.1

[![Build](https://img.shields.io/badge/build-passing-brightgreen?style=flat-square)]()
[![Rust](https://img.shields.io/badge/rust-1.75+-93450a?style=flat-square)]()
[![Tests](https://img.shields.io/badge/tests-4%2C683-brightgreen?style=flat-square)]()
[![Crates](https://img.shields.io/badge/crates-17-blue?style=flat-square)]()
[![Zones](https://img.shields.io/badge/zones-210-blue?style=flat-square)]()
[![Schemas](https://img.shields.io/badge/schemas-116-blue?style=flat-square)]()
[![Lines](https://img.shields.io/badge/lines-164K-blue?style=flat-square)]()

[Quick Start](#quick-start) · [The Thesis](#the-thesis) · [How It Works](#how-it-works) · [Architecture](#architecture) · [CLI](#cli) · [API](#api-server) · [Deploy](#deployment) · [Docs](./docs/)

</div>

---

## The thesis

Traditional Economic Zones take 3-7 years and $50-200M to establish: bilateral treaties, regulatory frameworks, banking relationships, corporate registries, dispute resolution, licensing regimes.

The EZ Stack reduces this to configuration.

```
$ mez deploy --profile digital-financial-center \
    --jurisdiction ae-adgm \
    --corridors "pk-sifc,sg"
```

One command selects from 210 zone definitions across 16 module families, evaluates compliance across 20 regulatory domains, establishes cross-border corridors with cryptographic receipt chains, and deploys a sovereign zone with verifiable credentials, autonomous policy execution, and full audit trails.

**This is not a blockchain.** It is a compliance orchestration engine backed by content-addressed artifacts, Ed25519 signatures, Merkle Mountain Ranges, and W3C Verifiable Credentials — running as a single Rust binary with Postgres.

### The AWS analogy

| AWS | EZ Stack | Status |
|-----|----------|--------|
| `ec2 run-instances` | `deploy-zone.sh <profile> <zone-id> <jurisdiction>` | Working |
| VPC | Zone — legal perimeter with compliance tensor | Working |
| VPC Peering | Corridor — bilateral compliance-verified channel | Working |
| IAM | Verifiable Credentials + Ed25519 key hierarchy | Working |
| CloudFormation | `zone.yaml` + `stack.lock` (SHA-256 pinned) | Working |
| S3 | Content-addressed store (CAS) | Working |
| CloudWatch | Prometheus + Grafana | Working |
| Marketplace | Module registry (323 modules, 16 families) | Working |
| Region | Jurisdiction (with lawpack, regpack, licensepack) | 210 defined |
| Availability Zone | Zone profile (6 types) | Working |

---

## Quick start

```bash
git clone https://github.com/momentum-ez/stack.git && cd stack/mez

cargo build --workspace                     # build all 17 crates
cargo test  --workspace                     # run 4,683 tests
cargo clippy --workspace -- -D warnings     # zero warnings policy

cargo run -p mez-api                        # start API server on :3000
cargo run -p mez-cli -- validate --all-modules  # validate 323 modules
```

**Prerequisites:** Rust 1.75+, Git. Optional: Docker 24+, kubectl 1.28+, Terraform 1.5+.

### Deploy a zone in 5 commands

```bash
# 1. Generate zone signing key
cargo run -p mez-cli -- vc keygen --output keys/ --prefix pk-sifc

# 2. Build content-addressed regpack artifacts
cargo run -p mez-cli -- regpack build --jurisdiction pk --all-domains --store

# 3. Generate deterministic lockfile (SHA-256 pinned)
cargo run -p mez-cli -- lock jurisdictions/pk-sifc/zone.yaml

# 4. Deploy sovereign zone (mez-api + Postgres + Prometheus + Grafana)
./deploy/scripts/deploy-zone.sh sovereign-govos org.momentum.mez.zone.pk-sifc pk

# 5. Verify
curl http://localhost:8080/health/readiness
```

See [docs/getting-started.md](./docs/getting-started.md) for the full walkthrough and [docs/ZONE-BOOTSTRAP-GUIDE.md](./docs/ZONE-BOOTSTRAP-GUIDE.md) for production deployment.

---

## How it works

### Zone definition

A zone is defined by a YAML file that composes jurisdictions, modules, and corridors:

```yaml
zone_id: org.momentum.mez.zone.pk-sifc
jurisdiction_id: pk

profile:
  profile_id: org.momentum.mez.profile.sovereign-govos
  version: "0.4.44"

lawpack_domains:
  - civil
  - financial
  - tax
  - aml

regpacks:
  - jurisdiction_id: pk
    domain: financial
    regpack_digest_sha256: "444ddded8419d9dedf8344a54063d7cd..."
    as_of_date: "2026-01-15"

corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border
```

This selects from 323 modules across 16 families and generates the complete operational substrate. The lockfile (`stack.lock`) pins every module, artifact, and dependency by SHA-256 digest for reproducible deployments.

### The orchestration pipeline

Every write operation follows the same path:

```
Request
  -> Auth (constant-time bearer comparison)
  -> Compliance Tensor (20-domain evaluation per entity/jurisdiction)
  -> Sanctions hard-block (NonCompliant = reject — legal requirement)
  -> Mass API call (delegate via mez-mass-client or sovereign Postgres)
  -> VC issuance (Ed25519-signed compliance attestation)
  -> Attestation storage (Postgres, for regulator queries)
  -> Response (OrchestrationEnvelope: mass_response + compliance + credential)
```

Read operations are pass-through — no compliance evaluation needed.

### Two deployment modes

| Mode | Architecture | Use case |
|------|-------------|----------|
| **Sovereign** (`SOVEREIGN_MASS=true`) | `mez-api` + Postgres — all data stays in-zone | Production sovereign deployment |
| **Proxy** (`SOVEREIGN_MASS=false`) | `mez-api` -> centralized Mass APIs at `mass.inc` | Integration with existing Mass |

In sovereign mode, each zone is an independent data sovereign — Zone A's data never leaves Zone A's infrastructure. This is the path to decentralized Mass.

---

## Architecture

### System layers

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP BOUNDARY                          │
│  mez-api:  Axum server, auth, rate limiting, OpenAPI        │
│  mez-cli:  Offline zone management, validation, signing     │
├─────────────────────────────────────────────────────────────┤
│                   ORCHESTRATION LAYER                        │
│  mez-agentic:     20 triggers, policy engine, audit trail   │
│  mez-arbitration:  Dispute lifecycle, evidence, escrow      │
│  mez-compliance:   Regpack -> tensor bridge                 │
├─────────────────────────────────────────────────────────────┤
│                 DOMAIN INTELLIGENCE                          │
│  mez-tensor:   Compliance Tensor V2 (20 domains, 5 states) │
│  mez-corridor: Receipt chains, fork resolution, netting     │
│  mez-state:    Typestate machines (corridor, entity, etc.)  │
│  mez-pack:     Lawpack, regpack, licensepack                │
├─────────────────────────────────────────────────────────────┤
│                CRYPTOGRAPHIC FOUNDATION                      │
│  mez-vc:     W3C Verifiable Credentials, Ed25519 proofs     │
│  mez-crypto: Ed25519 (zeroize), MMR, CAS, SHA-256          │
│  mez-zkp:    Sealed ProofSystem trait, 12 circuits          │
│  mez-core:   CanonicalBytes, ComplianceDomain(20), newtypes│
├─────────────────────────────────────────────────────────────┤
│                  EXTERNAL INTEGRATION                        │
│  mez-mass-client: Typed HTTP client for 5 Mass primitives   │
│  mez-schema:      116 JSON Schema (Draft 2020-12) validation│
└─────────────────────────────────────────────────────────────┘
```

### What the EZ Stack provides

| Domain | Crate | Capability |
|--------|-------|-----------|
| **Compliance** | `mez-tensor` | 20-domain compliance evaluation. Dijkstra-optimized migration paths. Merkle-committed state. |
| **Corridors** | `mez-corridor` | Cross-border receipt chains (MMR), fork detection/resolution, bilateral netting, SWIFT pacs.008. |
| **Pack Trilogy** | `mez-pack` | Lawpacks (Akoma Ntoso statutes), regpacks (sanctions, calendars), licensepacks (70+ jurisdictions). |
| **State Machines** | `mez-state` | Typestate-encoded lifecycles: corridor (6), entity (10), migration (8), license (5), watcher (4). Invalid transitions are compile errors. |
| **Credentials** | `mez-vc` | W3C Verifiable Credentials with Ed25519 proofs. |
| **Policy Engine** | `mez-agentic` | 20 trigger types, autonomous evaluation, deterministic conflict resolution, append-only audit trail. |
| **Arbitration** | `mez-arbitration` | 7-phase dispute lifecycle, evidence chain-of-custody, escrow, enforcement via VC-triggered transitions. |
| **Zero-Knowledge** | `mez-zkp` | Sealed `ProofSystem` trait, 12 circuit types. Phase 1: mock. Phase 2: Groth16/PLONK (feature-gated). |
| **Mass Gateway** | `mez-mass-client` | Typed HTTP client for all five Mass primitives. The only authorized path to Mass. |
| **Schema** | `mez-schema` | 116 JSON Schemas (Draft 2020-12) validated at API boundary. |

### The Mass/EZ boundary

The EZ Stack sits above [Mass](https://mass.inc), Momentum's five programmable primitives:

| Primitive | Service | Domain |
|-----------|---------|--------|
| Entities | `organization-info.api.mass.inc` | Formation, lifecycle, beneficial ownership |
| Ownership | `investment-info` | Cap tables, share classes, transfers |
| Fiscal | `treasury-info.api.mass.inc` | Accounts, payments, treasury |
| Identity | *(split across consent-info + org-info)* | KYC/KYB, credentials, DIDs |
| Consent | `consent.api.mass.inc` | Multi-party governance, audit trails |

**The boundary rule:** Mass owns CRUD. The EZ Stack owns compliance intelligence, corridor operations, and cryptographic provenance. `mez-mass-client` is the sole authorized gateway.

### Type-level safety

| Guarantee | Mechanism |
|-----------|-----------|
| No invalid state transitions | Typestate pattern: `Corridor<Draft>` has `.submit()` but no `.halt()` |
| No serialization divergence | `CanonicalBytes::new()` is the sole path to digest computation |
| No type confusion | Identifier newtypes: `EntityId`, `CorridorId`, `MigrationId`, `WatcherId`, `DisputeId` |
| No unauthorized proof backends | `ProofSystem` trait is sealed — only `mez-zkp` can implement it |
| No key material leakage | `SigningKey`: `Zeroize` + `ZeroizeOnDrop`, does not implement `Serialize` |
| No `unwrap()` in production | All errors via `thiserror`, exhaustive `match` on all enums |

---

## CLI

```bash
# Zone management
mez validate --all-modules                 # validate all 323 modules
mez validate --all-zones                   # validate all 210 zones
mez lock zone.yaml                         # generate deterministic lockfile
mez lock zone.yaml --check                 # verify existing lockfile

# Corridors
mez corridor create --id PK-AE --jurisdiction-a PK-REZ --jurisdiction-b AE-DIFC
mez corridor activate --id PK-AE --approval-a sig-a.json --approval-b sig-b.json
mez corridor status --id PK-AE
mez corridor mesh --all                    # generate N-factorial corridor mesh

# Verifiable Credentials
mez vc keygen --output keys/ --prefix zone-admin
mez vc sign --key keys/zone-admin.priv.json document.json
mez vc verify --pubkey keys/zone-admin.pub.json document.json --signature abc...

# Regpacks
mez regpack build --jurisdiction pk --all-domains --store

# Content-addressed artifacts
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

### Routes

| Route | Domain | Source |
|-------|--------|--------|
| `/v1/entities/*` | Mass Entities | Proxy or sovereign |
| `/v1/ownership/*` | Mass Ownership | Proxy or sovereign |
| `/v1/fiscal/*` | Mass Fiscal | Proxy or sovereign |
| `/v1/identity/*` | Mass Identity | Proxy or sovereign |
| `/v1/consent/*` | Mass Consent | Proxy or sovereign |
| `/v1/corridors/*` | Corridors | Native: lifecycle, receipts, forks, peers |
| `/v1/settlement/*` | Settlement | Native: netting, SWIFT instructions |
| `/v1/assets/*` | Smart Assets | Native: registry, compliance eval |
| `/v1/credentials/*` | Credentials | Native: VC issuance, verification |
| `/v1/triggers` | Agentic | Native: policy trigger evaluation |
| `/v1/regulator/*` | Regulator | Native: compliance monitoring |
| `/v1/trade/flows/*` | Trade | Native: trade flow lifecycle, 4 archetypes |
| `/v1/compliance/*` | Compliance | Native: entity/corridor compliance queries |
| `/v1/watchers/*` | Watchers | Native: bond, attest, slash lifecycle |
| `/health/liveness` | Health | Always 200 |
| `/health/readiness` | Health | Checks stores, key, locks |

---

## Deployment

### Docker Compose (single zone)

```bash
cd deploy/docker && docker-compose up -d
# mez-api (port 8080) + PostgreSQL 16 + Prometheus + Grafana
```

### Docker Compose (two-zone corridor)

```bash
./deploy/scripts/demo-two-zone.sh
# Zone A (PK-SIFC) + Zone B (AE-DIFC) + corridor receipt exchange
# Each zone: sovereign mez-api + Postgres (data never leaves zone boundary)
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
├── mez/                   Rust workspace (17 crates, 164K lines)
│   ├── Cargo.toml          Workspace manifest
│   └── crates/             All crate source (322 .rs files)
├── modules/                323 zone modules across 16 families
├── schemas/                116 JSON Schema files (Draft 2020-12)
├── spec/                   25 normative specification chapters
├── apis/                   4 OpenAPI 3.x specifications
├── jurisdictions/          210 zone definitions
├── registries/             Corridor registry (99 zones, 4,851 derivable pairs)
├── deploy/                 Docker, Kubernetes, Terraform
│   ├── docker/             Single-zone and two-zone compose files
│   ├── k8s/                Kubernetes manifests
│   ├── aws/terraform/      EKS + RDS + KMS infrastructure
│   └── scripts/            Deploy and demo scripts
├── contexts/               Zone composition contexts
├── rulesets/               Regulatory rulesets
├── governance/             Lifecycle state machines, changelog
├── dist/artifacts/         Content-addressed built artifacts
└── docs/                   Architecture, guides, roadmap, reference
```

---

## The path to decentralization

The EZ Stack is not merely an orchestration layer. It is the deployment substrate that **progressively decentralizes Mass** through sovereign zone deployments.

```
Phase 1 (Today)           Phase 2 (Near-term)          Phase 3-4 (End-state)
┌──────────────┐          ┌──────────────┐             ┌──────────────┐
│   MEZ Zone   │          │   MEZ Zone   │             │   MEZ Zone   │
│ (compliance, │          │ (compliance, │             │ (compliance, │
│  corridors)  │          │  corridors)  │◄──corridor──►  corridors)  │
├──────────────┤          ├──────────────┤             ├──────────────┤
│ Centralized  │          │  Sovereign   │             │  Sovereign   │
│  Mass APIs   │          │  Mass APIs   │             │ Mass + DAG   │
│ (mass.inc)   │          │ (in-zone)    │             │ (federated)  │
└──────────────┘          └──────────────┘             └──────────────┘
```

- Every zone deployment is a future Mass consensus node
- Every corridor is a future DAG edge
- Every compliance tensor evaluation is future JVM execution
- Every receipt chain is future consensus history
- Every watcher attestation is a future validator vote

The Mass Protocol's end-state emerges bottom-up from the federation of sovereign deployments — not from building a monolithic L1.

---

## Documentation

| Document | Path | Scope |
|----------|------|-------|
| Getting started | [`docs/getting-started.md`](./docs/getting-started.md) | Build, test, run, configure |
| Zone bootstrap | [`docs/ZONE-BOOTSTRAP-GUIDE.md`](./docs/ZONE-BOOTSTRAP-GUIDE.md) | End-to-end zone deployment |
| Architecture | [`docs/ARCHITECTURE.md`](./docs/ARCHITECTURE.md) | System design, data flow, invariants |
| Crate reference | [`docs/architecture/CRATE-REFERENCE.md`](./docs/architecture/CRATE-REFERENCE.md) | Per-crate API surface |
| Security model | [`docs/architecture/SECURITY-MODEL.md`](./docs/architecture/SECURITY-MODEL.md) | Trust boundaries, threat model |
| Mass integration | [`docs/architecture/MASS-INTEGRATION.md`](./docs/architecture/MASS-INTEGRATION.md) | Mass Protocol mapping |
| Error taxonomy | [`docs/ERRORS.md`](./docs/ERRORS.md) | P-codes, RFC 7807, recovery |
| Traceability | [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) | Spec chapter to Rust crate mapping |
| Deployment roadmap | [`docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md`](./docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md) | Phase gates and priorities |
| Specification | [`spec/`](./spec/) | 25 normative chapters |
| OpenAPI specs | [`apis/`](./apis/) | 4 contract-grade API specifications |

---

## Design principles

| Principle | Enforcement |
|-----------|-------------|
| **Fail closed** | Unknown = `NonCompliant`. Missing attestations invalidate. Empty tensor slices = error. |
| **Cryptographic integrity** | Every state transition produces proof. Tensor commitments are Merkle roots. Receipts form MMR chains. |
| **Type-level correctness** | Typestate machines. Sealed traits. Identifier newtypes. No `unwrap()`. No `todo!()`. |
| **Deterministic execution** | No floating point. No randomness. `BTreeMap` iteration. `CanonicalBytes` for all digests. |
| **Defense in depth** | Constant-time auth. Zeroize on key drop. Rate limiting. Schema validation at boundary. |
| **Verify, never trust** | All inputs validated. Signatures verified. Digests recomputed. Chains verified. |

---

## Phase readiness

| Phase | Target | Status |
|-------|--------|--------|
| Phase 1: Controlled Sandbox | Single zone, centralized Mass | **READY** — all entry criteria met |
| Phase 2: Corridor Activation | Two sovereign zones, one corridor | **READY** — infrastructure validated |
| Phase 3: Sovereign GovOS | Pakistan pilot with real institutions | BLOCKED — national system adapters |
| Phase 4: Corridor Mesh | 5+ zones, N-factorial corridors | BLOCKED — BBS+, real ZK backends |

---

## License

[Business Source License 1.1](./LICENSE.md)

---

<div align="center">

**[Momentum](https://momentum.inc)** · **[Mass](https://mass.inc)**

*Programmable institutions for durable economies.*

*164K lines of Rust. 4,683 tests. 210 zones. Zero `unwrap()`.*

</div>
