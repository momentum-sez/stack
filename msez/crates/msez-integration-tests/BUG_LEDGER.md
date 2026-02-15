# Bug Ledger — Test Hardening Sessions

| BUG | Severity | Crate | Campaign | Description | Status |
|-----|----------|-------|----------|-------------|--------|
| BUG-001 | P2 | msez-state | 1 | TransitionRecord missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-002 | P2 | msez-state | 1 | DynCorridorData missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-003 | P2 | msez-corridor | 1 | Obligation missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq + Eq added |
| BUG-004 | P2 | msez-corridor | 1 | NetPosition missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq + Eq added |
| BUG-005 | P2 | msez-corridor | 1 | SettlementLeg missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq + Eq added |
| BUG-006 | P1 | msez-corridor | 1 | SettlementPlan.reduction_percentage uses f64 (float precision risk, blocks PartialEq/Eq derive) | **RESOLVED** — converted to reduction_bps: u32 (integer basis points), Eq derived |
| BUG-007 | P2 | msez-corridor | 1 | SettlementInstruction missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-008 | P2 | msez-vc | 1 | Proof missing PartialEq derive — cannot verify serde round-trip fidelity | **RESOLVED** — PartialEq added |
| BUG-009 | P2 | msez-vc | 1 | VerifiableCredential missing PartialEq derive | **RESOLVED** — PartialEq added (with ContextValue, CredentialTypeValue, ProofValue) |
| BUG-010 | P2 | msez-agentic | 1 | AuditEntry missing PartialEq derive | **RESOLVED** — manual PartialEq impl already exists (intentionally omits timestamp) |
| BUG-011 | P2 | msez-tensor | 1 | TensorCell missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-012 | P2 | msez-tensor | 1 | TensorCell.determined_at is raw String — no ISO 8601 validation | **RESOLVED** — custom serde deserializer validates ISO 8601 format |
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
| BUG-023 | P1 | msez-api | 7 | Mass proxy routes return inconsistent status codes | **RESOLVED** — unified to return 501 when no Mass client configured, validation second |
| BUG-024 | P2 | msez-api | 7 | Settlement compute endpoint accepts negative obligation amounts | **RESOLVED** — validation rejects non-positive amounts |
| BUG-025 | P2 | msez-arbitration | 4 | EnforcementOrder::block() allows Pending and InProgress — documented as design intent | **RESOLVED** — block() from both states is valid (appeal during execution) |
| BUG-026 | P2 | msez-corridor | 5 | ReceiptChain sequence is 0-indexed — undocumented | **RESOLVED** — doc updated on ReceiptChain::append() |
| BUG-027 | P2 | msez-corridor | 5 | ReceiptChain prev_root must match MMR root — undocumented invariant | **RESOLVED** — doc updated on ReceiptChain::append() |
| BUG-028 | P2 | msez-corridor | 5 | CorridorBridge::reachable_from() always includes source at distance 0 — undocumented | **RESOLVED** — doc updated on reachable_from() |
| BUG-029 | P2 | msez-agentic | 4 | ActionScheduler::mark_failed() retry semantics undocumented | **RESOLVED** — doc updated on mark_failed() |
| BUG-030 | P1 | msez-api | 7 | Treasury proxy routes return 404 instead of 501 | **RESOLVED** — routes return 501 with clear error message |
| BUG-031 | P1 | msez-api | 7 | Consent proxy routes return 405 instead of 501 | **RESOLVED** — routes return 501 with clear error message |
| BUG-032 | P1 | msez-api | 7 | Identity proxy routes return 404/405 | **RESOLVED** — routes return 501 with clear error message |
| BUG-033 | P1 | msez-pack | 1 | LawpackRef serde Deserialize bypasses validation — empty fields accepted | **RESOLVED** — custom Deserialize rejects empty fields and invalid SHA-256 |
| BUG-034 | P2 | msez-arbitration | 1 | Claim missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-035 | P2 | msez-arbitration | 1 | Dispute missing PartialEq derive | **RESOLVED** — PartialEq added |
| BUG-036 | P2 | msez-arbitration | 4 | EnforcementOrder allows cancel() from Blocked state — bypasses appeal process | **RESOLVED** — cancel() restricted: Blocked orders cannot be cancelled |
| BUG-037 | P2 | msez-state | 4 | MigrationSaga allows compensate() from pre-InTransit states | **RESOLVED** — compensate() restricted to InTransit, DestinationVerification, DestinationUnlock |
| BUG-038 | P2 | msez-api | 7 | API returns 400 for JSON errors vs 422 for validation — inconsistent taxonomy | **RESOLVED** — JSON parse errors now return 422 (Unprocessable Entity) |
| BUG-039 | P1 | msez-api | 7 | Mass proxy POST routes validate body before checking client availability | **RESOLVED** — 501 returned first when no Mass client |
| BUG-040 | P2 | msez-api | 7 | Trigger endpoint rejects empty data payload with 422 | **RESOLVED** — empty data accepted for event-only triggers |
| BUG-041 | P1 | msez-corridor | 3 | SettlementPlan.reduction_percentage (f64) incompatible with CanonicalBytes | **RESOLVED** — converted to reduction_bps: u32, full CanonicalBytes compatibility |
| BUG-042 | P2 | msez-pack | 5 | LicenseCondition.condition_id accepts empty strings | **RESOLVED** — is_valid() method added for validation |
| BUG-043 | P1 | msez-pack | 5 | is_active()/is_expired() use string comparison for dates — wrong results for non-canonical dates | **RESOLVED** — date_before() uses chrono::NaiveDate parsing with string fallback |
| BUG-044 | P2 | msez-pack | 5 | blocks_jurisdiction("") accepts empty string | **RESOLVED** — returns false for empty input |
| BUG-045 | P2 | msez-pack | 5 | get_licenses_by_holder_did("") matches empty DIDs | **RESOLVED** — returns empty vec for empty input |
| BUG-046 | P2 | msez-state | 5 | Watcher::rebond(0) succeeds without new collateral | **RESOLVED** — rejects zero stake with InsufficientStake error |
| BUG-047 | P2 | msez-state | 5 | rebond() inconsistent with bond() on zero-stake validation | **RESOLVED** — both reject zero stake consistently |
| BUG-048 | P1 | msez-zkp | 5 | MockProofSystem::prove() omits circuit_data from hash | **RESOLVED** — prove() now computes SHA256(canonical(circuit_data) \|\| public_inputs) |
| BUG-049 | P2 | msez-pack | 5 | License accepts empty required fields via serde | **RESOLVED** — validate() method added for field-level checks |
| BUG-050 | P2 | msez-pack | 5 | resolve_licensepack_refs() uses unwrap_or("") for missing fields | **RESOLVED** — entries with missing jurisdiction_id or domain are skipped |

## Resolution Summary

**Session 4 (2026-02-15): 22 of 22 bugs resolved** (BUG-001 through BUG-022):
- **2 P0 bugs** (BUG-018, BUG-019): Production panic and silent data corruption eliminated
- **5 P1 bugs** (BUG-013 through BUG-017): Serde validation bypass closed for all identity newtypes
- **15 P2 bugs** resolved: PartialEq/Eq derives added, NettingEngine validation hardened

**Session 5 (2026-02-15): 28 bugs discovered and resolved** (BUG-023 through BUG-050):
- **BUG-006/041** (P1): SettlementPlan.reduction_percentage f64 → reduction_bps u32 — eliminates float non-determinism, enables CanonicalBytes
- **BUG-048** (P1): MockProofSystem now binds proofs to circuit_data — SHA256(canonical(circuit) || inputs)
- **BUG-043** (P1): Date comparison uses chrono parsing — malformed dates handled correctly
- **BUG-033** (P1): LawpackRef custom Deserialize validates non-empty fields and SHA-256
- **BUG-037** (P2): MigrationSaga compensate() restricted to InTransit+
- **BUG-046/047** (P2): Watcher rebond(0) rejected — consistent with bond(0)
- **BUG-023/030-032/039** (P1): API status codes unified — 501 for unconfigured Mass client
- **BUG-038** (P2): JSON errors return 422 consistently
- **BUG-012** (P2): TensorCell.determined_at validates ISO 8601 on deserialization
- **BUG-025/034-036** (P2): Arbitration enforcement states, PartialEq derives
- **BUG-026-029** (P2): Documentation fixes for ReceiptChain, CorridorBridge, ActionScheduler
- **BUG-042/044/045/049/050** (P2): Licensepack validation hardened

**Final tally: 50 bugs catalogued, 50 resolved, 0 deferred**

### Validation

- `cargo check --workspace`: zero warnings
- `cargo clippy --workspace -- -D warnings`: clean
- `cargo test --workspace`: **3450 tests, 0 failures**
