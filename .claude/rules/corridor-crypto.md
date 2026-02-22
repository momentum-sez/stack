---
paths:
  - "mez/crates/mez-corridor/**"
  - "mez/crates/mez-crypto/**"
  - "mez/crates/mez-vc/**"
  - "mez/crates/mez-state/**"
description: "Rules for corridor, crypto, VC, and state machine crates"
---

# Corridor & Crypto Rules

## mez-state: typestate FSMs

Invalid state transitions are compile errors. States: Draft → Pending → Active, with Halted/Suspended branches. Never bypass the type system with string-based state checks.

## mez-corridor: receipt chains

Dual commitment model: hash-chain continuity + MMR inclusion proofs. `ReceiptChain::append()` enforces prev_root linkage. Fork detection via signed watcher attestations with timestamp bounds.

Trade flows have typed structures (4 archetypes, 10 transitions) but NO FSM enforcement yet — this is a known gap.

## mez-crypto

Ed25519 keys use zeroize. MMR for inclusion proofs. CAS (content-addressed storage) via SHA-256. All SHA-256 MUST flow through `mez-core::digest` except in `mmr.rs`.

## mez-vc

W3C Verifiable Credentials with Ed25519Signature2020. Smart Asset Registry VC is the primary credential type. Proof verification requires the issuer's public key.

## ZK proofs (mez-zkp)

STUBS ONLY. MockProofSystem in dev, fail-closed ProofPolicy rejects mock in release builds. Do not write code that assumes ZK circuits work.
