# Pre-0.5 Master Roadmap

This document is a **repo-native** roadmap for taking the stack from
"specification-complete" to "production pilot ready" before the `0.4.x → 0.5.0` bump.

It synthesizes:

- The MEZ modular evolution framework (dependency-aware modules, overlay semantics, upgrade choreography)
- The MEZ fortification blueprint (watcher economy, finality ladder, fork prevention, routing/economics)
- Smart Asset spec v0.4.0 + deep dive (Merkle-DAG state, migration sagas, custody/coordination, compliance manifold)
- The existing MEZ corridor state channel + lawpack pipeline already implemented in `0.4.x`

## Prime directive

**Every meaningful commitment in a VC/receipt must have a deterministic resolution path**
(`ArtifactRef` + CAS), and every operational invariant must be enforced by:

1. schema validation,
2. deterministic hashing,
3. cryptographic signatures,
4. and testable policies.

## Workstreams

### W0: Correctness & security hardening

**Goal:** unambiguous verification semantics under forks, adversarial inputs, and partial availability.

P0 deliverables:

- Receipt verification fork-tolerance + deterministic fork resolution policy (already shipped in `0.4.18+`; expand tests).
- Finality ladder fully enumerated + machine-computable (`mez corridor state finality-status`).
- Watcher quorum policies and gossip-friendly head commitments.
- **Commitment completeness (transitive)**: `--transitive-require-artifacts` ensures registry commits imply dependency availability.

Repo targets:

- `tools/mez.py` (verification policies, CLI flags)
- `schemas/corridor.*.schema.json` (fork, finality, lifecycle)
- `tests/` (P0 suites run in CI)

Unit tests:

- Fork resolution correctness (canonical chain selection)
- Fork alarm triggers lifecycle HALT
- Transitive artifact completeness for registry commits

### W1: Watcher economy + accountability

**Goal:** watchers are not just observers; they are accountable economic actors.

P0/P1 deliverables:

- Watcher bond VC (collateral evidence, operator identity, scope)
- Slashing conditions for equivocation and false attestations
- Reputation feed (VC-based, content-addressed)

Repo targets:

- `schemas/vc.watcher-bond.schema.json`
- `schemas/vc.watcher-slash.schema.json` (new)
- `tools/watcher_economy.py` (new; policy engine)

Unit tests:

- Watcher equivocation detection
- Sybil heuristics (authority-chain correlation)

### W2: Disputes & arbitration integration

**Goal:** disputes are first-class protocol objects, not off-chain improvisation.

P0/P1 deliverables:

- Dispute claim VC schema + CLI (`mez dispute file`)
- Arbitration award VC schema + CLI ingestion/verification
- Institution registry (LCIA/SIAC/ICC/etc) as authority chain branch

Repo targets:

- `schemas/vc.dispute-claim.schema.json`
- `schemas/vc.arbitration-award.schema.json`
- `tools/dispute.py` (new)

Unit tests:

- Unauthorized arbitrator rejection
- Award references correct corridor/lawpacks/checkpoints

### W3: Routing + corridor economics

**Goal:** corridors compose, discovery is deterministic, and fees are explicit.

P1 deliverables:

- Route discovery algorithm (policy + cost function)
- Quoted routes as signed artifacts
- Fee schedules and fee receipts (transition types)

Repo targets:

- `schemas/corridor.routing.schema.json`
- `tools/routing.py` (new)

Unit tests:

- Multi-hop route selection under constraints
- Fee correctness + replay resistance

### W4: Authority registry chaining

**Goal:** root-of-trust is explicit: treaty body → national → zone → operator.

P0/P1 deliverables:

- Authority registry VC types for each tier
- Delegation constraints (scopes, expiry, revocation)
- Registry snapshots in CAS (content addressed)

Repo targets:

- `schemas/vc.authority-registry.schema.json` (tiered)
- `tools/authority_registry.py` (policy + verification)

Unit tests:

- Chain validation (no cycle, correct scope)
- Revocation and key rotation semantics

### W5: Smart Asset integration

**Goal:** a smart asset is a Merkle-DAG state machine whose checkpoints can be committed into corridor receipts.

P0/P1 deliverables:

- Canonical Smart Asset state schema + checkpoint object
- Transition types for asset movements/migrations, with optional ZK proofs
- Migration saga state machine + evidence artifacts

Repo targets:

- `schemas/smart-asset.*.schema.json` (new)
- `apis/smart-assets.openapi.yaml` (scaffold)
- `registries/transition-types.lock.json` entries for asset transitions

Unit tests:

- (done v0.4.30) Asset checkpoint binds to receipt inclusion proof (`tests/test_smart_asset_anchor_verify.py`, `mez asset anchor-verify`)
- Migration saga happy-path + compensation-path

### W6: Mass Protocol integration

**Goal:** zone = Mass Entity, corridor = Mass Consent, receipts/attestations = Mass attestations.

P1 deliverables:

- Mapping doc + invariants (what must be equal, what may diverge)
- Mass identifier binding to DIDs and authority registry chain

Repo targets:

- `docs/architecture/MASS-INTEGRATION.md` (expand)
- `schemas/mass.*.schema.json` (new, minimal)

Unit tests:

- Mass ↔ DID binding semantics

### W7: Operational readiness

**Goal:** pilots survive reality: monitoring, HA, DoS controls, disaster recovery.

P0/P1 deliverables:

- Observability spec (metrics, logs, traces)
- HA reference architecture
- Disaster recovery runbook + forensic tooling

Repo targets:

- `docs/operators/INCIDENT-RESPONSE.md` (expand)
- `docs/operators/DISASTER-RECOVERY.md` (new)

Unit tests:

- Rate-limit policy tests (where applicable)
- Artifact availability drills (simulated data withholding)

## Roadmap closure checklist

The checklist is intentionally mechanical: each item specifies **file targets, CLI targets, and test gates**.

### P0 closure

- [ ] `--transitive-require-artifacts` implemented + tested
- [ ] Fork resolution policy coverage: `tests/test_fork_resolution_*.py`
- [ ] Lifecycle HALT/RESUME gates: `tests/test_lifecycle_*.py`
- [ ] OpenAPI sanity: parse all files under `apis/` in CI
- [ ] Minimum smart-asset schemas + OpenAPI scaffold committed

### P1 closure

- [ ] Watcher bond/slash economy schemas + policy engine + tests
- [ ] Dispute filing CLI + VC schemas + tests
- [ ] Routing schema + reference algorithm + tests
- [ ] Authority registry tiered chaining + tests

### P2 closure

- [ ] ZK transition proof hooks + circuit registry integration
- [ ] L1 anchoring ceremony automation + external chain adapters
- [ ] Performance targets validated (100k receipt chains; 1k watchers)
