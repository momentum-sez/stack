# High-leverage integrity upgrades — v0.4.15

This note extends the v0.4.14 foundational upgrade with the next highest-leverage integrity and operational-resilience primitives:

1) watcher attestations + fork alarms
2) lawpack availability attestations
3) checkpoint finality policy
4) authority registry chaining (treaty body → national → zone)

The goals are:

- **Cheap, external integrity signals** that can be published widely (watchers)
- **Operational resilience** against data loss / custody failures (availability)
- **Scalable verification** for new verifiers (checkpoints + policy)
- **Reduced circular trust** for corridor activation (authority registry chain)

## 1) Watcher attestations + fork alarms

### Watcher attestation VC

Schema:

- `schemas/vc.corridor-watcher-attestation.schema.json`

Intended use:

- A watcher observes a signed corridor checkpoint and publishes a VC that commits to the checkpoint digest and observed head.
- Other verifiers can compare watcher attestations for the same corridor to detect divergent heads quickly.

Reference CLI:

- `msez corridor state watcher-attest ... --checkpoint <signed-checkpoint.json> --issuer <watcher-did> [--sign --key ...]`
- `msez corridor state watcher-compare <module> --vcs <dir-or-file> [--fail-on-lag] [--enforce-authority-registry] [--require-artifacts]`

### Fork alarm VC

Schema:

- `schemas/vc.corridor-fork-alarm.schema.json`

Intended use:

- When a watcher detects conflicting receipts for the same `(corridor_id, sequence, prev_root)` it can publish a fork alarm VC.
- Receipts can optionally be stored into the artifact CAS for third-party audit.

Reference CLI:

- `msez corridor state fork-alarm ... --receipt-a <r0.json> --receipt-b <r0_fork.json> --issuer <watcher-did> [--sign --key ...]`

## 2) Lawpack availability attestations

Schema:

- `schemas/vc.artifact-availability.schema.json`

Intended use:

- Operators/participants attest they can serve the corridor’s pinned lawpacks (and optionally other artifacts).
- This is **operational**, not legal: it answers “can I fetch the needed content?”

Reference CLI:

- `msez corridor availability-attest <module> --issuer <did> --endpoint <uri> [--sign --key ...]`
- `msez corridor availability-verify <module> --vcs <dir-or-file>`

## 3) Checkpoint finality policy

Agreement schema extension:

- `schemas/vc.corridor-agreement.schema.json` now allows `credentialSubject.state_channel.checkpointing`.

This enables deployments to declare (and verifiers to enforce) a checkpoint cadence and/or signature quorum.

Reference verifier upgrades:

- `msez corridor state verify` supports:
  - `--from-checkpoint <signed-checkpoint.json>` to bootstrap and verify only the receipt tail
  - `--checkpoint <signed-head-checkpoint.json>` to enforce the head commitment
  - `--enforce-checkpoint-policy` to apply agreement checkpointing policy where possible

## 4) Authority registry chaining

Schema extension:

- `schemas/vc.authority-registry.schema.json` now supports `credentialSubject.parent_registry_ref`.
- `schemas/corridor.schema.json` allows `authority_registry_vc_path` to be either a string or a list of strings.

Reference tool behavior:

- When a list is provided, the tool validates hierarchical delegation across the chain:
  - treaty body registry authorizes a national registry issuer
  - national registry authorizes a zone registry issuer
  - the zone registry becomes the effective allow-list for corridor signing roles

See `spec/40-corridors.md` for the normative description.
