# Prereqs to ship v0.4.40

This document is the **release gate** for the next version bump (`v0.4.39 → v0.4.40`).

**Rule:** do **not** bump the version number until every section below is ✅ complete.

The goal is to make `v0.4.40` the first *operator‑grade* release: reproducible playbooks, strict verification semantics, scenario-driven tests, and repo hygiene strong enough to support real-world deployment and independent implementations.

---

## 0.40 prime directive

Every meaningful commitment in the system MUST have:

1. **deterministic bytes** (canonical JSON, no nondeterminism),
2. a **strict digest** (semantic digest computed over the canonical signing input),
3. a **schema**,
4. and a **verification command + tests**.

If any object can be produced in two byte-inequivalent ways while remaining “semantically the same”, CI will eventually drift.

---

## A. Operator UX + scaffolding + witness bundles (complete, not partial)

### A1. Operator scaffolding commands exist and are documented

- [ ] `mez corridor module init <corridor_id>` creates a new corridor module from a template, without manual copying.
- [ ] `mez asset module init <asset_id>` creates `modules/smart-assets/<asset_id>/…` from template.
- [ ] `mez trade playbook init` (or documented equivalent) scaffolds `docs/examples/trade/src/...` for operators.
- [ ] Each init command:
  - [ ] is deterministic (same inputs → same bytes),
  - [ ] has `--help` with examples,
  - [ ] refuses to overwrite without `--force`,
  - [ ] has tests.

### A2. Witness bundles are first-class and portable

- [ ] `mez artifact graph verify … --bundle <zip>` produces a **portable audit packet** (witness bundle) for:
  - [ ] corridor receipts + checkpoint,
  - [ ] asset receipt chains + checkpoint,
  - [ ] settlement anchors + proof-bindings,
  - [ ] any referenced rulesets/lawpacks/schemas/VCs.
- [ ] Bundle manifest is canonical JSON; bundle digest is stable.
- [ ] Bundle verification is possible offline:
  - [ ] `mez artifact graph verify --from-bundle <zip> --strict …`.
- [ ] Optional provenance attestation works:
  - [ ] `mez artifact bundle attest` produces VC committing to `SHA256(JCS(manifest.json))`.
  - [ ] `mez artifact bundle verify` checks digest + signature.

### A3. Operator safety rails

- [ ] `mez lint` / `mez doctor` exists (or equivalent), and fails fast on:
  - [ ] schema invalidation,
  - [ ] missing CAS artifacts,
  - [ ] non-canonical JSON,
  - [ ] digest mismatches,
  - [ ] registry/lock mismatch,
  - [ ] trust-anchor enforcement gaps.

---

## B. Corridor-of-corridors (netting + multi-party settlement plans)

This section is *not optional*. `v0.4.40` must include corridor composition primitives sufficient to model real-world netted settlement across multiple participants and rails.

### B1. Multi-corridor, multi-currency, constrained netting

- [ ] Netting engine supports:
  - [ ] **multiple corridors** feeding one settlement plan,
  - [ ] **multiple currencies**,
  - [ ] **constraints** (per-party limits, per-rail limits, cutoffs, priority tiers),
  - [ ] deterministic tie-breaking,
  - [ ] traceable optimization output (explainable plan).

- [ ] Netting artifacts are content-addressed and verifiable:
  - [ ] `schemas/settlement-plan.schema.json` (or equivalent),
  - [ ] `schemas/netting-session.schema.json` (if sessions exist),
  - [ ] strict digest semantics,
  - [ ] CLI to validate/verify.

### B2. Settlement plan → settlement anchor → proof binding

- [ ] Settlement plan outputs are anchored into settlement corridors.
- [ ] Cross-corridor settlement anchoring is implemented and enforced:
  - [ ] obligation corridor checkpoint → settlement corridor checkpoint link via settlement anchor.
- [ ] Proof bindings exist for:
  - [ ] sanctions screening evidence,
  - [ ] carrier events evidence,
  - [ ] payment rails evidence.

### B3. Corridor registry binding and ruleset digests are correct

- [ ] Corridor transition registries are bound to corridor instances correctly.
- [ ] “expected ruleset digest” computation is correct and strict.
- [ ] There are tests that fail if registries or rulesets are rewritten.

---

## C. Trade instrument kit (invoice/BOL/LC schemas + transitions)

`v0.4.40` ships a minimal but real-world useful trade vocabulary that composes with the corridor and settlement primitives.

### C1. Schemas are present and in CAS

- [ ] At minimum:
  - [ ] commercial invoice schema,
  - [ ] bill of lading schema,
  - [ ] letter of credit schema,
  - [ ] carrier event schema,
  - [ ] sanctions check evidence schema,
  - [ ] SWIFT pacs.008-like settlement evidence schema.

- [ ] Every schema referenced by registries/rulesets/examples MUST exist in CAS:
  - [ ] `mez artifact graph verify --strict …` passes.

### C2. Transition types + rulesets exist and are pinned

- [ ] Trade transitions exist (even if policy logic is stubbed) with:
  - [ ] schemas,
  - [ ] ruleset descriptors in CAS,
  - [ ] transition registry lock entries.

### C3. End-to-end example artifacts exist

- [ ] A jurisdiction-sharded trade example exists with:
  - [ ] exporter zone module,
  - [ ] importer zone module,
  - [ ] lawpack overlays per zone,
  - [ ] obligation corridor receipts + checkpoint chain for invoice/BOL/LC transitions,
  - [ ] settlement corridor receipts + checkpoint chain for SWIFT pacs.008 settlement,
  - [ ] settlement plan + anchor linking obligation→settlement,
  - [ ] proof bindings referencing sanctions + carrier + payment rails.

---

## D. Determinism + strict semantics (the hard gate)

This is the defining quality of `v0.4.40`.

### D1. Canonical bytes everywhere (JCS)

- [ ] The generator and all tooling that writes JSON MUST use canonical JSON (RFC 8785 / JCS):
  - [ ] stable key ordering,
  - [ ] no trailing whitespace variability,
  - [ ] stable number formatting.

### D2. No nondeterministic fields

- [ ] No wall-clock timestamps in generated outputs.
- [ ] Use `SOURCE_DATE_EPOCH` (or a fixed deterministic epoch) for any required timestamps.
- [ ] Use deterministic UUIDs (`uuid5`) derived from stable labels; never `uuid4` in generated artifacts.
- [ ] Deterministic randomness: if sampling is needed, seed from stable inputs.

### D3. Strict digest semantics are enforced

- [ ] `mez law ingest … --strict/--check` enforces deterministic normalization.
- [ ] Zone locks are strict: deterministic bytes and strict digests.
- [ ] Corridor receipts/checkpoints are strict:
  - [ ] receipt digest = `next_root` (or the explicitly defined strict digest),
  - [ ] checkpoint digest computed over canonical signing input.
- [ ] Rulesets + lawpacks strict equality is enforced in verification and in CI.

### D4. Strict verification can run **without producing new bytes**

- [ ] `tools/dev/generate_trade_playbook.py --mode check`:
  - [ ] performs a full deterministic build in memory,
  - [ ] validates existing files byte-for-byte,
  - [ ] fails on drift,
  - [ ] writes **nothing**.

- [ ] `mez artifact graph verify --strict` detects tampered CAS entries.

### D5. “Bytes vs semantic digests” is explicitly documented

- [ ] Documentation explains:
  - [ ] byte equality gate (strongest),
  - [ ] strict semantic digests (domain correctness),
  - [ ] why we require both.

---

## E. Testing (scenario-driven, bug-finding, regression)

### E1. Playbook scenario tests validate the *entire artifact graph*

- [ ] Tests build/verify the full trade playbook graph end-to-end:
  - [ ] all ArtifactRefs resolve,
  - [ ] digests recompute,
  - [ ] closure is complete,
  - [ ] witness bundle verifies.

### E2. Real-world inspired scenario coverage

Minimum scenarios (examples — expand):

- [ ] exporter/importer mismatch and resolution
- [ ] document discrepancy (invoice vs BOL) halts settlement
- [ ] sanctions hit blocks a leg and requires dispute/arbitration attachment
- [ ] carrier event delay triggers amendment and re-anchoring
- [ ] partial shipment and partial settlement
- [ ] multi-currency netting with constrained participant limits
- [ ] forked receipt chain resolved via fork-resolution credential
- [ ] key rotation and threshold/quorum enforcement
- [ ] tampered CAS entry detected in strict mode
- [ ] missing artifact closure detected

### E3. “Epic bughunt” discipline

- [ ] For v0.4.40, we maintain a `docs/bughunt/` log with:
  - [ ] at least **10** previously-uncaught bugs revealed by new tests,
  - [ ] a regression test for each,
  - [ ] the fix PR/commit reference.

---

## F. Documentation (operator manual grade)

- [ ] README contains a *Playbook Overview* dashboard (cards/tables + flow diagrams).
- [ ] `docs/examples/trade/README.md` reads like an operator runbook.
- [ ] Every major subsystem has:
  - [ ] a design doc,
  - [ ] a verification doc,
  - [ ] CLI usage examples,
  - [ ] schema references.

Required docs:

- [ ] `docs/architecture/SMART-ASSET-INTEGRATION.md` updated to reflect jurisdiction sharding + non-token assets.
- [ ] `docs/roadmap/PRODUCTION_GRADE_SPEC.md` updated with `v0.4.40` execution plan.
- [ ] “Bughunt & cleanliness roadmap” doc exists and is current.

---

## G. Repo cleanliness + execution quality

Non-negotiable hygiene:

- [ ] No dead/placeholder imports (e.g., nonexistent `ArtifactCAS` usage).
- [ ] One canonical artifact store abstraction; no duplicated CAS logic.
- [ ] Clear directory semantics: `modules/`, `profiles/`, `registries/`, `rulesets/`, `schemas/`, `docs/`, `tools/`.
- [ ] Naming conventions enforced.
- [ ] Formatting/linting/type-checking wired.

---

## H. CI gate wiring

CI MUST enforce the hard invariants.

- [ ] CI runs on every PR:
  - [ ] schema validation,
  - [ ] unit tests,
  - [ ] scenario tests,
  - [ ] `generate_trade_playbook.py --mode check` (no writes),
  - [ ] corridor verify strict + canonical bytes,
  - [ ] artifact graph verify strict.

- [ ] CI uses `MEZ_ARTIFACT_STORE_DIRS` to include the playbook CAS under `docs/examples/trade/dist/artifacts`.

---

## Ship decision

A version bump to `v0.4.40` is authorized only when:

- every checklist item above is ✅,
- CI passes cleanly,
- the trade playbook can be verified offline from a witness bundle,
- and the repo remains clean, legible, and reproducible.
