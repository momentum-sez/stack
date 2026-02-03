# PHOENIX Architecture

## Smart Asset Operating System

Version 0.4.44 GENESIS

This document describes the architectural foundations of the PHOENIX system, the core infrastructure enabling autonomous Smart Assets to operate across programmable jurisdictions.

---

## Foundational Premise

Traditional assets are bound to territorial sovereignty. Their compliance is verified by human auditors, their movement requires bilateral agreements between financial institutions, and their settlement depends on correspondent banking relationships that take months to establish. Cross-border movement involves navigating 195+ incompatible regulatory regimes, each with its own documentation requirements.

Smart Assets transcend these limitations through three fundamental capabilities.

**Embedded Compliance Intelligence.** The asset carries its compliance state as an intrinsic property, represented as a 4-dimensional tensor that can be evaluated against any jurisdiction's requirements. The asset knows whether it is compliant, can identify what attestations are missing, and can compute optimal migration paths.

**Autonomous Migration.** When regulatory conditions change—a license expires, a sanctions list updates, a corridor closes—the asset responds without human intervention. It can lock itself, migrate to compliant jurisdictions, or halt operations as required by policy.

**Cryptographic Verification.** Every state transition produces verifiable proof. Attestations are signed by bonded watchers. Receipts chain cryptographically. Settlement is anchored to public blockchains. Trust is verified, never assumed.

---

## System Architecture

The PHOENIX stack is organized into three layers, each building on the capabilities of the layer below.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                              │
│                          LAYER 3: NETWORK COORDINATION                       │
│                                                                              │
│    Economic accountability and security infrastructure ensuring honest       │
│    behavior across the decentralized network of watchers and corridors.      │
│                                                                              │
│    ┌────────────────┐  ┌────────────────┐  ┌────────────────┐              │
│    │    Watcher     │  │    Security    │  │   Hardening    │              │
│    │    Economy     │  │     Layer      │  │     Layer      │              │
│    │                │  │                │  │                │              │
│    │  Bonds         │  │  Nonces        │  │  Validation    │              │
│    │  Slashing      │  │  Time Locks    │  │  Thread Safety │              │
│    │  Reputation    │  │  Audit Logs    │  │  Rate Limits   │              │
│    └────────────────┘  └────────────────┘  └────────────────┘              │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                     LAYER 2: JURISDICTIONAL INFRASTRUCTURE                   │
│                                                                              │
│    Path planning, migration execution, and settlement finality enabling      │
│    cross-border asset movement through corridor networks.                    │
│                                                                              │
│    ┌────────────────┐  ┌────────────────┐  ┌────────────────┐              │
│    │   Compliance   │  │   Migration    │  │   Corridor     │              │
│    │    Manifold    │  │    Protocol    │  │    Bridge      │              │
│    │                │  │                │  │                │              │
│    │  Dijkstra      │  │  Saga FSM      │  │  Two-Phase     │              │
│    │  Path Planning │  │  Compensation  │  │  Commit        │              │
│    └────────────────┘  └────────────────┘  └────────────────┘              │
│                              │                     │                         │
│                              └─────────┬───────────┘                         │
│                                        ▼                                     │
│                            ┌────────────────────┐                           │
│                            │     L1 Anchor      │                           │
│                            │      Network       │                           │
│                            │                    │                           │
│                            │  Ethereum · L2s    │                           │
│                            │  Cross-Chain       │                           │
│                            │  Settlement        │                           │
│                            └────────────────────┘                           │
│                                                                              │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│                          LAYER 1: ASSET INTELLIGENCE                         │
│                                                                              │
│    Core computational substrate providing compliance state representation,   │
│    privacy-preserving verification, and deterministic execution.             │
│                                                                              │
│    ┌────────────────┐  ┌────────────────┐  ┌────────────────┐              │
│    │   Compliance   │  │    ZK Proof    │  │   Smart Asset  │              │
│    │    Tensor      │  │ Infrastructure │  │       VM       │              │
│    │                │  │                │  │                │              │
│    │  4D Sparse     │  │  Groth16       │  │  256-bit Stack │              │
│    │  Lattice       │  │  PLONK         │  │  Coprocessors  │              │
│    │  Merkleized    │  │  STARK         │  │  Gas Metering  │              │
│    └────────────────┘  └────────────────┘  └────────────────┘              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Layer 1: Asset Intelligence

### Compliance Tensor

The Compliance Tensor is the mathematical core of Smart Asset autonomy. It represents the multi-dimensional compliance state of an asset across all bound jurisdictions as a single, cryptographically committable object.

**Mathematical Definition**

```
C: AssetID × JurisdictionID × ComplianceDomain × TimeQuantum → ComplianceState

where:
    AssetID ∈ {SHA256(genesis_document)}
    JurisdictionID ∈ {uae-difc, kz-aifc, us-delaware, ...}
    ComplianceDomain ∈ {AML, KYC, SANCTIONS, TAX, SECURITIES, CORPORATE, CUSTODY}
    TimeQuantum ∈ ℤ (block height or timestamp modulo period)
    ComplianceState ∈ {COMPLIANT, NON_COMPLIANT, PENDING, UNKNOWN, EXEMPT, EXPIRED}
```

**Lattice Algebra**

Compliance states form a bounded lattice under the compliance ordering:

```
            COMPLIANT
               /\
              /  \
           EXEMPT  \
            /       \
           /    PENDING
          /        /
         /        /
    UNKNOWN ────
        \       \
         \       \
          \   NON_COMPLIANT
           \     /
            \   /
           EXPIRED
```

The meet operation (∧) returns the least compliant of two states, ensuring pessimistic composition. If any domain is NON_COMPLIANT, the aggregate is NON_COMPLIANT. This fail-safe behavior is critical for regulatory safety.

**Cryptographic Commitment**

The tensor generates a Merkle commitment over all populated cells, enabling efficient proof that a specific (asset, jurisdiction, domain, time) coordinate has a claimed compliance state. The commitment uses deterministic cell ordering (sorted by coordinate) to ensure reproducibility.

### Zero-Knowledge Proofs

The ZK infrastructure enables privacy-preserving compliance verification. An asset can prove it is compliant without revealing the underlying evidence, transaction history, or beneficial ownership.

**Standard Circuits**

The circuit registry includes four pre-built circuits for common compliance operations.

Balance Sufficiency proves that a balance exceeds a threshold without revealing the exact amount. The circuit accepts (balance, threshold) as private inputs and outputs a single bit indicating sufficiency.

Sanctions Clearance proves non-membership in a sanctions set using a Merkle non-membership proof. The circuit accepts (entity_hash, sanctions_merkle_root, non_membership_proof) as private inputs and outputs a bit indicating clearance.

KYC Attestation proves that a valid KYC attestation exists from an approved issuer without revealing the attestation details. The circuit accepts (attestation_digest, issuer_signature, approved_issuers_merkle_root) as private inputs.

Compliance Tensor Inclusion proves that a specific tensor coordinate has a claimed compliance state. This enables selective disclosure of compliance status for specific jurisdictions without revealing the full tensor.

**Proof Systems**

The infrastructure supports three proof systems, each with different tradeoffs.

Groth16 provides constant-size proofs (192 bytes) with fast verification but requires a trusted setup per circuit.

PLONK provides universal trusted setup (one setup for all circuits) with slightly larger proofs and slower verification.

STARK provides post-quantum security with no trusted setup but larger proofs (tens of kilobytes).

### Smart Asset VM

The Smart Asset Virtual Machine provides deterministic execution of compliance operations across the decentralized network. The VM is specifically designed for Smart Asset operations, with coprocessors for compliance tensor access and migration protocol execution.

**Architecture**

The VM provides a 256-slot stack where each slot holds a 256-bit word, matching Ethereum's word size for compatibility. Memory is expandable up to 64KB with byte-level addressability. Storage is Merkleized for efficient state proofs.

**Instruction Set**

The instruction set comprises 60+ opcodes organized into ten categories.

Stack operations (0x00-0x0F) include PUSH variants for 1, 2, 4, 8, and 32 bytes, POP, DUP at various depths, and SWAP at various depths.

Arithmetic operations (0x10-0x1F) include ADD, SUB, MUL, DIV, and MOD, all operating on 256-bit unsigned integers with modular arithmetic.

Comparison operations (0x20-0x2F) include EQ, NE, LT, GT, LE, GE returning 1 or 0, and boolean operators AND, OR, NOT, XOR.

Memory operations (0x30-0x3F) include MLOAD (32-byte read), MSTORE (32-byte write), MSTORE8 (single byte), and MSIZE.

Storage operations (0x40-0x4F) include SLOAD and SSTORE for persistent Merkleized storage.

Control flow operations (0x50-0x5F) include JUMP (unconditional), JUMPI (conditional), JUMPDEST (marker), RETURN, and REVERT.

Context operations (0x60-0x6F) provide access to execution context: CALLER, ORIGIN, JURISDICTION, TIMESTAMP, BLOCK_HEIGHT, ASSET_ID, GAS, GASPRICE.

Compliance operations (0x70-0x7F) provide tensor access: TENSOR_GET, TENSOR_SET, TENSOR_EVAL, TENSOR_COMMIT, ATTEST, VERIFY_ATTEST, VERIFY_ZK.

Migration operations (0x80-0x8F) provide protocol access: LOCK, UNLOCK, TRANSIT_BEGIN, TRANSIT_END, SETTLE, COMPENSATE.

Crypto operations (0x90-0x9F) provide hash and signature functions: SHA256, KECCAK256, VERIFY_SIG, MERKLE_ROOT, MERKLE_VERIFY.

**Gas Metering**

Every operation has an associated gas cost, preventing infinite loops and enabling resource pricing. Storage operations cost significantly more than arithmetic (SSTORE costs 20,000 gas versus ADD at 3 gas). Compliance coprocessor operations are priced to reflect their computational complexity.

**Coprocessors**

The Compliance Coprocessor provides direct access to the compliance tensor, enabling contracts to query and update compliance state atomically with other operations.

The Migration Coprocessor provides access to the migration protocol state machine, enabling contracts to lock assets, initiate transit, and settle transfers.

---

## Layer 2: Jurisdictional Infrastructure

### Compliance Manifold

The Compliance Manifold models the jurisdictional landscape as a graph where nodes are jurisdictions and edges are corridors. It computes optimal migration paths considering compliance requirements, fees, time, and attestation gaps.

**Graph Structure**

Each jurisdiction node contains entry requirements (required attestations for inbound assets), supported asset classes (which asset types the jurisdiction accepts), licensing requirements, and regulatory classifications.

Each corridor edge contains bilateral compliance requirements, fee schedules (typically 10-100 basis points), settlement time (hours to days), watcher quorum requirements, and liquidity constraints.

**Path Planning**

Given a source jurisdiction, target jurisdiction, and asset with its current compliance tensor, the manifold computes the optimal path using Dijkstra's algorithm with compliance-aware edge weights.

The weight function combines fees, time, and attestation difficulty:

```
weight(corridor) = fee_bps + time_hours × time_weight + attestation_gap × gap_weight
```

The attestation gap is computed by comparing the asset's current attestations against the corridor's requirements.

**Attestation Gap Analysis**

For a given path, the manifold produces an attestation gap report listing every attestation the asset needs to acquire before migration can proceed. This enables proactive compliance—the asset can begin gathering attestations before initiating migration.

### Migration Protocol

The Migration Protocol implements cross-jurisdictional asset movement as a saga-based state machine with compensation for failures.

**State Machine**

The migration progresses through eight states:

INITIATED marks the request received with parameters validated. COMPLIANCE_CHECK verifies source compliance is valid. ATTESTATION_GATHERING collects any missing attestations. SOURCE_LOCK locks the asset at source jurisdiction. TRANSIT moves the asset through corridor network. DESTINATION_VERIFICATION verifies destination compliance. DESTINATION_UNLOCK unlocks the asset at destination. COMPLETED marks migration successful.

**Compensation**

If any state fails, the protocol executes compensating actions to restore consistency:

If SOURCE_LOCK fails, no action is needed as the asset was never locked. If TRANSIT fails, the protocol unlocks at source and returns to INITIATED. If DESTINATION_VERIFICATION fails, the protocol reverses transit and unlocks at source. If DESTINATION_UNLOCK fails, the protocol initiates dispute resolution.

**Evidence Bundle**

Throughout migration, the protocol collects a comprehensive evidence bundle including source tensor commitment, all gathered attestations, lock receipt with lock ID and timestamp, transit proof with corridor signatures, destination tensor commitment, and unlock receipt.

This bundle provides non-repudiable proof of the complete migration for audit and dispute resolution.

### Corridor Bridge

The Corridor Bridge orchestrates multi-hop transfers through intermediate jurisdictions using a two-phase commit protocol.

**Protocol Phases**

Phase 1 (PREPARE) locks the asset at each hop along the path. The bridge contacts each corridor in sequence, requesting a lock. Each corridor returns a prepare receipt containing lock_id, locked_amount, lock_expiry, and signatures from both the source jurisdiction and the corridor operator. If any prepare fails, all previous locks are released.

Phase 2 (COMMIT) executes the transfers atomically. The bridge contacts each corridor with the prepare receipts, authorizing transfer execution. Each corridor returns a commit receipt containing settlement_tx_id, settlement_block, and signatures. If any commit fails after a threshold, the protocol enters dispute resolution.

**Receipt Chain**

The bridge maintains an immutable receipt chain for each migration. The chain contains all prepare and commit receipts, enabling reconstruction of the complete transfer path for audit.

### L1 Anchor Network

The L1 Anchor Network provides settlement finality by checkpointing corridor state to Ethereum and L2 networks.

**Supported Chains**

Ethereum Mainnet (chain_id: 1) requires 64-block finality at approximately 13 minutes and provides maximum security with the highest gas costs.

Arbitrum One (chain_id: 42161) requires 1-block finality and provides Ethereum security with significantly lower costs.

Base (chain_id: 8453) requires 1-block finality and provides Ethereum security with low costs.

Polygon PoS (chain_id: 137) requires 256-block finality at approximately 9 minutes and provides lower security but very low costs.

**Checkpoint Structure**

A corridor checkpoint contains the corridor_id, checkpoint_height (monotonically increasing), receipt_merkle_root (root of all receipts since last checkpoint), state_root (corridor state commitment), timestamp, and watcher_signatures (threshold of corridor watchers).

**Inclusion Proofs**

Given an anchored checkpoint, any receipt can prove its inclusion via a Merkle path from the receipt to the receipt_merkle_root. This enables trustless verification that a specific transfer was included in a finalized checkpoint.

**Cross-Chain Verification**

For high-value transfers, the anchor network supports cross-chain verification where the same checkpoint is anchored to multiple chains. Verification succeeds only if all chains confirm the checkpoint, providing defense-in-depth against chain-specific attacks.

---

## Layer 3: Network Coordination

### Watcher Economy

The Watcher Economy provides economic accountability for corridor attestations. Watchers must stake collateral to participate, and misbehavior results in slashing.

**Bond Structure**

A watcher bond contains the watcher identity (DID and public key), collateral amount and currency, collateral custody address, bond status (active, suspended, withdrawn), attestation volume limit (proportional to collateral), and expiration time.

**Attestation Limits**

A watcher can only attest to transactions up to 10× their bonded collateral. This limit prevents under-collateralized attestations that could enable profitable misbehavior.

**Slashing Conditions**

Equivocation (100% slash) occurs when the watcher signs conflicting attestations for the same event. Equivocation is trivially provable by presenting both signatures.

False Attestation (50% slash) occurs when the watcher attests to an invalid state that is later proven incorrect. Proving false attestation requires presenting the incorrect attestation and evidence of the true state.

Availability Failure (1% slash) occurs when the watcher fails to attest within required time bounds. This encourages reliable participation.

Collusion (100% slash + permanent ban) occurs when multiple watchers coordinate to attest to false state. Detection requires multiple false attestations from different watchers for the same event.

**Reputation**

Beyond slashing, watchers accumulate reputation based on accuracy rate (correct attestations divided by total), availability rate (timely attestations divided by required), total volume attested, and age of participation. Reputation affects corridor selection—high-reputation watchers receive priority for high-value transfers.

### Security Layer

The Security Layer provides defense-in-depth protection against common attack vectors.

**Replay Prevention**

Every operation includes a nonce that is registered in a NonceRegistry. The registry tracks seen nonces with expiration times (default 7 days). Attempting to reuse a nonce fails with a replay-detected error.

**TOCTOU Protection**

Critical state is stored in VersionedStore, which provides compare-and-swap operations. Any update must specify the expected version; if the actual version differs (indicating concurrent modification), the update fails.

**Front-Running Prevention**

Sensitive operations like withdrawals require time locks. The operator announces the operation commitment (hash of operation details), waits for the delay period (7 days for withdrawals), then reveals the operation data for execution. This gives other participants time to respond to announced operations.

**Audit Logging**

All security-relevant events are logged to the AuditLogger, which maintains a hash chain. Each event includes the previous event's digest, enabling detection of log tampering. The log supports queries by actor, event type, resource, and time range.

### Hardening Layer

The Hardening Layer provides production-grade utilities for validation, thread safety, and economic attack prevention.

**Input Validation**

The Validators class provides comprehensive validation for strings (length, pattern, allowed characters), digests (64-char hex SHA256), addresses (Ethereum 0x + 40 hex), amounts (Decimal with bounds checking), timestamps (ISO8601 with freshness checking), and bytes (length bounds, hex decoding).

**Thread Safety**

ThreadSafeDict wraps a dictionary with RLock protection for all operations. AtomicCounter provides atomic increment/decrement with compare-and-set. The atomic decorator wraps any function with lock acquisition.

**Economic Guards**

EconomicGuard enforces limits against economic attacks. Attestation values cannot exceed 10× bond collateral. Minimum bond collateral is enforced. Slash rates are capped at 50% per epoch to prevent rapid draining. Whale concentration is detected when any operator exceeds 33% of total stake.

---

## Design Principles

Eight core principles guide every architectural decision in PHOENIX.

**Fail-Safe Defaults.** The system fails closed, never open. Unknown compliance states are treated as non-compliant. Missing attestations invalidate compliance. Expired credentials are treated as absent. When in doubt, deny.

**Cryptographic Integrity.** Every state transition produces verifiable proof. Tensor commitments are Merkle roots over canonical cell representations. Attestations are content-addressed by their digest. Receipts chain cryptographically. Nothing is trusted without verification.

**Atomic Operations.** There are no partial states. Migrations either complete fully or compensate entirely. Two-phase commit ensures all-or-nothing semantics. Saga patterns handle distributed failures with coordinated rollback.

**Economic Accountability.** Watchers stake real collateral for their attestations. Misbehavior is slashed automatically and provably. Reputation affects future opportunities. Incentives are designed to align with honest behavior.

**Privacy by Design.** Zero-knowledge proofs enable compliance verification without disclosure. Selective tensor slices reveal only necessary state. Range proofs hide exact amounts while proving sufficiency. Privacy is the default, not an afterthought.

**Defense in Depth.** Multiple layers protect against each threat class. Nonces prevent replay at the application layer. Time locks prevent front-running at the economic layer. Cross-chain verification prevents chain-specific attacks at the settlement layer.

**Zero Trust.** All inputs are untrusted until validated. External data is sanitized before use. Signatures are verified, not trusted. Digests are recomputed, not assumed correct. Trust is earned through cryptographic verification.

**Deterministic Execution.** The VM produces identical results across all nodes. No floating point operations (all arithmetic is integer). No randomness in execution (deterministic gas). No external state access (all state is explicit). Consensus is achievable.

---

## Security Considerations

### Threat Model

The PHOENIX system is designed to be secure against the following threat classes.

**Malicious Watchers.** A watcher may attempt to attest to false state for profit. Defense: slashing conditions with sufficient collateral to make attacks unprofitable.

**Network Partitions.** A network partition may cause conflicting attestations. Defense: equivocation detection with 100% slashing prevents rational actors from exploiting partitions.

**Front-Running.** An attacker may observe pending operations and act on them first. Defense: time-locked operations with commit-delay-reveal pattern.

**Replay Attacks.** An attacker may replay valid messages in new contexts. Defense: nonce registry with message freshness validation.

**State Manipulation.** An attacker may attempt to modify historical state. Defense: Merkle commitments anchored to L1 chains.

### Security Boundaries

The system maintains clear security boundaries.

**Trust Boundary 1:** All external input crosses a validation boundary implemented in the hardening layer.

**Trust Boundary 2:** All attestations require verification of watcher signatures and bond validity.

**Trust Boundary 3:** All state claims require Merkle proof verification against anchored commitments.

**Trust Boundary 4:** All cross-chain claims require verification on all configured chains.

---

## Future Work

The architecture is designed to support several planned extensions.

**Governance Framework.** Protocol parameters (slashing percentages, time lock durations, fee schedules) will be governable through a token-weighted voting mechanism with time-locked execution.

**Cross-Corridor Liquidity.** A liquidity aggregation layer will enable efficient routing across multiple corridors based on available liquidity, reducing settlement times and fees.

**Real-Time Compliance Monitoring.** Integration with regulatory data feeds will enable real-time updates to compliance state as sanctions lists change, licenses expire, or regulatory guidance evolves.

**Production ZK Backend.** The mock ZK implementation will be replaced with production backends (Gnark for Groth16/PLONK, StarkWare for STARK) enabling actual cryptographic verification.

---

Copyright © 2026 Momentum. All rights reserved.

Contact: engineering@momentum.inc
