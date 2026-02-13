# Spec-to-Crate Traceability Matrix

This document maps each specification chapter in `spec/` to its corresponding Rust crate implementation in `msez/crates/`. It serves as the authoritative reference for understanding which code implements which spec requirements, and identifies gaps where spec chapters have no Rust implementation.

Generated from the v0.4.44 GENESIS codebase. The Rust workspace contains 14 crates implementing the core protocol, cryptographic primitives, state machines, compliance evaluation, and HTTP API surface.

## Traceability Matrix

| Spec Chapter | Title | Rust Crate(s) | Key Types / Functions | Status |
|---|---|---|---|---|
| `00-terminology.md` | Terminology | `msez-core` | All identity newtypes (`Did`, `EntityId`, `CorridorId`, `JurisdictionId`, `Cnic`, `Ntn`, `PassportNumber`, `WatcherId`, `MigrationId`), `ComplianceDomain` (20 variants), `Timestamp` | Implemented |
| `01-mission.md` | Mission | N/A | Non-normative; no direct implementation required | N/A |
| `02-invariants.md` | Protocol Invariants | `msez-core` | `CanonicalBytes` (JCS canonicalization), `ContentDigest` (SHA-256), `MsezError` (error hierarchy) | Implemented |
| `03-standard-structure.md` | Standard Document Structure | `msez-cli` | `resolve_repo_root()`, `resolve_path()` | Implemented |
| `04-design-rubric.md` | Design Decision Criteria | N/A | Non-normative; architectural guidance only | N/A |
| `10-repo-layout.md` | Repository Structure | `msez-cli` | Path resolution and repo root detection | Implemented |
| `11-architecture-overview.md` | System Architecture | `msez-core`, `msez-api` | `AppState`, router assembly, middleware stack | Implemented |
| `12-mass-primitives-mapping.md` | MASS Five Primitives | `msez-api` | Routes: `entities`, `ownership`, `fiscal`, `identity`, `consent` | Implemented |
| `17-agentic.md` | Agentic Policy Automation | `msez-agentic` | `TriggerType` (20 variants per Def 17.1), `Policy`, `PolicyEngine`, `ActionScheduler`, `AuditTrail`, `standard_policies()` (4), `extended_policies()` (19) | Implemented |
| `20-module-system.md` | Module Composition | `msez-schema`, `msez-pack` | `SchemaValidator::validate_module()`, `PackValidationResult` | Implemented |
| `22-templating-and-overlays.md` | Templates and Overlays | `msez-pack` | `Lawpack::load()`, pack YAML/JSON parsing | Partial |
| `30-profile-system.md` | Zone Profile Definitions | `msez-schema`, `msez-cli` | `SchemaValidator::validate_profile()`, `run_validate()` | Implemented |
| `40-corridors.md` | Cross-Border Corridors | `msez-corridor`, `msez-state` | `Corridor<S>` typestate (`Draft`, `Pending`, `Active`, `Halted`, `Suspended`, `Deprecated`), `CorridorBridge` (Dijkstra routing), `ReceiptChain` (MMR-backed), `ForkDetector`, `ForkResolution` (3-level ordering), `AnchorReceipt`, `NettingEngine`, `SwiftPacs008` | Implemented |
| `41-nodes.md` | Node Architecture | `msez-api` | K8s probes (`liveness`, `readiness`), `AppConfig` | Partial |
| `50-conformance.md` | Conformance Testing | `msez-integration-tests` | 8 cross-crate integration test files | Implemented |
| `60-governance.md` | Governance Structures | `msez-state` | `Corridor<S>` lifecycle transitions, `governance/corridor.lifecycle.state-machine.v2.json` | Implemented |
| `61-network-diffusion.md` | Network Propagation | — | **GAP: No Rust implementation** | Not implemented |
| `71-regulator-console.md` | Regulator Query Interface | `msez-api` | `routes::regulator` module | Implemented |
| `80-security-privacy.md` | Security Model and Privacy | `msez-crypto`, `msez-zkp` | `Ed25519Signature` (signing/verification), `ProofSystem` trait (sealed), `MockProofSystem`, `Cdb` (Canonical Digest Bridge), 12 circuit types | Partial (ZKP mocked) |
| `90-provenance.md` | Provenance Tracking | `msez-crypto` | `ContentAddressedStore` (CAS), `ArtifactRef` | Implemented |
| `95-lockfile.md` | Stack Lockfile | `msez-cli`, `msez-pack` | `run_lock()`, `Lawpack::digest()` | Implemented |
| `96-lawpacks.md` | Lawpack System | `msez-pack` | `Lawpack` (load, digest, validation) | Implemented |
| `97-artifacts.md` | Content-Addressed Storage | `msez-crypto` | `ContentAddressedStore::store()`, `ContentAddressedStore::resolve()`, `ArtifactRef` | Implemented |
| `98-licensepacks.md` | Licensepack System | `msez-pack`, `msez-state` | `Licensepack` (evaluate_compliance), `License<S>` typestate (`Pending`, `Active`, `Suspended`, `Revoked`, `Expired`) | Implemented |

## Crate-to-Spec Reverse Index

| Rust Crate | Lines (lib.rs) | Primary Spec Chapters | Purpose |
|---|---|---|---|
| `msez-core` | ~1,700 | 00, 02 | Foundational types: canonical serialization (JCS), content digests, 20 compliance domains, identity newtypes, temporal types |
| `msez-crypto` | ~1,400 | 80, 90, 97 | Ed25519 signing/verification, Merkle Mountain Range (MMR), content-addressed storage (CAS), SHA-256 |
| `msez-vc` | ~800 | 12 (W3C VC) | Verifiable Credential envelope, Ed25519 proofs, Smart Asset Registry VCs |
| `msez-state` | ~1,800 | 40, 60, 98 | Typestate-encoded state machines: Corridor (6 states), Entity (10-stage dissolution), Migration (9 states), License (5 states), Watcher (4 states) |
| `msez-tensor` | ~1,700 | 14 (Compliance Manifold) | Compliance tensor (20 domains), Dijkstra path optimization, tensor commitments (Merkle root) |
| `msez-zkp` | ~1,000 | 80 (ZK Proofs) | Sealed ProofSystem trait, MockProofSystem (Phase 1), 12 circuit type definitions, Canonical Digest Bridge |
| `msez-pack` | ~1,000 | 96, 98, 20 | Pack trilogy: Lawpack (statute compilation), Regpack (regulatory requirements), Licensepack (license lifecycle) |
| `msez-corridor` | ~3,000 | 40 | Corridor bridge (Dijkstra routing), receipt chain (MMR), fork detection/resolution (3-level ordering, 5-min clock skew), L1 anchoring, settlement netting, SWIFT adapter |
| `msez-agentic` | ~2,000 | 17 | Policy engine: 20 trigger types, deterministic evaluation (Theorem 17.1), action scheduling (cron), tamper-evident audit trail |
| `msez-arbitration` | ~1,500 | 21 (Arbitration) | Dispute lifecycle (7 typestate phases), evidence management, escrow operations, enforcement with corridor receipts |
| `msez-schema` | ~700 | 20, 07 | Runtime JSON Schema validation (Draft 2020-12), `additionalProperties` security policy checks |
| `msez-api` | ~1,500 | 12, 40, 71 | Axum HTTP server: 8 route modules (5 primitives + corridors + assets + regulator), auth middleware, rate limiting, OpenAPI generation |
| `msez-cli` | ~1,000 | 03, 10 | Rust CLI replacing Python `tools/msez.py`: validate, lock, corridor, artifact, vc subcommands |
| `msez-integration-tests` | — | 50 | 8 cross-crate end-to-end test suites |

## Identified Gaps

### Spec chapters with no Rust implementation

| Spec Chapter | Title | Gap Description | Priority |
|---|---|---|---|
| `04-design-rubric.md` | Design Decision Criteria | Non-normative; no code required | N/A |
| `22-templating-and-overlays.md` | Templates and Overlays | Partially covered by `msez-pack` YAML parsing; full template engine not yet in Rust | Low |
| `41-nodes.md` | Node Architecture | K8s probes exist in `msez-api`; full node discovery/gossip protocol not implemented | Medium |
| `61-network-diffusion.md` | Network Propagation | **No Rust implementation.** Network gossip, receipt propagation, and peer discovery are not yet built. This is the largest remaining gap. | High |
| `80-security-privacy.md` | Security (ZKP portion) | `msez-zkp` has sealed traits and 12 circuit definitions, but all proofs are mocked (Phase 1). Real ZK backends (Groth16/PLONK) are feature-gated but unimplemented. | High (Phase 2) |

### Partial implementations

| Area | Current State | Gap |
|---|---|---|
| ZK Proofs | `MockProofSystem` (deterministic) | Real Groth16/PLONK/STARK backends (Phase 2, feature-gated as `groth16` and `plonk`) |
| Poseidon2 Hash | Stubbed in `msez-crypto` (`poseidon2` feature flag) | ZK-friendly hashing for Canonical Digest Bridge |
| BBS+ Signatures | Stubbed in `msez-crypto` (`bbs-plus` feature flag) | Selective disclosure for privacy-preserving compliance |
| SWIFT pacs.008 | `SwiftPacs008` adapter exists | Full ISO 20022 XML compliance not verified |

## Workspace Dependency Graph

```
msez-core (foundation - no internal deps)
  |
  +-- msez-crypto (Ed25519, MMR, CAS)
  |     |
  |     +-- msez-vc (Verifiable Credentials)
  |     |     |
  |     |     +-- msez-schema (Schema validation)
  |     |
  |     +-- msez-zkp (Zero-knowledge proofs)
  |     |
  |     +-- msez-tensor (Compliance tensor)
  |
  +-- msez-state (Typestate machines)
  |     |
  |     +-- msez-corridor (Cross-border operations)
  |     |
  |     +-- msez-arbitration (Dispute resolution)
  |
  +-- msez-pack (Lawpack, Regpack, Licensepack)
  |
  +-- msez-agentic (Policy engine)
  |
  +-- msez-api (Axum HTTP server)
  |     depends on: msez-core, msez-state, axum, tokio, utoipa
  |
  +-- msez-cli (CLI binary)
  |     depends on: msez-core, msez-crypto, msez-pack, msez-schema, msez-state, clap
  |
  +-- msez-integration-tests (all crates via dev-dependencies)
```

## Audit Finding Remediation Status in Rust

| Audit Finding | Status | Rust Prevention |
|---|---|---|
| 2.1: Bare exception handling | Resolved | Rust's `Result<T, E>` with `thiserror` eliminates bare `except` |
| 2.2: Poseidon2 unimplemented | Phase 2 | Feature-gated in `msez-crypto` and `msez-zkp` |
| 2.3: State name divergence | **Resolved** | Typestate pattern in `msez-state`: no string `"OPERATIONAL"` exists. v2 governance JSON created. |
| 2.4: Dual domain enums | Resolved | Single `ComplianceDomain` enum (20 variants) in `msez-core`, used by all crates |
| 2.5: ZKP entirely mocked | Phase 1 | `MockProofSystem` implements sealed `ProofSystem` trait; Phase 2 backends feature-gated |
| 3.1: `additionalProperties` lax | Resolved | `msez-schema` enforces security policy via `check_additional_properties_policy()` |
| 3.5: Fork resolution timestamp-only | Resolved | `msez-corridor::fork` implements 3-level ordering with `MAX_CLOCK_SKEW = 5 minutes` |
| 5.2: Inconsistent canonicalization | Resolved | All digests flow through `CanonicalBytes::new()` in `msez-core` |
