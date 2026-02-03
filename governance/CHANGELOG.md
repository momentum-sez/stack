# Changelog

All notable changes to the **Momentum SEZ Stack (MSEZ)** will be documented in this file.

The format is based on *Keep a Changelog* and the project aims to follow semantic versioning for modules/profiles, while the **stack spec version** advances independently.

## Unreleased

- TBD


## 0.4.44

### Added
- **Complete SEZ Module Families**: 21 new modules across 3 families:
  - Corporate Services (8 modules): entity-formation, registered-agent, corporate-secretarial, beneficial-ownership, corporate-governance, annual-compliance, dissolution, cap-table
  - Identity & Credentialing (6 modules): digital-identity, resident-credentials, progressive-kyc, professional-credentials, work-permits, identity-binding
  - Tax & Revenue (7 modules): tax-framework, zone-incentives, transfer-pricing, withholding, vat-gst, reporting, tax-treaties
- **Licensepack Infrastructure (spec/98-licensepacks.md)**: Live registry state management with real-time license status tracking, expiry monitoring, and compliance attestation
- **Multi-jurisdiction Composition Engine**: Complex zone deployments combining laws from multiple jurisdictions
- **AWS Deployment Automation**: Production-ready Terraform modules for EKS, RDS, ElastiCache, S3, KMS
- **PHOENIX GENESIS Codename**: Completing the SEZ-in-a-Box vision

### Changed
- Schema count increased to 116
- Module count increased to 86 (from 65)
- All profiles updated to v0.4.44

### Version
- Stack spec version bumped to `0.4.44` and all profiles updated.


## 0.4.43

### Added
- **PHOENIX Smart Asset Operating System**: Complete infrastructure for autonomous Smart Assets:
  - Compliance Tensor (tensor.py, 955 lines): 4D compliance state
  - ZK Proof Infrastructure (zkp.py, 766 lines): Circuit registry, proof generation
  - Compliance Manifold (manifold.py, 1,009 lines): Path planning
  - Migration Protocol (migration.py, 886 lines): Saga-based state machine
  - Corridor Bridge (bridge.py, 822 lines): Two-phase commit transfers
  - L1 Anchor (anchor.py, 816 lines): Settlement finality
  - Watcher Economy (watcher.py, 750 lines): Bonded attestations with slashing
  - Smart Asset VM (vm.py, 1,285 lines): Stack-based VM
  - Security Layer (security.py, 993 lines): Defense-in-depth
  - Hardening Layer (hardening.py, 744 lines): Validation, thread safety
- **9,221 lines of PHOENIX infrastructure** across 11 modules
- **92 new tests** for all PHOENIX components

### Changed
- Schema count increased to 110
- Total test count: 450+

### Version
- Stack spec version bumped to `0.4.43` and all profiles updated.


## 0.4.42

### Added
- **Agentic Execution Framework (Chapter 17)**: Complete autonomous asset behavior infrastructure:
  - Environment monitors: SanctionsListMonitor, LicenseStatusMonitor, CorridorStateMonitor, GuidanceUpdateMonitor, CheckpointDueMonitor
  - Policy evaluation engine with deterministic evaluation per Theorem 17.1
  - Action scheduling with retry semantics and authorization requirements
  - Audit trail generation for compliance and debugging
  - New module: `tools/agentic.py` (1,590 lines)
- **Extended Standard Policy Library**: 16 policies covering:
  - Sanctions freeze and notification
  - License suspension and renewal reminders
  - Corridor failover on fork detection
  - Automatic checkpointing (receipt and time thresholds)
  - Key rotation enforcement
  - Dispute and ruling handling
  - Settlement anchor notification
  - Watcher quorum checkpointing
  - Compliance deadline warnings
- **New Schemas** (6 schemas):
  - `agentic.environment-monitor.schema.json`
  - `agentic.trigger.schema.json`
  - `agentic.policy.schema.json`
  - `agentic.policy-evaluation.schema.json`
  - `agentic.action-schedule.schema.json`
  - `agentic.audit-trail.schema.json`
- **Specification Document**: `spec/17-agentic.md` — Complete MASS Protocol Chapter 17 specification
- **Test Coverage**: 95 new tests for agentic framework (total: 359 tests)

### Changed
- Schema count increased to 110

### Version
- Stack spec version bumped to `0.4.42` and all profiles updated.


## 0.4.41

### Added
- **Arbitration System (Chapter 26)**: Complete programmatic dispute resolution infrastructure:
  - Institution registry with DIFC-LCIA, SIAC, AIFC-IAC, ICC profiles
  - Dispute filing protocol with evidence packages per Definition 26.4/26.5
  - Arbitration ruling VCs with automatic enforcement per Definition 26.6
  - 9 arbitration transition kinds including EscrowRelease/EscrowForfeit per Definition 26.7
  - πruling ZK circuit schema (~35K constraints) per Definition 26.9
  - New schemas: `arbitration.*.schema.json` (10 schemas)
- **RegPack Integration (Chapter 20)**: Dynamic regulatory state management:
  - Sanctions list integration (OFAC/EU/UN) with SanctionsChecker
  - License registry and compliance calendar primitives
  - Regulator profile management
  - New schemas: `regpack.*.schema.json` (9 schemas)
- **Agentic Execution Primitives (Chapter 17)**:
  - AgenticTriggerType enum with 15 trigger categories
  - ImpactLevel, LicenseStatus, RulingDisposition enums
  - AgenticPolicy framework with STANDARD_POLICIES library
- **MASS Protocol Compliance**:
  - Protocol 14.1 (cross-jurisdiction transfer)
  - Protocol 16.1 (fork resolution with 4 strategies)
  - Protocol 18.1 (artifact graph verification)
  - Theorem 16.1 (offline operation) verification
  - Theorem 29.1 (identity immutability) verification
  - Theorem 29.2 (receipt chain non-repudiation) verification
- New `tools/mass_primitives.py` (1,630 lines): Complete MASS Protocol formal definitions
- New `tools/arbitration.py` (1,066 lines): Arbitration system implementation
- New `tools/regpack.py` (612 lines): RegPack implementation

### Changed
- Test coverage expanded to 264 tests
- Schema count increased to 104

### Version
- Stack spec version bumped to `0.4.41` and all profiles updated.


## 0.4.40

### Added
- **Trade Instrument Kit**: Canonical schemas for Invoice, Bill of Lading, Letter of Credit
  - Party and amount schemas
  - Transition payload schemas and rulesets for invoice/BOL/LC lifecycle transitions
  - Registered transition kinds in transition-types registry
- **Corridor-of-corridors settlement plans**: MSEZCorridorSettlementPlan schema with deterministic netting and settlement legs
  - CLI: `settlement-plan-init`, `settlement-plan-verify`
  - Attach settlement plans to corridor receipts
- **Strict verification semantics**: Bughunt gates for production operator ergonomics

### Changed
- Registry hardening: regenerated `transition-types.lock.json` for trade + settlement primitives
- Operator correctness improvements

### Version
- Stack spec version bumped to `0.4.40` and starter profiles updated.


## 0.4.39

### Added
- **Cross-corridor settlement anchoring**: MSEZ settlement-anchor schema + typed attachment
  - CLI: `settlement-anchor-init`, `settlement-anchor-verify`
  - Attach anchors to corridor receipts for externalized settlement finality
- **Proof binding primitives**: MSEZ proof-binding schema + typed attachment
  - CLI: `proof-binding-init`, `proof-binding-verify`
  - Enable replay-resistant binding of external proofs/VCs/blobs to corridor/asset commitments

### Version
- Stack spec version bumped to `0.4.39` and starter profiles updated.


## 0.4.38

### Added
- **Cross-corridor settlement anchoring** primitives:
  - New schemas: `corridor.checkpoint.attachment`, `corridor.settlement-anchor`, and `proof-binding` (+ attachment wrappers).
  - New CLI: `msez proof-binding init|verify` (portable, replay-resistant proof linking to commitment digests).
  - New CLI: `msez corridor settlement-anchor-init|settlement-anchor-verify` (bind obligation corridor checkpoint to settlement corridor checkpoint, with optional proof bindings).
- Corridor receipt UX: `msez corridor state receipt-init` gains typed attachment flags for corridor checkpoints, proof bindings, and settlement anchors.

### Changed
- Artifact graph strict mode gains semantic digest support for:
  - `proof-binding` (sha256(signing_input(binding)))
  - `settlement-anchor` (sha256(signing_input(anchor)))

### Version
- Stack spec version bumped to `0.4.38` and starter profiles updated.


## 0.4.36

### Added
- SmartAssetReceipt multi-jurisdiction scope hints: `jurisdiction_scope`, `harbor_ids`, `harbor_quorum` (schema + `msez asset state receipt-init` flags).
- Smart Asset registry VC quorum policy (`credentialSubject.quorum_policy`) + `msez asset registry-init --quorum-policy`.
- Portable rule evaluation evidence artifacts: `rule-eval-evidence` schemas + `msez asset rule-eval-evidence-init` (optionally sign + store).

### Changed
- `msez asset compliance-eval` now evaluates **active** bindings by default (and applies quorum policy when present).
- Artifact graph strict mode gains semantic digest support for `rule-eval-evidence` (sha256 of JCS signing input).



## 0.4.35

### Added
- **Directory roots for artifact graph verification**: `msez artifact graph verify --path <dir>` now scans all structured files (JSON/YAML) under the directory for embedded `ArtifactRef`s and resolves the full closure.
- **Directory roots in witness bundles**: when the closure root is a directory, witness bundles include the scanned structured root files under `root/<dirname>/...` (in addition to `manifest.json` and `artifacts/...`).
- **Operator UX**: new `msez asset module witness-bundle` command emits a portable audit packet for an asset module directory (receipts/checkpoints/proofs + referenced artifacts).

### Changed
- `MSEZArtifactGraphVerifyReport.root.mode` now supports `dir` in addition to `cas` and `file`.

### Version
- Stack spec version bumped to `0.4.35` and starter profiles updated.


## 0.4.34

### Added
- **Corridor anchoring for Smart Asset receipt-chain checkpoints**:
  - New CLI: `msez corridor state receipt-init --attach-smart-asset-receipt-checkpoint <checkpoint.json>` (repeatable) appends a typed transition attachment (`artifact_type: smart-asset-receipt-checkpoint`) committing to the checkpoint payload digest (excluding proof).
  - Convenience: `msez corridor state receipt-init --attach-smart-asset-checkpoint <asset_checkpoint.json>` (repeatable) appends the existing `smart-asset-checkpoint` typed attachment for state-root anchoring.
- **Anchor verification upgrades**:
  - `msez asset anchor-verify` now supports `--asset-receipt-checkpoint` (SmartAssetReceiptChainCheckpoint) and verifies that its typed attachment is present on the corridor receipt.
  - Optional “portable audit packet” path: if `--asset-receipt` + `--asset-inclusion-proof` are provided, `msez asset anchor-verify` also verifies receipt inclusion against the anchored checkpoint.
- New schema: `schemas/smart-asset.receipt-checkpoint.attachment.schema.json` documents the typed attachment shape.

### Changed
- Smart Asset receipt inclusion proofs now emit `checkpoint_ref.artifact_type: smart-asset-receipt-checkpoint` (schema accepts both legacy `checkpoint` and the typed variant).
- Artifact strict digest verification now treats `smart-asset-receipt-checkpoint` with the same semantic digest rule as `checkpoint` (sha256(JCS(checkpoint_without_proof))).

### Version
- Stack spec version bumped to `0.4.34` and starter profiles updated.

## 0.4.33

### Added
- **Smart Asset fork resolution** (receipt-chain concurrency):
  - New schema: `schemas/smart-asset.fork-resolution.schema.json` (+ VC wrapper schema).
  - New CLI: `msez asset state fork-resolve` (alias `fork-resolution-init`) emits an unsigned VC selecting the canonical `next_root` for a forked `(sequence, prev_root)` point.
  - `msez asset state verify`, `checkpoint`, and `inclusion-proof` now accept `--fork-resolutions` (file/dir) and apply corridor-style canonical selection semantics.

### Changed
- Smart Asset receipt-chain verification is now **fork-aware**:
  - duplicate receipts (same `next_root`) are merged;
  - forks require explicit fork resolution;
  - non-canonical/unreachable receipts are reported as warnings (not fatal).

### Version
- Stack spec version bumped to `0.4.33`.

## 0.4.32

### Added
- **Smart Asset module directory** scaffolding (`modules/smart-assets/<asset_id>/...`) with `asset.yaml`.
- New operator UX: `msez asset module init <asset_id>` (template-based scaffolding).

### Changed
- Stack spec version bumped to `0.4.32`.

## 0.4.31

### Added
- **Smart Asset receipt-chain primitives (non-blockchain)**:
  - Schemas for receipts, MMR checkpoints, and inclusion proofs.
  - CLI: `msez asset state {genesis-root,receipt-init,verify,checkpoint,inclusion-proof,verify-inclusion}`.
- Artifact graph strict mode now treats `smart-asset-receipt` digests semantically (`receipt.next_root`).

### Version
- Stack spec version bumped to `0.4.31`.

## 0.4.30

### Added
- **Smart Asset corridor anchoring (non-blockchain)**:
  - `msez asset anchor-verify` validates that a Smart Asset checkpoint digest is present as a typed attachment on a corridor receipt, and that the receipt is included in a corridor checkpoint via an MMR inclusion proof.
  - This provides a clean bridge between **asset-centric state** and **corridor state channels**, enabling redundant, cross-jurisdiction custody / audit trails.
- **Smart Asset DAG checkpoints (multi-parent) hardened**:
  - `msez asset checkpoint-build` now validates/normalizes parent digests (de-duped + sorted) to keep checkpoint graphs deterministic and schema-conformant.
- **Artifact graph strict verification hardened for Smart Asset artifact types**:
  - `msez artifact graph verify --strict` now recomputes digests for `smart-asset-genesis`, `smart-asset-checkpoint`, and `smart-asset-attestation` according to their artifact-specific semantics.

### Changed
- Smart Asset compliance evaluation accepts both the legacy `TransitionEnvelope` shape (`transition_kind`) and the stack-standard `MSEZTransitionEnvelope` shape (`kind`), and can unwrap corridor receipts (`MSEZCorridorStateReceipt`) by evaluating their embedded `transition`.
- Smart Asset `asset_id` derivation is now stable when the genesis document includes an informational `asset_id` field: the derived field is excluded from the digest commitment (sha256(JCS(genesis-without-asset_id))).
- Corridor state `corridor state verify-inclusion` correctly handles receipt sequence/index 0 (no falsy-default bug for `leaf_index` and `sequence`).

### Version
- Stack spec version bumped to `0.4.30`.

## 0.4.29

### Added
- **Smart Asset reference layer (non-blockchain)**:
  - Asset identity: `schemas/smart-asset.genesis.schema.json` (asset_id = `sha256(JCS(genesis))`).
  - Jurisdictional registry VC: `schemas/vc.smart-asset-registry.schema.json` (binds asset -> harbors + lawpacks + compliance/enforcement profiles).
  - Operational manifest: `schemas/smart-asset.manifest.schema.json` (node-local config; optional).
- **New CLI surface**: `msez asset ...`
  - `asset genesis-init`, `asset genesis-hash`
  - `asset registry-init` (optionally signs VC)
  - `asset checkpoint-build` (state_root = `sha256(JCS(state))`)
  - `asset attestation-init` (optionally store in CAS)
  - `asset compliance-eval` (declarative multi-jurisdiction compliance check)

### Version
- Stack spec version bumped to `0.4.29`.

## 0.4.28

### Added
- Witness-bundle provenance VC + CLI:
  - `msez artifact bundle attest <bundle.zip> ...` emits a Verifiable Credential (`MSEZArtifactWitnessBundleCredential`) committing to `SHA256(JCS(manifest.json))`.
  - `msez artifact bundle verify <bundle.zip> --vc <vc.json>` verifies digest match and (optionally) VC signatures.
- JSON Schema for witness-bundle attestation VCs:
  - `schemas/vc.artifact-witness-bundle.schema.json`.

### Documentation
- `spec/97-artifacts.md` extended to document witness bundles and witness bundle attestations.

### Version
- Stack spec version bumped to `0.4.28`.

## 0.4.27

### Added
- Witness-bundle verification mode for artifact closure graphs:
  - `msez artifact graph verify --from-bundle <zip>` extracts a witness bundle, adds its `artifacts/` tree as an offline CAS root, and verifies closure integrity without manual extraction.
  - If no explicit root is provided, the command infers the root from the bundle’s `manifest.json` (CAS root or file root).
- Canonical JSON Schema for `manifest.json`:
  - `schemas/artifact.graph-verify-report.schema.json` validates `MSEZArtifactGraphVerifyReport` manifests (best-effort validated in `--from-bundle` mode).

### Version
- Stack spec version bumped to `0.4.27`.

## 0.4.26

### Added
- `msez artifact graph verify` enhancements:
  - `--emit-edges`: include a machine-readable edge list (`edges[]`) so closure graphs can be analyzed programmatically.
  - `--bundle <zip>`: emit a self-contained witness bundle containing `manifest.json` + resolved artifacts under `artifacts/<type>/` for offline verification and transfer between environments.
  - `--bundle-max-bytes`: optional size cap to prevent accidental bundling of very large closures (0 = unlimited).

### Version
- Stack spec version bumped to `0.4.26`.

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

