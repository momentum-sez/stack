# Momentum SEZ Stack (MSEZ) — v0.4.18


This repository is a **reference specification + reference library** for building *programmable Special Economic Zones (SEZs)* as modular, forkable, and composable “jurisdiction nodes” in the Momentum/Mass network.

It is designed to function like an **open standard**: modular, versioned, testable, and implementable by multiple vendors.

> **Not legal advice.** This repository contains model texts, technical specs, and interoperability schemas. Deployments require local legal review, political authorization, and licensed financial/regulatory operators.

## What’s inside

- **`spec/`** — the normative MSEZ Stack specification (how modules work, how profiles compose, conformance rules)
- **`modules/`** — reference modules (legal/regulatory/financial/corridor/etc.) with machine-readable sources
- **`profiles/`** — curated “base templates/styles” (bundles of modules + parameters)
- **`schemas/`** — JSON Schemas for manifests, attestations, corridors
- **`rulesets/`** — content-addressed ruleset descriptors (pins verifier logic)
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
- Foundational upgrade / fortification plan (`docs/fortification/foundational-upgrade-v0.4.14.md`) (v0.4.14)
- High-leverage integrity upgrades (`docs/fortification/high-leverage-v0.4.15.md`) (v0.4.15)
- Watcher attestation aggregation (`watcher-compare`) for instant fork alarms (v0.4.16)
- Watcher quorum policy + compact head commitments (v0.4.17)
- Fork-aware receipt verification + transition envelopes + fork resolutions + anchors/finality scaffolds (v0.4.18)


## Tooling commands (v0.4+)

```bash
# corridor cryptographic verification (VCs + agreements)
python tools/msez.py corridor verify modules/corridors/swift
python tools/msez.py corridor status modules/corridors/swift
python tools/msez.py corridor availability-attest modules/corridors/swift --issuer did:example:operator --sign --key docs/examples/keys/dev.ed25519.jwk --out /tmp/availability.vc.json
python tools/msez.py corridor availability-verify modules/corridors/swift --vcs /tmp/availability.vc.json

# corridor state channels (verifiable receipts)
python tools/msez.py corridor state genesis-root modules/corridors/swift
python tools/msez.py corridor state receipt-init modules/corridors/swift   --sequence 0   --transition docs/examples/state/noop.transition.json   --sign --key docs/examples/keys/dev.ed25519.jwk   --out /tmp/receipt0.json
python tools/msez.py corridor state verify modules/corridors/swift --receipts /tmp/receipt0.json
python tools/msez.py corridor state verify modules/corridors/swift --receipts /tmp/receipt0.json --require-artifacts
python tools/msez.py corridor state checkpoint modules/corridors/swift --receipts /tmp/receipt0.json --issuer did:example:zone --sign --key docs/examples/keys/dev.ed25519.jwk --out /tmp/checkpoint.json
python tools/msez.py corridor state verify modules/corridors/swift --receipts /tmp/receipt0.json --checkpoint /tmp/checkpoint.json --enforce-checkpoint-policy
python tools/msez.py corridor state watcher-attest modules/corridors/swift --checkpoint /tmp/checkpoint.json --issuer did:example:watcher --sign --key docs/examples/keys/dev.ed25519.jwk --out /tmp/watcher.vc.json
python tools/msez.py corridor state fork-alarm modules/corridors/swift --receipt-a /tmp/receipt0.json --receipt-b /tmp/receipt0_fork.json --issuer did:example:watcher --sign --key docs/examples/keys/dev.ed25519.jwk --out /tmp/fork-alarm.vc.json

# transition type registries + lock snapshots (v0.4.5+; CAS v0.4.7+)
python tools/msez.py registry transition-types-lock registries/transition-types.yaml
python tools/msez.py registry transition-types-store registries/transition-types.lock.json
python tools/msez.py registry transition-types-resolve d8f22c0aa0114b30f961c208e48f08b17403655cdd0c25c9027b2373609fd207

# generic artifact CAS (dist/artifacts/<type>/<digest>.*) (v0.4.7+; schema/vc/checkpoint/proof-key types v0.4.8+; blob type v0.4.9+)
python tools/msez.py artifact resolve transition-types d8f22c0aa0114b30f961c208e48f08b17403655cdd0c25c9027b2373609fd207
python tools/msez.py artifact index-rulesets
python tools/msez.py artifact index-lawpacks
python tools/msez.py artifact index-schemas
python tools/msez.py artifact index-vcs

# sign and verify VCs (for co-signing / governance flows)
python tools/msez.py vc keygen --out /tmp/my.ed25519.jwk
python tools/msez.py vc sign docs/examples/vc/unsigned.corridor-definition.json   --key docs/examples/keys/dev.ed25519.jwk --out /tmp/signed.definition.json
python tools/msez.py vc verify /tmp/signed.definition.json

# lawpacks (v0.4.1+): ingest a jurisdiction corpus into a content-addressed lawpack
python tools/msez.py law ingest modules/legal/jurisdictions/us/ca/civil --as-of-date 2025-01-01 --fetch
```

## Development

```bash
pip install -r tools/requirements.txt
pytest -q

# fetch Akoma Ntoso schema bundle (needed for schema validation)
python tools/msez.py fetch-akoma-schemas

# validate all modules
python tools/msez.py validate --all-modules

# render an Akoma template to HTML/PDF
python tools/msez.py render modules/legal/akn-templates/src/akn/act.template.xml --pdf

# generate a deterministic lockfile for a zone deployment
python tools/msez.py lock jurisdictions/_starter/zone.yaml --out jurisdictions/_starter/stack.lock

# build a reproducible bundle (applies overlays + renders templates)
python tools/msez.py build --zone jurisdictions/_starter/zone.yaml --out dist/

# publish rendered artifacts
python tools/msez.py publish dist/bundle --out-dir dist/publish --pdf
```
