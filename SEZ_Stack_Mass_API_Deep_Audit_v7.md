# SEZ Stack & Mass API: Deep Architectural Audit, Fortification & Rewrite Specification

**Version**: 7.0 — February 15, 2026
**Classification**: CONFIDENTIAL
**Prepared for**: Raeez Lorgat, Managing Partner, Momentum
**Scope**: Full-stack architectural review covering `momentum-sez/stack` (Rust codebase), Mass Java/Spring Boot APIs, boundary integrity, CLAUDE.md operational prompt, and Pakistan GovOS deployment readiness.

---

## PART I: EXECUTIVE FINDINGS

This audit evaluates the complete Momentum sovereign infrastructure across three planes: (1) the Mass APIs implementing the five programmable primitives, (2) the Rust-based SEZ Stack providing jurisdictional orchestration, and (3) the CLAUDE.md operational prompt that governs all development. The audit was conducted against the CLAUDE.md v6.0 specification, the Pakistan GovOS Architecture v4.0 schematic, the Momentum SEZ Stack Technical Specification v0.4.44, the Mass Protocol Enhanced Specification, the Momentum Monograph, and all project knowledge sources.

The architecture is conceptually sound and genuinely differentiated. The two-system separation (Mass for CRUD primitives, SEZ Stack for jurisdictional intelligence) is the correct architecture for sovereign deployment. The five-primitive model (Entities, Ownership, Fiscal, Identity, Consent) maps cleanly onto institutional requirements. The compliance tensor, pack trilogy, and corridor system compose into a legitimate jurisdictional orchestration layer.

However, this audit identifies 7 breaking issues, 12 major issues, and 19 minor issues that must be resolved before sovereign production deployment. The most critical finding is a **coherence gap between the sales narrative ("five programmable primitives"), the live API topology, and the Rust crate structure** — specifically around the Identity primitive, which has no dedicated Mass service and whose SEZ Stack client is an aggregation facade over two other services. This is not merely a technical deficiency; it is a structural dishonesty that will surface during government technical due diligence.

---

## PART II: BREAKING ISSUES (B-CLASS)

These will cause deployment failures, data corruption, or contractual breach if not resolved before sovereign handover.

### B-001: Identity Primitive Has No Dedicated Mass API Service

**Severity**: Breaking
**Location**: Mass API topology; `msez-mass-client::IdentityClient`
**Impact**: The sales line "five programmable primitives" is architecturally false for Identity.

The Pakistan GovOS schematic shows Identity as a first-class primitive with CNIC-NTN cross-reference, passportable KYC/KYB, and NADRA integration. The CLAUDE.md v6.0 correctly flags this (P1-005): Identity has no dedicated `identity-info.api.mass.inc` service. Identity functionality is split across `consent-info` and `organization-info`, with the Rust `IdentityClient` serving as an aggregation facade.

This is not a technical nicety. Pakistan's PDA (Pakistan Digital Authority) will perform technical due diligence. When they see five boxes on the architecture diagram but only four dedicated API services, the discrepancy becomes a trust issue at sovereign partnership level.

**Required Resolution**:
(a) Ship `identity-info.api.mass.inc` as a dedicated Spring Boot service with its own persistence, its own Swagger spec, and its own deployment pipeline — matching the other four primitives.
(b) The service must own: KYC/KYB records, CNIC-NTN cross-reference data, DID issuance and management, credential storage, and passportable identity attestations.
(c) `msez-mass-client::IdentityClient` must be rewritten from aggregation facade to proper typed HTTP client against the new service's Swagger spec.
(d) Migration path: existing identity data scattered across consent-info and organization-info must be migrated to the new service with zero downtime.

**Timeline**: P0. Must ship before any sovereign deployment signs a binding technical annex.

### B-002: No Contract Tests Against Live Mass API Swagger Specs

**Severity**: Breaking
**Location**: `msez-mass-client/` test suite
**Impact**: The Rust client may silently diverge from the actual Java API behavior, causing runtime failures in production with real capital.

This was flagged as P0-008 in CLAUDE.md v6.0 and remains open. The `msez-mass-client` test suite uses hardcoded mock JSON responses. When the Java team changes a field name, adds a required field, or modifies a response envelope, the Rust client will silently accept the old shape in tests but fail in production.

For a system processing $1.7B+ in capital across multiple jurisdictions, this is not acceptable. A single deserialization failure on a treasury operation could cause a payment to fail silently or, worse, succeed with incorrect amounts.

**Required Resolution**:
(a) Fetch Swagger JSON specs from each live Mass API endpoint at test time (or from a committed snapshot with freshness validation).
(b) Auto-generate Rust request/response types from the Swagger specs using a build script or code generation tool (e.g., `openapi-generator` with Rust target, or custom `serde` type generation).
(c) Every `msez-mass-client` sub-client must have contract tests that validate: correct HTTP method, correct URL path construction (including context-path prefix), correct request body serialization, correct response body deserialization, and correct error response handling.
(d) CI must fail if Swagger specs change without corresponding Rust type updates.

### B-003: Tax Collection Pipeline Not End-to-End Integrated

**Severity**: Breaking
**Location**: `msez-api/src/routes/tax.rs`, `msez-agentic/src/tax.rs`
**Impact**: The 10.3% → 15% GDP target is the headline deliverable for Pakistan. Without an end-to-end pipeline, the entire GovOS value proposition collapses.

The Pakistan GovOS architecture shows a tax collection pipeline: every economic activity on Mass → tax event → automatic withholding at source → real-time FBR IRIS reporting → AI-powered gap analysis. This is flagged as P1-009 in CLAUDE.md v6.0 and remains open.

The routes exist. The agentic tax module exists. But there is no evidence of end-to-end integration testing that traces a single transaction from Mass fiscal API through tax event generation, withholding computation, FBR IRIS reporting, and gap analysis. Without this, the pipeline is a collection of components, not a system.

**Required Resolution**:
(a) Implement an integration test in `msez-integration-tests` that traces a complete transaction through the full pipeline: fiscal API call → agentic tax event generation → withholding computation → FBR IRIS report generation → gap analysis trigger.
(b) The test must use a realistic Pakistan jurisdiction configuration (lawpack with Income Tax Ordinance 2001, regpack with current FBR rates, licensepack with SECP registration).
(c) Withholding computation must handle all five tax categories visible in the architecture: income tax, sales tax, federal excise duty, customs duty, and provincial taxes.
(d) FBR IRIS integration must be adapter-based (real API in production, mock in test) with the adapter interface defined in `msez-mass-client` or a dedicated `msez-integrations` crate.

### B-004: Write-Path Orchestration Incomplete for All Five Primitives

**Severity**: Breaking
**Location**: `msez-api/src/routes/mass_proxy.rs`
**Impact**: Without full orchestration on write paths, Mass degrades to a generic CRUD API with no jurisdictional intelligence.

CLAUDE.md v6.0 P1-004 states entity creation is orchestrated but other primitives need verification. The correct pattern for every write operation is: compliance evaluation → Mass API call → VC issuance → attestation storage. If Ownership, Fiscal, Identity, or Consent write paths bypass compliance evaluation and VC issuance, those operations produce entities/transactions that are not provably compliant.

For sovereign deployment, every state-changing operation must produce a Verifiable Credential attesting to its compliance status. A government auditor must be able to trace any entity, any ownership change, any payment, any identity assertion, and any consent action to a cryptographic proof of compliance at the time of execution.

**Required Resolution**:
(a) Audit every POST/PUT/DELETE route in `mass_proxy.rs` for all five primitives.
(b) For each write path, implement the full orchestration: pre-flight compliance tensor evaluation → Mass API call → VC issuance (using the appropriate credential type) → attestation storage in Postgres.
(c) GET routes may remain as pass-through proxies.
(d) Each orchestrated write path must return an `OrchestrationEnvelope` containing: the Mass API response, the compliance evaluation result (across all relevant domains), the issued VC, and the attestation ID.

### B-005: CanonicalBytes Bypass Risk Unverified

**Severity**: Breaking
**Location**: All 16 crates
**Impact**: If any crate computes SHA-256 outside `CanonicalBytes::new()`, the deterministic digest guarantee is broken, making Verifiable Credentials and receipt chains unforgeable but unverifiable.

P1-010 in CLAUDE.md v6.0 flags this as open. The entire cryptographic integrity of the system rests on a single invariant: all SHA-256 computation flows through `CanonicalBytes::new()`. If a developer imports `sha2::Sha256` directly and computes a digest without going through the canonical path (which includes JSON Canonicalization Scheme normalization), the resulting digest will not match other digests of the same logical content. This breaks receipt chain verification, credential verification, and tensor commitment verification.

**Required Resolution**:
(a) `grep -r "sha2::Sha256" --include="*.rs"` across all crates. Every hit outside `msez-core/src/canonical.rs` and `msez-core/src/digest.rs` is a bug.
(b) Add a CI check that fails the build if `sha2::Sha256` appears in any file outside the canonical path.
(c) Consider using a Rust lint rule or a `#[deny]` attribute to enforce this at compile time.

### B-006: Auth Token Stored as Plain String

**Severity**: Breaking
**Location**: `msez-api/src/auth.rs`
**Impact**: Auth tokens in memory as `Option<String>` can be leaked through heap dumps, core dumps, or debug logging.

P2-004 in CLAUDE.md v6.0 flags this as open. For a system operating in sovereign contexts where infrastructure may be subject to physical security inspection, authentication tokens stored as plain strings represent a verifiable security deficiency.

**Required Resolution**:
(a) Replace `Option<String>` with `secrecy::SecretString` for all auth tokens in `AppState`.
(b) Implement `Zeroize` on drop for all authentication material.
(c) Ensure `Debug` and `Display` implementations for auth-bearing types do not expose token values (they should print `[REDACTED]` or similar).
(d) Audit all `tracing::info!` and `tracing::debug!` calls near auth/token/key variables to confirm no secrets appear in log output.

### B-007: Identifier Type Mismatch Between Mass Client and Core

**Severity**: Breaking for type safety
**Location**: `msez-mass-client` vs `msez-core`
**Impact**: `EntityId` in `msez-mass-client` is raw `uuid::Uuid`; in `msez-core` it is `EntityId(Uuid)`. This allows silent type confusion where a `JurisdictionId` UUID could be passed where an `EntityId` is expected.

P2-002 in CLAUDE.md v6.0 presents this as a decision point. The decision should be clear: **create a `msez-types` crate** that exports identifier newtypes and is depended on by both `msez-core` and `msez-mass-client`. This crate has zero logic — it is pure type definitions with `serde` derives. This avoids the "mass-client depends on core" coupling while maintaining type safety.

**Required Resolution**:
(a) Create `msez-types` crate containing: `EntityId`, `JurisdictionId`, `CorridorId`, `Did`, `Cnic`, `Ntn`, and all other identifier newtypes.
(b) `msez-core` depends on `msez-types`.
(c) `msez-mass-client` depends on `msez-types`.
(d) No other logic in `msez-types` — it is a leaf crate with zero internal dependencies.
(e) Update dependency invariants: `msez-types` has zero internal dependencies (new invariant 0), `msez-core` depends only on `msez-types`, `msez-mass-client` depends only on `msez-types`.

---

## PART III: MAJOR ISSUES (M-CLASS)

These will not cause immediate failures but represent significant architectural debt, security risk, or coherence gaps that degrade system quality and auditability.

### M-001: Python References Persist in Technical Specification

The SEZ Stack Technical Specification v0.4.44 still references Python module tables (tensor.py: 956 lines, zkp.py: 666 lines, etc.) in Chapter 5 context. While CLAUDE.md v6.0 confirms all Python has been removed (P2-005 resolved, codebase is pure Rust at 101K lines), the technical specification document has not been updated. This creates confusion for external reviewers who see Python module tables alongside claims of a pure-Rust codebase. The specification must be revised to reflect the current Rust crate structure exclusively.

### M-002: PHOENIX Module Suite Naming Conflict

The technical specification references a "PHOENIX Module Suite" with specific Python module names. The Rust codebase uses the `msez-*` naming convention. There is no mapping document between PHOENIX module names and Rust crate names. External technical reviewers will encounter "PHOENIX" in specs and `msez-*` in code with no bridge between them. Either retire the PHOENIX name entirely or create an explicit mapping table in both the specification and CLAUDE.md.

### M-003: Compliance Tensor Domain Count Must Be Verified

CLAUDE.md v6.0 specifies 20 compliance domains in the `ComplianceDomain` enum. The technical specification lists compliance domains but the count is not explicitly verified against the code. A mismatch between the documented domain count and the actual enum variant count would mean the compliance tensor has incorrect dimensionality, which propagates to every tensor evaluation, every compliance assessment, and every VC issued.

**Required**: Add a compile-time assertion (`const_assert!` or equivalent) that the `ComplianceDomain` enum has exactly 20 variants. Add the canonical list of 20 domains to CLAUDE.md.

### M-004: Corridor State FSM Needs Formal Verification Against Spec

The corridor lifecycle FSM (DRAFT → PENDING → ACTIVE → HALTED/SUSPENDED → TERMINATED) is defined in `governance/corridor.lifecycle.state-machine.v2.json` and implemented in `msez-state`. There is no property-based test or formal verification that the implementation matches the specification. For corridors handling $10.1B (PAK↔UAE), $5.4B (PAK↔KSA), and $23.1B (PAK↔CHN), an invalid state transition could result in a corridor being activated without proper compliance checks or halted without proper suspension procedures.

### M-005: Settlement Layer Specification vs. Implementation Gap

The technical specification describes a "two-tier sharded architecture achieving 100,000 to 10,000,000 effective transactions per second" with DAG-based consensus, Narwhal-Bullshark, and Plonky3 STARK proving. The CLAUDE.md v6.0 and the crate structure show no evidence of a settlement layer implementation — `msez-zkp` contains "sealed ProofSystem trait" with "mock implementations." The specification describes a production-grade L1 blockchain; the code contains stubs.

This is the most significant "spec-code gap" in the system. The specification is aspirational; the code is real. These must be reconciled. Either the specification must be clearly versioned with "IMPLEMENTED" and "PLANNED" labels on every capability, or the code must be advanced to match. For sovereign deployment, governments will read the specification and expect the implementation to match.

### M-006: Smart Asset Virtual Machine (SAVM) Is Specified But Not Implemented

The specification details a complete virtual machine with instruction categories (0x00-0xFF), gas metering, compliance coprocessor, and execution receipts. There is no corresponding Rust crate for VM execution. `msez-state` handles FSM transitions but is not a virtual machine. This is a significant specification-reality gap.

### M-007: ZKP Circuits Are Stubs

The specification describes 12 circuit types with specific constraint counts (πpriv ~34,000, πcomp ~25,000, etc.). CLAUDE.md v6.0 notes that `msez-zkp` contains "stubs with mock implementations." For any deployment that claims zero-knowledge proof capabilities in marketing materials or government proposals, this must be flagged as "PLANNED" not "IMPLEMENTED."

### M-008: Arbitration Module Institutional Registry Incomplete

`msez-arbitration` lists DIFC-LCIA, SIAC, and ICC as arbitration institutions. The Pakistan GovOS architecture shows an "Arbitration Corpus" with "Tax tribunal rulings, ATIR precedents, court filings, dispute history." Pakistan's arbitration infrastructure includes the Alternate Dispute Resolution Act 2017, the Arbitration Act 1940, and specialized tax tribunals (ATIR — Appellate Tribunal Inland Revenue). The arbitration module must be extended to support Pakistan-specific institutions and precedent databases.

### M-009: GovOS Console Routes Not Defined

The Pakistan GovOS architecture shows 5 dashboards at the experience layer: GovOS Console (40+ ministries), Tax & Revenue Dashboard, Digital Free Zone, Citizen Tax & Services, and Regulator Console. Only the Regulator Console has defined routes in `msez-api`. The other four dashboards need route definitions, data models, and access control specifications.

### M-010: SBP Raast Integration Adapter Missing

The Pakistan GovOS architecture shows "SBP API Gateway — Central bank direct, Raast, RTGS, FX" as a core infrastructure component. Raast is Pakistan's instant payment system operated by the State Bank of Pakistan. There is no evidence of a Raast adapter in the codebase. The `msez-mass-client::FiscalClient` handles generic treasury operations but has no Pakistan-specific payment rail adapters.

**Required**: Define a `PaymentRailAdapter` trait in `msez-corridor` or `msez-mass-client` with implementations for: Raast (PKR instant payments), RTGS (large-value settlements), SWIFT (cross-border via pacs.008, already stubbed), and Circle USDC (already stubbed). The adapter pattern allows jurisdiction-specific payment rails to be plugged in without modifying core orchestration logic.

### M-011: NADRA Integration Not Specified

The architecture shows NADRA (National Database and Registration Authority) as a critical national system for "National identity, CNIC-NTN cross-ref." There is no NADRA adapter or integration specification in the codebase. CNIC (Computerized National Identity Card) verification against NADRA is a legal requirement for KYC in Pakistan. Without this integration, the Identity primitive cannot function in Pakistan.

### M-012: Data Sovereignty Enforcement Not Programmatic

The Pakistan GovOS architecture states "All data, compute, AI models & inference — Pakistani jurisdiction · Pakistani infrastructure · Pakistani engineers." CLAUDE.md v6.0 states "the SEZ Stack respects this — it orchestrates, it does not centralize." But there is no programmatic enforcement of data sovereignty. No configuration that constrains which data can be replicated to which jurisdictions. No audit log that tracks cross-boundary data access. For sovereign deployment, data sovereignty must be enforced by code, not by policy.

---

## PART IV: MINOR ISSUES (S-CLASS)

### S-001: Templating Engine Not Listed as Mass Primitive
The templating engine (`templating-engine`, Heroku) generates formation certificates, tax filings, and compliance reports. It is a supporting service, not a sixth primitive, but it is not clearly positioned in the architecture. The GovOS schematic does not show it. It should be documented as a cross-cutting service that supports all five primitives.

### S-002: Swagger UI Accessibility
All five Mass API Swagger endpoints returned 403 during this audit. Either the endpoints are behind authentication (correct for production) or they are down. The CLAUDE.md lists them for reference, but developers need access to these specs for contract test generation. A CI-accessible spec mirror should be maintained.

### S-003: Integration Test Count
CLAUDE.md v6.0 mentions "60+ integration test files." The total test count is 3,029. The ratio of integration tests to unit tests should be documented, and integration tests should be tagged to distinguish from unit tests in CI reporting.

### S-004: Profile Templates
The architecture mentions 7 deployment profiles but does not enumerate them. These should be listed in CLAUDE.md with their target jurisdictions and configuration differences.

### S-005: JSON Schema Count Drift
CLAUDE.md mentions 116 JSON schemas. As the system evolves, this count will change. The number should be computed dynamically (e.g., `find schemas/ -name "*.json" | wc -l`) rather than hardcoded.

### S-006: OpenAPI Spec for msez-api
`msez-api` uses utoipa for OpenAPI generation. The generated spec should be committed to the repository and validated in CI, not generated only at runtime.

### S-007: Heroku Deployment for Investment Info
`investment-info` is deployed on Heroku while other Mass APIs are on `*.api.mass.inc`. This inconsistency should be resolved by migrating investment-info to `investment-info.api.mass.inc`.

### S-008: No Explicit Rate Limiting Configuration
CLAUDE.md v6.0 mentions rate limiting in `msez-api` but does not specify the configuration. Rate limits should be documented per endpoint category (public, authenticated, admin) and per jurisdiction deployment profile.

### S-009: Absent Health Check Endpoints
No mention of `/health` or `/ready` endpoints in `msez-api`. These are required for container orchestration (Kubernetes liveness/readiness probes) in sovereign data center deployments.

### S-010: No Observability Stack Specified
The architecture shows "SLAs, audit trails, risk dashboards" under the Regulator Console but does not specify the observability stack. For sovereign deployment, the logging, metrics, and tracing infrastructure must be specified and must run within the sovereign boundary.

### S-011: BBS+ Selective Disclosure Marked as Stubs
`msez-vc` has "BBS+ selective disclosure stubs." This capability is referenced in the KYC tier system and is critical for privacy-preserving credential verification. It should be promoted from stub to implementation priority.

### S-012: No Backup/Recovery Specification
For sovereign deployment, data backup and disaster recovery procedures must be specified. Postgres persistence is mentioned but backup schedules, RPO/RTO targets, and recovery procedures are not.

### S-013: Licensing Authority Bridge Not Detailed
The GovOS architecture shows connections to BOI, PTA, PEMRA, DRAP licensing authorities. The `msez-pack` licensepack system is generic. Pakistan-specific license type mappings (SECP categories, PTA telecom licenses, DRAP drug licenses, PEMRA media licenses) need to be created.

### S-014: Corridor Netting Not Tested
`msez-corridor` lists netting as a capability. For high-volume corridors (PAK↔CHN at $23.1B), netting is essential for settlement efficiency. Integration tests for netting logic are needed.

### S-015: No Explicit Versioning Strategy for Pack Trilogy
Lawpacks, regpacks, and licensepacks will change as laws are amended. The specification describes `previous_regpack_digest` for delta computation, but there is no explicit versioning strategy (semver? date-based? legislative session-based?) documented in CLAUDE.md.

### S-016: Watcher Economy Bonds and Slashing Not End-to-End Tested
The watcher economy is described with bonds, slashing, and reputation. This is a complex economic mechanism that needs game-theoretic analysis and adversarial testing.

### S-017: CLI Key Management
`msez-cli` performs VC keygen/sign/verify. Key storage and management practices for CLI-generated keys should be specified (hardware security module support, key derivation, rotation procedures).

### S-018: No Load Testing Framework
For a system targeting 100K+ TPS (specification) or even the more realistic current-state throughput, there is no load testing framework specified. Before sovereign deployment, the system needs benchmarking under realistic load.

### S-019: Agentic Trigger Taxonomy Needs Domain Validation
`msez-agentic` specifies "20 types × 5 domains" for the trigger taxonomy. These must be validated against the 20 ComplianceDomain variants and the five Mass primitives to ensure complete coverage.

---

## PART V: SALES LINE ↔ ARCHITECTURE COHERENCE ANALYSIS

### The Claim: "Five Programmable Primitives"

**Entities** — ✅ COHERENT. `organization-info.api.mass.inc` is a dedicated service. `msez-mass-client::EntityClient` is a typed HTTP client. Orchestration exists for entity creation. The GovOS schematic shows Entity as a first-class box with FBR binding.

**Ownership** — ✅ COHERENT (with caveat). `investment-info` is a dedicated service. `msez-mass-client::OwnershipClient` exists. Caveat: deployed on Heroku, not on `*.api.mass.inc` domain. This is a deployment inconsistency, not an architectural one.

**Fiscal** — ✅ COHERENT. `treasury-info.api.mass.inc` is a dedicated service. `msez-mass-client::FiscalClient` exists. PKR collection, withholding at source, and SBP Raast integration are specified in the GovOS architecture.

**Identity** — ❌ INCOHERENT. No dedicated `identity-info.api.mass.inc` service. Functionality split across `consent-info` and `organization-info`. Rust client is an aggregation facade. NADRA integration unspecified. This primitive exists in the sales deck but not in production architecture.

**Consent** — ✅ COHERENT. `consent.api.mass.inc` is a dedicated service. `msez-mass-client::ConsentClient` exists. Multi-party audit trails and governance approvals are specified.

### The Supporting Layer Claim: "Jurisdictional Context"

The SEZ Stack's value-add — the compliance tensor, pack trilogy, corridor system, Verifiable Credentials, and agentic automation — is well-architected and genuinely novel. The separation between Mass (CRUD) and SEZ Stack (jurisdictional intelligence) is the right architecture. The Pack Trilogy (lawpacks, regpacks, licensepacks) is a legitimate innovation in how regulatory knowledge is structured and consumed.

### Recommendation

Fix Identity (B-001) and the sales line becomes architecturally honest. The other four primitives have dedicated services, typed clients, and clear orchestration paths. The jurisdictional context layer is legitimate and differentiating. Once Identity ships as a dedicated service, the "five programmable primitives" claim is fully backed by production architecture.

---

## PART VI: CLAUDE.md v7.0 FORTIFICATION REQUIREMENTS

The current CLAUDE.md v6.0 is strong. It correctly identifies the Mass/SEZ boundary, enforces dependency invariants, and provides a clear audit methodology. The following additions are required for v7.0:

### 1. Add the Canonical 20 ComplianceDomain Variants
The enum must be listed explicitly so that every developer, auditor, and AI coding assistant knows the exact domains. Currently referenced as "20 domains" without enumeration.

### 2. Add Specification-Reality Gap Flags
For every capability claimed in the Technical Specification v0.4.44, CLAUDE.md should carry a status indicator: `[IMPLEMENTED]`, `[PARTIAL]`, `[STUB]`, or `[PLANNED]`. This prevents AI assistants from generating code that assumes capabilities exist when they do not.

### 3. Add Pakistan GovOS Deployment Checklist
Given that Pakistan is the first sovereign deployment, a Pakistan-specific checklist should be in CLAUDE.md covering: FBR IRIS integration status, SBP Raast adapter status, NADRA adapter status, SECP registration flow status, and all 40+ ministry dashboard route definitions.

### 4. Add Anti-Slop Expanded Patterns
The current anti-slop protocol is good. Add these patterns specific to this codebase:
- Mock implementations that return `Ok(())` without exercising any logic
- Swagger spec drift detection (any `msez-mass-client` response type that was last updated more than 30 days before the corresponding Mass API spec)
- Compliance tensor evaluations that return `Compliant` for all domains without actually checking anything
- VC issuance that issues credentials without performing the compliance check they attest to
- Corridor state transitions that skip the FSM validation

### 5. Add the `msez-types` Crate to Dependency Tree
Once B-007 is resolved, the dependency tree in Section V must be updated to show `msez-types` as the true leaf crate.

### 6. Add Deployment Profile Enumeration
List all 7 deployment profiles with their target jurisdictions, enabled modules, and configuration parameters.

### 7. Add Integration Point Registry
A table mapping every external system integration (FBR IRIS, SBP Raast, NADRA, SECP, Northern Trust, SWIFT, Circle, OFAC, EU sanctions, UN sanctions) to its adapter status, crate location, and test coverage.

---

## PART VII: MASS API DESIGN RECOMMENDATIONS

### 7.1 Identity Service Architecture

The new `identity-info.api.mass.inc` service should own:

**Core Resources:**
- `/identity-info/api/v1/identities` — CRUD for identity records (natural persons, legal entities)
- `/identity-info/api/v1/verifications` — KYC/KYB verification workflows
- `/identity-info/api/v1/credentials` — Credential issuance and management
- `/identity-info/api/v1/bindings` — CNIC-NTN, DID-Entity, and cross-reference bindings
- `/identity-info/api/v1/attestations` — Third-party attestation records

**Pakistan-Specific Extensions:**
- `/identity-info/api/v1/nadra/verify` — CNIC verification against NADRA
- `/identity-info/api/v1/fbr/ntn-lookup` — NTN lookup and cross-reference

**Design Principles:**
(a) Follow the same Spring Boot / context-path / Swagger convention as the other four services.
(b) Support progressive KYC tiers (Tier 0-3 as specified in the technical specification).
(c) Support multi-jurisdiction identity with jurisdiction-specific extensions as path segments or query parameters, not separate services.
(d) Support credential portability — an identity verified in Pakistan must be expressible as a W3C Verifiable Credential that can be verified in UAE, Kazakhstan, or any other Mass-deployed jurisdiction.

### 7.2 Cross-Service Consistency

All five Mass API services should enforce consistent patterns:

**Error Response Format:**
```json
{
  "error": {
    "code": "ENTITY_NOT_FOUND",
    "message": "Entity with ID {id} not found",
    "details": {},
    "trace_id": "uuid"
  }
}
```

**Pagination:**
```json
{
  "data": [...],
  "pagination": {
    "page": 1,
    "page_size": 20,
    "total_count": 142,
    "total_pages": 8
  }
}
```

**Audit Headers:**
Every response should include `X-Mass-Trace-Id`, `X-Mass-Jurisdiction`, and `X-Mass-Compliance-Status` headers for downstream orchestration.

### 7.3 Investment Info Migration

Move `investment-info` from Heroku to `investment-info.api.mass.inc` to match the deployment pattern of the other four services. This is a deployment concern, not an architectural one, but it affects the coherence of the API surface.

---

## PART VIII: IMMEDIATE ACTION PLAN

### Week 1-2: Breaking Issues
1. B-001: Design and begin implementation of `identity-info.api.mass.inc`
2. B-002: Set up contract test infrastructure using Swagger spec snapshots
3. B-005: Run `CanonicalBytes` bypass audit, add CI check
4. B-006: Replace auth token storage with `secrecy::SecretString`
5. B-007: Create `msez-types` crate, migrate identifier newtypes

### Week 3-4: Breaking Issues (continued) + Major Issues
6. B-003: Implement end-to-end tax pipeline integration test
7. B-004: Audit and complete orchestration on all five primitive write paths
8. M-003: Add compile-time assertion for ComplianceDomain variant count
9. M-010: Define PaymentRailAdapter trait and Raast stub
10. M-011: Specify NADRA integration adapter interface

### Week 5-8: Major Issues + Specification Reconciliation
11. M-001: Update technical specification to remove Python references
12. M-002: Resolve PHOENIX ↔ msez-* naming
13. M-005/M-006/M-007: Add [IMPLEMENTED]/[STUB]/[PLANNED] flags to specification
14. M-009: Define GovOS Console route specifications
15. M-012: Implement programmatic data sovereignty enforcement

### Week 9-12: Production Hardening
16. Resolve all S-class minor issues
17. Load testing framework setup
18. Observability stack specification
19. Backup/recovery procedures
20. Full audit re-run against v7.0 criteria

---

## PART IX: CONCLUSION

The Momentum SEZ Stack and Mass API architecture is fundamentally sound. The two-system separation is correct. The five-primitive model is the right abstraction. The compliance tensor, pack trilogy, and corridor system compose into a genuinely novel jurisdictional orchestration layer that does not exist elsewhere in the market.

The primary risk is not architectural — it is the gap between specification and implementation. The technical specification describes a production-grade L1 blockchain with ZK circuits, a virtual machine, and DAG-based consensus. The code contains a solid but incomplete jurisdictional orchestration layer with stubs where the specification claims implementations. This gap must be honestly acknowledged and systematically closed, prioritizing the components needed for sovereign deployment (tax pipeline, identity service, payment rail adapters) over theoretical completeness (SAVM, full ZKP circuits, settlement layer).

The single most impactful action is shipping `identity-info.api.mass.inc`. This transforms the "five programmable primitives" from a 4/5 truth to a 5/5 truth, and it is the primitive most scrutinized by government partners during technical due diligence.

---

**Momentum** · `momentum.inc`
**Mass** · `mass.inc`
**Confidential** · February 2026
