# SEZ Stack & Mass API Deep Audit Report
## `momentum-sez/stack` v0.4.44 GENESIS
### Prepared for: Pakistan Digital Authority Deployment Readiness Assessment
### Date: February 12, 2026

---

## Executive Summary

This audit examines the Momentum Open Source SEZ Stack repository against its specification, the live Mass API surface, and the Pakistan GovOS Platform Architecture deployment target. The stack represents an extraordinarily ambitious programmable jurisdiction framework — the specification is world-class in its theoretical rigor and the architecture is genuinely novel. The reference implementation, however, exhibits a systematic pattern: **the specification dramatically outpaces the code**. The phoenix layer (14,363 lines of Python) is a well-structured simulation of the spec's cryptographic and state-machine behaviors, but relies on mock cryptography, non-standard canonicalization, and in-memory data structures that cannot serve as production sovereign infrastructure without significant hardening.

The most critical finding is a **canonicalization split** between the core layer (smart_asset.py, vc.py, lawpack.py) which uses proper JCS, and the entire phoenix layer (17 files) which uses `json.dumps(sort_keys=True)` — a divergence that will produce different digests for the same data across module boundaries. For a system where content-addressed integrity is the foundational trust primitive, this is a production blocker.

The Pakistan GovOS deployment requires building approximately **70% net-new infrastructure** on top of what exists today, including all national system integrations (FBR IRIS, SBP Raast, NADRA, SECP), the sovereign AI layer, and the cross-border corridor bilateral engines. The existing stack provides a solid conceptual foundation and a functional CLI toolchain for zone validation, artifact management, and compliance evaluation — but the gap from "reference implementation with scaffold APIs" to "sovereign digital infrastructure serving 220M citizens" is substantial.

Below are the findings organized by the seven-pass audit methodology.

---

## PASS 1: Structural Integrity & Spec-to-Code Fidelity

### Critical Findings

**1.1 Canonicalization Split — CDB Violation Across Module Boundaries**

- **Files**: All 17 files in `tools/phoenix/` (security.py:94, tensor.py:716,818, zkp.py:287,365,527,710, anchor.py:141, bridge.py:136,194, migration.py:338,764, watcher.py:217,235, events.py:166, observability.py:499)
- **Issue**: The entire phoenix layer uses `json.dumps(content, sort_keys=True, separators=(",", ":"))` for digest computation, while the core layer (smart_asset.py, vc.py, lawpack.py) uses `jcs_canonicalize()` which applies `_coerce_json_types()` preprocessing — rejecting floats, normalizing datetimes to UTC ISO strings with Z suffix, and coercing non-string dict keys.
- **Impact**: Any data containing datetime objects, floats, or non-string keys will produce **different SHA256 digests** depending on whether it passes through the phoenix layer or the core layer. This means a compliance tensor commitment computed by tensor.py will not match a verification computed by smart_asset.py for the same underlying data. For a content-addressed system where digest equality is the trust primitive, this is a foundational integrity failure.
- **Remediation**: Replace all 17 instances of `json.dumps(sort_keys=True)` in the phoenix layer with calls to `jcs_canonicalize()` from `tools.lawpack`. Estimated effort: 2 engineering days plus full regression testing.

**1.2 Poseidon2 Hash — Specified but Not Implemented**

- **Files**: Entire codebase (searched all `tools/`)
- **Issue**: The specification defines the Canonical Digest Bridge as `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`. Poseidon2 is a ZK-friendly hash function critical for the proof system. The implementation uses only SHA256 everywhere. The single reference to "Poseidon" in the codebase is a pseudocode comment in `tools/mass_primitives.py:748`.
- **Impact**: All ZK proof circuits that reference the CDB will produce proofs against SHA256 digests, not Poseidon2 digests. If a production ZK backend is ever connected, every existing digest commitment becomes invalid.
- **Remediation**: This is architecturally acceptable as a Phase 1 decision — SHA256-only is fine for the deterministic compliance evaluation that the Pakistan deployment needs. However, the spec should be annotated to clearly mark CDB/Poseidon2 as a Phase 2 enhancement, and the code should use a `digest_algorithm` field in all commitment structures to enable future migration. Estimated effort: 1 day for annotation, 3 days for algorithm-agnostic refactor.

**1.3 Corridor State Machine — Spec-Implementation Divergence**

- **File**: `governance/corridor.lifecycle.state-machine.v1.json`
- **Issue**: The spec defines states as DRAFT→PENDING→ACTIVE with HALTED/SUSPENDED branches. The implementation defines PROPOSED→OPERATIONAL→HALTED→DEPRECATED. These are fundamentally different state machines with different state names, different transition semantics, and missing states (PENDING and SUSPENDED are absent from the implementation; ACTIVE maps imprecisely to OPERATIONAL).
- **Impact**: Any external system (including the Pakistan GovOS regulator console) that references corridor states by name will encounter mismatches. The corridor state machine JSON is referenced by the CI pipeline and the corridor-state OpenAPI — all downstream consumers inherit the wrong terminology.
- **Remediation**: Align the implementation to the spec's state names, or formally version the divergence as a spec amendment with a migration path. Estimated effort: 1 day.

### High-Severity Findings

**1.4 Compliance Tensor Domain Mismatch**

- **Files**: `tools/phoenix/tensor.py` vs `tools/msez/composition.py`
- **Issue**: The `ComplianceDomain` enum in tensor.py defines 8 domains (AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY, DATA_PRIVACY). The `Domain` enum in composition.py defines 20 domains including LICENSING, BANKING, PAYMENTS, CLEARING, SETTLEMENT, DIGITAL_ASSETS, EMPLOYMENT, IMMIGRATION, IP, CONSUMER_PROTECTION, and ARBITRATION. The spec describes LICENSING as a 9th tensor domain. This means the composition module can reference compliance domains that the tensor cannot materialize, slice, or commit.
- **Impact**: A multi-zone composition that requires LICENSING compliance evaluation will silently fail — the tensor has no cell for that domain. The Pakistan deployment explicitly includes 15+ license categories in its Licensepack layer.
- **Remediation**: Either expand the tensor's `ComplianceDomain` enum to include LICENSING (minimal fix), or implement a domain mapping layer between composition domains and tensor domains. Estimated effort: 1-3 days depending on approach.

**1.5 ZKP System — Entirely Mocked**

- **File**: `tools/phoenix/zkp.py`
- **Issue**: All proof generation uses `secrets.token_hex(32)` and `hashlib.sha256()` to create "deterministic mock proofs" (line 515, 525). The spec describes 5 NIZK systems (Groth16, PLONK, STARK, Bulletproofs, Halo2) with 12 circuit types. None have real cryptographic implementations.
- **Impact**: No zero-knowledge privacy guarantees exist in the current system. Any claim of "privacy-preserving compliance verification" is aspirational, not operational. This is acceptable for Phase 1 (Pakistan deployment doesn't require ZK proofs for tax compliance) but must be explicitly acknowledged.
- **Remediation**: Flag as Phase 2. For the Pakistan deployment, document that compliance verification is deterministic-transparent (not ZK-private) and that this is by design for a sovereign tax authority that requires full visibility.

**1.6 Module Implementation Gap — 583 Descriptors, 0 Python Files**

- **File**: `modules/` directory
- **Issue**: The modules directory contains 583 YAML descriptor files across 16 families, but zero Python implementation files. The module index claims 146/146 modules at 100% coverage. All actual implementation lives in `tools/msez.py` (15,472 lines), the phoenix layer, and supporting files.
- **Impact**: The "298 modules" or "146 modules" figure is a count of YAML metadata descriptors, not executable module implementations. Each YAML file defines a module's interfaces, dependencies, and compliance requirements — which is valuable for zone configuration validation — but there is no corresponding per-module business logic.
- **Remediation**: This is architecturally intentional — the YAML descriptors drive the `msez validate` and `msez lock` pipelines, and actual business logic is expected to be provided by the Mass API layer. However, the distinction between "module descriptor" and "module implementation" must be crystal clear in all deployment documentation.

---

## PASS 2: Schema & API Contract Integrity

### Critical Findings

**2.1 `additionalProperties: true` on Security-Critical Schemas**

- **Files**: `schemas/vc.smart-asset-registry.schema.json` (7 instances), `schemas/corridor.receipt.schema.json` (4 instances), `schemas/attestation.schema.json`
- **Issue**: Multiple security-critical schemas allow arbitrary additional properties. This means an attacker can inject unexpected fields into VCs, corridor receipts, and attestations that will pass schema validation.
- **Impact**: Schema injection attacks become possible. A malicious receipt could include fields that downstream processors interpret as authorization signals. The JCS canonicalization will include these injected fields in digest computation, potentially creating valid-looking commitments for tampered data.
- **Remediation**: Set `additionalProperties: false` on all security-critical schemas (VCs, receipts, attestations, proofs). Use `patternProperties` or explicit `additionalProperties` schemas where extensibility is genuinely needed. Estimated effort: 2-3 days for systematic lockdown plus regression testing.

**2.2 OpenAPI Specs Are Self-Described Scaffolds**

- **Files**: All 4 files in `apis/`
- **Issue**: Every OpenAPI spec explicitly labels itself as a "scaffold" or "skeleton." The mass-node API has only 2 endpoints (createEntity, submitAttestation). The regulator-console API has 1 endpoint (queryAttestations). These are spec-level stubs, not production APIs.
- **Impact**: The Pakistan GovOS architecture diagram shows 5 experience-layer portals (GovOS Console, Tax & Revenue Dashboard, Digital Free Zone, Citizen Tax & Services, Regulator Console) that need full API surfaces. The current scaffold APIs cover approximately 5% of the required endpoints.
- **Remediation**: The scaffold APIs should be expanded into full OpenAPI specifications as part of the Pakistan deployment engineering plan. The Mass API services (Organization Info, Investment Info, Consent Info, Treasury Info, Templating Engine) already provide substantial API surface that should be formally mapped into the SEZ Stack's OpenAPI layer.

**2.3 Mass API ↔ Five Primitives Mapping Gaps**

Based on the GovOS architecture diagram and the available Mass API endpoints, the following alignment issues exist:

The ENTITIES primitive maps well to the Organization Info API (entity formation, lifecycle, beneficial ownership). The OWNERSHIP primitive maps to the Investment Info API (cap tables, share classes, investor onboarding). The CONSENT primitive maps to the Consent Info API (multi-party authorization, audit trails). The FISCAL primitive maps to the Treasury Info API (accounts, payments, withholding). The IDENTITY primitive has **no corresponding Mass API** — this is a critical gap for the Pakistan deployment, which requires NADRA (CNIC) cross-referencing and passport/KYB integration.

Missing API families that exist in neither the SEZ Stack's `apis/` directory nor the live Mass API surface include: Tax API (FBR IRIS integration), Capital Markets API (SECP integration), Trade API (customs, bills of lading, letters of credit), Payment Rails API (SBP Raast), and Cross-Border Corridor API (bilateral settlement).

---

## PASS 3: Cryptographic Correctness & Security Hardening

### High-Severity Findings

**3.1 Constant-Time Comparison — Correctly Implemented**

- **File**: `tools/phoenix/hardening.py:417-422`
- **Status**: ✅ Uses `hmac.compare_digest()` which is the correct Python primitive for constant-time string comparison. This is well-implemented.

**3.2 ThreadSafeDict — Partial Coverage**

- **File**: `tools/phoenix/hardening.py:500-543`
- **Issue**: `ThreadSafeDict` wraps individual operations with `RLock`, but does not override `__iter__`, `keys()`, `values()`, or `items()`. A thread iterating over the dict while another thread modifies it will encounter `RuntimeError: dictionary changed size during iteration` or worse, silent data corruption.
- **Impact**: Any concurrent access pattern that involves iteration (e.g., listing all active watchers, scanning all tensor cells) is unsafe.
- **Remediation**: Override iteration methods to snapshot the dict under the lock, or document that iteration requires explicit use of the `transaction()` context manager. Estimated effort: 0.5 days.

**3.3 Swallowed Exceptions in Migration Compensation**

- **File**: `tools/phoenix/migration.py:627, 649, 664`
- **Issue**: The compensation saga catches `except Exception:` and silently sets `*_success = False` without logging the exception. If a compensation action (unlock_source, refund_fees, notify_counterparties) fails, the error details are lost.
- **Impact**: In a production migration failure, operators will see that compensation failed but have no diagnostic information about why. For a system handling cross-jurisdictional asset transfers, this is unacceptable.
- **Remediation**: Log the exception at ERROR level before setting the success flag. Estimated effort: 0.5 days.

**3.4 No Private Keys in Repository — Confirmed Clean**

- **File**: `docs/examples/keys/`
- **Status**: ✅ Example keys are clearly labeled as development/test keys. No private key material is committed to the repository. The Ed25519 JWK handling in `tools/vc.py` loads keys from file paths, not from hardcoded values.

**3.5 BBS+ Signatures — Not Implemented**

- **Issue**: The spec describes BBS+ signatures for selective disclosure of VC claims. The implementation has no BBS+ code. The `tools/vc.py` module supports only Ed25519 signatures.
- **Impact**: Selective disclosure of compliance attestations (e.g., proving KYC validity to a regulator without revealing personal data to counterparties) is not available. This is a Phase 2 feature.

---

## PASS 4: State Machine Correctness & Edge Cases

### High-Severity Findings

**4.1 Migration Timeout — No Deadline Enforcement**

- **File**: `tools/phoenix/migration.py`
- **Issue**: The migration saga defines `deadline` as a field in `MigrationSaga` but there is no timer, scheduler, or timeout handler that automatically triggers compensation when the deadline passes. A migration stuck in TRANSIT state will remain there indefinitely.
- **Impact**: An asset locked in SOURCE_LOCK or TRANSIT state with no timeout handler creates a permanent freeze. The asset cannot be used at either the source or destination jurisdiction.
- **Remediation**: Implement a deadline check that runs on every state transition attempt. If `datetime.now(utc) > deadline`, automatically transition to COMPENSATED with appropriate compensation actions. Estimated effort: 1-2 days.

**4.2 Compensation of Compensation — No Meta-Recovery**

- **File**: `tools/phoenix/migration.py`
- **Issue**: If a compensation action itself fails (e.g., UNLOCK_SOURCE fails because the source jurisdiction is unreachable), the migration enters a limbo state where compensation has been attempted but not completed, and there is no mechanism to retry or escalate.
- **Impact**: Asset permanently locked with no automated recovery path.
- **Remediation**: Implement a compensation retry queue with exponential backoff and a human escalation trigger after N failures. Estimated effort: 2-3 days.

**4.3 Corridor Fork Resolution — Timestamp Vulnerability**

- **File**: `governance/corridor.lifecycle.state-machine.v1.json`, spec Protocol 16.1
- **Issue**: The spec states "earlier-timestamped branch is presumptively valid" for fork resolution. The implementation doesn't include additional protections against timestamp manipulation. An attacker who can backdate timestamps can always win fork resolution.
- **Impact**: A malicious corridor participant could fork the receipt chain, backdate their branch, and have it accepted as canonical.
- **Remediation**: Add secondary ordering criteria (watcher attestation count, quorum diversity) to break timestamp ties, and implement a maximum clock skew tolerance. Estimated effort: 2 days.

### Medium-Severity Findings

**4.4 Entity Dissolution — No Time-Bounded Creditor Claims**

- **File**: `tools/lifecycle.py`
- **Issue**: The 10-stage entity dissolution state machine defines a creditor claims period (Stage 5) but does not enforce a time boundary. The implementation checks stage progression but not temporal deadlines.
- **Impact**: A dissolution could stall indefinitely at the creditor claims stage.

---

## PASS 5: Deployment Infrastructure & Operational Readiness

### High-Severity Findings

**5.1 Docker Services Reference Non-Existent Server Commands**

- **File**: `deploy/docker/docker-compose.yaml`
- **Issue**: Service definitions like `entity-registry` specify commands such as `python -m tools.msez entity-registry serve --port 8083`. The `tools/msez.py` CLI does not have an `entity-registry serve` subcommand. These are aspirational service definitions that will fail on `docker compose up`.
- **Impact**: The "one-click deployment" advertised in the README will not start any services.
- **Remediation**: Either implement the `serve` subcommands in msez.py, or restructure the Docker services to use the actual Mass API services (Spring Boot) as the runtime layer with the Python tools as a CLI sidecar. Estimated effort: 5-10 days depending on approach.

**5.2 Unpinned Dependencies**

- **File**: `tools/requirements.txt`
- **Issue**: All 5 dependencies (`pyyaml`, `jsonschema`, `lxml`, `pytest`, `cryptography`) are unpinned. A minor version bump in `jsonschema` (e.g., from 4.x to 5.x which changed default validator behavior) could break schema validation across the entire stack.
- **Impact**: Non-reproducible builds. A CI run today and a CI run tomorrow could produce different results.
- **Remediation**: Pin to exact versions with a `requirements.lock` file. Use `pip-compile` for reproducible resolution. Estimated effort: 0.5 days.

**5.3 Terraform — Security Group Review Needed**

- **File**: `deploy/aws/terraform/main.tf`
- **Issue**: Security group rules should be reviewed for over-permissive ingress. The Terraform configuration includes RDS instances — verify encryption at rest is enabled and KMS key rotation is configured.
- **Remediation**: Security audit of Terraform before any cloud deployment.

---

## PASS 6: Code Quality & Technical Debt

### High-Severity Findings

**6.1 Monolith: `tools/msez.py` at 15,472 Lines**

- **Issue**: This single file contains the entire CLI, all validation logic, artifact management, zone operations, corridor management, and more. It is larger than the entire phoenix layer combined.
- **Impact**: Maintenance burden, merge conflicts, onboarding friction, and inability to unit-test individual functions without loading the entire module.
- **Remediation**: Decompose into a package structure: `tools/msez/cli.py` (argument parsing), `tools/msez/validate.py` (zone validation), `tools/msez/corridor.py` (corridor operations), `tools/msez/artifact.py` (CAS operations), etc. The existing `tools/msez/` subpackage (composition.py, schema.py, core.py) shows the team already recognizes this need. Estimated effort: 5-8 days with backward-compatible CLI wrappers.

**6.2 Bare Exception Handling Across Phoenix**

- **Files**: `tools/phoenix/observability.py` (3 instances), `tools/phoenix/config.py` (1), `tools/phoenix/events.py` (1), `tools/phoenix/migration.py` (3)
- **Issue**: 8 instances of `except Exception:` without re-raise or logging across the phoenix layer. Cryptographic operations and state machine transitions should never silently fail.
- **Remediation**: Add structured logging to all exception handlers. For cryptographic operations, fail loudly with `raise SecurityViolation(...)`.

---

## PASS 7: Completeness & Gap Analysis

### Spec Coverage Matrix (Summary)

| Spec Area | Status | Notes |
|-----------|--------|-------|
| Smart Asset 5-tuple (G,R,M,C,H) | 🟡 Partial | Genesis + Registry implemented; Migration, Compliance, History are phoenix stubs |
| Compliance Tensor V2 | 🟡 Partial | 8/9 domains; tensor operations work but use non-JCS canonicalization |
| Compliance Manifold | 🟡 Partial | Differential geometry path optimization exists but not integrated with real attestation sources |
| Pack Trilogy (Law/Reg/License) | ✅ Implemented | Most complete area — lawpacks, regpacks, licensepacks all have working CLI + tests |
| Multi-Jurisdiction Composition | ✅ Implemented | 20 domains, compatibility rules, compose_zone factory — well done |
| Smart Asset VM | 🟡 Partial | 1,474 lines of instruction categories but mock execution engine |
| Migration Protocol | 🟡 Partial | 8-phase saga structure exists but no timeout, mock compensations |
| Watcher Economy | 🟡 Partial | Watcher profiles + slashing conditions defined; no real stake mechanics |
| Corridor Bridge | 🟡 Partial | PathRouter with Dijkstra exists; mock execution |
| L1 Anchoring | 🟡 Partial | Anchor structures exist; L1-optional design is correct; no chain integration |
| Receipt Chain + MMR | ✅ Implemented | Working MMR, checkpoint, fork resolution — this is solid |
| ZK Proof Systems | 🔴 Stub | Mock proofs only |
| Agentic Framework | ✅ Implemented | 1,686 lines; 20 trigger types; well-structured policy engine |
| Arbitration | ✅ Implemented | 1,217 lines; dispute lifecycle; evidence packages |
| OpenAPI Surface | 🔴 Scaffold | 4 skeleton specs covering ~5% of needed endpoints |

### Pakistan GovOS Deployment Gap Analysis

**What Exists Today (Ready for Phase 1 Pilot)**

The Pack Trilogy is the stack's strongest asset. The lawpack, regpack, and licensepack toolchain can ingest Pakistani statutes (Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act), model FBR tax calendars and SRO schedules, and track 15+ license categories. The `msez validate` and `msez lock` pipelines can verify zone configurations against schema constraints. The receipt chain and MMR infrastructure provides a solid audit trail foundation.

**What Must Be Built Before Deployment (Critical Path)**

Layer 04 — National System Integration represents the largest engineering gap. FBR IRIS integration requires a tax authority API adapter supporting NTN registration, return filing, and the IRIS XML schema. SBP Raast integration requires a real-time payment rail adapter for instant PKR settlement. NADRA integration requires CNIC cross-referencing capabilities (the Identity primitive has no API today). SECP integration requires corporate registry synchronization.

The Sovereign AI layer (Foundation Model, Tax Intelligence, Operational Intelligence, Regulatory Awareness, Forensic & Audit) is entirely net-new. There is no AI/ML infrastructure in the current stack. The Data Sovereignty component (on-premise GPU, Pakistani data centers) requires physical infrastructure procurement.

The Cross-Border Corridors require bilateral Pack Trilogy instances for each corridor (PAK↔KSA, PAK↔UAE, PAK↔CHN) covering customs duties, transfer pricing, and sanctions compliance specific to each bilateral relationship. The corridor bridge and routing infrastructure exists in code but needs real settlement rail integration (SWIFT adapter in `tools/integrations/` is a 15-line stub).

**What Can Be Deferred**

ZK proof systems, BBS+ selective disclosure, Poseidon2 CDB, and the full Watcher economy with real stake mechanics are Phase 2+ features. The Smart Asset VM (as opposed to the deterministic compliance evaluator) is not needed for tax compliance operations.

### Five Programmable Primitives — Architecture Coherence Assessment

The "five programmable primitives" sales line (ENTITIES, OWNERSHIP, FISCAL, IDENTITY, CONSENT) is well-supported by the current architecture, with the following coherence assessment:

**ENTITIES** is the strongest primitive. Organization Info API provides formation, lifecycle, beneficial ownership. The SEZ Stack's corporate module family (8 modules), legal module family (9 modules), and the entity dissolution state machine in lifecycle.py provide robust support. The gap is SECP integration for Pakistan-specific corporate registry requirements.

**OWNERSHIP** is well-served by the Investment Info API (cap tables, share classes, investor onboarding) and the Smart Asset registry VC system. The gap is capital gains tracking at transfer, which the Tax module family describes but doesn't implement.

**FISCAL** has a strong conceptual foundation in the Treasury Info API and the Tax module family (which has the most YAML descriptors of any family). The gap is the actual tax calculation engine — the system can describe tax obligations but cannot yet compute withholding amounts from transaction data, which is the core value proposition for the FBR.

**IDENTITY** is the weakest primitive. There is no Identity API in the Mass API surface, no NADRA integration adapter, and no passport/KYB verification workflow. The GovOS diagram shows this primitive requiring "Passport/KYC/KYB, NTN linkage, cross-reference NADRA" — all of which are net-new.

**CONSENT** is well-served by the Consent Info API (multi-party authorization, audit trails, tax assessment sign-off). This primitive is closest to production-ready for the Pakistan deployment, where tax assessment consent workflows are a core requirement.

---

## Prioritized Remediation Roadmap

### Tier 1: Production Blockers (Fix Before Any Deployment)

1. **Canonicalization unification** — Replace all `json.dumps(sort_keys=True)` in phoenix with `jcs_canonicalize()`. 2 days.
2. **Dependency pinning** — Pin all 5 dependencies to exact versions. 0.5 days.
3. **Schema hardening** — Set `additionalProperties: false` on all VC and receipt schemas. 2 days.
4. **Exception handling** — Replace bare `except Exception:` with structured logging across phoenix. 1 day.

### Tier 2: Pakistan Phase 1 Prerequisites (Fix Before Pakistan Pilot)

5. **Identity API** — Build the Identity primitive API with NADRA CNIC cross-referencing. 15-20 days.
6. **Tax calculation engine** — Implement withholding computation from transaction events. 10-15 days.
7. **Corridor state machine alignment** — Align to spec or formally amend spec. 1 day.
8. **Docker deployment** — Implement actual `serve` subcommands or restructure to use Mass API services. 5-10 days.
9. **Migration timeout enforcement** — Add deadline-based automatic compensation. 2 days.
10. **Compliance tensor domain expansion** — Add LICENSING to the tensor enum. 1 day.

### Tier 3: Production Hardening (Fix Before Scale)

11. **ThreadSafeDict iteration safety** — Override `__iter__`, `keys()`, `values()`, `items()`. 0.5 days.
12. **msez.py decomposition** — Break 15K-line monolith into package structure. 5-8 days.
13. **Compensation retry queue** — Implement retry with exponential backoff for failed compensations. 2-3 days.
14. **Fork resolution timestamp hardening** — Add secondary ordering criteria. 2 days.
15. **OpenAPI expansion** — Build full API specs for all five primitives plus corridors. 10 days.

### Tier 4: Phase 2+ (Track)

16. Poseidon2 CDB implementation
17. ZK proof system integration (Groth16/PLONK/STARK)
18. BBS+ selective disclosure
19. Watcher economy with real stake mechanics
20. Sovereign AI layer architecture

---

## Strengths Acknowledged

The SEZ Stack has several genuine strengths that should be preserved and amplified. The Pack Trilogy is production-grade — the lawpack, regpack, and licensepack toolchain represents the most complete open-source implementation of machine-readable jurisdictional configuration that exists. The content-addressed artifact store with SHA256 integrity verification is clean and correct (modulo the canonicalization issue). The specification itself is an intellectual achievement — 48 chapters covering every aspect of programmable jurisdiction design with mathematical rigor. The test suite at 87 files with 263+ test functions shows serious engineering investment in correctness. The architecture's L1-optional design is strategically correct for sovereign deployments that may not want blockchain dependencies. The agentic policy framework (1,686 lines, 20 trigger types, 7 standard policies) is a genuinely novel contribution to programmable compliance.

The core question for the Pakistan deployment is not "is this good?" — it is. The question is "how do we bridge the gap between a world-class reference implementation and world-class sovereign infrastructure?" This audit provides the engineering roadmap to do so.
