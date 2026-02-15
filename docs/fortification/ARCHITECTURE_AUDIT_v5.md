# Momentum SEZ Stack — Architecture Audit v5.0

**Date**: February 15, 2026  
**Scope**: Full repository audit of `momentum-sez/stack`, Mass API integration coherence, Python→Rust migration assessment, production hardening  
**Authority**: Supersedes CLAUDE.md v4.0 and all prior audit documents  
**Prepared for**: Raeez Lorgat, Managing Partner, Momentum

---

## 0. Executive Summary

The `momentum-sez/stack` repository contains two interleaved systems that serve fundamentally different purposes. **Mass** is Momentum's product — five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent) delivered as live Java/Spring Boot API services. The **SEZ Stack** is the open-source jurisdictional orchestration layer that sits above Mass, providing compliance evaluation, corridor management, credential issuance, and the regulatory context that transforms generic API calls into jurisdiction-aware operations.

This audit examines the full repository: 77,511 lines of Rust across 16 workspace crates, 45,290 lines of legacy Python across `tools/`, 116 JSON Schemas, 7 deployment profiles, OpenAPI specs, and deployment infrastructure. The audit was conducted against the live repository at `https://github.com/momentum-sez/stack` (commit history: 13 commits on `main`, version 0.4.44 GENESIS).

**Overall assessment**: The architecture is sound and the conceptual separation between Mass APIs and the SEZ Stack is correctly designed. The Rust codebase demonstrates genuine architectural discipline — newtype wrappers, a single `ComplianceDomain` enum with 20 variants, `parking_lot::RwLock` throughout, constant-time token comparison via `subtle`, and Ed25519 key zeroization on drop. However, the codebase carries significant technical debt from its AI-scaffolded origins: 2,201 `unwrap()` calls in non-test Rust code (392 in the HTTP server alone), a 45K-line Python prototype that remains the only implementation of several capabilities, and proxy routes that will need to evolve into genuine orchestration endpoints. The path from current state to sovereign-grade production is achievable but requires disciplined execution.

---

## 1. The Two Systems: Assessment of Separation Discipline

### 1.1 Mass APIs — The Five Programmable Primitives

Mass delivers five primitives as deployed services. This audit verified the live Swagger endpoints:

| # | Primitive | Live API Endpoint | Service |
|---|-----------|-------------------|---------|
| 1 | **Entities** | `organization-info.api.mass.inc` | Entity formation, lifecycle, dissolution, FBR registration, beneficial ownership, NTN binding |
| 2 | **Ownership** | `investment-info` (Heroku) | Cap tables, share classes, vesting, SAFE/convertible instruments, fundraising rounds |
| 3 | **Fiscal** | `treasury-info.api.mass.inc` | Accounts, payments, treasury ops, withholding tax, SBP Raast integration |
| 4 | **Identity** | Embedded across consent-info + NADRA integration | KYC/KYB, CNIC verification, NTN cross-reference, passportable credentials |
| 5 | **Consent** | `consent.api.mass.inc` | Multi-party governance approvals, audit trails, tax assessment sign-off |

Plus: `templating-engine` (Heroku) for document generation.

**Finding 1.1 (P1 — Architecture Drift in Identity Primitive):** The Identity primitive does not have its own dedicated API service. Identity is currently split across `consent-info` (which handles both consent workflows and identity verification) and `organization-info` (which handles entity-level identity like CNIC/NTN binding). The sales narrative of "five independent programmable primitives" is architecturally clean for four of the five. Identity requires a coherence decision: either extract a dedicated `identity-info.api.mass.inc` service, or formally document that Identity is a cross-cutting concern embedded across the other four. The former is recommended for the Pakistan GovOS deployment where NADRA integration demands a clear identity service boundary.

### 1.2 SEZ Stack — Jurisdictional Orchestration Layer

The Rust workspace (`msez/crates/`) contains 16 crates organized in a clean dependency tree:

```
msez-core (foundation — no internal deps)
├── msez-crypto (signing, MMR, CAS)
│   ├── msez-vc (verifiable credentials)
│   ├── msez-tensor (compliance tensor V2, manifold)
│   └── msez-zkp (zero-knowledge proof infrastructure)
├── msez-state (domain state machines)
│   ├── msez-corridor (corridor lifecycle, receipts)
│   └── msez-arbitration (dispute resolution)
├── msez-pack (lawpacks, regpacks, licensepacks)
├── msez-schema (JSON Schema validation)
├── msez-agentic (autonomous policy engine)
├── msez-compliance (compliance evaluation — composes tensor + pack)
├── msez-mass-client (typed HTTP client for Mass APIs)
└── msez-api (Axum HTTP server — composes everything)
    └── msez-cli (command-line interface)
```

**Finding 1.2 (PASS — Clean Dependency Graph):** The crate dependency graph has no cycles. `msez-core` has zero internal dependencies (only `serde`, `serde_json`, `thiserror`, `chrono`, `uuid`, `sha2`). Every other crate depends on `msez-core` and at most two or three peers. This is correct.

**Finding 1.3 (PASS — Mass Client Boundary):** `msez-mass-client` has zero dependencies on any `msez-*` crate. Its `Cargo.toml` lists only external dependencies: `reqwest`, `serde`, `serde_json`, `uuid`, `chrono`, `thiserror`, `tokio`, `tracing`, `url`. This is the correct boundary — the Mass client is a pure HTTP client that should never import SEZ Stack domain logic.

**Finding 1.4 (P2 — Mass Client Should Depend on msez-core):** While the clean boundary is good, `msez-mass-client` currently defines its own identifier types (entity IDs, etc.) rather than importing them from `msez-core`. This means the API layer (`msez-api`) must perform type conversions at the boundary. The recommendation is to add a single, narrow dependency on `msez-core` for identifier newtypes only (`EntityId`, `JurisdictionId`, etc.), which would eliminate a class of type-mismatch bugs at the proxy layer.

### 1.3 Boundary Violations and Duplication Assessment

**Finding 1.5 (P1 — Mass Proxy as Transitional Shim):** The file `msez-api/src/routes/mass_proxy.rs` (892 lines) correctly identifies itself as a transitional proxy layer. The handlers are passthrough proxies that delegate to `msez-mass-client`. They do NOT reimplant CRUD logic — this is correct. However, the file explicitly notes that compliance evaluation, corridor checks, and VC issuance are deferred to "Sprint 2C/2D." The end-state architecture requires these proxy routes to evolve into orchestration endpoints that compose Mass API calls with compliance tensor evaluation, corridor state updates, and credential issuance. This is the most critical architectural transformation remaining.

**Finding 1.6 (PASS — No Primitive Duplication in SEZ Stack):** The SEZ Stack does not duplicate Mass primitive CRUD operations. The `state.rs` file explicitly documents: "Entity, ownership, fiscal, identity, and consent data is NOT stored here. That data lives in the Mass APIs and is accessed via `msez-mass-client`." The in-memory stores in `state.rs` hold only SEZ-Stack-owned domain objects: corridors, smart assets, attestations, and agentic policy state.

---

## 2. Rust Codebase: Structural Integrity

### 2.1 Codebase Metrics

| Metric | Value | Assessment |
|--------|-------|------------|
| Total Rust LOC | 77,511 | Substantial codebase |
| Workspace crates | 16 | Well-factored |
| Largest file | `msez-pack/src/licensepack.rs` (2,265 lines) | Could benefit from submodule extraction |
| `unwrap()` in non-test code | 2,201 | **P0 — Must be resolved** |
| `expect()` in `msez-api` | 3 | P1 — Verify each is safe |
| `unimplemented!()` in production | 0 | PASS (was 14, now 0) |
| `todo!()` in production | 0 | PASS |
| `std::sync::RwLock` usage | 0 | PASS — all `parking_lot::RwLock` |
| `anyhow` in non-CLI code | 0 | PASS — confined to `msez-cli` only |
| JSON Schemas | 116 | Comprehensive |

### 2.2 The 2,201 `unwrap()` Problem

This remains the single highest-severity defect class in the codebase. Distribution by crate:

| Crate | `unwrap()` Count | Severity | Rationale |
|-------|-----------------|----------|-----------|
| `msez-cli` | 462 | P2 | CLI panics are annoying but not production-critical |
| **`msez-api`** | **392** | **P0** | Any unwrap in the HTTP server crashes the process on bad input |
| `msez-pack` | 311 | P1 | Pack parsing handles external data — panics are unacceptable |
| `msez-state` | 205 | P1 | State machine code must be robust to edge cases |
| **`msez-crypto`** | **157** | **P0** | Cryptographic code must never panic on malformed input |
| `msez-core` | 139 | P0 | Foundation layer — everything depends on this |
| `msez-arbitration` | 137 | P1 | Dispute handling cannot crash on unexpected input |
| `msez-corridor` | 116 | P1 | Corridor state management |
| `msez-vc` | 85 | P1 | Credential verification |
| `msez-agentic` | 64 | P2 | Policy engine |
| `msez-zkp` | 61 | P2 | ZKP infrastructure (mostly stubs) |
| `msez-tensor` | 42 | P1 | Compliance tensor evaluation |
| `msez-schema` | 17 | P2 | Schema validation |
| `msez-compliance` | 11 | P2 | Thin orchestration layer |
| `msez-mass-client` | 2 | P2 | Minimal — near-clean |

**Remediation strategy**: Each `unwrap()` must be categorized:

1. **Safe unwraps** (after a guard that guarantees `Some`/`Ok`): Annotate with `// SAFETY: checked on line N`.
2. **Lazy unwraps** (should propagate the error): Replace with `?` or `.map_err()`.
3. **Static initialization unwraps** (e.g., regex compilation of a literal): Replace with `expect("static regex — cannot fail")`.
4. **Bugs** (will panic on production data): Fix with proper error handling.

**Priority order**: `msez-api` (392) → `msez-crypto` (157) → `msez-core` (139) → `msez-pack` (311) → everything else.

### 2.3 Security Audit Results

**Finding 2.3.1 (PASS — P0-001 Resolved: Key Zeroization).** `msez-crypto/src/ed25519.rs` implements `Zeroize` for `SigningKey` (line 129) and `Drop` calls `self.zeroize()` (line 191-193). Ed25519-dalek's own `ZeroizeOnDrop` provides defense-in-depth. A test (`signing_key_drops_without_panic`) confirms the drop path.

**Finding 2.3.2 (PASS — P0-002 Resolved: Constant-Time Token Comparison).** `msez-api/src/auth.rs` implements `constant_time_token_eq()` using `subtle::ConstantTimeEq` (line 164-173). When lengths differ, a dummy comparison is performed to prevent timing leakage. The `subtle` crate is in workspace dependencies.

**Finding 2.3.3 (PASS — P0-003 Resolved: No Poisonable Locks).** All `RwLock` usage is `parking_lot::RwLock`, which does not poison on writer panic. The `Store<T>` wrapper in `state.rs` (line 39) correctly uses `Arc<RwLock<HashMap<Uuid, T>>>` with `parking_lot`. Zero instances of `std::sync::RwLock` in the codebase.

**Finding 2.3.4 (PASS — P0-004 Resolved: No `unimplemented!()` in Production).** Zero instances of `unimplemented!()` in non-test, non-comment code. Comments in `bbs.rs` and `poseidon.rs` reference `unimplemented!()` to describe the feature-gate strategy, but no actual calls exist.

**Finding 2.3.5 (PASS — P1-001 Resolved: Auth Before Rate Limiting).** The middleware stack in `msez-api` applies authentication before rate limiting, preventing unauthenticated requests from consuming rate limit budget.

**Finding 2.3.6 (P2 — Auth Token as Debug Redacted).** `AuthConfig` implements custom `Debug` that redacts the token value (line 149-155). This is correct. However, the token is stored as `Option<String>` in memory. Consider wrapping in a `Secret<String>` newtype that implements `Zeroize` for defense-in-depth against memory dumps.

### 2.4 Error Hierarchy Assessment

The error hierarchy is well-structured:

- `MsezError` (top-level, in `msez-core`) has 9 variants covering canonicalization, state transition, validation, schema, cryptographic, integrity, security, I/O, and JSON errors.
- `CanonicalizationError` (2 variants): Float rejection and serialization failure.
- `StateTransitionError`: Carries `from`, `to`, and `reason` context.
- `ValidationError`: Domain primitive validation with structured detail.
- `CryptoError` (in `msez-crypto`): Covers hex decoding, signature, verification, and key generation failures.
- `MassApiError` (in `msez-mass-client`): Covers HTTP, config, and response parsing failures.
- `AppError` (in `msez-api`): HTTP-layer errors with proper status code mapping.

**Finding 2.4.1 (PASS):** No crate exposes `anyhow::Error` in its public API. `anyhow` is confined to `msez-cli` (the CLI, where ergonomic error reporting outweighs structured error handling).

**Finding 2.4.2 (P2 — `MsezError::NotImplemented` Variant):** The `NotImplemented` variant exists but is not used in production code. It should either be removed (if truly unused) or gated behind `#[cfg(test)]`.

### 2.5 ComplianceDomain Enum — Single Source of Truth

The `ComplianceDomain` enum in `msez-core/src/domain.rs` defines exactly 20 variants matching the composition specification: Aml, Kyc, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy, Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets, Employment, Immigration, Ip, ConsumerProtection, Arbitration, Trade.

**Finding 2.5.1 (PASS — Domain Unification).** The Python codebase historically had two independent domain enums (8 in `phoenix/tensor.py`, 20 in `msez/composition.py`). The Rust codebase has a single enum. The doc comment on `domain.rs` explicitly references this finding. Compiler-enforced exhaustive `match` prevents silent domain omission.

### 2.6 Canonical Digest Pipeline

`CanonicalBytes::new()` in `msez-core/src/canonical.rs` is declared as the sole path to digest computation. The implementation applies JCS-compatible canonicalization with Momentum-specific rules (float rejection, datetime normalization, key coercion).

**Finding 2.6.1 (P1 — Verify No Bypass Paths):** The audit must verify that no SHA-256 computation in any other crate bypasses `CanonicalBytes`. A grep for `sha2::Sha256` or `Sha256::digest` outside of `canonical.rs` and `digest.rs` would reveal bypass paths. This verification is recommended as part of the CI pipeline.

---

## 3. Python Codebase: Deprecation Assessment

### 3.1 Current State

| Component | Lines | Purpose | Rust Equivalent |
|-----------|-------|---------|-----------------|
| `tools/msez.py` | 15,476 | CLI: validate, build, corridor ops, signing, artifact verification | `msez-cli` (partial) |
| `tools/phoenix/` | 16,838 | Tensor V2, manifold, ZKP, migration, watcher, VM, bridge, security | `msez-tensor`, `msez-state`, `msez-zkp`, etc. (partial) |
| `tools/mass_primitives.py` | 1,771 | Mass API interaction types | `msez-mass-client` (ported) |
| `tools/agentic.py` | 1,686 | Agentic policy engine | `msez-agentic` (ported) |
| `tools/arbitration.py` | 1,217 | Arbitration system | `msez-arbitration` (ported) |
| `tools/licensepack.py` | 1,197 | Licensepack system | `msez-pack/src/licensepack.rs` (ported) |
| `tools/lawpack.py` | 30K (with subdir) | Lawpack parsing and composition | `msez-pack/src/lawpack.rs` (ported) |
| `tools/regpack.py` | 23K (with subdir) | RegPack system | `msez-pack/src/regpack.rs` (ported) |
| `tools/smart_asset.py` | 30K | Smart asset lifecycle | `msez-state` (partial) |
| `tools/vc.py` | 14K | Verifiable credential issuance | `msez-vc` (ported) |
| `tools/netting.py` | 22K | Settlement netting | `msez-corridor` (partial) |
| `tools/mmr.py` | 11K | Merkle Mountain Range | `msez-crypto/src/mmr.rs` (ported) |

**Total Python**: 45,290 lines.

### 3.2 Python-Only Capabilities (Not Yet in Rust)

**Finding 3.2.1 (P1 — `msez.py` CLI commands without Rust equivalents):** The Python CLI (`tools/msez.py` at 15,476 lines) implements several commands that do not yet have Rust equivalents in `msez-cli`:

1. `msez validate --zone` — Full zone deployment validation with profile composition
2. `msez build --zone` — Reproducible zone bundle construction
3. `msez corridor state fork-inspect` — Receipt-level fork forensics
4. `msez artifact graph verify --bundle` — Witness bundle verification with offline CAS
5. `msez artifact bundle attest` / `msez artifact bundle verify` — Bundle provenance attestation
6. Advanced transitive artifact completeness (`--transitive-require-artifacts`)

These must be ported to `msez-cli` before the Python tools can be fully deprecated.

**Finding 3.2.2 (P1 — Phoenix Module Suite as Reference Implementation):** The `tools/phoenix/` directory (16,838 lines) serves as the reference implementation for the Compliance Tensor V2, Compliance Manifold, Smart Asset Virtual Machine, Watcher Economy, Migration Protocol, Corridor Bridge Protocol, L1 Anchoring, and Security Layer. While the Rust crates implement the core data structures and algorithms, the Python modules include specific business logic (e.g., the exact Dijkstra-with-compliance-weights algorithm in `manifold.py`, the saga compensation logic in `migration.py`) that should be validated against cross-language parity tests.

**Finding 3.2.3 (P2 — `tools/msez/composition.py`):** The Multi-Jurisdiction Composition Engine (the `compose_zone` factory function documented in the technical specification) exists only in Python. The Rust codebase has no equivalent. This is a key sales feature ("Deploy the civic code of New York with the corporate law of Delaware") and must be ported to Rust.

### 3.3 Deprecation Path

The recommended deprecation path:

1. **Phase 1 (Immediate):** Establish cross-language parity tests for canonicalization, MMR roots, VC signing, and compliance tensor evaluation. These tests invoke both the Python reference and the Rust implementation and assert identical outputs.

2. **Phase 2 (4 weeks):** Port the remaining `msez-cli` commands from Python to Rust. Focus on zone validation, zone build, and artifact verification.

3. **Phase 3 (6 weeks):** Port the composition engine to Rust (likely in `msez-pack` or a new `msez-composition` crate).

4. **Phase 4 (8 weeks):** Move Python `tools/` to `tools/_deprecated/` and ensure the test suite passes without Python in the path. Python code remains in-repo as a test oracle but is never invoked by production Rust code or deployment scripts.

---

## 4. Mass API Integration Coherence

### 4.1 Five Primitives Sales Line ↔ Architecture Mapping

The sales pitch: "Mass gives you five programmable primitives — Entities, Ownership, Fiscal, Identity, Consent."

Current architectural mapping:

| Primitive | Mass API Service | `msez-mass-client` Sub-Client | `mass_proxy` Route Prefix | Assessment |
|-----------|-----------------|-------------------------------|--------------------------|------------|
| Entities | `organization-info.api.mass.inc` | `entities::EntityClient` | `/v1/entities` | **Clean** |
| Ownership | `investment-info` (Heroku) | `ownership::OwnershipClient` | `/v1/ownership` | **Clean** |
| Fiscal | `treasury-info.api.mass.inc` | `fiscal::FiscalClient` | `/v1/fiscal` | **Clean** |
| Identity | Split across consent-info + org-info | `identity::IdentityClient` | `/v1/identity` | **Drift** |
| Consent | `consent.api.mass.inc` | `consent::ConsentClient` | `/v1/consent` | **Clean** |

**Finding 4.1.1 (P1 — Identity Primitive Coherence):** Four of the five primitives have a clean 1:1 mapping between the sales narrative, the Mass API service, the Rust client sub-module, and the proxy route prefix. Identity is the exception — it is architecturally split across `consent-info` (which the `IdentityClient` currently points to) and `organization-info` (which handles CNIC/NTN binding as part of entity formation). For the Pakistan GovOS deployment, this split is particularly visible because NADRA integration (national identity) must coordinate with FBR IRIS (tax identity) and SECP (corporate identity).

**Recommendation:** Create a dedicated `identity-info.api.mass.inc` service that owns all identity verification, KYC/KYB, and credential issuance. In the interim, document the split cleanly in the `msez-mass-client` so that the `IdentityClient` acts as an aggregation facade over the underlying services.

### 4.2 Mass Proxy → Orchestration Evolution

The current `mass_proxy.rs` correctly implements thin passthrough. The target architecture requires each proxy endpoint to evolve into an orchestration endpoint that composes Mass API calls with SEZ Stack domain operations. The Pakistan GovOS architecture diagram illustrates the target flow:

```
Entity Formation Request
  → msez-tensor: evaluate 20-domain compliance for entity + PAK jurisdiction
  → msez-pack: check lawpack (Income Tax Ordinance 2001, Sales Tax Act 1990)
  → msez-pack: check regpack (current SBP rates, FATF sanctions, filing calendars)
  → msez-pack: check licensepack (SECP registration status)
  → msez-mass-client → organization-info.api.mass.inc (create entity, bind NTN)
  → msez-mass-client → treasury-info.api.mass.inc (create PKR account, configure withholding)
  → msez-mass-client → consent.api.mass.inc (tax assessment sign-off workflow)
  → msez-vc: issue formation VC, compliance attestation VC
  → msez-corridor: update corridor state for PAK↔UAE trade corridor
  → msez-agentic: register entity for automatic tax event generation
```

**Finding 4.2.1 (P1 — Orchestration Endpoints Are the Critical Path):** The proxy routes are explicitly marked as Sprint 2C/2D work. This is the most architecturally significant remaining work. Each of the five primitive endpoints needs to become a multi-step orchestration that weaves Mass API calls with compliance evaluation, corridor state management, and credential issuance.

### 4.3 Pakistan GovOS: Validation Against Architecture Diagram

The uploaded architecture diagram (`mass_pakistan_architecture_v4__1_.html`) describes a four-layer deployment:

1. **S1 — Experience Layer**: GovOS Console, Tax & Revenue Dashboard, Digital Free Zone, Citizen Tax & Services, Regulator Console
2. **S2 — Platform Engine**: Five Programmable Primitives + Event & Task Engine, Cryptographic Attestation, Compliance Tensor, App Marketplace
3. **S3 — Jurisdictional Configuration**: Pack Trilogy (Lawpacks, Regpacks, Licensepacks, Arbitration Corpus)
4. **S4 — National System Integration**: FBR IRIS, SBP Raast, NADRA, SECP, SIFC, AGPR

Plus: Cross-Border Trade Corridors (PAK↔KSA $5.4B, PAK↔UAE $10.1B LIVE, PAK↔CHN $23.1B) and External APIs (Northern Trust, Correspondent Banks, Sanctions Feeds).

**Finding 4.3.1 (PASS — Architecture Layers Map to Codebase):** S1 maps to `msez-api` routes. S2 maps to `msez-mass-client` (primitives) + `msez-agentic` (event engine) + `msez-crypto` (attestation) + `msez-tensor` (compliance tensor). S3 maps to `msez-pack` (pack trilogy). S4 maps to `msez-mass-client` (Mass APIs that integrate with Pakistani national systems). The corridor boxes map to `msez-corridor`.

**Finding 4.3.2 (P1 — Sovereign AI Module Has No Rust Equivalent):** The architecture diagram includes a "Sovereign AI" column (Foundation Model, Tax Intelligence, Operational Intelligence, Regulatory Awareness, Forensic & Audit, Data Sovereignty) that has no corresponding Rust crate. This may be intentional (AI services deployed as separate microservices) but should be explicitly documented.

**Finding 4.3.3 (P1 — Tax Collection Pipeline Not Implemented):** The diagram's highlighted "TAX COLLECTION PIPELINE" ("Every economic activity on Mass generates a tax event → automatic withholding at source → real-time reporting to FBR IRIS → AI-powered gap analysis closes evasion") is the signature Pakistan deployment feature. The `msez-agentic` crate provides the trigger/policy framework, but the specific tax event generation, withholding calculation, and FBR IRIS reporting logic is not yet implemented in Rust. This is Pakistan-specific business logic that belongs in the SEZ Stack's jurisdictional configuration layer.

---

## 5. Crate-by-Crate Fortification Assessment

### 5.1 msez-core (Foundation — 139 unwraps)

**Assessment: Solid foundation, needs unwrap cleanup.**

Positive findings: `CanonicalBytes` as sole digest path; `ComplianceDomain` with 20 variants matching spec; newtype wrappers for all identifiers (`EntityId`, `CorridorId`, `JurisdictionId`, `Did`, `WatcherId`, `MigrationId`); proper `Display`/`FromStr` implementations; `Timestamp` wrapper over `chrono::DateTime<Utc>`; structured `MsezError` hierarchy with `thiserror`.

Issues: 139 `unwrap()` calls in the foundation layer that every other crate depends on. These must be categorized and eliminated.

### 5.2 msez-crypto (Cryptography — 157 unwraps)

**Assessment: Security properties verified, unwraps are critical risk.**

Positive findings: Ed25519 key zeroization on drop (confirmed); `subtle::ConstantTimeEq` for security-sensitive comparisons; `CanonicalBytes` enforcement on signing (type system prevents signing raw bytes); MMR implementation at 1,292 lines with proper verification.

Issues: 157 `unwrap()` calls in cryptographic code. A malformed input must produce an error, never a panic. BBS+ and Poseidon2 modules are documented stubs behind feature gates (correct approach — documented honestly rather than hidden).

### 5.3 msez-vc (Verifiable Credentials — 85 unwraps)

**Assessment: W3C VC data model compliance, credential module at 821 lines.**

The credential issuance and verification pipeline is functional. BBS+ selective disclosure is feature-gated (honest about implementation status). Needs unwrap cleanup in verification paths where malformed credentials could cause panics.

### 5.4 msez-tensor (Compliance Tensor V2 — 42 unwraps)

**Assessment: Lowest unwrap count of domain crates, mathematically clean.**

The tensor implementation (838 lines) and manifold (1,210 lines) are the mathematical core. Lattice operations (meet/join) follow the specified ordering: `NON_COMPLIANT < EXPIRED < UNKNOWN < PENDING < EXEMPT < COMPLIANT`. Cross-border predicate evaluation uses pessimistic meet. Manifold Dijkstra path optimization is implemented.

### 5.5 msez-pack (Pack Trilogy — 311 unwraps)

**Assessment: Comprehensive but needs significant hardening.**

All three pack types are implemented: lawpack (1,503 lines), regpack (1,518 lines), licensepack (2,265 lines), plus validation (1,311 lines). The licensepack is the largest single file and would benefit from submodule extraction. Pack parsing handles external data formats (Akoma Ntoso XML, sanctions lists, license registries) where malformed input is expected — the 311 `unwrap()` calls represent genuine risk.

### 5.6 msez-corridor (Corridor Lifecycle — 116 unwraps)

**Assessment: Receipt chain architecture is sound.**

Corridor state management (1,095 lines) implements the receipt chain with SHA-256 linking, MMR checkpoints, and fork detection. The corridor lifecycle state machine (DRAFT→PENDING→ACTIVE, HALTED, SUSPENDED, TERMINATED) uses `msez-state` for evidence-gated transitions.

### 5.7 msez-api (HTTP Server — 392 unwraps)

**Assessment: P0 severity. The HTTP server cannot have any unwrap in request-handling paths.**

The Axum server (991 lines in bootstrap, 892 in mass_proxy, 1,617 in corridors, 1,300 in regulator, 1,139 in settlement, 831 in agentic) uses `parking_lot::RwLock`, constant-time auth, and structured error responses. However, 392 `unwrap()` calls mean that a malformed request can crash the entire process. Every single one must be eliminated from request-handling paths.

### 5.8 msez-mass-client (Mass API Client — 2 unwraps)

**Assessment: Near-clean. The best-maintained crate.**

Only 2 `unwrap()` calls in production code. Clean sub-client architecture mapping 1:1 to the five primitives. Proper error handling with `MassApiError`. No dependencies on any `msez-*` crate (correct boundary). Uses `reqwest` with `rustls-tls` (not openssl — correct).

### 5.9 msez-agentic (Policy Engine — 64 unwraps)

**Assessment: Trigger taxonomy and policy evaluation are implemented.**

Policy engine (1,275 lines) and scheduler (988 lines) implement the 20-trigger-type taxonomy across 5 domains. Standard policy library includes sanctions-auto-freeze, license-expiration-warning, and auto-ruling-enforcement.

### 5.10 msez-arbitration (Dispute Resolution — 137 unwraps)

**Assessment: Comprehensive at 4,827 lines total.**

Dispute lifecycle (1,535 lines), evidence packages (1,029 lines), enforcement (1,252 lines), and escrow (1,011 lines) are the most feature-complete domain crate. Ruling enforcement via VC-triggered state transitions is implemented.

### 5.11 msez-schema (JSON Schema Validation — 17 unwraps)

**Assessment: Clean. 116 schemas with validation infrastructure.**

The `validate.rs` (1,240 lines) provides schema loading and validation. All 116 schemas in `schemas/` are referenced. The `codegen.rs` provides type generation from schemas.

### 5.12 msez-zkp (Zero-Knowledge Proofs — 61 unwraps)

**Assessment: Correctly documented as stubs/mocks.**

The ZKP crate implements the sealed `ProofSystem` trait with mock backends. Circuit modules (compliance, identity, migration, settlement) define the constraint structures documented in the specification. The Canonical Digest Bridge (CDB) computation is implemented. This is honest scaffolding — the full proving circuits await Plonky3 integration.

---

## 6. Production Hardening: Priority Findings

### 6.1 P0 Findings (Must Fix Before Sovereign Deployment)

| ID | Finding | Location | Remediation |
|----|---------|----------|-------------|
| P0-005 | 392 `unwrap()` in HTTP server | `msez-api/src/**` | Replace every `unwrap()` in request-handling paths with `?` or typed error |
| P0-006 | 157 `unwrap()` in crypto code | `msez-crypto/src/**` | No cryptographic function may panic on malformed input |
| P0-007 | 139 `unwrap()` in foundation layer | `msez-core/src/**` | Foundation panics cascade to every dependent crate |
| P0-008 | No integration tests against live Mass APIs | `msez-mass-client/tests/` | Add contract tests that validate against actual Swagger specs |

### 6.2 P1 Findings (Must Fix Before Production Traffic)

| ID | Finding | Location | Remediation |
|----|---------|----------|-------------|
| P1-004 | Mass proxy routes are passthrough, not orchestration | `msez-api/src/routes/mass_proxy.rs` | Evolve into multi-step orchestration endpoints |
| P1-005 | Identity primitive split across services | `msez-mass-client/src/identity.rs` | Either extract dedicated service or document aggregation facade |
| P1-006 | Composition engine exists only in Python | `tools/msez/composition.py` | Port to Rust crate |
| P1-007 | Several CLI commands Python-only | `tools/msez.py` | Port remaining commands to `msez-cli` |
| P1-008 | No database persistence | `msez-api/src/state.rs` | In-memory stores must migrate to Postgres for corridor state, tensor snapshots, VC audit log, agentic policy state |
| P1-009 | Tax collection pipeline not implemented | N/A | Pakistan-specific business logic for tax event generation, withholding, FBR IRIS reporting |
| P1-010 | `CanonicalBytes` bypass verification needed | All crates | CI check: ensure no SHA-256 computation bypasses `CanonicalBytes::new()` |

### 6.3 P2 Findings (Should Fix for Code Quality)

| ID | Finding | Location | Remediation |
|----|---------|----------|-------------|
| P2-001 | `MsezError::NotImplemented` variant unused | `msez-core/src/error.rs` | Remove or gate behind `#[cfg(test)]` |
| P2-002 | `msez-mass-client` does not share identifier types with `msez-core` | `msez-mass-client/Cargo.toml` | Add narrow dependency on `msez-core` for newtypes |
| P2-003 | `licensepack.rs` at 2,265 lines | `msez-pack/src/licensepack.rs` | Extract into submodules |
| P2-004 | Auth token stored as plain `Option<String>` | `msez-api/src/auth.rs` | Wrap in `Secret<String>` with `Zeroize` |
| P2-005 | 45K lines of Python still in `tools/` | `tools/**` | Follow deprecation path (Section 3.3) |
| P2-006 | No cross-language parity tests | N/A | Add tests that verify Rust and Python produce identical outputs for canonicalization, MMR, VC signing |

---

## 7. Deployment Infrastructure Assessment

### 7.1 Docker Compose (12 services)

The deployment infrastructure includes `deploy/docker/docker-compose.yaml` (373+ lines) orchestrating 12 services (zone-authority, entity-registry, license-registry, corridor-node, watcher, identity-service, settlement-service, compliance-service, regulator-console, postgres, redis, prometheus). Three Dockerfiles provide container images.

**Finding 7.1.1 (P2):** The Docker Compose setup should be verified end-to-end with health checks. The 12-service architecture maps correctly to the crate topology.

### 7.2 AWS Terraform (1,250+ lines)

Core infrastructure (`main.tf`, 545 lines) provisions VPC, EKS, RDS, ElastiCache, S3, KMS, and CloudWatch. Kubernetes resources (`kubernetes.tf`, 705 lines) deploy all 12 services with health checks and resource limits.

### 7.3 One-Click Deployment

The `deploy-zone.sh` script (255 lines) enables single-command zone deployment: `./deploy-zone.sh digital-financial-center my-zone ae-dubai-difc`. This is the signature capability described in the specification.

---

## 8. Schema and Spec Coherence

### 8.1 Schema Count and Coverage

116 JSON Schemas in `schemas/` covering: agentic (6), arbitration (8), artifact (2), attestation (1), circuit (1), corridor (12), credential (2), formation (1), identity (3), lawpack (3), license/licensepack (5), migration (5), module (4), profile (3), regpack (4), receipt chain (5), settlement (6), smart asset (6), tensor (4), trade (5), watcher (6), zone (4), and more.

**Finding 8.1.1 (P2 — Schema ↔ Rust Type Alignment):** The `msez-schema` crate provides validation infrastructure, but a systematic verification that each schema has a corresponding Rust type that can roundtrip serialize/deserialize has not been performed. This should be a CI gate.

### 8.2 OpenAPI Specs

Four OpenAPI specs in `apis/`:
- `corridor-state.openapi.yaml` (corridor state management)
- `mass-node.openapi.yaml` (Mass API reference)
- `regulator-console.openapi.yaml` (regulator dashboard)
- `smart-assets.openapi.yaml` (smart asset lifecycle)

**Finding 8.2.1 (P1 — Mass API Swagger Alignment):** The `mass-node.openapi.yaml` in this repo is a reference spec. It must be validated against the actual Swagger specs served by the live Mass API endpoints. Contract drift between the reference spec and the live APIs is a source of integration bugs.

---

## 9. Naming Convention Enforcement

| Term | Correct Usage | Never | Status |
|------|---------------|-------|--------|
| **Momentum** | The fund and studio | "Momentum Protocol" | ✅ Enforced in codebase |
| **Mass** | The product (five primitives) | | ✅ Consistent |
| **Mass Protocol** | Only in deeply technical contexts (L1, ZKP) | In sales materials or casual usage | ✅ Correct in code comments |
| **SEZ Stack** | This open-source codebase | "Momentum Protocol", "MSEZ Protocol" | ✅ Consistent |
| **momentum.inc** | Momentum's domain | momentum.xyz, momentum.io | ✅ Verified in all config files |
| **mass.inc** | Mass's domain | | ✅ Verified in API URLs |

---

## 10. Recommended Execution Sequence

### Sprint 1 (Weeks 1–2): Security and Correctness

1. Eliminate all `unwrap()` from `msez-api` request-handling paths (P0-005).
2. Eliminate all `unwrap()` from `msez-crypto` (P0-006).
3. Eliminate all `unwrap()` from `msez-core` (P0-007).
4. Add CI gate: `cargo clippy --workspace -- -D warnings`.
5. Add CI gate: verify no `unwrap()` in `msez-api/src/routes/**`.

### Sprint 2 (Weeks 3–4): Mass API Contract Alignment

1. Fetch live Swagger specs from all five Mass API endpoints.
2. Generate contract tests that validate `msez-mass-client` request/response types against live specs.
3. Add cross-language parity tests (Rust ↔ Python) for canonicalization, MMR, and VC signing.
4. Begin evolving `mass_proxy` entity creation route into an orchestration endpoint (compliance evaluation + Mass API call + VC issuance).

### Sprint 3 (Weeks 5–6): Python Deprecation Phase 1

1. Port remaining `msez-cli` commands from Python to Rust.
2. Port composition engine to Rust.
3. Move `tools/` to deprecation staging.

### Sprint 4 (Weeks 7–8): Persistence and Pakistan

1. Add Postgres persistence for corridor state, tensor snapshots, VC audit log, and agentic policy state.
2. Implement Pakistan tax collection pipeline (tax event generation, withholding calculation, FBR IRIS reporting).
3. Implement NADRA identity integration in the Identity primitive.

### Sprint 5 (Weeks 9–10): Production Hardening

1. Full Docker Compose orchestration verification.
2. Health check endpoints that verify all dependencies.
3. Structured logging (tracing) on every API request.
4. Prometheus metrics for request latency, error rates, Mass API call latency, compliance tensor evaluation time.

---

## 11. Success Criteria

The architecture audit is resolved when:

1. Zero `unwrap()` in `msez-api` request-handling paths.
2. Zero `unwrap()` in `msez-crypto` non-test code.
3. `cargo clippy --workspace -- -D warnings` passes clean.
4. `cargo test --workspace` passes all tests.
5. `msez-mass-client` contract tests pass against live Mass API Swagger specs.
6. Cross-language parity tests pass for canonicalization, MMR, and VC signing.
7. The composition engine exists in Rust (not only Python).
8. Each `mass_proxy` route is an orchestration endpoint (compliance + Mass API + VC issuance).
9. Postgres persistence for all SEZ Stack domain data (corridor state, tensor snapshots, VC audit log).
10. The Pakistan tax collection pipeline is operational (tax events → withholding → FBR IRIS).

---

## Appendix A: Crate Dependency Graph (Verified)

```
msez-core
├── msez-crypto → [msez-core]
│   ├── msez-vc → [msez-core, msez-crypto, msez-schema]
│   ├── msez-tensor → [msez-core, msez-crypto]
│   └── msez-zkp → [msez-core, msez-crypto]
├── msez-state → [msez-core]
│   ├── msez-corridor → [msez-core, msez-state, msez-crypto]
│   └── msez-arbitration → [msez-core, msez-state]
├── msez-pack → [msez-core]
├── msez-schema → [msez-core]
├── msez-agentic → [msez-core]
├── msez-compliance → [msez-core, msez-tensor, msez-pack]
├── msez-mass-client → [] (no internal deps — correct boundary)
├── msez-api → [msez-core, msez-crypto, msez-vc, msez-tensor,
│               msez-corridor, msez-state, msez-mass-client,
│               msez-agentic, msez-pack, msez-compliance]
└── msez-cli → [msez-core, msez-crypto, msez-pack, msez-schema, msez-state]
```

No cycles. No unnecessary edges. `msez-mass-client` correctly isolated.

## Appendix B: File Sizes (Top 20 Rust Files)

| File | Lines | Notes |
|------|-------|-------|
| `msez-pack/src/licensepack.rs` | 2,265 | Consider submodule extraction |
| `msez-api/src/routes/corridors.rs` | 1,617 | Complex but domain-appropriate |
| `msez-arbitration/src/dispute.rs` | 1,535 | Comprehensive dispute lifecycle |
| `msez-pack/src/regpack.rs` | 1,518 | RegPack system |
| `msez-pack/src/lawpack.rs` | 1,503 | Lawpack system |
| `msez-pack/src/validation.rs` | 1,311 | Pack validation |
| `msez-api/src/routes/regulator.rs` | 1,300 | Regulator dashboard |
| `msez-crypto/src/mmr.rs` | 1,292 | Merkle Mountain Range |
| `msez-agentic/src/policy.rs` | 1,275 | Policy engine |
| `msez-arbitration/src/enforcement.rs` | 1,252 | Ruling enforcement |
| `msez-schema/src/validate.rs` | 1,240 | Schema validation |
| `msez-tensor/src/manifold.rs` | 1,210 | Compliance manifold |
| `msez-api/src/routes/settlement.rs` | 1,139 | Settlement routes |
| `msez-state/src/corridor.rs` | 1,095 | Corridor state machine |
| `msez-cli/src/lock.rs` | 1,050 | Lockfile computation |
| `msez-arbitration/src/evidence.rs` | 1,029 | Evidence packages |
| `msez-arbitration/src/escrow.rs` | 1,011 | Escrow management |
| `msez-api/src/bootstrap.rs` | 991 | Server bootstrap |
| `msez-agentic/src/scheduler.rs` | 988 | Policy scheduler |
| `msez-api/tests/integration_tests.rs` | 916 | Integration test suite |

## Appendix C: Previously Reported P0 Defects — Status

| ID | Description | Status | Evidence |
|----|-------------|--------|----------|
| P0-001 | No Zeroize on signing key material | **RESOLVED** | `Zeroize` impl at line 129, `Drop` at line 191 of `ed25519.rs` |
| P0-002 | Non-constant-time bearer token comparison | **RESOLVED** | `subtle::ConstantTimeEq` in `auth.rs` line 164-173 |
| P0-003 | `expect("store lock poisoned")` panics | **RESOLVED** | All locks are `parking_lot::RwLock` (non-poisonable) |
| P0-004 | `unimplemented!()` in production paths | **RESOLVED** | Zero instances in non-test, non-comment code |
| P1-001 | Rate limiter before authentication | **RESOLVED** | Auth middleware runs before rate limiting |
| P1-003 | `preserve_order` feature guard missing | **NEEDS VERIFICATION** | Verify in `msez-core` canonicalization |

---

**End of Architecture Audit v5.0**

Momentum · `momentum.inc`  
Mass · `mass.inc`  
Confidential · February 2026
