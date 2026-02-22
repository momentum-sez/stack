---
paths:
  - "mez/crates/mez-api/**"
description: "Rules for the mez-api composition root and orchestration layer"
---

# API & Orchestration Rules

## Write-path pipeline (every write endpoint)

```
Request → Auth (constant-time bearer) → Compliance Tensor (20 domains)
  → Sanctions hard-block (NonCompliant = reject)
  → Mass API call (proxy or sovereign Postgres)
  → VC issuance (Ed25519-signed attestation)
  → Attestation storage (Postgres)
  → Response (OrchestrationEnvelope)
```

Read endpoints are pass-through — no compliance eval needed.

## Key files

- Entry point: `src/main.rs`
- Orchestration: `src/orchestration.rs`
- App state: `src/state.rs` (in-memory Store<T> + Postgres)
- Routes: `src/routes/*.rs`
- DB migrations: `migrations/`
- Sovereign Mass ops: `src/sovereign_ops.rs`

## Sovereign mode

When `SOVEREIGN_MASS=true`, mez-api serves Mass primitive routes directly. Mass client URLs point to self (`http://localhost:8080`). Each zone has its own Postgres — data never crosses zone boundaries.

## Database

SQLx with compile-time checked migrations. Optional — app works in memory-only mode without `DATABASE_URL`. Pool: 2-20 connections, 5s acquire timeout. Hydration on startup via `state.hydrate_from_db()`.

## Receipt chain invariants

- `receipt.prev_root == final_state_root` (hash-chain continuity)
- `receipt.next_root == SHA256(JCS(payload_without_proof_and_next_root))`
- `mmr_root() == MMR(next_roots)`

## Compliance tensor invariants

- All mandatory domains evaluated, no empty slices
- `NotApplicable` requires signed policy artifact
- Sanctions NonCompliant is a hard block (legal requirement)
- Extended 12 domains currently return Pending — this is the primary logic gap to address
