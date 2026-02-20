# Generalize the AWS of Economic Zones: Pakistan → N Jurisdictions

You are extending the MEZ Stack from a Pakistan-first prototype to a jurisdiction-agnostic deployment substrate. Pakistan (pk-sifc) is the only jurisdiction with the complete vertical: real AKN lawpack content, regpack builders with WHT rates/deadlines/sanctions, structured licensepack requirements, national adapter traits, a sovereign-govos profile, and a pinned stack.lock. Every other jurisdiction (100+ zone.yaml manifests, 14 licensepack modules) has scaffold-grade content: correct regulator/license-type taxonomy but empty `requirements: BTreeMap::new()`, placeholder 38-line AKN XML, zero regpack builders, and no adapter traits.

Your task: systematically generalize the Pakistan-specific implementation patterns into jurisdiction-agnostic infrastructure, then instantiate that infrastructure for the highest-value jurisdictions to make the "AWS of Economic Zones" pattern real.

Read CLAUDE.md for full context. Read `docs/roadmap/AWS_OF_ECONOMIC_ZONES.md` for strategic framing.

## Architecture of the change

### Layer 1 — Regpack builder generalization

**Current state:** `mez/crates/mez-pack/src/regpack.rs` contains a `pub mod pakistan` (lines 1541–2916) with hard-coded WHT rates, regulator profiles, sanctions entries, compliance deadlines, and reporting requirements. The CLI (`mez/crates/mez-cli/src/regpack.rs`) has a hard-coded `match jurisdiction { "pk" => ... }` in `run_build()` (line 78) and `build_regpack_for()` (line 115).

**Target:**

1. Extract `pub mod pakistan` from `regpack.rs` into `mez/crates/mez-pack/src/regpack/pakistan.rs`. Convert `regpack.rs` into `regpack/mod.rs` preserving all existing generic types (`Regpack`, `RegPackMetadata`, `SanctionsSnapshot`, `ComplianceDeadline`, `ReportingRequirement`, `RegulatorProfile`, `compute_regpack_digest`).

2. Create `regpack/uae.rs` and `regpack/singapore.rs` following the Pakistan pattern exactly: jurisdiction-specific regulator profiles, compliance deadlines, reporting requirements, and `build_<jid>_regpack()` / `build_<jid>_sanctions_regpack()` factory functions. Use real regulatory data:
   - **UAE**: CBUAE (Central Bank), SCA (Securities & Commodities Authority), ADGM FSRA, DFSA. VAT at 5% (Federal Decree-Law No. 8/2017). ESR reporting (Cabinet Resolution No. 57/2020). AML: Federal Decree-Law No. 20/2018.
   - **Singapore**: MAS (Monetary Authority), ACRA, IRAS. GST 9% (GST Act). AML: MAS Notice 626. Payment Services Act 2019 licensing.

3. Wire new jurisdictions into the CLI: extend the `match jurisdiction` in `run_build()` to include `"ae"` and `"sg"`, add corresponding `build_regpack_for()` arms, add imports.

4. Add determinism tests for each new builder (build twice → same CAS digest), mirroring Pakistan's existing tests.

### Layer 2 — Licensepack requirements enrichment

**Current state:** `mez/crates/mez-pack/src/licensepack/uae.rs` has 16+ regulators and ~65 license types but every single one has `requirements: BTreeMap::new()`. Same for `singapore.rs` (3 regulators, ~35 types). Pakistan is the only file with populated requirements (capital thresholds, legislation cross-references as structured keys).

**Target:**

Populate `requirements` for the financial-services license types in UAE and Singapore using the Pakistan pattern as template:

- UAE CBUAE commercial bank: minimum capital AED 2B (Article 68, Decretal Federal Law No. 14/2018). ADGM CML holder: minimum base capital USD 10M (FSMR 2015). DFSA Category 3A/4: base capital requirements per PIB module.
- Singapore MAS major payment institution: base capital SGD 250K (PS Act s6). Digital payment token: SGD 250K. Capital markets services: SGD 1M (SFA s86).

For each, add structured keys matching Pakistan's conventions: `minimum_paid_up_capital`, `car_requirement`, `aml_cft_program_required`, `statutory_reference`, etc.

### Layer 3 — Profile generalization

**Current state:** `profiles/sovereign-govos/profile.yaml` hard-codes Pakistan params: `identity_provider: nadra`, `tax_authority: fbr`, `corporate_registry: secp`, variant `ito-2001`, variant `raast`, `fiscal_year_start: "07-01"`.

**Target:**

1. Rename `sovereign-govos` to `sovereign-govos-pk` (it is Pakistan-specific and should be labeled as such).
2. Create `profiles/sovereign-govos-ae/profile.yaml` for UAE sovereign deployment: identity references Emirates ID (ICA), tax authority FTA, corporate registries vary by emirate (DED/ADGM/DIFC), fiscal year Jan-Dec, settlement currency AED, payment rail UAEFTS/IPP.
3. Create `profiles/sovereign-govos-sg/profile.yaml` for Singapore: identity MyInfo/Singpass (GovTech), tax authority IRAS, corporate registry ACRA, fiscal year varies (typically Jan-Dec or Apr-Mar), settlement currency SGD, payment rail FAST/PayNow.

### Layer 4 — Zone manifest instantiation

**Current state:** `jurisdictions/pk-sifc/zone.yaml` is the only zone with a complete manifest (regpack CAS digests, lawpack domains, national adapters, corridor peers). All other 100+ `zone.yaml` files are generated from `_starter/zone.yaml` with zone-specific IDs but no real regpack bindings.

**Target:**

1. Create `jurisdictions/ae-abudhabi-adgm/zone.yaml` and `jurisdictions/sg/zone.yaml` with real content: correct profile references, regpack CAS digests (from the builders you create in Layer 1), lawpack domains, compliance domains, and corridor definitions.
2. Generate `stack.lock` for each by running `mez lock`.

### Layer 5 — Multi-zone corridor topology

**Current state:** `docker-compose.two-zone.yaml` hard-codes Zone A = pk-sifc and Zone B = ae-difc. The corridor registry (`mez-corridor/src/registry.rs`) supports N-factorial pairwise generation but the deploy infrastructure only materializes 2 zones.

**Target:**

1. Create `deploy/docker/docker-compose.three-zone.yaml`: Zone A (pk-sifc), Zone B (ae-abudhabi-adgm), Zone C (sg). Three pairwise corridors. Each zone on its own internal Docker network with shared `mez-corridor-net`.
2. Extend `deploy/scripts/demo-two-zone.sh` → create `deploy/scripts/demo-multi-zone.sh` that exercises N=3 zones: corridor establishment (3 handshakes), receipt exchange across all 3 corridors, cross-zone compliance query, watcher attestation delivery.

### Layer 6 — National adapter trait generalization

**Current state:** `mez-mass-client/src/` has `fbr.rs`, `nadra.rs`, `secp.rs`, `raast.rs` — all Pakistan-specific. Core types `Cnic` and `Ntn` are baked into `mez-core/src/identity.rs`.

**Target:**

1. Create `mez-mass-client/src/adapter.rs` with a generic `NationalSystemAdapter` trait that abstracts the common patterns: identity verification, tax authority integration, corporate registry lookup, payment rail initiation. The Pakistan adapters already implement these patterns — extract the interface.
2. Create UAE adapter stubs: `mez-mass-client/src/uae/` with `emirates_id.rs` (ICA Emirates ID verification), `fta.rs` (Federal Tax Authority — VAT/excise), `ded.rs` (Department of Economic Development — trade license lookup).
3. Create Singapore adapter stubs: `mez-mass-client/src/singapore/` with `myinfo.rs` (GovTech MyInfo identity), `iras.rs` (IRAS tax), `acra.rs` (ACRA BizFile corporate registry).
4. Each adapter: define the trait interface + mock implementation, mirroring the FBR/NADRA/SECP/Raast pattern.
5. Do NOT move `Cnic`/`Ntn` out of `mez-core` — they are validated identity primitives that belong there. Instead, add equivalent newtypes for UAE (`EmiratesId` — 15-digit format `784-XXXX-XXXXXXX-X`) and Singapore (`Nric` — format `[STFGM]XXXXXXX[A-Z]`, `Uen` — 9-10 char Unique Entity Number) to `mez-core/src/identity.rs`, following the same newtype validation pattern as `Cnic`/`Ntn`.

## Constraints

- All existing tests must continue to pass. Run `cargo test --workspace` before committing.
- The Pakistan vertical must remain fully functional — you are generalizing, not replacing.
- Use real regulatory data for UAE and Singapore. Do not invent rates or thresholds. If you are uncertain about a specific value, use a clearly marked `// TODO: verify` comment rather than guessing.
- Follow the existing code style exactly. Look at Pakistan's implementations as your template.
- Every new regpack builder must have determinism tests.
- Every new licensepack requirements entry must have test coverage.
- Commit in logical units (one commit per layer, or subdivide if a layer is large).
- Push to the designated branch when complete.
