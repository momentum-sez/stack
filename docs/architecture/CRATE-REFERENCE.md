# Crate reference

Per-crate API surface for the 16 crates in the `msez/` workspace. Each section lists the crate's purpose, key types, public functions, and invariants enforced by the type system.

For the full architecture context, see [Architecture Overview](./OVERVIEW.md).

---

## msez-core

**Foundation types.** Every other crate depends on this.

### Key types

| Type | Purpose |
|------|---------|
| `ComplianceDomain` | Enum with 20 variants: `Aml`, `Kyc`, `Sanctions`, `Tax`, `Securities`, `Corporate`, `Custody`, `DataPrivacy`, `Licensing`, `Banking`, `Payments`, `Clearing`, `Settlement`, `DigitalAssets`, `Employment`, `Immigration`, `Ip`, `ConsumerProtection`, `Arbitration`, `Trade` |
| `CanonicalBytes` | JCS-compatible canonical serialization. **Sole path to digest computation.** |
| `ContentDigest` | 32-byte SHA-256 hash with algorithm tag |
| `EntityId` | UUID-based newtype for entities |
| `CorridorId` | String-based newtype for corridors |
| `MigrationId` | UUID-based newtype for migrations |
| `WatcherId` | UUID-based newtype for watchers |
| `JurisdictionId` | Validated string newtype |
| `Did` | W3C DID format |
| `Cnic` | Pakistan NADRA 13-digit identifier |
| `Ntn` | Pakistan FBR 7-digit tax number |
| `Timestamp` | `DateTime<Utc>` wrapper with ISO 8601 serialization |
| `MsezError` | Root error type (`thiserror`-based) |

### Key functions

```rust
CanonicalBytes::new<T: Serialize>(value: &T) -> Result<Self, CanonicalizationError>
sha256_digest(canonical: &CanonicalBytes) -> ContentDigest
ComplianceDomain::all() -> &'static [ComplianceDomain]
EntityId::new() -> Self  // generates UUID v4
```

### Invariant

All digest computation flows through `CanonicalBytes::new()`. Direct SHA-256 of non-canonical bytes is structurally impossible in downstream crates.

---

## msez-crypto

**Cryptographic primitives.** Ed25519 signing, MMR, CAS, SHA-256.

### Key types

| Type | Purpose |
|------|---------|
| `SigningKey` | Ed25519 private key. `Zeroize` + `ZeroizeOnDrop`. Does **not** implement `Serialize`. |
| `VerifyingKey` | Ed25519 public key. Serializes as lowercase hex. |
| `Ed25519Signature` | 64-byte signature. Serializes as lowercase hex. |
| `MerkleMountainRange` | Append-only accumulator for receipt chains |
| `MmrInclusionProof` | Proof of leaf inclusion in MMR |
| `ContentAddressedStore` | Trait for CAS implementations |
| `ArtifactRef` | Content-addressed artifact reference |

### Key functions

```rust
SigningKey::generate<R: CryptoRngCore>(csprng: &mut R) -> Self
SigningKey::sign(&self, canonical: &CanonicalBytes) -> Result<Ed25519Signature, CryptoError>
VerifyingKey::verify(&self, canonical: &CanonicalBytes, sig: &Ed25519Signature) -> Result<(), CryptoError>

MerkleMountainRange::new() -> Self
MerkleMountainRange::append(&mut self, next_root_hex: &str) -> Result<String, CryptoError>
MerkleMountainRange::root(&self) -> String
MerkleMountainRange::proof(&self, leaf_index: usize) -> Result<MmrInclusionProof, CryptoError>
verify_inclusion_proof(leaf_index, next_root_hex, mmr_root, proof) -> Result<bool, CryptoError>
```

### Feature flags

| Feature | What it enables |
|---------|----------------|
| `bbs-plus` | BBS+ selective disclosure signatures (Phase 2) |
| `poseidon2` | Poseidon2 ZK-friendly hashing (Phase 2) |

### Invariant

Signing requires `&CanonicalBytes`. Private keys zero memory on drop. Private keys cannot be serialized.

---

## msez-vc

**W3C Verifiable Credentials.** Issue, sign, and verify credentials with Ed25519 proofs.

### Key types

| Type | Purpose |
|------|---------|
| `VerifiableCredential` | W3C VC structure with JSON-LD context, type, issuer, subject, proofs |
| `Proof` | Ed25519Signature2018/2020 proof with verification method |
| `ProofType` | `Ed25519Signature2018`, `Ed25519Signature2020` |
| `ProofPurpose` | `AssertionMethod`, `Authentication` |
| `SmartAssetRegistryVc` | Specialized VC for smart asset registry binding |
| `JurisdictionBinding` | Jurisdiction + lawpack digest binding |

### Key functions

```rust
VerifiableCredential::sign(&mut self, signing_key: &SigningKey, verification_method: &str) -> Result<(), VcError>
VerifiableCredential::verify(&self, verifying_key: &VerifyingKey) -> Result<Vec<ProofResult>, VcError>
```

### Invariant

All VC signing flows through `CanonicalBytes` (inherited from `msez-crypto`). Proof structure is rigid -- `additionalProperties: false` in the schema.

---

## msez-state

**Typestate-encoded state machines.** Invalid transitions don't compile.

### Corridor (6 states)

```rust
Corridor<Draft>    → .submit()   → Corridor<Pending>
Corridor<Pending>  → .activate() → Corridor<Active>
Corridor<Active>   → .halt()     → Corridor<Halted>
Corridor<Active>   → .suspend()  → Corridor<Suspended>
Corridor<Halted>   → .deprecate()→ Corridor<Deprecated>
Corridor<Suspended>→ .resume()   → Corridor<Active>
```

### Entity (10 stages)

`Formation` → `Operational` → `Expansion` | `Contraction` → `Restructuring` → `Suspension` → `Dissolution` (7 sub-stages)

### Migration (8 phases + 3 terminal)

`Phase0Initiation` through `Phase4Finalization`, with terminal states `Completed`, `Aborted`, `CompensationFailed`.

### License (5 states)

`Pending` → `Active` → `Suspended` | `Revoked` | `Expired`

### Watcher (4 states)

`Bonding` → `Active` → `Slashed` → `Unbonding`

Slashing conditions: `InvalidProof`, `EquivocationDetected`, `InactivityViolation`, `PerjuryDetected`.

### Dynamic variants

Each typestate machine has a `Dyn*` enum for runtime dispatch when the state is not known at compile time (e.g., deserializing from JSON or database).

---

## msez-tensor

**Compliance Tensor V2.** 20-domain evaluation with Dijkstra manifold optimization.

### Key types

| Type | Purpose |
|------|---------|
| `ComplianceTensor<J: JurisdictionConfig>` | Parameterized tensor mapping domains to states |
| `ComplianceState` | 5-value lattice: `NotApplicable` > `Exempt` > `Compliant` > `Pending` > `NonCompliant` |
| `TensorCell` | Per-domain cell: state + attestations + timestamp + reason |
| `TensorSlice` | Snapshot of all 20 domains at a point in time |
| `TensorCommitment` | Merkle root of tensor state |
| `ComplianceManifold` | Jurisdiction graph for path optimization |
| `MigrationPath` | Optimal route with hops and total cost |
| `ComplianceDistance` | Cost vector: fee + time_days + risk_score |
| `PathConstraint` | `MaxFee`, `MaxDays`, `MaxRisk`, `ExcludeJurisdictions` |

### Key traits

```rust
trait JurisdictionConfig: Clone + Debug + Send + Sync {
    fn jurisdiction_id(&self) -> &JurisdictionId;
    fn applicable_domains(&self) -> &[ComplianceDomain];
}

trait DomainEvaluator: Send + Sync {
    fn evaluate(&self, context: &EvaluationContext) -> (ComplianceState, Option<String>);
}
```

### Key functions

```rust
ComplianceTensor::new(jurisdiction: J) -> Self
ComplianceTensor::evaluate_all(&self, entity_id: &str) -> TensorSlice
ComplianceTensor::commit(&self) -> Result<TensorCommitment, Error>
ComplianceManifold::shortest_path(&self, from, to, constraints) -> Option<MigrationPath>
```

---

## msez-corridor

**Cross-border corridor operations.** Receipt chains, fork resolution, netting, SWIFT.

### Key types

| Type | Purpose |
|------|---------|
| `CorridorReceipt` | Single receipt with `prev_root`, `next_root`, `sequence` |
| `ReceiptChain` | MMR-backed append-only receipt sequence |
| `Checkpoint` | Compressed state for fast sync |
| `ForkDetector` | Detects receipt chain forks |
| `ForkResolution` | 3-level resolution result |
| `CorridorBridge` | Dijkstra-weighted corridor routing |
| `BridgeRoute` | Computed route with path and total fee |
| `NettingEngine` | Bilateral/multilateral settlement compression |
| `SettlementPlan` | Netted settlement legs |
| `SwiftPacs008` | ISO 20022 pacs.008 instruction generator |

### Key functions

```rust
ReceiptChain::new(corridor_id: CorridorId) -> Self
ReceiptChain::append(&mut self, receipt: CorridorReceipt) -> Result<String, ReceiptError>
ReceiptChain::verify_inclusion(&self, receipt) -> Result<(), ReceiptError>
ForkDetector::resolve(&self, branch1, branch2) -> ForkResolution
CorridorBridge::compute_route(&self, from, to) -> Result<BridgeRoute, CorridorError>
NettingEngine::compute_plan(&self) -> Result<SettlementPlan, NettingError>
SwiftPacs008::generate_instruction(leg) -> Result<SettlementInstruction, Error>
```

### Fork resolution ordering

1. Primary: lexicographically earlier timestamp
2. Secondary: more watcher attestations
3. Tertiary: lexicographic digest ordering

Clock skew tolerance: 5 minutes (per spec section 3.5).

---

## msez-pack

**Pack Trilogy.** Lawpacks, regpacks, licensepacks.

### Key types

| Type | Purpose |
|------|---------|
| `Lawpack` | Akoma Ntoso statutory corpus with content-addressed digest |
| `Regpack` | Regulatory requirements + sanctions checker |
| `Licensepack` | License registry with authority tracking |
| `SanctionsChecker` | Fuzzy name matching against OFAC/UN/EU lists |
| `SanctionsEntry` | Individual sanctions list entry |
| `SanctionsMatch` | Match result with confidence score |
| `License` | License instance with status lifecycle |
| `LicenseStatus` | `Pending`, `Active`, `Suspended`, `Revoked`, `Expired` |

### Key functions

```rust
Lawpack::parse_yaml(content: &str) -> Result<Lawpack, PackError>
Lawpack::verify_digest(&self) -> Result<bool, PackError>
SanctionsChecker::check(&self, entity_name: &str, threshold: f64) -> Option<SanctionsMatch>
Licensepack::issue_license(&self, entity: &EntityId, license_type: &str) -> Result<License, PackError>
Licensepack::check_validity(&self, license: &License) -> Result<LicenseStatus, PackError>
```

---

## msez-agentic

**Autonomous policy engine.** 20 triggers, deterministic evaluation, audit trail.

### Trigger types (20)

Regulatory: `SanctionsListUpdate`, `LicenseStatusChange`, `GuidanceUpdate`, `ComplianceDeadline`
Arbitration: `DisputeFiled`, `RulingReceived`, `AppealPeriodExpired`, `EnforcementDue`
Corridors: `CorridorStateChange`, `SettlementAnchorAvailable`, `WatcherQuorumReached`
Assets: `CheckpointDue`, `KeyRotationDue`, `GovernanceVoteResolved`
Fiscal: `TaxYearEnd`, `WithholdingDue`
Entity: `EntityDissolution`, `PackUpdated`, `AssetTransferInitiated`, `MigrationDeadline`

### Key types

| Type | Purpose |
|------|---------|
| `Policy` | Trigger types + condition + actions + priority + auth requirement |
| `PolicyEngine` | BTreeMap-ordered policy evaluation |
| `Trigger` | Trigger type + timestamp + context |
| `PolicyAction` | `NoOp`, `Halt`, `Suspend`, `Review`, `Notify`, `UpdateCompliance`, `FreezeAsset`, `RequireApproval`, `ArchiveRecords` |
| `Condition` | Boolean logic: `Always`, `Never`, `EntityMatch`, `CorridorMatch`, `And`, `Or`, `Not` |
| `ScheduledAction` | Action queued for execution |
| `AuditTrail` | Append-only audit log with content-addressed entries |

### Key functions

```rust
PolicyEngine::process_trigger(&mut self, trigger, entity_id, jurisdiction) -> Vec<ScheduledAction>
PolicyEngine::resolve_conflicts(actions) -> ScheduledAction
ActionScheduler::get_ready_actions(&self) -> Vec<&ScheduledAction>
AuditTrail::export_digest(&self) -> Result<ContentDigest, String>
```

### Determinism (Theorem 17.1)

Given identical trigger events and policy state, evaluation is deterministic. Conflict resolution: Priority → Jurisdiction specificity → Policy ID.

---

## msez-arbitration

**Dispute resolution lifecycle.** Evidence, escrow, enforcement.

### Key types

| Type | Purpose |
|------|---------|
| `Dispute` | Full dispute with state, parties, claims, transition history |
| `DisputeState` | 9 states: `Filed`, `EvidencePhase`, `HearingScheduled`, `UnderDecision`, `Decided`, `Settled`, `DismissedOrWithdrawn`, `EnforcementInitiated`, `ReviewInitiated` |
| `EvidencePackage` | Collection of evidence items with chain of custody |
| `EvidenceItem` | Content-addressed evidence with authenticity attestation |
| `EscrowAccount` | Held funds with release conditions |
| `EnforcementOrder` | Ordered actions with deadline |
| `EnforcementAction` | `PayAmount`, `TransferAsset`, `ReleaseEscrow`, `FreezeAsset`, `UpdateCompliance` |

### Key functions

```rust
// Dispute lifecycle transitions
Dispute::file(claimant, respondent, claims) -> Dispute  // Filed state
Dispute::advance_to_evidence() -> Result<(), ArbitrationError>
Dispute::schedule_hearing() -> Result<(), ArbitrationError>
// ... state machine transitions

EscrowAccount::release(condition) -> Result<(), ArbitrationError>
EnforcementOrder::execute(action) -> Result<EnforcementReceipt, ArbitrationError>
```

---

## msez-compliance

**Jurisdiction configuration bridge.** Connects regpack data to the compliance tensor.

### Key types

| Type | Purpose |
|------|---------|
| `RegpackJurisdiction` | Implements `JurisdictionConfig` from regpack domain data |
| `SanctionsEvaluator` | Implements `DomainEvaluator` using `SanctionsChecker` |

### Key functions

```rust
RegpackJurisdiction::from_domain_names(jid: JurisdictionId, names: &[String]) -> Self
SanctionsEvaluator::with_threshold(checker: Arc<SanctionsChecker>, threshold: f64) -> Self
build_tensor(jurisdiction_id, applicable_domains, sanctions_entries) -> Option<ComplianceTensor<RegpackJurisdiction>>
```

### Architecture pattern

```
msez-pack (data) → msez-compliance (bridge) → msez-tensor (algebra)
```

---

## msez-zkp

**Zero-knowledge proof system.** Sealed trait, 12 circuits, CDB bridge.

### Key types

| Type | Purpose |
|------|---------|
| `ProofSystem` | **Sealed trait** -- only `msez-zkp` can implement it |
| `MockProofSystem` | Deterministic SHA-256 mock (Phase 1, current) |
| `Cdb` | Canonical Digest Bridge: `Poseidon2(Split256(SHA256(JCS(A))))` |
| `CircuitType` | 12 variants: `BalanceSufficiency`, `SanctionsClearance`, `TensorInclusion`, `MigrationEvidence`, `OwnershipChain`, `CompensationValidity`, `KycAttestation`, `AttestationValidity`, `ThresholdSignature`, `RangeProof`, `MerkleMembership`, `NettingValidity` |

### Feature-gated backends

| Feature | Backend | Phase |
|---------|---------|-------|
| *(default)* | `MockProofSystem` | 1 (current) |
| `groth16` | arkworks Groth16 SNARK | 2 |
| `plonk` | halo2 PLONK | 2 |
| `poseidon2` | Poseidon2 in CDB | 2 |

### Invariant

The `ProofSystem` trait is sealed. External crates cannot implement it. This prevents unauthorized proof backends.

---

## msez-schema

**JSON Schema validation.** Draft 2020-12, 116 schemas.

### Key types

| Type | Purpose |
|------|---------|
| `SchemaValidator` | Validates data against any of the 116 schemas |
| `SchemaValidationError` | Schema ID + JSON pointer + message |

### Key functions

```rust
SchemaValidator::new() -> Result<Self, String>  // loads all schemas
SchemaValidator::validate_module(&self, data, schema_id) -> Result<(), SchemaValidationError>
SchemaValidator::validate_zone(&self, data) -> Result<(), SchemaValidationError>
check_additional_properties_policy(schema) -> Result<(), AdditionalPropertiesViolation>
```

### Security enforcement

Security-critical schemas (`corridor.receipt`, `verifiable_credential`, `proof`, etc.) must have `additionalProperties: false`. Extensible paths (`credential_subject`, `metadata`, `context`) are exempted.

---

## msez-mass-client

**Typed HTTP client for Mass APIs.** The only authorized path from SEZ Stack to Mass.

### Key types

| Type | Purpose |
|------|---------|
| `MassClient` | Top-level client with sub-clients for each primitive |
| `MassApiConfig` | URLs + auth token + timeout (from environment) |
| `EntityClient` | Entity CRUD against `organization-info.api.mass.inc` |
| `OwnershipClient` | Cap table operations against `investment-info` |
| `FiscalClient` | Treasury operations against `treasury-info.api.mass.inc` |
| `IdentityClient` | KYC/DID operations |
| `ConsentClient` | Governance approval workflows |
| `TemplatingClient` | Document rendering |
| `MassApiError` | `Http`, `Config`, `Parse`, `NotFound`, `Unauthorized`, `BadRequest`, `ServerError` |

### Key functions

```rust
MassClient::new(config: MassApiConfig) -> Result<Self, MassApiError>
MassApiConfig::from_env() -> Result<Self, MassApiError>

client.entities().create_entity(req) -> Result<Entity, MassApiError>
client.entities().get_entity(id) -> Result<Entity, MassApiError>
client.ownership().get_cap_table(entity_id) -> Result<CapTable, MassApiError>
client.fiscal().record_payment(payment) -> Result<(), MassApiError>
client.identity().verify_kyc(req) -> Result<KycResult, MassApiError>
client.consent().request_governance_approval(req) -> Result<String, MassApiError>
client.templating().render_document(template_id, variables) -> Result<Vec<u8>, MassApiError>
```

### Invariant

All Mass API communication goes through this crate. Direct `reqwest` calls to Mass endpoints from other crates are forbidden.

---

## msez-api

**Axum HTTP server.** Composes all crates into an authenticated, rate-limited API.

### Routes

| Route | Module | Domain |
|-------|--------|--------|
| `/v1/entities/*` | `mass_proxy` | Mass proxy |
| `/v1/ownership/*` | `mass_proxy` | Mass proxy |
| `/v1/fiscal/*` | `mass_proxy` | Mass proxy |
| `/v1/identity/*` | `mass_proxy` | Mass proxy |
| `/v1/consent/*` | `mass_proxy` | Mass proxy |
| `/v1/corridors/*` | `corridors` | SEZ native |
| `/v1/settlement/*` | `settlement` | SEZ native |
| `/v1/assets/*` | `smart_assets` | SEZ native |
| `/v1/credentials/*` | `credentials` | SEZ native |
| `/v1/triggers` | `agentic` | SEZ native |
| `/v1/policies/*` | `agentic` | SEZ native |
| `/v1/regulator/*` | `regulator` | SEZ native |
| `/health/liveness` | *(built-in)* | Probes |
| `/health/readiness` | *(built-in)* | Probes |

### Middleware stack

```
TraceLayer → MetricsMiddleware → AuthMiddleware → RateLimitMiddleware → Handler
```

Auth uses `subtle::ConstantTimeEq` for bearer token comparison. Rate limiting uses per-route token bucket. Health probes are unauthenticated.

### Key types

| Type | Purpose |
|------|---------|
| `AppState` | Config + signing key + policy engine + corridor/asset/attestation stores + Mass client |
| `AppConfig` | Port + auth token |

---

## msez-cli

**Command-line interface.** Offline zone management.

### Subcommands

| Command | Purpose |
|---------|---------|
| `msez validate` | Validate modules, profiles, zones against schemas |
| `msez lock` | Generate/verify lockfiles |
| `msez corridor` | Corridor lifecycle: create, submit, activate, halt, suspend, resume, list, status |
| `msez artifact` | CAS operations: store, resolve, verify |
| `msez vc` | Key generation, document signing, signature verification |

### Key types

| Type | Purpose |
|------|---------|
| `ValidateArgs` | `--all-modules`, `--all-profiles`, `--all-zones`, or path |
| `LockArgs` | Zone path + `--check` + `--out` + `--strict` |
| `CorridorSubcommand` | `Create`, `Submit`, `Activate`, `Halt`, `Suspend`, `Resume`, `List`, `Status` |
| `SigningSubcommand` | `Keygen`, `Sign`, `Verify` |

---

## msez-integration-tests

**Cross-crate test suite.** 99 test files covering all crates and integration scenarios.

### Test categories

| Category | Files | Coverage |
|----------|-------|----------|
| Canonicalization & digests | 5 | JCS compatibility, SHA-256 determinism |
| MMR & receipt chains | 8 | Cross-language parity, append-only guarantee |
| Corridor & fork resolution | 12 | Typestate transitions, 3-level ordering, routing |
| Compliance tensor | 10 | Lattice operations, commitments, evaluation |
| Verifiable Credentials | 8 | Signing/verification, registry, W3C compliance |
| Agentic policy engine | 10 | Determinism (Thm 17.1), conflict resolution |
| Arbitration | 6 | Dispute lifecycle, evidence, escrow |
| Pack operations | 9 | Lawpack parsing, sanctions matching, license lifecycle |
| Schema validation | 8 | All 116 schemas, security policy checks |
| CLI operations | 7 | Signing commands, module validation, corridor lifecycle |
| API integration | 8 | Auth middleware, rate limiting, endpoint CRUD |
| Security & adversarial | 2 | Canonicalization attacks, signature malleability |
| Smart assets | 4 | Lifecycle, migration, compliance binding |
| Artifact graph | 5 | Dependency tracking, witness bundles |
| Regression | 1 | Known defect regressions |

### Running tests

```bash
# Full suite
cargo test -p msez-integration-tests

# Specific test file
cargo test -p msez-integration-tests --test test_corridor_typestate

# With output
cargo test -p msez-integration-tests -- --nocapture
```
