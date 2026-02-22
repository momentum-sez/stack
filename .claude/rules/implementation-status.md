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
sovereign Mass entity validation (name + jurisdiction required, treasury entity-ref validation).

## Structurally Complete, Logically Shallow

These have correct types and tests but business logic needs deepening:

- Compliance tensor extended 12 domains: metadata-driven business rule validation for all 12 (licensing expiry, Basel III CAR, float safeguarding, settlement cycles, token classification, labor/immigration status, arbitration frameworks, trade sanctions screening). Deep logic validates metadata against domain-specific rules but jurisdiction-specific regulator mapping not yet wired (e.g., "which CAR threshold for pk vs ae")
- National adapters (FBR, SECP, NADRA, Raast): HTTP wrappers exist — depend on live gov APIs
- Inter-zone corridor protocol: tested in-process — not tested across real network

## Stubs (return NotImplemented)

ZK proof circuits (mock, fail-closed in release). BBS+ (feature-gated trait). Poseidon2 (feature-gated). SWIFT/Circle payment adapters (trait only). Smart asset registry VC submission (Phase 2). Corridor L1 anchoring and finality computation (Phase 2). Anchor verification (Phase 2).

## Does Not Exist

Identity as dedicated Mass service. Smart Asset VM (SAVM). MASS L1 settlement. Web UI.
