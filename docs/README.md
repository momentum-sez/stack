# MSEZ Stack Documentation

## v0.4.44 GENESIS -- The Operating System for Special Economic Zones

The MSEZ Stack is programmable jurisdictional infrastructure that transforms incorporation, compliance, taxation, and cross-border trade into software primitives. It delivers **146 fully-implemented modules** across 16 families, powered by the **PHOENIX Smart Asset Operating System** and backed by a **14-crate Rust workspace** providing compile-time correctness guarantees for sovereign digital infrastructure.

This page is the central navigation hub for all MSEZ Stack documentation.

---

## Start Here

New to the MSEZ Stack? Begin with the getting started guide.

| Document | Description |
|----------|-------------|
| [Getting Started](./getting-started.md) | Installation, first zone creation, and core workflow |
| [Architecture Overview](./architecture/OVERVIEW.md) | System design and layer architecture |
| [Specification](../spec/) | Normative protocol specification (24 chapters) |
| [Traceability Matrix](./traceability-matrix.md) | Spec chapter to Rust crate mapping |

---

## Documentation Index

### Getting Started and Onboarding

| Document | Description |
|----------|-------------|
| [Getting Started](./getting-started.md) | Install dependencies, validate modules, build profiles |
| [Module Authoring](./authoring/modules.md) | How to create and structure zone modules |
| [Corridor Overview](./corridors/overview.md) | Cross-border corridor fundamentals |
| [Zone Deployment Guide](./operators/ZONE-DEPLOYMENT-GUIDE.md) | Deploy a zone from scratch |

### Architecture and Design

| Document | Description |
|----------|-------------|
| [Architecture (Full)](./ARCHITECTURE.md) | Complete PHOENIX architecture document |
| [Architecture Overview](./architecture/OVERVIEW.md) | System architecture summary |
| [Security Model](./architecture/SECURITY-MODEL.md) | Threat model, trust boundaries, cryptographic protocols |
| [Legal Integration](./architecture/LEGAL-INTEGRATION.md) | Akoma Ntoso legal text integration |
| [MASS Integration](./architecture/MASS-INTEGRATION.md) | MASS five primitives mapping |
| [Smart Asset Integration](./architecture/SMART-ASSET-INTEGRATION.md) | Smart Asset lifecycle and compliance |
| [Smart Asset OS](./architecture/SMART-ASSET-OS.md) | PHOENIX execution layer design |

### Operator Guides

| Document | Description |
|----------|-------------|
| [Zone Deployment](./operators/ZONE-DEPLOYMENT-GUIDE.md) | 7-step zone deployment procedure |
| [Corridor Formation](./operators/CORRIDOR-FORMATION-GUIDE.md) | Establish cross-border corridors |
| [Incident Response](./operators/INCIDENT-RESPONSE.md) | Incident handling and recovery playbooks |

### Authoring Guides

| Document | Description |
|----------|-------------|
| [Module Authoring](./authoring/modules.md) | Module YAML descriptors, families, and validation |
| [Akoma Ntoso](./authoring/akoma.md) | Legal text authoring in Akoma Ntoso XML |
| [Legal Corpus](./authoring/legal-corpus.md) | Building and maintaining the legal corpus |
| [Licensing Pack](./authoring/licensing-pack.md) | License registry and licensepack lifecycle |
| [Corridor Authoring](./authoring/corridors.md) | Defining corridor agreements and routing |

### Corridors

| Document | Description |
|----------|-------------|
| [Corridor Overview](./corridors/overview.md) | Corridor model, receipt chains, fork resolution |
| [Trade Playbooks](./corridors/playbooks.md) | End-to-end trade flow playbook generation |

### Error Handling

| Document | Description |
|----------|-------------|
| [Error Taxonomy](./ERRORS.md) | RFC 7807 compliant error codes, recovery strategies, PHOENIX error catalog |

### Performance

| Document | Description |
|----------|-------------|
| [Performance Harness](./PERFORMANCE.md) | Performance tests, workload tuning, regression guards |

### Specification

The `spec/` directory contains the normative protocol specification. All implementation decisions defer to the spec as the canonical source of truth.

| Chapter | Title |
|---------|-------|
| [00-terminology](../spec/00-terminology.md) | Terminology and definitions |
| [01-mission](../spec/01-mission.md) | Mission statement |
| [02-invariants](../spec/02-invariants.md) | Protocol invariants |
| [03-standard-structure](../spec/03-standard-structure.md) | Standard document structure |
| [04-design-rubric](../spec/04-design-rubric.md) | Design decision criteria |
| [10-repo-layout](../spec/10-repo-layout.md) | Repository structure |
| [11-architecture-overview](../spec/11-architecture-overview.md) | System architecture |
| [12-mass-primitives-mapping](../spec/12-mass-primitives-mapping.md) | MASS five primitives |
| [17-agentic](../spec/17-agentic.md) | Agentic policy automation |
| [20-module-system](../spec/20-module-system.md) | Module composition |
| [22-templating-and-overlays](../spec/22-templating-and-overlays.md) | Templates and overlays |
| [30-profile-system](../spec/30-profile-system.md) | Zone profile definitions |
| [40-corridors](../spec/40-corridors.md) | Cross-border corridors |
| [41-nodes](../spec/41-nodes.md) | Node architecture |
| [50-conformance](../spec/50-conformance.md) | Conformance testing |
| [60-governance](../spec/60-governance.md) | Governance structures |
| [61-network-diffusion](../spec/61-network-diffusion.md) | Network propagation |
| [71-regulator-console](../spec/71-regulator-console.md) | Regulator query interface |
| [80-security-privacy](../spec/80-security-privacy.md) | Security model and privacy |
| [90-provenance](../spec/90-provenance.md) | Provenance tracking |
| [95-lockfile](../spec/95-lockfile.md) | Zone lockfile specification |
| [96-lawpacks](../spec/96-lawpacks.md) | Lawpack format and lifecycle |
| [97-artifacts](../spec/97-artifacts.md) | Content-addressed artifact store |
| [98-licensepacks](../spec/98-licensepacks.md) | Licensepack format and lifecycle |

### Traceability Matrix

| Document | Description |
|----------|-------------|
| [Traceability Matrix](./traceability-matrix.md) | Maps every spec chapter to its implementing Rust crate, key types, and implementation status |

### Security Audit and Fortification

| Document | Description |
|----------|-------------|
| [Institutional Audit v2](./fortification/sez_stack_audit_v2.md) | Seven-pass audit report, Rust migration architecture, tiered execution roadmap |
| [Bug Hunt Log](./bughunt/BUGHUNT_LOG.md) | Historical bug tracking and resolution log |

### Attestations

| Document | Description |
|----------|-------------|
| [Attestation Catalog](./attestations/catalog.md) | Catalog of attestation types and their schemas |

### Roadmap

| Document | Description |
|----------|-------------|
| [Production Grade Spec](./roadmap/PRODUCTION_GRADE_SPEC.md) | Requirements for production deployment |
| [Roadmap Pre-0.5](./roadmap/ROADMAP_PRE_0.5.md) | Path from v0.4.44 GENESIS to v0.5 |
| [Delta v0.44 SEZ Complete](./roadmap/DELTA_V044_SEZ_COMPLETE.md) | Changelog for the GENESIS milestone |
| [Prerequisites to Ship v0.40](./roadmap/PREREQS_TO_SHIP_V0.40.md) | Pre-ship checklist for v0.40 |
| [v0.41 Regpack and Arbitration](./roadmap/ROADMAP_V041_REGPACK_ARBITRATION.md) | Regpack and arbitration framework roadmap |
| [v0.42 Agentic Framework](./roadmap/ROADMAP_V042_AGENTIC_FRAMEWORK.md) | Agentic policy automation roadmap |
| [v0.43 Hard Mode](./roadmap/ROADMAP_V043_HARD_MODE.md) | Production hardening roadmap |
| [v0.4.41 Regpack Arbitration](./roadmap/V0.4.41_REGPACK_ARBITRATION.md) | Detailed regpack and arbitration plan |

### Release Notes

Every patch release from v0.4.15 through v0.4.44 GENESIS is documented in `docs/patchlists/`:

| Release | Highlights |
|---------|------------|
| [v0.4.44](./patchlists/v0.4.44.md) | GENESIS -- full SEZ module coverage, Pack Trilogy, AWS deployment |
| [v0.4.43](./patchlists/v0.4.43.md) | Hard Mode hardening pass |
| [v0.4.42](./patchlists/v0.4.42.md) | Agentic framework and resilience |
| [v0.4.41](./patchlists/v0.4.41.md) | Regpack, arbitration, and netting |
| [v0.4.38](./patchlists/v0.4.38.md) | Performance and scaling |
| [v0.4.36](./patchlists/v0.4.36.md) | Corridor state machine refinement |
| [v0.4.35](./patchlists/v0.4.35.md) | Smart Asset VM opcodes |
| [v0.4.34](./patchlists/v0.4.34.md) | ZK proof scaffolding |
| [v0.4.33](./patchlists/v0.4.33.md) | Watcher economy and slashing |
| [v0.4.32](./patchlists/v0.4.32.md) | Migration saga protocol |
| [v0.4.31](./patchlists/v0.4.31.md) | Compliance tensor and manifold |
| [v0.4.28](./patchlists/v0.4.28_witness_bundle_attestation.md) | Witness bundle attestation |
| [v0.4.27](./patchlists/v0.4.27_artifact_graph_from_bundle.md) | Artifact graph from bundle |
| [v0.4.26](./patchlists/v0.4.26_artifact_graph_witness_bundle.md) | Artifact graph witness bundle |
| [v0.4.25](./patchlists/v0.4.25_artifact_graph_verify.md) | Artifact graph verification |
| [v0.4.24](./patchlists/v0.4.24_transitive_artifactref_closure.md) | Transitive artifact reference closure |
| [v0.4.23](./patchlists/v0.4.23_ruleset_transitive_closure_scaffold_scaling.md) | Ruleset transitive closure |
| [v0.4.22](./patchlists/v0.4.22_transitive_require_artifacts_roadmap_scaffold_patchlist.md) | Transitive require and artifact roadmap |
| [v0.4.20](./patchlists/v0.4.20_cli_signing_hardening_patchlist.md) | CLI signing hardening |
| [v0.4.19](./patchlists/v0.4.19_lifecycle_tests_patchlist.md) | Lifecycle tests |
| [v0.4.18](./patchlists/v0.4.18_fork_resolution_transition_envelopes_patchlist.md) | Fork resolution transition envelopes |
| [v0.4.17](./patchlists/v0.4.17_watcher_quorum_patchlist.md) | Watcher quorum |
| [v0.4.16](./patchlists/v0.4.16_watcher_compare_patchlist.md) | Watcher comparison |
| [v0.4.15](./patchlists/v0.4.15_high_leverage_patchlist.md) | High leverage initial patchlist |
| [v0.4 Spec](./patchlists/v0.4_spec_patchlist.md) | Specification patchlist |

### Examples

Working examples are provided in `docs/examples/`:

| Directory | Contents |
|-----------|----------|
| [Agentic](./examples/agentic/) | Sanctions halt scenario JSON |
| [Keys](./examples/keys/) | Development Ed25519 JWK key pairs |
| [Lawpack](./examples/lawpack/) | Akoma Ntoso enabling act example |
| [Regpack](./examples/regpack/) | License registry and sanctions snapshots |
| [State](./examples/state/) | No-op transition example |
| [Trade](./examples/trade/) | Full trade playbook with zones, artifacts, receipts, and settlement |
| [VC](./examples/vc/) | Verifiable Credential examples (signed and unsigned) |

---

## Rust Crate Workspace

The `msez/` directory contains the Rust implementation of the MSEZ protocol. It is a Cargo workspace with **14 crates**, approximately **70,000 lines** of Rust across **198 source files**, targeting Rust edition 2021 (MSRV 1.75). Licensed under BUSL-1.1.

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `msez-core` | Identity newtypes, canonical serialization, error hierarchy | `Did`, `EntityId`, `CorridorId`, `ComplianceDomain`, `CanonicalBytes`, `ContentDigest` |
| `msez-crypto` | Ed25519 signing, SHA-256, content-addressed storage | `Ed25519Signature`, `ContentAddressedStore`, `ArtifactRef` |
| `msez-vc` | W3C Verifiable Credential issuance and verification | `VerifiableCredential`, `Proof`, `CredentialSubject` |
| `msez-state` | Typestate corridor lifecycle, state machine transitions | `Corridor<Draft>`, `Corridor<Pending>`, `Corridor<Active>`, `Corridor<Halted>` |
| `msez-tensor` | 4D sparse compliance tensor, domain evaluation | `ComplianceTensor`, `TensorCommitment`, `DomainScore` |
| `msez-zkp` | Zero-knowledge proof system trait and mock implementation | `ProofSystem` (sealed trait), `MockProofSystem`, 12 circuit types |
| `msez-pack` | Lawpack, regpack, and licensepack validation | `Lawpack`, `PackValidationResult` |
| `msez-corridor` | Corridor bridge, receipt chains (MMR-backed), fork detection, netting | `CorridorBridge`, `ReceiptChain`, `ForkDetector`, `NettingEngine` |
| `msez-agentic` | Policy engine, triggers, action scheduling, audit trail | `TriggerType` (20 variants), `PolicyEngine`, `ActionScheduler` |
| `msez-arbitration` | Dispute lifecycle, evidence packages, enforcement | `DisputeRequest`, `ArbitrationAward`, `EnforcementReceipt` |
| `msez-schema` | JSON Schema Draft 2020-12 validation (116 schemas) | `SchemaValidator` |
| `msez-api` | Axum HTTP API, five primitives routes, OpenAPI generation | `AppState`, router assembly, middleware stack |
| `msez-cli` | Clap-based CLI, repo root detection, path resolution | `resolve_repo_root()`, `run_validate()` |
| `msez-integration-tests` | Cross-crate integration test suite | 8 integration test files |

### Rust Dependencies

The workspace uses pinned dependencies managed through `Cargo.lock`. Key external crates: `serde`, `serde_json`, `axum`, `tokio`, `ed25519-dalek`, `sha2`, `chrono`, `clap`, `tracing`, `sqlx` (Postgres), `utoipa` (OpenAPI), `proptest` (property testing).

See [Traceability Matrix](./traceability-matrix.md) for the complete mapping of spec chapters to Rust crate implementations.

---

## The Five MASS Programmable Primitives

The MSEZ Stack implements the Momentum MASS (Modular Architecture for Sovereign Services) model, which decomposes jurisdictional operations into five programmable primitives. Every government service, every compliance check, and every cross-border transaction maps to one or more of these primitives.

| Primitive | Domain | API Surface | Description |
|-----------|--------|-------------|-------------|
| **Entities** | Organization Info | `POST/GET /v1/entities`, dissolution, beneficial ownership | Entity formation, lifecycle management, corporate registry, 10-stage dissolution |
| **Ownership** | Investment Info | `POST/GET /v1/ownership`, cap table, transfers | Cap table management, share class definitions, ownership transfers with capital gains tracking |
| **Fiscal** | Treasury Info | `POST /v1/fiscal/payments`, withholding, tax events | Treasury accounts, payment initiation, withholding-at-source, tax event history, FBR IRIS integration |
| **Identity** | Identity Info | `POST /v1/identity/verify`, attestation, linking | KYC/KYB verification, external ID linking (CNIC, NTN, passport), identity attestations |
| **Consent** | Consent Info | `POST /v1/consent/request`, sign, audit trail | Multi-party consent workflows, cryptographic signing, full audit trail |

Architecture details: [MASS Integration](./architecture/MASS-INTEGRATION.md) |
Specification: [spec/12-mass-primitives-mapping](../spec/12-mass-primitives-mapping.md)

---

## Module Families

The MSEZ Stack ships 146 modules across 16 families. Every module is defined by a YAML descriptor in `modules/`.

| Family | Modules | Domain |
|--------|---------|--------|
| Legal Foundation | 9 | Enabling acts, regulatory frameworks, Akoma Ntoso legal corpus |
| Corporate Services | 8 | Formation, registered agent, secretarial, beneficial ownership, dissolution |
| Regulatory Framework | 8 | Compliance rules, sanctions screening, reporting obligations |
| Licensing | 16 | Business licenses, professional certifications, license lifecycle |
| Identity | 6 | DIDs, KYC tiers, credentials, entity-identity binding |
| Financial Infrastructure | 14 | Payment rails, treasury, settlement accounts, FX |
| Capital Markets | 9 | Securities, cap tables, vesting, share classes |
| Trade and Commerce | 8 | Import/export, customs, trade finance, proof bindings |
| Tax and Revenue | 7 | Withholding, incentives, transfer pricing, reporting |
| Corridors and Settlement | 7 | Bilateral corridors, receipt chains, netting, anchoring |
| Governance and Civic | 10 | Board governance, voting, civic registries |
| Arbitration | 8 | Dispute claims, evidence, hearings, enforcement |
| Operations | 9 | Monitoring, logging, health checks, deployment automation |
| PHOENIX Execution | 18 | Smart Asset VM, compliance tensor, ZK proofs, migration |
| Agentic Automation | 6 | Policy triggers, action scheduling, environment monitoring |
| Deployment | 11 | Terraform, Docker, Kubernetes, CI/CD pipelines |

---

## CLI Quick Reference

### Rust CLI (`msez-cli`)

```bash
# Build the workspace
cargo build --release -p msez-cli

# Run tests (2580+ tests, targeting 98% coverage)
cargo test --workspace

# Run a specific crate's tests
cargo test -p msez-corridor
cargo test -p msez-tensor

# Property-based testing
cargo test -p msez-core -- --include-ignored proptest
```

### Python CLI (`tools/msez.py`)

```bash
# Install dependencies
pip install -r tools/requirements.txt

# Validate all modules (146 modules, 16 families)
python -m tools.msez validate --all-modules

# Validate all profiles
python -m tools.msez validate --all-profiles

# Validate all zones
python -m tools.msez validate --all-zones

# Validate a specific profile
python -m tools.msez validate profiles/digital-financial-center/profile.yaml

# Lock a zone (generate stack.lock)
python -m tools.msez lock jurisdictions/_starter/zone.yaml

# Check lock integrity without regenerating
python -m tools.msez lock jurisdictions/_starter/zone.yaml --check

# Build a profile to dist/
python -m tools.msez build profiles/digital-financial-center/profile.yaml --out dist/

# Generate a trade playbook
python -m tools.msez trade generate --corridor PK-AE

# Store an artifact in the CAS
python -m tools.msez artifact store <file>

# Resolve an artifact by digest
python -m tools.msez artifact resolve <digest>

# Verify an artifact graph
python -m tools.msez artifact graph-verify <bundle>

# Sign a Verifiable Credential
python -m tools.msez sign --key <jwk> <vc.json>

# Run the full test suite
pytest -q

# Run performance tests
MSEZ_RUN_PERF=1 pytest -q
```

---

## PHOENIX Smart Asset Operating System

The PHOENIX layer is an 18-module, 14,000+ line execution environment for autonomous Smart Assets. It is organized into six layers.

```
+-----------------------------------------------------------------------------+
|                      LAYER 5: INFRASTRUCTURE PATTERNS                       |
|  Circuit Breaker | Retry Policy | Event Bus | Event Sourcing | Caching     |
+-----------------------------------------------------------------------------+
|                           LAYER 4: OPERATIONS                               |
|  Health Framework | Observability | Configuration | CLI                     |
+-----------------------------------------------------------------------------+
|                         LAYER 3: NETWORK COORDINATION                       |
|  Watcher Economy | Security Layer | Hardening Layer                        |
+-----------------------------------------------------------------------------+
|                     LAYER 2: JURISDICTIONAL INFRASTRUCTURE                  |
|  Compliance Manifold | Migration Protocol | Corridor Bridge | L1 Anchor    |
+-----------------------------------------------------------------------------+
|                          LAYER 1: ASSET INTELLIGENCE                        |
|  Compliance Tensor | ZK Proof System | Smart Asset VM                      |
+=============================================================================+
|                              LAYER 0: KERNEL                                |
|  Phoenix Runtime | Lifecycle | Context Propagation | Metrics | DI/Services |
+-----------------------------------------------------------------------------+
```

Source: `tools/phoenix/` (18 Python modules) | Design: [ARCHITECTURE.md](./ARCHITECTURE.md) | Error codes: [ERRORS.md](./ERRORS.md)

---

## Schemas

The `schemas/` directory contains **116 JSON schemas** (targeting JSON Schema Draft 2020-12) defining the public API surface of the MSEZ Stack. These schemas cover:

- Verifiable Credentials (corridor anchors, lifecycle transitions, watcher bonds, dispute claims, arbitration awards)
- Corridor protocol (receipts, checkpoints, fork resolution, finality, routing)
- Arbitration lifecycle (claims, evidence packages, orders, settlements, enforcement)
- Agentic automation (triggers, policies, action schedules, audit trails)
- Artifacts (content-addressed references, graph verification reports)
- Module and profile validation

Browse schemas: [`schemas/`](../schemas/)

---

## API Specifications

OpenAPI 3.x specifications live in `apis/`:

| Spec | Description |
|------|-------------|
| [smart-assets.openapi.yaml](../apis/smart-assets.openapi.yaml) | Smart Asset CRUD, compliance evaluation, anchor verification |
| [corridor-state.openapi.yaml](../apis/corridor-state.openapi.yaml) | Corridor receipts, forks, anchors, finality |
| [mass-node.openapi.yaml](../apis/mass-node.openapi.yaml) | Zone-to-MASS integration |
| [regulator-console.openapi.yaml](../apis/regulator-console.openapi.yaml) | Regulator query access |

---

## Deployment

| Resource | Description |
|----------|-------------|
| [Docker Compose](../deploy/docker/) | 12-service compose stack with Prometheus monitoring |
| [AWS Terraform](../deploy/aws/terraform/) | VPC, EKS, RDS, KMS infrastructure (main.tf + kubernetes.tf) |
| [deploy-zone.sh](../deploy/scripts/deploy-zone.sh) | 7-step zone deployment script |

---

## Contributing

See [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines, code style requirements, and the pull request process.

## License

BUSL-1.1. See [LICENSE](../LICENSES) for the full license text.
