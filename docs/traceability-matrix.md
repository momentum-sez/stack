# Spec-to-crate traceability matrix

Maps each specification chapter in `spec/` to its Rust implementation in `mez/crates/`. This is the authoritative reference for understanding which code implements which spec requirements.

Generated from the v0.4.44 GENESIS codebase. The workspace contains 17 crates (164K lines, 4,683 tests).

---

## Spec chapter -> implementation

| Chapter | Title | Crate(s) | Status |
|---------|-------|----------|--------|
| `00-terminology` | Terminology | `mez-core` | Implemented |
| `01-mission` | Mission | N/A | Non-normative |
| `02-invariants` | Protocol Invariants | `mez-core` | Implemented |
| `03-standard-structure` | Standard Document Structure | `mez-cli` | Implemented |
| `04-design-rubric` | Design Decision Criteria | N/A | Non-normative |
| `10-repo-layout` | Repository Structure | `mez-cli` | Implemented |
| `11-architecture-overview` | System Architecture | `mez-core`, `mez-api` | Implemented |
| `12-mass-primitives-mapping` | Mass Five Primitives | `mez-api`, `mez-mass-client` | Implemented |
| `17-agentic` | Agentic Policy Automation | `mez-agentic` | Implemented |
| `20-module-system` | Module Composition | `mez-schema`, `mez-pack` | Implemented |
| `22-templating-and-overlays` | Templates and Overlays | `mez-pack` | Partial |
| `30-profile-system` | Zone Profile Definitions | `mez-schema`, `mez-cli` | Implemented |
| `40-corridors` | Cross-Border Corridors | `mez-corridor`, `mez-state` | Implemented |
| `41-nodes` | Node Architecture | `mez-api` | Partial |
| `50-conformance` | Conformance Testing | `mez-integration-tests` | Implemented |
| `60-governance` | Governance Structures | `mez-state` | Implemented |
| `61-network-diffusion` | Network Propagation | -- | **Not implemented** |
| `71-regulator-console` | Regulator Query Interface | `mez-api` | Implemented |
| `80-security-privacy` | Security Model and Privacy | `mez-crypto`, `mez-zkp` | Partial (ZK mocked) |
| `90-provenance` | Provenance Tracking | `mez-crypto` | Implemented |
| `95-lockfile` | Stack Lockfile | `mez-cli`, `mez-pack` | Implemented |
| `96-lawpacks` | Lawpack System | `mez-pack` | Implemented |
| `97-artifacts` | Content-Addressed Storage | `mez-crypto` | Implemented |
| `98-licensepacks` | Licensepack System | `mez-pack`, `mez-state` | Implemented |

---

## Crate -> spec reverse index

| Crate | Primary Chapters | Purpose |
|-------|-----------------|---------|
| `mez-core` | 00, 02 | Canonical serialization, content digests, 20 compliance domains, identifier newtypes |
| `mez-crypto` | 80, 90, 97 | Ed25519, MMR, CAS, SHA-256 |
| `mez-vc` | 12 | W3C Verifiable Credentials, Ed25519 proofs |
| `mez-state` | 40, 60, 98 | Typestate machines: corridor (6), entity (10), migration (9), license (5), watcher (4) |
| `mez-tensor` | 14 | Compliance Tensor V2, Dijkstra manifold, Merkle commitments |
| `mez-zkp` | 80 | Sealed ProofSystem trait, 12 circuit types, CDB bridge |
| `mez-pack` | 96, 98, 20 | Lawpack, regpack, licensepack processing |
| `mez-corridor` | 40 | Receipt chain (MMR), fork resolution, netting, SWIFT, anchoring |
| `mez-agentic` | 17 | 20 trigger types, deterministic evaluation (Theorem 17.1), audit trail |
| `mez-arbitration` | 21 | 7-phase dispute lifecycle, evidence, escrow, enforcement |
| `mez-compliance` | -- | Regpack -> tensor bridge (jurisdiction configuration) |
| `mez-schema` | 20 | JSON Schema validation (Draft 2020-12), 116 schemas |
| `mez-mass-client` | 12 | Typed HTTP client for 5 Mass API primitives |
| `mez-api` | 12, 40, 71 | Axum HTTP server, 8 route modules, auth, rate limiting |
| `mez-cli` | 03, 10 | CLI: validate, lock, corridor, artifact, vc |
| `mez-integration-tests` | 50 | 113 cross-crate test files |

---

## Gaps

### Spec chapters without Rust implementation

| Chapter | Title | Gap | Priority |
|---------|-------|-----|----------|
| `61-network-diffusion` | Network Propagation | No gossip, peer discovery, or receipt propagation | High |

### Partial implementations

| Area | Status | Gap |
|------|--------|-----|
| ZK Proofs | `MockProofSystem` (Phase 1) | Groth16/PLONK backends feature-gated but not implemented |
| Poseidon2 | Stubbed (`poseidon2` flag) | ZK-friendly hashing for CDB |
| BBS+ | Stubbed (`bbs-plus` flag) | Selective disclosure for privacy-preserving compliance |
| Templates | `mez-pack` YAML parsing | Full template engine not in Rust |
| Node architecture | K8s probes in `mez-api` | Full node discovery/gossip not implemented |

---

## Dependency graph

```
mez-core (foundation — no internal deps)
  │
  ├── mez-crypto (Ed25519, MMR, CAS)
  │     ├── mez-vc (Verifiable Credentials)
  │     │     └── mez-schema (Schema validation)
  │     ├── mez-zkp (Zero-knowledge proofs)
  │     └── mez-tensor (Compliance tensor)
  │
  ├── mez-state (Typestate machines)
  │     ├── mez-corridor (Cross-border operations)
  │     └── mez-arbitration (Dispute resolution)
  │
  ├── mez-pack (Lawpack, Regpack, Licensepack)
  ├── mez-agentic (Policy engine)
  ├── mez-compliance (Regpack -> tensor bridge)
  ├── mez-mass-client (Mass HTTP client)
  │
  ├── mez-api (Axum server — composition root)
  ├── mez-cli (CLI binary)
  └── mez-integration-tests (all crates via dev-deps)
```
