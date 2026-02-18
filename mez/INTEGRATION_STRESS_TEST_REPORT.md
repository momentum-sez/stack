# Integration Stress Test Report — EZ Stack

**Date**: 2026-02-15
**Scope**: End-to-end deployment analysis, cross-crate integration audit, "battle hardening" assessment
**Method**: Full workspace build, 3,516-test execution, deep code path tracing, simulated deployment scenarios
**Baseline**: CLAUDE.md v5.0, AUDIT_FINDINGS.md, Architecture Audit v5.0

---

## Executive Summary

The EZ Stack compiles cleanly, passes 3,516 tests with 0 failures, and `cargo clippy --workspace -- -D warnings` is clean. The individual crates — tensor, pack, corridor, state FSM, crypto, VC — are well-implemented in isolation. **The problem is not in the parts. The problem is in the wiring.**

The modules were built in parallel, not in serial. Each crate works when tested alone, but the composition layer (`mez-api`) that should orchestrate them into end-to-end flows has gaps that would cause real failures in sovereign deployment. This report identifies those gaps by tracing actual request paths through the system.

### Verdicts

| Dimension | Grade | Evidence |
|-----------|-------|----------|
| **Compilation** | A | `cargo check` zero warnings, `cargo clippy -D warnings` clean |
| **Test coverage** | A | 3,516 tests, 0 failures, 1 ignored |
| **Security posture** | A | Zero production unwraps, zeroized keys, constant-time auth, parking_lot locks |
| **Crate-level correctness** | A | Tensor lattice algebra, FSM typestate, MMR proofs all verified |
| **Cross-crate wiring** | B- | Orchestration module exists but has atomicity gaps, type conversion fragility |
| **End-to-end completeness** | C+ | 6 of 10 endpoint families fully functional; 4 have stubs or missing executors |
| **Production readiness** | C | Database atomicity gap, deferred action executor missing, contract tests absent |

---

## I. What Actually Works End-to-End

These flows were traced from HTTP request → through all crate boundaries → to response. They work correctly.

### 1. Entity Formation with Compliance (POST /v1/entities)

```
Request → mass_proxy::create_entity()
  → orchestration::evaluate_compliance(jurisdiction, entity, ENTITY_DOMAINS)
    → compliance::build_tensor() → ComplianceTensor::new()
    → tensor.evaluate_all(entity_id)  [all 20 domains evaluated]
  → orchestration::check_hard_blocks()  [sanctions hard-block check]
  → mez_mass_client::entities::create()  [real HTTP to Mass org-info API]
  → orchestration::orchestrate_entity_creation()
    → issue_compliance_vc()  [Ed25519 signed VC via mez-vc]
    → store_attestation()  [in-memory + optional DB write-through]
  → OrchestrationEnvelope { mass_response, compliance, credential, attestation_id }
```

**Status**: WORKS. Compliance evaluation, VC issuance, and attestation storage all execute.

### 2. Payment with Tax Event Generation (POST /v1/fiscal/payments)

```
Request → mass_proxy::initiate_payment()
  → orchestration::evaluate_compliance(jurisdiction, account, PAYMENT_DOMAINS)
  → mez_mass_client::fiscal::create_payment()  [real HTTP to Mass treasury API]
  → generate_payment_tax_event()
    → TaxPipeline::process_event()  [WithholdingEngine computes PKR withholding]
    → TaxEventRecord stored in-memory + DB
  → orchestration::orchestrate_payment()  [VC issuance + attestation]
```

**Status**: WORKS. Tax events auto-generated on every payment with Pakistani withholding rules applied.

### 3. Corridor Lifecycle (POST/PUT /v1/corridors)

```
Request → corridors::create_corridor()
  → CorridorRecord created with DynCorridorState::Draft
  → DB persistence (write-through)

PUT /v1/corridors/:id/state → corridors::transition_corridor()
  → DynCorridorState FSM enforces legal transitions
  → TransitionRecord with optional ContentDigest evidence
  → Transition audit trail
```

**Status**: WORKS. Full FSM: DRAFT → PENDING → ACTIVE ↔ SUSPENDED → HALTED → DEPRECATED.

### 4. Receipt Chain Append (POST /v1/corridors/:id/receipts)

```
Request → corridors::append_receipt()
  → receipt_chains.get(corridor_id)
  → ReceiptChain::append() → MMR accumulation + sequence validation
  → Receipt with hash-linked predecessor, MMR root
```

**Status**: WORKS. SHA-256 chain integrity, MMR checkpoints, fork detection all functional.

### 5. Compliance Tensor Evaluation (POST /v1/assets/:id/evaluate-compliance)

```
Request → smart_assets::evaluate_compliance()
  → compliance::build_tensor(jurisdiction_id)
  → compliance::apply_attestations(tensor, attestations)
  → compliance::build_evaluation_result()
    → tensor.full_slice().aggregate_state()
    → tensor.commit() → SHA-256 commitment
  → ComplianceEvalResult { 20 domain_results, commitment, passing/blocking }
```

**Status**: WORKS. Full 20-domain evaluation with attestation evidence, commitment, and lattice aggregation.

### 6. Settlement Netting (POST /v1/settlement/netting)

```
Request → settlement::compute_netting()
  → NettingEngine::add_obligation() for each pair
  → NettingEngine::compute() → minimal net positions
  → NettingResult { obligations, positions, compression_ratio }
```

**Status**: WORKS. Bilateral netting compression verified.

---

## II. What Has Holes

### HOLE 1: Database Write-Through Not Atomic (P0)

**Location**: Every write handler in `mez-api/src/routes/`

**The Problem**:
```rust
// Step 1: In-memory write succeeds
state.smart_assets.insert(id, record.clone());

// Step 2: DB write may fail
if let Some(pool) = &state.db_pool {
    if let Err(e) = crate::db::smart_assets::insert(pool, &record).await {
        tracing::error!(...);  // Logged, but HTTP 201 already committed
    }
}
```

**Consequence**: If Postgres is temporarily unreachable (network blip, connection pool exhaustion), the in-memory store has the record but the DB does not. On restart, `hydrate_from_db()` loads from Postgres — the record is lost. The API returned 201 Created to the caller, who believes the operation succeeded.

**Fix**: Write to DB first. If DB write fails, don't insert into in-memory store. DB is source of truth.

```rust
// Correct order:
if let Some(pool) = &state.db_pool {
    crate::db::smart_assets::insert(pool, &record).await
        .map_err(|e| AppError::Internal(format!("persistence failed: {e}")))?;
}
state.smart_assets.insert(id, record);
```

**Affected**: `corridors.rs`, `smart_assets.rs`, `mass_proxy.rs`, `tax.rs`, `agentic.rs` — all write handlers.

---

### HOLE 2: Deferred Agentic Actions Never Execute (P1)

**Location**: `mez-api/src/routes/agentic.rs`

**The Problem**: The policy engine evaluates triggers and produces actions. `Halt` and `Resume` are dispatched immediately (they call corridor FSM transitions). But `UpdateManifest`, `ReportToRegulator`, `IssueAlert`, `SuspendOperations` are recorded as `ActionStatus::Scheduled` and **no background executor picks them up**.

```rust
match action {
    PolicyAction::Halt => { /* executes synchronously */ },
    PolicyAction::Resume => { /* executes synchronously */ },
    _ => ActionStatus::Scheduled,  // ← recorded, never executed
}
```

**Consequence**: A policy rule that says "on license revocation, suspend operations" will match correctly and log the intent but never actually suspend anything. Regulatory automation is illusory.

**Fix**: Add a background task (tokio::spawn) that polls scheduled actions and dispatches them:
```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        execute_pending_actions(&state).await;
    }
});
```

---

### HOLE 3: Mass Client Type Conversion Fragility (P1)

**Location**: `mez-api/src/routes/mass_proxy.rs`

**The Problem**: The API accepts `CreateEntityProxyRequest` but the Mass client expects `CreateEntityRequest`. Manual field mapping between them:

```rust
let mass_req = mez_mass_client::entities::CreateEntityRequest {
    name: req.legal_name,           // legal_name → name
    entity_type: Some(req.entity_type),
    jurisdiction: Some(req.jurisdiction_id),
    address: None,                  // ← silently dropped
    tags: vec![],                   // ← default
};
```

The `address` field is accepted by the caller but never forwarded to Mass. If the Mass API Swagger spec adds required fields, the proxy silently omits them.

**This pattern repeats** for fiscal accounts (entity_id passed as treasury_id), ownership (only first share class used), and consent.

**Fix**: Either use `mez_mass_client` types directly as API request types, or add compile-time mapping validation via `From` trait implementations with exhaustive field coverage.

---

### HOLE 4: No Contract Tests Against Live Mass API Specs (P0-008)

**Location**: `mez-mass-client/tests/`

**The Problem**: The Mass client has wiremock tests (good for regression) but no tests that validate against the actual Swagger specs served by the live Mass API endpoints. The types in `mez-mass-client` were written by reading the API docs — they may have drifted from the live schemas.

**Swagger endpoints to validate against**:
- `https://organization-info.api.mass.inc/organization-info/swagger-ui/index.html`
- `https://consent.api.mass.inc/consent-info/swagger-ui/index.html`
- `https://treasury-info.api.mass.inc/treasury-info/swagger-ui/index.html`
- `https://investment-info-production-*.herokuapp.com/investment-info/swagger-ui/index.html`
- `https://templating-engine-prod-*.herokuapp.com/templating-engine/swagger-ui/index.html`

**Fix**: Fetch Swagger JSON, generate serde test cases, validate roundtrip deserialization.

---

### HOLE 5: SWIFT pacs.008 Generation Stubbed (P2)

**Location**: `mez-corridor/src/swift.rs`, `mez-api/src/routes/settlement.rs`

**The Problem**: `SwiftPacs008` type exists. Netting engine produces settlement legs. But the settlement route never calls SWIFT generation — the type is defined but never instantiated in any handler.

**Consequence**: Cross-border settlements compute net positions but cannot generate ISO 20022 payment instructions.

---

### HOLE 6: Pack Composition Has No HTTP Endpoint (P2)

**Location**: `mez-pack/src/composition.rs` (implemented), `mez-api/src/routes/` (no endpoint)

**The Problem**: The multi-jurisdiction composition engine (`ZoneCompositionBuilder`) is implemented and tested (doc-tests pass). But there is no `POST /v1/packs/compose` or `GET /v1/packs/:jurisdiction` endpoint. The code works in the crate but is unreachable from the HTTP layer.

**Consequence**: Cannot query applicable packs via API. Cannot compose hybrid jurisdictions (e.g., "Delaware corporate + ADGM financial" as specified in the spec).

---

### HOLE 7: Identity Verification Endpoints Point to Non-Existent Paths (P1)

**Location**: `mez-mass-client/src/identity.rs`

**The Problem**: The `IdentityClient` constructs URLs like:
```
{base_url}/organization-info/identity/cnic/verify
```

This path may not exist on the live `organization-info.api.mass.inc` service. Identity is architecturally split across services (P1-005). The CNIC/NTN verification endpoints are aspirational — they assume a `/identity/` sub-path that the organization-info API may not expose.

**Consequence**: POST `/v1/identity/verify` will orchestrate compliance evaluation (works) but the Mass API call will likely 404.

---

### HOLE 8: No 5xx Retry in Mass Client (P2)

**Location**: `mez-mass-client/src/` (all sub-clients)

**The Problem**: Retry logic handles network errors and timeouts (200ms → 400ms → 800ms) but treats HTTP 5xx as a permanent failure. A transient Mass API outage (503 Service Unavailable) immediately fails the request.

```rust
if !resp.status().is_success() {
    return Err(MassApiError::ApiError { ... });  // No retry on 5xx
}
```

**Consequence**: Transient Mass API hiccups cascade into failed operations.

---

### HOLE 9: Compliance Tensor Commitment Not Persisted on Smart Assets (P2)

**Location**: `mez-api/src/state.rs::SmartAssetRecord`

**The Problem**: When compliance is evaluated for a smart asset, the tensor commitment (SHA-256 digest of the 20-domain evaluation) is computed and returned in the response. But it is not stored on the `SmartAssetRecord`. On the next read, the commitment is gone.

**Consequence**: Regulator cannot later verify what the compliance state was at creation time. The audit trail is incomplete.

---

### HOLE 10: Fiscal Account Creation Architecture Mismatch (P1)

**Location**: `mez-api/src/routes/mass_proxy.rs::create_account()`

**The Problem**: The Mass treasury API requires creating a **treasury** first, then an **account** within that treasury. The proxy route passes `entity_id` directly as `treasury_id`:

```rust
client.fiscal().create_account(req.entity_id, &idempotency_key, ...)
```

This works only if the entity_id happens to be a valid treasury_id, which is architecturally wrong.

**Consequence**: Account creation may fail or attach to the wrong treasury.

---

## III. Fundamental Tensions

### Tension 1: In-Memory Store vs. Database (Dual Source of Truth)

The current architecture maintains **both** an in-memory `Store<T>` and optional Postgres persistence. This creates a dual-source-of-truth problem:

- Reads come from in-memory (fast, but may be stale after failed DB writes)
- Writes go to in-memory first, then DB (non-atomic)
- Hydration on startup loads from DB (but in-memory may have had records that never reached DB)

**Resolution**: Either commit to DB-first (DB is source of truth, in-memory is cache) or commit to in-memory-only (and accept data loss on restart). The current hybrid is the worst of both worlds.

### Tension 2: Orchestration vs. Passthrough (Mass Proxy Identity Crisis)

The AUDIT_FINDINGS.md marks P1-004 as RESOLVED, and the code confirms that write endpoints DO compose compliance evaluation + Mass API + VC issuance. However, **read endpoints remain pure proxies** — `GET /v1/entities/:id` fetches from Mass without any EZ Stack enrichment. This means:

- A regulator querying an entity gets raw Mass data with no compliance context
- The compliance evaluation from the POST operation is not attached to the entity

**Resolution**: Read endpoints should return enriched responses that include the latest compliance evaluation and attestation status alongside the Mass API data.

### Tension 3: mez-compliance Crate vs. mez-api/compliance Module

Two different compliance interfaces exist:

1. `mez-compliance` crate: `build_tensor(jurisdiction_id, applicable_domains, sanctions_entries)` — takes regpack data as input, returns `ComplianceTensor<RegpackJurisdiction>`
2. `mez-api/src/compliance.rs`: `build_tensor(jurisdiction_id)` — takes only a string, returns `ComplianceTensor<DefaultJurisdiction>`

The API uses its own simpler version (#2) and ignores the richer crate (#1). The `mez-compliance` crate — designed to bridge packs and tensor — is not wired into the HTTP layer. The `SanctionsEvaluator` from `mez-compliance::evaluators` is never used in request handling.

**Resolution**: The API's `build_tensor()` should use `mez-compliance::build_tensor()` with actual regpack data from the zone bootstrap.

### Tension 4: Zone Bootstrap Runs, But Sanctions Checker Isn't Used

`bootstrap.rs` loads zone context including a `SanctionsChecker` from regpack data. But `state.zone` is `Option<ZoneContext>` and the orchestration module's `evaluate_compliance()` never accesses it. The zone's sanctions data sits in `ZoneContext::sanctions_checker` unused.

**Resolution**: Wire `ZoneContext::sanctions_checker` → `mez-compliance::SanctionsEvaluator` → `ComplianceTensor` evaluation pipeline. Currently, the sanctions hard-block check looks at the tensor's `Sanctions` domain state, which defaults to `Pending` (never `NonCompliant`), meaning **sanctions screening never actually blocks**.

---

## IV. Corrected Assessment of AUDIT_FINDINGS.md

The AUDIT_FINDINGS.md is largely accurate, with these corrections:

| AUDIT_FINDINGS Claim | Reality |
|---------------------|---------|
| P1-004 RESOLVED (mass proxy = orchestration) | **CORRECT** — write endpoints DO orchestrate (compliance + Mass + VC) |
| P1-008 RESOLVED (DB persistence) | **PARTIALLY CORRECT** — DB exists but write-through is non-atomic |
| P1-009 RESOLVED (tax collection pipeline) | **CORRECT** — TaxPipeline instantiated, withholding computed on payments |
| 11 of 12 success criteria met | **OVERSTATED** — should be 9-10 of 12 due to atomicity gap and sanctions not actually screening |
| P0-008 sole remaining item | **CORRECT** — contract tests against live Swagger still missing |

---

## V. Priority Action Items

### P0 — Fix Before Sovereign Deployment

| # | Issue | Impact | Location |
|---|-------|--------|----------|
| 1 | Database write order (DB first, then in-memory) | Data loss on Postgres hiccup | All write handlers |
| 2 | Wire ZoneContext sanctions checker to tensor evaluation | Sanctions screening is a no-op | orchestration.rs, compliance.rs |
| 3 | Contract tests against live Mass API Swagger specs | Type drift between client and API | mez-mass-client/tests/ |

### P1 — Fix Before Production Traffic

| # | Issue | Impact | Location |
|---|-------|--------|----------|
| 4 | Implement deferred action executor (background task) | Policy automation illusory | routes/agentic.rs |
| 5 | Fix fiscal account creation (treasury prerequisite) | Account creation architecturally broken | routes/mass_proxy.rs |
| 6 | Verify identity endpoint paths exist on live Mass APIs | Identity verification will 404 | mez-mass-client/identity.rs |
| 7 | Use mez-compliance crate instead of local build_tensor | Richer compliance evaluation unused | mez-api/src/compliance.rs |
| 8 | Add 5xx retry logic to Mass client | Transient outages cascade | mez-mass-client/src/*.rs |

### P2 — Code Quality / Feature Completion

| # | Issue | Impact | Location |
|---|-------|--------|----------|
| 9 | Add HTTP endpoint for pack composition | Key sales feature unreachable | routes/mod.rs |
| 10 | Store tensor commitment on SmartAssetRecord | Audit trail incomplete | state.rs |
| 11 | Implement SWIFT pacs.008 generation in settlement handler | Cross-border settlement incomplete | routes/settlement.rs |
| 12 | Complete OwnershipClient (share transfers, investors) | Cap table mutations blocked | mez-mass-client/ownership.rs |

---

## VI. What Is Genuinely Production-Ready

Despite the holes, these subsystems are solid:

- **Compliance Tensor V2**: 20-domain lattice algebra, meet/join, commitment — mathematically correct
- **Corridor FSM**: Typestate enforcement prevents invalid transitions at compile time
- **Receipt Chain + MMR**: Hash-linked chain with Merkle Mountain Range proofs and fork detection
- **Cryptographic Foundation**: Ed25519 zeroization, constant-time auth, SHA-256 via CanonicalBytes
- **Error Hierarchy**: No unwraps in production code, typed errors throughout, proper HTTP status mapping
- **Tax Pipeline**: Pakistani withholding rules (Income Tax Ordinance 2001) correctly implemented
- **Zone Bootstrap**: Zone manifest loading, signing key management, CAS directory resolution
- **Regulator Dashboard**: Read-only compliance posture aggregation with pagination

---

## VII. The One-Line Summary

**The crates are each well-built; the composition layer has wiring gaps that mean sanctions don't actually screen, database writes aren't atomic, and 4 of 10 action types are recorded but never executed.**

---

**End of Integration Stress Test Report**

Momentum · `momentum.inc`
Mass · `mass.inc`
Confidential · February 2026
