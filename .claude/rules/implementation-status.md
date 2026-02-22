---
description: "Honest implementation status — what is real, shallow, stubbed, or nonexistent"
---

# Implementation Status

Do not write code that assumes stubs or planned features work.

## Production-Grade

Typestate FSMs (mez-state), receipt chains (mez-corridor), Ed25519/MMR/CAS (mez-crypto),
canonicalization (mez-core), VCs (mez-vc), compliance tensor structure (mez-tensor),
JSON Schema validation (mez-schema), pack trilogy (mez-pack), database persistence
(SQLx + Postgres), write-path orchestration (mez-api), HTTP API (60+ endpoints),
Docker Compose (1/2/3-zone), AWS Terraform, K8s manifests, deploy script, Mass API client,
CI/CD pipeline (GitHub Actions: fmt, clippy, tests, audit, schema validation, credential guards, ZK mock guard),
trade flow FSM enforcement (4 archetypes, 10 transitions with validate_transition + TradeFlowManager),
fork resolution (evidence-driven 3-level ordering: timestamp → attestation count → lexicographic),
arbitration dispute lifecycle (7-phase API: file → review → evidence → hearing → decide → enforce → close, plus settle/dismiss),
agentic policy engine (reactive dispatch for Halt/Resume/ArbitrationEnforce/UpdateManifest + scheduled for remaining actions),
sovereign Mass entity validation (name + jurisdiction required, treasury entity-ref validation),
L1 anchor API (`POST /v1/corridors/state/anchor` — wired to `MockAnchorTarget`, accepts corridor ID, computes checkpoint digest from receipt chain, returns `AnchorReceipt`),
finality status API (`POST /v1/corridors/state/finality-status` — wired to `AnchorTarget::check_status`, returns `AnchorStatus`),
registry VC submission (`POST /v1/assets/registry` — issues Ed25519-signed `MezSmartAssetRegistryCredential` VC, stores attestation, updates asset status to Registered),
anchor verification API (`POST /v1/assets/{id}/anchors/corridor/verify` — wired to `AnchorTarget::check_status`, returns finality status for anchor digest).

## Structurally Complete, Logically Shallow

These have correct types and tests but business logic needs deepening:

- Compliance tensor extended 12 domains: metadata-driven business rule validation for all 12 (licensing expiry, Basel III CAR, float safeguarding, settlement cycles, token classification, labor/immigration status, arbitration frameworks, trade sanctions screening). Deep logic validates metadata against domain-specific rules but jurisdiction-specific regulator mapping not yet wired (e.g., "which CAR threshold for pk vs ae")
- Inter-zone corridor protocol: tested in-process — not tested across real network

## Mock Adapters (working test logic, awaiting live backends)

These have full trait definitions, error types, data models, serde round-trip tests, and
deterministic mock implementations. They function correctly in tests and dev but depend on
live external APIs that are not yet available or credentialed.

### National regulatory adapters (`mez-mass-client`)

| Adapter | Trait | Mock | Status |
|---------|-------|------|--------|
| FBR IRIS (tax authority) | `FbrIrisAdapter` | `MockFbrIrisAdapter` — NTN verification, tax event submission, withholding rate queries (S153/S149 rates), taxpayer profiles | Awaiting live FBR IRIS API credentials |
| NADRA (identity) | `NadraAdapter` | `MockNadraAdapter` — CNIC verification, biometric match simulation | Awaiting live NADRA e-Sahulat API |
| SECP (corporate registry) | `SecpAdapter` | `MockSecpAdapter` — company verification, director lookup | Awaiting live SECP eServices API |
| SBP Raast (payments) | `RaastAdapter` | `MockRaastAdapter` — IBAN validation, payment initiation, status checks, alias lookup (mobile/CNIC) | Awaiting live SBP Raast API |

### Payment rail adapters (`mez-corridor::payment_rail`)

| Adapter | Mock | Status |
|---------|------|--------|
| `RaastAdapter` | Returns `NotConfigured` | Thin corridor-layer stub; real logic lives in `mez-mass-client::raast::MockRaastAdapter` |
| `RtgsAdapter` | Returns `NotConfigured` | Thin corridor-layer stub; needs RTGS network credentials |
| `CircleUsdcAdapter` | Returns `NotConfigured` | Thin corridor-layer stub; needs Circle API credentials |

## Stubs (return NotImplemented / feature-gated)

### ZK proof backends (`mez-zkp`)

| Component | Location | Phase | Status |
|-----------|----------|-------|--------|
| `MockProofSystem` | `mez-zkp::mock` | 1 (active) | SHA-256 deterministic proofs, NO zero-knowledge. Fail-closed in release builds via CI guard. |
| `ProofSystem` trait | `mez-zkp::traits` | 1 | Sealed trait with `prove`/`verify`/`setup` — only `MockProofSystem`, `Groth16ProofSystem`, `PlonkProofSystem` can implement. |
| `Groth16ProofSystem` | `mez-zkp::groth16` | 2 | Feature-gated (`groth16`). Types compile, `prove`/`verify`/`setup` return `NotImplemented`. Awaits `ark-groth16` + `ark-bn254`. |
| `PlonkProofSystem` | `mez-zkp::plonk` | 2 | Feature-gated (`plonk`). Types compile, `prove`/`verify`/`setup` return `NotImplemented`. Awaits `halo2_proofs`. |
| Circuit data models | `mez-zkp::circuits::{compliance,identity,settlement,migration}` | 1 | 12 circuit structs with public inputs, witness fields, constraint count estimates, serde round-trip tests. Data model only — no R1CS/Plonkish constraints. |
| Proof policy engine | `mez-zkp::policy` | 1 | `ProofPolicy` decides which `ProofSystem` to use per circuit type. Currently always selects `MockProofSystem`. |
| CDB Poseidon2 path | `mez-zkp::cdb` | 4 | `#[cfg(feature = "poseidon2")]` codepath in CDB bridge is identity — compiles but does nothing until Poseidon2 lands. |

### Cryptographic primitives (`mez-crypto`)

| Component | Feature flag | Phase | Status |
|-----------|-------------|-------|--------|
| BBS+ signatures | `bbs-plus` | 4 | `bbs_sign`, `bbs_create_proof`, `bbs_verify_proof` all return `NotImplemented`. Types (`BbsSignature`, `BbsProof`, `BbsSigningKey`, `BbsVerifyingKey`) compile. Awaits `bbs` or `bbs-plus` crate. |
| Poseidon2 hash | `poseidon2` | 4 | `poseidon2_digest`, `poseidon2_node_hash` return `NotImplemented`. `Poseidon2Digest` type compiles. Awaits `poseidon2-plonky2` or equivalent. |

### L1 anchoring (`mez-corridor::anchor`)

| Component | Status |
|-----------|--------|
| `AnchorTarget` trait | Sealed trait, production-ready interface. |
| `MockAnchorTarget` | Working mock — immediate finality, deterministic tx IDs, atomic block counter. Wired to API via `AppState.anchor_target`. |
| `EvmAnchorTarget` | Feature-gated (`evm-anchor`). Full JSON-RPC implementation: `eth_sendTransaction`, `eth_getTransactionReceipt`, `eth_blockNumber`, configurable finality thresholds, address validation. Needs live EVM RPC endpoint. |
| Anchor API endpoint | `POST /v1/corridors/state/anchor` — **wired** to `MockAnchorTarget`. Computes checkpoint digest from receipt chain, anchors, returns receipt. |
| Finality API endpoint | `POST /v1/corridors/state/finality-status` — **wired** to `AnchorTarget::check_status`. Returns finality status for a prior anchor tx. |
| Anchor verification | `POST /v1/assets/{id}/anchors/corridor/verify` — **wired** to `AnchorTarget::check_status`. Returns finality status for provided anchor digest. |

### Smart asset registry (`mez-api::routes::smart_assets`)

| Component | Status |
|-----------|--------|
| Registry VC submission | `POST /v1/assets/registry` — **wired** to VC issuance pipeline. Issues `MezSmartAssetRegistryCredential` VC with Ed25519 signature, stores attestation, updates asset to `Registered`. |

## Does Not Exist

Identity as dedicated Mass service. Smart Asset VM (SAVM). MASS L1 settlement. Web UI.
