# Hardening Report — 2026-02-15

## Summary
- Total defects cataloged across all sessions: 22
- P0 (security/correctness): 7 total — **all resolved**
- P1 (reliability/safety): 11 total — **all resolved**
- P2 (quality/hygiene): 9 total — **7 resolved, 2 remaining**
- Schema violations: **14 → 0** (audit §3.1 fully remediated)
- All P0 and P1 defects resolved. Workspace clean: zero test failures, zero clippy warnings.

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

### P1-001: msez-api — Rate limiter applied after authentication
- **File:** crates/msez-api/src/lib.rs:86-87
- **Status:** RESOLVED — Swapped `.layer()` order so rate limiting runs before authentication
- **Commit:** 7affc8f

### P1-002: msez-api::state — SmartAssetRecord.status is untyped String
- **File:** crates/msez-api/src/state.rs:186
- **Status:** RESOLVED — AssetStatus enum introduced
- **Commit:** 2a7ed90

### P1-003: msez-api::routes — Phase 2 stub endpoints return 200
- **Status:** RESOLVED — All four now return 501 Not Implemented
- **Commit:** f0755c2

### P1-004: msez-api::state — AttestationRecord.status is untyped String
- **Status:** RESOLVED — AttestationStatus enum introduced
- **Commit:** 2a7ed90

### P1-005: Readiness probe is a no-op
- **File:** crates/msez-api/src/lib.rs:117-136
- **Status:** RESOLVED — Verifies zone signing key, policy engine lock, store accessibility

### P1-006: msez-api::routes::agentic — ActionResult.status is untyped String
- **Status:** RESOLVED — ActionStatus enum introduced
- **Commit:** cb7afa3

### P1-007: serde_json preserve_order guard missing
- **File:** crates/msez-core/build.rs, crates/msez-core/src/canonical.rs
- **Status:** RESOLVED — Three-layer defense in place

## Defects Fixed This Session

### P0-CRYPTO-001: Timing side-channel in MMR inclusion proof verification
- **File:** crates/msez-crypto/src/mmr.rs:512
- **Category:** Cryptographic safety
- **Impact:** Non-constant-time hash comparison (`==`) on hex-encoded SHA-256 root digests leaks proof validation timing. An attacker controlling proof inputs can discover the expected MMR root digest via timing oracle.
- **Fix:** Added `subtle` crate to msez-crypto. Replaced `computed_root == proof.root` with `from_hex()` + `ct_eq()` constant-time byte comparison.
- **Commit:** 070c969
- **Blast radius:** 1 file (mmr.rs)

### P0-CRYPTO-002: Timing side-channel in CAS artifact integrity check
- **File:** crates/msez-crypto/src/cas.rs:272
- **Category:** Cryptographic safety
- **Impact:** Non-constant-time digest comparison (`!=`) in `ContentAddressedStore::resolve()`. Attacker with filesystem access could discover correct artifact digest via timing differences.
- **Fix:** Replaced `recomputed.to_hex() != hex` with `recomputed.as_bytes().ct_eq(digest.as_bytes())` — constant-time comparison on raw 32-byte digests.
- **Commit:** e6990f0
- **Blast radius:** 1 file (cas.rs)

### P1-CRYPTO-003: Weak test assertion in mmr_leaf_hash_accepts_uppercase_hex
- **File:** crates/msez-crypto/src/mmr.rs:1297
- **Category:** Test quality — weak assertion
- **Impact:** Test asserted `is_ok()` without checking hash value. Corrupted implementation returning garbage `Ok()` would pass.
- **Fix:** Now asserts exact expected hash value.
- **Commit:** 6699966
- **Blast radius:** Test code only

### P1-CRYPTO-004: Test with no assertions — verifying_key_from_bytes
- **File:** crates/msez-crypto/src/ed25519.rs:527
- **Category:** Test quality — test cannot fail
- **Impact:** Test executed code but made no assertion (`let _ = result`). Cannot detect regressions.
- **Fix:** Added assertions documenting ed25519-dalek's behavior: all-zeros key accepted at construction but verification against it correctly fails.
- **Commit:** 6699966
- **Blast radius:** Test code only

### P1-CLI-001: Dead code / migration residue in artifact.rs
- **File:** crates/msez-cli/src/artifact.rs:115-117
- **Category:** Python migration residue
- **Impact:** Created synthetic `CanonicalBytes` value solely for import, then discarded. Dead code confuses readers.
- **Fix:** Removed dead lines and unused import.
- **Commit:** 889574f
- **Blast radius:** 1 file

### P1-MASS-001: Untyped entity_type/status fields in mass-client
- **File:** entities.rs, consent.rs, identity.rs, fiscal.rs in msez-mass-client
- **Category:** Soundness — invalid state representable
- **Impact:** String-typed fields accepted arbitrary values. No compile-time enforcement or exhaustive pattern matching.
- **Fix:** Added 8 typed enums with `#[serde(other)]` Unknown variant for forward compatibility. Updated proxy layer to parse strings into enums at the API boundary.
- **Commit:** 39837a4
- **Blast radius:** 6 files

### P2-STATE-001: Misleading f64::EPSILON comparison in watcher test
- **File:** crates/msez-state/src/watcher.rs:511-518
- **Category:** Test quality
- **Impact:** Used `abs() < f64::EPSILON` — accidentally correct for exact literal returns but misleading.
- **Fix:** Replaced with direct `assert_eq!` comparisons.
- **Commit:** af1abef
- **Blast radius:** Test code only

### P2-SCHEMA-002: Always-passing schema audit test
- **File:** crates/msez-schema/src/codegen.rs:302-332
- **Category:** Test quality — test cannot fail
- **Impact:** Used `eprintln!` for schema violations instead of `assert!`. Always passed.
- **Fix:** Added regression guard with known violation count (14). Fails if new violations appear.
- **Commit:** 530074c
- **Blast radius:** Test code only

## Remaining Defects

| ID | Severity | Crate | Description |
|----|----------|-------|-------------|
| P2-MASS-003 | P2 | msez-mass-client | Financial amounts (balance, par_value) are untyped Strings — need Decimal newtype |
| P2-VC-001 | P2 | msez-vc | Missing cross-language VC parity tests |

---

## Session 3 Results (current)
- Fixes applied: 11 (P0: 1, P1: 4, P2: 6) + 14 schema violations
- Files changed: 32 (+626/-222 lines)
- Defects remaining: 2 (P2 only)
- Schema violations: 14 → **0**
- Workspace status: `cargo test --workspace` **PASS** (1700+ tests), `cargo clippy --workspace` **PASS** (zero warnings)

### Session 3 Fixes
| ID | Severity | Crate | Description | Commit |
|----|----------|-------|-------------|--------|
| P0-ESCROW | P0 | msez-arbitration | parse_amount() unwrap_or(0) → Result with InvalidAmount error | 4c30ce9 |
| P1-ZKP-002 | P1 | msez-zkp | MockProofSystem prove/verify asymmetry — aligned to hash only public_inputs | 4c30ce9 |
| P1-MASS-002 | P1 | msez-mass-client | Added retry.rs with exponential backoff (200ms→400ms→800ms) across 6 clients | 4c30ce9 |
| P1-SCHEMA-001 | P1 | msez-schema | walk_for_modules now returns Result instead of swallowing IO errors | 4c30ce9 |
| P2-CORRIDOR-001 | P2 | msez-corridor | Duplicate obligation detection via BTreeSet in NettingEngine | 4c30ce9 |
| P2-CLI-002 | P2 | msez-cli | Silent file-not-found → tracing::warn! in lock.rs profile resolution | 4c30ce9 |
| P2-CLI-003 | P2 | msez-cli | O(n²) Vec::contains → HashSet dedup in validate.rs | 4c30ce9 |
| P2-CRYPTO-005 | P2 | msez-crypto | ArtifactRef.artifact_type String → ArtifactType validated newtype | 4c30ce9 |
| P2-API-001 | P2 | msez-api | SmartAssetRecord.asset_type String → SmartAssetType validated newtype | 4c30ce9 |
| P1-ZKP-TEST | P1 | msez-integration-tests | Updated 2 integration tests for ZKP prove/verify symmetry fix | 4c30ce9 |
| SCHEMA-§3.1 | P1 | schemas/ | Fixed all 14 additionalProperties violations across 7 schema files | 4c30ce9 |

### Session 3 Schema Remediation (Audit §3.1)
All 14 `additionalProperties: true` violations fixed to `false`:
- **vc.smart-asset-registry**: quorum_policy, quorum_policy/default, ComplianceProfile, EnforcementProfile
- **corridor.receipt**: zk, anchor
- **corridor.checkpoint**: mmr, anchor (added missing anchor properties)
- **corridor.fork-resolution**: candidates/items
- **vc.corridor-fork-resolution**: candidates/items
- **vc.dispute-claim**: forum (added structured properties)
- **vc.arbitration-award**: forum, outcome, obligations/items (added structured properties)
- **codegen.rs**: KNOWN_VIOLATION_COUNT 14 → 0, test now asserts zero violations

## Session 2 Results
- Fixes applied: 8 (P0: 2, P1: 4, P2: 2)
- New enums added: 8 (MassEntityType, MassEntityStatus, MassConsentType, MassConsentStatus, MassIdentityType, MassIdentityStatus, MassAccountType, MassPaymentStatus)

### Session 2 Fixes
| ID | Severity | Crate | Description | Commit |
|----|----------|-------|-------------|--------|
| P0-CRYPTO-001 | P0 | msez-crypto | Timing side-channel in MMR proof verification | 070c969 |
| P0-CRYPTO-002 | P0 | msez-crypto | Timing side-channel in CAS integrity check | e6990f0 |
| P1-CRYPTO-003 | P1 | msez-crypto | Weak MMR test assertion — now checks hash value | 6699966 |
| P1-CRYPTO-004 | P1 | msez-crypto | No-assertion test — now verifies Ed25519 behavior | 6699966 |
| P1-CLI-001 | P1 | msez-cli | Dead code removal in artifact.rs | 889574f |
| P1-MASS-001 | P1 | msez-mass-client | 8 typed enums replace String fields | 39837a4 |
| P2-STATE-001 | P2 | msez-state | f64::EPSILON → assert_eq! in watcher test | af1abef |
| P2-SCHEMA-002 | P2 | msez-schema | Schema audit test now prevents regressions | 530074c |
