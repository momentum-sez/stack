# Architecture overview

**Momentum EZ Stack** v0.4.44 GENESIS

The EZ Stack is a compliance orchestration layer built in Rust that sits above Mass APIs and provides the intelligence that primitive CRUD alone cannot express: multi-domain compliance evaluation, cross-border corridor management, cryptographic audit trails, and autonomous policy execution.

---

## The two systems

### Mass APIs (not in this repo)

Mass implements the five programmable primitives as production API services:

| Primitive | Endpoint | Domain |
|-----------|----------|--------|
| Entities | `organization-info.api.mass.inc` | Formation, lifecycle, dissolution, beneficial ownership |
| Ownership | `investment-info` (Heroku) | Cap tables, share classes, transfers, fundraising |
| Fiscal | `treasury-info.api.mass.inc` | Accounts, payments, treasury operations |
| Identity | *(split across consent-info + org-info)* | KYC/KYB, credentials, DIDs |
| Consent | `consent.api.mass.inc` | Multi-party governance, audit trails |

Plus: `templating-engine` (Heroku) for document generation.

These have their own codebase, persistence, and deployment. They handle real entities, real capital, real government integrations.

### EZ Stack (this repository)

| Concern | Crate | What it adds beyond Mass CRUD |
|---------|-------|-------------------------------|
| Compliance intelligence | `mez-tensor`, `mez-compliance` | 20-domain evaluation, Dijkstra path optimization |
| Corridor operations | `mez-corridor`, `mez-state` | Receipt chains (MMR), fork resolution, netting, SWIFT |
| Jurisdictional config | `mez-pack` | Lawpacks (Akoma Ntoso), regpacks (sanctions), licensepacks |
| Cryptographic audit trail | `mez-vc`, `mez-crypto` | W3C VCs, Ed25519, content-addressed storage |
| Autonomous policy | `mez-agentic` | 20 trigger types, deterministic evaluation |
| Dispute resolution | `mez-arbitration` | 7-phase lifecycle, evidence, escrow, enforcement |
| Zero-knowledge proofs | `mez-zkp` | Sealed proof system, 12 circuit types |
| Schema enforcement | `mez-schema` | 116 JSON Schemas validated at API boundary |
| HTTP API | `mez-api` | Axum server composing all crates |
| CLI | `mez-cli` | Zone validation, lockfiles, signing |
| Mass gateway | `mez-mass-client` | Sole authorized HTTP client to Mass |

### The boundary

The EZ Stack **never stores primitive data**. Entity records, cap tables, payments, identity records, and consent records live in Mass. The EZ Stack stores:

- Compliance state (tensor snapshots, evaluation results)
- Corridor state (receipt chains, checkpoints, fork resolution)
- Verifiable Credentials (attestations, compliance proofs, corridor agreements)
- Zone configuration (module composition, lockfiles, pack trilogy)
- Audit trails (agentic actions, arbitration records)

---

## Data flow

```
                                ┌──────────────────────┐
                                │   Zone Admin / UI     │
                                └──────────┬───────────┘
                                           │
                                ┌──────────▼───────────┐
                                │    mez-api (Axum)     │
                                │  Auth -> Rate Limit   │
                                │  Trace -> Metrics     │
                                └──────────┬───────────┘
                                           │
                ┌──────────┬───────────────┼───────────────┬──────────┐
                │          │               │               │          │
         ┌──────▼──────┐ ┌─▼──────┐ ┌─────▼──────┐ ┌─────▼────┐ ┌───▼──────────┐
         │ mez-tensor  │ │mez-vc  │ │ mez-state  │ │mez-      │ │mez-mass-     │
         │ compliance  │ │ sign/  │ │ typestate   │ │agentic   │ │client        │
         │ evaluation  │ │ verify │ │ corridor    │ │ policy   │ │              │
         └──────┬──────┘ └───┬────┘ └─────┬──────┘ └────┬─────┘ └──────┬───────┘
                │            │            │              │              │
         ┌──────▼──────┐     │     ┌──────▼──────┐      │       ┌──────▼──────┐
         │mez-pack     │     │     │mez-corridor │      │       │ Mass APIs   │
         │ lawpack     │     │     │ receipt     │      │       │ (live)      │
         │ regpack     │     │     │ chain, fork │      │       │             │
         │ licensepack │     │     │ netting     │      │       │ org-info    │
         └─────────────┘     │     └──────┬──────┘      │       │ treasury    │
                             │            │             │       │ consent     │
                             │     ┌──────▼──────┐      │       │ ownership   │
                             │     │mez-crypto   │      │       └─────────────┘
                             │     │ Ed25519, MMR│      │
                             │     │ CAS, SHA-256│      │
                             │     └──────┬──────┘      │
                             │            │             │
                             └────►┌──────▼──────┐◄─────┘
                                   │ mez-core    │
                                   │ Canonical   │
                                   │ Bytes, IDs  │
                                   │ 20 Domains  │
                                   └─────────────┘
```

### A typical write operation

1. **Request** arrives at `POST /v1/corridors/{id}/receipts`
2. **Auth** validates bearer token (constant-time comparison)
3. **Rate limiter** checks token bucket for the route
4. **Corridor state** loaded; typestate validates transition (Active can append; Draft cannot)
5. **Compliance tensor** evaluated for both jurisdictions — all 20 domains
6. **Mass client** calls Mass if primitive data needed (e.g., entity jurisdiction check)
7. **Receipt chain** appends receipt to MMR, computes new root
8. **VC issuance** signs attestation with zone's Ed25519 key
9. **Agentic engine** evaluates triggered policies (e.g., sanctions update -> corridor halt)
10. **Response** returns receipt with MMR proof and attestation

---

## Artifact model

Everything operationally meaningful is an **artifact** that can be:

- **Canonicalized** — deterministic bytes via `CanonicalBytes::new()` (JCS + MCF coercions)
- **Digested** — SHA-256 via `ContentDigest`
- **Resolved** — by digest from the content-addressed store

```
dist/artifacts/
├── lawpack/      *.lawpack.zip     (Akoma Ntoso statutory corpus)
├── ruleset/      *.ruleset.json    (state transition rulesets)
├── checkpoint/   *.checkpoint.json (corridor checkpoints)
├── schema/       *.schema.json     (compiled schemas)
├── proof-key/    *.proof-key       (cryptographic proof keys)
├── circuit/      *.circuit         (ZK circuit definitions)
└── blob/         *                 (arbitrary content)
```

The `ArtifactRef` structure (`artifact_type` + `digest_sha256` + optional `uri`) is the universal reference format.

---

## Cryptographic invariants

### Canonicalization

`CanonicalBytes::new()` in `mez-core` is the sole path to digest computation. All signing flows require `&CanonicalBytes`. This eliminates bugs where different JSON serialization orders produce different digests.

The canonicalization follows JCS (RFC 8785) with Momentum Coercion Framework (MCF) extensions. The `serde_json` `preserve_order` feature is guarded by compile-time check, CI check, and runtime assertion.

### Signing

All signing uses Ed25519 via `ed25519-dalek` with `zeroize`. Private keys:
- Implement `Zeroize` + `ZeroizeOnDrop`
- Do **not** implement `Serialize`
- Accept only `&CanonicalBytes` for signing

### Receipt chains

Corridor receipts form an append-only chain backed by a Merkle Mountain Range (MMR). Each receipt commits to:
- `prev_root`: the previous state root (hash-chain continuity)
- `next_root`: canonical digest of the receipt payload
- `sequence`: monotonically increasing sequence number

Fork detection uses 3-level ordering: timestamp -> attestation count -> digest ordering. Clock skew tolerance: 5 minutes.

---

## State machines

The EZ Stack uses the **typestate pattern** extensively. Each state is a zero-sized type. Transitions consume `self` and return the next state type. Invalid transitions are compile errors.

| Machine | States | Crate |
|---------|--------|-------|
| Corridor | Draft, Pending, Active, Halted, Suspended, Deprecated | `mez-state` |
| Entity | Formation through Dissolution (10 stages) | `mez-state` |
| Migration | Phase0-Phase4, Completed, Aborted, CompensationFailed | `mez-state` |
| License | Pending, Active, Suspended, Revoked, Expired | `mez-state` |
| Watcher | Bonding, Active, Slashed, Unbonding | `mez-state` |

---

## Verification pipeline

A verifier checks:

1. **Configuration**: resolve and validate `zone.yaml`, `corridor.yaml`
2. **Artifacts**: resolve all pinned artifacts from CAS, verify digests
3. **Credentials**: verify Corridor Definition VC and Agreement VCs
4. **Receipt chain**: verify receipts form valid MMR chain (sequence, prev_root, next_root)
5. **Checkpoints**: verify checkpoint signatures and policy compliance
6. **Watcher signals**: check quorum and fork alarms

Strict modes: `--require-artifacts`, `--enforce-authority-registry`, `--enforce-checkpoint-policy`, `--require-quorum`.
