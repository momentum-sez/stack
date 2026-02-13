# CLAUDE.md — Momentum SEZ Stack: Production Rust Fortification

**Version**: 3.0 — February 2026
**Target**: Ship production-grade Rust orchestration infrastructure for the momentum-sez/stack.

---

## I. YOUR IDENTITY

You are the systems engineer building the governance, compliance, and orchestration layer for sovereign digital infrastructure. You report to Raeez Lorgat, Managing Partner of Momentum. Your code orchestrates operations across jurisdictions processing $1.7B+ in capital, integrating with live Mass API services that implement the five programmable primitives.

You write code like Carmack: zero ambiguity, every edge case handled. You architect like Torvalds: minimal API surface, the type system does the enforcement.

---

## II. THE CRITICAL ARCHITECTURE DISTINCTION

**Read this section three times. It is the single most important thing in this document.**

There are two systems. They are not the same thing. Confusing them produces wrong code.

### Mass APIs — The Five Primitives (PRODUCTION, DEPLOYED, LIVE)

Mass is the product. Mass implements the five programmable primitives as deployed API services:

| Primitive | Live API | Domain |
|-----------|----------|--------|
| **ENTITIES** | `organization-info.api.mass.inc` | Formation, lifecycle, dissolution, beneficial ownership |
| **OWNERSHIP** | `investment-info` (Heroku) | Cap tables, share classes, transfers, fundraising rounds |
| **FISCAL** | `treasury-info.api.mass.inc` | Accounts, payments, treasury operations |
| **IDENTITY** | *(embedded in other APIs)* | KYC/KYB, passportable credentials, DIDs |
| **CONSENT** | `consent.api.mass.inc` | Multi-party governance, audit trails |

Plus: `templating-engine` (Heroku) for document generation.

These APIs are **written in their own codebase** (not this repository). They are live. They handle real entities, real capital, real government integrations. They have their own persistence, their own deployment, their own schemas.

### momentum-sez/stack (`msez/`) — The Orchestration Layer

The SEZ Stack is **not** a reimplementation of the five primitives. The SEZ Stack is the layer that sits ABOVE Mass and provides:

1. **Cryptographic Foundation** — Canonicalization, Ed25519 signing, MMR, CAS, VCs, ZKP infrastructure. This is `msez-core`, `msez-crypto`, `msez-vc`, `msez-zkp`.

2. **Compliance Engine** — The Compliance Tensor V2 (20-domain evaluation), Compliance Manifold (Dijkstra path optimization), jurisdictional configuration. This is `msez-tensor`.

3. **Pack Trilogy** — Lawpacks, regpacks, licensepacks. Jurisdictional legal/regulatory/licensing configuration. This is `msez-pack`.

4. **Corridor System** — Cross-border corridor lifecycle, receipt chains, fork resolution, netting, SWIFT pacs.008, L1 anchoring. This is `msez-corridor`.

5. **State Machines** — Entity lifecycle, corridor lifecycle, migration saga (8 phases), watcher economy. Domain state machines that orchestrate transitions across Mass API calls. This is `msez-state`.

6. **Agentic Engine** — Autonomous policy execution responding to environmental triggers (sanctions updates, license expirations, compliance breaches). This is `msez-agentic`.

7. **Arbitration** — Dispute resolution lifecycle, evidence management, ruling enforcement via VC-triggered state transitions. This is `msez-arbitration`.

8. **Schema Validation** — JSON Schema validation for all artifacts. This is `msez-schema`.

9. **CLI** — Command-line interface for zone deployment, validation, signing, corridor management. This is `msez-cli`.

10. **Mass API Client** — Typed Rust HTTP client that calls into the live Mass APIs for primitive operations. **This does not exist yet. It is the critical missing piece.**

### What This Means For The Codebase

The current `msez-api/src/routes/{entities,ownership,fiscal,identity,consent}.rs` files are **architectural mistakes**. They reimplement primitive CRUD that belongs in the Mass APIs. They should be replaced with:

- **A typed Mass API client** (`msez-mass-client/`) that wraps the live Mass API endpoints with Rust types, error handling, and retry logic.
- **Orchestration endpoints** that compose Mass API calls with SEZ Stack compliance evaluation, corridor operations, and agentic triggers.
- **The endpoints that genuinely belong in the SEZ Stack**: corridors, smart assets, regulator console, compliance tensor queries, pack management, and zone administration.

### The Correct Data Flow

```
User/Zone Admin → SEZ Stack API → Compliance Tensor evaluation
                                 → Corridor state check
                                 → Mass API client → organization-info.api.mass.inc (entity formation)
                                                   → treasury-info.api.mass.inc (account creation)
                                                   → consent.api.mass.inc (governance approval)
                                 → VC issuance (Ed25519 signing)
                                 → Receipt chain append (MMR)
                                 → Agentic policy evaluation
                                 → Response
```

The SEZ Stack is the **orchestrator**. Mass is the **primitive execution layer**. The SEZ Stack never stores entity records directly — it stores compliance state, corridor state, VCs, receipts, and zone configuration. Entity data lives in Mass.

---

## III. AUTHORITY HIERARCHY

1. **This document** (`CLAUDE.md`).
2. **The architecture distinction in Section II.** If you are about to write code that stores entity records, cap table records, payment records, or identity records in the SEZ Stack database — STOP. Those belong in Mass. The SEZ Stack stores compliance state, corridor state, VCs, receipts, zone configuration, and audit trails.
3. **Specification** (`spec/` directory).
4. **Canonicalization invariant**: `msez-core::canonical::CanonicalBytes::new()` is the sole path to digest computation.
5. **Schema contracts** (`schemas/` — 116+ JSON Schema files).
6. **Existing passing tests**.
7. **Live Mass API behavior**.

---

## IV. REPOSITORY MAP

```
msez/crates/
├── msez-core/          # [2,200L] Canonicalization, ComplianceDomain (20 variants),
│                       #   identifiers, error hierarchy. THE cryptographic foundation.
├── msez-crypto/        # [3,500L] Ed25519 signing, MMR, CAS, SHA-256.
│                       #   BBS+ and Poseidon2 behind feature flags.
├── msez-vc/            # [2,100L] Verifiable Credentials (W3C data model, proofs, registry)
├── msez-state/         # [4,400L] Domain state machines — entity lifecycle, corridor lifecycle,
│                       #   migration saga, watcher economy. These orchestrate Mass API calls.
├── msez-tensor/        # [3,300L] Compliance Tensor V2 — 20-domain evaluation,
│                       #   manifold optimization, tensor commitment.
├── msez-zkp/           # [3,000L] ZKP — sealed ProofSystem trait, mock/CDB/Groth16/PLONK
├── msez-pack/          # [7,800L] Pack Trilogy — lawpacks, regpacks, licensepacks
├── msez-corridor/      # [3,200L] Cross-border corridors — receipts, fork resolution,
│                       #   netting, SWIFT pacs.008, L1 anchoring
├── msez-agentic/       # [3,600L] Autonomous policy engine — triggers, scheduling, audit
├── msez-arbitration/   # [5,200L] Dispute resolution — lifecycle, evidence, enforcement, escrow
├── msez-schema/        # [2,600L] JSON Schema validation
├── msez-api/           # [8,500L] ★ Axum HTTP server — NEEDS ARCHITECTURAL CORRECTION ★
│   └── src/routes/
│       ├── entities.rs     # ⚠️ WRONG: reimplements Mass organization-info. Should be client.
│       ├── ownership.rs    # ⚠️ WRONG: reimplements Mass investment-info. Should be client.
│       ├── fiscal.rs       # ⚠️ WRONG: reimplements Mass treasury-info. Should be client.
│       ├── identity.rs     # ⚠️ WRONG: reimplements Mass identity. Should be client.
│       ├── consent.rs      # ⚠️ WRONG: reimplements Mass consent-info. Should be client.
│       ├── corridors.rs    # ✓ CORRECT: genuinely SEZ Stack domain
│       ├── smart_assets.rs # ✓ CORRECT: genuinely SEZ Stack domain
│       └── regulator.rs    # ✓ CORRECT: genuinely SEZ Stack domain
├── msez-cli/           # [4,400L] CLI tool
└── msez-integration-tests/ # [17,600L] Cross-crate integration tests

# MISSING — CRITICAL:
# msez-mass-client/     # Typed Rust HTTP client for live Mass APIs. Does not exist yet.
```

**Python reference layer** (`tools/`) — DO NOT SHIP. Reference + test oracle only.

---

## V. WHAT THE SEZ STACK ACTUALLY OWNS

These are the concerns that genuinely belong in this codebase and its Rust API:

### Compliance Tensor & Manifold
Evaluate compliance state across 20 domains for entity/jurisdiction pairs. Compute optimal migration paths. Issue compliance attestation VCs. This is the core intelligence of the SEZ Stack — no other system does this.

### Corridor Lifecycle
Create, negotiate, activate, and terminate cross-border corridors. Manage receipt chains with MMR append-only proofs. Detect and resolve forks. Execute bilateral and multilateral netting. Generate SWIFT pacs.008 instructions. Each corridor connects two jurisdictions and enables trade between them.

### Pack Trilogy
Parse, validate, compose, and serve lawpacks (Akoma Ntoso legal corpus), regpacks (sanctions lists, regulatory calendars, guidance), and licensepacks (live license registry snapshots). These define the jurisdictional configuration that the compliance tensor evaluates against.

### Smart Asset Lifecycle
Genesis document creation, registry credential binding, jurisdictional binding, operational manifest, receipt chain management. Smart Assets are the programmable substrate — an entity IS a Smart Asset, an ownership position IS a Smart Asset. The SEZ Stack manages their lifecycle; Mass manages the underlying primitive data.

### Zone Configuration & Deployment
Zone YAML composition, module validation, profile generation, lockfile determinism, one-click deployment. The `msez-cli` and `msez-schema` crates handle this.

### Verifiable Credentials
Issue, verify, and manage W3C VCs for: KYC verification, AML screening, sanctions clearance, license status, corridor definition, corridor agreement, lawpack attestation, watcher bond, fork resolution. VCs are the SEZ Stack's audit trail — they attest to compliance state that the Mass APIs' primitive operations produce.

### Arbitration
Dispute filing, evidence management, panel assignment, ruling issuance, ruling enforcement via VC-triggered state transitions. This is entirely an SEZ Stack concern — Mass primitives don't handle disputes.

### Agentic Execution
Policy definition, trigger matching, autonomous action execution. When a sanctions list updates, the agentic engine evaluates all affected corridors and freezes non-compliant ones. When a license expires, the agentic engine suspends the entity. These policies operate ACROSS Mass primitives, which is why they live in the SEZ Stack.

### Regulator Console
Read-only query interface for regulatory authorities. Compliance monitoring, attestation oversight, SLA tracking. The regulator sees the compliance tensor state and VCs, not the raw Mass API data.

---

## VI. WHAT THE SEZ STACK DOES NOT OWN

These concerns belong in the Mass APIs. The SEZ Stack calls into Mass for these operations via the Mass API client.

- Entity formation, update, dissolution → `organization-info.api.mass.inc`
- Cap table management, share transfers → `investment-info`
- Account creation, payments, treasury → `treasury-info.api.mass.inc`
- KYC/KYB verification, identity records → Mass identity services
- Consent requests, governance approvals → `consent.api.mass.inc`
- Document generation → `templating-engine`

The SEZ Stack may **cache** or **index** data from Mass APIs for compliance evaluation purposes (e.g., caching entity jurisdiction_id to evaluate the compliance tensor without hitting Mass on every request). But it does not **own** or **persist** the authoritative copy of primitive data.

---

## VII. KNOWN DEFECTS — RANKED BY SEVERITY

### P0: Production Blockers

**P0-001: No Zeroize on cryptographic key material.**
`msez-crypto/src/ed25519.rs` — SigningKey does not implement Zeroize.
*Fix*: Add zeroize crate, enable ed25519-dalek zeroize feature, implement Drop.

**P0-002: Non-constant-time bearer token comparison.**
`msez-api/src/auth.rs:43` — `provided == expected.as_str()` timing side-channel.
*Fix*: Use `subtle::ConstantTimeEq`.

**P0-003: Seven `expect("store lock poisoned")` panics.**
`msez-api/src/state.rs:52,60,69,77,89,97,104`
*Fix*: Replace `std::sync::RwLock` with `parking_lot::RwLock`.

**P0-004: 14 `unimplemented!()` macros in production paths.**
*Fix*: Replace with proper error returns.

### P1: Must Fix Before Sovereign Deployment

**P1-001: Rate limiter before authentication.**
*Fix*: Swap middleware layer order.

**P1-002: Readiness probe is a no-op.**
*Fix*: Verify store/service health.

**P1-003: `serde_json` `preserve_order` feature guard.**
*Fix*: Three-layer defense (test, CI, build.rs).

**P1-004: `msez-api` routes reimplement Mass primitives.**
This is the big one. The five primitive route files need to be replaced with a Mass API client crate and orchestration endpoints. See Sprint 2 below.

### P2: Production Quality

**P2-001: No cross-language MMR parity test.**
**P2-002: `determined_at` in TensorCell uses raw string instead of Timestamp newtype.**
**P2-003: Entity type/status are unvalidated strings** (this applies to the Mass API, not the SEZ Stack — but the Mass API client should define typed enums for these values).
**P2-004: OpenAPI spec divergence** between hand-written and utoipa-generated.

---

## VIII. SPRINT EXECUTION ORDER

### Sprint 0: P0 Fixes (1-2 days)
Fix P0-001 through P0-004. These are cryptographic and runtime safety issues in crates that genuinely belong to the SEZ Stack (msez-crypto, msez-core, msez-api infrastructure).

### Sprint 1: P1 Fixes + Canonicalization Guard (3 days)
Fix P1-001 through P1-003. Middleware ordering, readiness probe, preserve_order defense.

### Sprint 2: Mass API Client + Architecture Correction (1-2 weeks)
This is the critical sprint. Create `msez-mass-client/` — a typed Rust HTTP client for the live Mass APIs. Then restructure `msez-api/` to:

1. **Remove** the five primitive route files that reimplement Mass CRUD.
2. **Add** the Mass API client as a dependency.
3. **Create orchestration endpoints** that compose Mass API calls with compliance tensor evaluation, corridor checks, and VC issuance.
4. **Retain** corridor, smart asset, regulator, and compliance tensor endpoints that genuinely belong in the SEZ Stack.

The Mass API client should:
- Define Rust types matching the Mass API request/response schemas (derived from the Swagger specs at the live endpoints).
- Handle authentication, retries, circuit breaking, and error mapping.
- Provide both sync and async interfaces.
- Cache frequently-accessed data (entity jurisdiction, filer status) with configurable TTL for compliance tensor evaluation without per-request Mass API round-trips.

### Sprint 3: Corridor + Tensor Production Hardening (1 week)
The corridors and compliance tensor are the SEZ Stack's core value. Harden these:
- Postgres persistence for corridor state, compliance tensor snapshots, and VC audit logs (the SEZ Stack DOES need its own persistence — just not for Mass primitive data).
- Cross-language parity tests for MMR.
- Adversarial tests for corridor fork resolution.
- Load testing for compliance tensor evaluation at 10K concurrent corridor operations.

### Sprint 4: PDI Integration (1-2 weeks)
Wire up the Pakistan Digital Authority deployment:
- NTN and CNIC validation through the Mass API client (identity primitive).
- Tax withholding orchestration: SEZ Stack calls Mass treasury-info for payment creation, calculates withholding using regpack rate data (SEZ Stack concern), creates tax event in Mass via treasury-info, and issues a withholding VC for audit trail.
- FBR IRIS reporting adapter (external system integration — Layer 6 in the architecture).
- Cross-border corridor activation for PAK↔KSA, PAK↔UAE, PAK↔CHN.

---

## IX. NAMING CONVENTIONS

**Momentum** — the fund and studio. Never "Momentum Protocol." Domain: `momentum.inc`.
**Mass** — the product. "Mass Protocol" only in deeply technical contexts. Domain: `mass.inc`.
**Five Primitives**: Entities, Ownership, Fiscal, Identity, Consent. These are **Mass** concepts.
**SEZ Stack**: The orchestration layer. Corridors, Compliance Tensor, Pack Trilogy, Smart Assets, Agentic Engine, Arbitration, Regulator Console, Watcher Economy. These are **SEZ Stack** concepts.

---

## X. CODE QUALITY STANDARDS

All standards from v2.0 remain: no `unwrap()` in library crates, `thiserror` for errors, newtype wrappers for identifiers, exhaustive enum matching, `CanonicalBytes` as sole digest path, property-based tests for crypto and compliance operations.

Additional standard: **Every function that calls a Mass API must go through the `msez-mass-client` crate.** Direct HTTP requests to Mass endpoints from any other crate are forbidden.

---

## XI. SUCCESS CRITERIA

The Rust workspace is production-ready when:

1. `cargo check/test/clippy --workspace` all pass clean.
2. Zero `unimplemented!()` or panicking `expect()` in production code.
3. `msez-mass-client` provides typed access to all five Mass API primitives.
4. `msez-api` serves corridors, smart assets, regulator console, compliance tensor, and orchestration endpoints — NOT reimplemented primitive CRUD.
5. Corridor operations persist to Postgres (SEZ Stack's own database for corridor state, tensor snapshots, VCs).
6. Cross-language parity tests pass for canonicalization, MMR, VC signing.
7. P1-003 preserve_order guard is active in tests and CI.
8. CI pipeline passes on every PR.

---

**End of CLAUDE.md**

Momentum · `momentum.inc`
Mass · `mass.inc`
Confidential · February 2026
