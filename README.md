# Momentum EZ Stack

**Deploy jurisdictional infrastructure as code.**

Compliance, corridors, credentials, and capital flows — one Rust binary, one Postgres database, one command.

**v0.4.44** · BUSL-1.1

---

## Quick start

```bash
git clone https://github.com/momentum-ez/stack.git && cd stack/mez

cargo build --workspace               # build all 17 crates
cargo test  --workspace               # run ~4,700 tests
cargo clippy --workspace -- -D warnings  # zero warnings policy

cargo run -p mez-api                   # start API server on :3000
cargo run -p mez-cli -- validate --all-modules  # validate 323 modules
```

**Prerequisites:** Rust 1.75+, Git. Optional: Docker 24+, Terraform 1.5+.

### Deploy a zone

```bash
# Docker Compose: mez-api + Postgres + Prometheus + Grafana
cd deploy/docker && docker compose up -d

# Or use the deploy script (generates keys, secrets, spins up everything):
./deploy/scripts/deploy-zone.sh sovereign-govos-pk org.momentum.mez.zone.pk-sifc pk
```

See [docs/getting-started.md](./docs/getting-started.md) for the full walkthrough.

---

## What this does

The EZ Stack sits above [Mass](https://mass.inc) — Momentum's five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent). Mass handles CRUD. The EZ Stack adds:

- **Compliance evaluation** across 20 regulatory domains per jurisdiction
- **Cross-border corridors** with cryptographic receipt chains
- **Verifiable Credentials** (W3C, Ed25519) for compliance attestation
- **Zone deployment** with lawpacks, regpacks, and licensepacks per jurisdiction

Every write operation follows: compliance eval → Mass API call → VC issuance → attestation storage. Read operations pass through.

### Two deployment modes

| Mode | How it works |
|------|-------------|
| **Sovereign** (`SOVEREIGN_MASS=true`) | mez-api + Postgres. All data stays in-zone. |
| **Proxy** (`SOVEREIGN_MASS=false`) | mez-api proxies to centralized Mass APIs at mass.inc. |

---

## Architecture

```
mez-api          Axum HTTP server (50+ endpoints). Sole composition point.
mez-cli          Offline zone management, validation, signing.
mez-tensor       Compliance tensor (20 domains, Dijkstra manifold).
mez-corridor     Receipt chains, fork resolution, netting, trade flows.
mez-state        Typestate FSMs (corridor, entity, migration, watcher).
mez-pack         Lawpack/regpack/licensepack processing.
mez-vc           W3C Verifiable Credentials, Ed25519.
mez-crypto       Ed25519 (zeroize), MMR, CAS, SHA-256.
mez-agentic      Policy engine, 20 triggers, tax pipeline.
mez-arbitration  Dispute lifecycle, escrow, enforcement.
mez-compliance   Regpack → tensor bridge.
mez-schema       116 JSON Schemas (Draft 2020-12).
mez-mass-client  Typed HTTP client for Mass APIs.
mez-core         Foundation types, canonical bytes, digests. Zero internal deps.
```

---

## Deployment

### Docker Compose

```bash
# Single zone
cd deploy/docker && docker compose up -d
# → mez-api :8080, Postgres :5432, Prometheus :9090, Grafana :3000

# Two sovereign zones with corridor
docker compose -f docker-compose.two-zone.yaml up -d
```

### AWS (Terraform)

```bash
cd deploy/aws/terraform
terraform init
terraform apply -var-file=zone.tfvars
# → EKS + RDS (Multi-AZ) + KMS + S3 + ALB/TLS
```

### Kubernetes

```bash
kubectl apply -f deploy/k8s/
# → 2 replicas, rolling updates, non-root, resource limits, probes
```

---

## Repository layout

```
stack/
├── mez/                   Rust workspace (17 crates)
│   ├── Cargo.toml         Workspace manifest
│   └── crates/            All crate source
├── modules/               323 zone modules (16 families)
├── schemas/               116 JSON Schemas (Draft 2020-12)
├── spec/                  24 normative spec chapters
├── apis/                  OpenAPI specifications
├── jurisdictions/         210 zone definitions
├── deploy/                Docker, Kubernetes, Terraform
│   ├── docker/            Compose files + Dockerfile
│   ├── k8s/               K8s manifests
│   ├── aws/terraform/     EKS + RDS + KMS
│   └── scripts/           Deploy and demo scripts
├── governance/            Lifecycle state machines, changelog
└── docs/                  Architecture, guides, reference
```

---

## Documentation

| Document | Path |
|----------|------|
| Getting started | [docs/getting-started.md](./docs/getting-started.md) |
| Zone bootstrap | [docs/ZONE-BOOTSTRAP-GUIDE.md](./docs/ZONE-BOOTSTRAP-GUIDE.md) |
| Architecture | [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) |
| Deployment roadmap | [docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md](./docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md) |
| Error taxonomy | [docs/ERRORS.md](./docs/ERRORS.md) |
| Spec traceability | [docs/traceability-matrix.md](./docs/traceability-matrix.md) |
| Crate reference | [docs/architecture/CRATE-REFERENCE.md](./docs/architecture/CRATE-REFERENCE.md) |
| Security model | [docs/architecture/SECURITY-MODEL.md](./docs/architecture/SECURITY-MODEL.md) |
| Specification | [spec/](./spec/) |

---

## License

[Business Source License 1.1](./LICENSE.md)

**[Momentum](https://momentum.inc)** · **[Mass](https://mass.inc)**
