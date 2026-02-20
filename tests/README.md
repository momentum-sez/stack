# Conformance Tests

This folder contains **reference conformance fixtures** used by the Rust test suite.

## Running tests

```bash
# Build and test the full workspace (4,601 tests)
cd mez && cargo test --workspace

# Run a specific crate's tests
cargo test -p mez-corridor

# Run adversarial security vectors
cargo test -p mez-integration-tests -- adversarial

# Run golden vector conformance tests
cargo test -p mez-integration-tests -- golden_vector

# Run cross-language canonicalization vectors
cargo test -p mez-core -- cross_language
```

## Fixtures

| Directory | Contents |
|-----------|----------|
| `fixtures/` | JSON golden vectors: canonical bytes, CAS digests, corridor agreements, MMR roots, lockfile |

## Test layers

1. **Inline unit tests** — `#[cfg(test)]` modules co-located with source (145 files)
2. **Crate integration tests** — `crates/*/tests/*.rs` (per-crate integration)
3. **Cross-crate integration** — `mez-integration-tests` (90+ test files, adversarial vectors, E2E flows)
4. **Mass API contract tests** — `mez-mass-client/tests/` (7 files; staleness checks are `#[ignore]`d)
