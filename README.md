# Stack

Deploy a programmable economic zone on the
[Mass](https://momentum.inc/mass) infrastructure. Fork, configure, launch.

Stack is the deployment kit for the four-repo open-source set:
[Lex](https://github.com/momentum-sez/lex) (typed jurisdictional rules) +
[Op](https://github.com/momentum-sez/op) (typed compliance-carrying
workflows) + [gstore](https://github.com/momentum-sez/gstore)
(Merkle-authenticated proof persistence) compose into a runtime that this
repository configures, deploys, and operates as a sovereign economic zone.
Fork this repo, edit `zone.yaml`, run `make up`.

## Prerequisites

- Docker and Docker Compose
- A Mass API key ([request access](https://momentum.inc/builders))

## Quick Start

```bash
# Fork and clone
git clone https://github.com/momentum-sez/stack.git your-zone && cd your-zone

# Configure
cp .env.example .env
$EDITOR .env                    # Set AUTH_TOKEN and POSTGRES_PASSWORD

# Launch
make up

# Verify
curl http://localhost:8080/health/liveness
# {"status":"ok","zone":"org.momentum.mez.zone.example","version":"0.4.44"}

# Run your first operation
curl -X POST http://localhost:8080/v1/operations \
  -H "Authorization: Bearer ${AUTH_TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "operation": "entity.incorporate",
    "params": {
      "legal_name": "Test Company Ltd",
      "entity_type": "company",
      "incorporator_name": "Jane Doe",
      "incorporator_email": "jane@example.com"
    }
  }'
```

## What This Is

This repository is a **zone configuration template**. It contains:

- **zone.yaml** — Your zone's identity, compliance domains, and legal framework
- **operations/** — YAML workflows executed by the kernel (entity formation, payments, KYC, etc.)
- **deploy/** — Docker Compose to run the kernel locally or in production

The **kernel** is the Rust runtime published as a Docker image. It reads your
YAML, evaluates compliance, and proxies operations to Mass services. You
configure it — you don't build it.

## How It Works

```
┌─────────────────────────────────────┐
│  This Repo (your zone config)       │
│  zone.yaml · operations/*.yaml      │
└──────────────┬──────────────────────┘
               │ mounted as /zone
               ▼
┌─────────────────────────────────────┐
│  Kernel (mez-api Docker image)      │
│  Reads YAML at startup              │
│  Evaluates compliance tensor        │
│  Executes operation step DAGs       │
│  Issues verifiable credentials      │
└──────────────┬──────────────────────┘
               │ HTTP calls
               ▼
┌─────────────────────────────────────┐
│  Mass Services (hosted by Momentum) │
│  organization-info  treasury-info   │
│  identity-info  governance-info     │
│  investment-info  templating-engine │
└─────────────────────────────────────┘
```

The kernel connects to Mass services over HTTP.

## Repository Layout

```
zone.yaml               Zone identity and compliance configuration
operations/
  entity/
    incorporate.yaml     Company formation (4-step DAG)
  fiscal/
    payment.yaml         Payment with sanctions screening
  identity/
    verify.yaml          KYC verification
  ownership/
    issue-shares.yaml    Share issuance with board consent
  consent/
    resolution.yaml      Governance resolution
lawpacks/                Legal corpus (Akoma Ntoso XML, SHA-256 pinned)
corridors/
  corridors.yaml         Peer zone connections
adapters/                National system integration configs
deploy/
  docker-compose.yaml    Docker Compose deployment
schemas/
  zone.schema.json       JSON Schema for zone.yaml validation
  operation.schema.json  JSON Schema for operation YAML validation
examples/
  digital-free-zone/     8 compliance domains, lightest footprint
  financial-center/      23 domains, full regulatory stack
  charter-city/          12 domains, greenfield jurisdiction
  trade-zone/            10 domains, ports and customs
```

## Zone Configuration

Edit `zone.yaml`. The required fields:

| Field | Description | Example |
|-------|-------------|---------|
| `zone_id` | Unique zone identifier | `org.momentum.mez.zone.sc` |
| `jurisdiction_id` | Short code (ISO 3166-1 or composite) | `sc`, `ae-dubai-difc` |
| `zone_name` | Human-readable name | `Seychelles Sovereign Zone` |
| `profile.profile_id` | Baseline profile | See [Profiles](#profiles) |
| `compliance_domains` | Which of 23 domains apply | `[aml, kyc, sanctions, corporate]` |

### Profiles

| Profile | Domains | Use Case |
|---------|---------|----------|
| `minimal-mvp` | 4 | Development and testing |
| `digital-native-free-zone` | 8 | Digital businesses, light regulation |
| `digital-financial-center` | 23 | Financial services, full regulatory |
| `charter-city` | 12 | Greenfield jurisdictions |
| `trade-playbook` | 10 | Import/export, customs, ports |

### Compliance Domains

The kernel evaluates applicable domains on every mutating operation. Select
the domains relevant to your jurisdiction:

| Domain | What It Covers |
|--------|---------------|
| `aml` | Transaction monitoring, suspicious activity reporting |
| `anti_bribery` | Anti-corruption controls, gifts, facilitation payments |
| `arbitration` | Dispute resolution frameworks |
| `banking` | Reserve requirements, capital adequacy |
| `clearing` | CCP rules, netting, settlement finality |
| `consumer_protection` | Disclosure, warranties, dispute resolution |
| `corporate` | Formation, dissolution, beneficial ownership |
| `custody` | Asset safekeeping, segregation |
| `data_privacy` | GDPR, PDPA, cross-border data transfer |
| `digital_assets` | Token classification, exchange licensing |
| `employment` | Labor contracts, social security |
| `immigration` | Work permits, visa sponsorship |
| `insurance` | Insurance licensing, solvency, policyholder protections |
| `ip` | Patent, trademark, trade secret |
| `kyc` | Identity verification, due diligence |
| `licensing` | Business license validity, renewals |
| `payments` | PSP licensing, payment instrument rules |
| `sanctions` | Applicable sanctions-list screening and escalation handling |
| `securities` | Issuance, trading, disclosure |
| `settlement` | DvP, settlement cycles |
| `sharia` | Sharia compliance where applicable |
| `tax` | Withholding, reporting, filing |
| `trade` | Import/export controls, customs, tariffs |

Verdicts form a lattice: `NonCompliant < Pending < NotApplicable < Exempt < Compliant`.
Missing evidence = `Pending`. Corridor composition takes the minimum (most restrictive) per domain.

## Operations

Operations are YAML files describing multi-step workflows. Each step names
a Mass service, an API endpoint, compliance domains, and dependencies on
other steps. The kernel compiles them into a DAG and executes in order.

### Authoring an Operation

Create `operations/{primitive}/your-operation.yaml`:

```yaml
operation: entity.register-fund
jurisdiction: _default
version: "1.0"
description: Register an investment fund
legal_basis: Investment Funds Act 2024

params:
  required:
    - name: entity_name
      type: string
    - name: manager_id
      type: string

steps:
  - id: verify-manager
    service: identity-info
    api:
      method: POST
      path: /identity-info/api/v1/identities/verify
    params:
      subject_id: "{params.manager_id}"
    compliance_domains: [kyc, aml]
    on_failure: cancel_operation

  - id: create-fund
    service: organization-info
    depends_on: [verify-manager]
    api:
      method: POST
      path: /organization-info/api/v1/organization/create
    params:
      name: "{params.entity_name}"
      entityType: fund
    compliance_domains: [corporate, securities]
```

### Variable Interpolation

Steps can reference:

| Variable | Source | Example |
|----------|--------|---------|
| `{params.name}` | Operation input parameters | `{params.entity_name}` |
| `{steps.step_id.result.field}` | Output from a completed step | `{steps.create-fund.result.id}` |
| `{operation_id}` | Auto-generated operation UUID | — |
| `{initiator_email}` | Email from the caller's auth token | — |
| `{current_year}` | Current calendar year | `2026` |

### Jurisdiction Overrides

Place files named by jurisdiction to override the default:

```
operations/entity/
  _default.yaml          # Fallback
  ae-dubai-difc.yaml     # DIFC-specific (adds name reservation step)
  sg.yaml                # Singapore (adds ACRA filing step)
```

The kernel resolves: jurisdiction-specific first, then `_default`.

## Deployment

Four topologies, depending on what you need:

| Topology | Command | What It Runs | When To Use |
|----------|---------|-------------|-------------|
| **Standalone** | `make up` | Kernel + PostgreSQL | Simplest. Mass services hosted by Momentum. |
| **Full Stack** | `make full-stack` | Kernel + 7 Java services + 2× PostgreSQL + Redis | Run all Mass primitives locally. |
| **Kernel** | `make kernel` | Kernel + 7 Java services + 1× PostgreSQL (unified) | Single DB, write enforcement. Production. |
| **Two-Zone Corridor** | `make corridor` | 2× Kernel + 2× PostgreSQL + corridor network | Test cross-zone operations. |

```bash
# Standalone
make up
make down

# Full Stack
make full-stack
make full-stack-down

# Kernel Topology
make kernel
make kernel-down

# Two-Zone Corridor
make corridor
make corridor-down

# Common
make logs       # Follow kernel logs
make status     # Health check
make validate   # Schema-check zone.yaml and operations
```

### Standalone

Kernel + PostgreSQL. The kernel connects to Mass services hosted by Momentum over HTTP. Set `MASS_API_KEY` in `.env`.

### Full Stack

Runs the complete Mass infrastructure locally: organization-info, treasury-info, governance-info, investment-info, identity-info, templating-engine, attestation-engine. Two PostgreSQL instances (one for the kernel, one for Java services). Requires `MASS_POSTGRES_PASSWORD` in `.env`.

### Kernel Topology

Unified single-database architecture. One PostgreSQL instance owned by the kernel (read-write). Java services connect as read-only and delegate writes back through the kernel's internal API. Requires `KERNEL_INTERNAL_TOKEN` in `.env`.

### Two-Zone Corridor

Two independent zones (ports 8080 and 8081) with separate databases, connected via a corridor network for cross-zone operations. Databases are unreachable across zone boundaries. Set `ZONE_A_*` and `ZONE_B_*` variables in `.env`.

## Extending

### Add a Corridor

1. Add the peer to `zone.yaml` under `corridor_peers`
2. Set the peer's endpoint URL in `.env`
3. Restart: `make down && make up`

### Add a National Adapter

1. Add the adapter to `zone.yaml` under `national_adapters`
2. Set the endpoint URL and API key in `.env`
3. Restart

### Upgrade the Kernel

1. Update `MEZ_VERSION` in `.env` (or `docker-compose.yaml`)
2. Check [CHANGELOG.md](CHANGELOG.md) for breaking changes
3. Run `make validate` to verify configuration compatibility
4. `make down && make up`

## Relation to Lex, Op, gstore

Stack is the operational endpoint of the four-repo open-source set. The
typed boundaries between the repositories are stable:

- **[Lex](https://github.com/momentum-sez/lex)** — the rule and proof
  layer. A jurisdictional rule (e.g. "ADGM SPV minimum directors") is a
  Lex term; the type checker accepts it; the obligation extractor produces
  proof obligations; the certificate builder issues a signed verdict.
- **[Op](https://github.com/momentum-sez/op)** — the workflow layer. An
  operation (e.g. `entity.incorporate`) is an Op program: a typed DAG of
  steps with effect rows, contracts, and compensation. The Lex obligations
  appear as `requires` / `ensures` contracts on Op steps.
- **[gstore](https://github.com/momentum-sez/gstore)** — the persistence
  layer. The discharge certificates and Op proof bundles produced at
  runtime land as typed nodes in gstore's authenticated graph; the
  gstore Merkle Mountain Range root is anchored to public chains on a
  recurring cadence.
- **Stack (this repository)** — the configuration and deployment layer.
  `zone.yaml` declares which jurisdictional profile applies; `operations/`
  declares the Op programs the zone exposes; `deploy/` runs the runtime
  that loads them.

Each layer is published, versioned, and replayable independently. A cold
clone of Stack with `make up` produces a running zone whose every operation
can be replayed by any peer that has the same Stack configuration, the same
Lex rule digest, and the same Op program digest.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Security

See [SECURITY.md](SECURITY.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
