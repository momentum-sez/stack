# Civic governance infrastructure (v0.4+ scaffold)

This section makes the “governance” layer implementable without prematurely dictating a single political design.

The goal is to provide **standard building blocks** that can be composed by zones, and that can integrate with
Mass/Momentum primitives (especially *Consent*).

## Governance modules

A **governance module** is a module whose primary purpose is to define a decision/consent mechanism:

- binary votes (yes/no)
- ranked choice / STV
- approval / score voting
- quadratic voting
- quadratic funding
- liquid delegation / liquid democracy
- privacy-preserving participation (e.g., ZK eligibility proofs)

Governance modules SHOULD be `kind: governance` in `module.yaml`.

## Core interoperability objects

To remain interoperable across implementations, governance systems MUST converge on shared data formats.

The reference repository provides a `governance/core` module that defines canonical JSON Schemas for:

- **Proposal** (`proposal.schema.json`) — what is being decided, scope, timestamps, eligibility policy reference
- **Ballot / Vote** (`vote.schema.json`) — a participant’s expression of consent in the configured mechanism
- **Delegation** (`delegation.schema.json`) — delegation edges for liquid governance
- **Tally / Outcome** (`tally.schema.json`) — results and audit trail references
- **Eligibility proof** (`zk-eligibility-proof.schema.json`, optional) — a privacy-preserving proof that a voter is eligible

Implementations MAY extend these objects but MUST NOT break backwards compatibility for the required fields.

## Event + audit expectations

Governance outputs SHOULD be publishable into the zone’s audit log and/or transparency dashboard modules.

At minimum:

- proposal creation, vote casting, and tally publication SHOULD emit audit-log events
- tally outputs SHOULD be linked to:
  - the proposal identifier
  - an immutable artifact hash (content-addressable anchor)
  - the conformance version of the governance module(s) used

## Mapping to Mass primitives

Governance is the primary substrate for **Consent**.

Implementations SHOULD expose:

- a “consent decision” event stream that can be consumed by downstream systems (e.g., regulator console)
- explicit linkage between:
  - consent outcomes and the policy objects they authorize (licenses, rule changes, corridor agreements, etc.)

