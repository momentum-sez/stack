# Next Session: Trade Corridor Instruments — Live Runtime

Copy everything below the `---` line into "Ask Claude to write code..."

---

Build the Trade Corridor Instruments runtime. The entire spec layer exists — 5 document schemas, 10 transition payload schemas, 10 rulesets, 10 registered transition types with CAS digests, 5 module descriptors. Zero Rust implementation. Wire trade flows as first-class corridor operations: schema-validated, receipt-chained, compliance-gated, settlement-integrated, cross-zone-propagated, Postgres-persisted.

Read CLAUDE.md. Then read these files before writing any code:

Specification (the source of truth — do not modify):
- `schemas/trade.invoice.v1.schema.json` — invoice shape
- `schemas/trade.bill-of-lading.v1.schema.json` — BOL shape
- `schemas/trade.letter-of-credit.v1.schema.json` — LC shape
- `schemas/trade.party.v1.schema.json` — party shape
- `schemas/trade.amount.v1.schema.json` — amount shape (decimal string, never float)
- `schemas/transition.payload.trade.invoice.{issue,accept,settle}.v1.schema.json` — invoice transition payloads
- `schemas/transition.payload.trade.bol.{issue,endorse,release}.v1.schema.json` — BOL transition payloads
- `schemas/transition.payload.trade.lc.{issue,amend,present,honor}.v1.schema.json` — LC transition payloads
- `rulesets/mez.transition.trade.*.v1.json` — 10 validation profiles
- `registries/transition-types.yaml` — trade entries (lines 39-131) with CAS digests
- `modules/trade/module.yaml` — trade module family, compliance domains

Implementation patterns (read to absorb the architecture):
- `mez/crates/mez-corridor/src/receipt.rs` — how transitions become corridor receipts
- `mez/crates/mez-corridor/src/netting.rs` — settlement netting engine (Obligation, Party, NettingPlan)
- `mez/crates/mez-corridor/src/network.rs` — inter-zone peer exchange protocol
- `mez/crates/mez-corridor/src/lib.rs` — crate module structure
- `mez/crates/mez-core/src/canonical.rs` — CanonicalBytes: all digests via SHA256(JCS)
- `mez/crates/mez-schema/src/validate.rs` — SchemaValidator for runtime Draft 2020-12 validation
- `mez/crates/mez-api/src/orchestration.rs` — evaluate_compliance, check_hard_blocks, issue_compliance_vc, store_attestation, OrchestrationEnvelope
- `mez/crates/mez-api/src/routes/corridors.rs` — corridor API pattern (style reference for trade endpoints)
- `mez/crates/mez-api/src/routes/sovereign_ops.rs` — sovereign operation functions
- `mez/crates/mez-api/src/state.rs` — AppState, Store<T>, hydrate_from_db pattern
- `mez/crates/mez-api/src/db/mass_primitives.rs` — Postgres persistence: save_*, load_all_*
- `mez/crates/mez-api/src/db/audit.rs` — AuditEvent and append()
- `mez/crates/mez-api/src/auth.rs` — CallerIdentity, Role, require_role, RBAC
- `mez/crates/mez-api/src/lib.rs` — router composition
- `mez/crates/mez-api/migrations/` — SQL migration naming convention

What to build (6 deliverables):

1. Trade types and flow state machine in `mez/crates/mez-corridor/src/trade.rs`. Derive Rust structs from the 5 document schemas — field names match schema properties exactly, required fields are non-Option, optional fields are Option<T>. TradeAmount uses String for value (decimal, never f64). Build TradeTransitionPayload as a tagged enum with one variant per transition payload schema. Build TradeFlowState enum and TradeFlowType enum (Export, Import, LetterOfCredit, OpenAccount). Implement validate_transition(flow_type, current_state, transition_kind) -> Result<TradeFlowState, TradeError> as a match table encoding valid state orderings per archetype. Implement compute_trade_document_digest using CanonicalBytes + SHA256. Unit tests: full lifecycle per archetype, invalid transition rejection, serde roundtrip, digest determinism.

2. Trade flow manager in `mez/crates/mez-corridor/src/trade_manager.rs`. TradeFlowRecord with flow_id, corridor_id, flow_type, state, seller, buyer, transitions vec, timestamps. TradeFlowManager with create_flow, submit_transition, get_flow, list_flows. submit_transition validates the transition, extracts embedded documents, computes their content digests, records the transition, advances state. No orchestration concern here — that belongs in the API layer.

3. Trade API endpoints in `mez/crates/mez-api/src/routes/trade.rs`. Five endpoints: POST /v1/trade/flows (create), GET /v1/trade/flows (list), GET /v1/trade/flows/:flow_id (get), POST /v1/trade/flows/:flow_id/transitions (submit), GET /v1/trade/flows/:flow_id/transitions (list transitions). Every write handler follows the existing orchestration pattern: pre-flight compliance evaluation via evaluate_compliance → hard-block check → execute trade operation → issue compliance VC → store attestation → append audit event. RBAC: zone_admin and regulator can list all flows; entity_operator can only access flows where they are seller or buyer. Add orchestrate_trade_transition() to orchestration.rs following the existing orchestrate_entity_creation pattern. Wire trade::router() into the main router in lib.rs.

4. AppState wiring and Postgres persistence. Add trade_flows: Store<TradeFlowRecord> to AppState. Add db/trade.rs with save_trade_flow, save_trade_transition, load_all_trade_flows, load_transitions_for_flow following the mass_primitives.rs pattern. Add migration 20260221000001_trade_flow_tables.sql with trade_flows and trade_transitions tables. Hydrate trade flows from DB on startup in hydrate_from_db.

5. Integration tests in a new test file. Full export lifecycle test (create → 7 transitions → settled → verify history → reject post-settlement transition). Full LC lifecycle test. Invalid transition rejection test. Document digest determinism test. Use tower::ServiceExt::oneshot — match the test infrastructure pattern in existing integration tests. Every test creates its own AppState, builds the router, exercises the HTTP endpoints end-to-end.

6. Update CLAUDE.md Section 5 with Phase H (Trade Corridor Instruments). Update Section 9 coverage matrix.

Invariants (these are non-negotiable):

- I-CANON: All trade document digests computed via SHA256(CanonicalBytes). No exceptions.
- Amounts are TradeAmount { currency: String, value: String } where value matches ^[0-9]+(\.[0-9]{1,18})?$. Never f64.
- Dates: String for date fields (YYYY-MM-DD), DateTime<Utc> for date-time fields.
- additionalProperties: false is set on all trade schemas. Rust struct fields match schema properties exactly.
- State transitions validated at runtime per archetype. Invalid transitions return TradeError, never panic.
- Audit trail appends are fire-and-forget (let _ = ... .await;). Never block response on audit.
- No unsafe. No unwrap() in non-test code.

Constraints:

- Do NOT modify existing schemas, rulesets, registries, or transition-types.yaml.
- Do NOT modify receipt.rs, canonical.rs, or mez-core.
- Do NOT modify existing orchestration functions — only add orchestrate_trade_transition().
- Do NOT add new compliance tensor domains. Trade evaluates: aml, kyc, sanctions, tax.
- Do NOT use TypeState for trade flows. Use runtime state enum — trade states vary by archetype.
- Preserve all existing tests. Run cargo test --workspace before each commit.

Commit structure:

1. `feat: trade document types and flow state machine` — trade.rs, trade_manager.rs, unit tests in mez-corridor
2. `feat: trade flow API endpoints with full orchestration pipeline` — routes/trade.rs, db/trade.rs, migration SQL, AppState wiring, orchestration.rs addition
3. `feat: end-to-end trade flow integration tests` — lifecycle tests for export, LC, negative cases, digest determinism
4. `docs: update CLAUDE.md with Phase H trade corridor instruments` — sections 5 and 9

Run cargo test --workspace before each commit. Push to the designated branch when complete.
