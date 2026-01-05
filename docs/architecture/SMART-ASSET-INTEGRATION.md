# Smart Asset integration (v0.4.x scaffold)

This document describes a **minimal, composable** way to integrate the Smart Asset spec
with MSEZ corridors *before* the `0.4 → 0.5` bump.

## Why this matters

MSEZ corridors define a verifiable, law-bound state channel between zones.
Smart Assets define Merkle-DAG state machines whose state *moves* across jurisdictions.

The integration goal is to make *asset movement* a first-class transition type in corridors,
without hard-coding asset logic into corridor verification.

## Key binding points

### 1) Asset checkpoints commit into corridor receipts

- Smart Asset nodes produce a checkpoint with `asset_id` + `state_root_sha256`.
- A corridor receipt includes an `ArtifactRef` attachment to the checkpoint artifact.
- The corridor receipt's MMR enables selective inclusion proofs of a particular checkpoint
  without revealing other receipts.

### 2) Lawpacks govern asset semantics

Each Smart Asset transition should declare:

- `lawpack_digest_set` (jurisdictional legal corpus snapshots)
- `ruleset_digest_set` (policy-as-code validation)
- optional `zk.circuit` references for privacy-preserving proof-carrying transitions

### 3) Migration is a saga

Cross-zone migration should be modeled as an explicit saga:

1. `asset.migrate.propose`
2. `asset.migrate.lock`
3. `asset.migrate.commit`
4. `asset.migrate.finalize`
5. compensation transitions if aborting

Each stage is a corridor transition with typed semantics via the transition type registry.

## What this repo contains today

- Transition type registry + lock snapshots (CAS-addressed)
- Corridor receipts as typed transition envelopes with MMR inclusion proofs
- Artifact CAS with `--require-artifacts` (and transitive mode)
- Smart Asset OpenAPI scaffold: `apis/smart-assets.openapi.yaml`

## Next implementation steps

1. Add canonical Smart Asset schemas under `schemas/smart-asset.*.schema.json`
2. Define standard transition kinds for asset migration and checkpointing
3. Add rulesets for baseline validation (e.g., custody quorum)
4. Add integration tests that bind asset checkpoints into receipts and validate inclusion proofs