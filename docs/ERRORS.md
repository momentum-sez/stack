# Error taxonomy

**Momentum EZ Stack** v0.4.44 GENESIS

Structured error codes, meanings, and recovery strategies.

---

## Error code format

```
P[Layer][Category][Sequence]

Layer:
  1 = Compliance Intelligence (Tensor, ZK)
  2 = Jurisdictional Infrastructure (Manifold, Migration, Bridge, Anchor)
  3 = Network Coordination (Watcher, Security)
  4 = Operations (Config, Health, CLI)

Category:
  0 = Validation    3 = Economic     6 = Timeout
  1 = State         4 = Network      7 = Internal
  2 = Security      5 = Resource

Example: P2101 = Layer 2, State, Sequence 01
```

---

## Layer 1: Compliance Intelligence

### P10xx — Tensor validation

| Code | Name | Recovery |
|------|------|----------|
| P1001 | `INVALID_ASSET_ID` | Verify ID matches `^[a-zA-Z0-9_-]{1,64}$` |
| P1002 | `INVALID_JURISDICTION_ID` | Use ISO 3166-1 alpha-2 or zone ID format |
| P1003 | `INVALID_DOMAIN` | Use valid `ComplianceDomain` enum value |
| P1004 | `INVALID_TIME_QUANTUM` | Use Unix timestamp in valid range |
| P1005 | `COORDINATE_OUT_OF_BOUNDS` | Check tensor dimensions before access |
| P1006 | `SPARSE_CELL_LIMIT_EXCEEDED` | Compact tensor or increase limit |

### P11xx — Tensor state

| Code | Name | Recovery |
|------|------|----------|
| P1101 | `INVALID_STATE_TRANSITION` | Check `ComplianceState` lattice rules |
| P1102 | `ATTESTATION_EXPIRED` | Request fresh attestation |
| P1103 | `ATTESTATION_SCOPE_MISMATCH` | Verify attestation covers required scope |
| P1104 | `TENSOR_MERGE_CONFLICT` | Use `tensor_meet()` or `tensor_join()` explicitly |

### P13xx — ZK proof

| Code | Name | Recovery |
|------|------|----------|
| P1301 | `INVALID_PROOF_FORMAT` | Regenerate proof with valid format |
| P1302 | `PROOF_VERIFICATION_FAILED` | Check witness and public inputs |
| P1303 | `UNSUPPORTED_PROOF_SYSTEM` | Use Groth16 or PLONK |
| P1304 | `CIRCUIT_MISMATCH` | Use correct circuit for verification |

---

## Layer 2: Jurisdictional Infrastructure

### P20xx — Manifold path

| Code | Name | Recovery |
|------|------|----------|
| P2001 | `NO_PATH_EXISTS` | Check corridor availability and compliance |
| P2002 | `PATH_CONSTRAINT_VIOLATION` | Relax constraints or find alternative |
| P2003 | `CORRIDOR_NOT_ACTIVE` | Wait for activation or reroute |
| P2004 | `COMPLIANCE_BARRIER` | Obtain required attestations |
| P2005 | `PATH_COST_EXCEEDED` | Increase cost limit or find cheaper path |
| P2006 | `CIRCULAR_PATH_DETECTED` | Use acyclic path finding |

### P21xx — Migration state

| Code | Name | Recovery |
|------|------|----------|
| P2101 | `INVALID_MIGRATION_STATE` | Check state machine and retry |
| P2102 | `MIGRATION_TIMEOUT` | Trigger compensation or retry |
| P2103 | `ATTESTATION_QUORUM_NOT_MET` | Wait for more attestations |
| P2104 | `SOURCE_LOCK_FAILED` | Verify asset ownership and retry |
| P2105 | `DESTINATION_VERIFICATION_FAILED` | Check destination compliance |
| P2106 | `COMPENSATION_FAILED` | Manual intervention required |
| P2107 | `MIGRATION_ALREADY_COMPLETE` | No action needed |
| P2108 | `MIGRATION_CANCELLED` | Start new migration if needed |

### P22xx — Bridge

| Code | Name | Recovery |
|------|------|----------|
| P2201 | `PREPARE_PHASE_FAILED` | Release locks and retry |
| P2202 | `COMMIT_PHASE_FAILED` | Retry with exponential backoff |
| P2203 | `HOP_TIMEOUT` | Check corridor status |
| P2204 | `RECEIPT_CHAIN_BROKEN` | Sync receipts from corridor |
| P2205 | `ATOMIC_COMMIT_VIOLATION` | Trigger dispute resolution |

### P23xx — Anchor

| Code | Name | Recovery |
|------|------|----------|
| P2301 | `CHAIN_NOT_SUPPORTED` | Add chain adapter |
| P2302 | `ANCHOR_SUBMISSION_FAILED` | Retry with higher gas |
| P2303 | `CONFIRMATION_TIMEOUT` | Check chain status and resubmit |
| P2304 | `FINALITY_NOT_REACHED` | Wait for more confirmations |
| P2305 | `REORG_DETECTED` | Resubmit checkpoint |
| P2306 | `CONTRACT_CALL_FAILED` | Check contract state and inputs |
| P2307 | `INSUFFICIENT_FUNDS` | Fund anchor wallet |

---

## Layer 3: Network Coordination

### P30xx — Watcher

| Code | Name | Recovery |
|------|------|----------|
| P3001 | `INSUFFICIENT_COLLATERAL` | Deposit additional collateral |
| P3002 | `WATCHER_NOT_REGISTERED` | Register watcher first |
| P3003 | `ATTESTATION_SIGNATURE_INVALID` | Re-sign with correct key |
| P3004 | `DUPLICATE_ATTESTATION` | No action needed |
| P3005 | `SLASHING_TRIGGERED` | Review violation; appeal if invalid |
| P3006 | `BOND_LOCKED` | Wait for unlock period |
| P3007 | `QUORUM_SELECTION_FAILED` | Lower quorum or wait for more watchers |

### P31xx — Security

| Code | Name | Recovery |
|------|------|----------|
| P3101 | `NONCE_ALREADY_USED` | Use fresh nonce |
| P3102 | `NONCE_EXPIRED` | Generate new nonce |
| P3103 | `VERSION_MISMATCH` | Upgrade to compatible version |
| P3104 | `TIME_LOCK_ACTIVE` | Wait for unlock time |
| P3105 | `RATE_LIMIT_EXCEEDED` | Implement backoff |
| P3106 | `SIGNATURE_INVALID` | Verify signing key and data |
| P3107 | `UNAUTHORIZED_CALLER` | Check permissions |

### P32xx — Economic attack detection

| Code | Name | Recovery |
|------|------|----------|
| P3201 | `FRONT_RUNNING_DETECTED` | Use commit-reveal |
| P3202 | `SANDWICH_ATTACK_DETECTED` | Use MEV protection |
| P3203 | `WHALE_MANIPULATION` | Apply concentration limits |
| P3204 | `WASH_TRADING_DETECTED` | Flag accounts for review |
| P3205 | `COLLUSION_SUSPECTED` | Increase quorum diversity |

---

## Layer 4: Operations

### P40xx — Configuration

| Code | Name | Recovery |
|------|------|----------|
| P4001 | `CONFIG_FILE_NOT_FOUND` | Create config file or use defaults |
| P4002 | `CONFIG_PARSE_ERROR` | Fix YAML/JSON syntax |
| P4003 | `CONFIG_VALIDATION_FAILED` | Check against schema |
| P4004 | `REQUIRED_CONFIG_MISSING` | Set required values |
| P4005 | `ENV_VAR_NOT_SET` | Set environment variable |

### P41xx — Health

| Code | Name | Recovery |
|------|------|----------|
| P4101 | `DEPENDENCY_UNAVAILABLE` | Check dependency health |
| P4102 | `RESOURCE_EXHAUSTED` | Scale or free resources |
| P4103 | `DEGRADED_PERFORMANCE` | Investigate bottleneck |
| P4104 | `STARTUP_FAILED` | Check logs and config |

---

## Response format

All errors follow RFC 7807 Problem Details:

```json
{
  "type": "https://docs.momentum.inc/errors/P2101",
  "title": "Invalid Migration State",
  "status": 400,
  "detail": "Migration 'mig-abc123' is in state COMPLETED but expected TRANSIT",
  "instance": "/migrations/mig-abc123/advance",
  "code": "P2101",
  "timestamp": "2026-02-19T10:30:00Z",
  "correlation_id": "corr-xyz789",
  "context": {
    "migration_id": "mig-abc123",
    "current_state": "COMPLETED",
    "expected_states": ["TRANSIT"]
  },
  "recovery": {
    "strategy": "CHECK_STATE",
    "message": "Verify migration state before advancing."
  }
}
```

---

## Recovery strategies

### Automatic

| Strategy | Applies to |
|----------|-----------|
| `RETRY_WITH_BACKOFF` | P2302, P2303, P4101 |
| `REFRESH_ATTESTATION` | P1102, P1103 |
| `WAIT_FOR_QUORUM` | P2103, P3007 |
| `COMPACT_STATE` | P1006, P4102 |

### Manual

| Strategy | Applies to |
|----------|-----------|
| `MANUAL_INTERVENTION` | P2106, P2205 |
| `APPEAL_PROCESS` | P3005 |

---

## See also

- [Architecture](./ARCHITECTURE.md)
- [Security Model](./architecture/SECURITY-MODEL.md)
