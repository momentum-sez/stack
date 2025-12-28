# Transition Type Registry Lock Store (content-addressed)

> **Deprecated (v0.4.7+)**: the canonical CAS location is now `dist/artifacts/transition-types/...` (see `spec/97-artifacts.md`).
> This directory remains for backwards compatibility.


This directory is a **content-addressed** artifact store for Transition Type Registry Lock snapshots.

A lock snapshot is identified by its `snapshot_digest_sha256 = SHA256(JCS(snapshot))`.

## Filename convention

Each stored lock MUST be written as:

```
<digest>.transition-types.lock.json
```

Example:

```
d8f22c0a...609fd207.transition-types.lock.json
```

## Why this exists

Corridor receipts may commit to a registry snapshot via:

- `receipt.transition_type_registry_digest_sha256`

As corridors evolve, the *current* corridor module may update its registry/lock file — but historical receipts must remain
verifiable. This store lets verifiers resolve **historical registry snapshots by digest** even after updates.

## Tooling

- Generate a lock:
  - `msez registry transition-types-lock registries/transition-types.yaml`
- Store a lock by digest:
  - `msez registry transition-types-store registries/transition-types.lock.json`
- Resolve a lock by digest:
  - `msez registry transition-types-resolve <digest>`

The resolver consults `MSEZ_ARTIFACT_STORE_DIRS` (preferred; os.pathsep-separated) and `MSEZ_TRANSITION_TYPES_STORE_DIRS` (legacy) to support external stores.
