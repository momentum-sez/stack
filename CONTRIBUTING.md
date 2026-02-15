# Contributing to the Momentum SEZ Stack

## Getting started

```bash
git clone https://github.com/momentum-sez/stack.git
cd stack/msez

# Build all 16 crates
cargo build --workspace

# Run the full test suite (2,580+ tests)
cargo test --workspace

# Lint — zero warnings required
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --all -- --check
```

## Before submitting changes

All of these must pass:

```bash
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

## Code standards

### Error handling

- No `unwrap()` in library crates. Use `thiserror` for error types, propagate via `Result`.
- No `expect()` in production paths. Replace `std::sync::RwLock` panics with `parking_lot::RwLock`.
- No `unimplemented!()` in production paths. Return proper errors.

### Cryptographic safety

- All digest computation goes through `CanonicalBytes::new()` (in `msez-core`).
- All signing requires `&CanonicalBytes`.
- Private key types must implement `Zeroize` + `ZeroizeOnDrop`.
- Private key types must **not** implement `Serialize`.
- Bearer token comparison must use `subtle::ConstantTimeEq`.

### Type system

- Use identifier newtypes (`EntityId`, `CorridorId`, etc.) -- not raw `String` or `Uuid`.
- Use exhaustive `match` on all enums. No wildcard `_` patterns that silently ignore new variants.
- Typestate machines: each state is a distinct ZST. Invalid transitions should not compile.

### Mass API boundary

- All Mass API calls go through `msez-mass-client`. Direct `reqwest` to Mass endpoints from other crates is forbidden.
- The SEZ Stack does not store entity records, cap tables, payment records, identity records, or consent records. Those belong in Mass.

### Schema changes

- Place new schemas in `schemas/` with the naming convention `<domain>.<type>.schema.json`.
- Security-critical schemas must have `additionalProperties: false`.
- Update the schema count in documentation if adding new schemas.

## Commit messages

Write clear, descriptive commit messages. Use conventional commit prefixes:

- `feat(crate):` New feature
- `fix(crate):` Bug fix
- `refactor(crate):` Code restructuring
- `test(crate):` Test additions
- `docs:` Documentation changes

## Pull request process

1. Ensure all tests pass (`cargo test --workspace`)
2. Ensure zero clippy warnings (`cargo clippy --workspace -- -D warnings`)
3. Ensure formatting is correct (`cargo fmt --all -- --check`)
4. Include tests for new functionality
5. Update relevant documentation
6. Reference related issues in the PR description

## Specification compliance

When implementing features from the protocol specification (`spec/`), include the relevant spec section reference in code comments. This ensures traceability between the implementation and the normative specification.

## Reporting issues

Include: version, steps to reproduce, expected vs. actual behavior, and error messages.

## License

By contributing, you agree that your contributions will be licensed under BUSL-1.1 (see `LICENSES/`).
