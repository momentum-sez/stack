# CLAUDE.md — Momentum SEZ Stack v0.4.44 GENESIS

## Production Readiness Work Plan for Claude Code

**Repository:** `momentum-sez/stack`
**Spec Version:** 0.4.44-GENESIS
**License:** BUSL-1.1
**Architecture:** Rust workspace (single `msez-api` binary), 17 crates, replaces prior Python stack
**Audit Artifact Schema:** `schemas/audit/institutional-readiness-audit.schema.json` (Draft 2020-12)
**Last Audit Reconciliation:** 2026-02-18

---

## 1. REPOSITORY MAP

```
momentum-sez/stack/
├── .github/workflows/       # CI: Draft 2020-12 schema validation, serde guard, credential guard, ZK mock guard, trade-playbook closure
├── apis/                    # OpenAPI specs (scaffold-grade)
│   ├── smart-assets.openapi.yaml
│   ├── corridor-state.openapi.yaml
│   ├── mass-node.openapi.yaml
│   └── regulator-console.openapi.yaml
├── contexts/
├── deploy/
│   ├── docker/docker-compose.yaml   # Single Rust binary + Postgres + Prometheus
│   └── aws/terraform/               # EKS + RDS + KMS (content not fully verified)
├── dist/artifacts/schema/
├── docs/
├── governance/
│   ├── deprecated/corridor.lifecycle.state-machine.v1.json  # QUARANTINED (P1-GOV-001 resolved)
│   └── corridor.lifecycle.state-machine.v2.json             # Active lifecycle definition
├── jurisdictions/_starter/zone.yaml
├── modules/                 # Module descriptors (claim: 298 across 16 families)
├── msez/                    # Rust workspace root
│   ├── Cargo.toml           # workspace license: BUSL-1.1
│   └── crates/
│       ├── msez-api/        # Consolidated API binary
│       ├── msez-cli/        # CLI: corridor lifecycle, keygen, validate, lock
│       ├── msez-core/       # CanonicalBytes, ContentDigest, SHA-256 primitives
│       ├── msez-corridor/   # Receipt chain, checkpoint, fork resolution, anchor, bridge
│       ├── msez-crypto/     # Ed25519, MMR, Poseidon2 (stub), BBS+ (stub)
│       ├── msez-mass-client/# Mass API client boundary
│       ├── msez-pack/       # Pack Trilogy (lawpacks, regpacks, licensepacks)
│       ├── msez-schema/     # JSON Schema Draft 2020-12 validator + codegen policy
│       ├── msez-tensor/     # Compliance Tensor (20 domains, lattice aggregation)
│       ├── msez-vc/         # Verifiable Credentials (Smart Asset Registry VC)
│       └── msez-zkp/        # ZK proof system (Phase 1 deterministic mock)
├── profiles/
├── registries/
├── rulesets/
├── schemas/                 # JSON Schema files (Draft 2020-12)
│   ├── corridor.receipt.schema.json
│   ├── corridor.checkpoint.schema.json
│   ├── corridor.schema.json
│   └── stack.lock.schema.json
├── spec/                    # 25 chapters (repo spec, normative)
│   └── 40-corridors.md      # Corridor receipt/checkpoint hash rules (RFC 8785 JCS)
└── tests/
```

---

## 2. SYNTHESIZED P0 FINDINGS — PRODUCTION BLOCKERS

> **Audit Reconciliation Note (2026-02-18):** The following P0 findings were identified
> across multiple independent audit passes. Many have been **resolved** in the current
> codebase. Each finding below now carries a status tag: `[RESOLVED]`, `[OPEN]`, or
> `[PARTIALLY RESOLVED]`. Resolved findings are preserved for audit trail purposes.

All P0s below are confirmed across multiple independent audit passes. They are ordered by blast radius (integrity > compliance > deployment).

### P0-CORRIDOR-001: Receipt Chain Does Not Enforce Spec Hash-Chain Model — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/receipt.rs`
**Resolution:** `ReceiptChain` now maintains `final_state_root` (genesis-seeded hash-chain head). `append()` enforces `receipt.prev_root == final_state_root`. After appending, `final_state_root` is updated to `receipt.next_root`. Dual commitment model (hash-chain + MMR) implemented. Golden vector tests and adversarial tests (prev_root confusion) in place.
**Confirmed by:** Pass 1 Fidelity Audit, Formal Methods Audit, Institutional Assessment
**Issue:** Implementation enforces `receipt.prev_root == current_mmr_root`, but the spec requires `prev_root` to be the previous state root (hash-chain model seeded from `genesis_root`), and `next_root` to be derived from the receipt payload (excluding `proof` and `next_root`). These are two different commitment models.
**Impact:** Interoperability failure. Receipts produced by this implementation are not verifiable by any tooling following `spec/40-corridors.md` and `schemas/corridor.receipt.schema.json`. Cross-party corridor reconciliation becomes non-deterministic. Regulator verification breaks.
**Remediation:**
1. Implement two parallel commitments per spec: `final_state_root` (hash-chain head, genesis-seeded) and MMR over receipt digests (for inclusion proofs).
2. Change `append()` to enforce: `receipt.prev_root == final_state_root` (hash-chain continuity).
3. Enforce `receipt.next_root == SHA256(JCS(receipt_without_proof_and_next_root))`.
**Verification:** Golden vector conformance tests; schema validation roundtrip against `schemas/corridor.receipt.schema.json`; genesis_root fixture for first receipt.
**Effort:** M | **Owner:** protocol + security

### P0-CORRIDOR-002: next_root Is Not Computed or Verified — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/receipt.rs`
**Resolution:** `compute_next_root()` implemented: strips `proof` and `next_root` fields, deduplicates + lexicographically sorts digest sets, computes `SHA256(MCF(payload))`. `append()` recomputes and compares — rejects mismatch. Property tests cover permutations and duplicate digest sets.
**Confirmed by:** Pass 1, Formal Methods, Red Team
**Issue:** `append()` blindly appends `receipt.next_root` to MMR without recomputing or verifying it. Spec requires `next_root = SHA256(JCS(receipt_without_proof_and_next_root))` and digest-set normalization (dedupe + sort lexicographically).
**Impact:** Proof/commitment forgery surface. Caller can set arbitrary `next_root`. Non-determinism across implementations. Fork amplification.
**Remediation:**
1. Implement `compute_next_root(receipt)` that strips `proof` and `next_root`, normalizes digest sets, computes `SHA256(JCS(payload))`.
2. Enforce in `append()`: recompute and compare; reject mismatch.
**Verification:** Property tests (permutations + duplicates in digest sets must not change `next_root`); golden vector fixtures.
**Effort:** M | **Owner:** protocol + security

### P0-CORRIDOR-003: CorridorReceipt Is Not Schema-Conformant — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/receipt.rs`, `schemas/corridor.receipt.schema.json`
**Resolution:** `CorridorReceipt` struct now includes `proof` (as `Option<ReceiptProof>`), `DigestEntry` enum supporting both digest strings and `ArtifactRef`, optional `transition`/`zk`/`anchor` fields. Schema validation tests verify conformance. **Note:** `proof` is `Option` to support construction-before-signing; callers must ensure proof is populated before persistence.
**Confirmed by:** Pass 1, Pass 2
**Issue:** Rust struct omits required `proof` field. Digest sets are `Vec<String>` but schema allows `DigestString | ArtifactRef` union. Optional but important fields (`transition`, `zk`, `anchor`, `transition_type_registry_digest_sha256`) not represented.
**Impact:** Schema validation will reject any receipt this struct produces. Without `proof`, receipts are unsigned — any party with write access can inject/replay/rewrite.
**Remediation:** Extend `CorridorReceipt` to match schema: add `proof` (one object or array), implement digest-set item type as `enum { DigestString, ArtifactRef }`, add optional fields.
**Verification:** Serialize receipt → validate with `SchemaValidator` against `corridor.receipt.schema.json`; negative test: missing proof must fail.
**Effort:** M | **Owner:** protocol

### P0-CORRIDOR-004: Checkpoint Is Non-Conformant and Lacks Proof — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/receipt.rs`, `schemas/corridor.checkpoint.schema.json`
**Resolution:** Checkpoint struct now includes `genesis_root`, `final_state_root`, `receipt_count`, `MmrCommitment` (type/algorithm/size/root/peaks), digest sets, and `proof` field. Schema validation tests verify conformance. **Note:** Same `Option<proof>` construction pattern as receipts — callers must sign before persistence.
**Confirmed by:** Pass 1, Pass 2
**Issue:** Checkpoint only includes `(corridor_id, height, mmr_root, timestamp, checkpoint_digest)`. Schema requires `genesis_root`, `final_state_root`, `receipt_count`, digest sets, `mmr` object (type/algorithm/size/root/peaks), and `proof`. No proof means checkpoints are unsigned claims.
**Impact:** Verifier bootstrap impossible per spec. Forgery surface: malicious relayer can provide fake checkpoints.
**Remediation:** Implement schema-conformant checkpoint type with all required fields. Sign checkpoint payloads.
**Verification:** Schema validation tests + golden vectors; end-to-end: bootstrap from checkpoint and verify tail receipts.
**Effort:** L | **Owner:** protocol + security

### P0-CANON-001: Canonicalization Is Not RFC 8785 JCS — `[RESOLVED]`

**Files:** `msez/crates/msez-core/src/canonical.rs`, `spec/40-corridors.md`
**Resolution:** Option (b) chosen: `spec/40-corridors.md` updated to normatively define **MCF (Momentum Canonical Form)** — RFC 8785 JCS with two additional safety coercions: (1) float rejection, (2) datetime normalization to UTC with `Z` suffix. Both sides now agree. Cross-language golden vectors documented in `msez-core::canonical::tests`.
**Confirmed by:** Formal Methods Audit, Cryptographic Correctness Pass
**Issue:** `spec/40-corridors.md` normatively requires `SHA256(JCS(json))` where JCS is RFC 8785. But `CanonicalBytes` applies extra coercions not in RFC 8785: float rejection, RFC 3339 datetime normalization/truncation.
**Impact:** Implementation is provably not "JCS exact" as spec states. Any external verifier following the spec will compute different digests for the same payload.
**Remediation:** Either:
- (a) Update `CanonicalBytes` to implement exact RFC 8785 JCS, OR
- (b) Update `spec/40-corridors.md` to normatively define "JCS + Momentum coercions" and version/tag it.
Today it is inconsistent. Both sides must agree.
**Verification:** Cross-language golden vectors (Rust/TS/Python) for canonicalization output.
**Effort:** M | **Owner:** protocol + security

### P0-FORK-001: Fork Resolution Is Manipulable — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/fork.rs`
**Resolution:** Fork selection now requires cryptographically bound signed watcher attestations. Timestamp bounds enforced (monotonic time, drift bound). Equivocation detection implemented (conflicting attestations from same watcher trigger slashing). Adversarial tests cover backdated timestamps and attestation inflation.
**Confirmed by:** Formal Methods, Red Team, Institutional Assessment
**Issue:** Fork selection uses `(-attestation_count, timestamp, lex_candidate_root)` but `attestation_count` and `timestamp` are not cryptographically bound or verified — code explicitly states "must be independently verified" but doesn't implement it. Attacker can publish `ForkBranch` with `attest=Q+1, ts=1970-01-01` and deterministically win selection.
**Impact:** Safety violation: honest nodes converge on attacker-selected root. Honest progress can be overwritten.
**Remediation:**
1. Replace raw `(timestamp, attestation_count)` fields with a set (or Merkle root) of signed watcher attestations binding: parent root, candidate root, height/sequence, timestamp constraints.
2. Verify those signatures in the fork module.
3. Enforce monotonic time: `ts[i] >= ts[i-1]` and `ts[i] <= now + drift_bound`.
**Verification:** Adversarial test: craft fork with backdated timestamp; must be rejected.
**Effort:** L | **Owner:** security + protocol

### P0-MIGRATION-001: Saga Compensation Is Unprovable — `[RESOLVED]`

**Files:** `msez/crates/msez-corridor/src/migration.rs`
**Resolution:** Complete reimplementation with: (1) Explicit forward side-effects (`Lock`/`Unlock`/`Mint`/`Burn`) with defined inverses. (2) `compensate()` is idempotent — second call returns `Ok(Compensated)` (not error). (3) Timeout triggers compensation deterministically. (4) CAS versioning (`version` field incremented on every transition; `VersionConflict` error on mismatch). (5) `no_duplicate_invariant()` verifier: `¬(asset_exists_source ∧ asset_exists_dest)`. (6) `inverse(inverse(e)) == e` property tested for all effect types.
**Confirmed by:** Formal Methods Audit, Institutional Assessment
**Issue:** Migration state machine enforces deadlines and transitions, but:
- `compensate()` only logs and flips to `Compensated` — no modeling of forward side-effects or their inverses.
- `TimedOut` transition does not execute/record compensation.
- Second `compensate()` call errors (`AlreadyTerminal`) rather than no-op — not idempotent.
**Impact:** Cannot prove: compensation is inverse of forward steps, compensation is idempotent, partial failure cannot duplicate assets, timeout cannot orphan state. All four are required for financial safety.
**Remediation:**
1. Add explicit forward side-effect flags (`lock/unlock/mint/burn`) and compensation functions that invert them.
2. Make compensation idempotent (second call = no-op returning same terminal, not error).
3. On timeout, execute/record compensation deterministically.
4. Add `(migration_id, version)` CAS or pessimistic locking at persistence layer for distributed concurrency.
**Verification:** Property tests: `forward + compensate = pre-state`; no-dup invariant: `¬(asset_exists_source ∧ asset_exists_dest)`.
**Effort:** L | **Owner:** protocol

### P0-ANCHOR-001: Anchoring Is a Mock — `[OPEN]`

**Files:** `msez/crates/msez-crypto/` (anchor module)
**Confirmed by:** Formal Methods, Institutional Assessment
**Issue:** `MockAnchorTarget` always reports `confirmed`. No L1 finality proof, reorg handling, or inclusion verification exists.
**Impact:** No adversarial proof obligation about L1 finality can be satisfied. Anchoring is aspirational, not functional.
**Remediation:** Implement at minimum a real anchor target interface with finality confirmation delay, reorg detection, and inclusion proof verification. Feature-gate mock for dev only.
**Verification:** Integration test with simulated L1 (delayed confirmation, reorg scenario).
**Effort:** XL | **Owner:** protocol + infra

### P0-TENSOR-001: Extended Compliance Domains Default to NotApplicable (Passes) — `[RESOLVED]`

**Files:** `msez/crates/msez-tensor/src/evaluation.rs`, `msez/crates/msez-tensor/src/tensor.rs`
**Resolution:** (1) `TensorSlice::aggregate_state()` returns `ComplianceState::Pending` for empty slices (fail-closed). (2) `all_passing()` returns `false` for empty slices. (3) `ComplianceTensor::new()` initializes applicable domains to `Pending` (not `NotApplicable`). (4) Non-applicable domains only get `NotApplicable` when explicitly configured via `JurisdictionConfig::applicable_domains()`. (5) Tests verify empty slice returns Pending, not Compliant.
**Confirmed by:** Institutional Assessment, Red Team, Formal Methods
**Issue:** Only base domains evaluated. Extended domains (including LICENSING, BANKING, DATA_PRIVACY, SANCTIONS) return `NotApplicable`, which is treated as passing. Additionally, `TensorSlice::aggregate_state()` returns `Compliant` for empty slices (folds from `Compliant`).
**Impact:** Regulatory bypass. Transactions proceed without checks for domains that should be enforced. Attacker can induce empty slice → compliant aggregate.
**Remediation:**
1. Fail-closed on unimplemented domains in production: treat "not implemented" as `Pending` or `NonCompliant`, not `NotApplicable`.
2. Require all mandatory domains present and evaluated; treat empty slices as error/Pending.
3. Authority-bound state transitions for Exempt/NotApplicable decisions (signed policy artifact required).
**Verification:** Property tests: no path elevates failing state to passing; empty slice returns error.
**Effort:** M | **Owner:** protocol + security

### P0-CRYPTO-001: Poseidon2 Is a Stub — `[OPEN]`

**Files:** `msez/crates/msez-crypto/src/poseidon.rs` (feature-gated)
**Confirmed by:** Cryptographic Correctness Pass
**Issue:** `poseidon2_digest()` and `poseidon2_node_hash()` return `NotImplemented`.
**Impact:** Any spec element relying on Poseidon2 (ZK-friendly hashing, proof-internal commitments) cannot execute. Mixed deployments risk network divergence.
**Remediation:** Implement Poseidon2 with fixed parameters; publish test vectors; hard feature matrix preventing disagreement on hashing rules.
**Effort:** L-XL | **Owner:** security + protocol

### P0-CRYPTO-002: BBS+ Selective Disclosure Is a Stub — `[OPEN]`

**Files:** `msez/crates/msez-crypto/src/bbs.rs` (feature-gated)
**Confirmed by:** Cryptographic Correctness Pass
**Issue:** Entire BBS+ module is stubbed. Cannot support selective disclosure proofs for KYC/compliance claims.
**Remediation:** Integrate vetted BBS+ library; add proof verification tests; specify canonical message encoding and domain separation.
**Effort:** L-XL | **Owner:** security

### P0-ZK-001: ZK Proof System Is Phase 1 Mock — `[RESOLVED]`

**Files:** `msez/crates/msez-zkp/src/policy.rs`
**Resolution:** `ProofPolicy` module implements fail-closed production policy. (1) `PolicyMode::Production` unconditionally rejects `ProofBackend::Mock`. (2) Release builds (`not(debug_assertions)`) default to `Production` mode. (3) `MSEZ_PROOF_POLICY` env var allows runtime override. (4) CI guard checks that release builds have no mock features enabled. (5) The mock proof system remains available for development/testing but is gated behind `Development` policy. **Note:** The mock is still the only _implemented_ backend — real Groth16/PLONK backends are feature-gated stubs. But the policy layer ensures mock proofs cannot be accepted as authoritative in production builds.
**Confirmed by:** Red Team, Institutional Assessment
**Issue:** Phase 1 uses deterministic SHA-256 mock proof system by default. Real backends are feature-gated. If verifier accepts mock proofs as authoritative, attacker can supply proofs without possessing underlying witness/claims.
**Impact:** Catastrophic if proofs gate compliance: unauthorized transactions appear compliant.
**Remediation:**
1. Fail-closed production policy: require real proof backend; reject mock proof types.
2. Make proof backend choice a signed, content-addressed policy artifact.
3. Compile-time guardrails + CI checks for release builds.
**Verification:** CI gate: release profile must not enable mock feature; runtime attestation of proof backend.
**Effort:** M | **Owner:** security + protocol

### P0-IDENTITY-001: Identity Primitive Has No Dedicated Mass Service — `[OPEN]`

**Confirmed by:** Institutional Assessment
**Issue:** `IdentityClient` is a facade over other Mass services. There is no `identity-info.api.mass.inc`. This is a sovereign due-diligence blocker.
**Remediation:** Ship real Identity primitive service (Mass-side) + wire `IdentityClient` to it.
**Effort:** L-XL (2-6 weeks) | **Owner:** protocol + partner

### P0-CORRIDOR-NET-001: No Inter-Zone Corridor Networking — `[OPEN]`

**Confirmed by:** Institutional Assessment
**Issue:** Corridor receipt chain and cryptography exist, but no network protocol / discovery / handshake for Zone A ↔ Zone B. Blocks "AWS of Zones" cross-border value proposition.
**Remediation:** Implement inter-zone corridor protocol + 2-zone integration test (protocol spec + handshake + receipt exchange + watcher attestations + replay protection).
**Effort:** XL (4-8 weeks) | **Owner:** protocol + infra

### P0-PACK-001: Pack Trilogy Has No Real Jurisdiction Content — `[OPEN]`

**Confirmed by:** Institutional Assessment
**Issue:** Schemas + validation + signing + CAS exist, but no real Pakistan statutes / rates / license categories. Compliance evaluation not meaningful for sovereign deployment.
**Remediation:** Deliver real pack content for target jurisdiction(s). Minimum for Pakistan pilot: core tax statutes + withholding rules + SECP license categories + sanctions + AML/CFT calendars.
**Effort:** XL (6-16+ weeks, parallel) | **Owner:** legal + protocol

### P0-DEPLOY-001: Default Credentials in Deploy Paths — `[RESOLVED]`

**Confirmed by:** Institutional Assessment, Red Team
**Resolution:** Docker Compose uses `${POSTGRES_PASSWORD:?}` and `${GRAFANA_PASSWORD:?}` enforcement (required env vars, fails if unset). Deploy script generates random credentials via `openssl rand`. CI guard rejects `POSTGRES_PASSWORD=msez` or similar hardcoded defaults. No hardcoded credentials remain in deploy paths.
**Issue:** `docker-compose` and deploy script default to `POSTGRES_PASSWORD=msez`. Unacceptable outside local dev.
**Remediation:** Wire secret manager; eliminate all default credentials; key custody model (HSM/KMS) + rotation.
**Effort:** S-M | **Owner:** infra + security

---

## 3. SYNTHESIZED P1 FINDINGS — HIGH SEVERITY

### P1-CLI-001: Corridor CLI Discards Evidence Inputs — `[RESOLVED]`

**Files:** `msez/crates/msez-cli/src/corridor.rs`
**Resolution:** Evidence is now computed and stored for every transition type: `Submit` hashes agreement + pack_trilogy file contents via `Sha256Accumulator`, `Activate` hashes approval digests, `Halt` hashes reason + authority, `Suspend` hashes reason, `Resume` hashes resolution. All evidence digests stored in `TransitionRecord.evidence_digest`. **Remaining gap:** No enforcement that specific transition types REQUIRE evidence (evidence is always computed but not validated as mandatory). Consider adding `--strict` mode for production use.
**Effort:** S | **Owner:** protocol

### P1-SCHEMA-001: CI Validates Schemas as JSON Only, Not Draft 2020-12 — `[RESOLVED]`

**Files:** `.github/workflows/ci.yml`, `msez/crates/msez-schema/src/validate.rs`
**Resolution:** CI now runs `cargo test --package msez-schema -- test_load_all_schemas test_ref_resolution` which compiles all 116+ schemas with `Draft202012` and resolves all `$ref` URIs. `SchemaValidator` uses `jsonschema::options().with_draft(jsonschema::Draft::Draft202012).with_retriever(retriever)` for full compilation and cross-reference resolution.
**Effort:** S | **Owner:** infra

### P1-SCHEMA-002: Schema URI Inconsistency — `[RESOLVED]`

**Files:** `schemas/stack.lock.schema.json`, `msez/crates/msez-schema/src/validate.rs`
**Resolution:** CI now includes a guard that checks all `$id` URIs use the canonical domain `schemas.momentum-sez.org/msez/`. Non-canonical URIs cause CI failure. The validator resolves all URIs through the `LocalSchemaRetriever` using `Arc<HashMap<String, Value>>` for efficient sharing.
**Effort:** S | **Owner:** protocol

### P1-GOV-001: Deprecated Governance State Machine v1 Still Present — `[RESOLVED]`

**Files:** `governance/deprecated/corridor.lifecycle.state-machine.v1.json`
**Resolution:** v1 state machine moved to `governance/deprecated/` directory. v2 is the sole active reference at `governance/corridor.lifecycle.state-machine.v2.json`.
**Effort:** XS | **Owner:** protocol

### P1-API-001: OpenAPI Specs Are Scaffold-Grade

**Files:** `apis/*.openapi.yaml`
**Issue:** Each spec self-identifies as scaffold/skeleton. Gaps in error models, auth models, idempotency semantics, pagination.
**Remediation:** Version and harden with schema refs, auth, error models, idempotency, pagination.
**Effort:** L-XL (2-4 weeks) | **Owner:** protocol + infra

### P1-API-002: Mass API Alignment Cannot Be Verified

**Issue:** No live Swagger URLs discoverable in repo artifacts. Blocks integration audit.
**Remediation:** Publish stable Mass OpenAPI docs; commit versioned snapshots under `apis/mass/*.yaml`.
**Effort:** M | **Owner:** protocol + partner

### P1-DEPLOY-002: Doc/Deploy Drift — `[RESOLVED]`

**Issue:** Deploy script prints endpoints for defunct multi-service layout; docker-compose describes single binary architecture.
**Resolution:** Deploy script now outputs single-binary architecture endpoints: `MSEZ API (all services): http://localhost:8080`, health check, observability (Prometheus :9090, Grafana :3000), PostgreSQL :5432. No references to defunct multi-service layout remain.
**Effort:** S | **Owner:** infra

### P1-SCHEMA-003: additionalProperties:true on Security-Critical Objects

**Files:** Multiple corridor/API schemas
**Issue:** `evidence: type: object additionalProperties: true` on corridor finality status. Smart assets OpenAPI has multiple schemas with `additionalProperties: true`.
**Remediation:** Set `additionalProperties: false` on all security-critical objects; keep extensibility only in explicitly namespaced subobjects.
**Effort:** M | **Owner:** protocol + security

### P1-PERF-001: Schema Validator Constructs Per Call — `[RESOLVED]`

**Files:** `msez/crates/msez-schema/src/validate.rs`
**Resolution:** `SchemaValidator` now caches compiled validators per schema `$id` using `RwLock<HashMap<String, jsonschema::Validator>>`. First validation compiles and caches; subsequent calls reuse the cached validator via read lock (concurrent readers allowed). Schema map shared via `Arc<HashMap>` to avoid deep-cloning.
**Effort:** S-M | **Owner:** protocol

### P1-NAMING-001: Mass Primitives Naming Inconsistency

**Issue:** Repo spec says "Instruments"; investor/government materials say "Fiscal instruments/rails".
**Remediation:** Publish canonical glossary with explicit model/endpoint mapping.
**Effort:** S | **Owner:** protocol

---

## 4. SYNTHESIZED P2 FINDINGS — MEDIUM SEVERITY

### P2-CANON-002: Merkle Helper Uses String Concatenation

**Files:** `msez/crates/msez-tensor/` (commitment.rs Merkle helper)
**Issue:** Hashes canonicalized string concatenations of hex digests rather than byte-level Merkle. Deterministic but diverges from any external spec expecting byte-concat of 32-byte nodes.
**Effort:** S | **Owner:** protocol

### P2-SA-001: binding_status Is Unrestricted String

**Files:** `msez/crates/msez-vc/` (JurisdictionBinding)
**Issue:** `binding_status: String` allows invalid values; should be enum `{active, suspended, exited}`.
**Effort:** XS | **Owner:** protocol

### P2-SA-002: VC Constructor Doesn't Enforce asset_id Binding

**Files:** `msez/crates/msez-vc/`
**Issue:** VC `credentialSubject.asset_id` is not verified to equal `compute_asset_id(genesis)`.
**Effort:** S | **Owner:** protocol

### P2-CANON-003: Sorted Key Assumption Is Test-Only

**Files:** `msez/crates/msez-core/src/canonical.rs`
**Issue:** Test asserts `serde_json::Map` iterates sorted; not enforced in production builds. Supply-chain/feature-flag risk if `preserve_order` enabled.
**Effort:** S | **Owner:** security

### P2-CL-001: Corridor Transition Timestamps Are Wall-Clock

**Issue:** `transmute_to()` uses `Utc::now()` — non-deterministic if used in consensus.
**Effort:** S | **Owner:** protocol

### P2-DEPLOY-003: No General CAS Verification in CI

**Files:** `.github/workflows/ci.yml`
**Issue:** Trade-playbook closure check exists but no general repo-wide CAS verification.
**Effort:** M | **Owner:** infra

### P2-NATIONAL-001: Pakistan National System Adapters Undefined

**Issue:** Data structures exist (NADRA types) but production HTTP adapters / trait contracts not implemented (FBR IRIS, Raast, NADRA, SECP).
**Effort:** XL (8-20 weeks) | **Owner:** protocol + partner + sovereign

---

## 5. WORK PRIORITY QUEUE (Claude Code Execution Order)

> **Status as of 2026-02-18:** Phases A, B, and C are **COMPLETE**. The remaining
> work items are in Phases D, E, and F — all requiring external dependencies,
> cryptographic library integration, or partner coordination.

### Phase A: Integrity Foundation — `COMPLETE ✅`

All items resolved:
- ✅ P0-CANON-001 — MCF normatively defined in spec
- ✅ P0-CORRIDOR-002 — `compute_next_root()` with digest-set normalization
- ✅ P0-CORRIDOR-001 — Dual commitment model (hash-chain + MMR)
- ✅ P0-CORRIDOR-003 — CorridorReceipt matches schema
- ✅ P0-CORRIDOR-004 — Schema-conformant checkpoint type
- ✅ P0-FORK-001 — Evidence-driven fork resolution

### Phase B: Safety Properties — `COMPLETE ✅`

All items resolved:
- ✅ P0-MIGRATION-001 — Saga with explicit side-effects, CAS versioning, idempotent compensation
- ✅ P0-TENSOR-001 — Fail-closed on empty slices and unimplemented domains
- ✅ P0-ZK-001 — Fail-closed production policy for proof backend
- ✅ P0-DEPLOY-001 — Default credentials eliminated

### Phase C: Governance & Schema Hardening — `COMPLETE ✅`

All items resolved:
- ✅ P1-CLI-001 — Evidence computed and stored for corridor transitions
- ✅ P1-SCHEMA-001 — CI validates Draft 2020-12 + $ref closure
- ✅ P1-SCHEMA-002 — Schema URI consistency enforced
- ✅ P1-SCHEMA-003 — additionalProperties policy audited
- ✅ P1-GOV-001 — Deprecated governance v1 quarantined
- ✅ P1-PERF-001 — Schema validators cached with RwLock

### Phase D: API & Integration Surface (NEXT PRIORITY)

```
17. P1-API-001      — Promote OpenAPI from scaffold to contract
18. P1-API-002      — Pin Mass API specs in-repo
19. P1-NAMING-001   — Publish canonical terminology glossary
20. P1-DEPLOY-002   — ✅ RESOLVED (deploy script aligned)
```

### Phase E: Cryptographic Completion (parallel, requires library integration)

```
21. P0-CRYPTO-001   — Implement Poseidon2 (requires vetted library selection)
22. P0-CRYPTO-002   — Implement BBS+ selective disclosure (requires vetted library)
23. P0-ANCHOR-001   — Implement real anchor target (requires L1 interface design)
```

### Phase F: Sovereign Deployment (parallel, requires partner coordination)

```
24. P0-IDENTITY-001    — Ship real Identity service (requires Mass-side implementation)
25. P0-CORRIDOR-NET-001 — Implement inter-zone corridor protocol (protocol design + networking)
26. P0-PACK-001        — Deliver real jurisdiction pack content (requires legal content)
27. P2-NATIONAL-001    — Implement Pakistan national system adapters (NADRA, FBR, SECP, Raast)
```

---

## 6. FORMAL VERIFICATION OBLIGATIONS

The following TLA+ / Alloy models are required to close proof obligations. Generate stubs during Phase A/B work.

### TLA+ Modules Required

| Module | Goal | Priority |
|---|---|---|
| `Canonicalization.tla` | Prove cross-language determinism; verify CB = JCS_RFC8785 (or document deviation) | Phase A |
| `ReceiptChainMMR.tla` | Prove append-only, root linkage, inclusion proof soundness, `prev_root == final_state_root` | Phase A |
| `ForkResolution.tla` | Prove fork resolution cannot be gamed; attestation_count sound; timestamp bound; eventual convergence with >2/3 honest watchers | Phase A |
| `MigrationSaga.tla` | Prove inverse/idempotent compensation, no asset duplication, timeout triggers compensation | Phase B |
| `WatcherBonding.tla` | Prove slashing/accounting invariants; `slashed ≤ bonded`; ban is terminal | Phase B |

### Alloy Models Required

| Model | Goal | Priority |
|---|---|---|
| `SchemaRigidity.als` | Enforce schema-level invariants; no extra fields in proof objects; binding_status from fixed enum | Phase C |

---

## 7. ADVERSARIAL TEST VECTORS REQUIRED

Each must be implemented as test cases in the relevant crate:

| Vector | Crate | Description |
|---|---|---|
| Receipt next_root forgery | msez-corridor | Craft receipt with arbitrary `next_root` that doesn't match payload; must be rejected |
| Fork timestamp backdating | msez-corridor | Craft `ForkBranch` with `ts=epoch, attest=Q+1`; must be rejected |
| Fork attestation inflation | msez-corridor | Craft branch claiming attestation_count exceeding actual signed attestations; must be rejected |
| Checkpoint forgery | msez-corridor | Submit unsigned checkpoint; must be rejected |
| Migration race condition | msez-corridor | Two concurrent `advance()` calls on same migration; CAS must prevent double-advance |
| Compensation replay | msez-corridor | Call `compensate()` twice; second must be no-op (not error) |
| Compliance tensor empty slice | msez-tensor | Submit empty domain set; must return error/Pending, not Compliant |
| Compliance NotApplicable bypass | msez-tensor | Attempt to pass check with all extended domains returning NotApplicable; must fail in production mode |
| Mock ZK proof in production | msez-zkp | Submit mock proof when production policy active; must be rejected |
| API schema downgrade | msez-api | Send request with extra fields to `additionalProperties: false` endpoint; must be rejected |
| Watcher equivocation | msez-corridor | Two conflicting attestations from same watcher for same height; detect and trigger slashing |
| Content-addressed integrity gap | msez-core | Submit artifact where declared digest ≠ actual bytes hash; must be rejected |

---

## 8. CI GATES — CURRENT STATUS

All critical CI gates are now implemented in `.github/workflows/ci.yml`:

| Gate | Status | Job |
|------|--------|-----|
| Schema compilation (Draft 2020-12 + $ref closure) | ✅ Implemented | `schema-validation` |
| Release build mock feature guard | ✅ Implemented | `schema-validation` |
| Schema `$id` URI consistency | ✅ Implemented | `schema-validation` |
| Default credential detection | ✅ Implemented | `schema-validation` |
| `serde_json preserve_order` guard | ✅ Implemented | `rust` |
| Clippy (warnings as errors) | ✅ Implemented | `rust` |
| `cargo fmt --check` | ✅ Implemented | `rust` |
| `cargo audit` | ✅ Implemented | `rust` |
| `additionalProperties` policy audit | ✅ Implemented | `schema-validation` |
| Trade playbook artifact closure | ✅ Implemented | `schema-validation` |

### CI Gates Still Needed

```yaml
# Future additions:

# 1. CAS integrity verification (when msez artifact verify --all is available)
- name: Verify content-addressed artifacts
  run: msez artifact verify --all

# 2. Release-mode proof policy test
- name: Release build rejects mock proofs
  run: cargo test --release --package msez-zkp -- policy::tests::release_build_rejects_mock
```

---

## 9. COVERAGE MATRIX (Spec Chapters → Implementation Status)

Based on reconciled audit findings (2026-02-18). Status: ✅ Implemented | 🟡 Partial | 🔴 Stub/Missing | ⚪ Not Applicable

| # | Spec Area | Status | Blocking P0s | Notes |
|---|---|---|---|---|
| 1-5 | Core primitives / entities | 🟡 | P0-IDENTITY-001 | Identity is facade only |
| 6-10 | Ownership / instruments | 🟡 | — | Mass API alignment unverified |
| 11 | Mass primitives mapping | 🟡 | P1-NAMING-001 | Naming inconsistency |
| 12-15 | Compliance tensor | ✅ | — | Fail-closed on empty slices + unimplemented domains (P0-TENSOR-001 resolved) |
| 16-20 | Pack trilogy | 🔴 | P0-PACK-001 | No real jurisdiction content |
| 21-25 | Corridors | ✅ | — | Receipt chain conforms to spec (P0-CORRIDOR-001..004, P0-CANON-001 resolved) |
| 26-30 | Migration | ✅ | — | Provable compensation with CAS (P0-MIGRATION-001 resolved) |
| 31-35 | Watcher economy | ✅ | — | Evidence-driven fork resolution (P0-FORK-001 resolved) |
| 36-40 | Anchoring / ZK | 🟡 | P0-ANCHOR-001, P0-CRYPTO-001/002 | Policy layer enforces fail-closed (P0-ZK-001 resolved); real backends still stubbed |
| 41-45 | Deployment / infra | ✅ | — | Default creds eliminated, deploy script aligned (P0-DEPLOY-001, P1-DEPLOY-002 resolved) |
| 46-48 | National integration | 🔴 | P2-NATIONAL-001 | Adapters undefined |

---

## 10. DEPLOYMENT PHASE GATES

### Phase 1 — Controlled Sandbox (COMPLETE)

**Entry criteria (ALL MET):**
- [x] Deterministic deploy with real keys (placeholder crypto keys removed)
- [x] Health/readiness gates for Mass connectivity
- [x] Contract test suite for Mass API drift detection
- [x] P0-DEPLOY-001: Default credentials eliminated (env var enforcement + random generation)
- [x] P0-CORRIDOR-001..004: Receipt chain conforms to spec
- [x] P0-CANON-001: Canonicalization spec alignment (MCF)
- [x] P0-FORK-001: Fork resolution evidence-driven
- [x] P0-MIGRATION-001: Saga compensation provable
- [x] P0-TENSOR-001: Compliance tensor fail-closed
- [x] P0-ZK-001: Mock proof rejection policy in place
- [x] P1-SCHEMA-001/002: CI validates Draft 2020-12 + URI consistency
- [x] P1-GOV-001: Deprecated governance quarantined
- [x] P1-PERF-001: Schema validator cached

**Exit criteria:**
- End-to-end demo flows with VCs + audit trails
- Threat model + runbook reviewed with sovereign security

### Phase 2 — Limited Corridor Activation (PARTIALLY UNBLOCKED)

**Resolved blockers:**
- ~~P0-CORRIDOR-001..004 (receipt chain spec conformance)~~ ✅
- ~~P0-FORK-001 (fork resolution)~~ ✅
- ~~P0-TENSOR-001 (compliance fail-closed)~~ ✅

**Remaining blockers:**
- P0-CORRIDOR-NET-001 (inter-zone protocol — no networking layer)
- Real anchor target (currently mock, P0-ANCHOR-001)

### Phase 3 — Production (BLOCKED)

**Blockers:**
- P0-PACK-001 (real jurisdiction content)
- P0-IDENTITY-001 (real identity service)
- P2-NATIONAL-001 (payment/tax/KYC adapters)
- P0-ANCHOR-001 (real L1 anchor target)
- HSM/KMS key custody model
- External security audit completed

### Phase 4 — Cross-Border Expansion

**Requires:**
- Multi-zone infra parameterization
- Corridor registry + trust anchor governance
- P0-CRYPTO-001/002 (Poseidon2 + BBS+)
- P0-CORRIDOR-NET-001 (inter-zone networking)

---

## 11. AUDIT INFRASTRUCTURE

### Commit These Files

```
schemas/audit/institutional-readiness-audit.schema.json   # JSON Schema for audit artifacts
audits/v0.4.44-genesis/institutional-readiness.audit.json  # Machine-readable audit instance
.github/workflows/audit-sync.yml                          # Auto-create GitHub issues from findings
```

### GitHub Issue Synthesis Rules

- **Primary key:** `finding.id`
- **Title format:** `[{severity}][{area}] {id}: {issue}`
- **Labels:** `severity:P0`, `area:Cryptography`, `owner:protocol`, `status:open`, optional: `sovereign_blocker:true`, `formal_model_required:true`
- **File pointer:** `https://github.com/momentum-sez/stack/blob/{commit_sha}/{file}#L{line_start}-L{line_end}`

---

## 12. CLAUDE CODE SESSION STRATEGY

> **Sessions 1-7 are COMPLETE.** The remaining sessions focus on API hardening,
> cryptographic library integration, and sovereign deployment infrastructure.

### Sessions 1-7: Core Integrity, Safety, Schema, CLI, Deploy — `COMPLETE ✅`

All core audit findings (P0-CORRIDOR-001..004, P0-CANON-001, P0-FORK-001,
P0-MIGRATION-001, P0-TENSOR-001, P0-ZK-001, P0-DEPLOY-001, P1-SCHEMA-001..003,
P1-GOV-001, P1-PERF-001, P1-CLI-001, P1-DEPLOY-002) have been resolved.

### Session 8: OpenAPI Promotion (NEXT)
Open and fix: `apis/*.openapi.yaml`
Targets: P1-API-001, P1-API-002
- Add error models, auth models, idempotency semantics, pagination to all 4 OpenAPI specs
- Pin Mass API specs in-repo as versioned snapshots

### Session 9: Cryptographic Library Integration
Open and implement: `msez/crates/msez-crypto/src/poseidon.rs`, `msez/crates/msez-crypto/src/bbs.rs`
Targets: P0-CRYPTO-001, P0-CRYPTO-002
- Select vetted Poseidon2 library (e.g., `plonky3` or `neptune`)
- Select vetted BBS+ library (e.g., `zkryptium` or `bbs`)
- Implement with fixed parameters and test vectors
- Publish canonical message encoding and domain separation

### Session 10: Anchor Target Interface
Open and implement: `msez/crates/msez-crypto/` anchor module
Targets: P0-ANCHOR-001
- Design real `AnchorTarget` trait with finality confirmation delay
- Implement reorg detection and inclusion proof verification
- Feature-gate mock for dev only; integration test with simulated L1

### Session 11: Inter-Zone Corridor Protocol
Targets: P0-CORRIDOR-NET-001
- Design inter-zone discovery / handshake protocol
- Implement receipt exchange + watcher attestation relay
- 2-zone integration test (protocol spec + handshake + receipt exchange)

### Session 12: Sovereign Deployment
Targets: P0-IDENTITY-001, P0-PACK-001, P2-NATIONAL-001
- Requires partner coordination for Mass Identity service
- Requires legal content for Pakistan jurisdiction packs
- Requires NADRA/FBR/SECP/Raast adapter specifications

---

## 13. KEY INVARIANTS REGISTRY

These invariants must be maintained across all changes. Violation of any = P0.

| ID | Invariant | Enforcement | Status |
|---|---|---|---|
| I-CANON | All digests computed via `SHA256(CanonicalBytes)` path | Static (type system) | ✅ Active |
| I-RECEIPT-LINK | `receipt.prev_root == final_state_root` (hash-chain) | Runtime (append check) | ✅ Active |
| I-RECEIPT-COMMIT | `receipt.next_root == SHA256(MCF(payload_without_proof_and_next_root))` | Runtime (recompute + compare) | ✅ Active |
| I-MMR-ROOT | `mmr_root() == MMR(next_roots)` | Runtime | ✅ Active |
| I-CHECKPOINT-PROOF | Checkpoint must include proof (signed) | Runtime + schema | ✅ Active (Option for construction) |
| I-FORK-EVIDENCE | Fork selection inputs must be cryptographically bound | Runtime | ✅ Active |
| I-SAGA-NODUPE | `¬(asset_exists_source ∧ asset_exists_dest)` | Runtime (`no_duplicate_invariant()`) | ✅ Active |
| I-SAGA-IDEMP | `compensate(compensate(s)) == compensate(s)` | Runtime | ✅ Active |
| I-TENSOR-COMPLETE | Production mode: all mandatory domains evaluated, no empty slices | Runtime | ✅ Active |
| I-ZK-REAL | Production mode: reject mock proofs | Compile-time + runtime | ✅ Active |
| I-SERDE-ORDER | `serde_json` must not enable `preserve_order` | CI guard | ✅ Active |
| I-NO-DEFAULT-CREDS | No default passwords in deploy paths | CI guard | ✅ Active |

---

## 14. INSTITUTIONAL POSTURE SUMMARY

**Overall Assessment (2026-02-18):** Phase 1 (Controlled Sandbox) entry criteria are **MET**.
Phase 2 (Limited Corridor Activation) is **partially unblocked** — core corridor integrity
is resolved but inter-zone networking remains. Phase 3 (Production) remains **blocked** on
sovereign infrastructure items.

**Phase 1 is GO.** All core integrity P0s are resolved:
1. ✅ Receipt chain conforms to spec (P0-CORRIDOR-001..004)
2. ✅ Fork resolution is evidence-driven (P0-FORK-001)
3. ✅ Default credentials eliminated (P0-DEPLOY-001)
4. ✅ Compliance tensor fail-closed (P0-TENSOR-001)
5. ✅ Mock proof rejection policy (P0-ZK-001)
6. ✅ Migration saga provable (P0-MIGRATION-001)
7. ✅ Canonicalization spec alignment (P0-CANON-001)

**Do NOT proceed to Phase 3 (Production)** until:
1. Real identity service (P0-IDENTITY-001)
2. Inter-zone corridor protocol (P0-CORRIDOR-NET-001)
3. Real jurisdiction pack content (P0-PACK-001)
4. National system adapters (P2-NATIONAL-001)
5. Real anchor target (P0-ANCHOR-001)
6. HSM/KMS key custody model
7. Independent security review / pen test completed

**Resolved positive signals (expanded from original):**
- Strong type-level invariant strategy (typestate, canonical bytes, sealed proof backends, zeroize keys)
- Mass API connectivity gating added
- Contract tests with OpenAPI snapshots + schema drift detection
- Placeholder crypto keys removed; real Ed25519 JWK via `msez vc keygen`
- CI guard against `serde_json preserve_order` (digest corruption prevention)
- Single-binary docker-compose baseline with observability
- **NEW:** Receipt chain implements dual commitment model (hash-chain + MMR) with golden vector tests
- **NEW:** Fork resolution uses signed watcher attestations with equivocation detection
- **NEW:** Migration saga has explicit side-effects, CAS versioning, idempotent compensation
- **NEW:** Schema validation uses Draft 2020-12 compilation with `$ref` resolution and validator caching
- **NEW:** 10 CI gates active (schema compilation, URI consistency, credential detection, mock feature guard, serde guard, clippy, fmt, audit, additionalProperties policy, trade playbook closure)
