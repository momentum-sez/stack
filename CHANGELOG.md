# CHANGELOG

## [0.4.44] - 2026-02-03 - GENESIS

### Codename: GENESIS
**"The zone is born."**

This release transforms the MSEZ Stack from infrastructure into a fully forkable, deployable Special Economic Zone. v0.4.44 introduces three new module families (Corporate Services, Identity, Tax & Revenue), the Licensepack specification completing the "pack trilogy," and one-click deployment automation.

### Major Features

#### Licensepack Specification (spec/98-licensepacks.md)
Content-addressed snapshots of jurisdictional licensing state, completing the pack trilogy alongside lawpacks (static law) and regpacks (dynamic guidance). Licensepacks capture live licensing registries — who holds what licenses, under what conditions, with what permissions and restrictions.

Key capabilities include cryptographic digest computation for content addressing, license record schemas (conditions, permissions, restrictions), holder profile management with UBO integration, compliance tensor LICENSING domain integration, and verifiable credential export for license attestations.

The `tools/licensepack.py` module (900+ lines) provides LicensePack class with full CRUD operations, license verification for compliance tensor integration, delta computation between snapshots, and CLI commands for fetch, verify, lock, query, and export-vc operations.

#### Corporate Services Module Family (modules/corporate/)
Eight modules implementing full CSP (Corporate Service Provider) lifecycle:

The `formation` module provides entity incorporation workflows with JSON Schema forms for LLC, Corporation, Partnership, Trust, and DAO formation. Supports share capital definition, director appointments, UBO declarations, and jurisdiction-specific requirements.

The `beneficial-ownership` module implements UBO registry with verification chains, supporting FATF-compliant beneficial ownership declarations, ownership chain tracking, PEP and sanctions screening integration.

The `cap-table` module delivers complete capitalization table management including share classes with rights definitions, shareholder holdings with vesting schedules, option pools and grants, convertible instruments (SAFEs, notes), and warrants.

The `secretarial` module provides corporate secretarial services with 10+ board resolution templates, meeting minutes, annual returns, and director change filings.

The `annual-compliance` module implements compliance calendar automation with filing deadline tracking, reminder schedules, late penalty calculations, and event-triggered filings.

The `dissolution` module delivers voluntary dissolution workflows with 10-stage state machine from board resolution through final dissolution certificate.

The `registered-agent` module provides registered office and agent appointment services.

The `governance-templates` module delivers corporate governance document templates including articles of association, shareholder agreements, and director service agreements.

#### Identity & Credentialing Module Family (modules/identity/)
Six modules implementing the MASS Protocol Identity primitive with progressive verification tiers:

The `core` module provides DID management with did:key and did:web support, W3C DID Document schema, key lifecycle management, and multi-key authentication.

The `kyc-tiers` module implements 4-tier progressive KYC:
- Tier 0 (Pseudonymous): DID only, $1,000 transaction limit
- Tier 1 (Basic): Government ID + selfie, $15,000 limit
- Tier 2 (Enhanced): Address + source of funds, $100,000 limit
- Tier 3 (Institutional): Full due diligence, unlimited

Each tier includes detailed verification workflows, document requirements, screening checks, and capability unlocks.

The `credentials` module provides verifiable credential issuance with selective disclosure and ZK proof support.

The `binding` module enables entity-identity-instrument linkage for corporate identity management.

#### Tax & Revenue Module Family (modules/tax/)
Seven modules implementing zone fiscal infrastructure:

The `framework` module defines configurable tax regimes supporting tax-free zones, low-tax zones, and standard taxation with corporate income tax, personal income tax, withholding taxes, VAT/GST, stamp duty, and economic substance requirements.

The `fee-schedules` module delivers comprehensive zone fee catalogs including formation fees ($1,000-$2,500), annual fees (tiered by revenue), license fees (by license type), filing fees, service fees, and penalty structures.

The `incentive-programs` module provides 6 tax incentive programs: new establishment holiday (5-10 year tax exemption), innovation/R&D credit (25-35%), employment grants ($5k-$50k per hire), training subsidies (50%), headquarters incentive (composite), and green investment credit (30%).

The `reporting` module implements international tax reporting with CRS (Common Reporting Standard) and FATCA templates, including due diligence procedures, XML schema integration, and compliance calendars.

#### Deployment Automation (deploy/)
One-click zone deployment with Docker Compose:

`deploy/docker/docker-compose.yaml` orchestrates 12 services:
- Core: zone-authority, entity-registry, license-registry
- Corridor: corridor-node, watcher
- Identity: identity-service
- Financial: settlement-service
- Compliance: compliance-service, regulator-console
- Infrastructure: PostgreSQL, Redis
- Observability: Prometheus, Grafana

`deploy/docker/Dockerfile.*` provides optimized container images for each service type.

`deploy/docker/init-db.sql` initializes all required databases with schemas for entities, licensing, identity, compliance, corridors, watchers, and settlement.

`deploy/scripts/deploy-zone.sh` provides one-command deployment:
```bash
./deploy-zone.sh digital-financial-center my-zone ae-dubai-difc
```

### Schema Updates

#### zone.schema.json
Added `licensepack_domains` array for specifying license domains to pin, `licensepack_refresh_policy` object for per-domain refresh configuration, and `regpack_domains` array for regulatory pack domains.

#### stack.lock.schema.json
Added `licensepacks` array with full licensepack pinning support including digest, lock path, artifact path, snapshot timestamp, and content summary. Added `regpacks` array with parallel structure for regulatory packs.

### New Schemas

- `licensepack.schema.json` — Main licensepack structure
- `licensepack.license.schema.json` — Individual license records
- `licensepack.lock.schema.json` — Licensepack lock file format

### Module Count

| Category | v0.4.43 | v0.4.44 | Delta |
|----------|---------|---------|-------|
| Legal | 13 | 13 | - |
| Regulatory | 8 | 8 | - |
| Licensing | 11 | 11 | - |
| Financial | 10 | 10 | - |
| Corridors | 5 | 5 | - |
| Governance | 9 | 9 | - |
| Operational | 9 | 9 | - |
| **Corporate** | 0 | **8** | +8 |
| **Identity** | 0 | **6** | +6 |
| **Tax** | 0 | **7** | +7 |
| **TOTAL** | 65 | **86** | +21 |

### Breaking Changes

None. All new modules are additive. Existing zone manifests continue to work without modification.

### Migration Guide

To adopt new module families, add them to your profile:

```yaml
modules:
  # Existing modules...

  # Add Corporate Services
  - module_id: org.momentum.msez.corporate
    version: 0.1.0
    variant: baseline

  # Add Identity
  - module_id: org.momentum.msez.identity
    version: 0.1.0
    variant: baseline

  # Add Tax & Revenue
  - module_id: org.momentum.msez.tax
    version: 0.1.0
    variant: tax-free-zone
```

To enable licensepacks, add to zone.yaml:

```yaml
licensepack_domains:
  - financial
  - corporate

licensepack_refresh_policy:
  default:
    refresh_frequency: daily
    max_staleness_hours: 24
```

### Future Work

Version 0.4.45 will target Capital Markets module family (securities issuance, CSD, clearing, DVP/PVP).

Version 0.4.46 will focus on Trade & Commerce module family (letters of credit, trade documents, supply chain finance).

Version 0.4.47 will deliver MASS Five Primitives (Entities, Ownership, Instruments, Identity, Consent).

### Contributors

Engineering team at Momentum (engineering@momentum.inc)

---

## [0.4.43] - 2026-01-29 - PHOENIX ASCENSION

### Codename: PHOENIX ASCENSION

This release transforms the MSEZ Stack into an elite-tier Smart Asset Operating System with comprehensive security hardening, a novel virtual machine for decentralized network interaction, and defense-in-depth protection against economic attacks, replay attacks, and front-running.

### Major Features

#### Smart Asset Virtual Machine (SAVM)
A stack-based execution environment for deterministic Smart Asset operations across the decentralized MSEZ network. The VM provides gas-metered execution with compliance and migration coprocessors.

The VM architecture includes a 256-slot stack with 256-bit words, 64KB expandable memory, Merkleized persistent storage, compliance coprocessor for tensor operations, migration coprocessor for cross-jurisdictional transfers, and 10 instruction categories covering stack operations, arithmetic, comparison, memory, storage, control flow, context, compliance, migration, and cryptography.

Key capabilities include deterministic execution for consensus, gas metering for DoS prevention, pre-scanned jump destination validation, overflow-safe 256-bit arithmetic with modular wrapping, and two's complement signed number support.

#### Security Hardening Layer
Comprehensive defense-in-depth security addressing all identified vulnerabilities including attestation replay prevention through scope binding, TOCTOU protection via versioned state with compare-and-swap, front-running prevention through time-locked operations, signature verification infrastructure, nonce management for replay prevention, rate limiting for DoS protection, and tamper-evident audit logging with hash chains.

#### Validation Framework
Production-grade input validation with sanitization covering string validation with pattern matching, digest validation (SHA256 64-char hex), address validation (Ethereum format), amount validation with Decimal precision, timestamp validation with freshness checks, and bytes validation with hex decoding.

#### Thread Safety Infrastructure
Concurrency primitives for multi-threaded environments including ThreadSafeDict with RLock protection, AtomicCounter with Lock-based increment/decrement, VersionedStore for optimistic locking, and atomic decorator for function-level locking.

#### Economic Attack Prevention
Guards against malicious economic behavior through attestation value limits (10x collateral maximum), minimum collateral requirements ($1000 USD), slash rate limits per epoch (50% maximum), and whale concentration detection (33% maximum stake).

#### Time-Locked Operations
Front-running prevention through commit-delay-reveal pattern with configurable delays (7 days for withdrawals, 1 day for migrations, 3 days for parameter changes), operation commitment before reveal, expiration handling, and cancellation support.

#### Audit Logging System
Tamper-evident forensic trail with hash chain linking to previous events, event type categorization, actor and resource tracking, detailed metadata capture, chain integrity verification, and queryable event history.

### New Modules

The `hardening.py` module (740 lines) provides ValidationError and ValidationErrors exception types, Validators class with comprehensive input validation, CryptoUtils with secure comparison and Merkle operations, ThreadSafeDict and AtomicCounter concurrency primitives, InvariantChecker for state machine enforcement, EconomicGuard for attack prevention, and RateLimiter with token bucket algorithm.

The `security.py` module (670 lines) provides AttestationScope for scope binding, ScopedAttestation with cryptographic commitment, NonceRegistry for replay prevention, VersionedStore with compare-and-swap, TimeLock and TimeLockManager for front-running prevention, SignatureVerifier infrastructure, AuditLogger with tamper-evident chain, and SecureWithdrawalManager with time locks.

The `vm.py` module (900 lines) provides OpCode enum with 60+ instructions, Word class for 256-bit arithmetic, ExecutionContext for caller/jurisdiction context, VMState with stack/memory/storage, GasCosts for operation pricing, ComplianceCoprocessor for tensor integration, MigrationCoprocessor for lock/transit/settle, SmartAssetVM execution engine, and Assembler/disassembler utilities.

### Test Coverage

The test suite now includes 92 tests covering all PHOENIX components. New test classes include TestHardeningModule (8 tests) covering string, digest, amount, timestamp validation, thread-safe collections, atomic counter, Merkle proofs, and economic guards. TestSecurityModule (7 tests) covers attestation scope, scoped attestations, nonce registry, versioned store CAS, time lock manager, audit logger, and secure withdrawal. TestVMModule (10 tests) covers word arithmetic, overflow handling, negative numbers, basic execution, arithmetic operations, stack overflow, out of gas, invalid jump, compliance coprocessor, migration coprocessor, and assembler. TestIntegratedSecurity (1 test) covers end-to-end secure migration with all security features.

### Security Vulnerabilities Addressed

Attestation Replay Prevention: Attestations are now scope-bound to specific (asset, jurisdiction, domain) tuples with time validity windows. The scope commitment cryptographically binds the attestation to its intended context.

TOCTOU Protection: Critical state is now managed through VersionedStore with compare-and-swap operations, preventing race conditions between check and use.

Front-Running Prevention: Withdrawals and other sensitive operations now require a 7-day time lock, giving other participants time to respond to announced operations.

Signature Verification: Infrastructure now exists for cryptographic signature verification with nonce freshness checks and timestamp validation.

Economic Attack Prevention: Attestation values are capped at 10x bond collateral, preventing under-collateralized attestations that could enable attacks.

### Architecture Principles

The PHOENIX ASCENSION release follows eight core design principles. Fail-Safe Defaults ensure unknown compliance states default to non-compliant, ensuring security even with incomplete information. Cryptographic Integrity ensures every state transition produces verifiable proof through tensor commitments, attestation digests, and Merkle proofs. Atomic Operations ensure migrations either complete fully or compensate, with no partial states left in the system. Economic Accountability requires watchers to stake collateral for their attestations, creating skin-in-the-game incentives for honest behavior. Privacy by Design enables ZK proofs for compliance verification without disclosing sensitive transaction details. Defense in Depth implements multiple layers of security protection. Zero Trust verifies all inputs regardless of source. Deterministic Execution ensures VM operations produce identical results across all nodes.

### Breaking Changes

None. The PHOENIX modules are additive and do not modify existing APIs.

### Performance Characteristics

VM execution averages under 1ms for simple programs with gas costs calibrated for DoS prevention. Merkle root computation is O(n log n) for n leaves. Nonce registry supports O(1) lookup with periodic cleanup. Audit log verification is O(n) for chain integrity check.

### Known Limitations

The ZK circuits use mock implementations and production deployments should integrate with Gnark, Circom, or similar ZK backends. L1 anchoring uses mock chain adapters and production deployments require integration with ethers.js, web3.py, or chain-specific SDKs. Signature verification is mock implementation and production should use PyNaCl for Ed25519, ecdsa for secp256k1.

### Future Work

Version 0.4.44 will target production ZK backend integration with Gnark. Version 0.4.45 will focus on production L1 anchoring with ethers.js. Version 0.4.46 will deliver production signature verification with PyNaCl.

### Contributors

Engineering team at Momentum (engineering@momentum.inc)

### Major Features

#### Compliance Tensor Implementation
The mathematical core of Smart Asset autonomy. The Compliance Tensor represents multi-dimensional 
compliance state across jurisdictions and domains as a single cryptographically committable object.

Key capabilities include tensor algebra operations (meet, join), slicing along any dimension 
(asset, jurisdiction, domain, time), deterministic Merkle commitment generation, and selective 
disclosure proofs for specific coordinates.

The tensor follows a strict lattice for state composition: COMPLIANT ⊔ PENDING = PENDING, and 
any state meet NON_COMPLIANT = NON_COMPLIANT (absorbing). This ensures fail-safe defaults where 
unknown states are treated as non-compliant.

#### ZK Proof Infrastructure
Privacy-preserving compliance verification without disclosure. The infrastructure includes a 
content-addressed circuit registry, support for Groth16, PLONK, and STARK proof systems, and 
pre-built circuits for balance sufficiency, sanctions clearance, KYC attestation, tax compliance, 
and compliance tensor inclusion proofs.

Mock prover and verifier implementations enable testing without cryptographic backends, while the 
architecture supports future integration with production ZK backends.

#### Compliance Manifold
Path planning through the jurisdictional landscape. The manifold computes optimal migration 
routes between jurisdictions using Dijkstra's algorithm with compliance-aware edge weights.

Features include multi-hop path discovery, attestation gap analysis, cost and time estimation, 
path constraint support (max hops, excluded jurisdictions, fee limits), and alternative path 
finding for resilience.

#### Migration Protocol
Saga-based state machine for cross-jurisdictional Smart Asset migration. The protocol follows 
a strict state progression: INITIATED → COMPLIANCE_CHECK → ATTESTATION_GATHERING → SOURCE_LOCK 
→ TRANSIT → DESTINATION_VERIFICATION → DESTINATION_UNLOCK → COMPLETED.

Compensation paths handle failures at any stage, with automatic rollback of partial progress. 
The protocol collects comprehensive evidence bundles for regulatory audit, including compliance 
tensor snapshots, attestations, receipts, and settlement records.

#### Watcher Economy
Economic accountability infrastructure for corridor watchers. Watchers stake collateral bonds 
proportional to attested transaction volume, with slashing conditions for equivocation (100%), 
availability failure (1%), false attestation (50%), and collusion (100% + permanent ban).

The reputation system tracks availability scores, accuracy scores, and tenure bonuses, 
determining which corridors watchers can participate in and their fee tier.

#### L1 Anchor Layer
Settlement finality through Ethereum and L2 checkpointing. Supports Ethereum mainnet, Arbitrum, 
Base, and Polygon with chain-specific adapters. Features include checkpoint submission and 
verification, inclusion proof generation, cross-chain verification, and cost comparison 
across chains.

#### Corridor Bridge Protocol
Orchestrates multi-hop asset transfers through intermediate corridors. The two-phase commit 
protocol (PREPARE → COMMIT) ensures atomicity across multiple hops with cryptographic receipts 
at each stage. Automatic compensation handles failures with lock release in reverse order.

### New Modules

The following new modules have been added under `tools/phoenix/`:

The `tensor.py` module provides the Compliance Tensor V2 implementation with 952 lines covering 
tensor algebra, commitment generation, slicing operations, and proof generation.

The `zkp.py` module delivers the ZK proof infrastructure with circuit definitions, registry, 
mock prover/verifier, and standard circuit builders.

The `manifold.py` module implements the Compliance Manifold with path planning, attestation 
gap analysis, jurisdiction and corridor definitions, and standard manifold factory.

The `migration.py` module contains the Migration Saga state machine, evidence bundle collection, 
compensation logic, and migration orchestrator.

The `watcher.py` module provides the Watcher Economy with bond management, slashing system, 
reputation tracking, equivocation detection, and watcher registry.

The `anchor.py` module delivers L1 anchoring with chain adapters, checkpoint management, 
inclusion proofs, and cross-chain verification.

The `bridge.py` module implements the Corridor Bridge Protocol with two-phase commit, receipt 
chain, and bridge orchestration.

### New Schemas

JSON schemas added for PHOENIX types:

The `phoenix.compliance-tensor.schema.json` schema covers compliance domains, states, 
attestation references, tensor cells, and tensor commitments.

The `phoenix.migration-saga.schema.json` schema defines migration states, requests, 
transitions, evidence bundles, and compensation records.

The `phoenix.anchor.schema.json` schema specifies chains, anchor status, corridor checkpoints, 
anchor records, and inclusion proofs.

### Test Coverage

Comprehensive test suite with 65 tests covering all PHOENIX components:

The Compliance Tensor tests (11 tests) cover creation, set/get operations, fail-safe defaults, 
lattice operations, evaluation, slicing, commitment determinism, merge operations, and 
attestation expiry.

The ZK Infrastructure tests (6 tests) verify circuit creation, digest determinism, registry 
operations, standard registry, mock prover/verifier, and proof system properties.

The Compliance Manifold tests (8 tests) validate manifold creation, jurisdiction/corridor 
addition, path finding, constraint handling, attestation gap analysis, compliance distance, 
and unreachable jurisdiction handling.

The Migration Protocol tests (8 tests) examine saga creation, state transitions, invalid 
transition rejection, compensation, cancellation, completion, evidence collection, and 
orchestrator functionality.

The Watcher Economy tests (8 tests) cover watcher registration, bond posting/activation, 
slashing claims, slash percentages, reputation scoring, watcher selection, and equivocation 
detection.

The L1 Anchor tests (8 tests) verify chain properties, checkpoint creation/determinism, 
anchor manager, cost comparison, anchor retrieval, inclusion proof verification, and 
cross-chain verification.

The Corridor Bridge tests (7 tests) validate bridge creation, execution success, hop receipts, 
no-path handling, fee constraints, statistics, and receipt chain.

The Full System Integration tests (3 tests) exercise complete migration flows, watcher 
attestation with tensor updates, and end-to-end cross-jurisdictional migration with all 
components.

The Edge Case tests (6 tests) handle empty tensors, stale attestations, migration timeouts, 
insufficient bonds, bond draining, and excluded jurisdictions.

### Architecture Principles

The PHOENIX release follows five core design principles:

FAIL-SAFE DEFAULTS: Unknown compliance states default to non-compliant, ensuring security 
even with incomplete information.

CRYPTOGRAPHIC INTEGRITY: Every state transition produces verifiable proof through tensor 
commitments, attestation digests, and Merkle proofs.

ATOMIC OPERATIONS: Migrations either complete fully or compensate, with no partial states 
left in the system.

ECONOMIC ACCOUNTABILITY: Watchers stake collateral for their attestations, creating 
skin-in-the-game incentives for honest behavior.

PRIVACY BY DESIGN: ZK proofs enable compliance verification without disclosing sensitive 
transaction details.

### Breaking Changes

None. The PHOENIX modules are additive and do not modify existing APIs.

### Migration Guide

No migration required. Existing v0.4.42 deployments can adopt PHOENIX components incrementally.

### Known Limitations

The ZK circuits use mock implementations. Production deployments should integrate with 
Gnark, Circom, or similar ZK backends.

L1 anchoring uses mock chain adapters. Production deployments require integration with 
ethers.js, web3.py, or chain-specific SDKs.

The Compliance Manifold includes only UAE-DIFC and KZ-AIFC jurisdictions. Additional 
jurisdictions should be added via `manifold.add_jurisdiction()`.

### Future Work

Version 0.4.44 will target Regulator Console API completion with privacy-preserving queries 
and real-time monitoring dashboard.

Version 0.4.45 will focus on Mass Protocol five primitives completion (Entities, Ownership, 
Financial Instruments, Identity, Consent).

Version 0.4.46 will deliver Moxie Protocol integration bridge for IP Operating System 
connectivity.

### Contributors

Engineering team at Momentum (engineering@momentum.inc)

---

## [0.4.42] - 2026-01-28 - Elite Hardening

Bug fixes and test hardening. 13 bugs fixed, 453 tests passing.

## [0.4.41] - 2026-01-27 - Foundation

Initial MSEZ Stack implementation with entity registry, KYC foundation, financial services 
module, governance and arbitration frameworks.
