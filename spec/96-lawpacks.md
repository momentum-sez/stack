# Lawpacks

This document specifies the **Lawpack supply chain** introduced in **Stack v0.4.1**.

A **lawpack** is a **content-addressed**, reproducible artifact representing a jurisdiction’s legal corpus snapshot for a given **domain** (e.g., civil, financial), normalized into a deterministic base format (Akoma Ntoso), with a fragment index for compilation and verification.

Lawpacks are the bridge between:
- **Human law** (sources: gazettes, statutes, regulations, PDFs, HTML pages)
- **Normalized machine representation** (Akoma Ntoso)
- **Cryptographic commitments** (digests pinned in `stack.lock` and referenced by corridor VCs)

## Goals

- Provide a deterministic legal corpus artifact that can be:
  - pinned by zones,
  - referenced by corridors,
  - used as a stable substrate for policy-to-code compilation and verification.
- Ensure provenance (sources + normalization recipe) is explicit.
- Avoid “floating law”: corridors must bind to specific corpus snapshots, not vague jurisdiction labels.

## Legal validity attestation (bridging digest → legal force) (v0.4.14+)

The **lawpack digest** binds operations to an immutable text snapshot. However, a digest alone does **not** prove that the text had **legal force** in a given jurisdiction.

To reduce the “legal‑cryptographic gap”, v0.4.14 introduces a standardized, optional VC:

- `schemas/vc.lawpack-attestation.schema.json`
- Credential type: `MSEZLawpackAttestationCredential`

An attestation SHOULD, at minimum:
- Identify `jurisdiction_id`, `domain`, and `as_of_date`
- Reference the lawpack digest (preferably as an `ArtifactRef` with `artifact_type: lawpack`)
- Provide a clear statement of the legal status (enacted / consolidated / partial / superseded) and evidence references

Reference tooling:
- `msez law attest-init` produces a VC skeleton for signing.
- `msez vc verify` can validate signatures for offline verification.

Corridor verifiers MAY require lawpack attestations as a policy decision (e.g., for high-stakes corridors), while still allowing experimental or private deployments without them.

## Lawpack artifact format

A lawpack is distributed as a **zip** with the following structure:

```text
lawpack.zip
├─ lawpack.yaml
├─ digest.sha256
├─ index.json
└─ akn/
   ├─ main.xml
   └─ (optional) additional Akoma Ntoso docs...
```

### lawpack.yaml

`lawpack.yaml` is the semantic metadata for the corpus snapshot:

- `jurisdiction_id` — identifier from `registries/jurisdictions.yaml`
- `domain` — e.g., `civil`, `financial`
- `as_of_date` — snapshot date (`YYYY-MM-DD`)
- `sources[]` — the input sources used (URLs, gazette references, PDFs, etc.)
- `sources[*].artifact_ref` — optional typed ArtifactRef (type `blob`) to captured raw source bytes stored in the global CAS (`dist/artifacts/blob/<digest>.*`).
- `license` — SPDX identifier (or `NOASSERTION`)
- `normalization` — the deterministic recipe used to convert sources → Akoma Ntoso

The reference schema is `schemas/lawpack.schema.json` via interface `msez.lawpack.metadata.v1`.

### akn/

`akn/` contains one or more **Akoma Ntoso** XML documents.

Determinism requirements (v0.4.1 reference implementation):
- XML canonicalization uses **Exclusive XML C14N**, with comments excluded.
- The emitted XML files may be formatted for human readability; the digest is computed over canonicalized form.

### index.json

`index.json` provides a deterministic mapping from **Akoma Ntoso element IDs (`eId`)** to:
- a canonical fragment digest,
- a stable selector (`xpath`),
- optional best-effort offsets.

This enables:
- “compile from law fragments” workflows,
- stable anchoring of machine rules to specific legal text fragments.

`index.json` is intentionally free of non-deterministic timestamps so that the lawpack digest is stable.

The reference structure is:

```json
{
  "index_version": "1",
  "jurisdiction_id": "us-ca",
  "domain": "civil",
  "documents": {
    "akn/main.xml": {
      "document_sha256": "<sha256 of canonical doc>",
      "fragments": {
        "p1": {
          "sha256": "<sha256 of canonical element>",
          "xpath": "/akomaNtoso/act/body/section[1]/p[1]",
          "byte_start": 123,
          "byte_end": 456
        }
      }
    }
  }
}
```

Notes:
- `byte_start`/`byte_end` may be `null` for some XML namespace contexts.
- The canonical fragment digest is always provided.

### digest.sha256

`digest.sha256` contains the lawpack digest (hex string) computed over canonicalized content.

#### Canonicalization

v0.4.1 defines canonicalization for lawpack digests as:

- YAML/JSON: **JCS** (RFC 8785) for objects without floats.
  - Floats are forbidden; represent quantities as strings or integers.
- XML: **Exclusive XML C14N**, no comments.

#### Digest computation

v0.4.1 defines the digest as:

```
SHA256(
  "msez-lawpack-v1\0" ||
  Σ(sorted(path) ( path || "\0" || canonical_bytes(path) || "\0" ))
)
```

where `path` iterates over:
- `lawpack.yaml` (canonicalized via JCS over its parsed structure),
- `index.json` (canonicalized via JCS),
- all `akn/**/*.xml` (canonicalized via XML C14N).

The reference implementation is in `tools/lawpack.py`.

## lawpack.lock.json

Each jurisdiction corpus module emits a lock entry:

- `<module_dir>/lawpack.lock.json`

This file includes:
- the lawpack digest,
- the artifact path to the emitted zip,
- component digests (metadata, index, AKN documents),
- provenance (sources manifest and normalization parameters).

The reference schema is `schemas/lawpack.lock.schema.json` via interface `msez.lawpack.lock.v1`.

## Ingestion pipeline

The reference CLI is:

```bash
python tools/msez.py law ingest modules/legal/jurisdictions/us/ca/civil \
  --as-of-date 2025-01-01 \
  --fetch
```

Steps:
1. Fetch sources declared in `sources.yaml` (optional; pluggable fetchers).
2. Normalize → Akoma Ntoso (deterministic recipe; reference implementation assumes `src/akn/**` exists).
3. Build `index.json`.
4. Compute `digest.sha256`.
5. Emit:
   - `dist/artifacts/lawpack/<digest>.lawpack.zip` (canonical CAS path; see `spec/97-artifacts.md`)
   - (optional) additional human-friendly copies (e.g., `dist/lawpacks/<jurisdiction_id>/<domain>/<digest>.lawpack.zip`)
   - `<module_dir>/lawpack.lock.json`

## Binding into stack.lock

Zones pin lawpacks in `stack.lock`:

```yaml
lawpacks:
  - jurisdiction_id: us-ca
    domain: civil
    lawpack_digest_sha256: <digest>
    lawpack_lock_path: modules/legal/jurisdictions/us/ca/civil/lawpack.lock.json
    lawpack_lock_sha256: <sha256>
    lawpack_artifact_path: dist/artifacts/lawpack/<digest>.lawpack.zip
    as_of_date: "2025-01-01"
```

Zones may optionally provide:
- `jurisdiction_stack`: a list of governing layers (e.g., `["ae", "ae-dubai", "ae-dubai-difc"]`)
- `lawpack_domains`: list of domains to pin (defaults: `["civil", "financial"]`)

The lock generator (`msez lock`) emits `lawpacks` pins by discovering:
- expected modules at `modules/legal/jurisdictions/<jurisdiction_id path>/<domain>/lawpack.lock.json`

## Corridor binding

Corridors become cryptographically meaningful when they bind to lawpack digests.

### Corridor Definition VC

The Corridor Definition VC (`schemas/vc.corridor-definition.schema.json`) may include:

```json
"lawpack_compatibility": {
  "required_domains": ["civil", "financial"],
  "allowed": [
    {
      "jurisdiction_id": "us-ca",
      "domain": "civil",
      "digests_sha256": ["..."]
    }
  ]
}
```

Semantics:
- `required_domains` indicates which domains each participant must pin and sign in their agreement VC.
- `allowed` optionally constrains acceptable digests.

### Corridor Agreement VC

Participant-specific Corridor Agreement VCs (`schemas/vc.corridor-agreement.schema.json`) include:

```json
"pinned_lawpacks": [
  {
    "jurisdiction_id": "us-ca",
    "domain": "civil",
    "lawpack_digest_sha256": "..."
  }
]
```

Each authority **signs** its pinned lawpack digests as part of its agreement VC payload.

Verification rules (`msez.corridor.verification.v1`) enforce:
- domain coverage (when `required_domains` is present),
- digest allowlists (when provided in definition VC).
