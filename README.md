# Momentum SEZ Stack (MSEZ) — v0.4.41

This repository is a **reference specification + reference library** for building *programmable Special Economic Zones (SEZs)* as modular, forkable, and composable "jurisdiction nodes" in the Momentum/MASS network.

It is designed to function like an **open standard**: modular, versioned, testable, and implementable by multiple vendors.

> **Not legal advice.** This repository contains model texts, technical specs, and interoperability schemas. Deployments require local legal review, political authorization, and licensed financial/regulatory operators.

## What's inside

- **`spec/`** — the normative MSEZ Stack specification (how modules work, how profiles compose, conformance rules)
- **`modules/`** — reference modules (legal/regulatory/financial/corridor/etc.) with machine-readable sources
- **`profiles/`** — curated "base templates/styles" (bundles of modules + parameters)
- **`schemas/`** — JSON Schemas for manifests, attestations, corridors, arbitration, regpacks (104 schemas)
- **`rulesets/`** — content-addressed ruleset descriptors (pins verifier logic)
- **`apis/`** — OpenAPI specs for MASS integration + regulator console APIs
- **`tools/`** — build/validate/publish tools (reference implementations)
- **`registries/`** — registries of module IDs, jurisdiction IDs, corridor IDs

## Installation

### Requirements

- Python 3.10+ (tested on 3.12)
- pip package manager

### Quick Install

```bash
# Clone the repository
git clone https://github.com/momentum-sez/stack.git
cd stack

# Install dependencies
pip install -r tools/requirements.txt

# Verify installation
pytest -q

# Fetch Akoma Ntoso schema bundle (needed for legal document validation)
python -m tools.msez fetch-akoma-schemas
```

### Dependencies

Core dependencies (see `tools/requirements.txt` for full list):
- `jsonschema` — JSON Schema validation
- `pyyaml` — YAML parsing
- `cryptography` — Ed25519 signatures and key management
- `pynacl` — NaCl cryptographic operations
- `pytest` — test framework

## Quick start

```bash
# 1) validate a profile bundle (schema + semantic checks)
python -m tools.msez validate profiles/digital-financial-center/profile.yaml

# 2) validate a zone deployment (profile+corridors+overlays)
python -m tools.msez validate --zone jurisdictions/_starter/zone.yaml

# 3) build a reproducible zone bundle (applies overlays + renders templates)
python -m tools.msez build --zone jurisdictions/_starter/zone.yaml --out dist/
```

## Key Features (v0.4.41)

### Arbitration System (Chapter 26)

Complete programmatic dispute resolution infrastructure with support for DIFC-LCIA, SIAC, AIFC-IAC, and ICC institutions. Features include dispute filing protocol, evidence packages, ruling VCs with automatic enforcement, and πruling ZK circuit verification (~35K constraints).

### RegPack Integration (Chapter 20)

Dynamic regulatory state management including sanctions list integration (OFAC/EU/UN), license registry, compliance calendar, and regulator profile management.

### Smart Asset Receipt Chains

Non-blockchain state progression with cryptographic integrity guarantees per MASS Protocol v0.2. Supports offline operation (Theorem 16.1), identity immutability (Theorem 29.1), and receipt chain non-repudiation (Theorem 29.2).

### Agentic Execution Primitives (Chapter 17)

15 trigger types across regulatory, arbitration, corridor, and asset lifecycle domains. Policy framework with standard policies library for autonomous asset behavior.

## Version History

### v0.4.41 — Arbitration System + RegPack Integration (Current)

- **Arbitration System (Chapter 26)**: Institution registry, dispute filing protocol, ruling VCs with automatic enforcement, 9 arbitration transition kinds, πruling ZK circuit
- **RegPack Integration (Chapter 20)**: Dynamic regulatory state snapshots, sanctions list integration, license registry, compliance calendar
- **Agentic Execution Primitives (Chapter 17)**: 15 trigger types, policy framework, standard policies library
- **MASS Protocol Compliance**: Protocol 14.1/16.1/18.1, Theorems 16.1/29.1/29.2 verification functions
- **Test coverage**: 264 tests passing

### v0.4.40 — Production Operator Ergonomics

- Trade instrument kit (Invoice, Bill of Lading, Letter of Credit)
- Corridor-of-corridors settlement plans with deterministic netting
- Strict verification semantics and bughunt gates

### v0.4.39 — Cross-corridor Settlement Anchoring

- Settlement anchor schema + typed attachments
- Proof binding primitives
- Trade instrument lifecycle transitions

See `governance/CHANGELOG.md` for complete version history.

## Repository conventions

- **Normative keywords**: "MUST/SHOULD/MAY" are interpreted per RFC 2119 + RFC 8174 (see `spec/00-terminology.md`).
- **Versioning**: semver for modules and profiles; "stack spec" has its own version.
- **Content licensing**: see `LICENSES/` and per-module `module.yaml`.

## Status

**Current version:** v0.4.41 (January 2026)

**Next version gate (v0.4.42):** `docs/roadmap/ROADMAP_V042_AGENTIC_FRAMEWORK.md`

v0.4.42 will complete the Agentic Execution Framework with environment monitors, policy evaluation, and CLI commands.

## Development

```bash
pip install -r tools/requirements.txt
pytest -q

# fetch Akoma Ntoso schema bundle (needed for schema validation)
python -m tools.msez fetch-akoma-schemas

# validate all modules
python -m tools.msez validate --all-modules
```

## Contributing

See `CONTRIBUTING.md` for guidelines on submitting issues and pull requests.

## License

This project is licensed under the terms specified in `LICENSES/`. Individual modules may have additional licensing terms specified in their `module.yaml` files.
