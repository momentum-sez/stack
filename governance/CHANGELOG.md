# Changelog

All notable changes to the **Momentum SEZ Stack (MSEZ)** will be documented in this file.

The format is based on *Keep a Changelog* and the project aims to follow semantic versioning for modules/profiles, while the **stack spec version** advances independently.

## Unreleased

- TBD

## 0.4.25

### Added
- `msez artifact graph verify`: artifact-closure graph verifier that can take a CAS root `(artifact_type,digest)` **or** a local JSON/YAML document, and emits a closure report (missing nodes, depth, counts).
- Optional `--strict` mode that recomputes artifact digests from on-disk content using the artifact’s canonical digest rules (e.g., JCS for JSON, VC signing-input, lawpack component canonicalization) to detect tampered CAS entries.

### Fixed
- `corridor state watcher-attest --store-artifacts` now correctly stores checkpoint artifacts into CAS (parameter mismatch fix).

### Version
- Stack spec version bumped to `0.4.25`.


## 0.4.24

- Expand `--transitive-require-artifacts` into a generic ArtifactRef closure verifier across receipts/attachments/VCs/checkpoints; treat transition-types registry lock digests as commitment roots.
- Extend `--require-artifacts` coverage to include receipt-level `zk.*` commitments (circuit / verifier key / proof blob).
- Add unit tests for transitive closure through attached VC artifacts and for receipt.zk artifact commitments.

## 0.4.23

### Added
- Deep transitive artifact completeness for transition type registry dependencies:
  - When a `transition-types.lock` snapshot references a *ruleset* digest, the verifier now also scans the resolved ruleset artifact for embedded `ArtifactRef` objects and requires those to be resolvable via artifact CAS.
  - This is forward-compatible hardening for proof-carrying transitions where rulesets commit to circuit/proof-key artifacts.
- Scenario scaffold scaling improvements:
  - Scenario scaffolds now skip at **module** level unless `MSEZ_RUN_SCAFFOLD=1` (keeps default CI fast).
  - Smart-asset integration scaffold matrix expanded (zones/rails/privacy/asset_class/custody axes).

### Fixed
- Removed accidental `__pycache__`/`*.pyc` artifacts from the repository tree (enforced via `.gitignore`).

### Tests
- Added unit test asserting that `--transitive-require-artifacts` detects missing nested ArtifactRefs inside ruleset artifacts referenced from a registry snapshot.

### Version
- Stack spec version bumped to `0.4.23`.





## 0.4.22

### Added
- `--transitive-require-artifacts` verification hardening:
  - Treats `transition_type_registry_digest_sha256` as a commitment root.
  - Verifies the referenced `transition-types.lock` snapshot exists *and* that all
    schema/ruleset/circuit digests referenced inside that lock can be resolved via artifact CAS.
- New test harness marker:
  - `scaffold` marker gated by `MSEZ_RUN_SCAFFOLD=1` (large scenario libraries for roadmap closure).
- Pre-0.5 roadmap integration:
  - `docs/roadmap/ROADMAP_PRE_0.5.md`
  - `docs/architecture/SMART-ASSET-INTEGRATION.md`
- Smart Asset scaffolds:
  - `apis/smart-assets.openapi.yaml`
  - `schemas/smart-asset.attestation.schema.json`
  - `schemas/smart-asset.checkpoint.schema.json`

### Fixed
- Receipt/fork inspection artifact resolution now uses the generic artifact CAS resolver.
- Attachment artifact completeness checks now accept both legacy string digests (blob) and typed `ArtifactRef` objects.

### Version
- Stack spec version bumped to `0.4.22`.




## 0.4.21

### Added
- `msez corridor state fork-inspect`: receipt-level fork forensics command (text or JSON output) emitting `MSEZCorridorForkInspectReport`.
- `schemas/corridor.fork-inspect-report.schema.json` for machine-validated fork forensics output.
- Performance harness support:
  - pytest markers `perf`/`slow` with environment-gated execution (`MSEZ_RUN_PERF=1`).
  - new perf tests for receipt verification throughput and watcher-compare scaling.
- New transition-type integrations (stub-ready but digest-stable):
  - `settle.swift.pacs008.v1` (SWIFT ISO20022 pacs.008 payload schema + ruleset)
  - `settle.usdc.circle.transfer.v1` (Circle USDC transfer payload schema + ruleset)
- Reference integration scaffolds:
  - `tools.integrations.swift_iso20022` (minimal pacs.008 XML bridge)
  - `tools.integrations.usdc_circle` (Circle adapter scaffold; dependency-light)

### Fixed
- Implemented missing trust-anchor loader and repaired receipt signature enforcement when `--enforce-trust-anchors` is enabled.
- Trust-anchor receipt enforcement now respects `corridor.yaml`'s `trust_anchors_path` and normalizes `identifier`→base DID.
- Corridor state CLI path handling now accepts either a corridor module directory or a direct `corridor.yaml` path.

### Version
- Stack spec version bumped to `0.4.21`.




## 0.4.20

### Added
- `tools.vc.load_proof_keypair(...)`: uniform loader for Ed25519 private JWK files returning `(private_key, verification_method)` for VC/receipt signing flows.
- CLI regression tests for signing subcommands:
  - `msez corridor state watcher-attest --sign`
  - `msez corridor state fork-alarm --sign`
  - `msez corridor availability-attest --sign`

### Fixed
- Signing paths for watcher attestations, fork alarms, and availability attestations now use the correct `add_ed25519_proof(vc, private_key, verification_method)` convention.
- `corridor availability-attest` now calls `corridor_expected_lawpack_digest_set(...)` with the correct signature.
- `msez law attest-init` no longer risks `NameError` due to missing `now_rfc3339` availability.

### Version
- Stack spec version bumped to `0.4.20`.




## 0.4.19

### Added
- Default corridor lifecycle state machine (`governance/corridor.lifecycle.state-machine.v1.json`) with evidence-gated HALT/RESUME transitions.
- `tools/lifecycle.py`: reference implementation for applying lifecycle transition VCs with evidence verification and policy enforcement.
- Unit tests for lifecycle evidence gating and fork resolution canonical chain selection.

### Changed
- `schemas/corridor.state-machine.schema.json` adds optional `requires_evidence_vc_types` for transition rules.
- `schemas/vc.corridor-lifecycle-transition.schema.json` adds optional `evidence` (ArtifactRef array).

### Fixed
- Receipt verification is now ArtifactRef-aware for digest sets and ProofResult-aware for signature verification (fixes `--require-artifacts` + typed attachments paths).
- Transition-type registry digest comparisons now coerce ArtifactRefs to digests.

### Version
- Stack spec version bumped to `0.4.19`.

## 0.4.18

### Added
- Typed transition envelopes (`transition-envelope.schema.json`) and typed attachment support via `ArtifactRef`.
- Receipt proposal artifacts (`corridor.receipt-proposal.schema.json`) for pre-signature negotiation.
- Fork resolution artifacts + VC wrapper (`corridor.fork-resolution.schema.json`, `vc.corridor-fork-resolution.schema.json`).
- Anchor VC (`vc.corridor-anchor.schema.json`) and finality status output schema (`corridor.finality-status.schema.json`).
- Additional corridor governance / ops schemas: lifecycle, lifecycle transitions, routing, watcher bond, dispute claim, arbitration award.

### Changed
- Corridor receipt verification is now fork-aware (canonical chain selection no longer requires receipts to be pre-sorted).
- Receipt transition payload schema is now referenced via `transition-envelope.schema.json`.

### CLI
- `msez corridor state propose`: generate unsigned receipt proposals.
- `msez corridor state fork-resolve`: generate unsigned fork-resolution VCs.
- `msez corridor state anchor`: generate unsigned corridor-anchor VCs.
- `msez corridor state finality-status`: compute a `MSEZCorridorFinalityStatus` for a corridor head.

## 0.4.17

### Added
- Watcher quorum + compact head commitments:
  - `schemas/vc.corridor-watcher-attestation.schema.json` extends `credentialSubject` with:
    - `genesis_root` (checkpoint-derived)
    - `head_commitment_digest_sha256` — deterministic digest over the corridor head fields so identical heads dedupe cleanly even if checkpoint timestamps differ.
  - `msez corridor state watcher-compare` gains quorum evaluation:
    - `--quorum-threshold` (e.g., `3/5` or `majority`)
    - `--max-staleness` (e.g., `1h`, `24h`, `PT1H`)
    - JSON report now includes `quorum` + per-watcher status flags.
- New schema: `schemas/watcher-compare-result.schema.json` for machine-readable watcher-compare output.
- New schema: `schemas/finality-level.schema.json` enumerating finality levels (for downstream systems to converge on common semantics).
- New documentation:
  - `docs/architecture/*` (overview, security model, legal integration, Mass integration)
  - `docs/operators/*` (zone deployment, corridor formation, incident response)

### Changed
- `msez corridor state watcher-attest` now emits `genesis_root` + `head_commitment_digest_sha256` by default.

### Version
- Stack spec version bumped to `0.4.17`.



## 0.4.16

### Added
- Watcher attestation aggregation:
  - `msez corridor state watcher-compare` — ingest multiple watcher attestation VCs and flag divergent `(receipt_count, final_state_root)` heads.
  - Supports `--fail-on-lag` (treat receipt-count divergence as failure), `--enforce-authority-registry` (optional allow-list), and `--require-artifacts` (commitment completeness for referenced checkpoints).

### Fixed
- `schema_validator(...)` call sites in checkpoint-related and availability-related commands now pass proper `Path` values (previous `str(...)` usage could break execution).

### Version
- Stack spec version bumped to `0.4.16`.



## 0.4.15

### Added
- Watcher integrity primitives:
  - `schemas/vc.corridor-watcher-attestation.schema.json` + `msez corridor state watcher-attest`
  - `schemas/vc.corridor-fork-alarm.schema.json` + `msez corridor state fork-alarm`
- Operational resilience primitive:
  - `schemas/vc.artifact-availability.schema.json`
  - `msez corridor availability-attest` and `msez corridor availability-verify`

### Changed
- Corridor Agreement VC schema adds `credentialSubject.state_channel.checkpointing` (checkpoint policy + thresholds).
- `msez corridor state verify` gains checkpoint-aware sync options:
  - `--from-checkpoint` to bootstrap verification from a prior signed checkpoint
  - `--checkpoint` to verify a head checkpoint matches the computed final root / receipt count
  - `--enforce-checkpoint-policy` to enforce signing thresholds and (when bootstrapping) receipt-gap bounds
- Authority registry support is extended for hierarchical chaining (treaty body → national → zone):
  - corridor.yaml `authority_registry_vc_path` now accepts an ordered list
  - `schemas/vc.authority-registry.schema.json` adds optional `credentialSubject.parent_registry_ref`

### Fixed
- Implemented `base_did()` helper used by authority registry verification.

### Version
- Stack spec version bumped to `0.4.15`.


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

