# Smart Asset OS (non-blockchain)

This document specifies the **Smart Asset OS** reference layer for the Momentum EZ Stack (MEZ).

> Design goal: run “smart assets” (stateful, policy constrained, attestable objects) **without requiring a blockchain**, while preserving strong commitment surfaces (digests + ArtifactRefs) and optional anchoring into corridor state channels.

## Why this exists

Corridors (capital, trade, arbitration, identity) create *jurisdiction-to-jurisdiction* pathways. Smart Assets are the *things that move* across those pathways.

The stack must support:

* **Multi-jurisdiction legality** (distinct lawpacks / enforcement profiles)
* **Non-tokenized assets** (legal entities, contracts, buildings, permits, obligations)
* **Redundancy + fault tolerance** (shards across jurisdictions, witnesses, archival trails)

## Core invariants

1. **Immutable identity**: `asset_id = sha256(JCS(genesis_without_asset_id))` (derived `asset_id` field omitted when hashing).
2. **Deterministic state commitments**: `state_root_sha256 = sha256(JCS(state))`.
3. **Explicit jurisdiction bindings**: an asset is “registered” into one or more harbors with explicit compliance + enforcement profiles.
4. **Everything resolvable**: any commitment MUST be resolvable via `ArtifactRef` + content-addressed storage (CAS).
5. **Optional anchoring**: assets can be anchored to corridor receipts by attaching checkpoints/attestations as artifacts.

## Canonical documents

### 1) Smart Asset Genesis

Schema: `schemas/smart-asset.genesis.schema.json`

* Immutable, content-addressed.
* May include an informational `asset_id` field, but `asset_id` is defined by digest.

### 2) Smart Asset Registry VC

Schema: `schemas/vc.smart-asset-registry.schema.json`

* A VC binding `asset_id` to a set of **JurisdictionBindings**.
* Each binding declares:
  - `harbor_id`
  - `lawpacks[]` (domain commitments)
  - `compliance_profile` (allowed transitions + required attestations)
  - `enforcement_profile` (for simulation / monitoring)
  - `shard_role` (primary / replica / witness / archive)

### 3) Smart Asset Checkpoint

Schema: `schemas/smart-asset.checkpoint.schema.json`

* Commits to a `state_root_sha256`.
* Can be stored in CAS and attached to corridor receipts.

### 4) Smart Asset Attestation

Schema: `schemas/smart-asset.attestation.schema.json`

* Typed statements (custody, KYC, audit, oracle feeds, etc.) with optional attachments.

### 5) Smart Asset Operational Manifest (optional)

Schema: `schemas/smart-asset.manifest.schema.json`

* Node-local configuration for storage + replication.

## Reference CLI

Smart Asset operations are available through the `mez` CLI (`mez-cli` crate):

```bash
# identity
mez asset genesis-init --asset-name "Acme Bond" --asset-class security --out dist/tmp/bond.genesis.json
mez asset genesis-hash dist/tmp/bond.genesis.json

# registry
mez asset registry-init --genesis dist/tmp/bond.genesis.json --bindings bindings.yaml --issuer did:key:... \
  --out dist/tmp/bond.registry.vc.unsigned.json

# checkpoint
mez asset checkpoint-build --asset-id <asset_id> --state state.json --store

# attestations
mez asset attestation-init --asset-id <asset_id> --issuer did:key:... --kind kyc.passed.v1 --claims claims.yaml --store

# compliance
mez asset compliance-eval --registry dist/tmp/bond.registry.vc.unsigned.json --transition transition-envelope.json

# asset-local receipt chain (non-blockchain)
#
# Optional (recommended): scaffold an operator module directory to keep receipts, checkpoints,
# proofs, and trust anchors in a single portable folder.
mez asset module init <asset_id>

# You can now pass the module directory (or asset.yaml) as the first argument to state subcommands:
mez asset state genesis-root modules/smart-assets/<asset_id>
mez asset state receipt-init modules/smart-assets/<asset_id> --sequence 0 --prev-root genesis \
  --sign --key jwk.json
mez asset state verify modules/smart-assets/<asset_id>

# If redundant writers cause forks (same sequence/prev_root with multiple next_roots),
# verification will fail until you provide a fork-resolution artifact selecting the canonical branch.
# Fork resolutions are expected to live asset-local at state/fork-resolutions/ (by convention).
mez asset state fork-resolve modules/smart-assets/<asset_id> \
  --sequence 1 --prev-root <prev_root> --chosen-next-root <next_root> \
  --issuer did:key:... \
  --out modules/smart-assets/<asset_id>/state/fork-resolutions/fork-resolution.seq1.json
mez asset state verify modules/smart-assets/<asset_id> \
  --fork-resolutions modules/smart-assets/<asset_id>/state/fork-resolutions

mez asset state checkpoint modules/smart-assets/<asset_id> --sign --key jwk.json
mez asset state inclusion-proof modules/smart-assets/<asset_id> --sequence 0 \
  --checkpoint modules/smart-assets/<asset_id>/state/checkpoints/smart-asset.receipt-chain.checkpoint.json
mez asset state verify-inclusion modules/smart-assets/<asset_id> \
  --receipt modules/smart-assets/<asset_id>/state/receipts/smart-asset.receipt.0.json \
  --proof modules/smart-assets/<asset_id>/state/proofs/smart-asset.receipt.0.inclusion-proof.json \
  --checkpoint modules/smart-assets/<asset_id>/state/checkpoints/smart-asset.receipt-chain.checkpoint.json

# (Legacy) You can still pass explicit --asset-id / --receipts paths if you prefer.
```

## Roadmap

### P0 (done in v0.4.29)

* Canonical identity + registry + manifest schemas
* CLI to generate/store documents
* Declarative multi-jurisdiction compliance evaluator

### P1

* Asset-local receipt chain (MMR) for transitions + checkpoints — landed in v0.4.31
* Smart Asset module directories + `mez asset module init` + module-aware state CLI — landed in v0.4.32
* Corridor anchoring helpers (attach checkpoint -> receipt)
  - `mez asset anchor-verify` landed in v0.4.30: verifies that a Smart Asset checkpoint digest is present as a typed receipt attachment, and that the receipt is included in a corridor checkpoint via an MMR inclusion proof.
* Replication policy + witness bundles for asset histories

### P2

* Policy-to-code integration (OPA/Rego / Cedar / custom engines)
* ZK proof slots for privacy-preserving compliance
* Cross-asset coordination primitives (2PC / saga)

### P3

* Asset corridor templates (capital, trade, arbitration)
* Regulator observability APIs + enforcement simulators
* Inter-zone “network of zones” routing policies
