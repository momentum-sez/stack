# Foundational Upgrade v0.4.14 — From “Composable Modules” to a Verifiable Jurisdiction Substrate

This document is a synthesis of the current Momentum SEZ Stack (MSEZ) direction and a concrete repository hardening plan driven by a deep critical review.

## What we are actually building

MSEZ is not “a blockchain project.” It is a **machine-verifiable, machine-traversable substrate** for:

- **Multi-jurisdictional legal state** (what law was in force when, and how it applies)
- **Multi-party corridor agreements** (who agreed, what was agreed, under what dispute forum)
- **Append-only operational receipts** (what transitions occurred, and how to prove them)
- **Attestable authority + legitimacy** (who was permitted to sign and why)
- **Selective disclosure + proof-carrying verification** (MMR inclusion proofs, ZK-ready transition types)

Blockchains can complement this substrate (timestamping, anchoring, settlement rails), but the substrate itself lives at the intersection of **law, operations, and cryptography**, where the object of interest is not “token balance” but **jurisdictional commitments**.

## The critical gaps (condensed)

The review identifies four systemic fault lines that are more dangerous than typical software bugs because they sit at legal–technical boundaries:

1. **Digest ≠ legal force (legal–cryptographic gap)**
2. **Trust anchor bootstrapping circularity**
3. **Receipt chain forks without explicit finality rules**
4. **Artifact availability as a hidden liveness dependency**

## v0.4.14: the first “fortification” step

This release is intentionally “foundation-first”: it adds the primitives needed to harden the system without forcing a premature governance model or a heavyweight consensus design.

### 1) Make ArtifactRefs the default substrate (optional)

Lockfiles are the canonical “what exactly is deployed” index. If lock generation emits raw digests, ArtifactRefs are merely tolerated by schema.

**v0.4.14 adds**:

- `msez lock --emit-artifactrefs`
  - lawpack pins are emitted as `artifact_type: lawpack`
  - corridor artifacts are emitted as `artifact_type: blob`

This shifts the project toward a world where *every important dependency is a resolvable, typed, content-addressed object*, suitable for automation and policy engines.

### 2) Introduce an external authority layer (mitigating trust-anchor circularity)

Trust anchors inside a corridor module are necessary, but they are not a sufficient “root of legitimacy.”

**v0.4.14 adds**:

- A new VC schema: `MSEZAuthorityRegistryCredential` (`schemas/vc.authority-registry.schema.json`)
- An optional corridor module field: `authority_registry_vc_path`
- Reference-tool enforcement that constrains corridor trust anchors + signers against the registry when present

This keeps the repo modular while enabling deployment environments to anchor authority to:

- a treaty body
- a government registry
- a standards consortium
- an arbitration institution

### 3) Add an explicit receipt signing policy surface (fork resistance)

A “state channel” without a fork rule is a forkable log.

**v0.4.14 adds**:

- A new optional policy field in Corridor Agreement VC:
  - `credentialSubject.state_channel.receipt_signing.thresholds`
- A verifier flag:
  - `msez corridor state verify --enforce-receipt-threshold`

The practical effect: corridors can require **N-of-M** signatures per receipt (often N=M for high-assurance corridors), creating an operational finality rule even before introducing BFT consensus.

### 4) Add the legal-force attestation primitive

We need a first-class way to assert: “this digest corresponds to enforceable law.”

**v0.4.14 adds**:

- A new VC schema: `MSEZLawpackAttestationCredential` (`schemas/vc.lawpack-attestation.schema.json`)
- A helper command: `msez law attest-init`

This unlocks the next phase (v0.4.15+): optional enforcement rules where corridor activation requires one or more recognized legal attestations per pinned lawpack.

## What comes next (repo-level work items)

This release is deliberately the start of a larger fortification arc. The repository should evolve toward:

- **Authority graph**
  - authority registries + delegation chains
  - rotation ceremonies with multi-party attestations
- **Receipt finality**
  - multi-signer thresholds by default
  - optional BFT “high assurance corridor” profiles
  - checkpointing + watcher attestations
- **Availability guarantees**
  - artifact availability attestations
  - redundant pinning policy hooks
- **Legal semantics**
  - lawpack succession credentials
  - explicit choice-of-law + forum clauses in templates

## Why this matters

The core thesis remains:

> We are defining a substrate in which a decentralized network more powerful than a blockchain can be constructed — because it is built from jurisdictional commitments, verifiable authority, and dispute-resolution primitives that intelligent systems can traverse.

v0.4.14 is the first step toward making that substrate **operationally and legally defensible**, not just technically elegant.
