# Nodes

This document defines how **nodes** are represented in the Momentum EZ Stack, with a focus on:

- cryptographic identity,
- capability discovery (including ZK and secure execution),
- compatibility constraints for corridors,
- future-proofing for optional L1 anchoring (without requiring tokenization).

## Mental model

- A **Zone** is a governance/operating unit that pins a supply chain (`stack.lock`).
- A **Node** is an operational agent acting for a zone (or for a corridor), with a stable cryptographic identity.
- A **Corridor** is an interoperability contract between zones, enforced by signed commitments and (optionally) verifiable computation.

The Stack aims to be *L1-optional*: all core commitments are content-addressed and signed, and may later be anchored to an L1 without redesign.

## Node identity

Nodes SHOULD have a DID (e.g., `did:key`, `did:web`) used for:
- signing protocol messages and receipts,
- signing verifiable credentials (VCs),
- key rotation and revocation.

Nodes SHOULD publish:
- a DID document,
- a discovery descriptor (`node.yaml`),
- optional capability attestations (VCs).

## Node descriptor

Nodes MAY publish a descriptor document (`node.yaml`) validated by `schemas/node.schema.json`.

The descriptor is not itself a credential; it is a discovery + routing document. Integrity is provided by:
- the zone’s `stack.lock` pinning (optional),
- a signed Node Attestation VC (recommended).

### ArtifactRef usage (normative)

To keep node commitments portable and mechanically verifiable, node descriptors SHOULD use **ArtifactRef** wherever a digest is used:

- `capabilities.zk.verifying_keys[*]` SHOULD be an `ArtifactRef` with `artifact_type: proof-key` and `digest_sha256` equal to the verifying key digest.
  - Legacy form `{key_id, sha256}` remains valid.
- `attestations[*]` SHOULD be an `ArtifactRef` with `artifact_type: vc` (node capability VC payload digest).
  - Legacy form `{vc_path, vc_sha256}` remains valid.

This makes node capability material resolvable via the repository CAS convention (`dist/artifacts/<type>/<digest>.*`).

## Capability attestation

Nodes SHOULD use a **Node Capability Attestation VC** (future-facing) to bind:
- node identity,
- declared capabilities,
- software/firmware versions,
- cryptographic commitments to verifier keys / circuits,
- optional hardware attestation evidence.

This allows corridors to specify “compatibility” at the capability layer, not only at the identity layer.

## ZK-ready design principles

Even before an L1 exists, node + corridor design should assume:

- **State commitments:** every meaningful corridor interaction should have a state root (Merkle root) that can be checkpointed.
- **Deterministic rulesets:** verifier logic must be content-addressed (digest pinned in `stack.lock`).
- **Proof-system explicitness:** corridors should declare accepted proof systems and verifier keys by digest, not by name.

Recommended pattern (future):
1. Corridor Definition VC pins:
   - `policy digest(s)` and `lawpack digest(s)`
   - `ruleset digest(s)` and (optional) verifier key digests
2. Each participant node publishes a capability VC pinning the verifier key digests it supports.
3. Corridor operations exchange:
   - state roots
   - receipts signed by node keys
   - optional ZK proofs that transitions satisfy pinned rules.

## Optional anchoring (L1-optional)

If/when an L1 is introduced, the Stack should only need to add:

- an **anchor target** (chain + contract + method)
- a rule that periodically anchors:
  - corridor state roots,
  - transparency log roots,
  - lawpack digest snapshots (optional)

No change is required to the identity or digest model, because everything is already content-addressed.


## Corridor state channels

Nodes participating in a corridor SHOULD support the corridor state channel model in `spec/40-corridors.md`:

- track the latest `corridor_state_root` per corridor
- emit and verify **Corridor State Receipts** (`schemas/corridor.receipt.schema.json`)
- include digest-set bindings (`lawpack_digest_set`, `ruleset_digest_set`) to avoid floating law or floating verifier logic
- optionally produce or verify ZK proofs for transitions (capability-advertised)
- optionally anchor roots to an external chain (L1-optional)

Node descriptors (`node.yaml`) and Node Capability Attestation VCs SHOULD include enough information to
negotiate these features (supported proof systems, verifier key digests, anchoring endpoints).

## Related specs

- `spec/40-corridors.md`
- `spec/96-lawpacks.md`
- `spec/95-lockfile.md`
