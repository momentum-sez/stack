# Terminology & Normative Language

This specification uses the key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** as described in RFC 2119 and RFC 8174.

## Core terms

- **Stack Spec**: The normative specification describing the module system, manifests, interfaces, and conformance rules.
- **Module**: A versioned, self-contained unit that produces one or more artifacts (legal texts, regulatory rules, schemas, APIs, forms, workflows).
- **Variant**: A named implementation of a module with different policy choices (e.g., `dispute.arbitration-first` vs `dispute.courts-first`).
- **Profile**: A bundle of modules + versions + parameters representing a deployable "style" (e.g., `digital-financial-center`).
- **Zone Node**: An instantiated deployment of a profile in a real jurisdiction (a "Mass network node" in the project context).
- **Corridor**: A configuration + institutional agreement pattern enabling cross-node interoperability (passporting, recognition, settlement).

- **Corridor Agreement VC**: A Verifiable Credential used to express participant-specific acceptance of a corridor definition and define activation thresholds.
- **Agreement-set digest**: A content-addressed SHA256 digest over (definition VC payload hash + agreement VC payload hashes) used to pin an activated corridor state deterministically.
- **Activation blockers**: A list of `<partyDid>:<commitment>` strings identifying non-affirmative commitments that prevent corridor activation.
- **Governance module**: A module that implements decision/consent mechanisms (voting, delegation, quadratic mechanisms) for zone governance workflows.

- **Verifiable Credential (VC)**: A digitally signed data structure (per the W3C VC model) used in MEZ to bind critical artifacts (e.g., corridor manifests) to an issuer identity (typically a DID) in a tamper-evident way.
- **Proof**: A cryptographic signature attached to a VC. MEZ supports multi-proof credentials for multi-party co-signing.

## Organizational terms

- **Momentum**: A venture fund and studio (`momentum.inc`). The parent organization behind Mass and the EZ Stack. Never "Momentum Protocol".
- **Mass**: The five programmable primitives (Entities, Ownership, Fiscal, Identity, Consent) provided as Java/Spring Boot services at `mass.inc`. Use "Mass Protocol" only when discussing the L1 settlement layer and ZKP circuits.
- **EZ Stack**: The Rust workspace (`mez-*` crates) providing jurisdictional intelligence on top of Mass. Owns compliance, corridors, packs, and orchestration. Never duplicates Mass CRUD.

## Five Primitives

- **Entities**: Organizations, legal persons, and other addressable actors. Managed by the `organization-info` Mass service.
- **Ownership**: Cap table structures, investment records, and beneficial ownership chains. Managed by `consent-info` (cap tables) and `investment-info` (investments).
- **Fiscal**: Treasury operations, fund flows, and financial reporting. Managed by the `treasury-info` Mass service.
- **Identity**: KYC/KYB identity verification and credential binding. Currently split across `consent-info` and `organization-info`; a dedicated `identity-info` service is planned.
- **Consent**: Authorization grants, permission management, and access control. Managed by the `consent-info` Mass service.

## Compliance domains (20)

The compliance tensor evaluates across exactly 20 domains. Every `match` in the codebase is exhaustive over this set:

| Domain | Description |
|--------|-------------|
| **AML** | Anti-Money Laundering — transaction monitoring, suspicious activity detection, STR filing |
| **KYC** | Know Your Customer — identity verification, due diligence levels (simplified, standard, enhanced) |
| **Sanctions** | Sanctions screening — OFAC, UN, EU, and national sanctions lists. `NonCompliant` is a hard-block (legal requirement) |
| **Tax** | Tax compliance — withholding obligations, reporting requirements, filer status differentials |
| **Securities** | Securities regulation — prospectus requirements, investor accreditation, offering restrictions |
| **Corporate** | Corporate governance — board composition, filing obligations, beneficial ownership disclosure |
| **Custody** | Asset custody — segregation requirements, custodian qualifications, client asset protection |
| **DataPrivacy** | Data privacy — consent management, cross-border data transfer rules, data residency |
| **Licensing** | Business licensing — license requirements by activity type, renewal tracking, compliance conditions |
| **Banking** | Banking regulation — capital adequacy, prudential requirements, reserve ratios |
| **Payments** | Payment services regulation — PSP licensing, transaction limits, settlement obligations |
| **Clearing** | Clearing and settlement — CCP requirements, margin calls, netting enforceability |
| **Settlement** | Settlement finality — irrevocability timing, delivery-versus-payment, L1 anchoring |
| **DigitalAssets** | Digital asset regulation — token classification, custody rules, exchange licensing |
| **Employment** | Employment law — labor permits, social security, minimum wage, contract requirements |
| **Immigration** | Immigration compliance — work authorization, visa status verification |
| **IP** | Intellectual property — patent/trademark registration, licensing, technology transfer controls |
| **ConsumerProtection** | Consumer protection — disclosure requirements, cooling-off periods, dispute resolution |
| **Arbitration** | Dispute resolution — arbitration enforceability, institutional rules, evidence standards |
| **Trade** | International trade — customs duties, export controls, trade agreements, rules of origin |

## Compliance states

- **Compliant**: The entity/operation satisfies all requirements for the evaluated domain.
- **NonCompliant**: One or more requirements are not met. For `Sanctions`, this is always a hard-block.
- **Pending**: Evaluation is in progress or awaiting additional information.
- **Exempt**: The entity/operation is exempt from this domain's requirements (requires signed policy artifact).
- **NotApplicable**: The domain does not apply to this jurisdiction or entity type (requires signed policy artifact in production).

## Compliance tensor

- **Compliance Tensor**: A multi-dimensional evaluation function `T: (entity, operation, jurisdiction, domain) -> ComplianceState`. Evaluated across all 20 domains for every write operation.
- **Tensor Cell**: A single `(domain, ComplianceState)` evaluation result within a tensor.
- **Evaluation Context**: The set of inputs (entity type, operation type, jurisdiction, timestamp) required to evaluate the tensor.
- **Fail-closed**: Production mode behavior where all mandatory domains must be evaluated, empty slices are errors, and `Pending` defaults to non-compliant.

## Compliance Manifold

- **Compliance Manifold**: A Dijkstra-weighted routing space across corridor compliance states. Used to find the lowest-cost compliant path between jurisdictions.

## Pack Trilogy

- **Lawpack**: A content-addressed snapshot of the legal corpus for a jurisdiction. Normalized to Akoma Ntoso XML. Provides the statutory foundation for regulatory rules.
- **Regpack**: A regulatory requirement set that maps statutory provisions (from lawpacks) to operational compliance rules. Consumed by the compliance tensor for domain evaluation.
- **Licensepack**: A business license and certification lifecycle manager. Tracks 15+ license categories for Pakistan (EMI, NBFC, PSO, PSP, etc.), including issuance, renewal, and revocation.

## Corridor lifecycle

- **Corridor FSM**: The finite state machine governing corridor lifecycle transitions. States: `Draft` -> `Pending` -> `Active` (with `Halted`, `Suspended`, `Deprecated` branches). Implemented as a typestate machine — invalid transitions are compile-time errors.
- **Receipt Chain**: An append-only sequence of corridor operation records. Each receipt references the previous state root, forming a hash chain. Backed by an MMR for efficient inclusion proofs.
- **Corridor Receipt**: A single append-only record in the receipt chain. Contains the operation payload, previous and next state roots, and proof objects.
- **Checkpoint**: A periodic snapshot of corridor state, including the receipt chain head, MMR root, and optional L1 anchor reference.
- **Fork Detection**: The process of identifying divergent receipt chains in a corridor. Uses a three-level ordering for resolution: timestamp (primary), watcher attestation count (secondary), lexicographic digest tiebreaker (tertiary).
- **Fork Resolution**: Selection of the canonical branch after a fork. Requires signed watcher attestations and respects a 5-minute maximum clock skew tolerance.

## Anchoring

- **Anchor Target**: An L1 blockchain endpoint for recording corridor checkpoint digests. Provides settlement finality. L1 anchoring is optional — the system functions without blockchain dependencies.
- **Anchor Commitment**: The checkpoint digest and metadata submitted to an anchor target.
- **Anchor Receipt**: Proof of successful on-chain recording, including transaction ID, block number, and finality status.
- **Anchor Status**: `Pending` (submitted), `Confirmed` (mined but not finalized), `Finalized` (irreversible), `Failed` (transaction reverted).

## Trade flow instruments

- **Trade Flow**: A structured commercial transaction following one of four archetypes.
- **Letter of Credit (LC)**: A bank-guaranteed payment instrument for international trade. Supports amendments, document requirements, and multi-party endorsement.
- **Bill of Lading (BoL)**: A transport document serving as receipt of goods, contract of carriage, and document of title. Supports endorsement chains.
- **Trade Invoice**: A commercial invoice with line items, tax calculations, and currency details.
- **Trade Flow Type**: One of `LC` (Letter of Credit), `Spot`, `Forward`, `Swap`.
- **Trade Flow State**: `Draft`, `Initiated`, `Confirmed`, `Settled`, `Disputed`, `Completed`.

## Settlement

- **Netting Engine**: Compresses bilateral or multilateral settlement obligations into net positions, reducing the number of actual fund transfers.
- **Settlement Leg**: A single directed payment between two parties, derived from netting a set of obligations.
- **Settlement Plan**: The complete set of settlement legs produced by the netting engine for a given set of obligations.

## Agentic policy engine

- **Agentic Trigger**: An environmental event that may invoke a policy action. 20 trigger types defined across regulatory, arbitration, corridor, and asset lifecycle categories.
- **Agentic Policy**: A mapping from trigger condition to automated action, with authorization requirements.
- **Policy Action**: A scheduled autonomous state transition executed by the policy engine.
- **Authorization Requirement**: The approval level required for a policy action: `Automatic`, `Quorum`, `Unanimous`, or `Governance`.
- **Policy Engine**: A deterministic evaluation engine that maps triggers to actions via the policy set. Audit-trailed.

## Identifiers

- **JurisdictionId**: An ISO 3166-1 alpha-2 code or zone-specific identifier (e.g., `PK`, `PK-REZ`, `AE-ABUDHABI-ADGM`, `SG`).
- **CorridorId**: A UUID-based bilateral trade channel identifier.
- **EntityId**: A UUID newtype for entity references within the EZ Stack.
- **MigrationId**: A UUID newtype for migration saga references.
- **WatcherId**: A UUID newtype for watcher references.
- **DID**: A W3C Decentralized Identifier used as the issuer identity for Verifiable Credentials.
- **CNIC**: Pakistan Computerized National Identity Card number (13 digits, with or without dashes). Validated by `mez_core::Cnic`.
- **NTN**: Pakistan National Tax Number (7 digits). Issued by FBR. Validated by `mez_core::Ntn`.
- **NRIC**: Singapore National Registration Identity Card number. Validated by `mez_core::Nric`.
- **UEN**: Singapore Unique Entity Number for business registration. Validated by `mez_core::Uen`.
- **EmiratesId**: UAE national identity number (15 digits). Validated by `mez_core::EmiratesId`.
- **ContentDigest**: A SHA-256 digest (32 bytes) computed over canonical bytes. Used for content-addressing throughout the system.

## Cryptographic primitives

- **CanonicalBytes**: JSON Canonicalization Scheme (JCS, RFC 8785) serialized bytes. All signing and digest operations use canonical bytes as input to prevent serialization-dependent malleability.
- **Momentum Canonical Form (MCF)**: The normative extension of JCS used throughout the EZ Stack. Defined in ADR-002.
- **Ed25519**: The digital signature algorithm used for all VC signing, corridor attestations, and watcher bonds. Key material is zeroized on drop.
- **Merkle Mountain Range (MMR)**: An append-only authenticated data structure for receipt chains. Provides O(log n) inclusion proofs.
- **Content-Addressed Storage (CAS)**: Artifacts stored and retrieved by their content digest. Ensures immutability and deduplication.
- **Key Provider**: An abstraction over Ed25519 key storage backends (local memory, environment variable, AWS KMS envelope encryption). Defined by the `KeyProvider` trait in `mez-crypto`.

## Data sovereignty

- **DataCategory**: Classification of data for sovereignty enforcement: `Pii`, `Financial`, `Tax`, `Corporate`, `Compliance`, `KeyMaterial`, `Analytics`, `PublicRegulatory`.
- **Data Residency**: The requirement that certain data categories remain within jurisdictional boundaries. Enforced by `mez-core/src/sovereignty.rs`.

## Watcher economy

- **Watcher**: An independent observer that monitors corridor operations and provides attestations for fork resolution and compliance verification.
- **Watcher Bond**: A stake deposited by a watcher to participate in the economy. Subject to slashing for misbehavior.
- **Slashing Condition**: A rule that reduces a watcher's bond. Four conditions defined: `DoubleAttestation`, `Inactivity`, `InvalidAttestation`, `ProtocolViolation`.
- **Watcher States**: `Bonding` -> `Active` -> `Unbonding`, with `Slashing` as a penalty state.

## Arbitration

- **Dispute**: A formal disagreement between corridor parties, tracked through a lifecycle FSM.
- **Escrow**: Funds held by a neutral party during dispute resolution, released upon resolution.
- **Institution Registry**: A registry of recognized arbitration institutions and their rules.

## Migration

- **Migration Saga**: An 8-phase migration process with idempotent compensation. Backed by CAS for artifact persistence and `EffectExecutor` for side-effect management.
- **Side Effect**: An operation with external consequences (e.g., database write, API call) that must be tracked for idempotent replay during migration.
- **EffectExecutor**: A trait for executing and compensating side effects during migration phases.

## Architecture layers

| Layer | Name | Description |
|-------|------|-------------|
| A | Enabling Authority | Foundational legal authorization for the economic zone |
| B | Legal Operating System | Legal corpus management via lawpacks |
| C | Regulatory Supervision | Regulatory requirements via regpacks and compliance tensor |
| D | Financial Infrastructure | Banking, payments, custody, and settlement infrastructure |
| E | Corridors | Cross-border interoperability, trade flows, and settlement |
| F | Observability & Control Plane | Monitoring, audit trails, and operational dashboards |
| G | Civic Governance & Diffusion | Public participation, governance modules, and zone expansion |

## Deployment modes

- **Sovereign mode** (`SOVEREIGN_MASS=true`): The zone IS the Mass server. Postgres-backed. No external Mass dependency. The EZ Stack handles both compliance evaluation and primitive CRUD.
- **Proxy mode** (`SOVEREIGN_MASS=false`): The zone proxies to centralized Mass APIs via `mez-mass-client`. The EZ Stack handles compliance evaluation; Mass handles primitive CRUD.

## Orchestration

- **Orchestration Pipeline**: The write-path sequence for every sovereign/proxy operation: pre-flight compliance -> hard-block check -> Mass API call -> VC issuance -> attestation storage -> return `OrchestrationEnvelope`.
- **OrchestrationEnvelope**: The composite response from a write operation, containing the Mass response, compliance evaluation, Verifiable Credential, and attestation ID.
- **Hard-block**: An operation rejection due to a `NonCompliant` result in the `Sanctions` domain. This is a legal requirement and cannot be overridden.

## National system adapters (Pakistan)

- **FBR IRIS**: Pakistan's Federal Board of Revenue Inland Revenue Information System. Provides NTN verification, tax event submission, withholding rate queries, and taxpayer profile retrieval.
- **Filer Status**: A taxpayer's FBR classification: `Filer` (compliant, lower withholding), `NonFiler` (unregistered, higher withholding, typically 2x), `LateFiler` (overdue filings, intermediate rates).
- **NADRA**: Pakistan's National Database and Registration Authority. Provides CNIC-based identity verification with biometric and biographical matching.
- **SBP Raast**: Pakistan's instant payment system operated by the State Bank of Pakistan. Provides real-time credit transfers, account verification, and alias-based (mobile/CNIC) lookups.
- **SECP**: Pakistan's Securities and Exchange Commission. Provides corporate registry lookup, license verification, filing status checks, and director/beneficial ownership queries.

## Schema and validation

- **JSON Schema (Draft 2020-12)**: The schema standard used for all 116 schemas in the `schemas/` directory. Compiled and cached at startup.
- **Schema URI**: All schema `$ref` values use full `schemas.momentum-ez.org` URIs.

## Phase gates

| Phase | Name | Status |
|-------|------|--------|
| 1 | Controlled Sandbox | READY |
| 2 | Corridor Activation | READY |
| 3 | Production | BLOCKED by identity service, real anchor target, HSM/KMS, external pen test |
| 4 | Cross-Border Expansion | Requires Poseidon2, BBS+, real ZK backends, watcher bond economics |
