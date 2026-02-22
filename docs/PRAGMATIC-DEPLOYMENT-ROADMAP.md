# Pragmatic Deployment Roadmap: AWS of Economic Zones

**Codebase:** v0.4.44 | ~4,700 tests | 17 crates | ~164K lines Rust

---

## I. CURRENT REALITY ASSESSMENT

### What Exists and Works

The MEZ Stack has crossed a critical threshold from "spec document with code stubs"
to "functional prototype with production-grade internals." Here is what is real:

**Core Protocol (Production-Ready Internals)**

| Component | State | Evidence |
|---|---|---|
| Receipt chain (dual-commitment: hash-chain + MMR) | Implemented | `mez-corridor/src/receipt.rs` — 1,507 lines, golden vectors, adversarial tests |
| `compute_next_root()` with digest-set normalization | Implemented | Strips proof/next_root, dedup+sort, SHA256(MCF(payload)) |
| Fork resolution (evidence-driven, signed attestations) | Implemented | `mez-corridor/src/fork.rs` — 977 lines, equivocation detection, timestamp bounds |
| Compliance tensor (20 domains, fail-closed) | Implemented | `mez-tensor/src/evaluation.rs` — exhaustive match, Pending default for extended |
| Canonicalization (MCF = JCS + float rejection + datetime normalization) | Implemented | `mez-core/src/canonical.rs` |
| Ed25519 signatures, MMR, content addressing | Implemented | `mez-crypto/` |
| Schema validation (Draft 2020-12) | Implemented | `mez-schema/` with cached validators |
| Inter-zone corridor networking | Implemented | `mez-corridor/src/network.rs` — handshake, receipt exchange, replay protection |
| N-factorial corridor registry | Implemented | `registries/corridor-registry.yaml` — 99 zones, 4,851 derivable pairs |
| Zone manifest system | Implemented | `mez-pack/src/zone_manifest.rs` — 6 profiles, deploy scripts |
| Watcher attestation model | Implemented | `WatcherRegistry`, `create_attestation()`, signature verification |
| Pakistan lawpacks (civil, tax, financial, AML) | Implemented | `modules/legal/jurisdictions/pk/` — real legal content |
| Pakistan regpacks (sanctions, financial, SBP rates) | Implemented | `mez-pack/src/regpack.rs` — 1,300+ lines |
| 70+ jurisdiction licensepacks | Implemented | `mez-pack/src/licensepack/` |
| Verifiable Credentials (Smart Asset Registry VC) | Implemented | `mez-vc/` with Ed25519 JWK keygen |

**Deployment Infrastructure**

| Component | State | Evidence |
|---|---|---|
| Single-binary Axum HTTP server | Implemented | `mez-api/` — all 5 primitive APIs consolidated |
| Docker Compose (API + Postgres + Prometheus + Grafana) | Implemented | `deploy/docker/docker-compose.yaml` |
| Two-zone Docker Compose for corridor testing | Implemented | `deploy/docker/docker-compose.two-zone.yaml` |
| Zone deploy script with profile selection | Implemented | `deploy/scripts/deploy-zone.sh` |
| AWS Terraform (EKS + RDS + KMS) | Exists | `deploy/aws/terraform/` (not fully verified) |
| Health/readiness probes with Mass API gating | Implemented | Liveness + readiness endpoints |
| Secret injection (no default credentials) | Implemented | `${POSTGRES_PASSWORD:?must be set}` pattern |
| Mass API client with contract tests | Implemented | `mez-mass-client/` + OpenAPI snapshot validation |

**Audit Remediations Completed Since Pinned Commit**

| Finding | Status | Commit |
|---|---|---|
| P0-CORRIDOR-001: Receipt chain dual commitment | CLOSED | `9e38a24` |
| P0-CORRIDOR-002: compute_next_root() | CLOSED | `9e38a24` |
| P0-CORRIDOR-003: Schema-conformant receipt | CLOSED | `9e38a24` |
| P0-CORRIDOR-004: Schema-conformant checkpoint | CLOSED | `9e38a24` |
| P0-FORK-001: Evidence-driven fork resolution | CLOSED | `0f54e69` |
| P0-TENSOR-001: Fail-closed extended domains | CLOSED | `02b1984` |
| P0-DEPLOY-001: Default credentials eliminated | CLOSED | `02b1984` |
| P0-MIGRATION-001: Saga safety properties | CLOSED | `0f54e69` (CAS + idempotency + EffectExecutor + property tests) |
| P0-ZK-001: Fail-closed ZK policy | CLOSED | `0f54e69` |
| P0-CORRIDOR-NET-001: Inter-zone protocol | CLOSED | `6ea3f8e` |
| P0-PACK-001: Pakistan Pack Trilogy | CLOSED | `b996ecc` |
| P1-CLI-001: Evidence-gated transitions | CLOSED | `0f54e69` |
| P1-SCHEMA-001: CI schema validation | CLOSED | `49e177a` |
| P1-GOV-001: Deprecated v1 quarantined | CLOSED | `02b1984` |
| P1-PERF-001: Cached schema validators | CLOSED | `f69aee7` |

### What's Still Stubbed or Missing

| Component | Reality | Pragmatic Importance |
|---|---|---|
| Poseidon2 hash | Stub (`NotImplemented`) | Low for Phase 1-2 (SHA-256 suffices) |
| BBS+ selective disclosure | Stub (feature-gated) | Medium — needed for privacy-preserving KYC at scale |
| ZK proof system | Phase 1 mock (deterministic SHA-256) | Low for sandbox, HIGH for production compliance |
| Real anchor target (L1 finality) | Mock (`always confirmed`) | Medium — AFC tier not needed for LVC/EFC operations |
| Identity service (Mass-side) | Facade over other Mass services | HIGH for sovereign deployment |
| Pakistan national adapters | Types defined, no HTTP clients | HIGH for GovOS launch |
| OpenAPI specs | Contract-grade (60+ endpoints, security scheme, all schemas, trade/agentic/watcher routes) | LOW — promotion complete |
| Schema URI consistency | RESOLVED — all `$ref` values use full `schemas.momentum-ez.org` URIs | CLOSED |

---

## II. THE "AWS OF ECONOMIC ZONES" — WHAT IT ACTUALLY MEANS

### The Analogy Decoded

| AWS Concept | MEZ Stack Equivalent | Status |
|---|---|---|
| `aws ec2 run-instances` | `deploy-zone.sh <profile> <zone-id> <jurisdiction>` | Working |
| VPC | Zone (legal perimeter with compliance tensor) | Working |
| VPC Peering | Corridor (bilateral compliance-verified channel) | Working |
| IAM | Verifiable Credentials + Ed25519 key hierarchy | Working |
| CloudFormation | Zone manifest YAML + stack.lock | Working |
| S3 (content addressed) | CAS store (SHA-256 content digests) | Working |
| CloudWatch | Prometheus + Grafana observability stack | Working |
| AWS Marketplace | Module registry (323 modules, 16 families) | Working |
| Region | Jurisdiction (with lawpack, regpack, licensepack) | 210 defined |
| Availability Zone | Zone profile (6 types: financial center, trade hub, govos...) | Working |

### What Makes This Pragmatically Deployable Today

1. **One-command zone deployment**: `./deploy/scripts/deploy-zone.sh sovereign-govos org.momentum.mez.zone.pk-sifc pk`
2. **Corridor auto-bootstrapping**: N-factorial pairwise enumeration from zone catalog
3. **Two-zone integration testing**: `docker-compose.two-zone.yaml` validates cross-zone receipt exchange
4. **Profile-based configuration**: 6 zone types cover the known market segments
5. **Pack content for lead jurisdiction**: Pakistan has real lawpacks, regpacks, licensepacks

### What "Pragmatic Deployment" Requires Next

The gap between "working prototype" and "deployable Economic Zone" is not in the
cryptographic foundations (those are solid). It's in three categories:

---

## III. PRAGMATIC PRIORITY QUEUE

### Priority 1: Make the Demo Irrefutable (Weeks 1-2)

**Goal**: A sovereign operator can deploy a zone, observe a cross-border corridor
transaction with full receipt chain, and verify the compliance tensor evaluates
correctly — all in under 30 minutes.

| Task | What | Why |
|---|---|---|
| End-to-end demo script | Shell script: deploy 2 zones, establish corridor, send receipt, verify checkpoint, query compliance | Eliminates "does it actually work?" question |
| Zone bootstrap CLI | `mez zone init --jurisdiction pk --profile sovereign-govos` | Removes manual YAML editing |
| Pack content hash computation | `mez regpack build` must produce real CAS digests (currently zero-filled) | Zero digests in `zone.yaml` undermine CAS story |
| Corridor establishment walkthrough | CLI flow: `mez corridor propose` → `accept` → `activate` → `send-receipt` | Proves inter-zone protocol end-to-end |

### Priority 2: API Surface Hardening (Weeks 2-4)

**Goal**: An external integrator (e.g., a bank, a government IT team) can read the
API spec, call endpoints, and get back schema-conformant responses.

| Task | What | Why |
|---|---|---|
| OpenAPI spec promotion | Move from scaffold to contract: error models, auth, pagination, idempotency | Integrators need machine-readable contracts |
| Corridor API endpoints | Document and test: `/corridors/propose`, `/corridors/{id}/receipts`, `/corridors/{id}/checkpoint` | Core value prop must be API-accessible |
| Compliance query endpoint | `GET /compliance/{entity_id}` returns tensor state across all domains | Regulators need programmatic compliance visibility |
| Mass API contract pinning | Commit versioned Mass OpenAPI snapshots under `apis/mass/` | Blocks integration drift |

### Priority 3: Sovereign GovOS Reality (Weeks 3-8)

**Goal**: Pakistan GovOS pilot can process real-world transactions through the
compliance tensor with actual institutional data.

| Task | What | Why |
|---|---|---|
| FBR IRIS adapter | HTTP client for Pakistan tax authority integration | Tax-to-GDP is the business case |
| NADRA adapter | Identity verification against national database | KYC domain requires real attestation source |
| SBP Raast adapter | Payment system integration | Fiscal primitive requires real money movement |
| SECP adapter | License verification for securities/corporate domains | Licensing domain requires real data |
| GovOS experience portal | Minimal web UI for zone operator dashboard | Decision-makers need to see it, not read JSON |

### Priority 4: Corridor Network Scale (Weeks 4-12)

**Goal**: Multiple zones operating simultaneously with live corridor receipt exchange,
demonstrating the network effect thesis.

| Task | What | Why |
|---|---|---|
| Watcher bond economics | REST API: register, bond, activate, slash, rebond, unbond + 10 OpenAPI endpoints | **IMPLEMENTED** — full lifecycle with 4 slashing conditions |
| Multi-zone orchestration | Kubernetes operator or Terraform module for N-zone deployment | Can't scale with docker-compose |
| Corridor health monitoring | `GET /v1/corridors/health` — per-corridor status, receipt chain heights, peer count, watcher count | **IMPLEMENTED** — aggregated health endpoint |
| Cross-zone compliance evaluation | Tensor evaluation that spans corridor endpoints | **IMPLEMENTED** — `GET /v1/compliance/corridor/{id}` |

### Priority 5: Cryptographic Completion (Parallel Track)

**Goal**: Phase 4 crypto enables privacy-preserving compliance at scale.

| Task | When | Why |
|---|---|---|
| BBS+ selective disclosure | Before production KYC | Privacy-preserving credential presentation |
| Poseidon2 hash | Before ZK circuit activation | ZK-internal hashing |
| Real ZK backend (Groth16/Plonk) | Before AFC-tier finality | Settlement layer proofs |
| Real anchor target | Before cross-chain anchoring | L1 finality guarantees |

---

## IV. MASS SPEC ALIGNMENT: SOVEREIGN DEPLOYMENT AS THE PATH TO DECENTRALIZATION

### The Core Thesis

The MEZ Stack is not an "orchestration layer on top of Mass." It is the **deployment
substrate that progressively decentralizes Mass** through sovereign zone deployments.

```
Phase 1 (Today)          Phase 2 (Near-term)         Phase 3-4 (End-state)
┌──────────────┐         ┌──────────────┐            ┌──────────────┐
│   MEZ Zone   │         │   MEZ Zone   │            │   MEZ Zone   │
│ (compliance, │         │ (compliance, │            │ (compliance, │
│  corridors)  │         │  corridors)  │◄──corridor──►  corridors)  │
├──────────────┤         ├──────────────┤            ├──────────────┤
│ Centralized  │         │  Sovereign   │            │  Sovereign   │
│  Mass APIs   │         │  Mass APIs   │            │ Mass + Consensus│
│ (mass.inc)   │         │ (in-zone)    │            │ (JDC node)   │
└──────────────┘         └──────────────┘            └──────────────┘
     Single                 Per-zone                   Federated
    deployment             sovereign                  decentralized
```

Each sovereign zone deployment is a future Mass consensus node. Each corridor
receipt chain is a future DAG edge. The Mass spec's end-state emerges bottom-up
from the federation of sovereign deployments — not from building a monolithic L1.

### Mass Spec Concepts → MEZ Stack Precursors → Evolution Path

| Mass Spec Concept | MEZ Stack Precursor (Today) | Evolution Path |
|---|---|---|
| Asset Harbor | Zone deployment (zone.yaml + deploy-zone.sh) | Zone running sovereign Mass APIs = Harbor |
| JDC (Jurisdictional DAG Consensus) | Corridor receipt chain (bilateral) | N-zone corridor mesh → DAG topology |
| TLC (Treaty Lattice Consensus) | CorridorDefinitionVC + compliance tensor | Treaty = corridor agreement; lattice = N-factorial graph |
| JVM (Jurisdictional Virtual Machine) | Pack Trilogy evaluation (static) | Static packs → programmable evaluation → JVM |
| LVC (Local Validity Certificate) | Checkpoint with proof | Checkpoint + proof = LVC |
| EFC (Economic Finality Certificate) | Cross-corridor settlement anchoring | Settlement corridor binding → EFC |
| AFC (Anchored Finality Certificate) | Anchor target interface (mock) | Real L1 anchor → AFC |
| Asset Orbit Protocol | Single-zone asset management | Multi-zone corridor tracking → orbits |
| Corridor Aggregate Signatures | Individual Ed25519 | Aggregate signature scheme → CAS |
| Jurisdictional Vector Clocks | Wall-clock + corridor sequence numbers | Causal ordering → vector clocks |
| Watcher Economy | Attestation model + bond types | Real staking/slashing + bond economics |
| Lawpack (Akoma Ntoso) | `modules/legal/jurisdictions/*/src/akn/` | ALIGNED — content pipeline operational |
| Regpack (dynamic regulatory state) | `mez-pack/src/regpack.rs` | ALIGNED — Pakistan content exists |

### Strategic Implication

The advanced protocol features (JDC, TLC, JVM, Asset Orbits) are not items to
"implement someday." They **emerge** from sovereign deployments:

- Deploy 1 sovereign zone → you have a Harbor
- Connect 2 sovereign zones via corridor → you have the first DAG edge
- Add watcher attestations → you have validator votes
- Add N zones → you have Jurisdictional DAG Consensus
- Add corridor treaty agreements → you have Treaty Lattice Consensus

**The MEZ Stack IS the Mass Protocol delivery vehicle.** Every sovereign deployment
moves the system closer to the spec's end-state. This is why the work we do today
on zone deployment, corridor protocol, and pack content directly serves the
decentralized execution vision — it's not a detour, it's the path.

---

## V. STRATEGIC DEPLOYMENT SEQUENCE

### Phase 1: Controlled Sandbox (NOW — Ready)

**Who**: Internal team + friendly sovereign partner (Pakistan SIFC)
**What**: Single-zone deployment with MEZ Stack pointing to centralized Mass APIs,
demonstrating compliance tensor, receipt chain, and zone manifest lifecycle.

Entry criteria met:
- [x] Clean build, 4,666+ tests passing
- [x] Default credentials eliminated
- [x] Mass API health gating
- [x] Receipt chain spec-conformant
- [x] Zone manifest + deploy script
- [x] End-to-end demo script (`scripts/e2e-corridor-demo.sh` — 30+ assertions, full lifecycle)
- [x] CAS digest computation for regpacks (real SHA-256 digests via `mez regpack build`)

### Phase 2: Two Sovereign Zones, One Corridor (Weeks 2-6)

**Who**: Pakistan SIFC ↔ UAE DIFC
**What**: Two zones deployed, each with **sovereign Mass API instances** running
inside the zone's infrastructure. Corridor established between them. Receipts
exchanged and verified bilaterally. This is the first step toward decentralized Mass.

Infrastructure exists (`docker-compose.two-zone.yaml`), needs:
- [x] Containerized Mass API services deployable per-zone (`mez-mass-stub` crate + `Dockerfile.mass-stub`)
- [x] Real corridor establishment flow tested end-to-end (receipt chain API + checkpoint endpoints + E2E integration tests)
- [x] Cross-zone compliance query (`GET /v1/compliance/corridor/{id}` — bilateral tensor evaluation)
- [x] Demo proving sovereign data residency (`sovereign_mass_test.rs` — Zone A's data never leaves Zone A)

### Phase 3: Sovereign GovOS Pilot (Weeks 4-12)

**Who**: Pakistan government (FBR, NADRA, SBP, SECP)
**What**: Real transactions flowing through real institutional adapters,
with sovereign Mass APIs running inside Pakistan's infrastructure,
compliance tensor evaluated against real Pakistani regulatory data.

Requires:
- [ ] National system HTTP adapters (FBR, NADRA, SBP, SECP)
- [ ] Sovereign Mass API deployment within Pakistan infrastructure
- [ ] Zone operator dashboard (minimal web UI)
- [ ] Security review / pen test
- [ ] Key custody model (HSM/KMS rotation — zone operator controls keys)

### Phase 4: Corridor Mesh → Embryonic JDC (Weeks 12-24)

**Who**: Multiple zone operators across jurisdictions
**What**: 5+ sovereign zones with N-factorial corridor mesh. Each zone runs
its own Mass APIs. The corridor mesh IS the Jurisdictional DAG in practice.

Requires:
- [ ] Multi-zone Kubernetes orchestration (N sovereign Mass deployments)
- [x] Watcher bond economics (real staking/slashing) — REST API with full lifecycle + 4 slashing conditions
- [x] Corridor health monitoring — aggregated health endpoint with per-corridor status
- [ ] BBS+ for privacy-preserving KYC across corridors
- [ ] Real ZK backend for settlement proofs
- [ ] The federation of sovereign zones → this is Mass Protocol emerging from the ground up

---

## VI. ARCHITECTURE DECISION RECORDS

### ADR-001: Single Binary Over Microservices
**Decision**: All 5 primitive APIs + corridor + compliance in one Axum binary.
**Rationale**: Eliminated the 12-service Python deployment that couldn't start.
Reduces operational complexity for sovereign operators who are not SRE teams.
**Consequence**: Horizontal scaling requires stateless API design (which exists).

### ADR-002: MCF Over Pure JCS
**Decision**: Momentum Canonical Form = RFC 8785 JCS + float rejection + datetime normalization.
**Rationale**: Pure JCS allows floats and doesn't normalize timestamps, creating
cross-implementation divergence risk. MCF trades strict JCS compliance for
determinism guarantees.
**Consequence**: Must document MCF as a normative extension. External verifiers
need MCF library, not just JCS library.

### ADR-003: SHA-256 First, Poseidon2 Later
**Decision**: All production digests use SHA-256. Poseidon2 is feature-gated stub.
**Rationale**: SHA-256 is universally available, audited, and sufficient for
LVC/EFC-tier operations. Poseidon2 is only needed inside ZK circuits.
**Consequence**: Phase 4 ZK activation requires Poseidon2 but not before.

### ADR-004: Pack Content as Data, Not Code
**Decision**: Lawpacks, regpacks, licensepacks are declarative data artifacts,
not executable programs.
**Rationale**: Sovereign operators update law by updating data (regpack refresh),
not by deploying code. This is the "regulatory agility" value proposition.
**Consequence**: The JVM (Jurisdictional Virtual Machine) from the Mass spec is
deferred — static evaluation covers Phase 1-3 requirements.

### ADR-005: Zone Profiles as Deployment Templates
**Decision**: 6 zone profiles (digital-financial-center, trade-hub, tech-park,
sovereign-govos, charter-city, digital-native-free-zone) as templates.
**Rationale**: "AWS of EZ" means customers pick a template and deploy, not
write configuration from scratch.
**Consequence**: Each profile must have a tested deploy path.

### ADR-006: Sovereign Mass Deployment as Path to Decentralization
**Decision**: The MEZ Stack deploys sovereign Mass API instances per-zone rather
than permanently proxying to centralized Mass.
**Rationale**: The Mass spec describes a decentralized execution layer. Building
a monolithic L1 is a multi-year effort with high risk. Instead, each sovereign
zone deployment IS a future consensus node. The corridor protocol between zones
IS the embryonic Jurisdictional DAG. Decentralization emerges bottom-up from
sovereign deployments, not top-down from protocol design.
**Consequence**: Zone deploy tooling must support containerized Mass API services.
`mez-mass-client` must abstract over both centralized and sovereign Mass endpoints.
Key custody, data residency, and operational authority belong to the zone operator.

---

## VII. RISK REGISTER

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Mass API breaks without notice | High | Zone bootstrap fails | Contract tests exist; pin Mass OpenAPI snapshots |
| Pakistan national system APIs unavailable | High | GovOS pilot blocked | Build adapter interfaces now; mock implementations until APIs available |
| ZK mock proofs accepted in production | Medium | Compliance bypass | `ProofPolicy` fail-closed exists; CI gate for release builds |
| Single-binary scaling ceiling | Low | Performance under load | Stateless design; horizontal scale path exists |
| Competitor ships faster | Medium | Market position | Focus on sovereign relationships (sticky); tech moat is pack content |
| Regulatory environment changes | Medium | Pack content stale | Regpack refresh pipeline (daily/hourly) handles this |

---

## VIII. METRICS THAT MATTER

For the "AWS of EZ" thesis to be proven, track:

| Metric | Target | Current |
|---|---|---|
| Time to deploy a zone | < 30 minutes | ~1 hour (manual YAML + docker-compose) |
| Jurisdictions with real pack content | 5+ | 5 (Pakistan, UAE, Singapore, Hong Kong, Cayman) + 70+ licensepack shells |
| Active corridors (receipt chain height > 0) | 10+ | 0 (testing only) |
| Tests passing | 100% | 100% (4,683+/4,683+) |
| Compliance domains with real evaluation | 20/20 | 8/20 (original 8 attested, 12 extended pending) |
| External integrators onboarded | 3+ | 0 (scaffold APIs) |
| Sovereign deployments live | 1+ | 0 (sandbox ready) |
