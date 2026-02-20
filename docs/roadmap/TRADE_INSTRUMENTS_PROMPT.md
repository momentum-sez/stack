# Next Session Prompt: Trade Corridor Instruments Runtime

The following is a Claude Code session prompt designed for Opus 4.6.
Copy everything below the `---` line into a new "Ask Claude to write code..." session.

---

Implement the Trade Corridor Instruments runtime. The spec layer exists — 5 trade document schemas, 10 transition payload schemas, 10 rulesets, 10 transition types registered with CAS digests, 5 module descriptors. Zero Rust implementation. Build the engine.

## Ground yourself — read before writing

These are your specification. The code you write must conform to them exactly.

**Trade document schemas** (the canonical document shapes):
- `schemas/trade.invoice.v1.schema.json` — required: `invoice_id`, `issue_date`, `seller`, `buyer`, `total`
- `schemas/trade.bill-of-lading.v1.schema.json` — required: `bol_id`, `issue_date`, `carrier`, `shipper`, `consignee`, `port_of_loading`, `port_of_discharge`, `goods`
- `schemas/trade.letter-of-credit.v1.schema.json` — required: `lc_id`, `issue_date`, `applicant`, `beneficiary`, `issuing_bank`, `amount`, `expiry_date`
- `schemas/trade.party.v1.schema.json` — required: `party_id`; optional: `name`, `lei`, `did`, `address`
- `schemas/trade.amount.v1.schema.json` — required: `currency`, `value` (decimal string, never float)

**Transition payload schemas** (what each transition carries — `oneOf: [embedded doc, artifact_ref]`):
- `schemas/transition.payload.trade.invoice.{issue,accept,settle}.v1.schema.json`
- `schemas/transition.payload.trade.bol.{issue,endorse,release}.v1.schema.json`
- `schemas/transition.payload.trade.lc.{issue,amend,present,honor}.v1.schema.json`

**Rulesets** (validation profiles per transition):
- `rulesets/mez.transition.trade.*.v1.json` — 10 files, each binding a `transition_kind` to its `payload_schema`

**Transition type registry** (already registered with CAS digests):
- `registries/transition-types.yaml` — 10 trade entries (lines 39-131) + 2 settlement entries

**Module descriptors** (define interfaces and compliance domains):
- `modules/trade/module.yaml` — family: 5 submodules, compliance: TRADE + AML_CFT + SANCTIONS
- `modules/trade/trade-documents/module.yaml` — document types, operations, standards
- `modules/trade/letters-of-credit/module.yaml` — UCP 600, LC workflow, document checking

**Implementation patterns** (follow these exactly):
- `mez/crates/mez-corridor/src/receipt.rs` — receipt chain: how transitions become receipts
- `mez/crates/mez-corridor/src/lib.rs` — crate module registration
- `mez/crates/mez-core/src/canonical.rs` — `CanonicalBytes`: all digests go through this
- `mez/crates/mez-api/src/routes/mass_proxy.rs` — orchestration handler pattern: validate → compliance → execute → VC → attestation → audit
- `mez/crates/mez-api/src/routes/sovereign_ops.rs` — sovereign operation functions (pattern for trade ops)
- `mez/crates/mez-api/src/orchestration.rs` — `evaluate_compliance()`, `check_hard_blocks()`, `issue_compliance_vc()`, `store_attestation()` — backend-agnostic
- `mez/crates/mez-api/src/routes/corridors.rs` — existing corridor API endpoints (style reference)
- `mez/crates/mez-api/src/routes/mod.rs` — route module registration
- `mez/crates/mez-api/src/lib.rs` — router composition
- `mez/crates/mez-api/src/state.rs` — `AppState` struct, `Store<T>` pattern, `DashMap` stores
- `mez/crates/mez-api/src/db/mass_primitives.rs` — Postgres persistence pattern: `save_*`, `load_all_*`
- `mez/crates/mez-api/migrations/` — SQL migration file naming convention
- `mez/crates/mez-schema/src/validate.rs` — `SchemaValidator` for runtime document validation
- `mez/crates/mez-api/src/db/audit.rs` — `AuditEvent` and `append()` for audit trail

Read `CLAUDE.md` for the full audit context, invariant registry, and architectural constraints.

## Architecture

Trade flows are corridors with trade-typed transitions. A trade flow is a corridor instance bound to a trade module template (export, import, LC, open-account). Each transition carries a trade document payload validated against the corresponding schema, CAS-stored as an artifact, and recorded in the corridor receipt chain via `CorridorReceipt`. The compliance tensor evaluates at every transition. VCs are issued per transition. Audit events are appended. The trade flow manager enforces state ordering per archetype — you cannot settle before shipping.

Nothing new is invented. Trade instruments compose existing primitives: `CanonicalBytes` for digests, `SchemaValidator` for document validation, `CorridorReceipt` for receipts, `evaluate_compliance()` for the tensor, `issue_compliance_vc()` for credentials, `audit::append()` for the trail. The trade layer is a typed, state-ordered interface on top of the corridor protocol.

## Deliverables

### 1. Trade types: `mez/crates/mez-corridor/src/trade.rs`

New module. Register `pub mod trade;` in `mez-corridor/src/lib.rs`.

**Document structs** — one per trade document schema. Derive `Debug, Clone, Serialize, Deserialize`. Field names match schema `properties` exactly (snake_case). Required schema fields are non-`Option`. Optional schema fields are `Option<T>`. `TradeAmount` has `currency: String` + `value: String`. `TradeParty` has `party_id: String` + optional `name`, `lei`, `did`, `address`, etc.

Structs needed: `TradeParty`, `TradeAmount`, `TradePartyAddress`, `TradeInvoice`, `InvoiceLineItem`, `BillOfLading`, `BolGoods`, `BolEndorsement`, `LetterOfCredit`, `LcDocumentRequirement`.

**Transition payloads** — one enum variant per transition payload schema. Each uses `oneOf` (embedded document or `ArtifactRef`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TradeTransitionPayload {
    #[serde(rename = "trade.invoice.issue.v1")]
    InvoiceIssue { invoice: Option<TradeInvoice>, invoice_ref: Option<ArtifactRef>, issued_by_party_id: Option<String>, notes: Option<String> },
    #[serde(rename = "trade.invoice.accept.v1")]
    InvoiceAccept { invoice_id: String, accepted: bool, discrepancies: Option<Vec<String>>, notes: Option<String> },
    #[serde(rename = "trade.invoice.settle.v1")]
    InvoiceSettle { invoice_id: String, settlement_amount: TradeAmount, settlement_ref: Option<String>, notes: Option<String> },
    #[serde(rename = "trade.bol.issue.v1")]
    BolIssue { bol: Option<BillOfLading>, bol_ref: Option<ArtifactRef>, issued_by_party_id: Option<String>, notes: Option<String> },
    #[serde(rename = "trade.bol.endorse.v1")]
    BolEndorse { bol_id: String, from_party_id: String, to_party_id: String, notes: Option<String> },
    #[serde(rename = "trade.bol.release.v1")]
    BolRelease { bol_id: String, released_to_party_id: String, notes: Option<String> },
    #[serde(rename = "trade.lc.issue.v1")]
    LcIssue { lc: Option<LetterOfCredit>, lc_ref: Option<ArtifactRef>, issued_by_party_id: Option<String>, notes: Option<String> },
    #[serde(rename = "trade.lc.amend.v1")]
    LcAmend { lc_id: String, amendments: serde_json::Value, notes: Option<String> },
    #[serde(rename = "trade.lc.present.v1")]
    LcPresent { lc_id: String, documents: Vec<ArtifactRef>, presented_by_party_id: Option<String>, notes: Option<String> },
    #[serde(rename = "trade.lc.honor.v1")]
    LcHonor { lc_id: String, honored: bool, discrepancies: Option<Vec<String>>, settlement_amount: Option<TradeAmount>, notes: Option<String> },
}
```

Adapt the exact field names from the transition payload schemas. Use `ArtifactRef` from `mez-core` (or define locally if not exported — check `schemas/artifact-ref.schema.json` for the shape).

**Trade flow state**:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeFlowState {
    Created,
    InvoiceIssued,
    InvoiceAccepted,
    GoodsShipped,      // After bol.issue
    BolEndorsed,
    GoodsReleased,     // After bol.release
    LcIssued,
    DocumentsPresented,
    LcHonored,
    SettlementInitiated,
    Settled,
}
```

**Trade flow type** (archetype):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TradeFlowType {
    Export,          // invoice.issue → invoice.accept → bol.issue → bol.endorse → bol.release → invoice.settle
    Import,          // invoice.issue → invoice.accept → bol.release → invoice.settle
    LetterOfCredit,  // lc.issue → bol.issue → lc.present → lc.honor → invoice.settle
    OpenAccount,     // invoice.issue → bol.issue → bol.release → invoice.settle
}
```

**Transition validation** — function that takes `(flow_type, current_state, transition_kind) -> Result<TradeFlowState, TradeError>`. Encodes the valid state transitions per archetype as a match table. Returns the next state or error if the transition is invalid for the current state.

**Content digest** — `compute_trade_document_digest(doc: &impl Serialize) -> Result<ContentDigest>` using `CanonicalBytes::from_value()` then `SHA256`.

**Unit tests in-module** — test each archetype's full transition sequence. Test invalid transitions return errors. Test serde round-trip for all document types. Test digest determinism (same doc → same digest).

### 2. Trade flow manager: `mez/crates/mez-corridor/src/trade_manager.rs`

Register `pub mod trade_manager;` in `mez-corridor/src/lib.rs`.

```rust
pub struct TradeFlowRecord {
    pub flow_id: Uuid,
    pub corridor_id: Option<Uuid>,
    pub flow_type: TradeFlowType,
    pub state: TradeFlowState,
    pub seller: TradeParty,
    pub buyer: TradeParty,
    pub transitions: Vec<TradeTransitionRecord>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct TradeTransitionRecord {
    pub transition_id: Uuid,
    pub kind: String,                    // e.g., "trade.invoice.issue.v1"
    pub from_state: TradeFlowState,
    pub to_state: TradeFlowState,
    pub payload: serde_json::Value,
    pub document_digests: Vec<String>,   // SHA-256 hex of CAS-stored documents
    pub receipt_digest: Option<String>,  // corridor receipt digest, if produced
    pub created_at: DateTime<Utc>,
}

pub struct TradeFlowManager {
    flows: DashMap<Uuid, TradeFlowRecord>,
}

impl TradeFlowManager {
    pub fn new() -> Self;
    pub fn create_flow(&self, flow_type: TradeFlowType, seller: TradeParty, buyer: TradeParty) -> TradeFlowRecord;
    pub fn submit_transition(&self, flow_id: Uuid, payload: TradeTransitionPayload) -> Result<TradeFlowRecord, TradeError>;
    pub fn get_flow(&self, flow_id: &Uuid) -> Option<TradeFlowRecord>;
    pub fn list_flows(&self) -> Vec<TradeFlowRecord>;
}
```

`submit_transition` must:
1. Look up the flow (404 if missing)
2. Call `validate_transition(flow.flow_type, flow.state, &payload)` to get next state
3. Extract embedded documents, compute their digests via `compute_trade_document_digest`
4. Record the transition with digests
5. Advance the flow state
6. Update `updated_at`

### 3. API endpoints: `mez/crates/mez-api/src/routes/trade.rs`

Register `pub mod trade;` in `routes/mod.rs`. Add `trade::router()` to the router merge chain in `lib.rs`.

**Request/response types** — define in the same file:
```rust
#[derive(Deserialize)]
pub struct CreateTradeFlowRequest {
    pub flow_type: TradeFlowType,
    pub seller: TradeParty,
    pub buyer: TradeParty,
    pub jurisdiction_id: Option<String>,
}

#[derive(Deserialize)]
pub struct SubmitTransitionRequest {
    pub payload: TradeTransitionPayload,
    pub jurisdiction_id: Option<String>,
}
```

**Endpoints**:

| Method | Path | Handler |
|--------|------|---------|
| `POST` | `/v1/trade/flows` | `create_trade_flow` |
| `GET` | `/v1/trade/flows` | `list_trade_flows` |
| `GET` | `/v1/trade/flows/:flow_id` | `get_trade_flow` |
| `POST` | `/v1/trade/flows/:flow_id/transitions` | `submit_transition` |
| `GET` | `/v1/trade/flows/:flow_id/transitions` | `list_transitions` |

**Write handler pattern** (follow `mass_proxy.rs`):
```rust
async fn submit_transition(
    State(state): State<AppState>,
    Path(flow_id): Path<Uuid>,
    Json(req): Json<SubmitTransitionRequest>,
) -> Result<impl IntoResponse, AppError> {
    let jurisdiction = req.jurisdiction_id.as_deref().unwrap_or("pk-sifc");

    // 1. Pre-flight compliance
    let (_, summary) = orchestration::evaluate_compliance(jurisdiction, "trade-transition", &["aml", "kyc", "sanctions", "tax"]);
    if let Some(reason) = orchestration::check_hard_blocks(&summary) {
        return Err(AppError::Forbidden(reason));
    }

    // 2. Execute transition
    let record = state.trade_flow_manager.submit_transition(flow_id, req.payload)
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // 3. Orchestrate (VC + attestation)
    let mass_response = serde_json::to_value(&record)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    let envelope = orchestration::orchestrate_trade_transition(&state, jurisdiction, &record, mass_response);

    // 4. Persist
    if let Some(ref pool) = state.db_pool {
        db::trade::save_trade_flow(pool, &record).await.ok();
    }

    // 5. Audit trail
    if let Some(ref pool) = state.db_pool {
        let _ = db::audit::append(pool, db::audit::AuditEvent {
            event_type: format!("trade.transition.{}", record.transitions.last().map(|t| t.kind.as_str()).unwrap_or("unknown")),
            actor_did: Some(state.zone_did.clone()),
            resource_type: "trade_flow".to_string(),
            resource_id: Some(flow_id),
            action: "transition".to_string(),
            metadata: serde_json::json!({
                "flow_type": record.flow_type,
                "from_state": record.transitions.last().map(|t| &t.from_state),
                "to_state": record.state,
            }),
        }).await;
    }

    Ok((StatusCode::OK, Json(envelope)))
}
```

Note: `orchestrate_trade_transition` may not exist yet in `orchestration.rs`. If the existing `orchestrate_*` functions don't fit trade flows, add a thin `orchestrate_trade_transition()` function in `orchestration.rs` that follows the same pattern as `orchestrate_entity_creation()` — compose compliance summary + VC + attestation into an `OrchestrationEnvelope`. If `OrchestrationEnvelope` works directly, use it. If trade needs a different envelope shape, define `TradeOrchestrationEnvelope` in the trade route module. Prefer reusing the existing envelope.

### 4. AppState wiring: `mez/crates/mez-api/src/state.rs`

Add `pub trade_flow_manager: Arc<TradeFlowManager>` to `AppState`. Initialize in the constructor. Hydrate from DB on startup (call `db::trade::load_all_trade_flows()` → populate the DashMap).

### 5. Persistence: `mez/crates/mez-api/src/db/trade.rs`

Register `pub mod trade;` in `db/mod.rs`.

**Migration**: `mez/crates/mez-api/migrations/20260221000001_trade_flow_tables.sql`
```sql
CREATE TABLE IF NOT EXISTS trade_flows (
    flow_id UUID PRIMARY KEY,
    corridor_id UUID,
    flow_type TEXT NOT NULL,
    state TEXT NOT NULL,
    seller JSONB NOT NULL,
    buyer JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS trade_transitions (
    transition_id UUID PRIMARY KEY,
    flow_id UUID NOT NULL REFERENCES trade_flows(flow_id),
    kind TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    payload JSONB NOT NULL,
    document_digests JSONB NOT NULL DEFAULT '[]',
    receipt_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_trade_transitions_flow_id ON trade_transitions(flow_id);
```

**Functions**: `save_trade_flow`, `save_trade_transition`, `load_all_trade_flows`, `load_transitions_for_flow`. Follow the `mass_primitives.rs` pattern with `sqlx::query!` or `sqlx::query_as!`. Handle the case where `db_pool` is `None` (dev mode without Postgres).

### 6. Integration test: `mez/crates/mez-integration-tests/tests/trade_flow_test.rs`

```rust
#[tokio::test]
async fn export_flow_full_lifecycle() {
    // Build AppState with trade_flow_manager
    // Build router from app(state)
    // 1. POST /v1/trade/flows — create Export flow with seller (pk-sifc) and buyer (ae-abudhabi-adgm)
    //    Assert: 201, flow_id returned, state = Created
    // 2. POST /v1/trade/flows/{id}/transitions — InvoiceIssue with embedded TradeInvoice
    //    Assert: 200, state = InvoiceIssued, compliance evaluated, document_digests non-empty
    // 3. POST transitions — InvoiceAccept
    //    Assert: state = InvoiceAccepted
    // 4. POST transitions — BolIssue with embedded BillOfLading
    //    Assert: state = GoodsShipped
    // 5. POST transitions — BolEndorse
    //    Assert: state = BolEndorsed
    // 6. POST transitions — BolRelease
    //    Assert: state = GoodsReleased
    // 7. POST transitions — InvoiceSettle
    //    Assert: state = Settled
    // 8. GET /v1/trade/flows/{id} — verify complete history (7 transitions)
    // 9. GET /v1/trade/flows/{id}/transitions — verify all 7 records
    // 10. Negative: POST transitions on Settled flow — 400 (no valid transitions from Settled)
}

#[tokio::test]
async fn lc_flow_lifecycle() {
    // 1. Create LetterOfCredit flow
    // 2. LcIssue → LcIssued
    // 3. BolIssue → GoodsShipped
    // 4. LcPresent → DocumentsPresented
    // 5. LcHonor → LcHonored
    // 6. InvoiceSettle → Settled
    // Assert: complete lifecycle, all transitions recorded
}

#[tokio::test]
async fn invalid_transition_rejected() {
    // Create Export flow
    // Attempt InvoiceSettle on Created state (skipping invoice issue)
    // Assert: 400 with clear error message naming the invalid transition
}

#[tokio::test]
async fn document_digest_determinism() {
    // Create identical TradeInvoice twice
    // compute_trade_document_digest on both
    // Assert: digests are identical
}
```

Use `tower::ServiceExt::oneshot` for testing without starting a server. Match the test infrastructure pattern in existing integration tests (`sovereign_orchestration_test.rs`).

### 7. Update CLAUDE.md

Add to Section 5, after Phase G:
```
### Phase H: Trade Corridor Instruments (v0.4.37)

36. Trade document Rust types + flow state machine — CLOSED
37. Trade flow API endpoints with orchestration — CLOSED
38. Trade flow persistence (Postgres) — CLOSED
39. End-to-end trade flow integration tests — CLOSED
```

Update Section 9 (Coverage Matrix), row for corridors, to note trade instruments implemented.

## Invariants

- I-CANON: All trade document digests computed via `SHA256(CanonicalBytes)`.
- I-RECEIPT-LINK/COMMIT: Trade transitions that produce corridor receipts must satisfy receipt chain invariants.
- Amounts are always `TradeAmount { currency: String, value: String }`. The `value` field matches `^[0-9]+(\.[0-9]{1,18})?$`. Never use `f64`.
- Dates are ISO 8601 strings in serialization. Use `String` for `date` fields (`YYYY-MM-DD`), `DateTime<Utc>` for `date-time` fields.
- `additionalProperties: false` is already set on all trade schemas. Rust struct fields must match schema properties exactly — no extra fields, no missing required fields.
- State transitions are validated at runtime per archetype. Invalid transitions return `TradeError`, not panic.
- Audit trail appends are fire-and-forget (`let _ = ... .await;`). Never block the response on audit.
- BUSL-1.1 license header on all new `.rs` files.
- No `unsafe`. No `unwrap()` in non-test code.

## Constraints

- Do NOT modify existing trade schemas (`schemas/trade.*.schema.json`), transition payload schemas, rulesets, or `registries/transition-types.yaml`. These are the spec. Your code implements them.
- Do NOT modify `orchestration.rs` unless adding a thin `orchestrate_trade_transition()` function. Do not change existing `orchestrate_*` functions.
- Do NOT modify `receipt.rs`, `canonical.rs`, or `mez-core`.
- Do NOT add new compliance tensor domains. Trade evaluates existing domains: `aml`, `kyc`, `sanctions`, `tax`.
- Do NOT introduce TypeState for trade flows. Use runtime state enum. The corridor FSM uses TypeState because corridor states are a compile-time safety property; trade flow states vary by archetype and are data-driven.
- Preserve existing test suite. Run `cargo test --workspace` before each commit.

## Commit structure

1. `feat: trade document types and flow state machine (mez-corridor)` — `trade.rs`, `trade_manager.rs`, unit tests
2. `feat: trade flow API endpoints with orchestration pipeline (mez-api)` — `routes/trade.rs`, `db/trade.rs`, migration SQL, AppState wiring
3. `feat: end-to-end trade flow integration tests` — `trade_flow_test.rs` with export, LC, and negative-case tests
4. `docs: update CLAUDE.md with Phase H trade corridor instruments` — CLAUDE.md update

Run `cargo test --workspace` before each commit. Push to the designated branch when complete.
