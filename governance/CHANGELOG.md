# Changelog

All notable changes to the **Momentum SEZ Stack (MSEZ)** will be documented in this file.

The format is based on *Keep a Changelog* and the project aims to follow semantic versioning for modules/profiles, while the **stack spec version** advances independently.

## Unreleased

- TBD


## 0.4.14

### Added
- `msez lock --emit-artifactrefs` to optionally emit **ArtifactRef** objects by default for digest-bearing fields:
  - Lawpack pins (`artifact_type: lawpack`)
  - Corridor artifacts (`artifact_type: blob`), including trust anchors, key rotation config, corridor manifest, corridor definition VC, and agreement VC(s)
- Receipt fork-resistance scaffolding:
  - Corridor Agreement VC templates now optionally include `credentialSubject.state_channel.receipt_signing.thresholds` (defaults to activation thresholds)
  - `msez corridor state verify --enforce-receipt-threshold` to enforce per-receipt multi-signer thresholds
- New VC schema stubs (legal + governance hardening substrates):
  - `schemas/vc.authority-registry.schema.json` (out-of-band signer authorization layer)
  - `schemas/vc.lawpack-attestation.schema.json` (legal validity attestations for lawpack digests)
- `msez law attest-init` helper to generate a lawpack attestation VC skeleton.

### Changed
- Stack spec version bumped to `0.4.14`.


## 0.4.13

### Added
- Stack lockfile and node descriptor schemas now accept **ArtifactRef** forms for key digest fields:
  - `schemas/stack.lock.schema.json` digest-bearing fields such as `lawpack_digest_sha256` and corridor artifact hashes.
  - `schemas/node.schema.json` entries for ZK verifier keys and attestation VCs.
- Transition type registry lock schema now accepts ArtifactRef forms for per-type digests (`schema`, `ruleset`, `circuit`).

### Changed
- Profiles updated to stack spec `0.4.13`.
- `tools/msez.py` `STACK_SPEC_VERSION` updated to `0.4.13`.


## 0.4.12

### Added
- Digest-bearing fields that historically carried bare sha256 strings MAY now carry an **ArtifactRef** in place of the raw digest (backwards compatible).


## 0.4.11

### Added
- A single reusable **ArtifactRef** schema (`schemas/artifact-ref.schema.json`) reused across receipts, VCs, lawpack metadata, and checkpoint/inclusion proof structures.


## 0.4.10

### Added
- Typed attachment artifact references in corridor receipts: `transition.attachments[*] = {artifact_type, digest_sha256, uri?, ...}`.
  - Enables attachments to commit to non-blob artifacts (e.g., `schema`, `vc`, `checkpoint`) without overloading raw-byte semantics.

### Changed
- Corridor receipt schema (`schemas/corridor.receipt.schema.json`) now accepts both:
  - Legacy attachments (no `artifact_type`; treated as `blob`)
  - Typed attachments (explicit `artifact_type`)
- `msez corridor state verify --require-artifacts` now resolves attachments using `artifact_type` when present (defaults to `blob` for legacy).


## 0.4.9

### Added
- New CAS artifact type: `blob` (`dist/artifacts/blob/<digest>.*`) for raw byte commitments such as:
  - `transition.attachments[*].digest_sha256`
  - `receipt.zk.proof_sha256`

### Changed
- `msez corridor state verify --require-artifacts` now treats attachment digests and ZK proof digests as `blob` artifacts (commitment completeness covers *all* receipt digest commitments).



## 0.4.8

### Added
- Expanded the recommended artifact CAS type set to include: `schema`, `vc`, `checkpoint`, and `proof-key` (`spec/97-artifacts.md`, `dist/artifacts/*`).
- New `msez artifact` helpers:
  - `artifact index-schemas` (materialize JSON Schemas into CAS)
  - `artifact index-vcs` (materialize VC payloads into CAS by payload digest)
- **Commitment completeness** mode for corridor receipt verification:
  - `msez corridor state verify ... --require-artifacts`
  - fails verification if any committed digest cannot be resolved via the artifact CAS.

### Changed
- Corridor and artifact documentation updated to describe commitment completeness and the expanded CAS type set.

## 0.4.7

### Added
- **Generic artifact CAS convention**: `dist/artifacts/<type>/<digest>.*` (`spec/97-artifacts.md`).
- New `msez artifact ...` commands:
  - `artifact store` (store by (type,digest))
  - `artifact resolve` (resolve by (type,digest))
  - `artifact index-rulesets` (materialize ruleset descriptors into CAS)
  - `artifact index-lawpacks` (materialize locally built lawpacks into CAS)
- Populated reference CAS directories under `dist/artifacts/` for example lawpacks, rulesets, and transition registry locks.

### Changed
- Transition type registry lock snapshots now default to storing and resolving from `dist/artifacts/transition-types` (legacy `dist/registries/transition-types` remains supported).
- Zone `stack.lock` generation prefers lawpack artifacts from `dist/artifacts/lawpack/...` when present.
- Profiles updated to stack spec `0.4.7`.

## 0.4.6

### Added
- Content-addressed storage + lookup for transition registry lock snapshots by digest (historical resolution).

## 0.4.5

### Added
- `transition-types.lock.json` (content-addressed registry snapshot) and support for corridor receipts to reference a registry digest rather than repeating per-transition digests.

## 0.4.4

### Added
- Transition type registry: optional schema digest, ruleset digest, and ZK circuit digest per `transition.kind`.

## 0.4.3

### Added
- Typed transition envelopes (generic) and inclusion-proof-friendly accumulator (MMR) for corridor state channels.

## 0.4.2

### Added
- Corridors formalized as verifiable state channels with corridor state roots and signed receipts.

## 0.4.1

### Added
- Lawpack supply chain scaffolding: deterministic ingestion to Akoma Ntoso + index + digest + lockfile.
- `msez law ingest` command.

## 0.4.0

### Added
- Synthesis of corridor definition VCs, corridor agreement VCs, threshold signing, and status locking.

## 0.3.0 (UNRELEASED)

(legacy placeholder)

