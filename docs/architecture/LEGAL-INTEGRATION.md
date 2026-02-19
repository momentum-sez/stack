# Legal Integration

MEZ is a **legal infrastructure** project enabled by cryptography.

The system’s critical move is to bind corridor operations to **specific snapshots of law and policy** (“lawpacks”) via content-addressed digests. This gives you determinism:

- A corridor operation at time T is governed by a specific legal corpus digest.
- Historical disputes can reference the exact governing law snapshot.
- Law can evolve without invalidating historical commitments.


## Lawpacks are evidence, not enforcement

A `lawpack` digest commits to **text bytes** (after deterministic normalization), not to legal force.

That distinction matters:

- A party can sign an agreement committing to a lawpack digest and still later argue that the underlying text was never enacted or recognized as binding law.
- Two different legal sources can be legally equivalent but normalize to different digests.

MEZ addresses this by treating lawpacks as a **verifiable evidence layer**, and adding explicit legal attestation mechanisms on top.


## Recommended legal-hardening primitives

### 1) Legal attestation VCs

For high-assurance corridors, introduce a signed credential that attests that a given lawpack digest is a valid, enforceable snapshot of law in a jurisdiction.

This allows verifiers to reason about two separate questions:

- *Does the corridor commit to the digest?* (cryptographic fact)
- *Is the digest recognized as legally binding by the relevant authority?* (attestation)


### 2) Choice-of-law and dispute venue

Corridor agreements should include explicit choice-of-law and dispute-resolution clauses so legal enforceability does not depend on informal interpretation of cryptographic bindings.

Where possible, reference arbitration institutions or dispute processes that can ingest MEZ artifacts.


### 3) Availability and provenance

Legal artifacts are only useful if they remain available:

- ensure every corridor participant stores every pinned lawpack
- pin to content-addressed public storage where possible
- publish availability attestations as operational evidence


## Operational rule of thumb

Treat the cryptographic stack as the **ground truth for what was agreed and executed**, and the legal attestation layer as the bridge that convinces courts, regulators, and arbitrators that those cryptographic facts should be recognized.

The closer the system gets to moving real value, the more you should invest in:

- signed legal attestations
- institutional dispute resolution
- redundant artifact storage
- incident response procedures

See `spec/96-lawpacks.md` for the lawpack specification and `docs/attestations/catalog.md` for attestation types.
