# Corridors overview

A **corridor** is a cross-border interoperability arrangement between zone nodes.

This stack provides corridor modules for:

- SWIFT / ISO20022 messaging (`modules/corridors/swift/`)
- stablecoin settlement corridors (`modules/corridors/stablecoin-settlement/`)
- open-banking / account-to-account corridors (`modules/corridors/open-banking/`)

Each corridor module contains:

- a `corridor.yaml` manifest (machine-readable)
- a playbook (`docs/playbook.md`) for legal, compliance, and ops steps
- a `required-attestations.yaml` list (what proofs are required for corridor eligibility)

Corridor modules can be made **cryptographically meaningful** by attaching verifiable credentials:

- `corridor.vc.json` (Corridor Definition VC): binds the corridor manifest + security artifacts by hash and signature
- optional Corridor Agreement VC(s): binds participant acceptance + activation thresholds (e.g., 2-of-2 zone authorities)


## Verifiable corridor operations (state channels)

Beyond VCs (definition + agreement), corridor operations can be modeled as a **verifiable state channel**:

- every transition produces a signed receipt
- receipts deterministically update a `corridor_state_root`
- receipts bind to `lawpack_digest_set` and `ruleset_digest_set` to prevent “floating law” and “floating verifier logic”
- ZK proofs and/or external-chain anchoring can be layered later without redesign

### Typed transitions (v0.4.3+)

Receipts SHOULD carry a typed transition envelope (`type: MSEZTransitionEnvelope`) with:

- `kind` (corridor-specific transition type)
- `payload_sha256` (commitment to the payload)
- optional inline `payload` (which must match `payload_sha256` via `SHA256(JCS(payload))`)

This makes receipts machine-parseable while keeping corridors generic.

### Transition type registry (v0.4.4+)

To support interoperable transition kinds across ecosystems without making corridor receipts corridor-specific,
corridors MAY publish a **transition type registry** mapping `transition.kind` to optional digest references:

- `schema_digest_sha256` (payload format)
- `ruleset_digest_sha256` (validation semantics)
- optional `zk_circuit_digest_sha256` (proof-carrying transitions)

If configured via `state_channel.transition_type_registry_path`, the corridor definition VC SHOULD pin the registry
as an artifact so kind semantics are bound into the corridor's genesis substrate.

### Transition type registry lock (v0.4.5+)

For cryptographic compactness, corridors SHOULD also publish a **content-addressed registry lock**
(`transition-types.lock.json`) and pin it in the corridor definition VC.

Receipts can then reference a single `transition_type_registry_digest_sha256` commitment (the lock's
`snapshot_digest_sha256`) instead of repeating schema/ruleset/circuit digests per transition.

### Content-addressed lock distribution (v0.4.7+)

Receipts MAY reference **historical** transition registry snapshots by digest even if a corridor module later updates.
To keep those receipts verifiable over time, operators SHOULD publish referenced registry lock snapshots in a
content-addressed store.

Reference repository path:

- `dist/artifacts/transition-types/<digest>.transition-types.lock.json`

Reference tooling:

- `msez registry transition-types-store <lock.json>` (write `<digest>.transition-types.lock.json` into the store)
- `msez registry transition-types-resolve <digest>` (locate a snapshot by digest)
- `msez artifact resolve transition-types <digest>` (generic resolver; searches `dist/artifacts`)

The resolver also consults `MSEZ_ARTIFACT_STORE_DIRS` (os.pathsep-separated) so deployments can use external
artifact stores (HTTP mirror, object store, IPFS gateway/pinset, etc.).

For backwards compatibility, transition-type resolution also honors `MSEZ_TRANSITION_TYPES_STORE_DIRS` (legacy).

### Commitment completeness (v0.4.8+)

To make digest commitments *operationally meaningful*, verifiers MAY enforce that **every committed digest**
can be resolved via the artifact CAS (`dist/artifacts/<type>/<digest>.*`), rather than treating digests as
purely declarative.

Reference tooling:

- `msez corridor state verify ... --require-artifacts`

In this mode, verification fails if any referenced digest (lawpacks, rulesets, transition registry snapshots,
schemas, circuits, proof keys, and attachments / proof bytes) cannot be located by `(type, digest)`.

Notes (v0.4.10+):
- `transition.attachments[*]` are **typed artifact references**: `{artifact_type, digest_sha256, ...}`.
- For backward compatibility, if an attachment omits `artifact_type`, verifiers treat it as `artifact_type = "blob"`.
- ZK proof bytes (`receipt.zk.proof_sha256`) remain `blob` artifacts.

### Inclusion proofs (MMR) (v0.4.3+)

A plain hash-chain root is great for ordering but does not yield compact inclusion proofs. To support
"prove receipt _i_ exists" without disclosing all other receipts, corridors SHOULD maintain an
append-only **Merkle Mountain Range (MMR)** over receipt digests (`next_root`) and publish signed
**checkpoint** objects committing to the MMR root.

Reference tooling:

- `msez corridor state checkpoint ...` (produce a checkpoint with `mmr.root`)
- `msez corridor state proof ...` (produce an inclusion proof for a receipt index)
- `msez corridor state verify-inclusion ...` (verify proof + receipt + checkpoint)

See `spec/40-corridors.md`.
