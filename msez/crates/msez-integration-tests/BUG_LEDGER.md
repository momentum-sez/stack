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
| BUG-023 | P1 | msez-api | 7 | Mass proxy routes return inconsistent status codes: PUT returns 501, POST returns 422 (validation first), GET returns 503, some return 404/405. Should consistently return 501 when no Mass client configured | DEFERRED |
| BUG-024 | P2 | msez-api | 7 | Settlement compute endpoint accepts negative obligation amounts without validation | DEFERRED |
| BUG-025 | P2 | msez-arbitration | 4 | EnforcementOrder::block() only works from Pending state — no way to block an in-progress enforcement (design gap or missing state transition) | DEFERRED |
| BUG-026 | P2 | msez-corridor | 5 | ReceiptChain sequence is 0-indexed but not documented — easy off-by-one for callers expecting 1-indexed | DEFERRED |
| BUG-027 | P2 | msez-corridor | 5 | ReceiptChain prev_root must match MMR root (not previous next_root) — undocumented invariant causes silent append failures | DEFERRED |
| BUG-028 | P2 | msez-corridor | 5 | CorridorBridge::reachable_from() always includes source node at distance 0 even in empty graph — not documented, misleading for callers checking reachability | DEFERRED |
| BUG-029 | P2 | msez-agentic | 4 | ActionScheduler::mark_failed() silently retries (returns to Pending) when retries remain — callers expecting terminal failure must check retries_remaining or use with_max_retries(0) | DEFERRED |
| BUG-030 | P1 | msez-api | 7 | Treasury proxy routes (/v1/treasury/*) return 404 instead of 501 — routes appear unregistered | DEFERRED |
| BUG-031 | P1 | msez-api | 7 | Consent proxy routes (/v1/consent/*) return 405 instead of 501 — routes registered but wrong HTTP method | DEFERRED |
| BUG-032 | P1 | msez-api | 7 | Identity proxy routes (/v1/identity/*) return 404/405 — routes appear unregistered or wrong method | DEFERRED |
| BUG-033 | P1 | msez-pack | 1 | LawpackRef serde Deserialize bypasses validation — empty strings for jurisdiction_id, domain, version accepted; LawpackRef::parse would reject them | DEFERRED |
| BUG-034 | P2 | msez-arbitration | 1 | Claim missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-035 | P2 | msez-arbitration | 1 | Dispute missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-036 | P2 | msez-arbitration | 4 | EnforcementOrder allows cancel() from Blocked state — blocked orders pending appeal can be cancelled, bypassing the appeal process | DEFERRED |
| BUG-037 | P2 | msez-state | 4 | MigrationSaga allows compensate() from pre-InTransit states (e.g. ComplianceCheck) — compensation should only be available after InTransit when rollback is needed | DEFERRED |
| BUG-038 | P2 | msez-api | 7 | API returns 400 for JSON deserialization errors vs 422 for validation failures — inconsistent error taxonomy makes client error handling unreliable | DEFERRED |
| BUG-039 | P1 | msez-api | 7 | Mass fiscal/identity proxy POST routes validate request body (422) before checking Mass client availability — should return 501 first when no client configured | DEFERRED |
| BUG-040 | P2 | msez-api | 7 | Trigger endpoint rejects empty data payload with 422 — valid trigger types (e.g. SanctionsListUpdate) should accept empty data for event-only triggers | DEFERRED |
| BUG-041 | P1 | msez-corridor | 3 | SettlementPlan.reduction_percentage (f64) incompatible with CanonicalBytes — cannot canonicalize settlement plans for receipt chain digests; breaks end-to-end netting→settlement→receipt flow | DEFERRED |
| BUG-042 | P2 | msez-pack | 5 | LicenseCondition.condition_id is plain String — empty strings accepted without validation, causes empty keys in BTreeMap lookups | DEFERRED |
| BUG-043 | P1 | msez-pack | 5 | LicenseCondition::is_active() and License::is_expired() use string comparison for dates — malformed dates (e.g. "2025-9-01" missing leading zero) compare incorrectly, giving wrong expiry results | DEFERRED |
| BUG-044 | P2 | msez-pack | 5 | LicenseRestriction::blocks_jurisdiction("") accepts empty jurisdiction string — should reject, not silently treat as blocked | DEFERRED |
| BUG-045 | P2 | msez-pack | 5 | Licensepack::get_licenses_by_holder_did("") matches licenses with holder_did: Some("") — empty DID should be rejected, not used as filter | DEFERRED |
| BUG-046 | P2 | msez-state | 5 | Watcher::rebond(0) succeeds — transitions from Slashed to Bonded without posting new collateral. bond(0) correctly rejects but rebond(0) does not | DEFERRED |
| BUG-047 | P2 | msez-state | 5 | Watcher::rebond() inconsistent with bond() — bond() validates stake > 0 but rebond() does not, creating a validation gap in the recovery path | DEFERRED |
| BUG-048 | P1 | msez-zkp | 5 | MockProofSystem::prove() hashes only public_inputs, not circuit_data — two different circuits with same public_inputs produce identical proofs. Doc says SHA256(canonical(circuit) || public_inputs) but implementation does SHA256(public_inputs) only | DEFERRED |
| BUG-049 | P2 | msez-pack | 5 | License struct accepts empty strings for all required fields (license_id, holder_id, regulator_id) via serde — no validation on deserialization | DEFERRED |
| BUG-050 | P2 | msez-pack | 5 | resolve_licensepack_refs() uses unwrap_or("") for missing jurisdiction_id and domain — silently creates refs with empty identifiers instead of returning error | DEFERRED |

## Resolution Summary

**Session 4 (2026-02-15): 20 of 22 bugs resolved** (BUG-001 through BUG-022):
- **2 P0 bugs** (BUG-018, BUG-019): Production panic and silent data corruption eliminated
- **5 P1 bugs** (BUG-013 through BUG-017): Serde validation bypass closed for all identity newtypes
- **13 P2 bugs** resolved: PartialEq derives added, NettingEngine validation hardened

**Session 5 (2026-02-15): 18 new bugs discovered** (BUG-033 through BUG-050):
- **4 P1 bugs**: LawpackRef serde bypass, date string comparison, SettlementPlan f64 canonicalization, ZKP circuit-data omission
- **14 P2 bugs**: Missing validation in licensepack, watcher, agentic, and API layers

**Current tally: 50 bugs catalogued, 20 resolved, 30 deferred**
