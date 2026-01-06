# Smart Asset OS (non-blockchain)

This document specifies the **Smart Asset OS** reference layer for the Momentum SEZ Stack (MSEZ).

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

The reference implementation lives in `tools/smart_asset.py` and is wired into `tools/msez.py`:

```bash
# identity
msez asset genesis-init --asset-name "Acme Bond" --asset-class security --out dist/tmp/bond.genesis.json
msez asset genesis-hash dist/tmp/bond.genesis.json

# registry
msez asset registry-init --genesis dist/tmp/bond.genesis.json --bindings bindings.yaml --issuer did:key:... \
  --out dist/tmp/bond.registry.vc.unsigned.json

# checkpoint
msez asset checkpoint-build --asset-id <asset_id> --state state.json --store

# attestations
msez asset attestation-init --asset-id <asset_id> --issuer did:key:... --kind kyc.passed.v1 --claims claims.yaml --store

# compliance
msez asset compliance-eval --registry dist/tmp/bond.registry.vc.unsigned.json --transition transition-envelope.json

# asset-local receipt chain (non-blockchain)
msez asset state genesis-root --asset-id <asset_id>
msez asset state receipt-init --asset-id <asset_id> --sequence 0 --prev-root genesis \
  --sign --key jwk.json --out dist/tmp/bond.receipt.0.json
msez asset state verify --asset-id <asset_id> --receipts dist/tmp/receipts/
msez asset state checkpoint --asset-id <asset_id> --receipts dist/tmp/receipts/ \
  --sign --key jwk.json --out dist/tmp/bond.receipt-chain.checkpoint.json
msez asset state inclusion-proof --asset-id <asset_id> --receipts dist/tmp/receipts/ \
  --sequence 0 --checkpoint dist/tmp/bond.receipt-chain.checkpoint.json --out dist/tmp/bond.receipt.0.proof.json
msez asset state verify-inclusion --asset-id <asset_id> \
  --receipt dist/tmp/bond.receipt.0.json --proof dist/tmp/bond.receipt.0.proof.json \
  --checkpoint dist/tmp/bond.receipt-chain.checkpoint.json
```

## Roadmap

### P0 (done in v0.4.29)

* Canonical identity + registry + manifest schemas
* CLI to generate/store documents
* Declarative multi-jurisdiction compliance evaluator

### P1

* Asset-local receipt chain (MMR) for transitions + checkpoints — landed in v0.4.31
* Corridor anchoring helpers (attach checkpoint -> receipt)
  - `msez asset anchor-verify` landed in v0.4.30: verifies that a Smart Asset checkpoint digest is present as a typed receipt attachment, and that the receipt is included in a corridor checkpoint via an MMR inclusion proof.
* Replication policy + witness bundles for asset histories

### P2

* Policy-to-code integration (OPA/Rego / Cedar / custom engines)
* ZK proof slots for privacy-preserving compliance
* Cross-asset coordination primitives (2PC / saga)

### P3

* Asset corridor templates (capital, trade, arbitration)
* Regulator observability APIs + enforcement simulators
* Inter-zone “network of zones” routing policies
