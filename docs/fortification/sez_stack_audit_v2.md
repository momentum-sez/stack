# SEZ Stack v0.4.44 GENESIS — Institutional Audit & Rust Migration Architecture

**Classification:** Momentum Internal — Principal Architecture Document
**Version:** 2.0
**Date:** February 12, 2026
**Author:** Office of the Managing Partner
**Repository:** `momentum-sez/stack` at `main` branch
**Scope:** Seven-pass audit of the current Python implementation, complete Rust migration architecture, API services design, and tiered execution roadmap

---

## Preface

This document serves three simultaneous purposes. First, it is the complete institutional-grade audit of the SEZ Stack v0.4.44 GENESIS codebase — every critical finding, every production blocker, every edge case discovered through systematic seven-pass analysis. Second, it is the architectural specification for a full Rust migration that eliminates entire classes of defects discovered in the audit by making them structurally impossible at compile time. Third, it is the operational CLAUDE.md that will drive the implementation — a machine-readable execution contract for the engineering team and for AI-assisted development.

The core insight motivating this document is that the audit findings are not primarily implementation bugs. They are *language-level defect classes* — categories of error that Python permits by design and that Rust prevents by design. The canonicalization split, the state machine divergence, the swallowed exceptions, the domain mismatch — each traces to a specific property that Rust's type system enforces and Python's cannot. A Rust migration is not a rewrite for performance. It is a rewrite for *correctness guarantees* at the level required for sovereign digital infrastructure serving nation-states.

---

## Part I: Current State Assessment

### 1.1 Repository Structure

The SEZ Stack is the open-source implementation of Momentum's "Special Economic Zones in a Box" thesis — programmable jurisdictional infrastructure that transforms incorporation, compliance, taxation, and cross-border trade into software primitives. The codebase represents the most complete open-source implementation of machine-readable jurisdictional configuration that exists.

```
momentum-sez/stack/                     v0.4.44-GENESIS
├── apis/                               4 OpenAPI scaffold specs (~5% coverage)
│   ├── smart-assets.openapi.yaml       Smart Asset CRUD, compliance eval, anchor verify
│   ├── corridor-state.openapi.yaml     Corridor receipts, forks, anchors, finality
│   ├── mass-node.openapi.yaml          Zone-to-Mass integration (2 endpoints)
│   └── regulator-console.openapi.yaml  Regulator query access (1 endpoint)
├── deploy/
│   ├── docker/                         Compose (12 services), init-db.sql, prometheus
│   ├── aws/terraform/                  VPC, EKS, RDS, KMS (main.tf: 545L, k8s: 705L)
│   └── scripts/deploy-zone.sh          7-step deployment (255 lines)
├── dist/                               Content-addressed artifact store
│   ├── artifacts/                      CAS: {type}/{digest}.json naming
│   ├── lawpacks/                       Compiled lawpack bundles
│   └── registries/                     Compiled registry snapshots
├── governance/
│   └── corridor.lifecycle.state-machine.v1.json    ← DEFECTIVE state names
├── modules/                            583 YAML descriptors, 16 families, 0 Python
│   ├── index.yaml                      Claims 146/146 (100%) — means descriptors
│   └── {family}/{module}/module.yaml
├── schemas/                            116 JSON schemas (Draft 2020-12 target)
├── spec/                               48 chapters — ground truth for all decisions
├── tests/                              87 test files, 1.2M total, 263+ test functions
└── tools/                              ALL executable code (Python)
    ├── msez.py                         15,472L CLI monolith
    ├── smart_asset.py                  839L — Smart Asset primitives (uses JCS ✓)
    ├── vc.py                           436L — Verifiable Credential signing (uses JCS ✓)
    ├── lawpack.py                      698L — jcs_canonicalize() SOURCE OF TRUTH
    ├── regpack.py                      620L — Regpack operations
    ├── licensepack.py                  1,179L — Licensepack lifecycle
    ├── agentic.py                      1,686L — Agentic policy engine (20 triggers)
    ├── arbitration.py                  1,217L — Dispute lifecycle
    ├── mass_primitives.py              1,771L — Five primitives implementation
    ├── mmr.py                          326L — Merkle Mountain Range
    ├── lifecycle.py                    245L — Entity dissolution state machine
    ├── netting.py                      559L — Settlement netting
    ├── artifacts.py                    219L — CAS store/resolve
    ├── requirements.txt                5 UNPINNED dependencies
    ├── msez/                           Subpackage (composition, schema, core)
    │   ├── composition.py              652L — Multi-zone composition (20 domains)
    │   ├── schema.py                   285L — Schema validation
    │   └── core.py                     222L — Core types
    └── phoenix/                        PHOENIX Smart Asset OS (14,363 lines)
        ├── __init__.py                 514L — Layer architecture
        ├── tensor.py                   1,092L — Compliance Tensor (8 domains)
        ├── manifold.py                 1,020L — Compliance Manifold
        ├── vm.py                       1,474L — Smart Asset VM (mock)
        ├── migration.py                933L — Migration saga
        ├── bridge.py                   829L — Corridor bridge (Dijkstra)
        ├── anchor.py                   819L — L1 anchoring
        ├── zkp.py                      809L — ZK proofs (ALL MOCKED)
        ├── watcher.py                  753L — Watcher economy
        ├── security.py                 997L — Defense-in-depth
        ├── hardening.py                797L — Validators, ThreadSafeDict
        ├── resilience.py               1,045L — Circuit breaker, retry
        ├── events.py                   1,069L — Event bus, saga orchestration
        ├── runtime.py                  1,063L — Runtime context
        ├── cache.py                    1,064L — LRU/TTL/tiered cache
        ├── health.py                   551L — K8s probes
        ├── observability.py            537L — Structured logging
        ├── config.py                   491L — YAML/env config
        └── cli.py                      506L — Phoenix CLI
```

### 1.2 Quantitative Summary

The codebase totals approximately 37,000 lines of Python across the core tools and phoenix layer, 583 YAML module descriptors totaling 5.5M of jurisdictional configuration, 116 JSON schemas at 435K, 48 specification chapters, and 87 test files at 1.2M. The CI pipeline runs validation of all modules, profiles, and zones, deterministic byte-level verification of trade playbook artifacts, and the full pytest suite.

---

## Part II: Critical Audit Findings

Each finding below is presented with its root cause in the current Python implementation, the severity and impact assessment, and — critically — the Rust type-system property that would have prevented the defect from existing. This dual framing is the evidentiary basis for the migration decision.

### 2.1 CRITICAL: Canonicalization Split — CDB Violation

**Affected Files:** 17 phoenix files (security.py, tensor.py, zkp.py, anchor.py, bridge.py, migration.py, watcher.py, events.py, observability.py) at 17+ distinct locations.

**Finding:** The core layer (smart_asset.py, vc.py, lawpack.py) computes all content-addressed digests using `jcs_canonicalize()` from `tools/lawpack.py`, which applies `_coerce_json_types()` preprocessing: rejection of floats, normalization of datetimes to UTC ISO8601 with Z suffix, coercion of non-string dict keys to strings, and conversion of tuples to lists. The entire phoenix layer computes digests using bare `json.dumps(content, sort_keys=True, separators=(",", ":"))` without any preprocessing.

**Impact:** Same data produces different SHA256 digests depending on which module computes them. A compliance tensor commitment computed by tensor.py will not match verification by smart_asset.py. For a content-addressed system where digest equality is the foundational trust primitive, this is a systemic integrity failure. Every cross-layer verification path is potentially compromised.

**Root Cause in Python:** Python has no way to distinguish "bytes produced by JCS canonicalization" from "bytes produced by json.dumps." Both are `bytes`. The function signature `hashlib.sha256(data: bytes)` accepts either without complaint. There is no type-level encoding of byte provenance.

**Prevention in Rust:** A newtype wrapper makes the defect class structurally impossible:

```rust
/// Bytes produced exclusively by JCS-compatible canonicalization.
/// The only constructor is `CanonicalBytes::new()`, which applies
/// full type coercion before serialization.
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// The sole construction path — enforces JCS preprocessing.
    pub fn new(obj: &impl Serialize) -> Result<Self, CanonicalizationError> {
        let coerced = coerce_json_types(obj)?;
        Ok(Self(serde_jcs::to_vec(&coerced)?))
    }
}

/// Content-addressed digest. Can ONLY be computed from CanonicalBytes.
pub fn sha256_digest(data: &CanonicalBytes) -> ContentDigest {
    ContentDigest(Sha256::digest(&data.0).into())
}
```

Every function in the codebase that computes a digest would require a `CanonicalBytes` argument. Passing raw `serde_json::to_vec()` output would be a compile error. The entire class of "wrong serialization path" becomes impossible, not merely unlikely. Seventeen files, seventeen instances — none could exist.

### 2.2 CRITICAL: Poseidon2 Hash — Specified but Not Implemented

**Specification:** The CDB (Canonical Digest Bridge) is defined as `CDB(A) = Poseidon2(Split256(SHA256(JCS(A))))`.

**Implementation:** Only SHA256 is used anywhere. The single "Poseidon" reference is a pseudocode comment in mass_primitives.py:748.

**Impact:** All ZK proof circuits reference SHA256 digests, not Poseidon2. If a production ZK backend were connected, every existing commitment would be invalid. The Poseidon2 hash is ZK-friendly (arithmetic-circuit-native), making it essential for efficient proof generation over compliance states.

**Rust Ecosystem Advantage:** The reference Poseidon2 implementations are Rust-native: `poseidon2-rs`, `neptune` (used by Filecoin), and the `plonky2` Poseidon implementation used by Polygon. Integration into a Rust codebase is a direct dependency addition with zero FFI overhead. In the current Python stack, this would require either fragile C FFI bindings or a pure-Python reimplementation that defeats the performance purpose of a ZK-friendly hash.

**Phase Strategy:** SHA256-only is acceptable for Phase 1 deterministic compliance evaluation. Poseidon2 activates in Phase 2 with the ZK proof system. The Rust migration should define `DigestAlgorithm` as an enum from day one, with all commitment structures carrying an algorithm tag for forward migration:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    Sha256,
    Poseidon2,
}

pub struct ContentDigest {
    pub algorithm: DigestAlgorithm,
    pub bytes: [u8; 32],
}
```

### 2.3 CRITICAL: Corridor State Machine — Spec Divergence

**Specification:** `DRAFT → PENDING → ACTIVE` with `HALTED` and `SUSPENDED` branches.

**Implementation (governance/corridor.lifecycle.state-machine.v1.json):** `PROPOSED → OPERATIONAL → HALTED → DEPRECATED`.

**Impact:** Different state names, different transitions, missing states (PENDING and SUSPENDED absent from implementation). Any external system referencing corridor states by name — including the Mass API layer, regulator console, and cross-border corridor agreements — will encounter mismatches. This is not a cosmetic issue; corridor state names appear in legally binding bilateral agreements between sovereign jurisdictions.

**Root Cause in Python:** State names are strings. The JSON state machine definition uses string-typed state identifiers with no compile-time link to the code that processes them. The `msez.py` CLI can reference `"OPERATIONAL"` and the spec can say `"ACTIVE"` and nothing detects the conflict until a production corridor transition fails.

**Prevention in Rust — Typestate Pattern:**

```rust
// Each state is a distinct type. Invalid transitions are compile errors.
pub struct Draft;
pub struct Pending;
pub struct Active;
pub struct Halted;
pub struct Suspended;
pub struct Deprecated;

pub struct Corridor<S: CorridorState> {
    id: CorridorId,
    jurisdictions: (JurisdictionId, JurisdictionId),
    pack_trilogy: PackTrilogyRef,
    _state: PhantomData<S>,
}

impl Corridor<Draft> {
    pub fn submit(self, evidence: SubmissionEvidence) -> Result<Corridor<Pending>, CorridorError> {
        // Can ONLY produce Corridor<Pending>. Cannot produce Corridor<Active>.
        Ok(Corridor { _state: PhantomData, ..self.transmute() })
    }
}

impl Corridor<Pending> {
    pub fn activate(self, evidence: ActivationEvidence) -> Result<Corridor<Active>, CorridorError> {
        Ok(Corridor { _state: PhantomData, ..self.transmute() })
    }
}

impl Corridor<Active> {
    pub fn halt(self, reason: HaltReason) -> Result<Corridor<Halted>, CorridorError> { /* ... */ }
    pub fn suspend(self, reason: SuspendReason) -> Result<Corridor<Suspended>, CorridorError> { /* ... */ }
}

// Corridor<Draft> has no .halt() method. Calling it is a compile error.
// There is no string "OPERATIONAL" anywhere in the system.
```

### 2.4 CRITICAL: Compliance Tensor Domain Mismatch

**Finding:** `tools/phoenix/tensor.py` defines 8 compliance domains (AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY, DATA_PRIVACY). `tools/msez/composition.py` defines 20 domains (adding LICENSING, BANKING, PAYMENTS, CLEARING, SETTLEMENT, DIGITAL_ASSETS, EMPLOYMENT, IMMIGRATION, IP, CONSUMER_PROTECTION, ARBITRATION, and more).

**Impact:** The composition module can reference compliance domains that the tensor cannot materialize. Pakistan's deployment requires 15+ license categories — the LICENSING domain in particular. Compliance evaluation for licensing will silently return no results rather than failing loudly.

**Root Cause in Python:** Two independent enum definitions with no compile-time enforcement that they stay synchronized. Python's `Enum` has no exhaustive match requirement — a `match` statement (or if/elif chain) that handles 8 of 20 cases runs without warning.

**Prevention in Rust:**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComplianceDomain {
    Aml, Kyc, Sanctions, Tax, Securities, Corporate,
    Custody, DataPrivacy, Licensing, Banking, Payments,
    Clearing, Settlement, DigitalAssets, Employment,
    Immigration, Ip, ConsumerProtection, Arbitration, Trade,
}

fn evaluate_domain(domain: ComplianceDomain) -> ComplianceState {
    match domain {
        ComplianceDomain::Aml => { /* ... */ }
        ComplianceDomain::Kyc => { /* ... */ }
        // Compiler error: non-exhaustive patterns.
        // EVERY variant must be handled.
    }
}
```

A single enum definition. One source of truth. Adding a new domain forces every `match` expression in the entire codebase to handle it. The audit finding becomes a compiler error.

### 2.5 CRITICAL: ZKP System — Entirely Mocked

**File:** `tools/phoenix/zkp.py`

**Finding:** All proof generation uses `secrets.token_hex(32)` and `hashlib.sha256()` for "deterministic mock proofs" (lines 515, 525). The spec describes 5 NIZK systems (Groth16, PLONK, STARK, Bulletproofs, Halo2) with 12 circuit types. None have real cryptographic implementations.

**Impact:** Zero actual zero-knowledge privacy guarantees exist. The system is currently deterministic-transparent, not ZK-private. Acceptable for Phase 1 (Pakistan tax compliance does not require ZK for its initial deployment), but must be explicitly acknowledged and scoped.

**Rust Ecosystem Advantage:** Every serious ZK proof library in production is Rust-native. The migration provides direct access without FFI:

| System | Rust Crate | Used By |
|--------|-----------|---------|
| Groth16 | `arkworks/groth16` | Zcash, Aleo |
| PLONK | `halo2_proofs` | Zcash Orchard, Scroll |
| STARK | `plonky2`, `winterfell` | Polygon, StarkWare |
| Bulletproofs | `bulletproofs` | Monero, Mimblewimble |
| Halo2 | `halo2_proofs` | Zcash, Privacy pools |

The Rust implementation should define a trait-based proof system abstraction from day one:

```rust
pub trait ProofSystem: Send + Sync {
    type Proof: Serialize + DeserializeOwned;
    type VerifyingKey: Clone;
    type ProvingKey;
    type Circuit: Clone;

    fn prove(&self, pk: &Self::ProvingKey, circuit: &Self::Circuit) -> Result<Self::Proof, ProofError>;
    fn verify(&self, vk: &Self::VerifyingKey, proof: &Self::Proof, public_inputs: &[Fr]) -> Result<bool, VerifyError>;
}

// Phase 1: Mock implementation (deterministic, transparent)
pub struct MockProofSystem;
impl ProofSystem for MockProofSystem { /* ... */ }

// Phase 2: Real implementations
pub struct Groth16System;
impl ProofSystem for Groth16System { /* arkworks integration */ }

pub struct PlonkSystem;
impl ProofSystem for PlonkSystem { /* halo2 integration */ }
```

### 2.6 CRITICAL: Module Implementation Gap

**Finding:** The `modules/` directory contains 583 YAML descriptors across 16 families and zero Python implementation files. `modules/index.yaml` claims "146/146 modules (100%)." All business logic lives in `tools/msez.py` (15,472 lines) and the phoenix layer.

**Clarification:** This is architecturally intentional. The YAML descriptors drive the `msez validate` and `msez lock` pipelines. Business logic execution is delegated to the Mass API layer. The descriptor/implementation distinction must be crystal clear in all documentation, but the architecture itself is sound.

**Rust Migration Consideration:** The YAML descriptors remain as data files. The Rust implementation replaces the Python tools that parse and validate them. `serde_yaml` provides zero-copy deserialization into strongly-typed Rust structs, with schema validation enforced at the type level rather than by a JSON Schema validator at runtime.

---

## Part III: High-Severity Findings

### 3.1 Schema Security — `additionalProperties: true` on Security-Critical Schemas

**Affected:** `schemas/vc.smart-asset-registry.schema.json` (7 instances), `schemas/corridor.receipt.schema.json` (4 instances), plus attestation, checkpoint, and fork-resolution schemas.

**Impact:** Schema injection attacks are possible. A malicious corridor receipt could include arbitrary fields that downstream processors interpret as authorization signals. All VC envelope schemas, proof array element schemas, and receipt schemas must set `additionalProperties: false`. Only `credentialSubject`, `metadata`, and `extensions` objects should remain extensible per the W3C VC specification.

### 3.2 OpenAPI Scaffolds — ~5% Endpoint Coverage

**Finding:** All 4 API specs in `apis/` are explicitly labeled "scaffold" or "skeleton." The mass-node API has 2 endpoints. The regulator-console API has 1 endpoint. The GovOS architecture diagram specifies 5 experience-layer portals requiring full API surfaces.

**Rust Impact:** The migration replaces these YAML scaffolds with Axum router definitions where every endpoint is type-checked at compile time. Request/response types are generated from the schema definitions, ensuring the API surface and the data model cannot diverge.

### 3.3 Mass API ↔ Five Primitives Mapping

This mapping is critical for the Rust migration because it determines the API service boundaries:

**ENTITIES** (Organization Info API): Well-served. Entity formation, lifecycle, beneficial ownership, dissolution state machine. Gap: SECP integration for Pakistan corporate registry.

**OWNERSHIP** (Investment Info API): Cap tables, share classes functional. Gap: Capital gains tracking at transfer — the system can record transfers but cannot compute withholding amounts from transaction data.

**FISCAL** (Treasury Info API + Tax modules): Strong foundation. Gap: Actual tax calculation engine. The system can describe obligations but cannot compute withholding amounts, file returns, or reconcile with FBR IRIS.

**IDENTITY**: Weakest primitive. No Identity API exists in the Mass surface. No NADRA integration, no passport/KYB workflow. The GovOS architecture requires "Passport/KYC/KYB, NTN linkage, cross-reference NADRA" — all net-new construction.

**CONSENT** (Consent Info API): Closest to production-ready. Multi-party authorization, audit trails, tax assessment sign-off all functional.

### 3.4 Cryptographic Defects

**ThreadSafeDict** (`tools/phoenix/hardening.py:500-543`): Wraps individual operations with RLock but does not override `__iter__`, `keys()`, `values()`, `items()`. Concurrent iteration while another thread mutates will produce `RuntimeError` or silent state corruption.

**Rust Equivalent:** `Arc<RwLock<HashMap<K, V>>>` provides correct concurrent access by construction. Iterating requires acquiring a read lock, which blocks writers. The borrow checker enforces that you cannot hold a mutable reference and an immutable reference simultaneously.

**Swallowed Exceptions** (`tools/phoenix/migration.py:627,649,664`): Compensation saga catches `except Exception:` and silently sets `*_success = False` without logging. A production migration failure will have zero diagnostic information.

**Rust Equivalent:** `Result<T, E>` must be explicitly handled. The `#[must_use]` attribute on `Result` produces a compiler warning if an error is ignored. `except Exception:` has no Rust equivalent.

**BBS+ Signatures:** Specified for selective disclosure of VC claims. No implementation exists — only Ed25519 in `tools/vc.py`. The `bbs` crate provides a Rust-native BBS+ implementation.

### 3.5 State Machine Edge Cases

**Migration Timeout:** `MigrationSaga` defines a `deadline` field with no enforcement. A migration stuck in TRANSIT remains frozen indefinitely, creating permanent asset lock.

**Compensation Failure:** If a compensation action fails (e.g., UNLOCK_SOURCE fails because the source jurisdiction is unreachable), the migration enters limbo with no retry or escalation mechanism.

**Fork Resolution Timestamp Vulnerability:** "Earlier-timestamped branch is presumptively valid" with no secondary ordering. An attacker who backdates timestamps always wins fork resolution. Requires secondary criteria (watcher attestation count, quorum diversity) and maximum clock skew tolerance.

### 3.6 Deployment Infrastructure

**Docker Compose — Non-Functional:** Services reference `serve` subcommands (e.g., `python -m tools.msez entity-registry serve --port 8083`) that do not exist in the msez.py CLI. `docker compose up` will fail on every service.

**Unpinned Dependencies:** All 5 Python dependencies (pyyaml, jsonschema, lxml, pytest, cryptography) are unpinned. A jsonschema 4.x→5.x bump changed default validator behavior — builds are non-reproducible.

---

## Part IV: Strengths to Preserve

The audit is not purely critical. Several components represent genuine innovations that the Rust migration must preserve and elevate.

**Pack Trilogy (Lawpack, Regpack, Licensepack):** The most complete open-source implementation of machine-readable jurisdictional configuration in existence. Production-grade. The Rust migration must replicate this fidelity.

**Content-Addressed Artifact Store:** Clean SHA256 integrity verification with CAS naming convention. The design is correct; only the canonicalization inconsistency undermines it.

**48-Chapter Specification:** An intellectual achievement covering every aspect of programmable jurisdiction design with mathematical rigor. The Rust implementation must be traceable to these chapters.

**Test Suite:** 87 files with 263+ test functions. Serious engineering investment in correctness. The Rust migration must achieve equivalent or superior coverage from day one using the same test scenarios expressed as Rust integration tests.

**L1-Optional Design:** Strategically correct for sovereign deployments that may resist blockchain dependencies. The Rust implementation must preserve this optionality through trait abstractions.

**Agentic Policy Framework:** 1,686 lines, 20 trigger types, 7 standard policies. A genuinely novel contribution to programmable compliance.

**Receipt Chain + MMR:** Working Merkle Mountain Range implementation, checkpoint mechanism, fork resolution. Solid infrastructure.

**Multi-Jurisdiction Composition:** 20 domains, compatibility rules, compose_zone factory. Well-designed.

---

## Part V: The Rust Migration Architecture

### 5.1 Language Decision Rationale

The migration to Rust is not a performance decision. It is a correctness decision driven by the specific defect classes this audit revealed. Every critical finding maps to a Rust type-system property:

| Audit Finding | Python Defect Class | Rust Prevention Mechanism |
|--------------|---------------------|--------------------------|
| Canonicalization split | No byte provenance tracking | Newtype `CanonicalBytes` — constructor-enforced |
| State machine divergence | String-typed states | Typestate pattern — `Corridor<Draft>` vs `Corridor<Active>` |
| Domain mismatch | Independent enum definitions | Single enum + exhaustive `match` |
| Swallowed exceptions | `except Exception:` | `Result<T, E>` + `#[must_use]` |
| Thread safety gaps | Manual lock management | `Arc<RwLock<T>>` + borrow checker |
| ZKP mock system | No trait enforcement | `trait ProofSystem` — compile-time interface contract |
| Migration timeout | Unchecked deadline field | Builder pattern with required `deadline` validation |

Additionally, the Rust ecosystem provides native access to the entire ZK proof stack (arkworks, halo2, plonky2), the Poseidon2 hash function, BBS+ signatures, and Ed25519 — all without FFI boundaries.

### 5.2 API Services: Axum on Tokio/Tower/Hyper

Going full Rust, the most elegant API services choice is **Axum**. This is not a compromise — it is the optimal architecture for a system where type safety must extend from the HTTP handler through to the cryptographic proof generator without a single dynamic dispatch boundary.

**Why Axum over alternatives:**

Axum is built on the Tokio async runtime, the Tower middleware framework, and the Hyper HTTP library. It provides type-safe request extraction (path parameters, query strings, JSON bodies, headers all decoded into typed Rust structs at compile time), Tower middleware composability (authentication, rate limiting, tracing, metrics all stack as typed layers), and native async without the colored-function problem. It is production-proven at Cloudflare, Discord, and other infrastructure-critical deployments.

The critical advantage over introducing a JVM layer (Kotlin/Spring Boot) is elimination of the FFI boundary. The cryptographic core, the state machines, and the API handlers all live in the same process, the same type system, the same memory model. A `CanonicalBytes` value computed in the crypto layer can be passed directly to the API response serializer without marshaling through JNI. A `Corridor<Active>` returned by the state machine can be pattern-matched in the handler without converting to a string representation.

**Axum service architecture for the five primitives:**

```rust
// Each primitive is an Axum Router with typed extractors and responses.
pub fn entities_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/entities", post(create_entity).get(list_entities))
        .route("/v1/entities/:id", get(get_entity).put(update_entity))
        .route("/v1/entities/:id/beneficial-owners", get(beneficial_owners))
        .route("/v1/entities/:id/dissolution/initiate", post(initiate_dissolution))
        .route("/v1/entities/:id/dissolution/status", get(dissolution_status))
        .with_state(state)
}

// Request/response types are compile-time contracts.
async fn create_entity(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateEntityRequest>,
) -> Result<Json<EntityResponse>, AppError> {
    // req is fully validated by the type system.
    // AppError implements IntoResponse with structured error bodies.
    let entity = state.entity_service.create(req).await?;
    Ok(Json(entity.into()))
}
```

**Supporting Rust crates for the API layer:**

| Concern | Crate | Rationale |
|---------|-------|-----------|
| HTTP framework | `axum` 0.8+ | Type-safe extractors, Tower middleware, Tokio runtime |
| Serialization | `serde` + `serde_json` | Zero-copy where possible, derive macros |
| Database | `sqlx` | Compile-time SQL query verification against live schema |
| Validation | `validator` | Derive-based struct validation |
| Auth | `axum-extra` + `jsonwebtoken` | JWT/Bearer token extraction |
| Tracing | `tracing` + `tracing-subscriber` | Structured logging, distributed tracing |
| Metrics | `metrics` + `metrics-exporter-prometheus` | Prometheus-compatible |
| OpenAPI gen | `utoipa` | Derive-based OpenAPI 3.1 spec generation from types |
| gRPC (inter-service) | `tonic` | Protobuf-based, Tokio-native |
| Config | `config` | Layered config from files, env, CLI |
| Error handling | `thiserror` | Derive-based error types with Display |
| Testing | `axum-test` + `sqlx::test` | HTTP-level integration tests with DB fixtures |

### 5.3 Crate Architecture — Workspace Layout

The Rust implementation uses a Cargo workspace with clearly delineated crate boundaries. Each crate has a single responsibility and a defined dependency direction.

```
msez/
├── Cargo.toml                          # Workspace root
├── Cargo.lock                          # Exact dependency pins (committed)
│
├── crates/
│   ├── msez-core/                      # FOUNDATIONAL — no external deps except serde
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── canonical.rs            # CanonicalBytes, jcs_canonicalize, type coercion
│   │   │   ├── digest.rs               # ContentDigest, DigestAlgorithm (SHA256/Poseidon2)
│   │   │   ├── domain.rs              # ComplianceDomain (single enum, 20 variants)
│   │   │   ├── jurisdiction.rs         # JurisdictionId, JurisdictionConfig
│   │   │   ├── identity.rs             # DID, CNIC, NTN, PassportNumber
│   │   │   ├── temporal.rs             # Timestamp (UTC-only, Z-suffix, seconds precision)
│   │   │   └── error.rs               # MsezError hierarchy
│   │   └── Cargo.toml                  # Deps: serde, thiserror
│   │
│   ├── msez-crypto/                    # CRYPTOGRAPHIC PRIMITIVES
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── ed25519.rs              # Ed25519 signing/verification
│   │   │   ├── bbs.rs                  # BBS+ selective disclosure (Phase 2)
│   │   │   ├── poseidon.rs             # Poseidon2 hash (Phase 2, trait-gated)
│   │   │   ├── mmr.rs                  # Merkle Mountain Range
│   │   │   └── cas.rs                  # Content-Addressed Storage
│   │   └── Cargo.toml                  # Deps: msez-core, ed25519-dalek, sha2
│   │
│   ├── msez-vc/                        # VERIFIABLE CREDENTIALS
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── credential.rs           # VC structure, signing, verification
│   │   │   ├── proof.rs                # Proof types (Ed25519, BBS+)
│   │   │   └── registry.rs            # Smart Asset Registry VC
│   │   └── Cargo.toml                  # Deps: msez-core, msez-crypto
│   │
│   ├── msez-state/                     # STATE MACHINES (typestate-encoded)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── corridor.rs             # Corridor<Draft|Pending|Active|Halted|Suspended>
│   │   │   ├── migration.rs            # MigrationSaga with deadline enforcement
│   │   │   ├── entity.rs               # Entity lifecycle (10-stage dissolution)
│   │   │   ├── license.rs              # License lifecycle
│   │   │   └── watcher.rs              # Watcher bonding/slashing state machine
│   │   └── Cargo.toml                  # Deps: msez-core
│   │
│   ├── msez-tensor/                    # COMPLIANCE TENSOR & MANIFOLD
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── tensor.rs               # ComplianceTensor<J> (generic over jurisdiction)
│   │   │   ├── manifold.rs             # Path optimization over tensor space
│   │   │   ├── commitment.rs           # Tensor commitment (CanonicalBytes → Digest)
│   │   │   └── evaluation.rs           # Domain evaluation logic
│   │   └── Cargo.toml                  # Deps: msez-core, msez-crypto
│   │
│   ├── msez-zkp/                       # ZERO-KNOWLEDGE PROOF SYSTEM
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs               # ProofSystem trait definition
│   │   │   ├── mock.rs                 # MockProofSystem (Phase 1)
│   │   │   ├── groth16.rs              # Groth16System (Phase 2, feature-gated)
│   │   │   ├── plonk.rs                # PlonkSystem (Phase 2, feature-gated)
│   │   │   ├── circuits/               # 12 circuit definitions
│   │   │   │   ├── compliance.rs       # Compliance attestation circuits
│   │   │   │   ├── migration.rs        # Migration evidence circuits
│   │   │   │   ├── identity.rs         # Identity verification circuits
│   │   │   │   └── settlement.rs       # Settlement proof circuits
│   │   │   └── cdb.rs                  # Canonical Digest Bridge
│   │   └── Cargo.toml                  # Deps: msez-core, msez-crypto
│   │                                   # Optional: ark-groth16, halo2_proofs
│   │
│   ├── msez-pack/                      # PACK TRILOGY
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── lawpack.rs              # Lawpack: statute → machine-readable rules
│   │   │   ├── regpack.rs              # Regpack: regulatory requirement sets
│   │   │   ├── licensepack.rs          # Licensepack: license lifecycle management
│   │   │   ├── parser/                 # YAML/JSON/Akoma Ntoso parsers
│   │   │   └── validation.rs           # Pack validation rules
│   │   └── Cargo.toml                  # Deps: msez-core, serde_yaml
│   │
│   ├── msez-corridor/                  # CORRIDOR OPERATIONS
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── bridge.rs               # Dijkstra routing, fee computation
│   │   │   ├── receipt.rs              # Corridor receipt chain
│   │   │   ├── fork.rs                 # Fork detection + resolution
│   │   │   ├── anchor.rs               # L1 anchoring
│   │   │   ├── netting.rs              # Settlement netting engine
│   │   │   └── swift.rs                # SWIFT pacs.008 adapter
│   │   └── Cargo.toml                  # Deps: msez-core, msez-state, msez-crypto
│   │
│   ├── msez-agentic/                   # AGENTIC POLICY ENGINE
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── policy.rs               # Policy definition (20 trigger types)
│   │   │   ├── evaluation.rs           # Policy evaluation engine
│   │   │   ├── scheduler.rs            # Action scheduling
│   │   │   └── audit.rs                # Policy audit trail
│   │   └── Cargo.toml                  # Deps: msez-core, msez-tensor
│   │
│   ├── msez-arbitration/               # DISPUTE RESOLUTION
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── dispute.rs              # Dispute lifecycle
│   │   │   ├── evidence.rs             # Evidence package management
│   │   │   ├── escrow.rs               # Escrow operations
│   │   │   └── enforcement.rs          # Award enforcement + receipts
│   │   └── Cargo.toml                  # Deps: msez-core, msez-state
│   │
│   ├── msez-schema/                    # SCHEMA VALIDATION & CODEGEN
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── validate.rs             # Runtime schema validation (for YAML modules)
│   │   │   └── codegen.rs              # Schema → Rust type generation (build.rs)
│   │   ├── build.rs                    # Compile-time schema codegen
│   │   └── Cargo.toml                  # Deps: msez-core, jsonschema, schemars
│   │
│   ├── msez-api/                       # AXUM API SERVICES (five primitives + corridors)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── main.rs                 # Entry point, router assembly
│   │   │   ├── state.rs                # AppState (shared services)
│   │   │   ├── error.rs                # AppError → HTTP response mapping
│   │   │   ├── auth.rs                 # JWT/Bearer middleware
│   │   │   ├── extractors.rs           # ValidatedJson, ValidatedQuery
│   │   │   ├── routes/
│   │   │   │   ├── entities.rs         # ENTITIES primitive
│   │   │   │   ├── ownership.rs        # OWNERSHIP primitive
│   │   │   │   ├── fiscal.rs           # FISCAL primitive
│   │   │   │   ├── identity.rs         # IDENTITY primitive
│   │   │   │   ├── consent.rs          # CONSENT primitive
│   │   │   │   ├── corridors.rs        # Corridor operations
│   │   │   │   ├── smart_assets.rs     # Smart Asset CRUD + compliance eval
│   │   │   │   └── regulator.rs        # Regulator console
│   │   │   ├── middleware/
│   │   │   │   ├── tracing.rs          # Request tracing
│   │   │   │   ├── metrics.rs          # Prometheus metrics
│   │   │   │   └── rate_limit.rs       # Per-jurisdiction rate limiting
│   │   │   └── openapi.rs              # Auto-generated OpenAPI 3.1 via utoipa
│   │   └── Cargo.toml                  # Deps: axum, tower, tokio, sqlx, utoipa
│   │
│   └── msez-cli/                       # CLI TOOL (replaces tools/msez.py)
│       ├── src/
│       │   ├── main.rs                 # clap-based CLI
│       │   ├── validate.rs             # Zone/module/profile validation
│       │   ├── lock.rs                 # Lockfile generation + verification
│       │   ├── corridor.rs             # Corridor management commands
│       │   ├── artifact.rs             # CAS operations
│       │   └── signing.rs              # Ed25519/VC signing commands
│       └── Cargo.toml                  # Deps: clap, msez-core, msez-pack, msez-crypto
│
├── schemas/                            # JSON schemas (unchanged, data files)
├── modules/                            # YAML descriptors (unchanged, data files)
├── spec/                               # Specification chapters (unchanged)
├── governance/                         # State machine definitions (updated to v2)
├── jurisdictions/                      # Zone configurations (unchanged)
├── profiles/                           # Profile definitions (unchanged)
├── registries/                         # Registry definitions (unchanged)
├── rulesets/                           # Transition rulesets (unchanged)
├── dist/                               # Content-addressed artifact store
├── deploy/
│   ├── docker/                         # Updated: Rust binary containers
│   ├── aws/terraform/                  # Updated: ECS/Fargate for Rust services
│   └── k8s/                            # Kubernetes manifests
└── tests/
    ├── integration/                    # Rust integration tests
    ├── fixtures/                       # Test data (shared with Python provenance)
    └── scenarios/                      # Scenario-based tests (ported from Python)
```

### 5.4 Dependency Direction — Enforced by Cargo

The crate dependency graph is strictly layered. No cycles are possible because Cargo enforces acyclic dependencies at build time.

```
                    msez-api (Axum services)
                   /    |    \        \
                  /     |     \        \
         msez-cli  msez-agentic  msez-arbitration
              |        |    \         |
              |        |     \        |
    msez-corridor  msez-tensor  msez-zkp
         |    \        |         |
         |     \       |         |
    msez-pack  msez-state  msez-vc
         |         |         |
         |         |         |
         msez-schema    msez-crypto
              \          /
               \        /
              msez-core  ← FOUNDATIONAL (no external deps except serde)
```

**Rules:**

Crates may only depend on crates below them in this graph. `msez-core` depends on nothing internal. `msez-crypto` depends only on `msez-core`. `msez-api` sits at the top and can depend on everything. Violations of this ordering are compile errors.

### 5.5 Type System Design Principles

**Newtypes for Domain Primitives.** Every domain identifier is a newtype, not a bare string:

```rust
// These are distinct types. You cannot pass a JurisdictionId where an EntityId is expected.
pub struct JurisdictionId(String);
pub struct EntityId(Uuid);
pub struct CorridorId(Uuid);
pub struct MigrationId(Uuid);
pub struct WatcherId(Uuid);
pub struct NTN(String);           // Pakistan National Tax Number
pub struct CNIC(String);          // Pakistan CNIC (national identity)
pub struct ContentDigest([u8; 32]);
pub struct CanonicalBytes(Vec<u8>);
pub struct Ed25519Signature([u8; 64]);
```

**Sealed Traits for Extension Points.** Where the system must support multiple implementations (proof systems, settlement rails, identity providers), use sealed traits that prevent external crates from implementing them without explicit opt-in:

```rust
mod private { pub trait Sealed {} }

pub trait SettlementRail: private::Sealed + Send + Sync {
    fn settle(&self, instruction: SettlementInstruction) -> Result<SettlementReceipt, SettlementError>;
}

// Only authorized implementations:
impl private::Sealed for SwiftPacs008 {}
impl private::Sealed for CircleUsdcTransfer {}
impl private::Sealed for SbpRaast {}
```

**Builder Pattern with Compile-Time Required Fields.** For complex construction like migration sagas, use the typestate builder pattern where the compiler enforces that all required fields are set before building:

```rust
pub struct MigrationBuilder<D> { /* ... */ _deadline: PhantomData<D> }
pub struct NoDeadline;
pub struct HasDeadline;

impl MigrationBuilder<NoDeadline> {
    pub fn deadline(self, deadline: Timestamp) -> MigrationBuilder<HasDeadline> {
        MigrationBuilder { deadline: Some(deadline), _deadline: PhantomData, ..self }
    }
}

impl MigrationBuilder<HasDeadline> {
    // .build() only exists when deadline has been set.
    pub fn build(self) -> Result<MigrationSaga<Initiated>, MigrationError> { /* ... */ }
}

// MigrationBuilder<NoDeadline> has no .build() method. Compile error.
```

### 5.6 Feature Flags for Phase Gating

Cargo features provide compile-time phase gating for capabilities that are specified but not yet implemented:

```toml
# msez-zkp/Cargo.toml
[features]
default = ["mock"]
mock = []                        # Phase 1: deterministic mock proofs
groth16 = ["dep:ark-groth16"]    # Phase 2: real Groth16
plonk = ["dep:halo2_proofs"]     # Phase 2: real PLONK
stark = ["dep:plonky2"]          # Phase 2: real STARK
poseidon2 = ["dep:poseidon2-rs"] # Phase 2: ZK-friendly hashing
bbs-plus = ["dep:bbs"]           # Phase 2: selective disclosure
```

In Phase 1, the system builds with `--features mock`. The mock proof system is transparent and deterministic — identical to the current Python behavior but with the trait interface already defined for the real implementations.

### 5.7 Database Layer — SQLx with Compile-Time Query Verification

SQLx provides compile-time verification that every SQL query is syntactically correct and that the result types match the Rust struct fields. This is unique to the Rust ecosystem and has no equivalent in Python, Java, or Kotlin.

```rust
// This query is verified AGAINST THE LIVE DATABASE at compile time.
// If the column types or names don't match, it's a compile error.
let entity = sqlx::query_as!(
    EntityRecord,
    r#"
    SELECT id, name, jurisdiction_id, status as "status: EntityStatus",
           created_at, updated_at
    FROM entities
    WHERE id = $1
    "#,
    entity_id.as_uuid()
)
.fetch_optional(&pool)
.await?;
```

The database schema becomes a compile-time dependency. Schema migrations that break queries are caught before the binary is built.

### 5.8 Observability — Native Tracing

The `tracing` crate provides structured, hierarchical logging with zero-cost span creation:

```rust
#[tracing::instrument(
    name = "corridor.transition",
    skip(self),
    fields(corridor_id = %self.id, from_state = %current_state, to_state = %target_state)
)]
pub fn transition(&mut self, target_state: CorridorState) -> Result<(), CorridorError> {
    tracing::info!("Initiating corridor state transition");
    // Spans nest automatically. Child operations inherit the corridor_id field.
}
```

All the swallowed-exception defects from the Python audit are structurally prevented because `tracing` integrates with `Result` — errors are logged at the point they occur with full span context.

---

## Part VI: Migration Strategy

### 6.1 Phase 0 — Foundation Crates (Weeks 1-4)

Build `msez-core`, `msez-crypto`, and `msez-schema`. These are the primitives everything else depends on.

**Deliverables:** `CanonicalBytes` newtype with JCS canonicalization, `ContentDigest` with `DigestAlgorithm` enum, `ComplianceDomain` single-enum with all 20 variants, Ed25519 signing/verification, MMR implementation, CAS store/resolve, JSON Schema validation for YAML module descriptors.

**Validation:** Port `tests/test_deterministic_canonical_bytes.py`, `tests/test_mmr.py`, `tests/test_schemas.py`, `tests/test_lawpack_artifact_canonical_bytes.py`, and `tests/test_lawpack_determinism.py` as Rust integration tests. All must produce byte-identical outputs to the Python originals.

**Critical Constraint:** The Rust CAS must produce identical content-addressed digests to the Python CAS for the same input data. This is the interoperability bridge. Write a cross-validation test that serializes test data through both implementations and asserts digest equality.

### 6.2 Phase 1 — State Machines & Pack Trilogy (Weeks 5-10)

Build `msez-state`, `msez-vc`, `msez-pack`, and `msez-zkp` (mock feature only).

**Deliverables:** Typestate-encoded corridor state machine (v2 — spec-aligned state names), migration saga with compile-time deadline enforcement, entity lifecycle with 10-stage dissolution, lawpack/regpack/licensepack parsers and validators, VC signing with Ed25519, mock proof system.

**Validation:** Port `tests/test_lifecycle_state_machine.py`, `tests/test_licensepack.py`, `tests/test_regpack.py`, `tests/test_lawpack_*.py`, `tests/test_composition.py`, and `tests/test_corridor_*.py` scenarios.

**Critical Constraint:** `msez lock jurisdictions/_starter/zone.yaml --check` must produce byte-identical lockfile output from the Rust CLI. This validates that the entire validation and lockfile pipeline is functionally equivalent.

### 6.3 Phase 2 — Corridor & Tensor (Weeks 11-16)

Build `msez-tensor`, `msez-corridor`, `msez-agentic`, and `msez-arbitration`.

**Deliverables:** Compliance tensor with all 20 domains, compliance manifold with path optimization, corridor bridge with Dijkstra routing, receipt chain with MMR, fork resolution with secondary ordering (watcher count, lexicographic tiebreak, clock skew tolerance), netting engine, agentic policy engine with 20 trigger types, dispute lifecycle with escrow.

**Validation:** Port `tests/test_phoenix*.py`, `tests/test_trade_corridors.py`, `tests/test_agentic*.py`, `tests/test_arbitration.py`, `tests/test_composition.py`, `tests/test_fork_*.py`.

### 6.4 Phase 3 — API Services & CLI (Weeks 17-22)

Build `msez-api` (Axum) and `msez-cli` (clap).

**Deliverables:** Five primitive API services (Entities, Ownership, Fiscal, Identity, Consent), corridor operations API, smart asset CRUD + compliance evaluation, regulator console, auto-generated OpenAPI 3.1 specs via utoipa, CLI with full validation/lock/corridor/artifact/signing subcommands.

**Validation:** Full API integration test suite using `axum-test`. CLI must pass all CI pipeline commands:

```bash
msez validate --all-modules
msez validate --all-profiles
msez validate --all-zones
msez lock jurisdictions/_starter/zone.yaml --check
```

### 6.5 Phase 4 — ZK & Advanced Crypto (Weeks 23-30)

Activate feature flags for Groth16, PLONK, and/or STARK. Implement Poseidon2 CDB. Implement BBS+ selective disclosure.

**Deliverables:** Real zero-knowledge proof generation and verification for compliance attestation circuits, migration evidence circuits, and settlement proof circuits. Poseidon2 digest computation alongside SHA256. BBS+ selective disclosure for VC claims.

### 6.6 Phase 5 — Production Hardening & Sovereign AI (Weeks 31+)

Production deployment infrastructure (Docker containers with static Rust binaries, Kubernetes manifests, health probes), Pakistan-specific integrations (FBR IRIS adapter, SBP Raast adapter, NADRA CNIC cross-reference, SECP corporate registry sync), Sovereign AI layer architecture.

---

## Part VII: Detailed Rust Implementations for Critical Defects

This section provides the complete Rust code for the highest-priority defect resolutions. These are not sketches — they are implementation-ready module designs.

### 7.1 Canonicalization Engine (`msez-core/src/canonical.rs`)

```rust
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CanonicalizationError {
    #[error("Float values are not permitted in canonical representations; use string or integer for amount: {0}")]
    FloatRejected(f64),
    #[error("Serialization failed: {0}")]
    SerializationFailed(#[from] serde_json::Error),
}

/// Bytes produced exclusively by JCS-compatible canonicalization with
/// Momentum-specific type coercion rules.
///
/// # Invariants
/// - The only constructor is `CanonicalBytes::new()`.
/// - All datetime values are UTC ISO8601 with Z suffix, truncated to seconds.
/// - All numeric amounts are integers or strings, never floats.
/// - All dict keys are strings.
/// - Tuples/sequences are JSON arrays.
/// - Serialization uses sorted keys with compact separators.
///
/// These invariants are enforced by the constructor and cannot be violated
/// by downstream code because the inner `Vec<u8>` is private.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CanonicalBytes(Vec<u8>);

impl CanonicalBytes {
    /// Construct canonical bytes from any serializable value.
    ///
    /// Applies the full Momentum type coercion pipeline before serialization.
    /// This is the ONLY way to construct CanonicalBytes. All digest computation
    /// in the entire stack must flow through this constructor.
    pub fn new(obj: &impl Serialize) -> Result<Self, CanonicalizationError> {
        let value = serde_json::to_value(obj)?;
        let coerced = coerce_json_value(value)?;
        // JCS: sorted keys, compact separators, no trailing whitespace
        let bytes = serde_jcs::to_vec(&coerced)
            .map_err(|e| CanonicalizationError::SerializationFailed(
                serde_json::Error::custom(e.to_string())
            ))?;
        Ok(Self(bytes))
    }

    /// Access the canonical bytes for digest computation.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8]> for CanonicalBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Recursively coerce JSON values according to Momentum canonicalization rules.
fn coerce_json_value(value: Value) -> Result<Value, CanonicalizationError> {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if n.is_f64() && !n.is_i64() && !n.is_u64() {
                    return Err(CanonicalizationError::FloatRejected(f));
                }
            }
            Ok(Value::Number(n))
        }
        Value::Object(map) => {
            let mut coerced = serde_json::Map::new();
            for (k, v) in map {
                // Keys are already strings in JSON; this handles the edge case
                // where a non-string key was somehow introduced.
                coerced.insert(k, coerce_json_value(v)?);
            }
            Ok(Value::Object(coerced))
        }
        Value::Array(arr) => {
            let coerced: Result<Vec<_>, _> = arr.into_iter()
                .map(coerce_json_value)
                .collect();
            Ok(Value::Array(coerced?))
        }
        Value::String(s) => {
            // Datetime normalization: if the string parses as a datetime,
            // normalize to UTC ISO8601 with Z suffix, truncated to seconds.
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&s) {
                let utc = dt.with_timezone(&chrono::Utc);
                Ok(Value::String(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string()))
            } else {
                Ok(Value::String(s))
            }
        }
        other => Ok(other), // Bool, Null pass through unchanged
    }
}
```

### 7.2 Corridor Typestate Machine (`msez-state/src/corridor.rs`)

```rust
use std::marker::PhantomData;
use chrono::{DateTime, Utc};
use msez_core::{CorridorId, JurisdictionId, ContentDigest};

// ─── State Types (each is a distinct type at compile time) ───────────
pub struct Draft;
pub struct Pending;
pub struct Active;
pub struct Halted;
pub struct Suspended;
pub struct Deprecated;

/// Marker trait for all valid corridor states.
/// Sealed: only the states defined here implement it.
pub trait CorridorState: private::Sealed + std::fmt::Debug {
    fn name() -> &'static str;
    fn is_terminal() -> bool { false }
}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Draft {}
    impl Sealed for super::Pending {}
    impl Sealed for super::Active {}
    impl Sealed for super::Halted {}
    impl Sealed for super::Suspended {}
    impl Sealed for super::Deprecated {}
}

impl CorridorState for Draft     { fn name() -> &'static str { "DRAFT" } }
impl CorridorState for Pending   { fn name() -> &'static str { "PENDING" } }
impl CorridorState for Active    { fn name() -> &'static str { "ACTIVE" } }
impl CorridorState for Halted    { fn name() -> &'static str { "HALTED" } }
impl CorridorState for Suspended { fn name() -> &'static str { "SUSPENDED" } }
impl CorridorState for Deprecated {
    fn name() -> &'static str { "DEPRECATED" }
    fn is_terminal() -> bool { true }
}

// ─── Evidence Types (each transition requires specific evidence) ─────
pub struct SubmissionEvidence {
    pub bilateral_agreement_digest: ContentDigest,
    pub pack_trilogy_digest: ContentDigest,
    pub submitter_attestation: Vec<u8>,
}

pub struct ActivationEvidence {
    pub regulatory_approval_a: ContentDigest,
    pub regulatory_approval_b: ContentDigest,
    pub watcher_quorum_attestation: Vec<u8>,
}

pub struct HaltReason {
    pub reason: String,
    pub authority: JurisdictionId,
    pub evidence: ContentDigest,
}

pub struct SuspendReason {
    pub reason: String,
    pub expected_resume: Option<DateTime<Utc>>,
}

pub struct ResumeEvidence {
    pub resolution_attestation: ContentDigest,
}

// ─── The Corridor ────────────────────────────────────────────────────
pub struct Corridor<S: CorridorState> {
    pub id: CorridorId,
    pub jurisdiction_a: JurisdictionId,
    pub jurisdiction_b: JurisdictionId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    inner: CorridorInner,
    _state: PhantomData<S>,
}

struct CorridorInner {
    pack_trilogy_digest: Option<ContentDigest>,
    bilateral_agreement_digest: Option<ContentDigest>,
    halt_reason: Option<HaltReason>,
    suspend_reason: Option<SuspendReason>,
    transition_log: Vec<TransitionRecord>,
}

struct TransitionRecord {
    from: String,
    to: String,
    timestamp: DateTime<Utc>,
    evidence_digest: ContentDigest,
}

// ─── State-Specific Methods ──────────────────────────────────────────
// Only Corridor<Draft> has .submit(). Only Corridor<Active> has .halt().
// Calling .halt() on Corridor<Draft> is a COMPILE ERROR.

impl Corridor<Draft> {
    pub fn new(
        id: CorridorId,
        jurisdiction_a: JurisdictionId,
        jurisdiction_b: JurisdictionId,
    ) -> Self {
        Corridor {
            id, jurisdiction_a, jurisdiction_b,
            created_at: Utc::now(), updated_at: Utc::now(),
            inner: CorridorInner {
                pack_trilogy_digest: None,
                bilateral_agreement_digest: None,
                halt_reason: None,
                suspend_reason: None,
                transition_log: Vec::new(),
            },
            _state: PhantomData,
        }
    }

    pub fn submit(mut self, evidence: SubmissionEvidence) -> Corridor<Pending> {
        self.inner.pack_trilogy_digest = Some(evidence.pack_trilogy_digest);
        self.inner.bilateral_agreement_digest = Some(evidence.bilateral_agreement_digest);
        self.updated_at = Utc::now();
        Corridor {
            id: self.id, jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at, updated_at: self.updated_at,
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl Corridor<Pending> {
    pub fn activate(mut self, evidence: ActivationEvidence) -> Corridor<Active> {
        self.updated_at = Utc::now();
        Corridor {
            id: self.id, jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at, updated_at: self.updated_at,
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl Corridor<Active> {
    pub fn halt(mut self, reason: HaltReason) -> Corridor<Halted> {
        self.inner.halt_reason = Some(reason);
        self.updated_at = Utc::now();
        Corridor {
            id: self.id, jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at, updated_at: self.updated_at,
            inner: self.inner,
            _state: PhantomData,
        }
    }

    pub fn suspend(mut self, reason: SuspendReason) -> Corridor<Suspended> {
        self.inner.suspend_reason = Some(reason);
        self.updated_at = Utc::now();
        Corridor {
            id: self.id, jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at, updated_at: self.updated_at,
            inner: self.inner,
            _state: PhantomData,
        }
    }
}

impl Corridor<Suspended> {
    pub fn resume(mut self, evidence: ResumeEvidence) -> Corridor<Active> {
        self.inner.suspend_reason = None;
        self.updated_at = Utc::now();
        Corridor {
            id: self.id, jurisdiction_a: self.jurisdiction_a,
            jurisdiction_b: self.jurisdiction_b,
            created_at: self.created_at, updated_at: self.updated_at,
            inner: self.inner,
            _state: PhantomData,
        }
    }
}
```

### 7.3 Five Primitives API — Axum Router Assembly (`msez-api/src/main.rs`)

```rust
use axum::Router;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use std::sync::Arc;

mod routes;
mod state;
mod error;
mod auth;
mod extractors;
mod middleware;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    // Database connection pool with compile-time query verification
    let pool = PgPoolOptions::new()
        .max_connections(50)
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    // Shared application state
    let state = AppState::new(pool);

    // Assemble the router from the five primitive services
    let app = Router::new()
        .merge(routes::entities::router())
        .merge(routes::ownership::router())
        .merge(routes::fiscal::router())
        .merge(routes::identity::router())
        .merge(routes::consent::router())
        .merge(routes::corridors::router())
        .merge(routes::smart_assets::router())
        .merge(routes::regulator::router())
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(middleware::metrics::MetricsLayer::new())
                .layer(middleware::rate_limit::RateLimitLayer::new())
                .layer(auth::AuthLayer::new(state.clone())),
        )
        .with_state(state);

    // Health probes (unauthenticated)
    let health = Router::new()
        .route("/health/liveness", axum::routing::get(|| async { "ok" }))
        .route("/health/readiness", axum::routing::get(routes::health::readiness));

    let app = Router::new().merge(app).merge(health);

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Part VIII: Pakistan GovOS Deployment Gap Analysis

This analysis is unchanged from the original audit but is now framed against the Rust migration timeline.

### 8.1 What Exists Today (Ready for Phase 1 Pilot After Rust Phase 0-1)

The Pack Trilogy — lawpack, regpack, licensepack — is the strongest asset. Once ported to `msez-pack` (Rust Phase 1), it can ingest Pakistani statutes (Income Tax Ordinance 2001, Sales Tax Act 1990, Federal Excise Act), model FBR tax calendars and SRO schedules, and track 15+ license categories. The content-addressed artifact store, receipt chain, and MMR infrastructure provide a solid audit trail.

### 8.2 What Must Be Built (Rust Phases 3-5)

**Layer 04 — National System Integration (Largest Gap):** FBR IRIS integration (NTN registration, return filing, IRIS XML schema), SBP Raast integration (real-time PKR settlement), NADRA integration (CNIC cross-referencing — Identity primitive has no API today), SECP integration (corporate registry sync). These become Rust service adapters in `msez-api` with jurisdiction-specific trait implementations.

**Cross-Border Corridors:** Bilateral Pack Trilogy instances for PAK↔KSA ($5.4B), PAK↔UAE ($10.1B), PAK↔CHN ($23.1B). Corridor bridge and routing infrastructure exists conceptually but needs real settlement rail integration. The SWIFT adapter is a 15-line Python stub — the Rust `msez-corridor/src/swift.rs` will be a proper pacs.008 implementation.

**Sovereign AI Layer:** Entirely net-new. No AI/ML infrastructure in the current stack. Foundation model, tax intelligence, operational intelligence — all require physical infrastructure procurement and are independent of the Rust migration.

### 8.3 What Can Be Deferred (Rust Phase 4+)

ZK proof systems (real Groth16/PLONK/STARK), BBS+ selective disclosure, Poseidon2 CDB, full watcher economy with real stake mechanics. The Smart Asset VM as a general-purpose execution environment is not needed for tax compliance operations.

---

## Part IX: Updated CLAUDE.md — Rust Migration Execution Contract

This section is the machine-readable execution contract that will be placed at `CLAUDE.md` in the repository root to drive AI-assisted development of the Rust migration.

---

### CLAUDE.md Content Begins Here

```markdown
# CLAUDE.md — SEZ Stack Rust Migration & Fortification

## Identity

You are the Principal Systems Architect executing a full Rust migration of the
Momentum Open Source SEZ Stack. The current Python implementation (v0.4.44 GENESIS)
has been audited at institutional grade. This migration is not for performance —
it is for correctness guarantees at the level required for sovereign digital
infrastructure serving nation-states.

You think like a protocol designer: "is it correct under adversarial conditions,
concurrent access, temporal edge cases, and at nation-state scale?" You write
code the way you write a cryptographic proof: every line justified, every edge
case addressed, every failure mode visible. The compiler is your first reviewer.

---

## Ground Truth Hierarchy

1. **Specification** (`spec/` — 48 chapters): Canonical. Code matches spec, not vice versa.
2. **This Audit** (`docs/fortification/sez_stack_audit_v2.md`): Defect map and Rust architecture.
3. **Existing Tests** (`tests/` — 87 files): Encode validated behaviors. Port scenarios exactly.
4. **Schema Contracts** (`schemas/` — 116 JSON schemas): Public API surface.
5. **YAML Module Descriptors** (`modules/` — 583 files): Data files, unchanged by migration.

---

## Rust Workspace Layout

See Part V §5.3 of the audit for the complete workspace layout. Key crates:

- `msez-core`: CanonicalBytes, ContentDigest, ComplianceDomain, newtypes
- `msez-crypto`: Ed25519, MMR, CAS, (Phase 2: BBS+, Poseidon2)
- `msez-vc`: Verifiable Credentials
- `msez-state`: Typestate corridor/migration/entity/license state machines
- `msez-tensor`: Compliance Tensor (20 domains, exhaustive match)
- `msez-zkp`: Proof system trait + mock impl (Phase 2: arkworks/halo2)
- `msez-pack`: Lawpack/Regpack/Licensepack
- `msez-corridor`: Bridge, receipt chain, fork resolution, netting, SWIFT
- `msez-agentic`: Policy engine (20 triggers)
- `msez-arbitration`: Dispute lifecycle
- `msez-schema`: JSON Schema validation + codegen
- `msez-api`: Axum services (five primitives + corridors + regulator)
- `msez-cli`: clap CLI (replaces tools/msez.py)

---

## Non-Negotiable Type System Rules

1. **CanonicalBytes newtype.** ALL digest computation flows through
   `CanonicalBytes::new()`. No raw `serde_json::to_vec()` for digests. Ever.

2. **Typestate state machines.** Corridor, Migration, Entity, License lifecycle
   use typestate pattern. Invalid transitions are compile errors, not runtime checks.

3. **Single ComplianceDomain enum.** One definition in `msez-core/src/domain.rs`.
   All 20 domains. Every `match` is exhaustive.

4. **Newtypes for domain primitives.** JurisdictionId, EntityId, CorridorId,
   NTN, CNIC — all newtypes. No bare strings for identifiers.

5. **Result<T, E> everywhere.** No `.unwrap()` outside tests. No `panic!()`.
   Errors propagate with `?` and carry structured context via `thiserror`.

6. **Feature flags for phase gating.** Phase 2 crypto behind Cargo features.
   Phase 1 builds with `--features mock` only.

---

## Execution Protocol

### Phase 0: Foundation (Weeks 1-4)
Build: msez-core, msez-crypto, msez-schema

Validation:
- Port test_deterministic_canonical_bytes.py
- Port test_mmr.py
- Port test_schemas.py
- Cross-validate: Python CAS digest == Rust CAS digest for same input

### Phase 1: State Machines & Packs (Weeks 5-10)
Build: msez-state, msez-vc, msez-pack, msez-zkp (mock)

Validation:
- Port test_lifecycle_state_machine.py
- Port test_licensepack.py, test_regpack.py
- Port test_corridor_*.py scenarios
- msez lock jurisdictions/_starter/zone.yaml --check (byte-identical)

### Phase 2: Corridors & Tensors (Weeks 11-16)
Build: msez-tensor, msez-corridor, msez-agentic, msez-arbitration

Validation:
- Port test_phoenix*.py
- Port test_trade_corridors.py
- Port test_agentic*.py, test_arbitration.py
- Port test_fork_*.py

### Phase 3: API & CLI (Weeks 17-22)
Build: msez-api (Axum), msez-cli (clap)

Validation:
- Full API integration tests via axum-test
- CLI passes all CI commands
- OpenAPI specs auto-generated via utoipa

### Phase 4: ZK & Advanced Crypto (Weeks 23-30)
Enable: groth16, plonk, poseidon2, bbs-plus features

### Phase 5: Production & Pakistan (Weeks 31+)
FBR IRIS, SBP Raast, NADRA, SECP adapters

---

## API Services: Axum Architecture

Five primitive routers assembled into a single Axum application:

- /v1/entities/*     → routes::entities
- /v1/ownership/*    → routes::ownership
- /v1/fiscal/*       → routes::fiscal
- /v1/identity/*     → routes::identity
- /v1/consent/*      → routes::consent
- /v1/corridors/*    → routes::corridors
- /v1/smart-assets/* → routes::smart_assets
- /v1/regulator/*    → routes::regulator

Tower middleware stack: TraceLayer → MetricsLayer → RateLimitLayer → AuthLayer

Database: PostgreSQL via SQLx with compile-time query verification.
Serialization: serde + serde_json (same serialization layer as crypto core).
OpenAPI: auto-generated from handler types via utoipa derive macros.

---

## Anti-Patterns

1. Do NOT use `json.dumps`-equivalent for digests. CanonicalBytes::new() only.
2. Do NOT use String for state names. Typestate types only.
3. Do NOT add .unwrap() outside test code.
4. Do NOT add dependencies without workspace-level justification.
5. Do NOT change schema $id or $ref URIs without grep -rn verification.
6. Do NOT mock crypto in tests. Use real CanonicalBytes, real sha256.
7. Do NOT use `Box<dyn Error>`. Use thiserror-derived error enums.
8. Do NOT use unsafe without a // SAFETY: comment explaining the invariant.

---

## Completion Criteria

The Rust migration is complete when:

1. `cargo test --workspace` passes with zero failures.
2. All 87 Python test scenarios have Rust equivalents producing identical outputs.
3. `msez validate --all-modules && msez lock jurisdictions/_starter/zone.yaml --check`
   produces byte-identical artifacts to the Python implementation.
4. `ComplianceDomain` has 20 variants with exhaustive match everywhere.
5. Corridor state machine uses typestate with spec-aligned names.
6. CanonicalBytes newtype is the sole path to digest computation.
7. Five primitive API services respond on Axum with auto-generated OpenAPI.
8. `cargo clippy --workspace -- -D warnings` produces zero warnings.
9. `cargo audit` reports zero known vulnerabilities.
10. Docker containers start, pass health probes, and serve API requests.
```

---

## Part X: Prioritized Remediation Roadmap (Combined)

This roadmap integrates the immediate Python fixes (for any interim deployments before the Rust migration is complete) with the Rust migration phases.

### Tier 0: Immediate Python Fixes (5.5 days) — Execute Now

These fixes apply to the current Python codebase and should be completed immediately, independent of the Rust migration timeline, to stabilize any interim deployments.

1. **Canonicalization unification** — Replace all `json.dumps(sort_keys=True)` in the 17 phoenix files with `jcs_canonicalize()` from `tools/lawpack.py`. 2 days.
2. **Dependency pinning** — Pin all 5 dependencies to exact versions. 0.5 days.
3. **Schema hardening** — Set `additionalProperties: false` on all VC and receipt schemas. 2 days.
4. **Exception handling** — Replace bare `except Exception:` with structured logging across the phoenix layer. 1 day.

### Tier 1: Rust Phase 0 — Foundation Crates (4 weeks)

Build `msez-core`, `msez-crypto`, `msez-schema`. Establish the type-system foundation that prevents the critical audit findings by construction.

### Tier 2: Rust Phase 1 — State Machines & Packs (6 weeks)

Build `msez-state`, `msez-vc`, `msez-pack`, `msez-zkp` (mock). Typestate-encode all state machines. Port the Pack Trilogy.

### Tier 3: Rust Phase 2 — Corridors & Tensors (6 weeks)

Build `msez-tensor`, `msez-corridor`, `msez-agentic`, `msez-arbitration`. Complete the protocol logic layer.

### Tier 4: Rust Phase 3 — API & CLI (6 weeks)

Build `msez-api` (Axum five primitives) and `msez-cli` (clap). Replace the Python API scaffolds and the 15,472-line msez.py monolith.

### Tier 5: Rust Phase 4 — ZK & Advanced Crypto (8 weeks)

Activate real proof systems (arkworks Groth16, halo2 PLONK), Poseidon2 CDB, BBS+ selective disclosure.

### Tier 6: Rust Phase 5 — Production & Pakistan (Ongoing)

FBR IRIS, SBP Raast, NADRA, SECP adapters. Sovereign AI layer. Cross-border corridor settlement rails.

---

## Conclusion

The current Python implementation of the SEZ Stack is a world-class reference architecture. The specification is an intellectual achievement. The test suite demonstrates serious engineering investment. The Pack Trilogy is the most complete open-source jurisdictional configuration system in existence.

But the audit reveals that the gap between "world-class reference implementation" and "sovereign infrastructure for 220M citizens" is precisely the gap between a language that permits entire classes of defects and a language that prevents them by construction.

The Rust migration is not a rewrite. It is a *compilation* — taking the ideas, the specification, the test scenarios, and the architectural insights that already exist and expressing them in a medium where the compiler verifies the properties that the audit had to discover manually. Every critical finding in this document is a property that the Rust type system enforces automatically, silently, on every build, forever.

The specification is world-class. The architecture is world-class. The implementation language must match.
