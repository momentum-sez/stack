---
description: "Honest implementation status — what is real, shallow, stubbed, or nonexistent"
---

# Implementation Status

Do not write code that assumes stubs or planned features work.

## Production-Grade

Typestate FSMs (mez-state), receipt chains (mez-corridor), Ed25519/MMR/CAS (mez-crypto),
canonicalization (mez-core), VCs (mez-vc), compliance tensor structure (mez-tensor),
JSON Schema validation (mez-schema), pack trilogy (mez-pack), database persistence
(SQLx + Postgres), write-path orchestration (mez-api), HTTP API (50+ endpoints),
Docker Compose (1/2/3-zone), AWS Terraform, K8s manifests, deploy script, Mass API client.

## Structurally Complete, Logically Shallow

These have correct types and tests but business logic needs deepening:

- Compliance tensor extended 12 domains: metadata-driven business rule validation for all 12 (licensing expiry, Basel III CAR, float safeguarding, settlement cycles, token classification, labor/immigration status, arbitration frameworks, trade sanctions screening). Deep logic validates metadata against domain-specific rules but jurisdiction-specific regulator mapping not yet wired (e.g., "which CAR threshold for pk vs ae")
- Trade flow instruments: 4 archetypes, 10 transitions typed — no FSM enforcement of ordering
- Sovereign Mass persistence: in-memory + Postgres CRUD — no business validation
- Agentic policy engine: trigger ingestion works; reactive execution limited
- Arbitration: 7-phase lifecycle typed — not wired end-to-end in API
- Fork resolution: evidence-driven logic typed — not wired into live corridors
- National adapters (FBR, SECP, NADRA, Raast): HTTP wrappers exist — depend on live gov APIs
- Inter-zone corridor protocol: tested in-process — not tested across real network

## Stubs (return NotImplemented)

ZK proof circuits (mock, fail-closed in release). BBS+ (feature-gated trait). Poseidon2 (feature-gated). SWIFT/Circle payment adapters (trait only).

## Does Not Exist

Identity as dedicated Mass service. Smart Asset VM (SAVM). MASS L1 settlement. CI/CD pipeline. Web UI.
