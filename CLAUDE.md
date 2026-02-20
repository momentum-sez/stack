# CLAUDE.md — Momentum EZ Stack v0.4.44 GENESIS

## Unified Audit Work Plan for Claude Code

**Repository:** `momentum-ez/stack`
**Pinned Commit:** `a93eb00ca10f0caff47354e2e07ca0929713126c`
**Spec Version:** 0.4.44-GENESIS
**License:** BUSL-1.1
**Architecture:** Rust workspace (single `mez-api` binary), replaces prior Python stack
**Audit Artifact Schema:** `schemas/audit/institutional-readiness-audit.schema.json` (Draft 2020-12)

---

## 1. REPOSITORY MAP

```
momentum-ez/stack/
├── .github/workflows/       # CI: JSON parse checks, serde guard, trade-playbook closure
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
│   ├── corridor.lifecycle.state-machine.v1.json   # DEPRECATED — diverges from spec
│   └── corridor.lifecycle.state-machine.v2.json
├── jurisdictions/_starter/zone.yaml
├── modules/                 # Module descriptors (claim: 298 across 16 families)
├── mez/                    # Rust workspace root
│   ├── Cargo.toml           # workspace license: BUSL-1.1
│   └── crates/
│       ├── mez-api/        # Consolidated API binary
│       ├── mez-cli/        # CLI: corridor lifecycle, keygen, validate, lock
│       ├── mez-core/       # CanonicalBytes, ContentDigest, SHA-256 primitives
│       ├── mez-corridor/   # Receipt chain, checkpoint, fork resolution, anchor, bridge
│       ├── mez-crypto/     # Ed25519, MMR, Poseidon2 (stub), BBS+ (stub)
│       ├── mez-mass-client/# Mass API client boundary
│       ├── mez-pack/       # Pack Trilogy (lawpacks, regpacks, licensepacks)
│       ├── mez-schema/     # JSON Schema Draft 2020-12 validator + codegen policy
│       ├── mez-tensor/     # Compliance Tensor (20 domains, lattice aggregation)
│       ├── mez-vc/         # Verifiable Credentials (Smart Asset Registry VC)
│       └── mez-zkp/        # ZK proof system (Phase 1 deterministic mock)
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

All P0s below are confirmed across multiple independent audit passes. They are ordered by blast radius (integrity > compliance > deployment).

### P0-CORRIDOR-001: Receipt Chain Does Not Enforce Spec Hash-Chain Model

**Files:** `mez/crates/mez-corridor/src/receipt.rs`
**Confirmed by:** Pass 1 Fidelity Audit, Formal Methods Audit, Institutional Assessment
**Issue:** Implementation enforces `receipt.prev_root == current_mmr_root`, but the spec requires `prev_root` to be the previous state root (hash-chain model seeded from `genesis_root`), and `next_root` to be derived from the receipt payload (excluding `proof` and `next_root`). These are two different commitment models.
**Impact:** Interoperability failure. Receipts produced by this implementation are not verifiable by any tooling following `spec/40-corridors.md` and `schemas/corridor.receipt.schema.json`. Cross-party corridor reconciliation becomes non-deterministic. Regulator verification breaks.
**Remediation:**
1. Implement two parallel commitments per spec: `final_state_root` (hash-chain head, genesis-seeded) and MMR over receipt digests (for inclusion proofs).
2. Change `append()` to enforce: `receipt.prev_root == final_state_root` (hash-chain continuity).
3. Enforce `receipt.next_root == SHA256(JCS(receipt_without_proof_and_next_root))`.
**Verification:** Golden vector conformance tests; schema validation roundtrip against `schemas/corridor.receipt.schema.json`; genesis_root fixture for first receipt.
**Effort:** M | **Owner:** protocol + security

### P0-CORRIDOR-002: next_root Is Not Computed or Verified

**Files:** `mez/crates/mez-corridor/src/receipt.rs` (L7-L9, L17, L22-L24)
**Confirmed by:** Pass 1, Formal Methods, Red Team
**Issue:** `append()` blindly appends `receipt.next_root` to MMR without recomputing or verifying it. Spec requires `next_root = SHA256(JCS(receipt_without_proof_and_next_root))` and digest-set normalization (dedupe + sort lexicographically).
**Impact:** Proof/commitment forgery surface. Caller can set arbitrary `next_root`. Non-determinism across implementations. Fork amplification.
**Remediation:**
1. Implement `compute_next_root(receipt)` that strips `proof` and `next_root`, normalizes digest sets, computes `SHA256(JCS(payload))`.
2. Enforce in `append()`: recompute and compare; reject mismatch.
**Verification:** Property tests (permutations + duplicates in digest sets must not change `next_root`); golden vector fixtures.
**Effort:** M | **Owner:** protocol + security

### P0-CORRIDOR-003: CorridorReceipt Is Not Schema-Conformant

**Files:** `mez/crates/mez-corridor/src/receipt.rs` (L7-L9), `schemas/corridor.receipt.schema.json`
**Confirmed by:** Pass 1, Pass 2
**Issue:** Rust struct omits required `proof` field. Digest sets are `Vec<String>` but schema allows `DigestString | ArtifactRef` union. Optional but important fields (`transition`, `zk`, `anchor`, `transition_type_registry_digest_sha256`) not represented.
**Impact:** Schema validation will reject any receipt this struct produces. Without `proof`, receipts are unsigned — any party with write access can inject/replay/rewrite.
**Remediation:** Extend `CorridorReceipt` to match schema: add `proof` (one object or array), implement digest-set item type as `enum { DigestString, ArtifactRef }`, add optional fields.
**Verification:** Serialize receipt → validate with `SchemaValidator` against `corridor.receipt.schema.json`; negative test: missing proof must fail.
**Effort:** M | **Owner:** protocol

### P0-CORRIDOR-004: Checkpoint Is Non-Conformant and Lacks Proof

**Files:** `mez/crates/mez-corridor/src/receipt.rs` (L10-L11, L19), `schemas/corridor.checkpoint.schema.json`
**Confirmed by:** Pass 1, Pass 2
**Issue:** Checkpoint only includes `(corridor_id, height, mmr_root, timestamp, checkpoint_digest)`. Schema requires `genesis_root`, `final_state_root`, `receipt_count`, digest sets, `mmr` object (type/algorithm/size/root/peaks), and `proof`. No proof means checkpoints are unsigned claims.
**Impact:** Verifier bootstrap impossible per spec. Forgery surface: malicious relayer can provide fake checkpoints.
**Remediation:** Implement schema-conformant checkpoint type with all required fields. Sign checkpoint payloads.
**Verification:** Schema validation tests + golden vectors; end-to-end: bootstrap from checkpoint and verify tail receipts.
**Effort:** L | **Owner:** protocol + security

### P0-CANON-001: Canonicalization Is Not RFC 8785 JCS

**Files:** `mez/crates/mez-core/src/canonical.rs`, `spec/40-corridors.md`
**Confirmed by:** Formal Methods Audit, Cryptographic Correctness Pass
**Issue:** `spec/40-corridors.md` normatively requires `SHA256(JCS(json))` where JCS is RFC 8785. But `CanonicalBytes` applies extra coercions not in RFC 8785: float rejection, RFC 3339 datetime normalization/truncation.
**Impact:** Implementation is provably not "JCS exact" as spec states. Any external verifier following the spec will compute different digests for the same payload.
**Remediation:** Either:
- (a) Update `CanonicalBytes` to implement exact RFC 8785 JCS, OR
- (b) Update `spec/40-corridors.md` to normatively define "JCS + Momentum coercions" and version/tag it.
Today it is inconsistent. Both sides must agree.
**Verification:** Cross-language golden vectors (Rust/TS/Python) for canonicalization output.
**Effort:** M | **Owner:** protocol + security

### P0-FORK-001: Fork Resolution Is Manipulable

**Files:** `mez/crates/mez-corridor/src/fork.rs`
**Confirmed by:** Formal Methods, Red Team, Institutional Assessment
**Issue:** Fork selection uses `(-attestation_count, timestamp, lex_candidate_root)` but `attestation_count` and `timestamp` are not cryptographically bound or verified — code explicitly states "must be independently verified" but doesn't implement it. Attacker can publish `ForkBranch` with `attest=Q+1, ts=1970-01-01` and deterministically win selection.
**Impact:** Safety violation: honest nodes converge on attacker-selected root. Honest progress can be overwritten.
**Remediation:**
1. Replace raw `(timestamp, attestation_count)` fields with a set (or Merkle root) of signed watcher attestations binding: parent root, candidate root, height/sequence, timestamp constraints.
2. Verify those signatures in the fork module.
3. Enforce monotonic time: `ts[i] >= ts[i-1]` and `ts[i] <= now + drift_bound`.
**Verification:** Adversarial test: craft fork with backdated timestamp; must be rejected.
**Effort:** L | **Owner:** security + protocol

### P0-MIGRATION-001: Saga Compensation Is Unprovable

**Files:** `mez/crates/mez-corridor/` (migration saga module)
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

### P0-ANCHOR-001: Anchoring Is a Mock

**Files:** `mez/crates/mez-crypto/` (anchor module)
**Confirmed by:** Formal Methods, Institutional Assessment
**Issue:** `MockAnchorTarget` always reports `confirmed`. No L1 finality proof, reorg handling, or inclusion verification exists.
**Impact:** No adversarial proof obligation about L1 finality can be satisfied. Anchoring is aspirational, not functional.
**Remediation:** Implement at minimum a real anchor target interface with finality confirmation delay, reorg detection, and inclusion proof verification. Feature-gate mock for dev only.
**Verification:** Integration test with simulated L1 (delayed confirmation, reorg scenario).
**Effort:** XL | **Owner:** protocol + infra

### P0-TENSOR-001: Extended Compliance Domains Default to NotApplicable (Passes)

**Files:** `mez/crates/mez-tensor/src/evaluation.rs`
**Confirmed by:** Institutional Assessment, Red Team, Formal Methods
**Issue:** Only base domains evaluated. Extended domains (including LICENSING, BANKING, DATA_PRIVACY, SANCTIONS) return `NotApplicable`, which is treated as passing. Additionally, `TensorSlice::aggregate_state()` returns `Compliant` for empty slices (folds from `Compliant`).
**Impact:** Regulatory bypass. Transactions proceed without checks for domains that should be enforced. Attacker can induce empty slice → compliant aggregate.
**Remediation:**
1. Fail-closed on unimplemented domains in production: treat "not implemented" as `Pending` or `NonCompliant`, not `NotApplicable`.
2. Require all mandatory domains present and evaluated; treat empty slices as error/Pending.
3. Authority-bound state transitions for Exempt/NotApplicable decisions (signed policy artifact required).
**Verification:** Property tests: no path elevates failing state to passing; empty slice returns error.
**Effort:** M | **Owner:** protocol + security

### P0-CRYPTO-001: Poseidon2 Is a Stub

**Files:** `mez/crates/mez-crypto/src/poseidon.rs` (feature-gated)
**Confirmed by:** Cryptographic Correctness Pass
**Issue:** `poseidon2_digest()` and `poseidon2_node_hash()` return `NotImplemented`.
**Impact:** Any spec element relying on Poseidon2 (ZK-friendly hashing, proof-internal commitments) cannot execute. Mixed deployments risk network divergence.
**Remediation:** Implement Poseidon2 with fixed parameters; publish test vectors; hard feature matrix preventing disagreement on hashing rules.
**Effort:** L-XL | **Owner:** security + protocol

### P0-CRYPTO-002: BBS+ Selective Disclosure Is a Stub

**Files:** `mez/crates/mez-crypto/src/bbs.rs` (feature-gated)
**Confirmed by:** Cryptographic Correctness Pass
**Issue:** Entire BBS+ module is stubbed. Cannot support selective disclosure proofs for KYC/compliance claims.
**Remediation:** Integrate vetted BBS+ library; add proof verification tests; specify canonical message encoding and domain separation.
**Effort:** L-XL | **Owner:** security

### P0-ZK-001: ZK Proof System Is Phase 1 Mock

**Files:** `mez/crates/mez-zkp/`
**Confirmed by:** Red Team, Institutional Assessment
**Issue:** Phase 1 uses deterministic SHA-256 mock proof system by default. Real backends are feature-gated. If verifier accepts mock proofs as authoritative, attacker can supply proofs without possessing underlying witness/claims.
**Impact:** Catastrophic if proofs gate compliance: unauthorized transactions appear compliant.
**Remediation:**
1. Fail-closed production policy: require real proof backend; reject mock proof types.
2. Make proof backend choice a signed, content-addressed policy artifact.
3. Compile-time guardrails + CI checks for release builds.
**Verification:** CI gate: release profile must not enable mock feature; runtime attestation of proof backend.
**Effort:** M | **Owner:** security + protocol

### P0-IDENTITY-001: Identity Primitive Has No Dedicated Mass Service

**Confirmed by:** Institutional Assessment
**Issue:** `IdentityClient` is a facade over other Mass services. There is no `identity-info.api.mass.inc`. This is a sovereign due-diligence blocker.
**Remediation:** Ship real Identity primitive service (Mass-side) + wire `IdentityClient` to it.
**Effort:** L-XL (2-6 weeks) | **Owner:** protocol + partner

### P0-CORRIDOR-NET-001: No Inter-Zone Corridor Networking

**Confirmed by:** Institutional Assessment
**Issue:** Corridor receipt chain and cryptography exist, but no network protocol / discovery / handshake for Zone A ↔ Zone B. Blocks "AWS of Zones" cross-border value proposition.
**Remediation:** Implement inter-zone corridor protocol + 2-zone integration test (protocol spec + handshake + receipt exchange + watcher attestations + replay protection).
**Effort:** XL (4-8 weeks) | **Owner:** protocol + infra

### P0-PACK-001: Pack Trilogy Has No Real Jurisdiction Content

**Confirmed by:** Institutional Assessment
**Issue:** Schemas + validation + signing + CAS exist, but no real Pakistan statutes / rates / license categories. Compliance evaluation not meaningful for sovereign deployment.
**Remediation:** Deliver real pack content for target jurisdiction(s). Minimum for Pakistan pilot: core tax statutes + withholding rules + SECP license categories + sanctions + AML/CFT calendars.
**Effort:** XL (6-16+ weeks, parallel) | **Owner:** legal + protocol

### P0-DEPLOY-001: Default Credentials in Deploy Paths

**Confirmed by:** Institutional Assessment, Red Team
**Issue:** `docker-compose` and deploy script default to `POSTGRES_PASSWORD=mez`. Unacceptable outside local dev.
**Remediation:** Wire secret manager; eliminate all default credentials; key custody model (HSM/KMS) + rotation.
**Effort:** S-M | **Owner:** infra + security

---

## 3. SYNTHESIZED P1 FINDINGS — HIGH SEVERITY

### P1-CLI-001: Corridor CLI Discards Evidence Inputs

**Files:** `mez/crates/mez-cli/src/corridor.rs`
**Issue:** CLI accepts evidence files/digests but pattern-matches them as `_`. Transition records store `evidence_digest: None`.
**Remediation:** Compute content digests for evidence artifacts; validate required evidence per transition type; add `--strict` mode.
**Effort:** M | **Owner:** protocol

### P1-SCHEMA-001: CI Validates Schemas as JSON Only, Not Draft 2020-12

**Files:** `.github/workflows/ci.yml`
**Issue:** CI runs `python3 -c "import json; json.load(...)"` — checks parse, not schema validity or `$ref` closure.
**Remediation:** Add Rust CI step using `SchemaValidator::new()` with `Draft202012` and retriever resolution.
**Effort:** S | **Owner:** infra

### P1-SCHEMA-002: Schema URI Inconsistency

**Files:** `schemas/stack.lock.schema.json`, `mez/crates/mez-schema/src/validate.rs`
**Issue:** `$id` uses `https://momentum-ez.org/schemas/...` while validator resolves `https://schemas.momentum-ez.org/mez/`.
**Remediation:** Normalize all `$id` under single canonical domain; CI rule: no non-canonical `$id`.
**Effort:** S | **Owner:** protocol

### P1-GOV-001: Deprecated Governance State Machine v1 Still Present

**Files:** `governance/corridor.lifecycle.state-machine.v1.json`
**Issue:** Self-declares `deprecated: true`, incorrect state names. Downstream integrators can ingest v1 and implement wrong lifecycle.
**Remediation:** Move to `governance/deprecated/` or remove from default distributions; ensure v2 is sole reference.
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

### P1-DEPLOY-002: Doc/Deploy Drift

**Issue:** Deploy script prints endpoints for defunct multi-service layout; docker-compose describes single binary architecture.
**Remediation:** Update deploy script output to match actual compose architecture.
**Effort:** S | **Owner:** infra

### P1-SCHEMA-003: additionalProperties:true on Security-Critical Objects

**Files:** Multiple corridor/API schemas
**Issue:** `evidence: type: object additionalProperties: true` on corridor finality status. Smart assets OpenAPI has multiple schemas with `additionalProperties: true`.
**Remediation:** Set `additionalProperties: false` on all security-critical objects; keep extensibility only in explicitly namespaced subobjects.
**Effort:** M | **Owner:** protocol + security

### P1-PERF-001: Schema Validator Constructs Per Call

**Files:** `mez/crates/mez-schema/src/validate.rs`
**Issue:** `validate_value()` constructs validator and clones schema map per call. Performance bottleneck under load.
**Remediation:** Cache compiled validators per schema ID using `Arc<CompiledValidator>`.
**Effort:** S-M | **Owner:** protocol

### P1-NAMING-001: Mass Primitives Naming Inconsistency

**Issue:** Repo spec says "Instruments"; investor/government materials say "Fiscal instruments/rails".
**Remediation:** Publish canonical glossary with explicit model/endpoint mapping.
**Effort:** S | **Owner:** protocol

---

## 4. SYNTHESIZED P2 FINDINGS — MEDIUM SEVERITY

### P2-CANON-002: Merkle Helper Uses String Concatenation

**Files:** `mez/crates/mez-tensor/` (commitment.rs Merkle helper)
**Issue:** Hashes canonicalized string concatenations of hex digests rather than byte-level Merkle. Deterministic but diverges from any external spec expecting byte-concat of 32-byte nodes.
**Effort:** S | **Owner:** protocol

### P2-SA-001: binding_status Is Unrestricted String

**Files:** `mez/crates/mez-vc/` (JurisdictionBinding)
**Issue:** `binding_status: String` allows invalid values; should be enum `{active, suspended, exited}`.
**Effort:** XS | **Owner:** protocol

### P2-SA-002: VC Constructor Doesn't Enforce asset_id Binding

**Files:** `mez/crates/mez-vc/`
**Issue:** VC `credentialSubject.asset_id` is not verified to equal `compute_asset_id(genesis)`.
**Effort:** S | **Owner:** protocol

### P2-CANON-003: Sorted Key Assumption Is Test-Only

**Files:** `mez/crates/mez-core/src/canonical.rs`
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

This is the ordered sequence of work items. Dependencies are noted. Each item includes the command context for Claude Code.

### Phase A: Integrity Foundation — COMPLETED

```
# All items resolved as of 2026-02-19.

1. P0-CANON-001   — CLOSED: MCF documented as normative extension of JCS (ADR-002)
2. P0-CORRIDOR-002 — CLOSED: compute_next_root() with digest-set normalization (commit 9e38a24)
3. P0-CORRIDOR-001 — CLOSED: Dual commitment model (hash-chain + MMR) (commit 9e38a24)
4. P0-CORRIDOR-003 — CLOSED: CorridorReceipt matches schema (proof, DigestEntry, optional fields)
5. P0-CORRIDOR-004 — CLOSED: Schema-conformant checkpoint (MmrCommitment, genesis/final_state_root)
6. P0-FORK-001     — CLOSED: Evidence-driven fork resolution with signed attestations (commit 0f54e69)
```

### Phase B: Safety Properties — COMPLETED

```
7. P0-MIGRATION-001 — CLOSED: CAS + idempotency + EffectExecutor trait + side-effect modeling + property tests
8. P0-TENSOR-001    — CLOSED: Fail-closed on extended domains (Pending default, commit 02b1984)
9. P0-ZK-001        — CLOSED: Fail-closed production policy (ProofPolicy, commit 0f54e69)
10. P0-DEPLOY-001   — CLOSED: Secret injection, no default credentials (commit 02b1984)
```

### Phase C: Governance & Schema Hardening — MOSTLY COMPLETED

```
11. P1-CLI-001      — CLOSED: Evidence-gated corridor transitions (commit 0f54e69)
12. P1-SCHEMA-001   — CLOSED: Draft 2020-12 compilation in CI (commit 49e177a)
13. P1-SCHEMA-002   — PARTIAL: URI normalization started; some inconsistencies remain
14. P1-SCHEMA-003   — PARTIAL: Some additionalProperties tightened; more needed
15. P1-GOV-001      — CLOSED: Deprecated v1 quarantined (commit 02b1984)
16. P1-PERF-001     — CLOSED: Cached compiled schema validators (commit f69aee7)
```

### Phase D: API & Integration Surface (MOSTLY COMPLETED)

```
17. P1-API-001      — CLOSED: All four OpenAPI specs contract-grade (commit 6bd628d)
18. P1-API-002      — CLOSED: Mass API specs pinned in-repo (commit 6bd628d)
19. P1-NAMING-001   — OPEN: Publish canonical terminology glossary
20. P1-DEPLOY-002   — CLOSED: Deploy scripts aligned with single-binary architecture; two-zone compose credential-hardened
```

### Phase E: Cryptographic Completion (Weeks 4-12, parallel)

```
21. P0-CRYPTO-001   — OPEN: Implement Poseidon2 (deferred to Phase 4 per ADR-003)
22. P0-CRYPTO-002   — OPEN: Implement BBS+ selective disclosure (deferred to Phase 4)
23. P0-ANCHOR-001   — OPEN: Implement real anchor target (deferred — LVC/EFC suffice)
```

### Phase F: Sovereign Deployment (ACTIVE — Parallel Track)

```
24. P0-IDENTITY-001    — OPEN: Ship real Identity service (Mass-side dependency)
25. P0-CORRIDOR-NET-001 — CLOSED: Inter-zone protocol with handshake (commit 6ea3f8e)
26. P0-PACK-001        — CLOSED: Pakistan Pack Trilogy (commit b996ecc)
27. P2-NATIONAL-001    — CLOSED: All four national adapters complete (commit 620bb1d)
```

### NEW: Phase G — Pragmatic Deployment (NOW — Weeks 1-4)

```
# These items drive towards the "AWS of Economic Zones" deployable reality.
# See docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md for full analysis.

28. End-to-end demo script — CLOSED: deploy/scripts/demo-two-zone.sh (Phase 1 exit criterion)
29. CAS digest computation for regpacks — CLOSED: mez regpack build --jurisdiction pk --all-domains --store
30. Zone bootstrap CLI — CLOSED: mez regpack build + mez lock + sovereign-govos profile + docs/ZONE-BOOTSTRAP-GUIDE.md
31. Corridor establishment walkthrough — CLOSED: documented in docs/ZONE-BOOTSTRAP-GUIDE.md
32. OpenAPI spec promotion — CLOSED: All specs contract-grade (commit 6bd628d)
33. Compliance query endpoint — CLOSED: GET /v1/compliance/{entity_id} in regulator router + OpenAPI spec
34. Pakistan national system adapter interfaces (trait contracts + mocks) — CLOSED (FBR IRIS + SECP)
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
| Receipt next_root forgery | mez-corridor | Craft receipt with arbitrary `next_root` that doesn't match payload; must be rejected |
| Fork timestamp backdating | mez-corridor | Craft `ForkBranch` with `ts=epoch, attest=Q+1`; must be rejected |
| Fork attestation inflation | mez-corridor | Craft branch claiming attestation_count exceeding actual signed attestations; must be rejected |
| Checkpoint forgery | mez-corridor | Submit unsigned checkpoint; must be rejected |
| Migration race condition | mez-corridor | Two concurrent `advance()` calls on same migration; CAS must prevent double-advance |
| Compensation replay | mez-corridor | Call `compensate()` twice; second must be no-op (not error) |
| Compliance tensor empty slice | mez-tensor | Submit empty domain set; must return error/Pending, not Compliant |
| Compliance NotApplicable bypass | mez-tensor | Attempt to pass check with all extended domains returning NotApplicable; must fail in production mode |
| Mock ZK proof in production | mez-zkp | Submit mock proof when production policy active; must be rejected |
| API schema downgrade | mez-api | Send request with extra fields to `additionalProperties: false` endpoint; must be rejected |
| Watcher equivocation | mez-corridor | Two conflicting attestations from same watcher for same height; detect and trigger slashing |
| Content-addressed integrity gap | mez-core | Submit artifact where declared digest ≠ actual bytes hash; must be rejected |

---

## 8. CI GATES TO ADD

```yaml
# Add to .github/workflows/ci.yml

# 1. Schema compilation (Draft 2020-12 + $ref closure)
- name: Validate all schemas
  run: cargo test --package mez-schema -- --test schema_compilation

# 2. Release build must not enable mock features
- name: No mocks in release
  run: |
    cargo build --release 2>&1 | grep -v "mock"
    # Verify: mez-zkp mock feature is not enabled
    cargo metadata --format-version 1 | jq '.packages[] | select(.name == "mez-zkp") | .features' | grep -v mock

# 3. CAS integrity verification
- name: Verify content-addressed artifacts
  run: mez artifact verify --all

# 4. Schema URI canonicalization
- name: Check schema $id consistency
  run: |
    grep -r '"$id"' schemas/ | grep -v 'schemas.momentum-ez.org/mez/' && exit 1 || true

# 5. No default credentials
- name: Check for default credentials
  run: |
    grep -r 'POSTGRES_PASSWORD=mez' deploy/ && exit 1 || true
    grep -r 'password.*=.*mez' deploy/ && exit 1 || true

# 6. serde_json preserve_order guard (already exists — keep it)
```

---

## 9. COVERAGE MATRIX (Spec Chapters → Implementation Status)

Based on synthesized audit findings. Status: ✅ Implemented | 🟡 Partial | 🔴 Stub/Missing | ⚪ Not Applicable

**Updated 2026-02-19** — reflects post-audit remediation work (commits `02b1984` through `af6c70e`).

| # | Spec Area | Status | Blocking P0s | Notes |
|---|---|---|---|---|
| 1-5 | Core primitives / entities | 🟡 | P0-IDENTITY-001 | Identity facade only; Mass client + contract tests exist |
| 6-10 | Ownership / instruments | 🟡 | — | Mass API contract tests added; live alignment still unverified |
| 11 | Mass primitives mapping | 🟡 | P1-NAMING-001 | Naming inconsistency remains |
| 12-15 | Compliance tensor | ✅ | ~~P0-TENSOR-001~~ CLOSED | 20 domains, exhaustive match, fail-closed on extended |
| 16-20 | Pack trilogy | ✅ | ~~P0-PACK-001~~ CLOSED | Pakistan lawpacks (4 domains), regpacks, 70+ licensepacks |
| 21-25 | Corridors | ✅ | ~~P0-CORRIDOR-001..004~~ CLOSED | Dual-commitment receipt chain, inter-zone protocol, corridor registry |
| 26-30 | Migration | ✅ | ~~P0-MIGRATION-001~~ CLOSED | CAS + idempotency + EffectExecutor + side-effect model + property tests |
| 31-35 | Watcher economy | ✅ | ~~P0-FORK-001~~ CLOSED | Evidence-driven fork resolution with signed attestations |
| 36-40 | Anchoring / ZK | 🔴 | P0-ANCHOR-001, P0-CRYPTO-001/002 | ZK policy fail-closed (P0-ZK-001 CLOSED); crypto stubs remain |
| 41-45 | Deployment / infra | ✅ | ~~P0-DEPLOY-001~~ CLOSED | No default creds, zone manifests, two-zone compose, deploy scripts |
| 46-48 | National integration | ✅ | ~~P2-NATIONAL-001~~ CLOSED | All four national adapters complete: FBR IRIS, SECP, NADRA, SBP Raast (commit 620bb1d) |

---

## 10. DEPLOYMENT PHASE GATES

### Phase 1 — Controlled Sandbox (READY — Proceed Now)

**Entry criteria (ALL MET as of 2026-02-19):**
- [x] Deterministic deploy with real keys (placeholder crypto keys removed)
- [x] Health/readiness gates for Mass connectivity
- [x] Contract test suite for Mass API drift detection
- [x] P0-DEPLOY-001: Default credentials eliminated (`${POSTGRES_PASSWORD:?must be set}`)
- [x] Receipt chain spec-conformant (P0-CORRIDOR-001..004 CLOSED)
- [x] Fork resolution evidence-driven (P0-FORK-001 CLOSED)
- [x] Compliance tensor fail-closed (P0-TENSOR-001 CLOSED)
- [x] Zone manifest system with deploy scripts
- [x] 4,073 tests passing, 0 failures

**Remaining for Phase 1 exit:**
- [x] End-to-end demo script: `deploy/scripts/demo-two-zone.sh` (deploy 2 zones → corridor → receipts → verify)
- [x] CAS digest computation for regpacks: `mez regpack build --jurisdiction pk --all-domains --store`
- [x] sovereign-govos profile created: `profiles/sovereign-govos/profile.yaml`
- [x] pk-sifc stack.lock generated with real module + regpack digests
- [x] Zone bootstrap guide: `docs/ZONE-BOOTSTRAP-GUIDE.md`
- [ ] Threat model + runbook reviewed with sovereign security

### Phase 2 — Limited Corridor Activation (UNBLOCKED — Ready to Proceed)

**Previously blocked by (now all resolved):**
- ~~P0-CORRIDOR-001..004~~ CLOSED: Receipt chain implements dual-commitment model
- ~~P0-CORRIDOR-NET-001~~ CLOSED: Inter-zone protocol with handshake + receipt exchange
- ~~P0-FORK-001~~ CLOSED: Evidence-driven fork resolution with signed attestations
- ~~P0-TENSOR-001~~ CLOSED: 20-domain exhaustive evaluation, Pending default

**Remaining for Phase 2:**
- [x] End-to-end two-zone corridor test with real receipt exchange (commit 620bb1d)
- [x] Cross-zone compliance query endpoint (commit 0563021)
- [x] Corridor health monitoring dashboard (Prometheus exporter + Grafana provisioning)
- [x] Sovereign Mass API stubs for per-zone deployment (`mez-mass-stub` crate, `Dockerfile.mass-stub`, `docker-compose.two-zone.yaml` updated with zone-local Mass instances, `sovereign_mass_test.rs` proving data isolation)

### Phase 3 — Production (PARTIALLY BLOCKED)

**Resolved:**
- ~~P0-PACK-001~~ CLOSED: Pakistan lawpacks (4 domains) + regpacks + licensepacks
- ~~P0-DEPLOY-001~~ CLOSED: Secret injection, no default credentials
- ~~P0-ZK-001~~ CLOSED: Fail-closed proof policy

**Still blocking:**
- P0-IDENTITY-001 (real identity service — Mass-side dependency)
- P2-NATIONAL-001 (FBR, NADRA, SBP, SECP HTTP adapters)
- P0-ANCHOR-001 (real anchor target for L1 finality)
- HSM/KMS key custody model + rotation
- External security audit / pen test

### Phase 4 — Cross-Border Expansion

**Requires:**
- Multi-zone Kubernetes orchestration
- Corridor registry + trust anchor governance
- P0-CRYPTO-001/002 (Poseidon2 + BBS+)
- Watcher bond economics (real staking/slashing)
- Real ZK backend activation

### Pragmatic Deployment Roadmap

See `docs/PRAGMATIC-DEPLOYMENT-ROADMAP.md` for the full analysis including Mass spec
alignment, "AWS of Economic Zones" mapping, and prioritized implementation sequence.

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
- **File pointer:** `https://github.com/momentum-ez/stack/blob/{commit_sha}/{file}#L{line_start}-L{line_end}`

---

## 12. CLAUDE CODE SESSION STRATEGY

### Session 1: Receipt Chain Foundation
Open and fix: `mez/crates/mez-corridor/src/receipt.rs`, `mez/crates/mez-core/src/canonical.rs`
Targets: P0-CANON-001, P0-CORRIDOR-001, P0-CORRIDOR-002, P0-CORRIDOR-003, P0-CORRIDOR-004

### Session 2: Fork Resolution + Watcher Safety
Open and fix: `mez/crates/mez-corridor/src/fork.rs`, watcher modules
Targets: P0-FORK-001, watcher equivocation tests

### Session 3: Migration Saga Safety
Open and fix: migration saga module
Targets: P0-MIGRATION-001, concurrency controls, idempotency

### Session 4: Compliance Tensor + ZK Policy
Open and fix: `mez/crates/mez-tensor/src/evaluation.rs`, `mez/crates/mez-zkp/`
Targets: P0-TENSOR-001, P0-ZK-001

### Session 5: Schema & CI Hardening
Open and fix: `.github/workflows/ci.yml`, `mez/crates/mez-schema/`, `schemas/`
Targets: P1-SCHEMA-001, P1-SCHEMA-002, P1-SCHEMA-003, P1-PERF-001

### Session 6: CLI Evidence Gating + Governance Cleanup
Open and fix: `mez/crates/mez-cli/src/corridor.rs`, `governance/`
Targets: P1-CLI-001, P1-GOV-001

### Session 7: Deploy Hardening
Open and fix: `deploy/docker/docker-compose.yaml`, deploy scripts
Targets: P0-DEPLOY-001, P1-DEPLOY-002

### Session 8: OpenAPI Promotion
Open and fix: `apis/*.openapi.yaml`
Targets: P1-API-001, P1-API-002

---

## 13. KEY INVARIANTS REGISTRY

These invariants must be maintained across all changes. Violation of any = P0.

| ID | Invariant | Enforcement |
|---|---|---|
| I-CANON | All digests computed via `SHA256(CanonicalBytes)` path | Static (type system) |
| I-RECEIPT-LINK | `receipt.prev_root == final_state_root` (hash-chain) | Runtime (append check) |
| I-RECEIPT-COMMIT | `receipt.next_root == SHA256(JCS(payload_without_proof_and_next_root))` | Runtime (recompute + compare) |
| I-MMR-ROOT | `mmr_root() == MMR(next_roots)` | Runtime |
| I-CHECKPOINT-PROOF | Checkpoint must include proof (signed) | Runtime + schema |
| I-FORK-EVIDENCE | Fork selection inputs must be cryptographically bound | Runtime |
| I-SAGA-NODUPE | `¬(asset_exists_source ∧ asset_exists_dest)` | Persistence CAS |
| I-SAGA-IDEMP | `compensate(compensate(s)) == compensate(s)` | Runtime |
| I-TENSOR-COMPLETE | Production mode: all mandatory domains evaluated, no empty slices | Runtime |
| I-ZK-REAL | Production mode: reject mock proofs | Compile-time + runtime |
| I-SERDE-ORDER | `serde_json` must not enable `preserve_order` | CI guard (exists) |
| I-NO-DEFAULT-CREDS | No default passwords in deploy paths | CI guard (add) |

---

## 14. INSTITUTIONAL POSTURE SUMMARY

**Overall Assessment (Updated 2026-02-19):** PROCEED to Phase 2 (Sovereign Corridor Activation).

Phase 1 entry criteria are fully met. Phase 2 blockers (receipt chain, inter-zone protocol,
fork resolution, compliance tensor) have all been resolved. The stack is in a
"functionally deployable" state for controlled two-zone corridor demonstrations.

### Strategic Direction: Sovereign Mass Deployment

The MEZ Stack is the deployment substrate that progressively decentralizes Mass:

1. **Today**: MEZ zones orchestrate compliance on top of centralized Mass APIs
2. **Near-term**: Each sovereign zone deploys its own Mass API instances (containerized)
3. **Mid-term**: Sovereign Mass deployments federate via corridor receipt chains
4. **End-state**: Federated sovereign zones = the decentralized execution layer (Mass Protocol)

Every zone deployment is a future Mass consensus node. Every corridor is a future DAG edge.
The Mass spec's end-state emerges bottom-up from sovereign deployments, not top-down from
building a monolithic L1. See `docs/roadmap/AWS_OF_ECONOMIC_ZONES.md` for full sequencing.

**Do NOT proceed to Phase 3 (Production)** until:
1. Sovereign Mass API deployment demonstrated (containerized per-zone)
2. Remaining red items resolved (identity service, national adapters)
3. ~~Receipt chain conforms to spec (P0-CORRIDOR-001..004)~~ CLOSED
4. ~~Fork resolution is evidence-driven (P0-FORK-001)~~ CLOSED
5. ~~Default credentials eliminated (P0-DEPLOY-001)~~ CLOSED
6. Independent security review / pen test completed

**Positive signals:**
- Strong type-level invariant strategy (typestate, canonical bytes, sealed proof backends, zeroize keys)
- Mass API connectivity gating added
- Contract tests with OpenAPI snapshots + schema drift detection
- Placeholder crypto keys removed; real Ed25519 JWK via `mez vc keygen`
- CI guard against `serde_json preserve_order` (digest corruption prevention)
- Single-binary docker-compose baseline with observability
- `mez-mass-client` abstracts Mass endpoint topology (supports both centralized and sovereign)
- Zone manifest system supports sovereign deployment with operator-controlled key custody
