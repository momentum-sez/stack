# MSEZ Rust Workspace

The native Rust implementation of the MSEZ Stack protocol. 14 crates, 70K lines, 2,580+ tests at 98% line coverage.

---

## Building

```bash
cd msez

# Full build
cargo build --workspace

# Run all tests
cargo test --workspace

# Lint (zero warnings)
cargo clippy --workspace -- -D warnings

# Check formatting
cargo fmt --check --all

# Generate documentation
cargo doc --workspace --no-deps --open

# Security audit
cargo install cargo-audit --locked && cargo audit
```

---

## Crate Dependency Graph

```
msez-core (foundation: canonicalization, types, domains)
  |
  +-- msez-crypto (Ed25519, MMR, CAS, SHA-256)
  |     |
  |     +-- msez-vc (W3C Verifiable Credentials)
  |     |
  |     +-- msez-zkp (ZK proof system, 12 circuits)
  |     |
  |     +-- msez-tensor (compliance tensor, Dijkstra manifold)
  |
  +-- msez-state (typestate machines: corridor, entity, migration, license, watcher)
  |     |
  |     +-- msez-corridor (bridge routing, receipt chain, fork resolution, netting, SWIFT)
  |     |
  |     +-- msez-arbitration (dispute lifecycle, evidence, escrow, enforcement)
  |
  +-- msez-pack (lawpack, regpack, licensepack)
  |
  +-- msez-agentic (policy engine: 20 triggers, evaluation, scheduling, audit)
  |
  +-- msez-schema (JSON Schema validation, additionalProperties policy)

msez-api (Axum HTTP server: 5 primitives + corridors + assets + regulator)
msez-cli (Rust CLI: validate, lock, artifact operations)
msez-integration-tests (97 cross-crate E2E test files)
```

---

## Crate Reference

| Crate | Purpose | Key Types | Spec |
|---|---|---|---|
| **msez-core** | Canonical serialization (JCS), content digests, 20 compliance domains, identity newtypes | `CanonicalBytes`, `ContentDigest`, `ComplianceDomain`, `Did`, `EntityId`, `JurisdictionId`, `Cnic`, `Ntn` | 00, 02 |
| **msez-crypto** | Ed25519 signing/verification, Merkle Mountain Range, content-addressed storage | `Ed25519Keypair`, `MerkleMountainRange`, `ContentAddressedStore`, `ArtifactRef` | 80, 90, 97 |
| **msez-vc** | W3C Verifiable Credential envelope with Ed25519 proofs | `VerifiableCredential`, `ProofType`, `SmartAssetRegistryVc` | 12 |
| **msez-state** | Typestate-encoded state machines (invalid transitions are compile errors) | `Corridor<Draft>`, `Entity<Active>`, `Migration<Transit>`, `License<Pending>`, `Watcher<Bonded>` | 40, 60, 98 |
| **msez-tensor** | Compliance tensor (20 domains), Dijkstra manifold path optimization | `ComplianceTensor`, `ComplianceManifold`, `TensorCommitment` | 14 |
| **msez-zkp** | Sealed `ProofSystem` trait, mock backend (Phase 1), 12 circuit definitions | `ProofSystem` (sealed), `MockProofSystem`, `CircuitType`, `Cdb` | 80 |
| **msez-pack** | Pack trilogy: statute compilation, regulatory requirements, license lifecycle | `Lawpack`, `Regpack`, `Licensepack`, `PackValidationResult` | 96, 98 |
| **msez-corridor** | Cross-border bridge with Dijkstra routing, MMR receipt chain, fork resolution | `CorridorBridge`, `ReceiptChain`, `ForkDetector`, `NettingEngine`, `SwiftPacs008` | 40 |
| **msez-agentic** | Policy engine with 20 trigger types and deterministic evaluation | `PolicyEngine`, `TriggerType`, `ActionScheduler`, `AuditTrail` | 17 |
| **msez-arbitration** | 7-phase dispute lifecycle with typestate enforcement | `Dispute<Filed>`, `EvidenceStore`, `Escrow`, `EnforcementOrder` | 21 |
| **msez-schema** | JSON Schema Draft 2020-12 validation, security policy analysis | `SchemaValidator`, `AdditionalPropertiesViolation` | 07, 20 |
| **msez-api** | Axum HTTP server with OpenAPI generation via utoipa | `AppState`, route modules for 8 endpoint groups | 11, 12 |
| **msez-cli** | CLI with clap derive macros, backward-compatible with Python CLI | `validate`, `lock`, `artifact`, `signing` subcommands | 03 |

---

## Key Design Patterns

### Typestate Machines

State machines use the typestate pattern — invalid transitions are caught at compile time, not runtime:

```rust
// Corridor can only transition Draft -> Pending -> Active
let corridor = Corridor::<Draft>::new(id, jurisdiction_a, jurisdiction_b);
let corridor = corridor.submit();       // Draft -> Pending (compiles)
let corridor = corridor.activate();     // Pending -> Active (compiles)
// corridor.submit();                   // Active -> Pending (COMPILE ERROR)
```

### Canonical Serialization

All digest computation flows through `CanonicalBytes`, ensuring identical serialization everywhere:

```rust
use msez_core::{CanonicalBytes, sha256_digest};

let canonical = CanonicalBytes::new(&json!({"b": 2, "a": 1}))?;
// Keys are sorted, floats rejected, datetimes normalized
let digest = sha256_digest(&canonical);
// digest.to_hex() == "43258cff..."
```

### Sealed Proof System

The `ProofSystem` trait is sealed — only crate-internal implementations can exist:

```rust
// Phase 1: MockProofSystem (deterministic, non-cryptographic)
// Phase 2: Groth16ProofSystem, PlonkProofSystem (behind feature flags)
let proof = MockProofSystem::prove(&circuit_data, &witness)?;
let valid = MockProofSystem::verify(&proof, &public_inputs)?;
```

---

## API Server

Start the Axum HTTP server:

```bash
cargo run -p msez-api
# Listening on 0.0.0.0:3000
```

| Route Group | Prefix | Primitive |
|---|---|---|
| Entities | `/v1/entities` | Organization lifecycle, beneficial ownership |
| Ownership | `/v1/ownership` | Cap table, transfers, share classes |
| Fiscal | `/v1/fiscal` | Treasury, payments, withholding |
| Identity | `/v1/identity` | KYC/KYB, identity linking |
| Consent | `/v1/consent` | Multi-party consent, signing |
| Corridors | `/v1/corridors` | State channel, receipts, fork resolution |
| Smart Assets | `/v1/assets` | Registry, compliance eval, anchor verify |
| Regulator | `/v1/regulator` | Query access, compliance reports |

OpenAPI spec auto-generated at `/openapi.json`.

---

## Testing

```bash
# All tests
cargo test --workspace

# Specific crate
cargo test -p msez-core

# Specific test
cargo test -p msez-corridor -- test_receipt_chain

# With output
cargo test -p msez-api -- --nocapture

# Integration tests only
cargo test -p msez-integration-tests
```

### Coverage

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --html
# Report at target/llvm-cov/html/index.html
```

Current coverage: **98% line coverage** across all crates.

---

## Adding a New Crate

1. Create the crate: `cargo new crates/msez-<name> --lib`
2. Add to workspace members in `Cargo.toml`
3. Add shared dependencies with `{ workspace = true }`
4. Add to `msez-integration-tests/Cargo.toml` dev-dependencies
5. Add tests following existing patterns
6. Update the traceability matrix in `docs/traceability-matrix.md`

---

## References

| Document | Path |
|----------|------|
| Spec-to-Crate Traceability | [docs/traceability-matrix.md](../docs/traceability-matrix.md) |
| Architecture Deep Dive | [docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md) |
| Security Audit | [docs/fortification/sez_stack_audit_v2.md](../docs/fortification/sez_stack_audit_v2.md) |
| CI Pipeline | [.github/workflows/ci.yml](../.github/workflows/ci.yml) |
