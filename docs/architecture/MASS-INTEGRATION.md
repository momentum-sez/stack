# Mass Protocol Integration

This document describes how to map MSEZ primitives onto the Mass Protocol framing (Entity → Consent → Attestation).

The key idea is that MSEZ corridors already implement the core of a decentralized consent and attestation fabric:

- Zones are entities with identities and governance.
- Corridors are consent structures connecting entities.
- Receipts, checkpoints, and watcher signals are attestations about state.


## Conceptual mapping

### Zone = Mass Entity

An MSEZ **zone** (node) is an entity with:

- a jurisdictional identity (authority registry chain)
- pinned legal and policy commitments (lawpacks, rulesets)
- governance and operational processes


### Corridor = Mass Consent

A corridor’s **definition VC** specifies the contract surface.

Participant-specific **agreement VCs** represent consent, including threshold activation and pinned commitments.


### Receipt/Checkpoint/Watcher = Mass Attestation

In operation, a corridor emits:

- **receipts**: signed state transitions
- **checkpoints**: signed compressed commitments for sync and finality
- **watcher attestations**: cheap external signals of the observed head

This layered attestation structure supports both privacy (selective disclosure) and scalability (checkpoint sync).


## Practical integration hooks

MSEZ already provides a digest-based substrate; integrating with Mass typically requires:

- normalizing identifiers (corridor_id / zone ids)
- emitting Mass-friendly envelopes around VCs/receipts
- agreeing on registry and schema digests for interoperability

The repo’s `spec/12-mass-primitives-mapping.md` is the normative anchor; this document is operational guidance.
