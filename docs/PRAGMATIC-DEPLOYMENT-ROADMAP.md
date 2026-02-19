# Pragmatic Deployment Roadmap: AWS of Economic Zones

**Date:** 2026-02-19
**Codebase State:** v0.4.44-GENESIS | 4,073 tests | 0 failures | Clean build
**Commits:** 147 total | 16 Rust crates | ~74K lines production code

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
| P0-MIGRATION-001: Saga safety properties | PARTIAL | `0f54e69` (CAS + idempotency added) |
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
| OpenAPI specs | Scaffold-grade | Medium — blocks external integrator onboarding |
| Schema URI consistency | `momentum-ez.org` vs `schemas.momentum-ez.org` | Low — internal only for now |

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
| AWS Marketplace | Module registry (298 modules, 16 families) | Working |
| Region | Jurisdiction (with lawpack, regpack, licensepack) | 70+ defined |
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
| Watcher bond economics | Real bonding/slashing/reward implementation | Watcher economy is revenue model |
| Multi-zone orchestration | Kubernetes operator or Terraform module for N-zone deployment | Can't scale with docker-compose |
| Corridor health monitoring | Dashboard showing receipt chain height, checkpoint frequency, compliance state | Operators need visibility |
| Cross-zone compliance evaluation | Tensor evaluation that spans corridor endpoints | Proves bilateral compliance |

### Priority 5: Cryptographic Completion (Parallel Track)

**Goal**: Phase 4 crypto enables privacy-preserving compliance at scale.

| Task | When | Why |
|---|---|---|
| BBS+ selective disclosure | Before production KYC | Privacy-preserving credential presentation |
| Poseidon2 hash | Before ZK circuit activation | ZK-internal hashing |
| Real ZK backend (Groth16/Plonk) | Before AFC-tier finality | Settlement layer proofs |
| Real anchor target | Before cross-chain anchoring | L1 finality guarantees |

---

## IV. MASS SPEC ALIGNMENT ANALYSIS

### The Two-System Architecture

```
┌─────────────────────────────────────────────────┐
│              MEZ Stack (System B)                │
│  Jurisdictional context: compliance tensor,      │
│  corridors, lawpacks, regpacks, licensepacks    │
│                                                  │
│  Mass owns CRUD; MEZ Stack owns compliance       │
├─────────────────────────────────────────────────┤
│              Mass Platform (System A)            │
│  Five jurisdiction-agnostic primitives:          │
│  Entities | Ownership | Fiscal | Identity | Consent │
└─────────────────────────────────────────────────┘
```

### Mass Spec Concepts → MEZ Stack Implementation

| Mass Concept | MEZ Implementation | Alignment |
|---|---|---|
| Asset Harbor (vertex in JDG) | Zone (zone.yaml + deploy-zone.sh) | ALIGNED |
| Corridor (edge in JDG) | `mez-corridor` receipt chain + network protocol | ALIGNED |
| Smart Asset | Entities via `mez-mass-client` + VC layer | PARTIAL — no autonomous agent model yet |
| JDC (Jurisdictional DAG Consensus) | N/A — single-zone consensus only | GAP — multi-zone consensus not implemented |
| TLC (Treaty Lattice Consensus) | N/A — no lattice join operator | GAP — advanced consensus deferred |
| JVM (Jurisdictional Virtual Machine) | Pack Trilogy evaluation (static, not programmable) | GAP — no execution VM |
| LVC (Local Validity Certificate) | Checkpoint with proof | PARTIAL — proof signing exists, no cert format |
| EFC (Economic Finality Certificate) | N/A | GAP |
| AFC (Anchored Finality Certificate) | Mock anchor target | GAP |
| Asset Orbit Protocol | N/A | GAP — multi-harbor sharding not implemented |
| Lawpack (Akoma Ntoso) | `modules/legal/jurisdictions/*/src/akn/main.xml` | ALIGNED |
| Regpack (dynamic regulatory state) | `mez-pack/src/regpack.rs` with Pakistan content | ALIGNED |
| Corridor Aggregate Signatures | Standard Ed25519 (no aggregate scheme) | GAP — using individual sigs |
| Jurisdictional Vector Clocks | Wall-clock timestamps (`Utc::now()`) | GAP — no causal consistency protocol |
| Watcher Economy | Attestation model + bond types defined | PARTIAL — no real staking/slashing |

### Pragmatic Take on Mass Alignment

The Mass spec describes a complete distributed operating system for jurisdictional
execution. The MEZ Stack implements the **deployment and compliance infrastructure
layer** — which is the part that sovereign operators actually need first.

The advanced consensus protocols (JDC, TLC, JVM), finality certificate hierarchy,
and asset orbit model are architecturally sound in the spec but represent a
**multi-year engineering effort** that is not required for Phase 1-2 deployments.

What IS required, and what the codebase delivers:
- Zone deployment (Asset Harbor equivalent)
- Corridor receipt exchange (verifiable cross-zone transactions)
- Compliance tensor evaluation (jurisdictional context enforcement)
- Pack content (legal, regulatory, licensing data)
- Cryptographic integrity (receipts, checkpoints, signatures)

**Recommendation**: Do not chase full Mass spec parity. Instead, demonstrate the
value proposition through real sovereign deployments, then let the advanced
protocol features (JDC, TLC, JVM, Asset Orbits) be pulled by actual multi-zone
operational requirements rather than pushed by spec completeness.

---

## V. STRATEGIC DEPLOYMENT SEQUENCE

### Phase 1: Controlled Sandbox (NOW — Ready)

**Who**: Internal team + friendly sovereign partner (Pakistan SIFC)
**What**: Single-zone deployment with simulated Mass API, demonstrating compliance
tensor evaluation, receipt chain integrity, and zone manifest lifecycle.

Entry criteria already met:
- [x] Clean build, 4,073 tests passing
- [x] Default credentials eliminated
- [x] Mass API health gating
- [x] Receipt chain spec-conformant
- [x] Zone manifest + deploy script

Remaining:
- [ ] End-to-end demo script
- [ ] CAS digest computation for regpacks (currently zero-filled)

### Phase 2: Two-Zone Corridor Demo (Weeks 2-4)

**Who**: Pakistan SIFC ↔ UAE DIFC demo
**What**: Two zones deployed, corridor established, receipts exchanged,
compliance verified bilaterally.

Infrastructure exists (`docker-compose.two-zone.yaml`), needs:
- [ ] Real corridor establishment flow tested end-to-end
- [ ] Cross-zone compliance query
- [ ] Demo video / walkthrough for stakeholders

### Phase 3: Sovereign GovOS Pilot (Weeks 4-12)

**Who**: Pakistan government (FBR, NADRA, SBP, SECP)
**What**: Real transactions flowing through real institutional adapters,
with compliance tensor evaluated against real regulatory data.

Requires:
- [ ] National system HTTP adapters (FBR, NADRA, SBP, SECP)
- [ ] Zone operator dashboard (minimal web UI)
- [ ] Security review / pen test
- [ ] Key custody model (HSM/KMS rotation)

### Phase 4: Network Effect (Weeks 12-24)

**Who**: Multiple zone operators across jurisdictions
**What**: Corridor network demonstrating mutual recognition at scale.

Requires:
- [ ] Multi-zone Kubernetes orchestration
- [ ] Watcher bond economics
- [ ] BBS+ for privacy-preserving KYC
- [ ] Real ZK backend for settlement proofs

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
| Jurisdictions with real pack content | 5+ | 1 (Pakistan) with 70+ licensepack shells |
| Active corridors (receipt chain height > 0) | 10+ | 0 (testing only) |
| Tests passing | 100% | 100% (4,073/4,073) |
| Compliance domains with real evaluation | 20/20 | 8/20 (original 8 attested, 12 extended pending) |
| External integrators onboarded | 3+ | 0 (scaffold APIs) |
| Sovereign deployments live | 1+ | 0 (sandbox ready) |
