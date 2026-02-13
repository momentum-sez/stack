# Baseline Snapshot — msez Rust Workspace

**Date**: 2026-02-13
**Workspace version**: 0.4.44
**Commit**: Pre-audit baseline (observation only, no changes)

---

## Compilation

```
cargo check --workspace
```

**Result**: CLEAN. Zero errors, zero warnings.

All 13 crates compile successfully:
- msez-core, msez-crypto, msez-state, msez-pack, msez-agentic, msez-tensor,
  msez-zkp, msez-vc, msez-schema, msez-corridor, msez-arbitration, msez-api, msez-cli

---

## Tests

```
cargo test --workspace
```

**Result**: ALL PASS.

| Metric   | Count |
|----------|-------|
| Passed   | 2,651 |
| Failed   | 0     |
| Ignored  | 1     |
| Measured | 0     |

The 1 ignored test is a doctest in `msez-api/src/extractors.rs` (line 21).

Breakdown by crate (unit + integration tests, excluding doctests):

| Crate                    | Passed |
|--------------------------|--------|
| msez-core                | 109    |
| msez-crypto              | 281    |
| msez-state               | 182    |
| msez-api                 | 282    |
| msez-pack                | 158    |
| msez-agentic             | 21     |
| msez-tensor              | 146    |
| msez-zkp                 | 83     |
| msez-vc                  | 136    |
| msez-schema              | 4      |
| msez-corridor            | 131    |
| msez-arbitration         | 84     |
| msez-cli                 | 30     |
| msez-integration-tests   | 802    |
| Doctests (all crates)    | 3 passed, 1 ignored |

---

## Clippy

```
cargo clippy --workspace
```

**Result**: CLEAN. Zero warnings.

---

## Unimplemented Macros

**Total `unimplemented!()` calls**: 11 (excluding comments/doc strings)
**Total `todo!()` calls**: 0

All 11 are behind Cargo feature flags (not compiled by default):

### msez-core (1 instance)

| Line | Feature Gate | Function |
|------|-------------|----------|
| `src/digest.rs:128` | `#[cfg(feature = "poseidon2")]` | `poseidon2_digest()` |

### msez-crypto (5 instances)

| Line | Feature Gate | Function |
|------|-------------|----------|
| `src/poseidon.rs:63` | Module gated: `#[cfg(feature = "poseidon2")]` in lib.rs:28 | `poseidon2_digest()` |
| `src/poseidon.rs:76` | Module gated: `#[cfg(feature = "poseidon2")]` in lib.rs:28 | `poseidon2_node_hash()` |
| `src/bbs.rs:95` | Module gated: `#[cfg(feature = "bbs-plus")]` in lib.rs:31 | `bbs_sign()` |
| `src/bbs.rs:114` | Module gated: `#[cfg(feature = "bbs-plus")]` in lib.rs:31 | `bbs_derive_proof()` |
| `src/bbs.rs:132` | Module gated: `#[cfg(feature = "bbs-plus")]` in lib.rs:31 | `bbs_verify_proof()` |

### msez-zkp (5 instances)

| Line | Feature Gate | Function |
|------|-------------|----------|
| `src/cdb.rs:87` | `#[cfg(feature = "poseidon2")]` (line 78) | `Cdb::digest()` poseidon2 branch |
| `src/groth16.rs:89` | Module gated: `#[cfg(feature = "groth16")]` in lib.rs:56 | `Groth16ProofSystem::prove()` |
| `src/groth16.rs:101` | Module gated: `#[cfg(feature = "groth16")]` in lib.rs:56 | `Groth16ProofSystem::verify()` |
| `src/plonk.rs:90` | Module gated: `#[cfg(feature = "plonk")]` in lib.rs:59 | `PlonkProofSystem::prove()` |
| `src/plonk.rs:102` | Module gated: `#[cfg(feature = "plonk")]` in lib.rs:59 | `PlonkProofSystem::verify()` |

**Assessment**: All 11 `unimplemented!()` calls are behind feature flags that are **not** in any crate's `default` features. They cannot be reached in a default build. However, enabling any of these features (e.g., `cargo build --features poseidon2`) would introduce panicking code paths in library crates. Per CLAUDE.md P0-004, these should be converted to `Err(...)` returns regardless of feature gating.

---

## Unwrap/Expect in Production

**Scope**: `msez-api/src/` — production code paths only (excluding `#[cfg(test)]` modules and test functions).

### Production `expect()` calls: 8

**state.rs** — 7 calls (P0-003):

| Line | Method | Call |
|------|--------|------|
| 52 | `Store::insert()` | `.expect("store lock poisoned")` |
| 60 | `Store::get()` | `.expect("store lock poisoned")` |
| 69 | `Store::list()` | `.expect("store lock poisoned")` |
| 77 | `Store::update()` | `.expect("store lock poisoned")` |
| 89 | `Store::remove()` | `.expect("store lock poisoned")` |
| 97 | `Store::contains()` | `.expect("store lock poisoned")` |
| 104 | `Store::len()` | `.expect("store lock poisoned")` |

**middleware/rate_limit.rs** — 1 call:

| Line | Method | Call |
|------|--------|------|
| 61 | `RateLimiter::check()` | `.expect("rate limit lock poisoned")` |

### Production `unwrap()` calls: 0

All other `unwrap()` calls in msez-api/src/ are inside `#[cfg(test)]` modules (test-only). Verified files: auth.rs, error.rs, extractors.rs, openapi.rs, metrics.rs, and all route handlers (entities.rs, ownership.rs, fiscal.rs, identity.rs, consent.rs, corridors.rs, smart_assets.rs, regulator.rs).

### Additional production concern (not expect/unwrap):

**auth.rs:43** — Token comparison uses `provided == expected.as_str()` (PartialEq on `&str`), which is **not** constant-time. This is P0-002, a timing side-channel vulnerability, but it is not a panicking call.

---

## Feature Flags

### serde_json `preserve_order`

```
cargo tree -e features -i serde_json 2>&1 | grep preserve_order
```

**Result**: No match. The `preserve_order` feature is **NOT** enabled anywhere in the dependency tree.

This means `serde_json::Map` uses `BTreeMap` (lexicographic key ordering), which is the correct behavior for `CanonicalBytes` canonicalization. The canonicalization invariant is safe from this particular dependency-induced corruption vector.

### Cargo feature flags summary

| Crate | Feature | Default? | Status |
|-------|---------|----------|--------|
| msez-core | `poseidon2` | No | Stub with `unimplemented!()` |
| msez-crypto | `poseidon2` | No | Stub with `unimplemented!()` |
| msez-crypto | `bbs-plus` | No | Stub with `unimplemented!()` |
| msez-zkp | `mock` | **Yes** | Functional mock proof system |
| msez-zkp | `groth16` | No | Stub with `unimplemented!()` |
| msez-zkp | `plonk` | No | Stub with `unimplemented!()` |
| msez-zkp | `poseidon2` | No | Stub with `unimplemented!()` |

---

## Key Zeroization

```
grep -rn 'Zeroize\|zeroize' crates --include="*.rs"
```

**Result**: Zero matches. No references to `Zeroize`, `zeroize`, or `ZeroizeOnDrop` anywhere in the Rust workspace.

**Assessment**: P0-001 is confirmed. The `ed25519-dalek` `SigningKey` wrapper in `msez-crypto/src/ed25519.rs` does not implement `Zeroize` or `ZeroizeOnDrop`. When a signing key is dropped, the secret key material remains in memory until overwritten by subsequent allocations. The `zeroize` crate is **not** in the dependency tree (though `ed25519-dalek` supports a `zeroize` feature flag). The crate `zeroize v1.8.2` was downloaded during compilation (it's a transitive dependency of `ed25519-dalek`), but no code in `msez-crypto` explicitly uses it.

---

## Summary

| Category | Status |
|----------|--------|
| Compilation | Clean (0 errors, 0 warnings) |
| Tests | 2,651 passed, 0 failed, 1 ignored |
| Clippy | Clean (0 warnings) |
| `unimplemented!()` in default build | 0 reachable (11 behind feature flags) |
| `todo!()` macros | 0 |
| Production `expect()` in msez-api | 8 (7 in state.rs + 1 in rate_limit.rs) |
| Production `unwrap()` in msez-api | 0 |
| `serde_json` preserve_order | Not enabled (safe) |
| Zeroize on signing keys | Not implemented (P0-001) |
| Constant-time token comparison | Not implemented (P0-002) |
