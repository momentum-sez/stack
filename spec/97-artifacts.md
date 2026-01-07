# Artifact CAS conventions

Stack v0.4.7+ introduces a **generic content-addressed storage (CAS) convention** for all artifacts referenced by digest in receipts / VCs.

Stack v0.4.8 extends the recommended type set to include **schemas, VCs, checkpoints, and proof keys**, and adds an optional verification mode that enforces **commitment completeness** (all committed digests are resolvable).

Stack v0.4.9 adds a `blob` artifact type for **raw byte commitments** (e.g., proof bytes and legacy attachments).

Stack v0.4.10 generalizes `transition.attachments[*]` into **typed artifact references** `{artifact_type, digest_sha256, ...}` so attachments can point at non-blob artifacts (VC payload digests, schemas, checkpoints, etc.) while remaining backward compatible (missing `artifact_type` defaults to `blob`).

Stack v0.4.11 introduces a single reusable **ArtifactRef** schema and reuses it across receipts, VCs, lawpack metadata, and checkpoint/inclusion proof structures so that typed artifact references are mechanically consistent everywhere.

Stack v0.4.12 extends this pattern further: **digest-bearing fields** that historically carried bare sha256 strings MAY now carry an **ArtifactRef** in place of the raw digest (while remaining backwards compatible). Verifiers MUST treat the commitment as the `digest_sha256` value and use `artifact_type` only to determine the CAS resolution path.

Stack v0.4.13 applies the same pattern to additional supply-chain surfaces:
- `stack.lock` digest-bearing fields (lawpacks and corridor artifact hashes)
- `node.yaml` digest-bearing fields (proof keys and VC attestations)
- `transition-types.lock.json` snapshot digests for schema/ruleset/circuit entries

Stack v0.4.14 adds a reference-tool option to make ArtifactRef the *default substrate produced by tooling*:

```bash
python -m tools.msez lock --emit-artifactrefs <zone.yaml>
```

Stack v0.4.26 adds a **witness bundle** packaging format for artifact closures (manifest + resolved CAS nodes) so verifiers can transfer a minimal, offline-ready closure between environments.

Stack v0.4.28 adds an optional **witness bundle attestation VC** that signs the bundle's `manifest.json` digest (provenance / chain-of-custody) without changing underlying receipt/VC digest commitments.

ArtifactRef schema id: `https://schemas.momentum-sez.org/msez/artifact-ref.schema.json`

## ArtifactRef

ArtifactRefs are small typed commitments:

```json
{
  "artifact_type": "lawpack",
  "digest_sha256": "<64-hex>",
  "uri": "dist/artifacts/lawpack/<64-hex>.lawpack.zip",
  "media_type": "application/zip",
  "byte_length": 123456
}
```

Notes:
- `artifact_type` MUST match the CAS directory name under `dist/artifacts/<type>/...`.
- `digest_sha256` is the canonical sha256 commitment.
- `uri`, `media_type`, and `byte_length` are **hints only** and MUST NOT be used for verification.


## Convention

All artifacts SHOULD be stored (and resolvable) under:

```text
dist/artifacts/<type>/<digest>.*
```

Where:
- `<type>` is a lowercase artifact category identifier.
- `<digest>` is a lowercase sha256 hex digest.
- `.*` is an optional suffix describing format (`.lawpack.zip`, `.transition-types.lock.json`, `.schema.json`, `.vc.json`, `.json`, `.r1cs`, `.wasm`, ...).

Recommended type set (non-exhaustive):

- `lawpack` — jurisdictional legal corpora (see `spec/91-lawpacks.md`).
- `ruleset` — ruleset descriptors / semantics definitions.
- `transition-types` — registry lock snapshots (`*.transition-types.lock.json`).
- `schema` — JSON Schemas referenced by digest.
- `vc` — Verifiable Credentials referenced by payload digest (VC without `proof`).
- `checkpoint` — corridor checkpoints referenced by payload digest (checkpoint without `proof`).
- `circuit` — ZK circuits / programs referenced by digest.
- `proof-key` — proof system keys (e.g., verifying keys) referenced by digest.
- `blob` — raw byte blobs (attachments, proof bytes, documents) referenced by digest.

This makes every digest commitment **obviously resolvable** in a repository checkout, without requiring knowledge of module versioning or file layout.

## Resolver semantics

Given `(type, digest)`:

1. Search each configured store root for `dist/artifacts/<type>/<digest>.*`.
2. If exactly one match exists, resolve to that file.
3. If no match exists, the resolver MAY apply type-specific legacy fallbacks (non-normative).
4. If multiple matches exist, resolution MUST fail as ambiguous.

The reference implementation is:
- `tools/artifacts.py`
- CLI: `python -m tools.msez artifact resolve <type> <digest>`

## Populating the store

Reference implementation helpers:

- Populate rulesets:

```bash
python -m tools.msez artifact index-rulesets
```

- Populate lawpacks (copies locally built `dist/lawpacks/**/*.lawpack.zip`):

```bash
python -m tools.msez artifact index-lawpacks
```

- Populate JSON Schemas (copies `schemas/**/*.schema.json`):

```bash
python -m tools.msez artifact index-schemas
```

- Populate VCs (copies common `*.vc.json` files in modules/docs/tests):

```bash
python -m tools.msez artifact index-vcs
```

- Store any specific artifact:

```bash
python -m tools.msez artifact store <type> <digest> <path>
```

## Commitment completeness

Verifiers MAY choose to enforce that **every digest commitment is resolvable** via the CAS.

The reference CLI exposes this as:

```bash
python -m tools.msez corridor state verify ... --require-artifacts
```

When enabled, verification fails if any committed digest in receipts cannot be resolved via `dist/artifacts/<type>/<digest>.*` (or configured store roots).

## Why this matters

Corridor receipts and VCs commit to digests, for example:

- `lawpack_digest_set`
- `ruleset_digest_set`
- `transition_type_registry_digest_sha256`
- `schema_digest_sha256`
- `definition_payload_sha256`
- `checkpoint_digest_sha256`
- `zk_circuit_digest_sha256`
- `verifier_key_digest_sha256`
- `transition.attachments[*].digest_sha256` (scoped by `transition.attachments[*].artifact_type`; defaults to `blob`)
- `receipt.zk.proof_sha256`

Without a shared resolution convention, these commitments remain difficult to verify outside a single repository layout.

This CAS convention makes verification and portability straightforward and supports historical resolution of artifacts by digest.

## Witness bundles

For offline / air-gapped verification and transfer between environments, the reference implementation can emit a **witness bundle** that contains:

- `manifest.json`: a full `MSEZArtifactGraphVerifyReport` (closure root, stats, node list, optional edges)
- `artifacts/<type>/<digest>.*`: one file per resolved CAS node
- `root/*`: when the closure root was a local JSON/YAML file, the root document is included for audit convenience
- `root/<dirname>/*`: when the closure root was a local directory, structured files (JSON/YAML) under that directory are included for audit convenience

Create a witness bundle:

```bash
# CAS root
python -m tools.msez artifact graph verify <type> <digest> --bundle /tmp/msez-witness.zip --strict --json

# Local file root (JSON/YAML)
python -m tools.msez artifact graph verify --path ./some/root.yaml --bundle /tmp/msez-witness.zip --strict --json

# Local directory root (scan structured files for embedded ArtifactRefs)
python -m tools.msez artifact graph verify --path ./modules/smart-assets/<asset_id> --bundle /tmp/asset-module.witness.zip --strict --json

# Operator UX wrapper (Smart Asset portable audit packet)
python -m tools.msez asset module witness-bundle ./modules/smart-assets/<asset_id> --out /tmp/asset-module.witness.zip --json
```

Verify using a witness bundle as an offline CAS root:

```bash
python -m tools.msez artifact graph verify --from-bundle /tmp/msez-witness.zip --strict --json
```

### Witness bundle attestation

Bundles are *witnesses*, not authorities. To make "who assembled this closure" explicit, v0.4.28 adds a provenance VC that commits to the bundle's `manifest.json` via `SHA256(JCS(manifest.json))`:

```bash
python -m tools.msez artifact bundle attest /tmp/msez-witness.zip --issuer did:key:... --sign --key /path/to/ed25519.jwk
python -m tools.msez artifact bundle verify /tmp/msez-witness.zip --vc /tmp/msez-witness.attestation.vc.json
```

This VC is optional but enables:

- chain-of-custody for closures shared between organizations
- cheap provenance / attribution for "complete closure" artifacts used in disputes
- a clean substrate for future watcher economies (bonded watchers can attest to bundle completeness in addition to head state)
