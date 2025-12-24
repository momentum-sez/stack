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

### Hash computation (normative)

Implementations MUST compute file digests deterministically.

- For JSON artifacts: digest MUST be `SHA256(JCS(json))` where JCS is RFC 8785 JSON Canonicalization Scheme.
- For YAML artifacts: implementations MUST parse YAML into an equivalent JSON data model and digest MUST be
  `SHA256(JCS(json_model))`.

For Corridor Agreement binding, the **definition payload hash** MUST be computed over the Corridor Definition VC
**payload excluding** `proof`:

- `definition_payload_sha256 = SHA256(JCS(definition_vc_without_proof))`

The reference tool exposes this as:

- `msez vc payload-hash <vc.json>`

## Corridor Agreement VC (normative)

A Corridor Agreement VC captures **participant-specific acceptance** of a specific corridor definition and defines
the **activation rule** for bringing the corridor online.

A Corridor Agreement VC:

- MUST conform to `schemas/vc.corridor-agreement.schema.json`
- MUST bind to a specific Corridor Definition VC payload via:
  - `credentialSubject.definition_payload_sha256 = SHA256(JCS(definition_vc_without_proof))`
- SHOULD include `credentialSubject.definition_vc_id` (the Corridor Definition VC `id`) for human correlation
- MUST enumerate `credentialSubject.participants` with stable identifiers and roles
- MUST include `credentialSubject.activation.thresholds` describing required signatures by role

Activation is satisfied when **all** threshold rules are met. Example:

- `2-of-2` `zone_authority` signatures before activation

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

See also: `spec/95-lockfile.md` for how corridor state is pinned into `stack.lock`.

