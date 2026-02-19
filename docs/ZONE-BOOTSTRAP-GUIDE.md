# Zone Bootstrap Guide

How to deploy a new MEZ economic zone from scratch using the `mez` CLI toolchain.

## Prerequisites

- Rust toolchain (1.75+)
- Docker and Docker Compose
- OpenSSL (for credential generation)
- PostgreSQL 16+ (via Docker or host-installed)

## Quick Start: Pakistan SIFC Zone

```bash
# 1. Build the CLI
cargo build --release --bin mez

# 2. Generate Ed25519 zone signing key
mez vc keygen --output keys/ --prefix pk-sifc

# 3. Build and store regpack artifacts (CAS)
mez regpack build --jurisdiction pk --all-domains --store

# 4. Generate the deterministic lockfile
mez lock jurisdictions/pk-sifc/zone.yaml

# 5. Deploy the zone
./deploy/scripts/deploy-zone.sh sovereign-govos org.momentum.mez.zone.pk-sifc pk
```

## Step-by-Step Walkthrough

### 1. Choose a Profile

Profiles define the module composition for a zone type:

| Profile | Use Case |
|---|---|
| `minimal-mvp` | Bare minimum: legal core, identity, payment adapter |
| `digital-financial-center` | Financial free zone: DFC modules, corridors, settlement |
| `digital-native-free-zone` | Digital-first free zone with enhanced smart asset support |
| `trade-playbook` | Trade finance corridor: BoL, invoice, LC, SWIFT |
| `charter-city` | Full charter city governance: legal codes, admin, land registry |
| `sovereign-govos` | National deployment: tax, multi-regulator, national system adapters |

For the Pakistan SIFC pilot, use `sovereign-govos`.

### 2. Prepare Zone Configuration

Each zone needs a `zone.yaml` in `jurisdictions/<jurisdiction-id>/`:

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
    regpack_digest_sha256: "444ddded8419d9dedf8344a54063d7cd80c0148338c78bbe77a47baa44dd392f"
    as_of_date: "2026-01-15"
  - jurisdiction_id: pk
    domain: sanctions
    regpack_digest_sha256: "e59056a2b9bdbf3e452857b1cbdc06b5cdff3e29f56de1e235475e8a4a57506f"
    as_of_date: "2026-01-15"

corridors:
  - org.momentum.mez.corridor.swift.iso20022-cross-border
```

### 3. Build Regpack Artifacts

Regpacks are content-addressed regulatory data snapshots. Build them into CAS:

```bash
# Build and store all Pakistan regpacks
mez regpack build --jurisdiction pk --all-domains --store

# Output:
#   jurisdiction: pk
#   domain:       financial
#   digest:       444ddded...
#   stored:       dist/artifacts/regpack/444ddded...json
#
#   jurisdiction: pk
#   domain:       sanctions
#   digest:       e59056a2...
#   stored:       dist/artifacts/regpack/e59056a2...json
```

Update `zone.yaml` with the computed digest values if they differ.

### 4. Generate Zone Signing Key

Each zone needs an Ed25519 key pair for signing VCs and corridor receipts:

```bash
mez vc keygen --output keys/ --prefix pk-sifc

# Output:
#   Private key: keys/pk-sifc-private.key
#   Public key:  keys/pk-sifc-public.pub
#   JWK:         keys/pk-sifc.jwk
```

The signing key hex is extracted from the JWK `d` field and set via
`ZONE_SIGNING_KEY_HEX` environment variable at deployment time. The deploy
script handles this automatically.

### 5. Generate Deterministic Lockfile

The lockfile pins all modules, profiles, and regpacks to content-addressed digests:

```bash
mez lock jurisdictions/pk-sifc/zone.yaml

# Verify:
mez lock jurisdictions/pk-sifc/zone.yaml --check
```

The lockfile is written to `jurisdictions/pk-sifc/stack.lock` and contains
SHA-256 digests for every module, lawpack, regpack, and corridor definition.

### 6. Validate Configuration

```bash
# Validate all modules referenced by the profile
mez validate --all-modules

# Validate the zone configuration
mez validate jurisdictions/pk-sifc/zone.yaml

# Validate all profiles
mez validate --all-profiles
```

### 7. Deploy

#### Docker Compose (Single Zone)

```bash
./deploy/scripts/deploy-zone.sh sovereign-govos org.momentum.mez.zone.pk-sifc pk
```

The script will:
1. Generate random credentials (Postgres, Grafana, auth token)
2. Build or locate the `mez-cli` binary
3. Generate a real Ed25519 zone signing key
4. Start Docker Compose (mez-api + PostgreSQL + Prometheus + Grafana)
5. Wait for health checks
6. Report endpoints

#### Docker Compose (Two-Zone Corridor Demo)

```bash
# Full demo: deploy, test, teardown
./deploy/scripts/demo-two-zone.sh

# Keep running after demo
./deploy/scripts/demo-two-zone.sh --no-teardown
```

#### Kubernetes

```bash
# Apply the deployment manifest
kubectl apply -f deploy/k8s/deployment.yaml

# Secrets must be pre-created:
kubectl create secret generic mez-secrets \
  --from-literal=DATABASE_URL="postgresql://..." \
  --from-literal=MEZ_JWT_SECRET="..."
```

#### AWS (Terraform)

```bash
cd deploy/aws/terraform
terraform init
terraform plan -var-file=examples/hybrid-zone.tfvars
terraform apply
```

### 8. Post-Deploy Verification

```bash
# Health check
curl http://localhost:8080/health/readiness

# Regulator dashboard
curl -H "Authorization: Bearer $AUTH_TOKEN" http://localhost:8080/v1/regulator/dashboard

# List corridors
curl -H "Authorization: Bearer $AUTH_TOKEN" http://localhost:8080/v1/corridors
```

## Corridor Establishment Walkthrough

Once two zones are deployed, establish a cross-border corridor:

### 1. Zone A Proposes

```bash
curl -X POST http://zone-a:8080/v1/corridors/peers/propose \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZONE_A_TOKEN" \
  -d '{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "proposer_jurisdiction_id": "pk",
    "proposer_zone_id": "org.momentum.mez.zone.pk-sifc",
    "proposer_verifying_key_hex": "<zone-a-public-key-hex>",
    "proposer_did": "did:mass:zone:pk-sifc",
    "responder_jurisdiction_id": "ae-difc",
    "proposed_at": "2026-02-19T00:00:00Z",
    "parameters": {},
    "signature": "<ed25519-signature>"
  }'
```

### 2. Zone B Accepts

```bash
curl -X POST http://zone-a:8080/v1/corridors/peers/accept \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZONE_A_TOKEN" \
  -d '{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "responder_zone_id": "org.momentum.mez.zone.ae-difc",
    "responder_verifying_key_hex": "<zone-b-public-key-hex>",
    "responder_did": "did:mass:zone:ae-difc",
    "genesis_root_hex": "<shared-genesis-root>",
    "accepted_at": "2026-02-19T00:00:00Z",
    "signature": "<ed25519-signature>"
  }'
```

### 3. Exchange Receipts

Once the corridor is active, receipts flow between zones via the peer exchange API:

```bash
curl -X POST http://zone-b:8081/v1/corridors/peers/receipts \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $ZONE_B_TOKEN" \
  -d '{
    "corridor_id": "org.momentum.mez.corridor.pk-ae.cross-border",
    "origin_zone_id": "org.momentum.mez.zone.pk-sifc",
    "sequence": 0,
    "receipt_json": { ... },
    "receipt_digest": "<sha256-hex>",
    "signature": "<ed25519-signature>",
    "produced_at": "2026-02-19T00:00:01Z"
  }'
```

Replay protection ensures the same receipt cannot be delivered twice (HTTP 409).

## Available CLI Commands

| Command | Purpose |
|---|---|
| `mez validate --all-modules` | Validate all modules against schemas |
| `mez validate --all-profiles` | Validate all profiles |
| `mez validate --all-zones` | Validate all zones |
| `mez lock <zone.yaml>` | Generate deterministic lockfile |
| `mez lock <zone.yaml> --check` | Verify existing lockfile |
| `mez corridor create` | Create a new corridor |
| `mez corridor submit` | Submit corridor evidence |
| `mez corridor activate` | Activate a pending corridor |
| `mez corridor status` | Show corridor state |
| `mez corridor list` | List all corridors |
| `mez regpack build` | Build regpack artifacts |
| `mez artifact store` | Store artifact in CAS |
| `mez artifact verify` | Verify CAS artifact integrity |
| `mez vc keygen` | Generate Ed25519 key pair |
| `mez vc sign` | Sign a document |
| `mez vc verify` | Verify a signature |
