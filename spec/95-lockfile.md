# Lockfile semantics (normative)

Deployments MUST record a resolved, deterministic lockfile called `stack.lock`.

## Purpose

The lockfile ensures:

- deterministic builds,
- reproducible deployments,
- upgrade diffs,
- auditability of which legal/regulatory artifacts were active at a point in time.

## Canonical structure

`stack.lock` MUST validate against `schemas/stack.lock.schema.json` and MUST include, at minimum:

- `stack_spec_version`
- `generated_at` (RFC 3339 timestamp string)
- `zone_id`
- `profile` (`profile_id` + `version`)
- `modules[]` (resolved module list, including at least `module_id`, `version`, `variant`, and a `module_manifest_sha256`)
- `lawpacks[]` (pinned lawpack digests for jurisdictional corpora; see `spec/96-lawpacks.md`)
- `overlays[]` (patch digests, when overlays are used)
- `corridors[]` (corridor integrity state; see below)

## Lawpack lock entries

Each `lawpacks[]` entry MUST include, at minimum:

- `jurisdiction_id`
- `domain`
- `lawpack_digest_sha256`

`lawpack_digest_sha256` MAY be either:
- a raw sha256 digest string (legacy), or
- an **ArtifactRef** with `artifact_type: lawpack` and `digest_sha256` equal to the lawpack digest.

Tooling MAY emit ArtifactRefs by default. The reference tool supports:

```bash
mez lock --emit-artifactrefs <zone.yaml>
```

(v0.4.14+)

Implementations MUST treat the commitment as the digest value (`digest_sha256`) and use `artifact_type` only to determine the CAS resolution path.

If the pin was produced from a `lawpack.lock.json`, the entry SHOULD also include:

- `lawpack_lock_path`
- `lawpack_lock_sha256`
- `lawpack_artifact_path`
- `as_of_date`

Lawpack digest semantics are defined in `spec/96-lawpacks.md`.

## Corridor lock entries

Each `corridors[]` entry MUST include:

- `corridor_id`
- `corridor_manifest_sha256` (hash of `corridor.yaml`)
- `trust_anchors_sha256` (hash of `trust-anchors.yaml`)
- `key_rotation_sha256` (hash of `key-rotation.yaml`)
- `corridor_definition_vc_sha256` (hash of the Corridor Definition VC file **including** `proof`)
- `corridor_definition_signers[]` (DIDs observed in Corridor Definition VC proof(s))

Digest-bearing corridor lock fields (`*_sha256`) MAY also be expressed as **ArtifactRef** objects (v0.4.13+). Legacy raw digest strings remain valid.

When `corridor.yaml` configures `agreement_vc_path`, the lock entry MUST additionally include:

- `corridor_agreement_vc_sha256[]` — hash of each Corridor Agreement VC file **including** `proof`
- `corridor_agreement_signers[]` — signer DIDs observed in agreement VC proof(s) (or derived from `signed_parties`)
- `corridor_activated` — best-effort activation status at lock generation time
- `corridor_activation_blockers[]` — human-readable blockers like `<partyDid>:<commitment>` that prevented activation

### Agreement payload hash locking (normative)

To make corridor activation cryptographically meaningful (not merely declarative), the lockfile MUST also record
proof-excluded payload hashes and an agreement-set digest:

- `corridor_agreement_payload_sha256_by_path` — a mapping of **relative path → payload hash**, where each payload hash is
  `SHA256(JCS(payload))` for the corresponding agreement VC (payload excludes `proof`).
- `corridor_agreement_set_sha256` — a digest that content-addresses the agreement-set as a whole.

The agreement-set digest MUST be computed as:

1. Compute `definition_payload_sha256` as `SHA256(JCS(definition_vc_without_proof))`.
2. For each agreement VC file, compute `agreement_payload_sha256` as `SHA256(JCS(agreement_vc_without_proof))`.
3. Construct the JSON object:

```json
{
  "corridor_id": "<corridor_id>",
  "definition_payload_sha256": "<sha256>",
  "agreement_payload_sha256": ["<sha256>", "..."]
}
```

4. Canonicalize that object using RFC 8785 JSON Canonicalization Scheme (JCS) and compute `SHA256` over the UTF-8 bytes.

Implementations MUST sort the `agreement_payload_sha256` array lexicographically before digesting.

This allows:
- deterministic lock generation,
- stable audit references to “the activated agreement-set,” and
- tamper-evident detection of agreement edits even if `proof` is re-issued.

