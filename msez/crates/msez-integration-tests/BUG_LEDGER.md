# Bug Ledger — Test Hardening Session 1

| BUG | Severity | Crate | Campaign | Description | Status |
|-----|----------|-------|----------|-------------|--------|
| BUG-001 | P2 | msez-state | 1 | TransitionRecord missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-002 | P2 | msez-state | 1 | DynCorridorData missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-003 | P2 | msez-corridor | 1 | Obligation missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-004 | P2 | msez-corridor | 1 | NetPosition missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-005 | P2 | msez-corridor | 1 | SettlementLeg missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-006 | P2 | msez-corridor | 1 | SettlementPlan missing PartialEq derive; reduction_percentage uses f64 (float precision risk) | DEFERRED |
| BUG-007 | P2 | msez-corridor | 1 | SettlementInstruction missing PartialEq derive | DEFERRED |
| BUG-008 | P2 | msez-vc | 1 | Proof missing PartialEq derive — cannot verify serde round-trip fidelity | DEFERRED |
| BUG-009 | P2 | msez-vc | 1 | VerifiableCredential missing PartialEq derive | DEFERRED |
| BUG-010 | P2 | msez-agentic | 1 | AuditEntry missing PartialEq derive | DEFERRED |
| BUG-011 | P2 | msez-tensor | 1 | TensorCell missing PartialEq derive | DEFERRED |
| BUG-012 | P2 | msez-tensor | 1 | TensorCell.determined_at is raw String instead of Timestamp newtype — no validation | DEFERRED |
| BUG-013 | P1 | msez-core | 1 | Did serde Deserialize bypasses format validation — invalid DIDs accepted | DEFERRED |
| BUG-014 | P1 | msez-core | 1 | Ntn serde Deserialize bypasses 7-digit validation | DEFERRED |
| BUG-015 | P1 | msez-core | 1 | Cnic serde Deserialize bypasses 13-digit validation | DEFERRED |
| BUG-016 | P1 | msez-core | 1 | PassportNumber serde Deserialize bypasses length validation | DEFERRED |
| BUG-017 | P1 | msez-core | 1 | JurisdictionId serde Deserialize bypasses empty-string validation | DEFERRED |
| BUG-018 | P0 | msez-corridor | 1 | NettingEngine gross_total computation uses i64 sum — overflow on large amounts | DEFERRED |
| BUG-019 | P0 | msez-core | 2 | Did::method() panics on serde-deserialized invalid DID (expect("validated at construction") bypassed) | DEFERRED |
| BUG-020 | P2 | msez-corridor | 2 | NettingEngine accepts self-obligations (from_party == to_party) without validation | DEFERRED |
| BUG-021 | P2 | msez-corridor | 2 | NettingEngine accepts empty party ID strings without validation | DEFERRED |
| BUG-022 | P2 | msez-corridor | 2 | NettingEngine accepts empty currency strings without validation | DEFERRED |
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
