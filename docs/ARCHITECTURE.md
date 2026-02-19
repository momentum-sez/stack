# Architecture

**Momentum EZ Stack** v0.4.44 GENESIS — 16 crates, 151K lines of Rust, 4,073 tests

This document describes the system architecture. For per-crate API details, see [Crate Reference](./architecture/CRATE-REFERENCE.md). For the Mass integration boundary, see [Mass Integration](./architecture/MASS-INTEGRATION.md).

---

## Foundational premise

Traditional Economic Zones take 3-7 years and $50-200M to establish: bilateral treaties, regulatory frameworks, banking relationships, corporate registries, dispute resolution, licensing regimes.

The EZ Stack reduces this to configuration. A `zone.yaml` file selects jurisdictions, composes modules, and the Rust workspace provides:

- **Type-level correctness** — invalid state transitions don't compile
- **Cryptographic integrity** — every state transition produces verifiable proof
- **Compliance intelligence** — 20-domain tensor evaluation with Dijkstra-optimized migration paths
- **Autonomous policy execution** — 20 trigger types with deterministic conflict resolution

---

## System layers

```
┌───────────────────────────────────────────────────────────────┐
│                      HTTP BOUNDARY                             │
│  mez-api:  Axum server, auth, rate limiting, OpenAPI          │
│  mez-cli:  Offline zone management, validation, signing       │
├───────────────────────────────────────────────────────────────┤
│                   ORCHESTRATION LAYER                          │
│  mez-agentic:     20 triggers, policy engine, audit trail     │
│  mez-arbitration:  Dispute lifecycle, evidence, escrow        │
│  mez-compliance:   Regpack -> tensor bridge                   │
├───────────────────────────────────────────────────────────────┤
│                 DOMAIN INTELLIGENCE                            │
│  mez-tensor:   Compliance Tensor V2 (20 domains, 5 states)   │
│  mez-corridor: Receipt chains, fork resolution, netting       │
│  mez-state:    Typestate machines (corridor, entity, etc.)    │
│  mez-pack:     Lawpack, regpack, licensepack                  │
├───────────────────────────────────────────────────────────────┤
│                CRYPTOGRAPHIC FOUNDATION                        │
│  mez-vc:     W3C Verifiable Credentials, Ed25519 proofs       │
│  mez-crypto: Ed25519 (zeroize), MMR, CAS, SHA-256            │
│  mez-zkp:    Sealed ProofSystem trait, 12 circuits            │
│  mez-core:   CanonicalBytes, ComplianceDomain(20), newtypes  │
├───────────────────────────────────────────────────────────────┤
│                  EXTERNAL INTEGRATION                          │
│  mez-mass-client: Typed HTTP client for 5 Mass primitives     │
│  mez-schema:      116 JSON Schema (Draft 2020-12) validation  │
└───────────────────────────────────────────────────────────────┘
```

---

## The Mass/EZ boundary

**Mass APIs** (Java, not in this repo) own CRUD for five primitives: Entities, Ownership, Fiscal, Identity, Consent. **The EZ Stack** owns everything above primitive CRUD: compliance evaluation, corridor state, verifiable credentials, zone configuration, audit trails.

The boundary rule: if it's "create/read/update/delete a business object" — Mass. If it's "evaluate whether that operation is compliant in this jurisdiction" — this repo. `mez-mass-client` is the sole authorized gateway.

The EZ Stack **never stores primitive data**. Entity records, cap tables, payments, identity records, and consent records live in Mass.

---

## Compliance Tensor

The tensor is a function:

```
T(entity, jurisdiction) : ComplianceDomain -> ComplianceState
```

20 domains, each producing a 5-state lattice value:

```
NotApplicable > Exempt > Compliant > Pending > NonCompliant
```

Parameterized by `JurisdictionConfig` that determines which domains apply. The `ComplianceManifold` models jurisdictions as nodes, corridors as weighted edges, and Dijkstra shortest-path computes optimal migration routes subject to constraints (max fee, time, risk). Tensor state is Merkle-committed for anchoring and VC inclusion.

---

## Corridors

A **corridor** is a bilateral trade channel between jurisdictions:

- **Receipt chain**: append-only signed receipts backed by a Merkle Mountain Range (MMR)
- **Fork detection**: watchers monitor for conflicting receipts at the same height
- **Fork resolution**: 3-level deterministic ordering (timestamp, attestation count, digest)
- **Netting**: bilateral and multilateral settlement compression
- **SWIFT pacs.008**: ISO 20022 payment instruction generation
- **L1 anchoring**: optional checkpoint anchoring to external chains

### Corridor lifecycle (typestate-enforced)

```
Draft ──submit──> Pending ──activate──> Active ──halt──> Halted ──deprecate──> Deprecated
                                          │
                                          └──suspend──> Suspended ──resume──> Active
```

`Corridor<Draft>` has `.submit()` but no `.halt()`. Attempting to halt a draft is a compile error.

---

## Agentic policy engine

20 trigger types drive autonomous policy evaluation:

| Category | Triggers |
|----------|----------|
| Regulatory | `SanctionsListUpdate`, `LicenseStatusChange`, `GuidanceUpdate`, `ComplianceDeadline` |
| Arbitration | `DisputeFiled`, `RulingReceived`, `AppealPeriodExpired`, `EnforcementDue` |
| Corridors | `CorridorStateChange`, `SettlementAnchorAvailable`, `WatcherQuorumReached` |
| Assets | `CheckpointDue`, `KeyRotationDue`, `GovernanceVoteResolved` |
| Fiscal | `TaxYearEnd`, `WithholdingDue` |
| Entity | `EntityDissolution`, `PackUpdated`, `AssetTransferInitiated`, `MigrationDeadline` |

**Determinism guarantee** (Theorem 17.1): given identical trigger events and policy state, evaluation produces identical scheduled actions. Enforced by `BTreeMap` iteration, pure condition evaluation, and priority-based conflict resolution.

---

## Arbitration

7-phase dispute lifecycle:

```
Filed -> Evidence -> Hearing -> Decision -> Enforcement
                                    │
                                    ├-> Settled
                                    ├-> Dismissed
                                    └-> Review
```

Evidence is content-addressed with chain-of-custody tracking. Escrow supports configurable release conditions. Enforcement actions (`PayAmount`, `TransferAsset`, `FreezeAsset`, `UpdateCompliance`) execute via VC-triggered state transitions.

---

## Pack Trilogy

| Pack | Source | Configures |
|------|--------|-----------|
| **Lawpack** | Akoma Ntoso XML | Statutory corpus: enabling acts, tax law, corporate law |
| **Regpack** | YAML/JSON | Sanctions lists, reporting obligations, compliance calendars |
| **Licensepack** | YAML/JSON | License types, issuing authorities, validity periods |

Each pack is content-addressed (SHA-256 via `CanonicalBytes`) and version-controlled. The `SanctionsChecker` in `mez-pack` provides fuzzy name matching against OFAC/UN/EU sanctions lists with configurable confidence thresholds.

---

## Zero-knowledge proofs

`mez-zkp` defines a **sealed** `ProofSystem` trait — external crates cannot implement it.

**Phase 1** (current): `MockProofSystem` produces deterministic SHA-256 mock proofs across 12 circuit types. Production policy (`ProofPolicy`) rejects mock proofs in release builds.

**Phase 2** (feature-gated): Groth16 (`groth16` flag) and PLONK (`plonk` flag) backends. The Canonical Digest Bridge (CDB) transforms `SHA256(JCS(A))` to `Poseidon2(Split256(...))` for efficient in-circuit verification.

---

## Watcher economy

Watchers are bonded attestors that monitor corridor receipt chains:

```
Bonding -> Active -> Slashed -> Unbonding
```

Slashing conditions: `InvalidProof`, `EquivocationDetected`, `InactivityViolation`, `PerjuryDetected`. Fork detection relies on watcher attestation quorum — a fork alarm is raised when watchers report conflicting head roots.

---

## Cryptographic invariants

| Invariant | Enforcement |
|-----------|-------------|
| All digests via canonical path | `CanonicalBytes::new()` is the sole entry to `sha256_digest()` |
| No non-canonical signing | Signing requires `&CanonicalBytes` |
| Key material zeroed on drop | `SigningKey`: `Zeroize` + `ZeroizeOnDrop`, no `Serialize` |
| Deterministic serialization | `serde_json` `preserve_order` guarded (compile-time, CI, runtime) |
| Receipt chain continuity | `receipt.prev_root == final_state_root` |
| Receipt commitment integrity | `receipt.next_root == SHA256(JCS(payload_sans_proof))` |

---

## Security model

### What cryptography guarantees

- **Integrity**: content-addressed artifacts verified by digest; receipts chain via roots
- **Authorship**: Ed25519 signatures verify the signer's key
- **Causality**: MMR receipt chains enforce append-only ordering

### What cryptography does not guarantee

- **Legal force** of a lawpack digest (requires governance)
- **Social legitimacy** of an authority registry (requires operational controls)
- **Availability** of artifacts (requires redundancy and monitoring)

See [Security Model](./architecture/SECURITY-MODEL.md) for the full threat model and mitigation matrix.

---

## Deployment architecture

| Environment | Components |
|-------------|-----------|
| **Docker Compose** | `mez-api` binary + PostgreSQL 16 + Prometheus + Grafana |
| **Kubernetes** | 2 replicas, rolling updates, non-root, resource limits, probes |
| **AWS (Terraform)** | EKS (auto-scaling) + RDS (Multi-AZ) + ElastiCache + S3 + KMS + ALB/TLS |

---

## Design principles

| Principle | How |
|-----------|-----|
| **The type system does the work** | Typestate machines, sealed traits, identifier newtypes, exhaustive matching |
| **Single source of truth** | `CanonicalBytes` for digests, `ComplianceDomain` for all 20 domains, Mass for primitives |
| **Fail closed** | Unknown = `NonCompliant`. Missing attestations invalidate. Empty tensor slices = error. |
| **No magic** | No floating point. No randomness. No external state in evaluation. `BTreeMap` ordering. |
| **Defense in depth** | Zeroize on drop. Constant-time auth. Rate limit after auth. Schema validation at boundary. |
| **Verify, never trust** | All inputs validated. Signatures verified. Digests recomputed. Chains verified. |
