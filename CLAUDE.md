# CLAUDE.md — Momentum EZ Stack v0.4.44 GENESIS

**Repository:** `momentum-ez/stack`
**Version:** 0.4.44-GENESIS
**License:** BUSL-1.1
**Architecture:** Rust workspace (single `mez-api` binary) + 210 zone definitions + 323 modules + 116 schemas

---

## I. WHAT THIS IS

A Rust workspace that provides **jurisdictional orchestration** for Mass — Momentum's five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent). Mass is Java/Spring Boot, live, processing real capital. This repo sits above Mass and adds compliance intelligence, corridor management, and cryptographic provenance.

17 crates, 159K lines of Rust, 4,683 tests, zero Python.

## II. THE BOUNDARY

**Mass APIs** (Java, NOT in this repo) own CRUD for five primitives:

| Primitive | Service | Base URL |
|-----------|---------|----------|
| Entities | organization-info | `organization-info.api.mass.inc/organization-info/` |
| Ownership | investment-info | `investment-info-production-*.herokuapp.com/investment-info/` |
| Fiscal | treasury-info | `treasury-info.api.mass.inc/treasury-info/` |
| Identity | **No dedicated service yet** — split across consent-info + org-info | — |
| Consent | consent-info | `consent.api.mass.inc/consent-info/` |

API path convention: `{base_url}/{context-path}/api/v1/{resource}`

**This repo** (Rust) owns jurisdictional intelligence:

- Compliance tensor evaluation (20 domains, exhaustive match)
- Pack Trilogy (lawpacks, regpacks, licensepacks)
- Corridor state machines and receipt chains
- Trade flow instruments (4 archetypes, 10 transition types)
- Verifiable Credential issuance
- Orchestration pipeline: compliance eval -> Mass API call -> VC -> attestation
- Smart Asset lifecycle, watcher economy, arbitration
- Zone composition algebra and corridor mesh
- Deployment tooling (Docker, Terraform, K8s)

**The rule**: If it's "create/read/update/delete a business object" -> Mass. If it's "evaluate whether that operation is compliant in this jurisdiction" -> this repo. `mez-mass-client` is the sole authorized gateway to Mass APIs. No other crate may call Mass directly.

## III. CRATE MAP

```
mez-core             Foundation. CanonicalBytes, ComplianceDomain (20 variants),
                      identifier newtypes, MezError, Timestamp, data sovereignty.
                      ZERO internal deps.

mez-crypto           Ed25519 (Zeroize), MMR, CAS, constant-time comparison.
mez-vc               W3C Verifiable Credentials, Ed25519Signature2020.
mez-tensor           Compliance Tensor V2, Manifold (Dijkstra path optimization).
mez-pack             Pack Trilogy processing, multi-jurisdiction composition engine.
mez-state            Typestate FSMs: corridor, migration, entity, license, watcher.
mez-corridor         Corridor lifecycle, receipt chains, fork resolution, netting,
                      SWIFT pacs.008, trade flow instruments, inter-zone protocol.
mez-agentic          Autonomous policy engine, trigger taxonomy, tax event generation.
mez-arbitration      Dispute lifecycle, escrow, institution registry.
mez-compliance       Jurisdiction config bridge (regpack -> tensor).
mez-schema           JSON Schema validation (116 schemas, Draft 2020-12).
mez-zkp              ZK proof traits + stubs (Groth16, PLONK — mock only).
mez-mass-client      Typed HTTP client for Mass APIs. Depends on mez-core only.
mez-mass-stub        Standalone Mass API stub server for dev/testing without Postgres.
mez-api              Axum HTTP server. Sole composition point — depends on all above.
mez-cli              CLI: zone validate/build/lock/sign, VC keygen/sign/verify,
                      corridor mesh, regpack build.
mez-integration-tests  Cross-crate integration test suite.
```

Dependency DAG:
```
mez-core (leaf — zero internal deps)
├── mez-crypto, mez-tensor, mez-pack, mez-state, mez-schema, mez-agentic
├── mez-vc (core + crypto)
├── mez-corridor (core + crypto + state)
├── mez-arbitration (core + crypto + vc)
├── mez-compliance (core + tensor + pack)
├── mez-zkp (core + crypto)
├── mez-mass-client (core only — newtypes)
├── mez-mass-stub (core + mass-client)
└── mez-api (ALL crates — sole composition point)
```

## IV. BUILD & VERIFY

```bash
cargo check --workspace              # zero warnings required
cargo clippy --workspace -- -D warnings  # zero diagnostics required
cargo test --workspace               # all 4,601 tests must pass
```

After any code change, run all three. No exceptions.

## V. INVARIANTS (violating any is a blocking failure)

1. **`mez-core` has zero internal crate dependencies.** External: serde, serde_json, thiserror, chrono, uuid, sha2 only.

2. **`mez-mass-client` depends only on `mez-core`** (for identifier newtypes). Never import tensors, corridors, packs, VCs, or other domain crates.

3. **No dependency cycles.** `mez-api` is the sole composition root. No other crate depends on it.

4. **All SHA-256 flows through `mez-core::digest`.** Three tiers:
   - Domain objects: `CanonicalBytes::new()` -> `sha256_digest()`
   - Raw bytes: `sha256_raw()`
   - Streaming: `Sha256Accumulator`
   - `sha2::Sha256` direct usage appears ONLY in `mez-core/src/digest.rs` and `mez-crypto/src/mmr.rs`.
   - Verify: `grep -rn "use sha2" crates/ --include="*.rs"` — hits outside those two files are bugs.

5. **ComplianceDomain has exactly 20 variants**, defined once in `mez-core/src/domain.rs`. Every `match` is exhaustive. Compile-time assertion enforces COUNT == 20.

6. **Zero `unwrap()` in production code.** All `.unwrap()` must be inside `#[cfg(test)]`. Use `?`, `.map_err()`, `.ok_or_else()`, or `expect("reason")` for static values.

7. **Zero `unimplemented!()` or `todo!()` outside tests.** Stubs return `MezError::NotImplemented` (HTTP 501).

8. **`serde_json` must not enable `preserve_order`.** CI guard exists. Digest corruption if violated.

9. **No default credentials in deploy paths.** All compose/deploy files use `${VAR:?must be set}`.

10. **Receipt chain invariants:**
    - `receipt.prev_root == final_state_root` (hash-chain continuity)
    - `receipt.next_root == SHA256(JCS(payload_without_proof_and_next_root))`
    - `mmr_root() == MMR(next_roots)`

11. **Compliance tensor fail-closed:** Production mode: all mandatory domains evaluated, no empty slices. `NotApplicable` requires signed policy artifact.

12. **ZK proof policy fail-closed:** Release builds reject mock proof types.

## VI. ORCHESTRATION PATTERN

Every **write** endpoint in sovereign/proxy mode follows:

```
1. Pre-flight compliance -> evaluate tensor across relevant domains for jurisdiction
2. Hard-block check -> Sanctions NonCompliant = reject (legal requirement)
3. Mass API call -> delegate via mez-mass-client (proxy) or sovereign_ops (sovereign)
4. VC issuance -> sign compliance attestation as Verifiable Credential
5. Attestation storage -> persist to Postgres for regulator queries
6. Return OrchestrationEnvelope { mass_response, compliance, credential, attestation_id }
```

**Read** endpoints (GET) are pass-through — no compliance eval needed on reads.

Two deployment modes:
- `SOVEREIGN_MASS=true`: Zone IS the Mass server. Postgres-backed. No external Mass dependency.
- `SOVEREIGN_MASS=false`: Zone proxies to centralized Mass APIs via `mez-mass-client`.

## VII. WHAT IS REAL vs. STUB vs. PLANNED

| Capability | Status | Notes |
|-----------|--------|-------|
| Pack Trilogy (law/reg/licensepacks) | **IMPLEMENTED** | mez-pack, composition engine, Pakistan content |
| Compliance Tensor (20 domains) | **IMPLEMENTED** | mez-tensor, fail-closed on extended domains |
| Corridor FSM (typestate) | **IMPLEMENTED** | mez-state — compile-time invalid transition prevention |
| Receipt chain + MMR | **IMPLEMENTED** | Dual commitment: hash-chain + MMR inclusion proofs |
| Fork resolution (evidence-driven) | **IMPLEMENTED** | Signed watcher attestations, timestamp bounds |
| Content-addressed artifacts | **IMPLEMENTED** | mez-crypto/cas.rs |
| Verifiable Credentials (Ed25519) | **IMPLEMENTED** | mez-vc |
| Write-path orchestration (5 primitives) | **IMPLEMENTED** | Both proxy and sovereign modes |
| Trade flow instruments | **IMPLEMENTED** | 4 archetypes, 10 transitions, Postgres persistence |
| Tax pipeline (Pakistan) | **IMPLEMENTED** | mez-agentic + mez-api/routes/tax.rs |
| Inter-zone corridor protocol | **IMPLEMENTED** | Handshake, receipt exchange, corridor registry |
| Zone composition algebra | **IMPLEMENTED** | 6-layer type system, corridor mesh, 210 zones |
| Data sovereignty enforcement | **IMPLEMENTED** | mez-core/src/sovereignty.rs |
| Agentic policy engine | **IMPLEMENTED** | mez-agentic |
| Arbitration system | **IMPLEMENTED** | mez-arbitration |
| Migration saga (8 phases) | **IMPLEMENTED** | CAS + idempotent compensation + EffectExecutor |
| Watcher economy (bonds/slashing) | **IMPLEMENTED** | mez-state/watcher.rs + REST API (10 endpoints) |
| National system adapters (PK) | **IMPLEMENTED** | FBR IRIS, SECP, SBP Raast, NADRA (trait + mock) |
| Payment rail adapters (Raast, SWIFT, Circle) | **STUB** | Trait defined, no real HTTP impl |
| BBS+ selective disclosure | **STUB** | Trait only (feature-gated) |
| ZK circuits (12 types) | **STUB** | Mock implementations, fail-closed in release |
| Poseidon2 hash | **STUB** | Returns NotImplemented (feature-gated) |
| Identity as dedicated Mass service | **PLANNED** | No identity-info.api.mass.inc yet |
| Smart Asset VM (SAVM) | **PLANNED** | No code exists |
| MASS L1 settlement layer | **PLANNED** | No code exists |

**Rule**: Do not write code that assumes STUB or PLANNED capabilities exist. STUB features return `MezError::NotImplemented`. PLANNED features have no code.

## VIII. ANTI-SLOP PROTOCOL

Before writing any code, verify it earns its existence. Kill on sight:

- Functions never called outside their module
- Types that duplicate another type with trivially different field names
- Match arms that all return the same value
- Tests that assert `true == true` or test only JSON deserialization with no logic
- Doc comments that restate the function signature
- `#[allow(dead_code)]` — if dead, delete it
- Mock impls that return `Ok(())` without exercising logic
- Compliance evals that return `Compliant` for all domains without checking
- VC issuance without actual compliance verification
- Corridor transitions that skip FSM validation
- Constants referenced only in tests
- Trait impls with exactly one implementor "for future genericity"

## IX. CODE REVIEW GATES

Before any merge:

- [ ] No new `unwrap()` outside `#[cfg(test)]`
- [ ] No `unimplemented!()`/`todo!()` in production paths
- [ ] No `anyhow` outside `mez-cli`
- [ ] No `std::sync::RwLock` — use `parking_lot`
- [ ] No SHA-256 bypassing `mez-core::digest`
- [ ] No Mass CRUD duplicated in EZ Stack
- [ ] No Python added
- [ ] No direct `reqwest` to Mass outside `mez-mass-client`
- [ ] Error types carry diagnostic context
- [ ] Naming: Momentum (never "Momentum Protocol"), Mass (never "Mass Protocol" casually), domains end `.inc`
- [ ] New types use mez-core newtypes (not raw String/Uuid)

## X. NAMING

| Term | Correct | Never |
|------|---------|-------|
| **Momentum** | "Momentum is a $1B+ venture fund and studio." | "Momentum Protocol" |
| **Mass** | "Mass provides five programmable primitives." | — |
| **Mass Protocol** | Only when discussing L1 settlement layer, ZKP circuits | In sales, README, casual usage |
| **momentum.inc** | Momentum's domain | momentum.xyz, .io, .com |
| **mass.inc** | Mass's domain | mass.xyz, .io |

## XI. PRIORITY ORDER

When conflicts arise:

1. **Security** — keys, secrets, auth, constant-time ops
2. **Correctness** — tensor eval, receipt chain linking, canonical digests
3. **Mass/EZ boundary** — no CRUD duplication, clean orchestration
4. **Deployment blockers** — resolve open items
5. **Code quality** — dead code, slop, untested paths

## XII. AUDIT STATUS

### Resolved (Phases A-H complete)

All P0 findings from the institutional readiness audit have been addressed except the items listed under "Open" below. Key closures:

- **P0-CORRIDOR-001..004**: Receipt chain implements dual-commitment model (hash-chain + MMR). Schema-conformant receipts and checkpoints with proof fields.
- **P0-CANON-001**: Momentum Canonical Form (MCF) documented as normative extension of JCS (ADR-002).
- **P0-FORK-001**: Evidence-driven fork resolution with signed watcher attestations, timestamp bounds.
- **P0-MIGRATION-001**: CAS + idempotent compensation + EffectExecutor trait + property tests.
- **P0-TENSOR-001**: Fail-closed on extended domains (Pending default, empty slice = error).
- **P0-ZK-001**: Fail-closed production policy (ProofPolicy, release rejects mock).
- **P0-DEPLOY-001**: Secret injection, no default credentials.
- **P0-PACK-001**: Pakistan Pack Trilogy (lawpacks, regpacks, licensepacks).
- **P0-CORRIDOR-NET-001**: Inter-zone corridor protocol with handshake + receipt exchange.
- **P1-CLI-001**: Evidence-gated corridor transitions.
- **P1-SCHEMA-001**: Draft 2020-12 compilation in CI.
- **P1-GOV-001**: Deprecated v1 state machine quarantined.
- **P1-PERF-001**: Cached compiled schema validators.
- **P1-API-001/002**: Contract-grade OpenAPI specs, Mass API specs pinned.

### Open

| ID | Issue | Severity | Owner |
|----|-------|----------|-------|
| P0-CRYPTO-001 | Poseidon2 stub (feature-gated) | P0 | Deferred Phase 4 |
| P0-CRYPTO-002 | BBS+ stub (feature-gated) | P0 | Deferred Phase 4 |
| P0-ANCHOR-001 | Anchor target is mock | P0 | Deferred Phase 4 |
| P0-IDENTITY-001 | No dedicated Mass identity service | P0 | Mass-side dependency |
| P1-SCHEMA-002 | ~~Schema URI inconsistency~~ **RESOLVED** — all `$ref` values use full `schemas.momentum-ez.org` URIs | P1 | protocol |
| P1-SCHEMA-003 | additionalProperties — security-critical schemas locked; non-critical schemas remain extensible | P1 | protocol |
| P1-NAMING-001 | Terminology glossary needed | P1 | protocol |

### Deployment Phase Gates

- **Phase 1 (Controlled Sandbox)**: READY. All entry criteria met.
- **Phase 2 (Corridor Activation)**: READY. All blockers resolved.
- **Phase 3 (Production)**: BLOCKED by identity service, real anchor target, HSM/KMS, external pen test.
- **Phase 4 (Cross-Border Expansion)**: Requires Poseidon2, BBS+, real ZK backends, watcher bond economics.

## XIII. KEY FILES

| Purpose | Path |
|---------|------|
| Workspace manifest | `mez/Cargo.toml` |
| API entry point | `mez/crates/mez-api/src/main.rs` |
| Orchestration pipeline | `mez/crates/mez-api/src/orchestration.rs` |
| Proxy routes | `mez/crates/mez-api/src/routes/mass_proxy.rs` |
| Sovereign routes | `mez/crates/mez-api/src/routes/mass_sovereign.rs` |
| Compliance tensor | `mez/crates/mez-tensor/src/evaluation.rs` |
| Receipt chain | `mez/crates/mez-corridor/src/receipt.rs` |
| Fork resolution | `mez/crates/mez-corridor/src/fork.rs` |
| Trade flow engine | `mez/crates/mez-corridor/src/trade.rs` |
| Canonical bytes | `mez/crates/mez-core/src/canonical.rs` |
| Zone composition | `mez/crates/mez-corridor/src/composition.rs` |
| Corridor registry | `mez/crates/mez-corridor/src/registry.rs` |
| Pack validation | `mez/crates/mez-pack/src/validation.rs` |
| Watcher economy API | `mez/crates/mez-api/src/routes/watchers.rs` |
| Schema validator | `mez/crates/mez-schema/src/validate.rs` |
| Corridor FSM (spec) | `governance/corridor.lifecycle.state-machine.v2.json` |
| Normative spec | `spec/` (24 chapters) |
| JSON schemas | `schemas/` (116 files, Draft 2020-12) |
| OpenAPI specs | `apis/` (4 contract-grade specs) |
| Zone definitions | `jurisdictions/` (210 zones) |
| Module descriptors | `modules/` (323 across 16 families) |
| Deploy configs | `deploy/docker/`, `deploy/aws/terraform/` |
| Deployment roadmap | `docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md` |
| Zone bootstrap guide | `docs/ZONE-BOOTSTRAP-GUIDE.md` |

---

**End of CLAUDE.md**

Momentum · `momentum.inc`
Mass · `mass.inc`
