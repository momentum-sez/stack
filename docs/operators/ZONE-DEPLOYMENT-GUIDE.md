# Zone Deployment Guide

This guide describes the minimal operational steps to deploy a zone using the MSEZ stack.

It is intentionally tool-centric and assumes you are using the reference CLI (`tools/msez.py`).

## 0) Pick a profile

Profiles live under `profiles/` and define a baseline module set.

Example:

```bash
python3 tools/msez.py profile validate profiles/minimal-mvp/profile.yaml
```

## 1) Instantiate a zone repo

Create a `zones/<zone-id>` directory (or a separate repo) with:

- `zone.yaml`
- `stack.lock`
- pinned artifact store under `dist/artifacts/...`

At minimum:

```yaml
# zone.yaml
zone_id: example.zone.alpha
profile: profiles/minimal-mvp/profile.yaml
jurisdiction_id: "US-UT"   # example
```

## 2) Build and pin lawpacks

For each jurisdiction/domain you want to support, produce a lawpack:

```bash
python3 tools/msez.py lawpack ingest --jurisdiction US-UT --domain civil --as-of 2025-01-01 \
  --out dist/artifacts/lawpack/
```

Then pin the resulting digests into your lockfile.

## 3) Generate or update stack.lock

```bash
python3 tools/msez.py lock generate zones/example.zone.alpha/zone.yaml \
  --emit-artifactrefs \
  --out zones/example.zone.alpha/stack.lock
```

Treat `stack.lock` as an auditable manifest of everything your zone is committing to run.

## 4) Validate the zone

```bash
python3 tools/msez.py zone validate zones/example.zone.alpha/zone.yaml
python3 tools/msez.py lock verify zones/example.zone.alpha/stack.lock --require-artifacts
```

## 5) Operate modules

Module execution is outside the scope of this repo (it depends on your runtime), but the operational invariant is:

1. run only modules pinned by `stack.lock`
2. publish module outputs as content-addressed artifacts when outputs must be verifiable

## 6) Publish identifiers and trust roots

For real deployments, publish:

- your authority registry chain artifacts
- your zone authority DID(s)
- how to resolve your pinned artifacts

This enables corridor counterparts and auditors to verify your commitments.