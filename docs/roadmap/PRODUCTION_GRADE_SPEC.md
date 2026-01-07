# Production-grade evolution spec

This document is a forward-looking spec for evolving the Momentum SEZ Stack (MSEZ) from a reference implementation into a production-grade system.

It is written to be **predictive** (anticipates failure modes and scaling constraints), **roadmap-generative** (breaks work into shippable increments), and consistent with the stack’s philosophy: **least resistance + true generality**.

---

## North stars

1. **Non-blockchain by default**: all critical guarantees (integrity, ordering, authorization, auditability) are provided without requiring a global consensus chain.
2. **Jurisdictionally composable**: compliance, enforcement, and dispute resolution can be *sharded* across jurisdictions and operators, while remaining verifiable end-to-end.
3. **Artifact-graph everything**: every claim reduces to a small set of content-addressed artifacts + verifiable proofs + optional witness bundles.
4. **Determinism at the edges**: canonicalization, digest semantics, schema validation, and rule evaluation must be deterministic and replayable.
5. **Audits are cheap**: inclusion proofs and compact bundles enable verifiable audits without full replication.

---

## System decomposition

### Layer 0: Content addressed artifacts

**Primitives**

- ArtifactRef (`{artifact_type, digest_sha256, uri?}`)
- Store roots (`dist/artifacts`, S3 prefixes, IPFS, etc.)
- Artifact-graph closure + witness bundles (zip + manifest + attestation)

**Guarantees**

- Portability: any verifier can reconstruct the closure from store roots.
- Tamper evidence: strict semantic digests (for domain objects) detect rewriting.

### Layer 1: Verifiable registries

**Primitives**

- Transition Type Registry (lock)
- Smart Asset Registry VC (jurisdiction bindings)

**Guarantees**

- Versionable policy: “what transitions exist” is explicit, pinned, and hash-addressed.

### Layer 2: Receipt chains

**Primitives**

- Corridor receipts (existing)
- Smart Asset receipts (v0.4.31)
- Fork resolution credentials (corridor exists; smart asset planned)

**Guarantees**

- Append-only history with local ordering.
- Offline-first progression: receipts can be generated without network coordination.

### Layer 3: Accumulators + checkpoints

**Primitives**

- Merkle Mountain Range (MMR) over receipt `next_root` values
- Signed chain checkpoints
- Inclusion proofs (receipt ∈ checkpoint)

**Guarantees**

- Log compression: checkpoint commits to *many* receipts.
- Audit efficiency: verify inclusion with O(log n) proof.

### Layer 4: Anchoring + cross-domain binding

**Primitives**

- Typed attachments on corridor receipts
- Smart asset checkpoint anchoring in corridor state (existing)
- Planned: smart asset receipt-chain checkpoint anchoring

**Guarantees**

- Cross-domain integrity: corridor state can anchor asset state.

---

## Failure modes the roadmap must anticipate

1. **Forks from concurrency**: multiple parties issue receipts at the same sequence/prev_root.
2. **Partial replication**: different observers see different subsets of receipts.
3. **Key compromise and rotation**: receipts/checkpoints must support multiple proofs and key rollover.
4. **Schema drift**: policy/registry upgrades must be explicit and replayable.
5. **Jurisdictional unavailability**: a harbor/authority may be offline; history must still advance with later attestations.
6. **Adversarial bundling**: malicious bundles omit critical artifacts; closure verification must catch this.

---

## Roadmap increments

### v0.4.32 — Asset module layout + replication ergonomics

**Why**: reduce friction for operators; turn conventions into tooling.

Deliverables

- `modules/smart-assets/<asset_id>/` template
  - `asset.yaml` (asset_id, purpose(s), trust-anchors path, optional defaults)
  - `state/receipts/`, `state/checkpoints/`, `state/proofs/`
- CLI: allow `msez asset state ... modules/smart-assets/<asset_id>` (like corridors)
- Docs/examples: end-to-end directory example

### v0.4.33 — Smart asset fork resolution credential

**Why**: receipt chains become robust under concurrency + redundant writers.

Deliverables

- `smart-asset.fork-resolution.schema.json` (+ VC wrapper schema)
- CLI: `msez asset state fork-resolve` (alias: `fork-resolution-init`)
  - `msez asset state verify --fork-resolutions ...`
  - `msez asset state checkpoint --fork-resolutions ...`
  - `msez asset state inclusion-proof --fork-resolutions ...`
- Deterministic chain selection algorithm (match corridor semantics where possible)
- Asset module template includes `state/fork-resolutions/`
- Tests: forked asset receipt chain resolved via fork-resolution VC

### v0.4.34 — Asset receipt-chain checkpoint anchoring

**Why**: enable “asset state proven inside corridor state” without shipping the entire asset history.

Deliverables

- Typed attachment schema for `SmartAssetReceiptChainCheckpoint` attachment
- CLI: `msez corridor state receipt-init --attach-smart-asset-receipt-checkpoint ...`
- Verify: `msez asset anchor-verify` extended to accept chain checkpoints + inclusion proofs

### v0.4.35 — Witness bundle for asset histories

**Why**: production audits need a single portable artifact.

Deliverables

- Witness bundle workflows (portable audit packets):
  - `msez asset module witness-bundle modules/smart-assets/<asset_id> --out <bundle.zip>`
  - Generic closure bundling: `msez artifact graph verify --path <dir> --bundle <bundle.zip>`
- “Asset history bundle attestation” profile template (who can attest bundles, quorum rules):
  - `profiles/asset-history-bundle-attestation/profile.yaml`

### v0.4.36 — Multi-jurisdiction compliance receipts

**Why**: make sharded compliance explicit and verifiable.

Deliverables

- Receipt field conventions for jurisdiction scopes (e.g., `jurisdiction_scope`, `harbor_ids`)
- Optional rule evaluation evidence attachment (outputs, hashes, zk commitments)
- Policy for “quorum of harbors” per transition class (ruleset-driven)

---

## Production hardening checklist

### Cryptography + canonicalization

- Canonical JSON (JCS) everywhere a digest is defined.
- Strict semantic digests for domain objects (genesis/checkpoint/attestation/receipts).
- Multi-proof support and key rotation patterns.

### Storage + transport

- Deterministic artifact layout in store roots.
- Witness bundle format stability (manifest schema + hash commitments).
- Optional streaming transport (HTTP range requests, S3 signed URLs) for large bundles.

### API + operator UX

- "One command" workflows for operators:
  - init asset module
  - append receipt
  - checkpoint
  - generate proof
  - build witness bundle
- Clear error taxonomy (schema error vs signature error vs missing artifact vs fork).

### Testing + verification

- Property tests: chain invariants, fork selection, proof verification.
- Cross-implementation vectors: fixtures that a second implementation can verify.
- Fuzzing: malformed receipts, adversarial bundles, invalid proofs.

---

## Design rules

1. **No hidden state**: anything needed to verify must be present as artifacts or proofs.
2. **Explicit versioning**: schema ids and lock digests are always pinned.
3. **Graceful partial knowledge**: verifiers can say “unknown” rather than “false” when artifacts are missing.
4. **Do the simplest thing that composes**: start with linear chains and optional fork credentials; later add richer coordination.
