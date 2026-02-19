# Getting started

Build, test, run the API server, use the CLI.

---

## Prerequisites

| Tool | Version | Required |
|------|---------|----------|
| Rust | 1.75+ | Yes |
| Git | 2.30+ | Yes |
| Docker | 24+ | For containerized deployment |
| kubectl | 1.28+ | For Kubernetes deployment |
| Terraform | 1.5+ | For AWS provisioning |

---

## Build and test

```bash
git clone https://github.com/momentum-ez/stack.git
cd stack/mez

cargo build --workspace                     # build all 16 crates
cargo test  --workspace                     # run 4,073 tests
cargo clippy --workspace -- -D warnings     # zero warnings policy
cargo doc --workspace --no-deps --open      # generate rustdoc
```

---

## Repository layout

| Directory | Contents |
|-----------|----------|
| `mez/` | Rust workspace — 16 crates, 151K lines |
| `modules/` | 323 zone modules across 16 families |
| `schemas/` | 116 JSON Schema files (Draft 2020-12) |
| `spec/` | 24 normative specification chapters |
| `apis/` | OpenAPI 3.x specifications |
| `deploy/` | Docker, Kubernetes, Terraform manifests |
| `contexts/` | Zone composition contexts |
| `jurisdictions/` | 100 zone definitions (US states, UAE free zones, PK, CN, etc.) |
| `dist/artifacts/` | Content-addressed built artifacts |
| `governance/` | Lifecycle state machines, changelog |
| `docs/` | This documentation |

---

## Core concepts

### Zones

A **zone** is a deployable Economic Zone defined by a `zone.yaml` file. It selects which jurisdictions provide legal, regulatory, and financial capabilities, and composes modules to generate the complete operational substrate.

### Modules

**Modules** are building blocks. Each implements one capability — entity formation, AML screening, SWIFT settlement, license issuance, etc. 323 modules across 16 families: legal, corporate, regulatory, licensing, identity, financial, capital markets, trade, tax, corridors, governance, arbitration, operations, smart assets, mass primitives, and template.

### Corridors

**Corridors** are bilateral trade channels between jurisdictions. They manage receipt chains (MMR-backed), fork detection and resolution, settlement netting, and optional L1 anchoring.

### Compliance Tensor

The **Compliance Tensor** evaluates an entity's regulatory state across 20 domains (AML, KYC, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy, Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets, Employment, Immigration, IP, ConsumerProtection, Arbitration, Trade) per jurisdiction, producing a 5-state lattice value per domain: `Compliant`, `Pending`, `NonCompliant`, `Exempt`, `NotApplicable`.

### Pack Trilogy

Three content-addressed configuration packs define jurisdictional rules:
- **Lawpacks** — Akoma Ntoso XML statutes (enabling acts, tax law, corporate law)
- **Regpacks** — sanctions lists, reporting obligations, compliance calendars
- **Licensepacks** — license types, issuing authorities, validity periods

### Mass APIs

The five **Mass primitives** (Entities, Ownership, Fiscal, Identity, Consent) are live API services operated by [Mass](https://mass.inc). The EZ Stack orchestrates them through `mez-mass-client` — the sole authorized gateway.

---

## First steps

### 1. Validate all modules

```bash
cargo run -p mez-cli -- validate --all-modules
```

Validates all 323 module YAML descriptors against their JSON Schemas and verifies artifact references.

### 2. Validate a zone

```bash
cargo run -p mez-cli -- validate jurisdictions/_starter/zone.yaml
```

### 3. Generate a lockfile

```bash
cargo run -p mez-cli -- lock jurisdictions/_starter/zone.yaml
```

The lockfile (`stack.lock`) pins every module, artifact, and dependency by SHA-256 digest for reproducible deployments.

### 4. Verify lockfile integrity

```bash
cargo run -p mez-cli -- lock jurisdictions/_starter/zone.yaml --check
```

### 5. Start the API server

```bash
cargo run -p mez-api
# Listening on 0.0.0.0:3000
# OpenAPI spec at http://localhost:3000/openapi.json
```

The API server exposes corridor operations, smart asset management, compliance evaluation, settlement, VC issuance, agentic policy triggers, and regulator queries. Mass primitive routes (`/v1/entities/*`, `/v1/fiscal/*`, etc.) proxy through `mez-mass-client`.

### 6. Generate Ed25519 keys

```bash
cargo run -p mez-cli -- vc keygen --output keys/ --prefix dev
# Creates keys/dev.priv.json and keys/dev.pub.json
```

### 7. Sign and verify documents

```bash
cargo run -p mez-cli -- vc sign --key keys/dev.priv.json document.json
cargo run -p mez-cli -- vc verify --pubkey keys/dev.pub.json document.json --signature abc...
```

### 8. Run individual crate tests

```bash
cargo test -p mez-corridor
cargo test -p mez-tensor
cargo test -p mez-agentic
cargo test -p mez-integration-tests
cargo test -p mez-crypto -- --nocapture     # with output
```

---

## Configuration

### API server environment variables

| Variable | Default | Purpose |
|----------|---------|---------|
| `MEZ_PORT` | `3000` | HTTP listen port |
| `MEZ_AUTH_TOKEN` | *(required)* | Bearer token for authenticated routes |
| `MASS_API_TOKEN` | *(required)* | Authentication token for Mass APIs |
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
#   mez-api     -> port 8080
#   postgres    -> port 5432
#   prometheus  -> port 9090
#   grafana     -> port 3000
```

---

## Next steps

| Topic | Document |
|-------|----------|
| System design | [Architecture Overview](./architecture/OVERVIEW.md) |
| Per-crate API | [Crate Reference](./architecture/CRATE-REFERENCE.md) |
| Security model | [Security Model](./architecture/SECURITY-MODEL.md) |
| Mass integration | [Mass Integration](./architecture/MASS-INTEGRATION.md) |
| Module authoring | [Module Authoring](./authoring/modules.md) |
| Error codes | [Error Taxonomy](./ERRORS.md) |
| Spec-to-code mapping | [Traceability Matrix](./traceability-matrix.md) |
