# Claude Code Session Prompt — Cross-Border Migration Saga API

> Paste everything below the line into "Ask Claude to write code..."

---

Implement the complete cross-border migration saga API surface — wiring `MigrationSaga` from `mez-corridor/src/migration.rs` into the `mez-api` HTTP layer with persistence-backed CAS, corridor receipt emission per lifecycle step, and Raast payment rail integration for settlement. This closes P0-MIGRATION-001 (the last open P0 item in our control) and delivers the core revenue operation: cross-border asset transfer between two sovereign zones with provable atomic compensation.

## What exists (read these files first — do not re-implement what's already built)

**Migration saga model (COMPLETE):** `mez/crates/mez-corridor/src/migration.rs`
- `MigrationSaga` struct with CAS versioning, `advance()`, `compensate()`, side-effect model (`Lock/Unlock/Mint/Burn` with `inverse()`), deadline enforcement, idempotent compensation, `no_duplicate_invariant()`, 12 unit tests. The in-memory model is production-grade. What's missing is the API surface and persistence integration.

**API patterns to follow exactly:** `mez/crates/mez-api/src/routes/corridors.rs` and `mez/crates/mez-api/src/routes/settlement.rs`
- Axum Router with `State<AppState>` extractor
- Request types implementing `Validate` trait from `crate::extractors`
- Response types deriving `Serialize, ToSchema`
- Error handling via `AppError` from `crate::error`
- `extract_validated_json` for input validation

**State management pattern:** `mez/crates/mez-api/src/state.rs`
- `Store<T>` — thread-safe in-memory KV store with `try_update()` for atomic read-validate-update (eliminates TOCTOU). This is the CAS integration point — use `try_update()` to enforce version checks inside the write lock.
- `AppState` holds `Store<CorridorRecord>`, `Store<SmartAssetRecord>`, etc. Add `Store<MigrationRecord>` following the same pattern.

**Receipt chain integration:** `mez/crates/mez-corridor/src/receipt.rs`
- `ReceiptChain::append()` takes a `CorridorReceipt` and enforces `I-RECEIPT-LINK` and `I-RECEIPT-COMMIT` invariants.
- Each migration step should emit a corridor receipt on the corridor connecting source and destination zones. The receipt's digest set should include the migration ID and step name.

**Raast payment rail:** `mez/crates/mez-mass-client/src/raast.rs`
- `RaastAdapter` trait with `initiate_payment`, `check_payment_status`, `verify_account`, `lookup_by_alias`. `MockRaastAdapter` is fully implemented. Wire this into the settlement step of the migration (when `DestinationMinted → Completed`, optionally trigger a Raast payment instruction).

**OpenAPI spec pattern:** `apis/corridor-state.openapi.yaml`
- Bearer auth, `ProblemDetail` error model, `Idempotency-Key` header on mutations, `additionalProperties: false` on all request schemas, `$ref` to canonical schema URIs.

## Deliverables (in execution order)

### 1. `MigrationRecord` in `mez-api/src/state.rs`

Add `MigrationRecord` as the API-layer representation of `MigrationSaga`. Add `Store<MigrationRecord>` to `AppState`. The record should include: `id`, `version`, `state` (serialized as `SCREAMING_SNAKE_CASE` like `AssetStatus`), `source_zone`, `dest_zone`, `asset_id`, `corridor_id` (the corridor connecting the two zones), `deadline`, `forward_effects`, `compensation_effects`, `created_at`, `updated_at`, and `settlement_reference` (optional Raast payment ID).

### 2. `mez-api/src/routes/migrations.rs` — Full CRUD + lifecycle

Endpoints:

```
POST   /v1/migrations                       — Create migration saga
GET    /v1/migrations                        — List migrations (paginated)
GET    /v1/migrations/{migration_id}         — Get migration by ID
POST   /v1/migrations/{migration_id}/advance — Advance to next state (CAS-protected)
POST   /v1/migrations/{migration_id}/compensate — Trigger compensation (idempotent)
GET    /v1/migrations/{migration_id}/effects — Get side-effect audit trail
```

**Create**: Validate that `source_zone != dest_zone`, `asset_id` is non-empty, `deadline` is in the future, and a corridor exists between source and dest zones (look up in the corridor store). Instantiate `MigrationSaga::new()`, wrap in `MigrationRecord`, insert into store.

**Advance**: Use `Store::try_update()` to atomically read the saga, call `saga.advance(expected_version)` inside the write lock, and emit a corridor receipt on the corridor linking the two zones. The `expected_version` comes from the request body. On the `DestinationMinted → Completed` transition (the final step), if a `raast_account` field is provided in the request, initiate a Raast payment via `MockRaastAdapter` and store the payment reference.

**Compensate**: Use `Store::try_update()` to call `saga.compensate(expected_version)`. Must be idempotent — if already compensated, return 200 with current state, not an error.

**Error mapping**: Map `MigrationError` variants to HTTP status codes:
- `AlreadyTerminal` → 409 Conflict
- `VersionConflict` → 409 Conflict
- `TimedOut` → 408 Request Timeout (or 409 with timeout detail)
- `InvalidTransition` → 422 Unprocessable Entity

### 3. Receipt emission on each migration step

When `advance()` succeeds, construct a `CorridorReceipt` and append it to the receipt chain of the corridor connecting source ↔ dest zones. The receipt should:
- Set `corridor_id` to the corridor linking the two zones
- Include `migration_id` and the new state name in the digest set
- Follow the `compute_next_root()` path (do NOT set `next_root` manually — let the receipt chain compute it)
- Include `prev_root` from the current chain head

This connects the migration lifecycle to the corridor's cryptographic audit trail — every migration step is a verifiable receipt.

### 4. OpenAPI spec: `apis/migrations.openapi.yaml`

Create a new OpenAPI 3.0.3 spec following the exact pattern of `corridor-state.openapi.yaml`:
- `info.version: "0.1.0"`
- `bearerAuth` security scheme
- `Idempotency-Key` header on `POST /v1/migrations` and `POST .../advance`
- `ProblemDetail` error model on all error responses
- `additionalProperties: false` on all request/response schemas
- Server URL: `https://{zone_node}/api`

### 5. Wire into router in `mez-api/src/lib.rs` or `routes/mod.rs`

Add the migrations router to the main app, nested under `/v1/migrations`. Follow how corridors and settlement routes are mounted.

### 6. Integration tests: `mez/crates/mez-integration-tests/tests/test_migration_saga_e2e.rs`

Write tests proving the complete cross-border transfer lifecycle:

1. **Happy path**: Create migration → advance through all 3 steps → verify terminal state `Completed` → verify 3 corridor receipts emitted → verify receipt chain integrity (prev_root linkage)
2. **Compensation path**: Create → advance to `SourceLocked` → compensate → verify terminal state `Compensated` → verify `Unlock` effect in compensation log → verify compensation receipt emitted
3. **Idempotent compensation**: Compensate twice on same saga → second call returns 200 with same state, not error
4. **CAS conflict**: Two concurrent advance calls with same version → one succeeds, one gets 409
5. **Deadline enforcement**: Create migration with deadline in the past → advance → returns timeout error with compensation executed
6. **No-duplicate invariant**: At every step, assert `no_duplicate_invariant()` holds
7. **Cross-zone receipt verification**: After full migration, verify the corridor receipt chain between the two zones contains the migration receipts with correct digest sets
8. **Raast settlement**: Create migration with `raast_account` → advance to completion → verify `settlement_reference` is populated

### 7. Update two-zone demo script

Add a migration flow to `deploy/scripts/demo-two-zone.sh` after the existing corridor establishment:
```
# Step 11b: Cross-border migration (asset transfer PK-SIFC → AE-DIFC)
# POST /v1/migrations → advance × 3 → verify receipts on corridor
```

## Constraints

- Do NOT modify `mez-corridor/src/migration.rs` — the model is complete and tested. Build ON TOP of it.
- Do NOT add Postgres persistence yet — use the existing `Store<T>` in-memory pattern. Database integration is a separate task.
- Do NOT implement a real Raast HTTP client — use `MockRaastAdapter`. The trait boundary is the contract.
- Do NOT add new crate dependencies without justification. Everything needed is already in the workspace.
- Do NOT over-engineer — no generic migration framework, no plugin system, no webhook callbacks. Just the six endpoints, receipt emission, and tests.
- Follow existing code style exactly: `utoipa::ToSchema` derives, `serde(rename_all)` conventions, `Validate` trait pattern, `AppError` mapping.

## Success criteria

1. `cargo build --workspace` succeeds with zero warnings
2. `cargo test --workspace` passes with zero failures (existing 4,073+ tests plus new migration tests)
3. All 8 integration test scenarios listed above pass
4. The OpenAPI spec validates as valid OpenAPI 3.0.3
5. `POST /v1/migrations/{id}/advance` emits a corridor receipt on each step — verifiable via `GET /v1/corridors/{id}/receipts`
6. The two-zone demo script includes a migration flow and exits cleanly
7. `compensate()` called on an already-compensated saga returns 200, not 4xx/5xx
