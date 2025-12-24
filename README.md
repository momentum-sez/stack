# Momentum SEZ Stack (MSEZ) — v0.4.0


This repository is a **reference specification + reference library** for building *programmable Special Economic Zones (SEZs)* as modular, forkable, and composable “jurisdiction nodes” in the Momentum/Mass network.

It is designed to function as an **open standard**: modular, versioned, testable, and implementable by multiple vendors.

> **Not legal advice.** This repository contains model texts, technical specs, and interoperability schemas. Deployments require local legal review, political authorization, and licensed financial/regulatory operators.

## What’s inside

- **`spec/`** — the normative MSEZ Stack specification (how modules work, how profiles compose, conformance rules)
- **`modules/`** — reference modules (legal/regulatory/financial/corridor/etc.) with machine-readable sources
- **`profiles/`** — curated “base templates/styles” (bundles of modules + parameters)
- **`schemas/`** — JSON Schemas for manifests, attestations, corridors
- **`apis/`** — OpenAPI specs for Mass integration + regulator console APIs
- **`tools/`** — build/validate/publish tools (reference implementations)
- **`registries/`** — registries of module IDs, jurisdiction IDs, corridor IDs

## Quick start

```bash
# 1) validate a profile bundle (schema + semantic checks)
python tools/msez.py validate profiles/digital-financial-center/profile.yaml

# 2) validate a zone deployment (profile+corridors+overlays)
python tools/msez.py validate --zone jurisdictions/_starter/zone.yaml

# 3) build a reproducible zone bundle (applies overlays + renders templates)
python tools/msez.py build --zone jurisdictions/_starter/zone.yaml --out dist/
```

## Repository conventions

- **Normative keywords**: “MUST/SHOULD/MAY” are interpreted per RFC 2119 + RFC 8174 (see `spec/00-terminology.md`).
- **Versioning**: semver for modules and profiles; “stack spec” has its own version.
- **Content licensing**: see `LICENSES/` and per-module `module.yaml`.

## Status

Skeleton created: 2025-12-21.


## Added in expanded skeleton

- Akoma Ntoso Act/Regulation/Bylaw templates (`modules/legal/akn-templates/`)
- Licensing scaffolding pack (`modules/licensing/*`)
- Corridor manifests + playbooks for SWIFT, stablecoin settlement, open banking (`modules/corridors/*`)
- Operational modules: regulator console and data classification (`modules/operational/*`)
- Documentation guides (`docs/`) and CI workflow (`.github/workflows/ci.yml`)


## v0.4 commands



# 4) verify a corridor is cryptographically bound (manifest + security artifacts + VC)
python tools/msez.py corridor verify modules/corridors/swift

# 4b) show corridor activation status and blockers (thresholds + commitments)
python tools/msez.py corridor status modules/corridors/swift

# 5) sign and verify a VC (for co-signing / governance flows)
python tools/msez.py vc sign docs/examples/vc/unsigned.corridor-definition.json --key docs/examples/keys/dev.ed25519.jwk --out /tmp/signed.json
python tools/msez.py vc verify /tmp/signed.definition.json

# 6) generate an Ed25519 key (writes JWK, prints did:key)
python tools/msez.py vc keygen --out /tmp/my.ed25519.jwk

# 7) scaffold corridor VCs from a corridor package
python tools/msez.py corridor vc-init-definition <corridor-dir> --issuer did:key:z... --out corridor.vc.unsigned.json
python tools/msez.py corridor vc-init-agreement <corridor-dir> --party did:key:z... --role zone_authority --out agreement.unsigned.json

```bash
pip install -r tools/requirements.txt
python tools/msez.py validate --all-modules
python tools/msez.py fetch-akoma-schemas
pytest -q
python tools/msez.py render modules/legal/akn-templates/src/akn/act.template.xml --pdf
python tools/msez.py lock jurisdictions/_starter/zone.yaml --out jurisdictions/_starter/stack.lock
python tools/msez.py check-coverage --zone jurisdictions/_starter/zone.yaml
python tools/msez.py build --zone jurisdictions/_starter/zone.yaml --out dist/
python tools/msez.py publish dist/bundle --out-dir dist/publish --pdf
```
