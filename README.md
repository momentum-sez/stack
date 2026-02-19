<div align="center">

# Momentum EZ Stack

### Programmable jurisdictional infrastructure in Rust.

**v0.4.44** · 16 crates · ~48K lines · 2,580+ tests

[![CI](https://img.shields.io/badge/CI-passing-brightgreen?style=flat-square)]()
[![Rust](https://img.shields.io/badge/rust-1.75+-blue?style=flat-square)]()
[![Crates](https://img.shields.io/badge/crates-16-blue?style=flat-square)]()
[![Tests](https://img.shields.io/badge/tests-2%2C580+-brightgreen?style=flat-square)]()
[![Modules](https://img.shields.io/badge/zone_modules-146-brightgreen?style=flat-square)]()
[![License](https://img.shields.io/badge/license-BUSL--1.1-lightgrey?style=flat-square)]()

[Quickstart](#quickstart) · [Architecture](#architecture) · [Crates](#crate-map) · [CLI](#cli) · [API Server](#api-server) · [Deployment](#deployment) · [Docs](./docs/)

</div>

---

## What this is

The EZ Stack deploys Economic Zones the way you deploy cloud infrastructure: as configuration.

A zone definition file selects jurisdictions, composes legal/regulatory/financial modules, and generates the complete operational substrate -- entity registry, compliance framework, banking adapters, dispute resolution, cross-border corridors -- backed by cryptographic proofs and verifiable credentials.

The Rust workspace is the **orchestration layer** that sits above the live [Mass](https://mass.inc) APIs. Mass implements the five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent) as deployed services. The EZ Stack provides compliance intelligence, corridor operations, and jurisdictional composition that Mass primitives alone cannot express.

```
Zone Admin ──> EZ Stack API ──> Compliance Tensor (20-domain evaluation)
                              ──> Corridor state machine (typestate-enforced)
                              ──> Mass API client ──> organization-info.api.mass.inc
                                                   ──> treasury-info.api.mass.inc
                                                   ──> consent.api.mass.inc
                              ──> VC issuance (Ed25519 signing)
                              ──> Receipt chain (MMR append)
                              ──> Agentic policy evaluation
                              ──> Response
```

---

## Quickstart

```bash
git clone https://github.com/momentum-ez/stack.git
cd stack/mez

# Build all 16 crates
cargo build --workspace

# Run the full test suite
cargo test --workspace

# Zero clippy warnings
cargo clippy --workspace -- -D warnings

# Generate rustdoc
cargo doc --workspace --no-deps --open

# Start the API server (port 3000, OpenAPI at /openapi.json)
cargo run -p mez-api

# Use the CLI
cargo run -p mez-cli -- validate --all-modules
cargo run -p mez-cli -- corridor list
cargo run -p mez-cli -- vc keygen --output keys/ --prefix dev
```

---

## Architecture

The EZ Stack owns **orchestration, compliance, and cryptographic state**. It does not own primitive data (entities, cap tables, payments, identity records, consent) -- that belongs to the Mass APIs.

### What the EZ Stack owns

| Domain | Crate | What it does |
|--------|-------|-------------|
| **Compliance Tensor** | `mez-tensor` | Evaluate compliance across 20 regulatory domains per entity/jurisdiction. Dijkstra-optimized migration paths. Merkle-committed state. |
| **Corridors** | `mez-corridor` | Cross-border trade channels with MMR-backed receipt chains, fork detection/resolution, bilateral netting, SWIFT pacs.008 generation. |
| **Pack Trilogy** | `mez-pack` | Parse and validate lawpacks (Akoma Ntoso statutes), regpacks (sanctions lists, regulatory calendars), and licensepacks (license registries). |
| **State Machines** | `mez-state` | Typestate-encoded lifecycles for corridors (6 states), entities (10 stages), migrations (8 phases), licenses (5 states), watchers (4 states). Invalid transitions are compile errors. |
| **Verifiable Credentials** | `mez-vc` | W3C VC issuance and verification with Ed25519 proofs. Credentials for KYC, sanctions clearance, corridor agreements, compliance attestations. |
| **Agentic Engine** | `mez-agentic` | 20 trigger types, autonomous policy evaluation, deterministic conflict resolution, append-only audit trail. |
| **Arbitration** | `mez-arbitration` | Dispute lifecycle (7 phases), evidence chain-of-custody, escrow management, enforcement via VC-triggered state transitions. |
| **Zero-Knowledge** | `mez-zkp` | Sealed `ProofSystem` trait with 12 circuit types. Phase 1: deterministic mock. Phase 2: Groth16/PLONK backends (feature-gated). |
| **Mass API Client** | `mez-mass-client` | Typed Rust HTTP client for all five Mass API primitives. The only authorized path from EZ Stack to Mass. |

### What Mass owns (not in this repo)

Entities, cap tables, payments, identity/KYC records, and consent -- all live in Mass API services (`organization-info.api.mass.inc`, `treasury-info.api.mass.inc`, etc.). The EZ Stack calls Mass through `mez-mass-client`; it never stores primitive data directly.

---

## Crate map

16 crates, resolver v2, edition 2021, MSRV 1.75.

```
mez/crates/
├── mez-core            Canonicalization (JCS), 20 ComplianceDomain variants,
│                        identifier newtypes, error hierarchy
├── mez-crypto          Ed25519 (zeroize-on-drop), MMR, CAS, SHA-256
│                        BBS+ and Poseidon2 behind feature flags
├── mez-vc              W3C Verifiable Credentials, Ed25519 proofs, registry
├── mez-state           Typestate machines: Corridor, Entity, Migration,
│                        License, Watcher — invalid transitions don't compile
├── mez-tensor          Compliance Tensor V2 (20 domains x 5-state lattice),
│                        Dijkstra manifold, Merkle commitments
├── mez-zkp             Sealed ProofSystem trait, 12 circuits, CDB bridge
├── mez-pack            Lawpack / Regpack / Licensepack — sanctions checker
├── mez-corridor        Receipt chain (MMR), fork resolution, netting, SWIFT
├── mez-agentic         Policy engine: 20 triggers, scheduling, audit trail
├── mez-arbitration     Dispute lifecycle, evidence, escrow, enforcement
├── mez-compliance      Jurisdiction config bridge (regpack → tensor)
├── mez-schema          JSON Schema validation (Draft 2020-12, 116 schemas)
├── mez-mass-client     Typed HTTP client for all 5 Mass API primitives
├── mez-api             Axum HTTP server — corridors, settlement, assets,
│                        credentials, regulator, agentic, Mass proxy
├── mez-cli             CLI: validate, lock, corridor, artifact, vc
└── mez-integration-tests  99 cross-crate test files
```

### Dependency graph

```
mez-core ─────────────────────────────────────────────────────────┐
  │                                                                │
  ├── mez-crypto ──┬── mez-vc                                    │
  │                 ├── mez-zkp                                   │
  │                 └── mez-tensor ── mez-compliance             │
  │                                                                │
  ├── mez-state ───┬── mez-corridor                              │
  │                 └── mez-arbitration                           │
  │                                                                │
  ├── mez-pack                                                    │
  ├── mez-agentic                                                 │
  ├── mez-schema                                                  │
  ├── mez-mass-client                                             │
  │                                                                │
  └── mez-api (depends on most crates above)                      │
      mez-cli (depends on core, crypto, schema)                   │
      mez-integration-tests (depends on everything)───────────────┘
```

### Type-level safety guarantees

| Guarantee | Mechanism |
|-----------|-----------|
| No invalid state transitions | Typestate pattern -- each state is a distinct ZST; transitions are methods that consume `self` and return the next state type. `Corridor<Draft>` has `.submit()` but no `.halt()`. |
| No serialization divergence | `CanonicalBytes::new()` is the sole path to digest computation. All signing flows require `&CanonicalBytes`. |
| No type confusion | Identifier newtypes: `EntityId`, `CorridorId`, `MigrationId`, `WatcherId`, `DisputeId` -- the compiler rejects mixing them. |
| No unauthorized proof backends | `ProofSystem` trait is sealed. Only `mez-zkp` can implement it. |
| No key material leakage | `SigningKey` implements `Zeroize` + `ZeroizeOnDrop`. Does not implement `Serialize`. |
| No Mass API calls outside client | All Mass HTTP calls go through `mez-mass-client`. Direct `reqwest` to Mass endpoints from other crates is forbidden by convention. |
| No `unwrap()` in library crates | All errors use `thiserror`. Exhaustive `match` on all enums. |

---

## CLI

The `mez` binary (`mez-cli` crate) provides offline zone management operations.

```bash
# Validate modules, profiles, zones
mez validate --all-modules
mez validate --all-profiles
mez validate --all-zones
mez validate path/to/module.yaml

# Generate and verify lockfiles
mez lock zone.yaml                         # generate stack.lock
mez lock zone.yaml --check                 # verify existing lockfile
mez lock zone.yaml --strict --out prod.lock

# Corridor lifecycle
mez corridor create --id PK-AE --jurisdiction-a PK-REZ --jurisdiction-b AE-DIFC
mez corridor submit --id PK-AE --agreement corridor-agreement.json --pack-trilogy packs/
mez corridor activate --id PK-AE --approval-a sig-a.json --approval-b sig-b.json
mez corridor status --id PK-AE
mez corridor list

# Artifact CAS operations
mez artifact store --type lawpack path/to/archive.zip
mez artifact resolve --type ruleset --digest abc123...
mez artifact verify --type schema --digest abc123...

# Verifiable Credential operations
mez vc keygen --output keys/ --prefix zone-admin
mez vc sign --key keys/zone-admin.priv.json document.json
mez vc verify --pubkey keys/zone-admin.pub.json document.json --signature abc...
```

---

## API server

The `mez-api` crate runs an Axum HTTP server with OpenAPI documentation.

```bash
cargo run -p mez-api
# Listening on 0.0.0.0:3000
# OpenAPI spec: GET /openapi.json
```

### Middleware stack

```
TraceLayer → MetricsMiddleware → AuthMiddleware → RateLimitMiddleware → Handler
```

Authentication is constant-time bearer token comparison (`subtle::ConstantTimeEq`). Rate limiting uses a per-route token bucket. Health probes (`/health/liveness`, `/health/readiness`) are unauthenticated.

### Route table

| Route | Domain | Source |
|-------|--------|--------|
| `POST/GET /v1/entities/*` | Mass Entities | Proxy via `mez-mass-client` |
| `POST/GET /v1/ownership/*` | Mass Ownership | Proxy via `mez-mass-client` |
| `POST/GET /v1/fiscal/*` | Mass Fiscal | Proxy via `mez-mass-client` |
| `POST/GET /v1/identity/*` | Mass Identity | Proxy via `mez-mass-client` |
| `POST/GET /v1/consent/*` | Mass Consent | Proxy via `mez-mass-client` |
| `POST/GET /v1/corridors/*` | EZ corridors | Native: lifecycle, receipts, forks |
| `POST /v1/settlement/*` | EZ settlement | Native: netting, SWIFT instructions |
| `POST/GET /v1/assets/*` | EZ smart assets | Native: registry, compliance eval |
| `POST /v1/credentials/*` | EZ credentials | Native: VC issuance, verification |
| `POST /v1/triggers` | EZ agentic | Native: policy trigger evaluation |
| `GET /v1/policies/*` | EZ agentic | Native: policy CRUD |
| `GET /v1/regulator/*` | EZ regulator | Native: compliance monitoring |
| `GET /health/liveness` | Probes | Always 200 |
| `GET /health/readiness` | Probes | Checks stores, signing key, locks |

### Example: Create a corridor

```bash
curl -X POST http://localhost:3000/v1/corridors \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "corridor_id": "PK-AE-001",
    "jurisdiction_a": "PK-REZ",
    "jurisdiction_b": "AE-DIFC",
    "agreement_digest": "abc123...",
    "pack_trilogy_digest": "def456..."
  }'
```

### Example: Evaluate compliance

```bash
curl http://localhost:3000/v1/assets/asset-001/compliance \
  -H "Authorization: Bearer $TOKEN"
# Returns: 20-domain compliance tensor slice with attestation references
```

---

## Zone composition

A zone is defined by a YAML file that selects jurisdictions and composes modules:

```yaml
# zone.yaml
zone_id: momentum.zone.nyc-de-adgm
name: "NYC-Delaware-ADGM Hybrid"

jurisdictions:
  civic: us-ny
  corporate: us-de
  financial: ae-adgm
  digital: ae-adgm

corridors:
  - swift-iso20022
  - stablecoin-usdc

arbitration:
  primary: difc-lcia
```

This generates legal infrastructure, regulatory frameworks, financial rails, corporate services, and dispute resolution from the 146 zone modules across 16 families.

### Module families

| Family | Count | Examples |
|--------|-------|---------|
| Legal | 9 | Enabling acts, court systems, property registry |
| Corporate | 8 | Formation, governance, beneficial ownership, dissolution |
| Regulatory | 8 | AML/CFT, sanctions, data protection, export controls |
| Licensing | 16 | Business permits, professional standards, insurance |
| Identity | 6 | KYC, DIDs, credentials, risk scoring |
| Financial | 14 | Payment rails, treasury, settlement, wire transfer |
| Capital Markets | 9 | Securities issuance, trading, clearing, DVP/PVP |
| Trade | 8 | Bills of lading, documentary credits, trade finance |
| Tax | 7 | Corporate tax, VAT/GST, withholding, transfer pricing |
| Corridors | 7 | SWIFT, correspondent banking, stablecoin settlement |
| Governance | 10 | Board operations, shareholder voting, audit |
| Arbitration | 8 | Dispute claims, evidence, hearings, enforcement |
| Operations | 9 | Monitoring, incident management, business continuity |
| Smart Assets | 1 | Smart asset infrastructure |
| Mass Primitives | 5 | Mass API module bindings |
| Template | 1 | Module template |

All modules live in `modules/` with YAML descriptors validated against `schemas/`.

---

## Deployment

### Docker Compose

```bash
cd deploy/docker
docker-compose up -d
```

Services: `mez-api` (Rust binary, port 8080), PostgreSQL 16, Prometheus, Grafana.

### Kubernetes

```bash
kubectl apply -f deploy/k8s/
```

Manifests: namespace, configmap, secret, deployment (2 replicas, rolling update, non-root security context, resource limits), service.

### AWS (Terraform)

```bash
cd deploy/aws/terraform
terraform apply -var-file=examples/hybrid-zone.tfvars
```

Provisions: EKS (auto-scaling), RDS PostgreSQL (Multi-AZ), ElastiCache Redis, S3, KMS, ALB with TLS.

---

## Schemas

116 JSON Schema files (Draft 2020-12) in `schemas/` define the public API surface:

- Corridor protocol (receipts, checkpoints, fork resolution, finality, routing)
- Verifiable Credentials (corridor anchors, lifecycle transitions, compliance attestations)
- Arbitration lifecycle (claims, evidence packages, orders, enforcement)
- Agentic automation (triggers, policies, action schedules, audit trails)
- Artifacts (content-addressed references, graph verification)
- Module and profile validation

---

## Specification

25 normative chapters in `spec/` define the protocol. Key chapters:

| Chapter | Topic |
|---------|-------|
| `02-invariants` | System invariants (fail-safe defaults, cryptographic integrity) |
| `11-architecture-overview` | Layered architecture |
| `12-mass-primitives-mapping` | Mass API integration contract |
| `17-agentic` | Agentic policy engine (Theorem 17.1: determinism) |
| `40-corridors` | Corridor protocol (receipt chains, fork resolution) |
| `80-security-privacy` | Security model and threat boundaries |
| `96-lawpacks` | Lawpack format (Akoma Ntoso legal corpus) |

---

## Design principles

| Principle | How it's enforced |
|-----------|-------------------|
| **Fail-safe defaults** | Unknown = `NonCompliant`. Missing attestations invalidate. System fails closed. |
| **Cryptographic integrity** | Every state transition produces proof. Tensor commitments are Merkle roots. Receipts form MMR chains. |
| **Type-level correctness** | Typestate machines. Sealed traits. Identifier newtypes. No `unwrap()`. |
| **Deterministic execution** | No floating point. No randomness in evaluation. BTreeMap iteration order. `CanonicalBytes` for all digests. |
| **Economic accountability** | Watchers bond collateral. Misbehavior triggers slashing. Fork resolution uses 3-level ordering. |
| **Privacy by design** | ZK proofs verify without disclosure. Selective tensor slices. Sealed proof backends prevent unauthorized verification. |
| **Defense in depth** | Constant-time auth comparison. Zeroize on key drop. Rate limiting after auth. Input validation at API boundary. |

---

## Repository structure

```
stack/
├── mez/                  Rust workspace (16 crates)
│   ├── Cargo.toml         Workspace manifest with centralized dependencies
│   └── crates/            All crate source
├── modules/               146 zone modules (16 families)
├── schemas/               116 JSON Schema files (Draft 2020-12)
├── spec/                  25 normative specification chapters
├── apis/                  OpenAPI 3.x specifications
├── deploy/                Docker, Kubernetes, Terraform
│   ├── docker/            docker-compose.yaml + Dockerfile
│   ├── k8s/               Kubernetes manifests
│   └── aws/terraform/     EKS + RDS + S3 + KMS
├── contexts/              Zone composition contexts
├── jurisdictions/         Zone configuration files
├── rulesets/              Regulatory rulesets
├── registries/            Live registries
├── dist/artifacts/        CAS-indexed built artifacts
├── governance/            Governance state machines
├── docs/                  Documentation
├── tools/                 Reference tooling (not shipped)
├── CLAUDE.md              Engineering instructions
├── CHANGELOG.md           Version history
└── VERSION                0.4.44-GENESIS
```

---

## Documentation

| Document | Path |
|----------|------|
| **Quickstart** | [`docs/getting-started.md`](./docs/getting-started.md) |
| **Architecture overview** | [`docs/architecture/OVERVIEW.md`](./docs/architecture/OVERVIEW.md) |
| **Crate reference** | [`docs/architecture/CRATE-REFERENCE.md`](./docs/architecture/CRATE-REFERENCE.md) |
| **Mass integration** | [`docs/architecture/MASS-INTEGRATION.md`](./docs/architecture/MASS-INTEGRATION.md) |
| **Security model** | [`docs/architecture/SECURITY-MODEL.md`](./docs/architecture/SECURITY-MODEL.md) |
| **Zone deployment** | [`docs/operators/ZONE-DEPLOYMENT-GUIDE.md`](./docs/operators/ZONE-DEPLOYMENT-GUIDE.md) |
| **Corridor formation** | [`docs/operators/CORRIDOR-FORMATION-GUIDE.md`](./docs/operators/CORRIDOR-FORMATION-GUIDE.md) |
| **Incident response** | [`docs/operators/INCIDENT-RESPONSE.md`](./docs/operators/INCIDENT-RESPONSE.md) |
| **Error taxonomy** | [`docs/ERRORS.md`](./docs/ERRORS.md) |
| **Traceability matrix** | [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) |
| **Spec-to-code mapping** | [`docs/traceability-matrix.md`](./docs/traceability-matrix.md) |
| **Attestation catalog** | [`docs/attestations/catalog.md`](./docs/attestations/catalog.md) |
| **Contributing** | [`CONTRIBUTING.md`](./CONTRIBUTING.md) |

---

<div align="center">

**[Momentum](https://momentum.inc)** · **[Mass](https://mass.inc)**

*Programmable institutions for durable economies.*

</div>
