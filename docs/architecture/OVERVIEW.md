# Architecture Overview

**v0.4.44 GENESIS — 146/146 Modules (100%)**

The Momentum SEZ Stack (MSEZ) is a **programmable jurisdictional operating system**: a modular, content-addressed substrate for deploying zones (nodes) and connecting them with cryptographically-verifiable corridors (state channels) that can be governed by pinned lawpacks and rulesets.

## System Capabilities

**PHOENIX Smart Asset Operating System — 13,068 lines across 17 modules**

| Layer | Module | Lines | Description |
|-------|--------|-------|-------------|
| **Layer 1: Asset Intelligence** ||||
|| Compliance Tensor | 955 | 4D sparse tensor for compliance state |
|| ZK Proofs | 766 | Groth16/PLONK/STARK verification |
|| Smart Asset VM | 1,285 | Stack-based execution with gas metering |
| **Layer 2: Jurisdictional Infrastructure** ||||
|| Compliance Manifold | 1,009 | Dijkstra path planning across jurisdictions |
|| Migration Protocol | 886 | Saga-based migration with compensation |
|| Corridor Bridge | 822 | Two-phase commit for multi-hop transfers |
|| L1 Anchor | 816 | Ethereum/L2 settlement finality |
| **Layer 3: Network Coordination** ||||
|| Watcher Economy | 750 | Bonded attestors with slashing |
|| Security Layer | 993 | Nonces, versioning, time locks |
|| Hardening Layer | 744 | Input validation, thread safety |
| **Layer 4: Operations** ||||
|| Health Framework | 400 | Kubernetes liveness/readiness probes |
|| Observability | 500 | Structured logging, distributed tracing |
|| Configuration | 492 | YAML/environment binding, validation |
|| CLI Framework | 450 | Unified command interface |
| **Layer 5: Infrastructure Patterns** ||||
|| Resilience | 750 | Circuit breaker, retry, bulkhead, timeout |
|| Events | 650 | Event bus, event sourcing, saga pattern |
|| Cache | 600 | LRU/TTL caching, tiered cache |

This document describes the system at three layers:

1. **Artifacts & commitments** (what is pinned and how it is resolved)
2. **Nodes** (zones) as jurisdictional execution environments
3. **Corridors** as verifiable state channels linking nodes


## Core primitives

### Artifact commitments

MSEZ treats *everything that matters operationally* as an artifact that can be:

- **canonicalized** (deterministic bytes)
- **digested** (SHA-256)
- **resolved** by digest in a local artifact store

The repository uses a uniform convention:

```
dist/artifacts/<type>/<digest>.*
```

Examples:

- `dist/artifacts/lawpack/<digest>.lawpack.zip`
- `dist/artifacts/ruleset/<digest>.ruleset.json`
- `dist/artifacts/checkpoint/<digest>.checkpoint.json`
- `dist/artifacts/vc/<digest>.vc.json`
- `dist/artifacts/schema/<digest>.schema.json`

This makes any commitment in receipts or VCs mechanically resolvable.


### ArtifactRef

Most schemas accept a reusable `ArtifactRef` object:

```json
{
  "artifact_type": "lawpack",
  "digest_sha256": "<64-hex>",
  "uri": "ipfs://..." 
}
```

The **digest** is the integrity anchor; the optional `uri` is a distribution hint.


### Authority registry chaining

MSEZ separates *cryptography* from *authorization*.

Keys can always produce valid signatures, but whether a signature is **authorized** for a particular action is governed by an **authority registry chain**:

```
treaty body → national authority → zone authority
```

Each link is itself an artifact, pinned and verifiable. This prevents “trust anchor circularity” where a repo fork can silently add new signers.


## Nodes

A **zone** is a node that composes:

- a profile (what modules it intends to run)
- a set of pinned artifacts (lawpacks, schemas, rulesets, registries)
- operator configuration (deployment environment, governance parameters)

In practice, zones are instantiated from profiles under `profiles/*` and resolved into deployable module sets using `stack.lock`.


## Corridors

A **corridor** is a cryptographically meaningful interoperability link between nodes.

In MSEZ, corridors are specified and operated as **verifiable state channels**:

- A corridor definition VC describes the corridor and its compatibility constraints.
- Participant-specific corridor agreement VCs express consent and activate the corridor (often thresholded).
- The corridor’s operational history is an append-only sequence of **signed receipts**.
- A corridor can periodically publish **checkpoints** to enable fast sync.

Watchers can publish signed attestations of the observed head to detect forks cheaply, without transporting receipts.


## Verification pipeline

At a high level, a verifier checks:

1. **Resolve & validate configuration** (`corridor.yaml`, `zone.yaml`)
2. **Resolve pinned artifacts** (CAS) and check digests
3. **Verify credentials** (Corridor Definition VC, Agreement VCs)
4. **Verify state channel history** (receipts → state roots)
5. **Verify checkpoints** (signatures + policy)
6. **Apply watcher signals** (quorum, fork alarms)

The tooling supports strict modes (e.g., `--require-artifacts`) where verification fails if any committed digest cannot be resolved.


## Why this architecture

The core design goal is to make the system **machine-intelligence traversable**:

- The legal substrate (lawpacks) is pinned and content-addressed.
- Operational commitments are explicit and resolvable.
- Verification is deterministic and automatable.
- Governance and authorization are separated from transport and storage.

This produces a substrate that is **complementary to blockchains**: corridors can later be anchored to an external chain, but the corridor state channel model stands on its own and remains settlement-rail agnostic.
