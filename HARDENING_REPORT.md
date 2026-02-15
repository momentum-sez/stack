# Hardening Report — 2026-02-15

## Summary
- Total defects catalogued: 12
- P0 (security/correctness): 4 (all previously resolved)
- P1 (reliability/safety): 7 (4 fixed this session, 3 previously resolved)
- P2 (quality/hygiene): 1 (remaining)

## Previously Resolved Defects (confirmed during discovery)

These defects from CLAUDE.md were found already fixed in prior sessions:

### P0-001: msez-crypto::ed25519 — SigningKey missing Zeroize
- **File:** crates/msez-crypto/src/ed25519.rs
- **Status:** RESOLVED — `Zeroize` impl and `Drop` with `zeroize()` call present
- **Evidence:** Lines 129-137 (Zeroize impl), lines 191-195 (Drop impl)

### P0-002: msez-api::auth — Non-constant-time bearer token comparison
- **File:** crates/msez-api/src/auth.rs
- **Status:** RESOLVED — Uses `constant_time_token_eq` with `subtle::ConstantTimeEq`
- **Evidence:** Lines 164-173

### P0-003: msez-api::state — Lock poisoning panics (7x expect())
- **File:** crates/msez-api/src/state.rs
- **Status:** RESOLVED — Uses `parking_lot::RwLock` and `parking_lot::Mutex`
- **Evidence:** parking_lot used throughout, no std::sync::RwLock/Mutex

### P0-004: 14 unimplemented!() macros in production paths
- **Status:** RESOLVED — No `unimplemented!()` or `todo!()` in production code paths.
  Feature-gated stubs (bbs-plus, poseidon) remain behind cargo feature flags as designed.

### P1-005: Readiness probe is a no-op
- **File:** crates/msez-api/src/lib.rs:117-136
- **Status:** RESOLVED — Verifies zone signing key, policy engine lock, store accessibility

### P1-007: serde_json preserve_order guard missing
- **File:** crates/msez-core/build.rs, crates/msez-core/src/canonical.rs
- **Status:** RESOLVED — Three-layer defense in place:
  1. Runtime test `serde_json_map_must_use_sorted_order()`
  2. CI hook documented: `cargo tree -e features -i serde_json | grep -q preserve_order && exit 1`
  3. build.rs documentation and cfg emission

## Defects Fixed This Session

### P1-001: msez-api — Rate limiter applied after authentication
- **File:** crates/msez-api/src/lib.rs:86-87
- **Category:** Middleware ordering
- **Impact:** Unauthenticated request floods bypass rate limiting, enabling auth middleware DoS
- **Fix:** Swapped `.layer()` order so rate limiting runs before authentication
- **Commit:** 7affc8f
- **Blast radius:** 1 file

### P1-002: msez-api::state — SmartAssetRecord.status is untyped String
- **File:** crates/msez-api/src/state.rs:186
- **Category:** Soundness — invalid state representable
- **Impact:** Any arbitrary string stored as asset status; no compile-time validation
- **Fix:** Introduced `AssetStatus` enum (Genesis, Registered, Active, Pending, Suspended, Retired)
  with `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]` for API contract compatibility
- **Commit:** 2a7ed90
- **Blast radius:** 5 files (state.rs, smart_assets.rs, compliance.rs, auth.rs, regulator.rs)

### P1-003: msez-api::routes — Phase 2 stub endpoints return 200
- **Files:** crates/msez-api/src/routes/corridors.rs, smart_assets.rs
- **Category:** API contract violation
- **Impact:** Four stub endpoints returned 200 with fake success data, misleading clients:
  - `POST /v1/corridors/state/anchor` — returned `{"status":"anchored"}`
  - `POST /v1/corridors/state/finality-status` — returned `{"status":"pending","confirmations":0}`
  - `POST /v1/assets/registry` — returned `{"status":"submitted"}`
  - `POST /v1/assets/:id/anchors/corridor/verify` — returned `{"verified":true}`
- **Fix:** All four now return 501 Not Implemented via `AppError::NotImplemented`
- **Commit:** f0755c2
- **Blast radius:** 2 files

### P1-004: msez-api::state — AttestationRecord.status is untyped String
- **File:** crates/msez-api/src/state.rs:205
- **Category:** Soundness — invalid state representable
- **Impact:** Any arbitrary string stored as attestation status
- **Fix:** Introduced `AttestationStatus` enum (Active, Pending, Revoked, Expired)
  with `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`. Also updated
  `QueryAttestationsRequest.status` filter from `Option<String>` to `Option<AttestationStatus>`,
  which prevents querying with invalid status values.
- **Commit:** 2a7ed90 (combined with P1-002)
- **Blast radius:** 2 files (state.rs, regulator.rs)

### P1-006: msez-api::routes::agentic — ActionResult.status is untyped String
- **File:** crates/msez-api/src/routes/agentic.rs:88
- **Category:** Soundness — invalid state representable
- **Impact:** Action status compared with string literals in tests (typo-prone, not caught at compile time)
- **Fix:** Introduced `ActionStatus` enum (Executed, Scheduled, Skipped, Failed)
  with `#[serde(rename_all = "snake_case")]` for wire format compatibility
- **Commit:** cb7afa3
- **Blast radius:** 1 file

## Session Results
- Fixes applied: 5 (P0: 0, P1: 5, P2: 0)
- New enums added: 3 (AssetStatus, AttestationStatus, ActionStatus)
- Defects remaining: 1 (P2)
- Workspace status: `cargo test --workspace` PASS, `cargo clippy --workspace -- -D warnings` PASS

### Fixes Applied
| ID | Severity | Crate | Description | Commit |
|----|----------|-------|-------------|--------|
| P1-001 | P1 | msez-api | Swap middleware order — rate limit before auth | 7affc8f |
| P1-002 | P1 | msez-api | SmartAssetRecord.status String → AssetStatus enum | 2a7ed90 |
| P1-003 | P1 | msez-api | Phase 2 stub endpoints return 501 instead of 200 | f0755c2 |
| P1-004 | P1 | msez-api | AttestationRecord.status String → AttestationStatus enum | 2a7ed90 |
| P1-006 | P1 | msez-api | ActionResult.status String → ActionStatus enum | cb7afa3 |

### Remaining (for next session)
| ID | Severity | Crate | Description |
|----|----------|-------|-------------|
| P2-001 | P2 | msez-api | `asset_type` field in SmartAssetRecord and CreateAssetRequest remains String (low blast radius but would benefit from enum if asset types stabilize) |
