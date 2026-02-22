# CLAUDE.md — Momentum EZ Stack

> Instructions for Claude Code sessions operating on this repository.
> **Repository:** `momentum-sez/stack` · **License:** BUSL-1.1

---

## I. WHAT THIS REPO IS

A Rust workspace that deploys **jurisdictional infrastructure as code** — compliance evaluation, cross-border corridors, cryptographic credentials, and capital flow orchestration. The product vision: "the AWS of Economic Zones." One command deploys a sovereign zone with compliance intelligence, corridor connectivity, and verifiable audit trails.

The workspace produces a single binary (`mez-api`) that serves all HTTP routes, plus a CLI (`mez-cli`) for offline zone management.

**17 crates. ~164K lines of Rust. ~4,700 tests. Zero Python.**

---

## II. BUILD & VERIFY

```bash
cargo check --workspace                  # zero warnings required
cargo clippy --workspace -- -D warnings  # zero diagnostics required
cargo test --workspace                   # all tests must pass
```

Run all three after any code change. No exceptions.

---

## III. ARCHITECTURE

### The Mass/EZ Boundary

**Mass** (Java/Spring Boot, NOT in this repo) owns CRUD for five primitives:

| Primitive | Mass Service |
|-----------|-------------|
| Entities | `organization-info.api.mass.inc` |
| Ownership | `investment-info` (Heroku) |
| Fiscal | `treasury-info.api.mass.inc` |
| Identity | Split across consent-info + org-info (no dedicated service yet) |
| Consent | `consent.api.mass.inc` |

**This repo** owns compliance intelligence and orchestration:
- Compliance tensor evaluation (20 domains, exhaustive match, fail-closed)
- Pack Trilogy (lawpacks, regpacks, licensepacks)
- Corridor state machines and receipt chains (MMR + hash-chain)
- Verifiable Credential issuance (Ed25519)
- Trade flow instruments
- Orchestration pipeline: compliance eval → Mass API call → VC → attestation

**The boundary rule:** CRUD → Mass. Compliance/orchestration → this repo. `mez-mass-client` is the sole authorized gateway to Mass APIs.

### Crate Dependency DAG

```
mez-core (ZERO internal deps — foundation types, CanonicalBytes, ComplianceDomain(20), digest)
├── mez-crypto       (Ed25519/zeroize, MMR, CAS, SHA-256)
├── mez-tensor       (Compliance Tensor V2, Dijkstra manifold)
├── mez-pack         (lawpack/regpack/licensepack processing)
├── mez-state        (typestate FSMs: corridor, entity, migration, watcher)
├── mez-schema       (116 JSON Schemas, Draft 2020-12)
├── mez-agentic      (policy engine, 20 triggers, tax pipeline)
├── mez-vc           (W3C Verifiable Credentials — core + crypto)
├── mez-corridor     (receipt chains, fork resolution, netting — core + crypto + state)
├── mez-arbitration  (dispute lifecycle, escrow — core + crypto + vc)
├── mez-compliance   (regpack → tensor bridge — core + tensor + pack)
├── mez-zkp          (sealed ProofSystem trait — core + crypto; STUBS ONLY)
├── mez-mass-client  (typed HTTP client for Mass — core ONLY)
├── mez-mass-stub    (dev Mass API server — core + mass-client)
├── mez-cli          (offline zone/vc/regpack/corridor CLI)
├── mez-api          (Axum HTTP server — depends on ALL crates, sole composition point)
└── mez-integration-tests (cross-crate test suite)
```

### Orchestration Pipeline

Every **write** endpoint follows:
```
Request → Auth (constant-time bearer) → Compliance Tensor (20 domains)
  → Sanctions hard-block (NonCompliant = reject)
  → Mass API call (proxy or sovereign Postgres)
  → VC issuance (Ed25519-signed attestation)
  → Attestation storage (Postgres)
  → Response (OrchestrationEnvelope)
```

**Read** endpoints are pass-through — no compliance eval.

### Two Deployment Modes

- `SOVEREIGN_MASS=true`: Zone IS the Mass server. mez-api serves all primitive routes directly, Postgres-backed.
- `SOVEREIGN_MASS=false`: Zone proxies to centralized Mass APIs via `mez-mass-client`.

---

## IV. INVARIANTS

Violating any is a blocking failure.

1. **`mez-core` has zero internal crate dependencies.** External: serde, serde_json, thiserror, chrono, uuid, sha2 only.
2. **`mez-mass-client` depends only on `mez-core`** (for newtypes). Never import tensors, corridors, packs, VCs.
3. **No dependency cycles.** `mez-api` is the sole composition root.
4. **SHA-256 flows through `mez-core::digest`.** Direct `sha2::Sha256` usage only in `mez-core/src/digest.rs` and `mez-crypto/src/mmr.rs`. Verify: `grep -rn "use sha2" crates/ --include="*.rs"`.
5. **ComplianceDomain has exactly 20 variants**, defined once in `mez-core/src/domain.rs`. Every `match` is exhaustive. Compile-time assertion enforces COUNT == 20.
6. **Zero `unwrap()` in production code.** All `.unwrap()` must be inside `#[cfg(test)]`.
7. **Zero `unimplemented!()` or `todo!()` outside tests.** Stubs return `MezError::NotImplemented`.
8. **`serde_json` must not enable `preserve_order`.** Digest corruption if violated.
9. **No default credentials in deploy paths.** All compose/deploy files use `${VAR:?must be set}`.
10. **Receipt chain continuity:** `receipt.prev_root == final_state_root`, `receipt.next_root == SHA256(JCS(payload))`, `mmr_root() == MMR(next_roots)`.
11. **Compliance tensor fail-closed:** All mandatory domains evaluated, no empty slices. `NotApplicable` requires signed artifact.
12. **ZK proofs fail-closed:** Release builds reject mock proof types.

---

## V. WHAT IS REAL vs. WHAT IS NOT

Be honest about status. Do not write code that assumes stubs or planned features work.

### Implemented and Tested

| Capability | Evidence |
|-----------|---------|
| Compliance tensor (20 domains, fail-closed) | `mez-tensor/src/evaluation.rs` — exhaustive match, Merkle commitments |
| Receipt chain (dual-commitment: hash-chain + MMR) | `mez-corridor/src/receipt.rs` — golden vectors, adversarial tests |
| Fork resolution (evidence-driven) | `mez-corridor/src/fork.rs` — signed attestations, timestamp bounds |
| Canonicalization (MCF = JCS + float reject + datetime normalize) | `mez-core/src/canonical.rs` |
| Ed25519, MMR, CAS | `mez-crypto/` |
| W3C Verifiable Credentials | `mez-vc/` |
| Typestate FSMs (corridor, entity, migration, watcher) | `mez-state/` — invalid transitions = compile errors |
| Pack Trilogy processing | `mez-pack/` — lawpacks (PK), regpacks, 70+ licensepacks |
| Agentic policy engine | `mez-agentic/` — 20 triggers, tax pipeline |
| Arbitration system | `mez-arbitration/` — dispute lifecycle, escrow |
| Inter-zone corridor protocol | `mez-corridor/src/network.rs` — handshake, receipt exchange |
| Trade flow instruments | `mez-corridor/src/trade.rs` — 4 archetypes, 10 transitions |
| JSON Schema validation (116 schemas) | `mez-schema/` — Draft 2020-12, cached validators |
| Write-path orchestration | `mez-api/src/orchestration.rs` — both proxy and sovereign modes |
| Sovereign Mass persistence | `mez-api/migrations/` — Postgres-backed 5-primitive CRUD |
| Zone manifest system | `mez-pack/src/zone_manifest.rs` — 6 profiles |
| HTTP API (50+ endpoints) | `mez-api/src/routes/` — corridors, compliance, watchers, trade, tax, etc. |
| Docker Compose (1-zone, 2-zone, 3-zone) | `deploy/docker/` |
| AWS Terraform (EKS + RDS + KMS) | `deploy/aws/terraform/` |
| K8s manifests | `deploy/k8s/` |

### Stubs (return NotImplemented)

| Capability | Notes |
|-----------|-------|
| ZK proof circuits (12 types) | Mock implementations. Fail-closed in release builds. |
| BBS+ selective disclosure | Feature-gated trait only. |
| Poseidon2 hash | Feature-gated, returns NotImplemented. |
| Payment rail adapters (Raast, SWIFT, Circle) | Trait defined, no HTTP implementation. |
| National system adapters (FBR, SECP, NADRA) | Trait + type definitions, no real HTTP calls. |

### Does Not Exist

| Capability | Notes |
|-----------|-------|
| Identity as dedicated Mass service | Split across consent-info + org-info. |
| Smart Asset VM (SAVM) | No code. |
| MASS L1 settlement layer | No code. |
| CI/CD pipeline for Docker image builds | No GitHub Actions / ECR push automation. |
| Web UI / operator dashboard | CLI and API only. |

---

## VI. DEPLOYMENT REALITY

### What works today

```bash
# Local: single zone with Postgres + Prometheus + Grafana
cd deploy/docker && docker compose up -d

# Local: two sovereign zones with corridor
cd deploy/docker && docker compose -f docker-compose.two-zone.yaml up -d

# Scripted zone deploy with key generation
./deploy/scripts/deploy-zone.sh sovereign-govos-pk org.momentum.mez.zone.pk-sifc pk
```

### What is needed for AWS production deployment

1. Build and push Docker image to ECR (Dockerfile exists and works)
2. Fill in `terraform.tfvars` with AWS specifics
3. `terraform apply` provisions EKS + RDS + KMS + S3 + ALB
4. `kubectl apply -f deploy/k8s/` deploys mez-api

### Key files

| Purpose | Path |
|---------|------|
| Workspace manifest | `mez/Cargo.toml` |
| API entry point | `mez/crates/mez-api/src/main.rs` |
| Orchestration pipeline | `mez/crates/mez-api/src/orchestration.rs` |
| App state | `mez/crates/mez-api/src/state.rs` |
| Routes | `mez/crates/mez-api/src/routes/*.rs` |
| DB migrations | `mez/crates/mez-api/migrations/` |
| Compliance tensor | `mez/crates/mez-tensor/src/evaluation.rs` |
| Receipt chain | `mez/crates/mez-corridor/src/receipt.rs` |
| Fork resolution | `mez/crates/mez-corridor/src/fork.rs` |
| Trade flows | `mez/crates/mez-corridor/src/trade.rs` |
| Canonical bytes | `mez/crates/mez-core/src/canonical.rs` |
| Dockerfile | `deploy/docker/Dockerfile` |
| Docker Compose | `deploy/docker/docker-compose.yaml` |
| Terraform (AWS) | `deploy/aws/terraform/main.tf`, `kubernetes.tf` |
| K8s manifests | `deploy/k8s/` |
| Deploy script | `deploy/scripts/deploy-zone.sh` |
| Zone definitions | `jurisdictions/` (210 zones) |
| Module descriptors | `modules/` (323 across 16 families) |
| JSON schemas | `schemas/` (116 files) |
| Normative spec | `spec/` (24 chapters) |
| Bug ledger | `mez/crates/mez-integration-tests/BUG_LEDGER.md` |

---

## VII. CODE QUALITY RULES

### Before writing code

- Read existing code first. Never propose changes to code you haven't read.
- Verify new code earns its existence (see anti-slop list below).
- Run `cargo check && cargo clippy -- -D warnings && cargo test` after changes.

### Anti-slop (kill on sight)

- Functions never called outside their module
- Types that duplicate another type with trivially different fields
- Match arms that all return the same value
- Tests that assert `true == true` or only test JSON round-trip with no logic
- Doc comments that restate the function signature
- `#[allow(dead_code)]` — if dead, delete it
- Mock impls that return `Ok(())` without exercising logic
- Compliance evals that return `Compliant` for all domains without checking
- Constants referenced only in tests
- Trait impls with exactly one implementor "for future genericity"

### Code review gates

- No new `unwrap()` outside `#[cfg(test)]`
- No `unimplemented!()`/`todo!()` in production paths
- No `anyhow` outside `mez-cli`
- No `std::sync::RwLock` — use `parking_lot`
- No SHA-256 bypassing `mez-core::digest`
- No Mass CRUD duplicated in EZ Stack
- No Python
- No direct `reqwest` to Mass outside `mez-mass-client`
- Error types carry diagnostic context
- New types use `mez-core` newtypes (not raw String/Uuid)

---

## VIII. NAMING

| Term | Correct | Never |
|------|---------|-------|
| Momentum | "Momentum is a $1B+ venture fund and studio." | "Momentum Protocol" |
| Mass | "Mass provides five programmable primitives." | — |
| Mass Protocol | Only when discussing L1 settlement layer | In casual usage |
| momentum.inc | Momentum's domain | momentum.xyz, .io, .com |
| mass.inc | Mass's domain | mass.xyz, .io |

---

## IX. PRIORITY ORDER

When conflicts arise:

1. **Security** — keys, secrets, auth, constant-time ops
2. **Correctness** — tensor eval, receipt chain, canonical digests
3. **Mass/EZ boundary** — no CRUD duplication, clean orchestration
4. **Deployment** — anything blocking `deploy-zone.sh` or Terraform
5. **Code quality** — dead code, slop, untested paths
