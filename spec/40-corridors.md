# Corridors (normative)

A **corridor** is a cross-border interoperability arrangement between zone nodes and external participants.

Corridors MUST define (at minimum):

- settlement model and limits
- required attestations and trust anchors
- dispute escalation path
- key rotation and revocation policy
- data-sharing expectations (minimization)

Corridor manifests MUST conform to `schemas/corridor.schema.json`.

Trust anchors MUST be expressed as machine-readable artifacts conforming to:

- `schemas/trust-anchors.schema.json`
- `schemas/key-rotation.schema.json`

### External signer authorization (authority registry) (v0.4.15+)

The stack historically authorizes corridor signers via the corridor module's `trust-anchors.yaml`.
This is sufficient for *technical* verification, but it creates a bootstrapping/circularity risk:

- `trust-anchors.yaml` authorizes the keys that sign `corridor.vc.json`, yet `trust-anchors.yaml` itself ships *inside* the same module.

To mitigate this, corridor manifests MAY additionally reference an out-of-band **Authority Registry VC**:



Chaining semantics (non-normative, but enforced by the reference tool when a list is provided):

- Each registry VC MAY include `credentialSubject.parent_registry_ref`.
- When a corridor supplies a list, the tool treats the **last** VC as the effective allow-list for corridor attestations.
- For each adjacent pair in the chain, the *child* registry issuer MUST be authorized by the *parent* registry with `allowed_attestations` containing `authority_registry` (or wildcard `*`).

This supports hierarchical delegation (treaty body → national authority → zone authority) while keeping corridor packages self-contained.
- `authority_registry_vc_path: <path>`
- `authority_registry_vc_path: [<treaty.vc.json>, <national.vc.json>, <zone.vc.json>]` (ordered chain: parent → child)

Chaining semantics (v0.4.15+):

- The registry chain represents hierarchical delegation: **treaty body → national authority → zone authority**.
- For each adjacent pair in the chain, the *child* registry issuer MUST be authorized by the *parent* registry with `allowed_attestations` containing `authority_registry` (or wildcard `*`).
- The **last** registry in the list is treated as the effective allow-list for corridor attestations.

If present, verifiers SHOULD ensure that:

- any signer used for `corridor_definition`, `corridor_agreement`, and/or `corridor_receipt` is listed in the authority registry under the corresponding `allowed_attestations`, and
- `trust-anchors.yaml` does not introduce signers that are *not* present in the authority registry.

The reference tool can enforce this constraint during corridor package verification.

## Cryptographically meaningful corridors (VC binding)

Corridor manifests are **declarative**. To make a corridor *cryptographically meaningful* (tamper-evident and
verifiable), every corridor MUST also ship a **Corridor Definition Verifiable Credential (VC)** that binds:

- the `corridor.yaml` manifest
- `trust-anchors.yaml`
- `key-rotation.yaml`

…to the corridor’s `corridor_id` using content hashes and a digital signature.

### Required files

Every corridor module MUST include:

- `corridor.yaml` (manifest)
- `trust-anchors.yaml` (trust anchors / issuer authorization)
- `key-rotation.yaml` (rotation + revocation policy)
- `corridor.vc.json` (a signed Corridor Definition VC)

Deployments MAY additionally configure corridor agreement VCs (see below) in order to make **activation**
cryptographically meaningful.

## Corridor Definition VC (normative)

The Corridor Definition VC:

- MUST conform to `schemas/vc.corridor-definition.schema.json`
- MUST include SHA256 hashes for:
  - `corridor.yaml`
  - `trust-anchors.yaml`
  - `key-rotation.yaml`
- MUST have at least one valid proof (`proof`) using `did:key` (offline verifiable)
- Each signer MUST be listed in `trust-anchors.yaml` with `allowed_attestations` containing `corridor_definition`

### Lawpack compatibility binding (v0.4.1+)

Corridors frequently depend on the **exact governing-law snapshot** in force for each participant.
To avoid “floating law”, Corridor Definition VCs MAY include a lawpack compatibility clause:

- `credentialSubject.lawpack_compatibility.required_domains` — domains each participant must pin (e.g., `civil`, `financial`)
- `credentialSubject.lawpack_compatibility.allowed[]` — optional allow-list of compatible lawpack digests by jurisdiction and domain

For consistency with the ArtifactRef pattern, allow-list entries (`digests_sha256[]`) MAY be expressed as either raw sha256 digest strings (legacy) OR as ArtifactRef objects with `artifact_type: lawpack`.

See `spec/96-lawpacks.md` for digest semantics and indexing.

### Hash computation (normative)

Implementations MUST compute file digests deterministically.

- For JSON artifacts: digest MUST be `SHA256(MCF(json))` where MCF is **Momentum Canonical Form** —
  RFC 8785 JSON Canonicalization Scheme (JCS) with two additional safety coercions:
  1. **Float rejection**: numeric values that are f64-only (not representable as i64/u64) MUST be rejected.
  2. **Datetime normalization**: strings that parse as RFC 3339 timestamps MUST be normalized to UTC with `Z` suffix, truncated to whole seconds.

  These coercions are normative deviations from pure RFC 8785 JCS. Any cross-language implementation
  MUST replicate these exact coercions to produce matching digests. See `msez-core::canonical` for the
  reference implementation and `msez-core::canonical::tests` for cross-language golden vectors.

- For YAML artifacts: implementations MUST parse YAML into an equivalent JSON data model and digest MUST be
  `SHA256(MCF(json_model))`.

For Corridor Agreement binding, the **definition payload hash** MUST be computed over the Corridor Definition VC
**payload excluding** `proof`:

- `definition_payload_sha256 = SHA256(MCF(definition_vc_without_proof))`

The reference tool exposes this as:

- `msez vc payload-hash <vc.json>`

## Corridor Agreement VC (normative)

A Corridor Agreement VC captures **participant-specific acceptance** of a specific corridor definition and defines
the **activation rule** for bringing the corridor online.

A Corridor Agreement VC:

- MUST conform to `schemas/vc.corridor-agreement.schema.json`
- MUST bind to a specific Corridor Definition VC payload via:
  - `credentialSubject.definition_payload_sha256 = SHA256(MCF(definition_vc_without_proof))`
    - `definition_payload_sha256` MAY be provided either as a raw sha256 digest string (legacy) OR as an **ArtifactRef** with `artifact_type: vc` and `digest_sha256` equal to the payload digest.
- SHOULD include `credentialSubject.definition_vc_id` (the Corridor Definition VC `id`) for human correlation
- MUST enumerate `credentialSubject.participants` with stable identifiers and roles
- MUST include `credentialSubject.activation.thresholds` describing required signatures by role

Activation is satisfied when **all** threshold rules are met. Example:

- `2-of-2` `zone_authority` signatures before activation

### Receipt signing policy (fork resistance) (v0.4.14+)

The corridor state-channel model is only fork-resistant if receipt production is constrained.
To make this explicit, Corridor Agreement VCs MAY specify a receipt signing policy:

- `credentialSubject.state_channel.receipt_signing.thresholds`

Semantics are identical to activation thresholds: a receipt is acceptable if all threshold rules are met.

If `state_channel.receipt_signing` is omitted, verifiers SHOULD default to `activation.thresholds`.

The reference tool can enforce this per-receipt threshold with:

- `msez corridor state verify --enforce-receipt-threshold ...`

### Lawpack pins (v0.4.1+)

Participant-specific Corridor Agreement VCs SHOULD include:

- `credentialSubject.pinned_lawpacks[]`

Each entry binds a jurisdiction + domain to an exact lawpack digest (`lawpack_digest_sha256`).

`lawpack_digest_sha256` MAY be provided either as a raw sha256 digest string (legacy) OR as an **ArtifactRef** with `artifact_type: lawpack` and `digest_sha256` equal to the lawpack digest.

When `lawpack_compatibility.required_domains` is present in the Corridor Definition VC, validators MUST require that each participant’s agreement VC includes pinned lawpacks covering those domains.

If `lawpack_compatibility.allowed[]` is present, validators MUST enforce that pinned digests appear in the allow-list for the corresponding jurisdiction and domain.

See `spec/96-lawpacks.md`.

### Participant-specific agreement VCs (recommended)

To support decentralized publication (each party can publish their own artifact) and clean revocation semantics,
implementations MAY represent a multi-party corridor agreement as **multiple agreement VCs**:

- Each VC SHOULD include `credentialSubject.party` describing the specific party making the commitment.
- Each such VC MUST be signed by that party: at least one valid proof `verificationMethod` MUST resolve to
  `credentialSubject.party.id` (normalized to the DID without fragment).
- `credentialSubject.party_terms` MAY contain party-specific addenda (fee schedules, operational SLAs, carveouts).
  When multiple agreement VCs are used, validators MUST compare only the **base subject fields** for consistency:
  `corridor_id`, `definition_payload_sha256` (and optionally `definition_vc_id`), `participants`, `activation` and
  any shared `terms`. Party-specific fields MUST be ignored for the base consistency check.

### Status lock (normative)

When multiple agreement VCs are used, `agreement_vc_path` MUST include **at most one current VC per party DID**
(identified by `credentialSubject.party.id`). If the same party id appears more than once across all agreement VCs,
validation MUST fail.

This “status lock” ensures deterministic interpretation of each participant’s current state.

### Commitment-aware activation (normative)

Participant-specific agreement VCs MAY include `credentialSubject.commitment` to express the party’s current status.

Affirmative commitments are defined by `credentialSubject.activation.accept_commitments`:

- If `accept_commitments` is omitted, it defaults to `['agree']`.
- Only commitments in `accept_commitments` count toward activation thresholds.
- Commitments not in `accept_commitments` are **non-affirmative** and MUST block activation regardless of threshold satisfaction.

For threshold evaluation, validators MUST count unique parties that have:

1. provided a valid signature, and
2. an affirmative commitment.

When `credentialSubject.party` is present, the `party.id` is counted; when `party` is absent, distinct signer DIDs are counted.

### Activation blockers (normative)

When activation fails due to non-affirmative commitments, validators MUST return (or log) a list of human-readable
blockers in the form:

- `<partyDid>:<commitment>`

The reference tool exposes these as `corridor_activation_blockers` in `msez corridor status` and `msez lock`.

### Signer authorization (normative)

Each signer of a Corridor Agreement VC MUST be listed in `trust-anchors.yaml` with `allowed_attestations`
containing `corridor_agreement`.


## Corridor state channels (verifiable receipts) (v0.4.2+)

Corridor manifests and VCs define *what the corridor is* and *when it is activated*.
To make corridor **operations** verifiable (and eventually ZK- or L1-anchorable), corridors SHOULD be
implemented as **verifiable state channels**.

A *corridor state channel* is an append-only sequence of signed receipts that deterministically update a
`corridor_state_root`.

### Corridor state root (normative)

The corridor state root is a 32-byte commitment encoded as 64 lowercase hex characters.

Implementations MUST define a deterministic **genesis root** that binds the operational state channel to the
corridor’s cryptographic substrate:

- Corridor Definition VC payload hash (excluding proofs)
- Corridor Agreement set digest (excluding proofs)
- the active `lawpack_digest_set`
- the active `ruleset_digest_set`

The reference genesis root definition is:

```
# msez.corridor.state.genesis.v1
genesis_root = SHA256(MCF({
  "tag": "msez.corridor.state.genesis.v1",
  "corridor_id": "...",
  "definition_payload_sha256": "...",
  "agreement_set_sha256": "...",
  "lawpack_digest_set": ["..."],
  "ruleset_digest_set": ["..." ]
}))
```

### Corridor State Receipt (normative)

Every corridor transition MUST be represented by a **Corridor State Receipt**:

- Receipts MUST conform to `schemas/corridor.receipt.schema.json`.
- Receipts MUST include: `(prev_root, next_root, lawpack_digest_set, ruleset_digest_set)`.
- Receipts MUST be signed (at least one valid `proof`).

`lawpack_digest_set` and `ruleset_digest_set` are **digest sets**. Each entry MAY be either:
- a raw 64-hex sha256 string (legacy), or
- an `ArtifactRef` (preferred; `artifact_type` MUST match the expected type).

Verifiers MUST treat these lists as sets of digests by coercing each entry to its underlying sha256 commitment.

Receipts SHOULD include a corridor-specific `transition` object.
Receipts MAY include `zk` proof scaffolding and/or `anchor` metadata.

### Typed transition envelopes (recommended; v0.4.3+)

To make corridor transitions machine-parseable while keeping the receipt model generic, v0.4.3
introduces a **typed transition envelope**.

Receipts SHOULD carry `transition` as a `MSEZTransitionEnvelope`.

v0.4.4 adds **optional digest references** so that ecosystems can converge on interoperable
transition kinds without making the corridor receipt schema corridor-specific:

- `schema_digest_sha256` — payload format
- `ruleset_digest_sha256` — transition validation semantics
- `zk_circuit_digest_sha256` — optional ZK circuit identifier for proof-carrying transitions

Example:

```json
{
  "type": "MSEZTransitionEnvelope",
  "kind": "...",
  "schema": "...",                // optional schema identifier/URI
  "schema_digest_sha256": "...",  // optional payload schema digest
  "ruleset_digest_sha256": "...", // optional transition ruleset digest
  "zk_circuit_digest_sha256": "...", // optional ZK circuit digest
  "payload": { "...": "..." },    // optional inline payload
  "payload_sha256": "...",        // required commitment
  "attachments": [
    {"artifact_type": "blob", "uri": "...", "digest_sha256": "..."},
    {"artifact_type": "schema", "digest_sha256": "..."}   // example: attach a schema/VC/checkpoint by digest
  ]
}
```

Rules:

- If `payload` is present, `payload_sha256` MUST equal `SHA256(MCF(payload))`.
- If `payload` is omitted, `payload_sha256` MUST commit to the out-of-band payload bytes per corridor policy.
- `kind` names the corridor-specific transition type (e.g., `mint`, `burn`, `settle`, `attest`).

If present, digest references MUST be either:
- a raw 64-hex sha256 string (legacy), or
- an `ArtifactRef` (preferred; `artifact_type` MUST match the expected type).

Verifiers MUST coerce these fields to their underlying sha256 commitment (`digest_sha256`).

Attachments are typed artifact references (v0.4.10+; standardized as `ArtifactRef` in v0.4.11):

- Each `transition.attachments[*]` object MUST include `digest_sha256`.
- New-style attachments SHOULD include `artifact_type` so the digest resolves via:
  - `dist/artifacts/<artifact_type>/<digest_sha256>.*`
- For backward compatibility, if an attachment omits `artifact_type`, verifiers MUST treat it as:
  - `artifact_type = "blob"`
- `uri` is an optional hint (non-normative); verifiers SHOULD NOT rely on it.

For `artifact_type = "blob"`:

- `digest_sha256` MUST be computed as `SHA256(bytes)` over the raw bytes of the referenced object.
- Implementations SHOULD publish the referenced bytes as an artifact resolvable by `(type='blob', digest)` under:
  - `dist/artifacts/blob/<digest_sha256>.*`


### Transition Type Registry (recommended; v0.4.4+)

To keep corridors generic while enabling ecosystems to standardize interoperable transition kinds,
a corridor MAY publish a **Transition Type Registry** that maps `transition.kind` to optional digest references.

Corridors MAY configure the registry path in `corridor.yaml`:

```yaml
state_channel:
  transition_type_registry_path: transition-types.yaml
```

Registry format (YAML):

```yaml
version: 1
transition_types:
  - kind: msez.example.transfer.v1
    schema_digest_sha256: "<sha256>"
    ruleset_digest_sha256: "<sha256>"
    zk_circuit_digest_sha256: "<sha256>"   # optional
```

Normative guidance:

- If a corridor uses a transition type registry, its **Corridor Definition VC SHOULD pin it** as an artifact
  (`credentialSubject.artifacts.transition_type_registry`) so the registry is cryptographically bound into the
  corridor's genesis substrate.
- Receipts MAY omit digest references and rely on the pinned registry for semantics.
- If a receipt includes digest references, they MUST match the pinned registry entry for that `kind` unless the
  receipt explicitly indicates an override (see v0.4.5+ below).
- If `transition.ruleset_digest_sha256` is present, it SHOULD also appear in `receipt.ruleset_digest_set`.
- If both `transition.zk_circuit_digest_sha256` and `receipt.zk.circuit_digest_sha256` are present, they MUST match.


### Transition Type Registry Lock (recommended; v0.4.5+)

YAML registries are authoring-friendly, but they are not ideal as cryptographic commitments because formatting
(whitespace, ordering) can change without changing semantics.

v0.4.5 introduces a **Transition Type Registry Lock**: a deterministic, content-addressed snapshot that can be
referenced by receipts.

Corridors MAY configure an optional lockfile path in `corridor.yaml`:

```yaml
state_channel:
  transition_type_registry_path: transition-types.yaml
  transition_type_registry_lock_path: transition-types.lock.json
```

Lock format (JSON): `schemas/transition-types.lock.schema.json`

Digest semantics (normative):

- The lockfile MUST include `snapshot` with `tag = msez.transition-types.registry.snapshot.v1`.
- The lockfile MUST include `snapshot_digest_sha256 = SHA256(MCF(snapshot))`.

Within `snapshot.transition_types[*]`, per-kind digest fields (`schema_digest_sha256`, `ruleset_digest_sha256`, `zk_circuit_digest_sha256`) MAY be expressed as either raw sha256 digest strings (legacy) OR as ArtifactRef objects with the corresponding `artifact_type` (`schema`, `ruleset`, `circuit`).

Receipt binding (recommended):

- Receipts MAY include `transition_type_registry_digest_sha256` equal to the lock's `snapshot_digest_sha256`.
- When this field is present, receipts MAY omit per-transition digest references and rely on the snapshot mapping.

Content-addressed distribution (recommended; v0.4.7+):

To keep *historical* receipts verifiable even when a corridor module later updates its registry, ecosystems SHOULD
publish registry lock snapshots in a content-addressed store keyed by `snapshot_digest_sha256`.

Reference filename convention (this repository's artifact layout; see `spec/97-artifacts.md`):

- `dist/artifacts/transition-types/<digest>.transition-types.lock.json`

Verifier requirements:

- A verifier resolving `receipt.transition_type_registry_digest_sha256` MUST fetch/locate the corresponding lock
  snapshot by digest (local cache, artifact registry, IPFS gateway/pinset, etc.).
- A verifier MUST recompute `SHA256(MCF(snapshot))` and ensure it equals the requested digest before trusting
  any `kind -> digest` mapping.

Commitment completeness (optional; v0.4.8+):

Verifiers MAY choose to enforce that **every digest commitment** in a receipt is resolvable via the artifact CAS
(`spec/97-artifacts.md`). In this mode, verification fails if any referenced digest
(lawpacks, rulesets, transition registry, schema/circuit/proof keys) cannot be located by `(type,digest)`.

The reference CLI exposes this as `python -m tools.msez corridor state verify ... --require-artifacts`.

Overrides (optional):

- A receipt MAY still include per-transition digest references (`schema_digest_sha256`, `ruleset_digest_sha256`,
  `zk_circuit_digest_sha256`) as an explicit override.
- When overriding, `transition.registry_override = true` SHOULD be set to make the intent unambiguous.

Corridor Definition VC pinning (recommended):

- If a corridor uses a registry lock, its **Corridor Definition VC SHOULD pin** the lockfile as an artifact
  (`credentialSubject.artifacts.transition_type_registry_lock`) so the snapshot is cryptographically bound into the
  corridor's genesis substrate.
- Corridors MAY additionally pin the authoring YAML registry for operator convenience.



### next_root computation (normative)

`next_root` MUST be computed deterministically as:

```
next_root = SHA256(MCF(receipt_without_proof_and_next_root))
```

Where `receipt_without_proof_and_next_root` is the receipt object with `proof` and `next_root` removed.

Implementations MUST treat both digest sets as **sets**:
- remove duplicates,
- sort lexicographically,
- then compute the root.

### Digest-set binding (normative)

Receipts are only meaningful when they bind to the **exact law + rules** in force.

- `lawpack_digest_set` MUST equal the union of the `pinned_lawpacks[].lawpack_digest_sha256` values in the
  activated Corridor Agreement VC set.
- `ruleset_digest_set` MUST include the digest of the active corridor state-transition ruleset descriptor (e.g., `msez.corridor.state-transition.v2`).
  Corridors MAY include additional ruleset digests for settlement logic, attestations, and dispute logic.


### Receipt inclusion proofs (MMR accumulator) (recommended; v0.4.3+)

A pure hash-chain state root is excellent for sequencing, but it does **not** support compact inclusion
proofs for arbitrary past receipts (you would need to reveal all later receipts to link forward).

To support **privacy-preserving inclusion proofs** ("prove receipt _i_ is in the channel history"),
corridors SHOULD maintain an append-only **Merkle Mountain Range (MMR)** over receipt digests
(`next_root`).

**Leaf hash** (domain-separated):

```
leaf_hash = SHA256(0x00 || next_root_bytes)
```

**Node hash** (domain-separated):

```
node_hash = SHA256(0x01 || left_child || right_child)
```

**Root:** the MMR root is computed by "bagging the peaks" right-to-left using `node_hash`.

**Checkpoints:** corridors SHOULD publish signed checkpoints committing to:

- `final_state_root` (hash-chain head)
- `receipt_count`
- `mmr.root` and `mmr.size` (= `receipt_count`)

Checkpoints MUST conform to `schemas/corridor.checkpoint.schema.json`.

**Inclusion proofs:** a party can provide an inclusion proof for a single receipt digest without
revealing other receipts. Proofs MUST conform to `schemas/corridor.inclusion-proof.schema.json`.

Inclusion proofs MAY include `checkpoint_ref` as an `ArtifactRef` (type `checkpoint`) so the committed checkpoint digest has an explicit CAS resolution path.

Verifiers SHOULD:

- verify checkpoint signature(s) (and, optionally, trust-anchor authorization),
- verify the receipt digest (`next_root`) and signature(s),
- verify the inclusion proof against the checkpoint MMR root.


### Checkpoint finality policy (v0.4.15+)

Corridor Agreement VCs MAY specify a checkpointing policy under:

- `credentialSubject.state_channel.checkpointing`

Suggested fields (see schema):

- `mode`: `optional` | `required`
- `max_receipts_between_checkpoints`: soft operational bound for how many receipts may elapse between successive checkpoints
- `thresholds`: signature quorum requirements for a checkpoint, expressed using the same role/threshold structure as `state_channel.receipt_signing.thresholds`

When `mode` is `required`, verifiers SHOULD treat the absence of a head checkpoint as a policy violation.

The reference tool supports scalable sync by allowing verifiers to bootstrap from a trusted checkpoint and verify only the tail receipts since that checkpoint.


### Watcher attestations and fork alarms (v0.4.15+)

Independent watchers can provide cheap, high-leverage integrity signals:

- **Watcher attestations**: a watcher issues a VC (`schemas/vc.corridor-watcher-attestation.schema.json`) committing to an observed head and referencing a checkpoint digest.
- **Fork alarms**: a watcher issues a VC (`schemas/vc.corridor-fork-alarm.schema.json`) presenting evidence of two conflicting receipts for the same `(corridor_id, sequence, prev_root)`.

Watcher artifacts are designed to be publishable out-of-band (public transparency logs, partner portals, etc.) and can be used to rapidly detect and respond to forks.

Reference tooling includes an aggregation primitive:

- `msez corridor state watcher-compare <module> --vcs <dir-or-file>`

The aggregator compares `(receipt_count, final_state_root)` across watcher attestations for the same `corridor_id`:

- If two or more attestations report the same `receipt_count` but different `final_state_root`, verifiers SHOULD treat this as a **fork alarm** (strong signal).
- If attestations disagree only on `receipt_count`, verifiers SHOULD treat this as **lag/out-of-sync** rather than a fork; implementations MAY fail verification in strict mode.


#### Watcher quorum policy (v0.4.17+)

Corridor Agreement VCs MAY specify a watcher quorum policy under:

- `credentialSubject.state_channel.watcher_quorum`

This policy is intended for **liveness monitoring** (and optional soft-finality):

- A corridor operator (or third-party monitoring system) ingests signed watcher attestation VCs.
- A quorum is reached when **K-of-N** authorized watchers agree on the same corridor head.
- If a fork-like divergence is detected (same `receipt_count`, different `final_state_root`), any apparent quorum is overridden and treated as a **critical alarm**.

The reference tool supports this via:

```bash
msez corridor state watcher-compare <corridor-module> \
  --vcs ./watcher-attestations/ \
  --quorum-threshold '3/5' \
  --require-quorum \
  --max-staleness '1h'
```

`--quorum-threshold` accepts values like:

- `majority` (default)
- `K/N` (e.g., `3/5`)

Implementations MAY derive `N` from the authority registry allow-list for `corridor_watcher_attestation` (when configured); otherwise they MAY derive `N` from the set of observed watchers.


#### Compact head commitments (gossip-friendly)

Watcher attestation VCs include a deterministic head commitment digest:

- `credentialSubject.head_commitment_digest_sha256`

This digest is computed over a stable subset of head fields (excluding timestamps), so identical heads dedupe perfectly even when checkpoint objects differ in timestamp/proof metadata.

Implementations SHOULD group attestations by `head_commitment_digest_sha256` when computing quorum and divergence signals.


### ZK proofs (optional)

Receipts MAY include a `zk` object. When present, implementations SHOULD:

- commit to `circuit_digest_sha256` and `verifier_key_digest_sha256`, and
- commit to `proof_sha256` (proof bytes may be carried out-of-band).

`proof_sha256` is typed as a `blob` artifact digest (`SHA256(bytes)`), and SHOULD be resolvable via:

- `dist/artifacts/blob/<proof_sha256>.*`


Validators MAY verify ZK proofs. Even when not verifying, validators MUST treat ZK digests as part of the
receipt payload (and therefore part of `next_root`).

### Anchoring (optional)

Receipts MAY include `anchor` metadata representing an external commitment (e.g., a tx hash on an L2).
Anchoring is intentionally optional and MUST NOT change the receipt/root model.


### Cross-corridor settlement anchoring (v0.4.38)

Corridors are intended to be *composable primitives*: a trade corridor can be coupled to a settlement corridor, an arbitration corridor, a collateral corridor, etc.

This stack therefore supports **cross-corridor anchoring** so that a receipt in one corridor can commit to *verifiable state* from another corridor.

The reference pattern is **cross-corridor settlement anchoring**:

- an **obligation corridor** (e.g., trade) produces an obligation receipt and checkpoint,
- a **settlement corridor** (e.g., stablecoin or bank rail) produces a settlement receipt and checkpoint,
- and the two are bound together by a content-addressed settlement anchor and optional proof bindings.

This works without a blockchain. Any external evidence (SWIFT messages, bank confirmations, chain receipts, audit reports, ZK proofs, etc.) is treated as an **artifact** committed by hash.


#### Typed attachments for inter-corridor state

Receipts MAY include typed transition attachments referencing other corridor checkpoints:

- `schemas/corridor.checkpoint.attachment.schema.json` (`artifact_type=checkpoint`)

Reference CLI:

```bash
msez corridor state receipt-init <corridor-module> \
  --transition <transition.json> \
  --attach-corridor-checkpoint <other-corridor-checkpoint.json>
```

This creates a *hash-level* coupling: the receipt commits to the other corridor's checkpoint payload digest (excluding proof), making cross-corridor references stable and replay-resistant.


#### Proof bindings

Because proof artifacts can be portable (and therefore replayable if not context-bound), the stack defines a lightweight **proof-binding** object:

- `schemas/proof-binding.schema.json`

A proof-binding commits to:

- a `proof_ref` (an ArtifactRef to the external proof), and
- one or more `commitments` (e.g., corridor checkpoint digests, receipt roots, settlement-anchor digests).

Reference CLI:

```bash
msez proof-binding init \
  --binding-purpose settlement.confirmation \
  --proof ./evidence/mt103.pdf \
  --proof-artifact-type blob \
  --commitment corridor.checkpoint:<trade_ck_digest>,corridor_id=trade \
  --commitment corridor.checkpoint:<settle_ck_digest>,corridor_id=settlement \
  --store
```


#### Settlement anchors

To bind an obligation corridor state to a settlement corridor state, the stack defines a structured settlement-anchor object:

- `schemas/corridor.settlement-anchor.schema.json`

A settlement-anchor records:

- obligation corridor checkpoint (and optional receipt root/sequence),
- settlement corridor checkpoint (and optional receipt root/sequence),
- optional `proof_bindings` and raw `proofs` as ArtifactRefs.

Reference CLI:

```bash
msez corridor settlement-anchor-init \
  --obligation-checkpoint ./trade/checkpoint.signed.json \
  --settlement-checkpoint ./settlement/checkpoint.signed.json \
  --proof-binding ./proof-binding.<digest>.json \
  --store
```

The resulting settlement-anchor artifact digest can then be attached to receipts on either corridor:

```bash
msez corridor state receipt-init <corridor-module> \
  --transition <transition.json> \
  --attach-settlement-anchor ./settlement-anchor.<digest>.json
```

Verifiers MAY treat this as a cross-corridor *atomicity* signal:

- if an obligation receipt claims settlement by attaching a settlement-anchor, then the referenced settlement corridor checkpoint SHOULD be validated and SHOULD include the corresponding settlement receipt.


#### ZK binding guidance (optional)

When using ZK proofs, implementations SHOULD include one of the following as a public input (or committed field) in the circuit:

- `settlement_anchor_digest_sha256` (digest of the settlement-anchor), and/or
- `proof_binding_digest_sha256` (digest of a proof-binding),

so that the ZK proof cannot be replayed against different corridor states.
## Verification ruleset

The reference ruleset identifier is:

- `msez.corridor.verification.v1`

A conforming validator MUST, at minimum:

1. Validate `corridor.yaml` against `schemas/corridor.schema.json`
2. Validate security artifacts against `schemas/trust-anchors.schema.json` and `schemas/key-rotation.schema.json`
3. Validate the Corridor Definition VC against `schemas/vc.corridor-definition.schema.json`
4. Verify the Corridor Definition VC signature(s) (cryptographic verification)
5. Verify the Corridor Definition VC hash bindings match the on-disk artifacts
6. Verify the Corridor Definition VC signer authorization against `trust-anchors.yaml`
7. If `agreement_vc_path` is present:
   1. Validate Corridor Agreement VC(s) against `schemas/vc.corridor-agreement.schema.json`
   2. Verify Corridor Agreement VC signature(s) (cryptographic verification)
   3. Verify Corridor Agreement VC binding to the Corridor Definition VC payload hash
   4. Verify Corridor Agreement VC signer authorization against `trust-anchors.yaml`
   5. Enforce status lock (at most one VC per `party.id`)
   6. Evaluate commitments against `accept_commitments`
   7. Enforce activation thresholds

## Reference CLI

The reference implementation includes:

- `msez vc keygen` — generate Ed25519 keys (writes JWK, prints did:key)
- `msez vc sign` — sign a Verifiable Credential
- `msez vc verify` — verify VC signature(s)
- `msez vc payload-hash` — compute SHA256 of signing input (payload excluding `proof`)
- `msez corridor vc-init-definition` — scaffold an unsigned Corridor Definition VC from a corridor package
- `msez corridor vc-init-agreement` — scaffold an unsigned Corridor Agreement VC from a corridor package
- `msez corridor verify` — verify corridor definition, agreement, and activation
- `msez corridor status` — summarize activation status and blockers across an agreement-set
- `msez corridor availability-attest` — create a lawpack artifact availability attestation VC
- `msez corridor availability-verify` — verify availability attestations cover the corridor lawpacks
- `msez corridor state genesis-root` — compute corridor state-channel genesis_root
- `msez corridor state receipt-init` — create a corridor state receipt (computes next_root; optionally signs)
- `msez corridor state verify` — verify a receipt chain and print the final root

See also: `spec/95-lockfile.md` for how corridor substrate (definition/agreement digests) and lawpacks are pinned into `stack.lock`.

