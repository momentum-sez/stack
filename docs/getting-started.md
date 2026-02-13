# Getting Started

This guide walks you through setting up the MSEZ Stack, running your first validation, and understanding the core concepts.

---

## Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| **Rust** | 1.75+ | Crate workspace: core protocol, API server, CLI |
| **Python** | 3.10+ | PHOENIX layer, reference CLI, test suite |
| **Git** | 2.30+ | Repository management |

---

## Installation

```bash
git clone https://github.com/momentum-sez/stack.git
cd stack
```

### Rust Workspace

```bash
cd msez

# Build all 14 crates
cargo build --workspace

# Verify: run the full test suite (2,580+ tests)
cargo test --workspace

# Verify: zero clippy warnings
cargo clippy --workspace -- -D warnings
```

### Python Toolchain

```bash
cd stack  # repository root
pip install -r tools/requirements.txt

# Verify: validate all modules
python -m tools.msez validate --all-modules
```

---

## Repository Layout

The MSEZ Stack is structured as a **specification + library**:

| Directory | Purpose |
|-----------|---------|
| `spec/` | **Normative**: 25 specification chapters defining how the system works |
| `schemas/` | **Contracts**: 116 JSON schemas (Draft 2020-12) defining the public API surface |
| `modules/` | **Building blocks**: 146 reusable zone modules across 16 families |
| `msez/` | **Rust workspace**: 14 crates implementing the core protocol (70K lines) |
| `tools/` | **Python toolchain**: Reference CLI + PHOENIX execution layer |
| `tests/` | **Conformance**: 294 Python tests validating spec compliance |
| `deploy/` | **Infrastructure**: Docker, Terraform, Kubernetes manifests |
| `governance/` | **State machines**: Corridor lifecycle definitions |

---

## Core Concepts

### Zones

A **zone** is a deployable Special Economic Zone defined by a `zone.yaml` file. It specifies which jurisdictions provide which legal/regulatory/financial capabilities.

### Modules

**Modules** are the building blocks. Each module implements one capability (e.g., "AML screening", "entity formation", "SWIFT settlement"). Modules are organized into 16 families.

### Profiles

**Profiles** are pre-configured bundles of modules for common use cases:
- `digital-financial-center` — Fintech, digital assets, modern financial services
- `trade-playbook` — International trade, logistics, supply chain finance
- `charter-city` — Comprehensive civic infrastructure for physical zones

### Corridors

**Corridors** are bilateral trade channels between jurisdictions. They handle settlement, receipts, fork resolution, and L1 anchoring.

### Smart Assets

**Smart Assets** are assets with embedded compliance intelligence. They carry a Compliance Tensor (4D sparse structure) that tracks compliance state across jurisdictions, domains, and time.

---

## First Steps

### 1. Validate All Modules

```bash
# Python CLI
python -m tools.msez validate --all-modules

# Rust CLI
cargo run -p msez-cli -- validate --all-modules
```

This validates all 146 module descriptors against their JSON schemas and verifies artifact references.

### 2. Validate a Zone

```bash
python -m tools.msez validate --all-zones
```

### 3. Generate a Lockfile

```bash
python -m tools.msez lock jurisdictions/_starter/zone.yaml
```

The lockfile (`stack.lock`) contains cryptographic hashes of every module, ensuring reproducible deployments.

### 4. Check Lockfile Integrity

```bash
python -m tools.msez lock jurisdictions/_starter/zone.yaml --check
```

### 5. Start the API Server

```bash
cd msez
cargo run -p msez-api
# Listening on 0.0.0.0:3000
# OpenAPI spec at http://localhost:3000/openapi.json
```

The API server exposes five programmable primitives: Entities, Ownership, Fiscal, Identity, and Consent.

### 6. Run the Test Suite

```bash
# Rust tests (2,580+)
cd msez && cargo test --workspace

# Python tests (294)
cd stack && pytest tests/ -q
```

---

## How to Adopt

1. **Choose a profile** from `profiles/` or compose your own
2. **Create a jurisdiction folder** at `jurisdictions/<your-id>/`
3. **Add overlays** at `jurisdictions/<your-id>/overlays/` rather than editing upstream modules
4. **Generate lockfile**: `python -m tools.msez lock jurisdictions/<your-id>/zone.yaml`
5. **Commit** `zone.yaml` + `stack.lock`
6. **Deploy** using Docker, Kubernetes, or Terraform (see `deploy/`)

---

## Next Steps

| Topic | Document |
|-------|----------|
| System architecture | [docs/ARCHITECTURE.md](./ARCHITECTURE.md) |
| Security model | [docs/architecture/SECURITY-MODEL.md](./architecture/SECURITY-MODEL.md) |
| Creating modules | [docs/authoring/modules.md](./authoring/modules.md) |
| Forming corridors | [docs/authoring/corridors.md](./authoring/corridors.md) |
| Deploying a zone | [docs/operators/ZONE-DEPLOYMENT-GUIDE.md](./operators/ZONE-DEPLOYMENT-GUIDE.md) |
| Incident response | [docs/operators/INCIDENT-RESPONSE.md](./operators/INCIDENT-RESPONSE.md) |
| Error codes | [docs/ERRORS.md](./ERRORS.md) |
| Rust crate details | [msez/README.md](../msez/README.md) |
| Spec-to-code mapping | [docs/traceability-matrix.md](./traceability-matrix.md) |
