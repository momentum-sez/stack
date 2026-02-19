# Security Model

This document summarizes the security model of the MEZ stack and the minimal threat-model assumptions you should make when operating zones and corridors.

MEZ is designed so that **verification does not require trust in transport** (you can verify receipts/VCS/checkpoints from untrusted channels), but it does still require explicit configuration of:

- authorized signers (an authority registry chain)
- pinned artifacts (lawpacks, rulesets, registries) and their digests
- operational policies (checkpointing, watcher quorum, key rotation)


## Trust boundaries

### What cryptography can guarantee

Given valid signatures and resolved content-addressed artifacts, a verifier can establish:

- **Integrity**: the bytes referenced by a digest are exactly those used in verification
- **Authorship**: the signer’s key produced the signature
- **Causality**: corridor receipts form an append-only chain by committing to `prev_root` → `next_root`


### What cryptography cannot guarantee by itself

Cryptography cannot (by itself) guarantee:

- **legal force** of a lawpack digest
- **social legitimacy** of an authority registry
- **availability** of artifacts in a decentralized network

Those require governance, redundancy, and operational controls.


## Key attack classes and mitigations

### 1) Forks in corridor receipt history

**Attack:** Two valid receipts are produced with the same `prev_root` but different `next_root`.

**Mitigations in stack:**

- explicit receipt fields (`sequence`, `prev_root`, `next_root`) plus checkpointing
- watcher attestations + watcher-compare to detect divergence without transporting receipts
- optional checkpoint policy enforcement to slow down and surface inconsistencies

**Operational guidance:**

- treat fork detection as an incident; halt the corridor
- rotate keys if compromise is suspected
- issue a fork alarm VC with evidence when receipts are available


### 2) Trust anchor circularity

**Attack:** A malicious repo fork adds a new signer to a trust-anchors file and issues “valid-looking” corridor artifacts.

**Mitigations in stack:**

- authority registry chaining (treaty → national → zone), pinned by digest
- optional strict verification that enforces registry allow-lists

**Operational guidance:**

- pin authority registry artifacts in lockfiles
- treat registry updates as ceremonies: multi-party review + signatures


### 3) Artifact withholding / availability attacks

**Attack:** A party commits to a digest but later refuses to serve the underlying artifact, making third-party verification impossible.

**Mitigations in stack:**

- uniform artifact CAS paths (`dist/artifacts/<type>/<digest>.*`)
- `--require-artifacts` strict mode, making completeness a first-class property
- lawpack availability attestations (optional monitoring)

**Operational guidance:**

- require each corridor participant to store all pinned artifacts
- pin to content-addressed public storage (IPFS/Arweave) when appropriate
- run periodic “completeness checks” as part of operations


### 4) Metadata leakage

**Attack:** Even when payloads are hidden, transition metadata (e.g., kinds) can leak economic activity.

**Mitigations in stack:**

- typed envelopes support payload-hash-only mode
- ZK hooks allow proof-carrying transitions later

**Operational guidance:**

- batch transitions when privacy matters
- use generic transition kinds plus encrypted payloads


## Verification modes

MEZ tooling supports a spectrum of strictness.

Recommended production modes:

- `--require-artifacts`: fail if any committed digest cannot be resolved
- `--enforce-authority-registry`: require authorized watchers/signers
- `--enforce-checkpoint-policy`: fail if corridor violates checkpoint policy
- watcher quorum monitoring with `--require-quorum` for liveness gates


## Incident response primitives

- **Watcher compare**: fast fork alarms without receipt transport
- **Fork alarm VC**: evidence-backed fork alarm (requires receipts)
- **Key rotation**: explicit policies and artifacts for rotation
- **Availability attestations**: operational evidence that pinned artifacts are stored and servable

See `spec/80-security-privacy.md` for the normative security specification.
