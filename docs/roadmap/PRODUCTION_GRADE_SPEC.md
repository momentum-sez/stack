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

**Goal:** turn “receipt chains + registry VCs” into a *jurisdiction-sharded, fault-tolerant compliance fabric* — where a transition can be evaluated, evidenced, and later audited across multiple harbors (jurisdictions), without requiring a blockchain.

**Deliverables (implemented):**

- **Receipt field conventions for jurisdiction scopes** in `SmartAssetReceipt`:
  - `jurisdiction_scope`: `all_active | subset | quorum`
  - `harbor_ids`: list of harbor/jurisdiction IDs relevant to this receipt’s compliance scope
  - `harbor_quorum`: optional quorum threshold when `jurisdiction_scope=quorum`
  - Implemented in `schemas/smart-asset.receipt.schema.json` and supported by `msez asset state receipt-init`.

- **Quorum policy in the Smart Asset Registry VC** (`vc.smart-asset-registry.schema.json`):
  - `credentialSubject.quorum_policy.default` and `credentialSubject.quorum_policy.by_transition_kind[kind]`
  - Evaluated by `msez asset compliance-eval` (counts **active** bindings only by default; quorum policy further restricts eligible sets).
  - `msez asset registry-init` supports `--quorum-policy <yaml|json>` to embed policy at issuance time.

- **Rule evaluation evidence as a portable, attachable artifact**:
  - New schemas: `schemas/rule-eval-evidence.schema.json` and `schemas/rule-eval-evidence.attachment.schema.json`
  - New CLI: `msez asset rule-eval-evidence-init` to create (optionally sign) `MSEZRuleEvaluationEvidence`.
  - CAS artifact type: `rule-eval-evidence` with **semantic digest** `sha256(JCS(evidence_without_proof))`.
  - Evidence artifacts can be referenced from any transition envelope via `attachments` (and will be carried in witness bundles).

**Why it matters:**
- You can now run “compliance as replicated consensus” across harbors:
  - an asset log head can advance with a defined quorum,
  - each harbor can emit portable evaluation evidence,
  - witness bundles can carry the full audit packet (receipts + checkpoints + evidence + referenced artifacts).

---

### v0.4.37 — Trade corridor instruments kit (baseline)

**Goal:** make the stack immediately useful for end-to-end trade rails by shipping a first-class “trade corridor vocabulary” (schemas + transition kinds + examples), aligned with common documents and operational steps.

**Deliverables:**

- **Trade document schemas (non-token, non-blockchain)** as signed/hashed artifacts:
  - Commercial Invoice, Purchase Order, Packing List
  - Bill of Lading / Air Waybill (B/L-AWB)
  - Certificate of Origin
  - Insurance Certificate
  - Customs Entry / Release Notice
  - Warehouse Receipt (tokenized or not)

- **Trade transitions (typed envelopes + rulesets)**:
  - `trade.order.place.v1`, `trade.order.accept.v1`
  - `trade.shipment.dispatch.v1`, `trade.shipment.arrive.v1`
  - `trade.customs.clear.v1`
  - `trade.settlement.initiate.v1`, `trade.settlement.confirm.v1`

- **Corridor module templates** for common flow archetypes:
  - Export (seller) corridor, Import (buyer) corridor
  - L/C-like instrument corridor (documentary conditions + release)
  - Open-account corridor (invoice + milestone + settlement)

---

### v0.4.38 — Capital corridor primitives (settlement + netting surface)

**Goal:** express high-volume capital flows (FX, stablecoin settlement, bank rails) as corridor transitions with auditable evidence and policy hooks.

**Deliverables:**

- **Settlement instruction + confirmation schemas** (ISO-20022-inspired but simplified)
- **Netting session artifacts** (batch definitions + netting proofs + reconciliation outputs)
- **Corridor state-machine patterns** for:
  - prefunded vs. postfunded settlement
  - DvP/PvP gating (atomicity via evidence/quorum rather than blockchain)
- **Inter-corridor anchoring**: allow settlement corridors to “anchor” trade corridor checkpoints.

---

### v0.4.39 — Arbitration corridors and dispute casefiles

**Goal:** make disputes first-class and portable: a “casefile” witness bundle that can be replayed by arbitrators across jurisdictions.

**Deliverables:**

- **Casefile schema** (claims, timeline, attachments, requested remedies)
- **Arbitration corridor template** (intake → evidence → hearing → award)
- **Award artifact** signed by arbitrator DID(s), referenced as attachments by trade/capital corridors.

---

### v0.4.40 — Production operator ergonomics and hardening

**Goal:** make operators successful by default (init flows, linting, CI, reproducibility, and safety rails).

**Deliverables:**

- `msez lint` / `msez doctor` for:
  - schema validation
  - CAS completeness
  - fork-resolution hygiene
  - quorum policy correctness
- Deterministic bundle builds (hash-stable zip manifests)
- Expanded examples + walkthroughs.

**Release gate:** `docs/roadmap/PREREQS_TO_SHIP_V0.40.md` is authoritative. Do **not** bump the version number until the checklist is complete and CI passes in strict/check modes.

