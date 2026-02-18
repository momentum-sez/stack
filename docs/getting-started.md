# Getting started

Build the workspace, run tests, start the API server, use the CLI.

---

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | 1.75+ | Workspace: 16 crates, API server, CLI |
| **Git** | 2.30+ | Clone and contribute |

Optional for deployment:

| Tool | Version | Purpose |
|------|---------|---------|
| **Docker** | 24+ | Container deployment |
| **kubectl** | 1.28+ | Kubernetes deployment |
| **Terraform** | 1.5+ | AWS infrastructure provisioning |

---

## Clone and build

```bash
git clone https://github.com/momentum-ez/stack.git
cd stack/mez

# Build all 16 crates
cargo build --workspace

# Run the full test suite (2,580+ tests)
cargo test --workspace

# Lint with zero warnings
cargo clippy --workspace -- -D warnings

# Generate rustdoc
cargo doc --workspace --no-deps --open
```

---

## Repository layout

| Directory | Purpose |
|-----------|---------|
| `mez/` | **Rust workspace** -- 16 crates implementing the protocol |
| `modules/` | 146 zone modules across 16 families |
| `schemas/` | 116 JSON Schema files (Draft 2020-12) |
| `spec/` | 25 normative specification chapters |
| `apis/` | OpenAPI 3.x specifications |
| `deploy/` | Docker, Kubernetes, Terraform manifests |
| `contexts/` | Zone composition contexts |
| `jurisdictions/` | Zone configuration files |
| `dist/artifacts/` | CAS-indexed built artifacts |
| `governance/` | Governance state machines |
| `docs/` | This documentation |

---

## Core concepts

### Zones

A **zone** is a deployable Economic Zone defined by a `zone.yaml` file. It selects which jurisdictions provide which legal, regulatory, and financial capabilities.

### Modules

**Modules** are the building blocks. Each module implements one capability (entity formation, AML screening, SWIFT settlement, etc.). 146 modules across 16 families.

### Corridors

**Corridors** are bilateral trade channels between jurisdictions. They manage settlement, receipt chains (MMR-backed), fork resolution, and optional L1 anchoring.

### Compliance Tensor

The **Compliance Tensor** evaluates an entity's compliance state across 20 regulatory domains (AML, KYC, Sanctions, Tax, Securities, etc.) per jurisdiction. It produces a 5-state lattice value per domain: `Compliant`, `Pending`, `NonCompliant`, `Exempt`, `NotApplicable`.

### Smart Assets

A **Smart Asset** is an asset with embedded compliance intelligence. It carries a compliance tensor, can identify missing attestations, and migrates across jurisdictions via the compliance manifold.

### Mass APIs

The five **Mass primitives** (Entities, Ownership, Fiscal, Identity, Consent) are live API services operated by Mass. The EZ Stack orchestrates these primitives through the `mez-mass-client` crate -- it never stores primitive data directly.

---

## First steps

### 1. Validate all modules

```bash
cargo run -p mez-cli -- validate --all-modules
```

Validates all 146 module YAML descriptors against their JSON schemas and verifies artifact references.

### 2. Validate a zone

```bash
cargo run -p mez-cli -- validate jurisdictions/_starter/zone.yaml
```

### 3. Generate a lockfile

```bash
cargo run -p mez-cli -- lock jurisdictions/_starter/zone.yaml
```

The lockfile (`stack.lock`) contains cryptographic hashes of every module, ensuring reproducible deployments.

### 4. Check lockfile integrity

```bash
cargo run -p mez-cli -- lock jurisdictions/_starter/zone.yaml --check
```

### 5. Start the API server

```bash
cargo run -p mez-api
# Listening on 0.0.0.0:3000
# OpenAPI spec at http://localhost:3000/openapi.json
```

The API server exposes corridor operations, smart asset management, compliance evaluation, settlement, VC issuance, agentic policy triggers, and regulator queries. Mass primitive routes (`/v1/entities/*`, `/v1/fiscal/*`, etc.) proxy through to the live Mass APIs via `mez-mass-client`.

### 6. Generate Ed25519 keys

```bash
cargo run -p mez-cli -- vc keygen --output keys/ --prefix dev
# Creates keys/dev.priv.json and keys/dev.pub.json
```

### 7. Sign a document

```bash
cargo run -p mez-cli -- vc sign --key keys/dev.priv.json document.json
```

### 8. Run individual crate tests

```bash
# Run tests for a specific crate
cargo test -p mez-corridor
cargo test -p mez-tensor
cargo test -p mez-agentic

# Run integration tests
cargo test -p mez-integration-tests

# Run with output
cargo test -p mez-crypto -- --nocapture
```

---

## Configuration

### API server environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEZ_PORT` | `3000` | HTTP listen port |
| `MEZ_AUTH_TOKEN` | *(none)* | Bearer token for authenticated routes |
| `MASS_API_TOKEN` | *(none)* | Authentication token for Mass APIs |
| `MASS_ORG_INFO_URL` | `https://organization-info.api.mass.inc` | Mass Entities endpoint |
| `MASS_TREASURY_INFO_URL` | `https://treasury-info.api.mass.inc` | Mass Fiscal endpoint |
| `MASS_CONSENT_INFO_URL` | `https://consent.api.mass.inc` | Mass Consent endpoint |
| `MASS_INV_INFO_URL` | *(heroku)* | Mass Ownership endpoint |
| `MASS_TEMPLATING_URL` | *(heroku)* | Mass Templating endpoint |
| `RUST_LOG` | `info` | Log level (tracing) |

---

## Docker quickstart

```bash
cd deploy/docker
docker-compose up -d

# Services:
#   mez-api    → port 8080
#   postgres    → port 5432
#   prometheus  → port 9090
#   grafana     → port 3000
```

---

## Next steps

| Topic | Document |
|-------|----------|
| Architecture deep dive | [docs/architecture/OVERVIEW.md](./architecture/OVERVIEW.md) |
| Per-crate API reference | [docs/architecture/CRATE-REFERENCE.md](./architecture/CRATE-REFERENCE.md) |
| Security model | [docs/architecture/SECURITY-MODEL.md](./architecture/SECURITY-MODEL.md) |
| Zone deployment | [docs/operators/ZONE-DEPLOYMENT-GUIDE.md](./operators/ZONE-DEPLOYMENT-GUIDE.md) |
| Corridor formation | [docs/operators/CORRIDOR-FORMATION-GUIDE.md](./operators/CORRIDOR-FORMATION-GUIDE.md) |
| Module authoring | [docs/authoring/modules.md](./authoring/modules.md) |
| Error codes | [docs/ERRORS.md](./ERRORS.md) |
| Spec-to-code mapping | [docs/traceability-matrix.md](./traceability-matrix.md) |
