# MEZ Rust Workspace — v0.4.44 GENESIS

The native Rust implementation of the Momentum EZ Stack. 17 crates, 159K lines, 4,601 tests.

---

## Building

```bash
cd mez

# Full build
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint (zero warnings required)
cargo clippy --workspace -- -D warnings

# Check formatting
cargo fmt --check --all
```

---

## Crate Dependency Graph

```
mez-core (foundation: canonicalization, types, 20 compliance domains)
  |
  +-- mez-crypto (Ed25519, MMR, CAS, SHA-256)
  |     |
  |     +-- mez-vc (W3C Verifiable Credentials)
  |     |
  |     +-- mez-zkp (ZK proof system, 12 circuits)
  |
  +-- mez-tensor (compliance tensor, Dijkstra manifold)
  |
  +-- mez-state (typestate machines: corridor, entity, migration, license, watcher)
  |     |
  |     +-- mez-corridor (receipt chain, fork resolution, netting, SWIFT, trade flows)
  |     |
  |     +-- mez-arbitration (dispute lifecycle, evidence, escrow, enforcement)
  |
  +-- mez-pack (lawpack, regpack, licensepack)
  |
  +-- mez-agentic (policy engine: 20 triggers, evaluation, scheduling, audit)
  |
  +-- mez-compliance (jurisdiction config bridge: regpack -> tensor)
  |
  +-- mez-schema (JSON Schema validation, Draft 2020-12)
  |
  +-- mez-mass-client (typed HTTP client for Mass APIs)
  |
  +-- mez-mass-stub (standalone Mass API stub for dev/testing)

mez-api (Axum HTTP server: composition root for all crates)
mez-cli (CLI: validate, lock, corridor, artifact, vc, regpack, deploy)
mez-integration-tests (cross-crate integration test suite)
```

---

## Crate Reference

| Crate | Purpose | Key Types |
|---|---|---|
| **mez-core** | Canonical serialization (JCS+MCF), content digests, 20 compliance domains, identity newtypes | `CanonicalBytes`, `ContentDigest`, `ComplianceDomain`, `EntityId`, `JurisdictionId`, `Cnic`, `Ntn`, `EmiratesId`, `Nric` |
| **mez-crypto** | Ed25519 signing/verification, Merkle Mountain Range, content-addressed storage | `Ed25519Keypair`, `MerkleMountainRange`, `ContentAddressedStore`, `ArtifactRef` |
| **mez-vc** | W3C Verifiable Credential envelope with Ed25519 proofs | `VerifiableCredential`, `ProofType`, `SmartAssetRegistryVc` |
| **mez-state** | Typestate-encoded state machines (invalid transitions are compile errors) | `Corridor<Draft>`, `Entity<Active>`, `Migration<Transit>`, `License<Pending>`, `Watcher<Bonded>` |
| **mez-tensor** | Compliance tensor (20 domains), Dijkstra manifold path optimization | `ComplianceTensor`, `ComplianceManifold`, `TensorCommitment` |
| **mez-zkp** | Sealed `ProofSystem` trait, mock backend (Phase 1), 12 circuit definitions | `ProofSystem` (sealed), `MockProofSystem`, `CircuitType`, `Cdb` |
| **mez-pack** | Pack trilogy: statute compilation, regulatory requirements, license lifecycle | `Lawpack`, `Regpack`, `Licensepack`, `PackValidationResult` |
| **mez-corridor** | Receipt chain (MMR), fork resolution, netting, SWIFT, trade flow instruments | `ReceiptChain`, `ForkDetector`, `NettingEngine`, `SwiftPacs008`, `TradeFlowManager` |
| **mez-agentic** | Policy engine with 20 trigger types and deterministic evaluation | `PolicyEngine`, `TriggerType`, `ActionScheduler`, `AuditTrail` |
| **mez-arbitration** | 7-phase dispute lifecycle with typestate enforcement | `Dispute<Filed>`, `EvidenceStore`, `Escrow`, `EnforcementOrder` |
| **mez-compliance** | Jurisdiction config bridge (regpack -> tensor evaluation) | `JurisdictionConfig`, `ComplianceEvaluator` |
| **mez-schema** | JSON Schema Draft 2020-12 validation with compiled validator cache | `SchemaValidator`, `AdditionalPropertiesViolation` |
| **mez-mass-client** | Typed HTTP client for all five Mass API primitives | `FiscalClient`, `OwnershipClient`, `IdentityClient`, `ConsentClient` |
| **mez-mass-stub** | Standalone Mass API stub server (DashMap-backed, for dev without Postgres) | Axum routes matching Mass API surface |
| **mez-api** | Axum HTTP server — composition root, orchestration pipeline, Postgres persistence | `AppState`, route modules for all endpoint groups |
| **mez-cli** | CLI: validate, lock, corridor, artifact, vc, regpack build, deploy gen | `validate`, `lock`, `artifact`, `signing`, `corridor`, `compose` subcommands |
| **mez-integration-tests** | Cross-crate integration test suite | End-to-end flow tests |

---

## API Server

Start the Axum HTTP server:

```bash
cargo run -p mez-api
# Listening on 0.0.0.0:3000
```

| Route Group | Prefix | Domain |
|---|---|---|
| Entities | `/v1/entities` | Mass Entities (proxy or sovereign) |
| Ownership | `/v1/ownership` | Mass Ownership |
| Fiscal | `/v1/fiscal` | Mass Fiscal |
| Identity | `/v1/identity` | Mass Identity |
| Consent | `/v1/consent` | Mass Consent |
| Corridors | `/v1/corridors` | Corridor lifecycle, receipts, forks |
| Settlement | `/v1/settlement` | Netting, SWIFT instructions |
| Smart Assets | `/v1/assets` | Registry, compliance eval |
| Credentials | `/v1/credentials` | VC issuance, verification |
| Trade Flows | `/v1/trade/flows` | Trade flow lifecycle, transitions |
| Compliance | `/v1/compliance` | Entity compliance queries |
| Regulator | `/v1/regulator` | Compliance monitoring |
| Health | `/health/liveness`, `/health/readiness` | Probes |

---

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p mez-core

# Specific test
cargo test -p mez-corridor -- test_receipt_chain

# Integration tests only
cargo test -p mez-integration-tests
```

---

## Adding a New Crate

1. Create the crate: `cargo new crates/mez-<name> --lib`
2. Add to workspace members in `Cargo.toml`
3. Add shared dependencies with `{ workspace = true }`
4. Add to `mez-integration-tests/Cargo.toml` dev-dependencies
5. Add tests following existing patterns
6. Update the traceability matrix in `docs/traceability-matrix.md`

---

## References

| Document | Path |
|----------|------|
| Project README | [README.md](../README.md) |
| Spec-to-Crate Traceability | [docs/traceability-matrix.md](../docs/traceability-matrix.md) |
| Architecture Deep Dive | [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) |
| Crate Reference | [docs/architecture/CRATE-REFERENCE.md](../docs/architecture/CRATE-REFERENCE.md) |
| CI Pipeline | [.github/workflows/ci.yml](../.github/workflows/ci.yml) |
