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
