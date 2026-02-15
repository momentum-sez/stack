# Bug Ledger — Test Hardening Sessions

| BUG | Severity | Crate | Campaign | Description | Status |
|-----|----------|-------|----------|-------------|--------|
| BUG-001 | P2 | msez-state | 1 | TransitionRecord missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-002 | P2 | msez-state | 1 | DynCorridorData missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-003 | P2 | msez-corridor | 1 | Obligation missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-004 | P2 | msez-corridor | 1 | NetPosition missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-005 | P2 | msez-corridor | 1 | SettlementLeg missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-006 | P2 | msez-corridor | 1 | SettlementPlan missing PartialEq derive; reduction_percentage uses f64 (float precision risk) | DEFERRED — f64 field blocks derive |
| BUG-007 | P2 | msez-corridor | 1 | SettlementInstruction missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-008 | P2 | msez-vc | 1 | Proof missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-009 | P2 | msez-vc | 1 | VerifiableCredential missing PartialEq derive | **RESOLVED** — PartialEq added (with ContextValue, CredentialTypeValue, ProofValue) |
| BUG-010 | P2 | msez-agentic | 1 | AuditEntry missing PartialEq derive | **RESOLVED** — manual PartialEq impl already exists (intentionally omits timestamp) |
| BUG-011 | P2 | msez-tensor | 1 | TensorCell missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-012 | P2 | msez-tensor | 1 | TensorCell.determined_at is raw String instead of Timestamp newtype — no validation | DEFERRED — requires type migration |
| BUG-013 | P1 | msez-core | 1 | Did serde Deserialize bypasses format validation — invalid DIDs accepted | **RESOLVED** — custom Deserialize validates via Did::new() |
| BUG-014 | P1 | msez-core | 1 | Ntn serde Deserialize bypasses 7-digit validation | **RESOLVED** — custom Deserialize validates via Ntn::new() |
| BUG-015 | P1 | msez-core | 1 | Cnic serde Deserialize bypasses 13-digit validation | **RESOLVED** — custom Deserialize validates via Cnic::new() |
| BUG-016 | P1 | msez-core | 1 | PassportNumber serde Deserialize bypasses length validation | **RESOLVED** — custom Deserialize validates via PassportNumber::new() |
| BUG-017 | P1 | msez-core | 1 | JurisdictionId serde Deserialize bypasses empty-string validation | **RESOLVED** — custom Deserialize validates via JurisdictionId::new() |
| BUG-018 | P0 | msez-corridor | 1 | NettingEngine gross_total computation uses i64 sum — overflow on large amounts | **RESOLVED** — checked_add returns NettingError::ArithmeticOverflow |
| BUG-019 | P0 | msez-core | 2 | Did::method() panics on serde-deserialized invalid DID (expect("validated at construction") bypassed) | **RESOLVED** — BUG-013 fix makes this panic path unreachable |
| BUG-020 | P2 | msez-corridor | 2 | NettingEngine accepts self-obligations (from_party == to_party) without validation | **RESOLVED** — NettingError::InvalidParties returned |
| BUG-021 | P2 | msez-corridor | 2 | NettingEngine accepts empty party ID strings without validation | **RESOLVED** — NettingError::InvalidParties returned |
| BUG-022 | P2 | msez-corridor | 2 | NettingEngine accepts empty currency strings without validation | **RESOLVED** — NettingError::InvalidCurrency returned |

## Resolution Summary (Session 4 — 2026-02-15)

**20 of 22 bugs resolved** in this session:
- **2 P0 bugs** (BUG-018, BUG-019): Production panic and silent data corruption eliminated
- **5 P1 bugs** (BUG-013 through BUG-017): Serde validation bypass closed for all identity newtypes
- **13 P2 bugs** resolved: PartialEq derives added, NettingEngine validation hardened

**2 bugs remain DEFERRED** (P2):
- BUG-006: SettlementPlan has f64 `reduction_percentage` field that blocks PartialEq derive
- BUG-012: TensorCell.determined_at should use Timestamp newtype (type migration needed)
