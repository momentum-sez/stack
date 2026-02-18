# Architecture overview

**Momentum EZ Stack** -- v0.4.44

The EZ Stack is a **compliance orchestration layer** built in Rust. It sits above the live Mass APIs and provides the intelligence that primitive CRUD operations alone cannot express: multi-domain compliance evaluation, cross-border corridor management, cryptographic audit trails, and autonomous policy execution.

This document covers the system design, data flow, key invariants, and the boundary between the EZ Stack and Mass.

---

## The two systems

There are two systems. They are not the same thing.

### Mass APIs (live, deployed, not in this repo)

Mass implements the **five programmable primitives** as production API services:

| Primitive | Endpoint | Domain |
|-----------|----------|--------|
| **Entities** | `organization-info.api.mass.inc` | Formation, lifecycle, dissolution, beneficial ownership |
| **Ownership** | `investment-info` (Heroku) | Cap tables, share classes, transfers, fundraising |
| **Fiscal** | `treasury-info.api.mass.inc` | Accounts, payments, treasury operations |
| **Identity** | *(embedded)* | KYC/KYB, passportable credentials, DIDs |
| **Consent** | `consent.api.mass.inc` | Multi-party governance, audit trails |

Plus: `templating-engine` (Heroku) for document generation.

These APIs have their own codebase, persistence, and deployment. They handle real entities, real capital, real government integrations.

### EZ Stack (this repository)

The EZ Stack is the **orchestrator**. It provides:

| Concern | Crate | What it adds beyond Mass primitives |
|---------|-------|-------------------------------------|
| Compliance intelligence | `msez-tensor`, `msez-compliance` | 20-domain compliance evaluation with Dijkstra path optimization across jurisdictions |
| Corridor operations | `msez-corridor`, `msez-state` | Cross-border receipt chains (MMR), fork resolution, netting, SWIFT pacs.008 |
| Jurisdictional configuration | `msez-pack` | Lawpacks (Akoma Ntoso statutes), regpacks (sanctions), licensepacks (license registries) |
| Cryptographic audit trail | `msez-vc`, `msez-crypto` | W3C Verifiable Credentials, Ed25519 signing, content-addressed storage |
| Autonomous policy execution | `msez-agentic` | 20 trigger types, deterministic evaluation, conflict resolution |
| Dispute resolution | `msez-arbitration` | 7-phase dispute lifecycle, evidence chain-of-custody, escrow, enforcement |
| Zero-knowledge proofs | `msez-zkp` | Sealed proof system with 12 circuit types (Phase 1: mock backend) |
| Schema enforcement | `msez-schema` | 116 JSON Schema files validated at the API boundary |
| HTTP API | `msez-api` | Axum server composing all of the above |
| CLI | `msez-cli` | Offline zone management: validation, lockfiles, signing |
| Mass API client | `msez-mass-client` | Typed Rust HTTP client -- the only path from EZ Stack to Mass |

### The boundary

The EZ Stack **never stores primitive data**. Entity records, cap tables, payment records, identity records, and consent records live in Mass. The EZ Stack stores:

- Compliance state (tensor snapshots, evaluation results)
- Corridor state (receipt chains, checkpoints, fork resolution records)
- Verifiable Credentials (attestations, compliance proofs, corridor agreements)
- Zone configuration (module composition, lockfiles, pack trilogy)
- Audit trails (agentic actions, arbitration records)

---

## Data flow

```
                                    ┌──────────────────────┐
                                    │    Zone Admin / UI    │
                                    └──────────┬───────────┘
                                               │
                                    ┌──────────▼───────────┐
                                    │     msez-api (Axum)   │
                                    │  Auth → Rate Limit    │
                                    │  Trace → Metrics      │
                                    └──────────┬───────────┘
                                               │
                    ┌──────────────┬────────────┼────────────┬──────────────┐
                    │              │            │            │              │
             ┌──────▼──────┐ ┌────▼────┐ ┌─────▼─────┐ ┌───▼───┐ ┌───────▼───────┐
             │ msez-tensor │ │msez-vc  │ │msez-state │ │msez-  │ │msez-mass-     │
             │ Compliance  │ │ VC sign │ │ Typestate │ │agentic│ │client         │
             │ evaluation  │ │ /verify │ │ corridor  │ │ policy│ │               │
             └──────┬──────┘ └────┬────┘ └─────┬─────┘ └───┬───┘ └───────┬───────┘
                    │              │            │            │              │
             ┌──────▼──────┐      │     ┌──────▼──────┐     │       ┌─────▼──────┐
             │msez-        │      │     │msez-        │     │       │ Mass APIs  │
             │compliance   │      │     │corridor     │     │       │ (live)     │
             │ regpack →   │      │     │ receipt     │     │       │            │
             │ tensor      │      │     │ chain, fork │     │       │ org-info   │
             └──────┬──────┘      │     │ netting     │     │       │ treasury   │
                    │              │     └──────┬──────┘     │       │ consent    │
             ┌──────▼──────┐      │            │            │       │ identity   │
             │ msez-pack   │      │     ┌──────▼──────┐     │       │ ownership  │
             │ lawpack     │      │     │msez-crypto  │     │       └────────────┘
             │ regpack     │      │     │ Ed25519     │     │
             │ licensepack │      │     │ MMR, CAS    │     │
             └─────────────┘      │     │ SHA-256     │     │
                                  │     └──────┬──────┘     │
                                  │            │            │
                                  │     ┌──────▼──────┐     │
                                  └────►│ msez-core   │◄────┘
                                        │ Canonical   │
                                        │ Bytes, IDs  │
                                        │ Domains     │
                                        └─────────────┘
```

### A typical corridor operation

1. **Request arrives** at `POST /v1/corridors/{id}/receipts`
2. **Auth middleware** validates bearer token (constant-time comparison)
3. **Rate limiter** checks token bucket for the route
4. **Corridor state** is loaded; typestate machine validates the transition is legal (Active corridors can append receipts; Draft corridors cannot)
5. **Compliance tensor** is evaluated for both jurisdictions in the corridor -- all 20 domains checked
6. **Mass API client** calls into Mass if primitive data is needed (e.g., entity jurisdiction check)
7. **Receipt chain** appends the new receipt to the MMR, computing a new root
8. **VC issuance** signs an attestation for the receipt with the zone's Ed25519 key
9. **Agentic engine** evaluates whether any policies trigger (e.g., sanctions list update → corridor halt)
10. **Response** returns the receipt with MMR proof and attestation

---

## Artifact model

Everything that matters operationally is an **artifact** that can be:

- **Canonicalized** (deterministic bytes via JCS-compatible `CanonicalBytes::new()`)
- **Digested** (SHA-256 via `ContentDigest`)
- **Resolved** by digest from the content-addressed store

Artifacts live in `dist/artifacts/<type>/<digest>.*`:

```
dist/artifacts/
├── lawpack/     *.lawpack.zip    (Akoma Ntoso statutory corpus)
├── ruleset/     *.ruleset.json   (state transition rulesets)
├── checkpoint/  *.checkpoint.json (corridor checkpoints)
├── schema/      *.schema.json    (compiled schemas)
├── proof-key/   *.proof-key      (cryptographic proof keys)
├── circuit/     *.circuit         (ZK circuit definitions)
└── blob/        *                 (arbitrary content)
```

The `ArtifactRef` structure (`artifact_type` + `digest_sha256` + optional `uri`) is the universal reference format across schemas.

---

## Cryptographic invariants

### Canonicalization

`CanonicalBytes::new()` (in `msez-core`) is the **sole path** to digest computation. All signing flows require `&CanonicalBytes`. This eliminates the class of bugs where different JSON serialization orders produce different digests for the same logical value.

The canonicalization follows JCS (JSON Canonicalization Scheme, RFC 8785) with Momentum-specific type coercion rules. The `serde_json` `preserve_order` feature is guarded by a three-layer defense (compile-time check, CI check, runtime assertion).

### Signing

All signing uses Ed25519 (via `ed25519-dalek` with `zeroize` feature). Private keys:
- Implement `Zeroize` + `ZeroizeOnDrop` (memory is scrubbed on drop)
- Do **not** implement `Serialize` (cannot be accidentally exported to JSON)
- Accept only `&CanonicalBytes` for signing (cannot sign non-canonical data)

### Receipt chains

Corridor receipts form an append-only chain backed by a Merkle Mountain Range (MMR). Each receipt commits to:
- `prev_root`: the MMR root before this receipt
- `next_root`: the canonical digest of this receipt's payload
- `sequence`: monotonically increasing sequence number

Fork detection uses 3-level ordering per spec section 3.5:
1. **Primary**: Lexicographically earlier timestamp
2. **Secondary**: More watcher attestations
3. **Tertiary**: Lexicographic digest ordering

Clock skew tolerance: 5 minutes.

---

## Compliance tensor

The compliance tensor is the core intelligence of the EZ Stack. It evaluates:

```
T(entity, jurisdiction) : ComplianceDomain → ComplianceState
```

Where:
- **ComplianceDomain** has 20 variants: `Aml`, `Kyc`, `Sanctions`, `Tax`, `Securities`, `Corporate`, `Custody`, `DataPrivacy`, `Licensing`, `Banking`, `Payments`, `Clearing`, `Settlement`, `DigitalAssets`, `Employment`, `Immigration`, `Ip`, `ConsumerProtection`, `Arbitration`, `Trade`
- **ComplianceState** is a 5-value lattice: `NotApplicable` > `Exempt` > `Compliant` > `Pending` > `NonCompliant`

The tensor is parameterized by a `JurisdictionConfig` that determines which domains are applicable in a given jurisdiction. The `msez-compliance` crate bridges regpack data into jurisdiction configurations.

**Manifold optimization**: The `ComplianceManifold` models jurisdictions as nodes and corridors as weighted edges. Dijkstra shortest-path finds the optimal migration route subject to constraints (max fee, max time, max risk, excluded jurisdictions).

**Commitment**: Tensor state is Merkle-committed for anchoring to L1 chains or inclusion in VCs.

---

## State machines

The EZ Stack uses the **typestate pattern** extensively. Each lifecycle state is a distinct zero-sized type (ZST). Transitions are methods that consume `self` and return the next state type. Invalid transitions are compile errors.

### Corridor lifecycle (6 states)

```
Draft ──submit──> Pending ──activate──> Active ──halt──> Halted ──deprecate──> Deprecated
                                          │
                                          └──suspend──> Suspended ──resume──> Active
```

`Corridor<Draft>` has `.submit()` but no `.halt()`. The compiler enforces it.

### Entity lifecycle (10 stages)

```
Formation → Operational → Expansion/Contraction → Restructuring → Suspension → Dissolution (7 sub-stages)
```

### Migration saga (8 phases)

```
Phase0 (Initiation) → Phase1 (Planning) → Phase2 (Execution) → Phase3 (Settlement) → Phase4 (Finalization)
```

Terminal states: `Completed`, `Aborted`, `CompensationFailed`.

### Watcher lifecycle (4 states)

```
Bonding ──> Active ──> Slashed ──> Unbonding
```

Slashing conditions: `InvalidProof`, `EquivocationDetected`, `InactivityViolation`, `PerjuryDetected`.

---

## Agentic engine

The agentic engine provides **autonomous policy execution** in response to environmental triggers. 20 trigger types cover regulatory changes, arbitration events, corridor state changes, fiscal events, and entity lifecycle events.

**Determinism guarantee** (Theorem 17.1): Given identical trigger events and policy state, evaluation produces identical scheduled actions. This is enforced by:
- `BTreeMap` for policy storage (deterministic iteration order)
- Pure condition evaluation (no external state)
- Deterministic conflict resolution: Priority → Jurisdiction specificity → Policy ID

---

## Verification pipeline

A verifier checks:

1. **Configuration**: Resolve and validate `zone.yaml`, `corridor.yaml`
2. **Artifacts**: Resolve all pinned artifacts from CAS, verify digests
3. **Credentials**: Verify Corridor Definition VC and Agreement VCs
4. **Receipt chain**: Verify receipts form a valid MMR chain (sequence, prev_root, next_root)
5. **Checkpoints**: Verify checkpoint signatures and policy compliance
6. **Watcher signals**: Check quorum and fork alarms

Strict modes: `--require-artifacts`, `--enforce-authority-registry`, `--enforce-checkpoint-policy`, `--require-quorum`.

---

## Why this architecture

The system is designed to be **machine-intelligence traversable**:

- The legal substrate (lawpacks) is pinned and content-addressed
- Operational commitments are explicit and resolvable
- Verification is deterministic and automatable
- Governance and authorization are separated from transport and storage
- The corridor state channel model is settlement-rail agnostic (can anchor to Ethereum, L2, or operate standalone)

The type system does the enforcement. If it compiles, the state transitions are valid, the signatures use canonical bytes, the identifiers don't mix, and the proof backends are authorized.
