# Next Session Prompt: Sovereign Mass Orchestration

The following is a Claude Code session prompt designed for Opus 4.6.
Copy everything below the `---` line into a new "Ask Claude to write code..." session.

---

Wire the full orchestration pipeline into sovereign Mass mode. Every sovereign write gets compliance evaluation, VC issuance, attestation storage, and audit trail — identical to proxy mode, zero HTTP self-loop.

Today sovereign mode (`SOVEREIGN_MASS=true`) mounts `mass_sovereign::sovereign_mass_router()` — raw CRUD at `/organization-info/*`, `/treasury-info/*`, etc. — and DROPS the orchestration router (`mass_proxy::router()`) that provides `/v1/entities/*`, `/v1/ownership/*`, etc. Sovereign writes bypass the compliance tensor, skip VC issuance, skip attestation storage, skip audit trail, skip agentic trigger firing. The sovereign routes are a database-backed stub. Production sovereign mode must produce the same `OrchestrationEnvelope` as proxy mode on every write, without the wasteful localhost HTTP round-trip.

Ground yourself first — read before writing

The orchestration pipeline (this is what sovereign writes currently lack):

- `mez/crates/mez-api/src/orchestration.rs` — `evaluate_compliance()`, `check_hard_blocks()`, `issue_compliance_vc()`, `store_attestation()`, `orchestrate_entity_creation()` and all per-primitive orchestrators. Note how `orchestrate_*` functions take a `mass_response: Value` and wrap it with compliance summary + VC + attestation. These are backend-agnostic — they don't care where the `mass_response` came from.
- `mez/crates/mez-api/src/routes/mass_proxy.rs` — the orchestrated endpoints. Study `create_entity()` (L466-515): validate → pre-flight compliance → hard-block check → Mass API call → post-operation orchestration. Every write handler follows this exact 5-step pattern. 1535 lines, 10 handlers.

The sovereign CRUD (this is what currently runs in sovereign mode):

- `mez/crates/mez-api/src/routes/mass_sovereign.rs` — 37 handlers, all `Json<Value>`. Study `org_create()` (L205-227): generates UUID, builds JSON, inserts into store, persists to Postgres, returns. No compliance. No VC. No attestation.

Current routing decision (this is the one-line root cause):

- `mez/crates/mez-api/src/lib.rs` — L101-106: `if state.sovereign_mass { sovereign_mass_router() } else { mass_proxy::router() }`. This either/or is the problem. Sovereign mode needs BOTH: the sovereign CRUD at its original paths (for inter-zone Mass API access) AND the orchestration endpoints at `/v1/*`.

State, stores, and persistence:

- `mez/crates/mez-api/src/state.rs` — `AppState` with `sovereign_mass: bool`, all `mass_*` stores, `Store<T>`, `hydrate_from_db()`.
- `mez/crates/mez-api/src/db/mass_primitives.rs` — `save_organization()`, `load_all_organizations()`, etc. The `persist!` macro pattern.

Audit trail:

- `mez/crates/mez-api/src/db/audit.rs` — `append(pool, AuditEvent)` with hash-chain integrity. Read the struct fields.

Agentic triggers:

- `mez/crates/mez-api/src/routes/agentic.rs` — trigger ingestion and policy evaluation flow. Understand `TriggerType` and how corridor transitions fire from policy actions.
- `mez/crates/mez-agentic/src/lib.rs` — `PolicyEngine::evaluate()`, `TriggerType` enum, `PolicyAction` variants.

Error handling:

- `mez/crates/mez-api/src/error.rs` — `AppError` variants, `Validation`, `Forbidden`, `upstream`, `Internal`. Mass proxy uses these; sovereign routes don't.

Integration tests to understand the sovereign pipeline:

- `mez/crates/mez-integration-tests/tests/sovereign_pipeline_test.rs` — the 13-step GovOS pipeline that exercises all sovereign Mass endpoints.
- `mez/crates/mez-integration-tests/tests/sovereign_persistence_test.rs` — persistence round-trip tests.

Deploy topology:

- `deploy/docker/docker-compose.two-zone.yaml` — note `MASS_ORG_INFO_URL: "http://localhost:8080"` — the mass_client is configured to self-loop. After this work, the orchestration handlers in sovereign mode will NOT use the mass_client at all for write operations.

## Architecture

The orchestrate_* functions in `orchestration.rs` are already backend-agnostic: they accept a `mass_response: Value` and wrap it with compliance + VC + attestation. They don't know or care whether that Value came from an HTTP call to Mass APIs or from a local sovereign store. Exploit this.

**Pattern for each mass_proxy write handler in sovereign mode:**

```
fn create_entity(State(state), Json(req)) -> Result<OrchestrationEnvelope> {
    req.validate()?;

    // 1. Pre-flight compliance (identical in both modes)
    let (_, summary) = evaluate_compliance(&jurisdiction, "pre-flight", ENTITY_DOMAINS);
    if let Some(reason) = check_hard_blocks(&summary) { return Err(Forbidden(reason)); }

    // 2. Mass operation — sovereign or proxy
    let mass_response = if state.sovereign_mass {
        sovereign_ops::create_entity(&state, &req).await?     // direct store write
    } else {
        let client = require_mass_client(&state)?;             // HTTP to Mass API
        let entity = client.entities().create(&mass_req).await?;
        serde_json::to_value(entity)?
    };

    // 3-5. Post-operation orchestration (identical in both modes)
    let envelope = orchestrate_entity_creation(&state, &jurisdiction, &name, mass_response);

    // 6. Audit trail (new — both modes)
    if let Some(pool) = &state.db_pool {
        audit::append(pool, AuditEvent { ... }).await.ok();
    }

    Ok((CREATED, Json(envelope)))
}
```

The key: steps 1, 3, 4, 5, 6 are mode-independent. Only step 2 branches.

## Deliverables

### 1. Sovereign operations module: `mez/crates/mez-api/src/routes/sovereign_ops.rs`

Extract the CRUD logic from `mass_sovereign.rs` into pure functions that take `&AppState` + typed inputs and return `Result<Value, AppError>`. These are the sovereign backend for the orchestration handlers.

Functions (one per Mass write operation in mass_proxy.rs):

- `pub async fn create_entity(state: &AppState, name: &str, jurisdiction: Option<&str>, entity_type: Option<&str>, tags: &[String]) -> Result<Value, AppError>` — generates UUID, builds camelCase JSON entity, inserts into `state.mass_organizations`, persists via `db::mass_primitives::save_organization()`, returns the entity JSON.
- `pub async fn update_entity(state: &AppState, id: Uuid, updates: &Value) -> Result<Value, AppError>` — updates in store, persists, returns. 404 if not found.
- `pub async fn get_entity(state: &AppState, id: Uuid) -> Result<Option<Value>, AppError>` — reads from store.
- `pub async fn list_entities(state: &AppState, ids: Option<&[Uuid]>) -> Result<Vec<Value>, AppError>` — reads from store, optional ID filter.
- `pub async fn create_cap_table(state: &AppState, org_id: &str, authorized_shares: u64, par_value: Option<&str>, shareholders: Option<&Value>) -> Result<Value, AppError>`
- `pub async fn get_cap_table_by_org(state: &AppState, org_id: &str) -> Result<Option<Value>, AppError>`
- `pub async fn create_treasury(state: &AppState, entity_id: &str, entity_name: Option<&str>, context: Option<&str>) -> Result<Value, AppError>`
- `pub async fn create_account(state: &AppState, treasury_id: Uuid, name: Option<&str>) -> Result<Value, AppError>`
- `pub async fn create_payment(state: &AppState, source_account_id: &str, amount: &str, currency: &str, reference: Option<&str>) -> Result<Value, AppError>`
- `pub async fn compute_withholding(entity_id: &str, transaction_amount: &str, currency: &str, transaction_type: &str, ntn: Option<&str>) -> Value` — pure computation, no state needed.
- `pub async fn create_consent(state: &AppState, org_id: &str, operation_id: Option<&Value>, operation_type: Option<&Value>, num_approvals: Option<u64>, requested_by: Option<&Value>) -> Result<Value, AppError>`
- `pub async fn get_consent(state: &AppState, id: Uuid) -> Result<Option<Value>, AppError>`
- `pub async fn verify_cnic(cnic: &str, full_name: Option<&str>) -> Result<Value, AppError>` — validation + mock verification (same as current).
- `pub async fn verify_ntn(ntn: &str, entity_name: Option<&str>) -> Result<Value, AppError>` — validation + mock verification.

Each function that writes must persist to Postgres when `state.db_pool.is_some()`. Use the same `persist!` macro pattern. Functions must produce JSON with identical camelCase field names to what the `mez-mass-client` types expect — the orchestration layer serializes mass_client types to Value in proxy mode, so sovereign mode must produce the same shapes.

Register this module in `routes/mod.rs`.

### 2. Modify mass_proxy.rs write handlers: sovereign branch

For each write handler in mass_proxy.rs, add a sovereign branch. The handler structure becomes:

```rust
async fn create_entity(State(state), Json(req)) -> Result<...> {
    req.validate().map_err(AppError::Validation)?;

    let (_tensor, pre_summary) = orchestration::evaluate_compliance(...);
    if let Some(reason) = orchestration::check_hard_blocks(&pre_summary) {
        return Err(AppError::Forbidden(reason));
    }

    let mass_response = if state.sovereign_mass {
        sovereign_ops::create_entity(&state, &req.legal_name, Some(&req.jurisdiction_id), Some(&req.entity_type), &[]).await?
    } else {
        let client = require_mass_client(&state)?;
        let entity = client.entities().create(&mass_req).await
            .map_err(|e| AppError::upstream(format!("Mass API error: {e}")))?;
        serde_json::to_value(entity)
            .map_err(|e| AppError::Internal(format!("serialization error: {e}")))?
    };

    let envelope = orchestration::orchestrate_entity_creation(&state, &jurisdiction_id, &legal_name, mass_response);

    // Audit trail
    if let Some(ref pool) = state.db_pool {
        let _ = crate::db::audit::append(pool, crate::db::audit::AuditEvent {
            event_type: "entity.created".to_string(),
            actor_did: Some(state.zone_did.clone()),
            resource_type: "entity".to_string(),
            resource_id: extract_uuid_from_envelope(&envelope),
            action: "create".to_string(),
            metadata: serde_json::json!({
                "jurisdiction": &jurisdiction_id,
                "compliance_status": &envelope.compliance.overall_status,
            }),
        }).await;
    }

    Ok((StatusCode::CREATED, Json(envelope)))
}
```

Apply this pattern to ALL write handlers in mass_proxy.rs:
- `create_entity` — calls `sovereign_ops::create_entity`
- `update_entity` — calls `sovereign_ops::update_entity`
- `create_cap_table` — calls `sovereign_ops::create_cap_table`
- `create_account` — calls `sovereign_ops::create_account` (need to look up treasury to get entity_id)
- `initiate_payment` — calls `sovereign_ops::create_payment`
- `verify_identity` — calls `sovereign_ops::verify_cnic` or `verify_ntn` based on identity_type
- `create_consent` — calls `sovereign_ops::create_consent`

For READ handlers (`get_entity`, `get_cap_table`, `get_consent`, `get_identity`): add sovereign branch that reads from the AppState stores instead of calling mass_client. Return 404 if not found.

### 3. Modify lib.rs routing: mount both routers in sovereign mode

Change L101-106 from:

```rust
let mass_routes = if state.sovereign_mass {
    routes::mass_sovereign::sovereign_mass_router()
} else {
    routes::mass_proxy::router()
};
```

To:

```rust
let mass_routes = routes::mass_proxy::router();
```

Then, when sovereign_mass is true, additionally merge the sovereign Mass router for direct Mass API surface access:

```rust
let mut api = Router::new().merge(mass_routes);
if state.sovereign_mass {
    tracing::info!("Sovereign Mass mode — mounting direct Mass API surface + orchestrated /v1/* endpoints");
    api = api.merge(routes::mass_sovereign::sovereign_mass_router());
}
api = api
    .merge(routes::identity::router())
    // ... rest of existing merges
```

This gives sovereign zones BOTH:
- `/v1/entities/*` — orchestrated (compliance + VC + attestation + audit)
- `/organization-info/api/v1/*` — direct CRUD (for other zones' mass_clients to call)

Proxy zones get only `/v1/*` as before.

### 4. Audit trail integration for all mass_proxy write handlers

After each successful write in mass_proxy.rs, append an audit event. Use these event types:

| Handler | event_type | resource_type | action |
|---|---|---|---|
| create_entity | entity.created | entity | create |
| update_entity | entity.updated | entity | update |
| create_cap_table | ownership.cap_table_created | cap_table | create |
| create_account | fiscal.account_created | account | create |
| initiate_payment | fiscal.payment_initiated | payment | create |
| verify_identity | identity.verified | identity | verify |
| create_consent | consent.created | consent | create |

The audit append must be fire-and-forget (log warning on failure, never block the response). Use `let _ = audit::append(...).await;` pattern.

Extract the entity UUID from the orchestration envelope's `mass_response.id` field. Use the existing `extract_id` helper in orchestration.rs, parse to Uuid.

### 5. Integration test: `mez/crates/mez-integration-tests/tests/sovereign_orchestration_test.rs`

Prove that sovereign mode produces full OrchestrationEnvelopes:

```rust
#[tokio::test]
async fn sovereign_entity_creation_produces_orchestration_envelope() {
    // Set SOVEREIGN_MASS=true in env
    // Build AppState with sovereign_mass: true
    // Build app(state) router
    // POST /v1/entities with CreateEntityProxyRequest body
    // Assert response is OrchestrationEnvelope
    // Assert envelope.compliance.domain_results has 20 entries
    // Assert envelope.credential is Some
    // Assert envelope.attestation_id is Some
    // Assert the entity also exists at GET /organization-info/api/v1/organization/{id}
}

#[tokio::test]
async fn sovereign_payment_produces_orchestration_envelope() {
    // Create entity → treasury → account → payment via /v1/* endpoints
    // Assert each response is OrchestrationEnvelope with compliance + VC
}

#[tokio::test]
async fn sovereign_consent_produces_orchestration_envelope() {
    // Create entity → consent request via /v1/consent
    // Assert OrchestrationEnvelope
}

#[tokio::test]
async fn proxy_mode_unaffected() {
    // SOVEREIGN_MASS=false (or unset)
    // Verify mass_proxy routes work as before (will fail with 503 if no mass_client, which is fine — that proves proxy mode is active)
}

#[tokio::test]
async fn sovereign_direct_and_orchestrated_routes_coexist() {
    // SOVEREIGN_MASS=true
    // Create entity via direct route: POST /organization-info/api/v1/organization/create
    // Read via orchestrated route: GET /v1/entities/{id}
    // Both should return the same entity
}
```

Use `tower::ServiceExt::oneshot` to test without starting a server. Use `axum::body::Body::from(serde_json::to_vec(&req)?)` for request bodies. Set the auth token via `AUTH_TOKEN` env var and include `Authorization: Bearer <token>` header.

### 6. Update mass_sovereign.rs: delegate to sovereign_ops

Simplify the handlers in mass_sovereign.rs to delegate to `sovereign_ops` functions where possible. This eliminates code duplication — the CRUD logic lives in one place. The mass_sovereign handlers become thin wrappers that parse the raw JSON request and call the shared function.

For example, `org_create` becomes:
```rust
async fn org_create(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let jurisdiction = body.get("jurisdiction").and_then(|v| v.as_str());
    match sovereign_ops::create_entity(&state, name, jurisdiction, None, &[]).await {
        Ok(entity) => (StatusCode::CREATED, Json(entity)).into_response(),
        Err(e) => e.into_response(),
    }
}
```

Not every handler needs this treatment. Focus on the ones that correspond to mass_proxy write operations (entity create/update, treasury, account, payment, consent, cap_table, investment). Leave identity verification, template, and submission handlers as-is (they're lightweight and have no proxy counterpart).

### 7. Update CLAUDE.md

In Section 5 (Work Priority Queue), Phase G:
- Add item: `35. Sovereign orchestration pipeline — CLOSED: mass_proxy handlers serve full OrchestrationEnvelope in sovereign mode (compliance + VC + attestation + audit)`

In Section 14 (Institutional Posture Summary), Phase 2 remaining:
- Mark sovereign orchestration as complete: `[x] Sovereign writes produce OrchestrationEnvelope with compliance evaluation, VC issuance, attestation storage, and audit trail`

In Section 10 (Deployment Phase Gates), Phase 2:
- Add: `[x] Sovereign Mass orchestration — all writes through /v1/* produce OrchestrationEnvelope (compliance tensor + signed VC + attestation + audit trail) without HTTP self-loop`

## Constraints

- Do NOT modify `orchestration.rs`. The orchestrate_* functions are already backend-agnostic. Use them as-is.
- Do NOT modify `mez-mass-client`. The sovereign backend is an alternative to the client, not a modification of it.
- Do NOT delete or rename any existing routes. The sovereign CRUD routes at `/organization-info/*` must continue to work for inter-zone access.
- Do NOT break any existing tests. Run `cargo test --workspace` before committing.
- Preserve camelCase JSON keys in all sovereign_ops return values. The orchestration layer and downstream consumers expect camelCase.
- The audit trail append must never block or fail the request. Fire-and-forget with logged warnings.
- No `unsafe`. No `unwrap()` in non-test code. BUSL-1.1 header on new files.
- Use `AppError` for all error returns in sovereign_ops. Do not return raw StatusCode.
- In mass_proxy handlers, the `if state.sovereign_mass` branch must produce a `Value` with the same JSON shape as `serde_json::to_value(mass_client_response)` would produce in proxy mode. Verify shape parity by examining the `mez-mass-client` type definitions (rename_all = "camelCase").

## Success criteria

- `cargo build --workspace` succeeds
- `cargo test --workspace` passes (all existing + new tests)
- `sovereign_pipeline_test.rs` still passes
- `sovereign_persistence_test.rs` still passes
- `sovereign_mass_test.rs` still passes
- New `sovereign_orchestration_test.rs` passes — proves sovereign entity creation returns `OrchestrationEnvelope` with 20-domain compliance, signed VC, and attestation ID
- In sovereign mode: `POST /v1/entities` returns `OrchestrationEnvelope` (not raw entity JSON)
- In sovereign mode: `POST /organization-info/api/v1/organization/create` still returns raw entity JSON (direct access preserved)
- In proxy mode: zero behavioral change — mass_proxy routes call mass_client as before
- Entity created via `/organization-info/*` is readable via `GET /v1/entities/{id}` (stores are shared)
- Audit events are written to Postgres for every write through `/v1/*` endpoints

Commit: `feat: sovereign Mass orchestration — full compliance pipeline on every sovereign write`

Push to branch when complete.
