# CLAUDE.md — Momentum EZ Stack

Rust workspace: jurisdictional infrastructure as code. Single binary (`mez-api`) serves all routes. CLI (`mez-cli`) for offline ops. **17 crates, ~164K lines, ~4,700 tests. Zero Python.**

## Build & verify

```bash
cargo check --workspace                  # zero warnings
cargo clippy --workspace -- -D warnings  # zero diagnostics
cargo test --workspace                   # all tests pass
```

Run all three after any code change.

## Architecture

**Mass** (Java, NOT in this repo) owns CRUD for five primitives (Entities, Ownership, Fiscal, Identity, Consent). **This repo** owns compliance intelligence and orchestration. `mez-mass-client` is the sole gateway to Mass APIs — no other crate may call Mass directly.

Every write: `Auth → Compliance Tensor (20 domains) → Sanctions hard-block → Mass API → VC issuance → Attestation storage`. Reads pass through.

Two modes: `SOVEREIGN_MASS=true` (zone IS Mass, Postgres-backed) or `false` (proxy to mass.inc).

### Crate DAG

```
mez-core (ZERO internal deps — types, CanonicalBytes, ComplianceDomain(20), digest)
├── mez-crypto       Ed25519/zeroize, MMR, CAS
├── mez-tensor       Compliance Tensor, Dijkstra manifold
├── mez-pack         lawpack/regpack/licensepack
├── mez-state        typestate FSMs: corridor, entity, migration, watcher
├── mez-schema       116 JSON Schemas (Draft 2020-12)
├── mez-agentic      policy engine, tax pipeline
├── mez-vc           W3C VCs (core + crypto)
├── mez-corridor     receipt chains, fork resolution, netting (core + crypto + state)
├── mez-arbitration  dispute lifecycle, escrow (core + crypto + vc)
├── mez-compliance   regpack → tensor bridge
├── mez-zkp          STUBS ONLY (core + crypto)
├── mez-mass-client  typed HTTP client (core ONLY)
├── mez-mass-stub    dev Mass server
├── mez-cli          offline zone/vc/corridor CLI
├── mez-api          Axum HTTP — sole composition root, depends on ALL crates
└── mez-integration-tests
```

## Invariants

Violating any is a blocking failure.

1. `mez-core` has zero internal crate deps. External: serde, serde_json, thiserror, chrono, uuid, sha2 only.
2. `mez-mass-client` depends only on `mez-core`. Never import tensors, corridors, packs, VCs.
3. No dependency cycles. `mez-api` is the sole composition root.
4. SHA-256 flows through `mez-core::digest`. Direct `sha2::Sha256` only in `digest.rs` and `mmr.rs`.
5. ComplianceDomain has exactly 20 variants in `mez-core/src/domain.rs`. Every match exhaustive.
6. Zero `unwrap()` in production code — only inside `#[cfg(test)]`.
7. Zero `unimplemented!()`/`todo!()` outside tests — stubs return `MezError::NotImplemented`.
8. `serde_json` must not enable `preserve_order` — digest corruption.
9. No default credentials in deploy paths — use `${VAR:?must be set}`.

## Deployment

```bash
# Local single zone
cd deploy/docker && docker compose up -d
# Local two sovereign zones with corridor
docker compose -f docker-compose.two-zone.yaml up -d
# Scripted deploy with key gen
./deploy/scripts/deploy-zone.sh sovereign-govos-pk org.momentum.mez.zone.pk-sifc pk
```

AWS: `deploy/aws/terraform/` (EKS + RDS + KMS + S3 + ALB). K8s: `deploy/k8s/`.

## Code quality

- Read existing code before proposing changes.
- No `anyhow` outside `mez-cli`. No `std::sync::RwLock` — use `parking_lot`.
- No Mass CRUD duplicated in this repo. No Python. No direct `reqwest` outside `mez-mass-client`.
- New types use `mez-core` newtypes, not raw String/Uuid. Error types carry diagnostic context.
- Kill: dead functions, duplicate types, match arms all returning same value, `#[allow(dead_code)]`, tautological tests, doc comments restating signatures.

## Naming

Momentum (never "Momentum Protocol"). Mass (never "Mass Protocol" casually). Domains: momentum.inc, mass.inc (never .xyz/.io/.com).

## Priority order

Security > Correctness > Mass/EZ boundary > Deployment > Code quality
