# CLAUDE.md — Momentum SEZ Stack · Operational Anchor

**Version**: 8.0 — February 2026
**Authority**: Supersedes all prior versions. Based on Architecture Audit v8.0.

---

## I. WHAT THIS IS

A Rust workspace (`momentum-sez/stack`, v0.4.44) that provides **jurisdictional orchestration** for Mass — Momentum's five programmable primitives. Mass is Java/Spring Boot, live, processing real capital. This repo sits above Mass and adds compliance intelligence, corridor management, and cryptographic provenance.

The workspace has 16 crates, 109K lines of Rust, 3,029+ tests, zero Python.

## II. THE BOUNDARY (read this twice)

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
- Verifiable Credential issuance
- Orchestration pipeline: compliance eval → Mass API call → VC → attestation
- Smart Asset lifecycle, watcher economy, arbitration
- Deployment tooling (Docker, Terraform, K8s)

**The rule**: If it's "create/read/update/delete a business object" → Mass. If it's "evaluate whether that operation is compliant in this jurisdiction" → this repo. `msez-mass-client` is the sole authorized gateway to Mass APIs. No other crate may call Mass directly.

## III. CRATE MAP

```
msez-core         Foundation. CanonicalBytes, ComplianceDomain (20 variants),
                  identifier newtypes, MsezError, Timestamp, data sovereignty.
                  ZERO internal deps.

msez-crypto       Ed25519 (Zeroize), MMR, CAS, constant-time comparison.
msez-vc           W3C Verifiable Credentials, Ed25519Signature2020.
msez-tensor       Compliance Tensor V2, Manifold (Dijkstra path optimization).
msez-pack         Pack Trilogy processing, multi-jurisdiction composition engine.
msez-state        Typestate FSMs: corridor, migration, entity, license, watcher.
msez-corridor     Corridor lifecycle, receipt chains, netting, payment rail traits.
msez-agentic      Autonomous policy engine, trigger taxonomy, tax event generation.
msez-arbitration  Dispute lifecycle, escrow, institution registry.
msez-schema       JSON Schema validation (116 schemas in schemas/).
msez-zkp          ZK proof traits + stubs (Groth16, PLONK — mock only).
msez-compliance   Composes tensor + pack evaluation.
msez-mass-client  Typed HTTP client for Mass APIs. Depends on msez-core only.
msez-api          Axum HTTP server. Sole composition point — depends on all above.
msez-cli          CLI: zone validate/build/lock/sign, VC keygen/sign/verify.
msez-integration-tests  60+ test files, 34K lines.
```

Dependency DAG (simplified):
```
msez-core (leaf)
├── msez-crypto, msez-tensor, msez-pack, msez-state, msez-schema, msez-agentic
├── msez-vc (core + crypto)
├── msez-corridor (core + crypto + state)
├── msez-arbitration (core + crypto + vc)
├── msez-zkp (core + crypto)
├── msez-compliance (core + tensor + pack)
├── msez-mass-client (core only — newtypes)
└── msez-api (ALL crates — sole composition point)
```

## IV. BUILD & VERIFY

```bash
cargo check --workspace              # zero warnings required
cargo clippy --workspace -- -D warnings  # zero diagnostics required
cargo test --workspace               # all 3,029+ tests must pass
```

After any code change, run all three. No exceptions.

## V. INVARIANTS (violating any is a blocking failure)

1. **`msez-core` has zero internal crate dependencies.** External: serde, serde_json, thiserror, chrono, uuid, sha2 only.

2. **`msez-mass-client` depends only on `msez-core`** (for identifier newtypes). Never import tensors, corridors, packs, VCs, or other domain crates.

3. **No dependency cycles.** `msez-api` is the sole composition root. No other crate depends on it.

4. **All SHA-256 flows through `msez-core::digest`.** Three tiers:
   - Domain objects: `CanonicalBytes::new()` → `sha256_digest()`
   - Raw bytes: `sha256_raw()`
   - Streaming: `Sha256Accumulator`
   - `sha2::Sha256` direct usage appears ONLY in `msez-core/src/digest.rs` and `msez-crypto/src/mmr.rs`.
   - Verify: `grep -rn "use sha2" crates/ --include="*.rs"` — hits outside those two files are bugs.

5. **ComplianceDomain has exactly 20 variants**, defined once in `msez-core/src/domain.rs`:
   ```
   Aml, Kyc, Sanctions, Tax, Securities, Corporate, Custody, DataPrivacy,
   Licensing, Banking, Payments, Clearing, Settlement, DigitalAssets,
   Employment, Immigration, Ip, ConsumerProtection, Arbitration, Trade
   ```
   Every `match` is exhaustive. Compile-time assertion enforces COUNT == 20.

6. **Zero `unwrap()` in production code.** All `.unwrap()` must be inside `#[cfg(test)]`. Use `?`, `.map_err()`, `.ok_or_else()`, or `expect("reason")` for static values.

7. **Zero `unimplemented!()` or `todo!()` outside tests.** Phase 2 stubs return `MsezError::NotImplemented` (HTTP 501).

## VI. ORCHESTRATION PATTERN

Every **write** endpoint in `msez-api/src/routes/mass_proxy.rs` follows:

```
1. Pre-flight compliance → evaluate tensor across relevant domains for jurisdiction
2. Hard-block check → Sanctions NonCompliant = reject (legal requirement)
3. Mass API call → delegate via msez-mass-client (sole gateway)
4. VC issuance → sign compliance attestation as Verifiable Credential
5. Attestation storage → persist to Postgres for regulator queries
6. Return OrchestrationEnvelope { mass_response, compliance, credential, attestation_id }
```

**Read** endpoints (GET) are pass-through proxies — no compliance eval needed on reads.

This pattern is the SEZ Stack's entire value-add. Without it, Mass is generic CRUD.

## VII. WHAT IS REAL vs. STUB vs. PLANNED

| Capability | Status | Notes |
|-----------|--------|-------|
| Pack Trilogy (law/reg/licensepacks) | **IMPLEMENTED** | msez-pack, composition engine |
| Compliance Tensor (20 domains) | **IMPLEMENTED** | msez-tensor |
| Corridor FSM (typestate) | **IMPLEMENTED** | msez-state — compile-time invalid transition prevention |
| Receipt chain + MMR | **IMPLEMENTED** | msez-crypto |
| Content-addressed artifacts | **IMPLEMENTED** | msez-crypto/cas.rs |
| Verifiable Credentials (Ed25519) | **IMPLEMENTED** | msez-vc |
| Write-path orchestration (5 primitives) | **IMPLEMENTED** | msez-api/routes/mass_proxy.rs |
| Tax pipeline (Pakistan) | **IMPLEMENTED** | msez-agentic + msez-api/routes/tax.rs |
| Data sovereignty enforcement | **IMPLEMENTED** | msez-core/src/sovereignty.rs |
| Agentic policy engine | **IMPLEMENTED** | msez-agentic |
| Docker/K8s/Terraform deployment | **IMPLEMENTED** | deploy/ |
| Arbitration system | **IMPLEMENTED** | msez-arbitration |
| Compliance Manifold (Dijkstra) | **IMPLEMENTED** | msez-tensor/manifold.rs |
| Migration saga (8 phases) | **IMPLEMENTED** | msez-state/migration.rs |
| Watcher economy (bonds/slashing) | **IMPLEMENTED** | msez-state/watcher.rs |
| Payment rail adapters (Raast, SWIFT, Circle) | **STUB** | Trait defined, no real impl |
| BBS+ selective disclosure | **STUB** | Trait only |
| ZK circuits (12 types) | **STUB** | Mock implementations |
| Poseidon2 hash | **STUB** | Returns NotImplemented |
| Canonical Digest Bridge | **STUB** | Poseidon2 side unimplemented |
| Identity as dedicated Mass service | **PLANNED** | No identity-info.api.mass.inc yet |
| Smart Asset VM (SAVM) | **PLANNED** | No code exists |
| MASS L1 settlement layer | **PLANNED** | No code exists |

**IMPORTANT**: Do not write code that assumes STUB or PLANNED capabilities exist. Do not generate mock implementations that pretend to be real. When a feature is STUB, the code must return `MsezError::NotImplemented` with a clear message. When PLANNED, no code should reference it as if it works.

## VIII. SEVEN DEPLOYMENT BLOCKERS (current priority)

These block sovereign zone deployment. Address in this order:

1. **Mass API health gating** — `msez-api` bootstrap must verify Mass API connectivity before accepting traffic. Readiness probe must include Mass reachability.

2. **Identity primitive** — No dedicated `identity-info.api.mass.inc`. Rust client (`IdentityClient`) is an aggregation facade. Ship the Java service or honestly flag it as 4/5.

3. **Contract tests** — `msez-mass-client` tests use hardcoded mocks. No validation against live Swagger specs. A field rename in Java breaks the Rust client silently.

4. **Inter-zone networking** — Corridors work in-process only. No P2P protocol for two zones to exchange receipts over the network. Each deployed zone is an island.

5. **Pack Trilogy content** — Zero real lawpacks with real legislative text. No Pakistan Income Tax Ordinance 2001 in AKN XML. Tensor evaluates against empty rulesets.

6. **National system adapters** — FBR IRIS, SBP Raast, NADRA, SECP have no implementations. `NationalSystemAdapter` trait needed with production + mock impls.

7. **Placeholder crypto keys** — `deploy-zone.sh` writes placeholder Ed25519 keys. Use `msez-cli keygen` instead.

## IX. ANTI-SLOP PROTOCOL

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

## X. CODE REVIEW GATES

Before any merge:

- [ ] No new `unwrap()` outside `#[cfg(test)]`
- [ ] No `unimplemented!()`/`todo!()` in production paths
- [ ] No `anyhow` outside `msez-cli`
- [ ] No `std::sync::RwLock` — use `parking_lot`
- [ ] No SHA-256 bypassing `msez-core::digest`
- [ ] No Mass CRUD duplicated in SEZ Stack
- [ ] No Python added
- [ ] No direct `reqwest` to Mass outside `msez-mass-client`
- [ ] Error types carry diagnostic context
- [ ] Public functions have doc comments describing behavior
- [ ] Naming: Momentum (never "Momentum Protocol"), Mass (never "Mass Protocol" casually), domains end `.inc`
- [ ] New types use msez-core newtypes (not raw String/Uuid)

## XI. NAMING

| Term | Correct | Never |
|------|---------|-------|
| **Momentum** | "Momentum is a $1B+ venture fund and studio." | "Momentum Protocol" |
| **Mass** | "Mass provides five programmable primitives." | — |
| **Mass Protocol** | Only when discussing L1 settlement layer, ZKP circuits | In sales, README, casual usage |
| **momentum.inc** | Momentum's domain | momentum.xyz, .io, .com |
| **mass.inc** | Mass's domain | mass.xyz, .io |

## XII. PRIORITY ORDER

When conflicts arise:

1. **Security** — keys, secrets, auth, constant-time ops
2. **Correctness** — tensor eval, receipt chain linking, canonical digests
3. **Mass/SEZ boundary** — no CRUD duplication, clean orchestration
4. **Deployment blockers** — resolve §VIII items
5. **Code quality** — dead code, slop, untested paths

## XIII. AUDIT DIRECTIVES

When auditing, proceed in this order:

**Phase 1 — Structural**: Verify invariants §V. `grep` for `sha2::Sha256`, `reqwest::`, `unwrap()`, `todo!()` in production code.

**Phase 2 — Boundary**: For each primitive, trace HTTP request → orchestration → Mass API call → response. Confirm compliance → Mass → VC → attestation on every write path. Flag any SEZ code storing Mass-owned data.

**Phase 3 — Correctness**: CanonicalBytes determinism. ComplianceDomain == 20. ComplianceState lattice ops. Receipt chain hash linking. Corridor FSM vs `governance/corridor.lifecycle.state-machine.v2.json`.

**Phase 4 — Security**: Ed25519 Zeroize-on-drop. Constant-time token comparison. No secrets in logs. Auth on state-mutating endpoints.

**Phase 5 — Test quality**: No tautological assertions. No deser-only tests. Integration tests compose multiple crates. No `#[ignore]`.

## XIV. REFERENCE FILES

For deep context, read these (don't embed them here):

- `spec/` — Normative MSEZ specification (architecture, corridors, lawpacks, etc.)
- `schemas/` — 116 JSON schemas for all data structures
- `apis/` — OpenAPI specs (corridor-state, smart-assets, mass-node, regulator-console)
- `modules/mass-primitives/` — Five primitive module definitions with policy-to-code maps
- `profiles/` — Deployment templates (digital-financial-center, charter-city, trade-playbook, etc.)
- `governance/corridor.lifecycle.state-machine.v2.json` — Canonical corridor FSM definition
- `deploy/` — Docker, Terraform (AWS), K8s manifests, deployment scripts
- `SEZ_Stack_Mass_API_Deep_Audit_v7.md` — Prior audit findings
- `msez/AUDIT_FINDINGS.md` — Structural integrity audit
- `msez/HARDENING_REPORT.md` — Security defect resolution evidence

For Pakistan GovOS context: see project knowledge for `mass_pakistan_architecture_v4` schematic.

## XV. DATA FLOW (Pakistan GovOS example)

```
GovOS Console → msez-api
  ├─ msez-tensor: evaluate 20 domains for PAK jurisdiction
  ├─ msez-pack: check lawpack (Income Tax Ord. 2001, Sales Tax Act 1990, etc.)
  ├─ msez-pack: check regpack (FBR rates, FATF AML/CFT, sanctions)
  ├─ msez-pack: check licensepack (SECP, BOI, PTA, PEMRA, DRAP)
  ├─ msez-mass-client → organization-info (create entity, bind NTN)
  ├─ msez-mass-client → treasury-info (create PKR account, withholding config)
  ├─ msez-mass-client → consent (tax assessment sign-off)
  ├─ msez-vc: issue FormationComplianceCredential
  ├─ msez-corridor: update PAK↔UAE/KSA/CHN corridor state
  ├─ msez-agentic: register for auto tax event generation
  └─ Return: OrchestrationEnvelope { mass_response, compliance, credential, attestation_id }
```

Tax collection pipeline:
```
Every transaction on Mass → tax event (msez-agentic) → withholding (tax routes)
  → FBR IRIS report → AI gap analysis → 10.3% → 15%+ GDP target
```

---

**End of CLAUDE.md v8.0**

Momentum · `momentum.inc`
Mass · `mass.inc`
