# Architecture

## Momentum SEZ Stack -- Rust Workspace

**v0.4.44** -- 16 crates, ~48K lines of Rust

This document describes the architectural foundations of the SEZ Stack: a compliance orchestration layer built in Rust that sits above the live Mass APIs and provides the intelligence that primitive CRUD operations alone cannot express.

For the crate-by-crate API reference, see [Crate Reference](./architecture/CRATE-REFERENCE.md).
For the system overview, see [Architecture Overview](./architecture/OVERVIEW.md).

---

## Foundational premise

Traditional Special Economic Zones take 3-7 years and $50-200M to establish. They require bilateral treaties, regulatory frameworks, banking relationships, corporate registries, dispute resolution, and licensing regimes.

The SEZ Stack reduces this to configuration files backed by a Rust workspace that provides:

- **Type-level correctness**: Invalid state transitions don't compile. Identifier types can't mix. Proof backends are sealed.
- **Cryptographic integrity**: Every state transition produces verifiable proof via Ed25519 signatures, MMR-backed receipt chains, and W3C Verifiable Credentials.
- **Compliance intelligence**: A 20-domain compliance tensor evaluates regulatory state per entity/jurisdiction pair, with Dijkstra-optimized migration paths across the jurisdiction graph.
- **Autonomous policy execution**: 20 trigger types drive deterministic policy evaluation with formal conflict resolution guarantees (Theorem 17.1).

---

## System layers

The workspace is organized around distinct concerns, each implemented by one or more crates:

```
┌─────────────────────────────────────────────────────────────────────┐
│                        HTTP BOUNDARY                                │
│  msez-api: Axum server, auth, rate limiting, OpenAPI generation     │
│  msez-cli: Offline zone management, validation, signing            │
├─────────────────────────────────────────────────────────────────────┤
│                     ORCHESTRATION LAYER                              │
│  msez-agentic:     20 triggers, policy engine, audit trail          │
│  msez-arbitration: Dispute lifecycle, evidence, escrow              │
│  msez-compliance:  Regpack → tensor bridge                          │
├─────────────────────────────────────────────────────────────────────┤
│                   DOMAIN INTELLIGENCE                               │
│  msez-tensor:  Compliance Tensor V2 (20 domains, 5-state lattice)  │
│  msez-corridor: Receipt chains, fork resolution, netting, SWIFT     │
│  msez-state:   Typestate machines (corridor, entity, migration)     │
│  msez-pack:    Lawpack, regpack, licensepack (Pack Trilogy)          │
├─────────────────────────────────────────────────────────────────────┤
│                  CRYPTOGRAPHIC FOUNDATION                           │
│  msez-vc:     W3C Verifiable Credentials, Ed25519 proofs            │
│  msez-crypto: Ed25519 (zeroize), MMR, CAS, SHA-256                 │
│  msez-zkp:    Sealed ProofSystem trait, 12 circuits, CDB bridge     │
│  msez-core:   CanonicalBytes, ComplianceDomain(20), ID newtypes     │
├─────────────────────────────────────────────────────────────────────┤
│                    EXTERNAL INTEGRATION                             │
│  msez-mass-client: Typed HTTP client for 5 Mass API primitives      │
│  msez-schema:      116 JSON Schema (Draft 2020-12) validation       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Smart Assets

A **Smart Asset** is an asset with embedded compliance intelligence. It carries a compliance tensor that tracks its regulatory state across all applicable domains and jurisdictions. The asset knows:

- Whether it is compliant in a given jurisdiction
- What attestations are missing
- The optimal migration path to a target jurisdiction (via the compliance manifold)

Smart Assets are the SEZ Stack's programmable substrate. An entity IS a Smart Asset. An ownership position IS a Smart Asset. The SEZ Stack manages their compliance lifecycle; Mass manages the underlying primitive data.

### Compliance Tensor

The compliance tensor is a function:

```
T(entity, jurisdiction) : ComplianceDomain → ComplianceState
```

20 regulatory domains, each producing a 5-state lattice value:

```
NotApplicable > Exempt > Compliant > Pending > NonCompliant
```

The `msez-tensor` crate implements this with pluggable `DomainEvaluator` instances per domain, parameterized by a `JurisdictionConfig` that determines which domains apply.

### Compliance Manifold

The `ComplianceManifold` models jurisdictions as nodes and corridors as weighted edges. Each edge carries a `ComplianceDistance` (fee, time, risk). Dijkstra shortest-path computes optimal migration routes subject to constraints:

```rust
manifold.shortest_path(
    &from_jurisdiction,
    &to_jurisdiction,
    &[PathConstraint::MaxFee(1000.into()), PathConstraint::MaxDays(30)]
) // → Option<MigrationPath>
```

### Tensor Commitment

Tensor state is Merkle-committed via `TensorCommitment`. This allows:
- Anchoring compliance state to L1 chains
- Including compliance snapshots in Verifiable Credentials
- Auditable proof of compliance evaluation at a point in time

---

## Corridors

A **corridor** is a bilateral trade channel between two jurisdictions. It provides:

- **Receipt chain**: Append-only sequence of signed receipts backed by a Merkle Mountain Range (MMR). Each receipt commits to `prev_root` and `next_root`, forming a tamper-evident chain.
- **Fork detection**: Watchers monitor the receipt chain. Forks are detected when two receipts share the same `prev_root` but different `next_root`.
- **Fork resolution**: 3-level deterministic ordering (timestamp → attestation count → digest).
- **Netting**: Bilateral and multilateral settlement compression via `NettingEngine`.
- **SWIFT pacs.008**: ISO 20022 payment instruction generation via `SwiftPacs008`.
- **L1 anchoring**: Optional anchoring of checkpoints to external chains.

### Corridor lifecycle

The corridor lifecycle is enforced at the type level via the typestate pattern:

```
Draft ──submit──> Pending ──activate──> Active ──halt──> Halted ──deprecate──> Deprecated
                                          │
                                          └──suspend──> Suspended ──resume──> Active
```

`Corridor<Draft>` has a `.submit()` method but no `.halt()` method. Attempting to halt a draft corridor is a compile error, not a runtime error.

---

## Agentic policy engine

The agentic engine executes autonomous policies in response to environmental triggers. When a sanctions list updates, the engine evaluates all affected corridors and freezes non-compliant ones. When a license expires, the engine suspends the entity. These policies operate ACROSS Mass primitives.

### 20 trigger types

| Category | Triggers |
|----------|----------|
| Regulatory | `SanctionsListUpdate`, `LicenseStatusChange`, `GuidanceUpdate`, `ComplianceDeadline` |
| Arbitration | `DisputeFiled`, `RulingReceived`, `AppealPeriodExpired`, `EnforcementDue` |
| Corridors | `CorridorStateChange`, `SettlementAnchorAvailable`, `WatcherQuorumReached` |
| Assets | `CheckpointDue`, `KeyRotationDue`, `GovernanceVoteResolved` |
| Fiscal | `TaxYearEnd`, `WithholdingDue` |
| Entity | `EntityDissolution`, `PackUpdated`, `AssetTransferInitiated`, `MigrationDeadline` |

### Determinism guarantee (Theorem 17.1)

Given identical trigger events and policy state, evaluation produces identical scheduled actions. Enforced by:
- `BTreeMap` for policy storage (deterministic iteration order)
- Pure condition evaluation (no external state, no randomness)
- Conflict resolution: Priority → Jurisdiction specificity → Policy ID

---

## Arbitration

The arbitration system manages dispute resolution through a 7-phase lifecycle:

```
Filed → EvidencePhase → HearingScheduled → UnderDecision → Decided → EnforcementInitiated
                                                            │
                                                            └→ Settled
                                                            └→ DismissedOrWithdrawn
                                                            └→ ReviewInitiated
```

Key capabilities:
- **Evidence management**: Content-addressed evidence items with authenticity attestations and chain-of-custody tracking
- **Escrow**: Held funds with configurable release conditions (dispute resolved, time elapsed, ruling enforced)
- **Enforcement**: Ordered actions (`PayAmount`, `TransferAsset`, `FreezeAsset`, `UpdateCompliance`) executed via VC-triggered state transitions

---

## Pack Trilogy

The Pack Trilogy provides jurisdictional configuration:

| Pack | Source format | What it configures |
|------|---------------|-------------------|
| **Lawpack** | Akoma Ntoso XML | Statutory corpus: enabling acts, tax law, corporate law |
| **Regpack** | YAML/JSON | Regulatory requirements: sanctions lists, reporting obligations, compliance calendars |
| **Licensepack** | YAML/JSON | License registry: license types, issuing authorities, validity periods, renewal windows |

Each pack is content-addressed (SHA-256 digest via `CanonicalBytes`) and version-controlled. The `SanctionsChecker` in `msez-pack` provides fuzzy name matching against OFAC/UN/EU sanctions lists with configurable confidence thresholds.

---

## Zero-knowledge proofs

The `msez-zkp` crate defines a **sealed** `ProofSystem` trait that external crates cannot implement. This prevents unauthorized proof backends from entering the system.

### Phase 1 (current)

`MockProofSystem`: Deterministic SHA-256 mock. All 12 circuit types produce transparent, reproducible proofs. Used for development, testing, and cross-language parity verification.

### Phase 2 (feature-gated)

| Backend | Feature flag | Framework |
|---------|-------------|-----------|
| Groth16 SNARK | `groth16` | arkworks |
| PLONK | `plonk` | halo2 |
| Poseidon2 in CDB | `poseidon2` | *(built-in)* |

### Canonical Digest Bridge (CDB)

The CDB transformation bridges between SHA-256 (used everywhere in the stack) and ZK-friendly hashing:

```
CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))
```

Phase 1: Poseidon2 is an identity transform. Phase 2: Poseidon2 is activated for efficient in-circuit verification.

### 12 circuit types

Compliance: `BalanceSufficiency`, `SanctionsClearance`, `TensorInclusion`
Migration: `MigrationEvidence`, `OwnershipChain`, `CompensationValidity`
Identity: `KycAttestation`, `AttestationValidity`, `ThresholdSignature`
Settlement: `RangeProof`, `MerkleMembership`, `NettingValidity`

---

## Mass API integration

The SEZ Stack does not reimplement Mass primitives. It orchestrates them.

| Mass Primitive | Mass API Endpoint | SEZ Stack adds |
|----------------|-------------------|----------------|
| Entities | `organization-info.api.mass.inc` | Compliance tensor evaluation, VC issuance, state machine lifecycle |
| Ownership | `investment-info` | Smart asset binding, corridor-aware transfer validation |
| Fiscal | `treasury-info.api.mass.inc` | Tax withholding computation (regpack rates), settlement netting, SWIFT instruction generation |
| Identity | *(embedded)* | KYC attestation VCs, sanctions screening (regpack checker) |
| Consent | `consent.api.mass.inc` | Multi-party corridor activation, governance-gated state transitions |

The `msez-mass-client` crate provides typed Rust HTTP clients for each primitive. All other crates are forbidden from making direct HTTP requests to Mass endpoints.

---

## Watcher economy

Watchers are bonded attestors that monitor corridor receipt chains for integrity. The watcher lifecycle uses the typestate pattern:

```
Bonding → Active → Slashed → Unbonding
```

Slashing conditions:
- `InvalidProof`: Watcher submitted a proof that fails verification
- `EquivocationDetected`: Watcher signed conflicting attestations
- `InactivityViolation`: Watcher failed to attest within the required window
- `PerjuryDetected`: Watcher attested to a false receipt chain state

Fork detection relies on watcher attestation quorum. A fork alarm is raised when watchers report conflicting head roots for the same corridor.

---

## Security model

### What cryptography guarantees

- **Integrity**: Content-addressed artifacts are verified by digest. Receipts chain via `prev_root` → `next_root`.
- **Authorship**: Ed25519 signatures verify the signer's key.
- **Causality**: MMR receipt chains enforce append-only ordering.

### What cryptography does not guarantee

- **Legal force** of a lawpack digest (requires governance)
- **Social legitimacy** of an authority registry (requires operational controls)
- **Availability** of artifacts (requires redundancy and monitoring)

### Mitigations

| Attack | Mitigation |
|--------|-----------|
| Receipt chain fork | Watcher attestation quorum + 3-level fork resolution |
| Trust anchor circularity | Authority registry chaining (treaty → national → zone), pinned by digest |
| Artifact withholding | CAS completeness checks, `--require-artifacts` strict mode |
| Timing side-channel | `subtle::ConstantTimeEq` for bearer token comparison |
| Key material in memory | `Zeroize` + `ZeroizeOnDrop` on `SigningKey` |
| Serialization divergence | `CanonicalBytes` as sole digest path, `preserve_order` guard |

See [Security Model](./architecture/SECURITY-MODEL.md) for the full threat model.

---

## Deployment architecture

### Docker Compose (development)

```
msez-api (Rust binary) ──> PostgreSQL 16
                        ──> Prometheus
                        ──> Grafana
```

### Kubernetes (production)

- 2 replicas with rolling updates
- Non-root security context, read-only filesystem
- Liveness, readiness, and startup probes
- Resource limits: 250m-1000m CPU, 256Mi-512Mi memory

### AWS (Terraform)

- EKS with auto-scaling node groups
- RDS PostgreSQL (Multi-AZ)
- ElastiCache Redis
- S3 for artifact storage
- KMS for key management
- ALB with TLS termination

---

## Design principles

| Principle | Enforcement |
|-----------|------------|
| **The type system does the work** | Typestate machines, sealed traits, identifier newtypes, exhaustive matching |
| **Single source of truth** | `CanonicalBytes` for digests, `ComplianceDomain` enum for all 20 domains, Mass APIs for primitive data |
| **Fail closed** | Unknown = `NonCompliant`. Missing attestations invalidate. System fails safe. |
| **No magic** | No floating point. No randomness in evaluation. No external state in condition evaluation. BTreeMap for deterministic iteration. |
| **Defense in depth** | Zeroize on key drop. Constant-time auth. Rate limit after auth. Schema validation at boundary. |
| **Verify, never trust** | All inputs validated. Signatures verified. Digests recomputed. Receipt chains verified. |

---

## Key external dependencies

| Crate | Purpose |
|-------|---------|
| `serde` / `serde_json` | Serialization (with `preserve_order` guard) |
| `axum` / `tokio` / `tower` | HTTP server and async runtime |
| `ed25519-dalek` | Ed25519 signing with zeroize support |
| `sha2` | SHA-256 digest computation |
| `chrono` | Timestamp handling |
| `clap` | CLI argument parsing |
| `tracing` | Structured logging and distributed tracing |
| `reqwest` | HTTP client (used only in `msez-mass-client`) |
| `sqlx` | PostgreSQL driver (async, compile-time query checking) |
| `utoipa` | OpenAPI spec generation |
| `proptest` | Property-based testing |
| `parking_lot` | Non-poisoning RwLock |
| `subtle` | Constant-time operations |
| `zeroize` | Memory scrubbing for key material |
| `uuid` | UUID v4 generation for identifiers |
