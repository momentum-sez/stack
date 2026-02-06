# PHOENIX Error Taxonomy

**v0.4.44 GENESIS — Systematic Error Classification & Recovery**

This document provides a comprehensive catalog of all PHOENIX error codes, their meanings, causes, and recovery strategies.

---

## Error Code Structure

```
P[Layer][Category][Sequence]

Layer:
  1 = Asset Intelligence (Tensor, VM, ZK)
  2 = Jurisdictional Infrastructure (Manifold, Migration, Bridge, Anchor)
  3 = Network Coordination (Watcher, Security, Hardening)
  4 = Operations (Config, Health, CLI)

Category:
  0 = Validation
  1 = State
  2 = Security
  3 = Economic
  4 = Network
  5 = Resource
  6 = Timeout
  7 = Internal

Example: P1201 = Layer 1, Security, Sequence 01
```

---

## Layer 1: Asset Intelligence

### P10xx — Tensor Validation Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P1001 | `INVALID_ASSET_ID` | Asset ID format invalid or empty | Verify asset ID matches `^[a-zA-Z0-9_-]{1,64}$` |
| P1002 | `INVALID_JURISDICTION_ID` | Jurisdiction ID format invalid | Use ISO 3166-1 alpha-2 or zone ID format |
| P1003 | `INVALID_DOMAIN` | Compliance domain not recognized | Use valid ComplianceDomain enum value |
| P1004 | `INVALID_TIME_QUANTUM` | Time quantum out of valid range | Use Unix timestamp in valid range |
| P1005 | `COORDINATE_OUT_OF_BOUNDS` | Tensor coordinate exceeds dimensions | Check tensor dimensions before access |
| P1006 | `SPARSE_CELL_LIMIT_EXCEEDED` | Too many sparse cells in tensor | Compact tensor or increase limit |

### P11xx — Tensor State Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P1101 | `INVALID_STATE_TRANSITION` | Compliance state transition not allowed | Check ComplianceState lattice rules |
| P1102 | `ATTESTATION_EXPIRED` | Attestation timestamp beyond validity | Request fresh attestation |
| P1103 | `ATTESTATION_SCOPE_MISMATCH` | Attestation scope doesn't match query | Verify attestation covers required scope |
| P1104 | `TENSOR_MERGE_CONFLICT` | Cannot merge tensors with conflicting states | Use tensor_meet() or tensor_join() explicitly |

### P12xx — VM Execution Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P1201 | `STACK_OVERFLOW` | VM stack exceeded maximum depth (1024) | Reduce stack usage or split computation |
| P1202 | `STACK_UNDERFLOW` | Pop from empty stack | Verify stack depth before operations |
| P1203 | `OUT_OF_GAS` | Gas limit exceeded during execution | Increase gas limit or optimize bytecode |
| P1204 | `INVALID_OPCODE` | Unrecognized opcode encountered | Use valid opcodes from OpCode enum |
| P1205 | `INVALID_JUMP_DESTINATION` | Jump target not a JUMPDEST | Verify jump destinations in bytecode |
| P1206 | `MEMORY_ACCESS_VIOLATION` | Memory access out of bounds | Check memory bounds before access |
| P1207 | `STORAGE_ACCESS_DENIED` | Storage operation not permitted | Verify storage permissions |
| P1208 | `DIVISION_BY_ZERO` | Division or modulo by zero | Validate divisor before operation |
| P1209 | `INTEGER_OVERFLOW` | Arithmetic overflow in 256-bit word | Use checked arithmetic operations |

### P13xx — ZK Proof Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P1301 | `INVALID_PROOF_FORMAT` | ZK proof structure malformed | Regenerate proof with valid format |
| P1302 | `PROOF_VERIFICATION_FAILED` | ZK proof did not verify | Check witness and public inputs |
| P1303 | `UNSUPPORTED_PROOF_SYSTEM` | Proof system not supported | Use Groth16, PLONK, or STARK |
| P1304 | `CIRCUIT_MISMATCH` | Proof circuit doesn't match expected | Use correct circuit for verification |

---

## Layer 2: Jurisdictional Infrastructure

### P20xx — Manifold Path Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P2001 | `NO_PATH_EXISTS` | No compliant path between jurisdictions | Check corridor availability and compliance |
| P2002 | `PATH_CONSTRAINT_VIOLATION` | Path violates specified constraints | Relax constraints or find alternative |
| P2003 | `CORRIDOR_NOT_ACTIVE` | Corridor in path is not active | Wait for corridor activation or reroute |
| P2004 | `COMPLIANCE_BARRIER` | Compliance state blocks path | Obtain required attestations |
| P2005 | `PATH_COST_EXCEEDED` | Path cost exceeds maximum allowed | Increase cost limit or find cheaper path |
| P2006 | `CIRCULAR_PATH_DETECTED` | Path contains cycles | Use acyclic path finding |

### P21xx — Migration State Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P2101 | `INVALID_MIGRATION_STATE` | Migration in unexpected state | Check state machine and retry |
| P2102 | `MIGRATION_TIMEOUT` | Migration phase exceeded timeout | Trigger compensation or retry |
| P2103 | `ATTESTATION_QUORUM_NOT_MET` | Insufficient watcher attestations | Wait for more attestations |
| P2104 | `SOURCE_LOCK_FAILED` | Failed to lock asset at source | Verify asset ownership and retry |
| P2105 | `DESTINATION_VERIFICATION_FAILED` | Destination rejected asset | Check destination compliance requirements |
| P2106 | `COMPENSATION_FAILED` | Saga compensation action failed | Manual intervention required |
| P2107 | `MIGRATION_ALREADY_COMPLETE` | Migration already finalized | No action needed |
| P2108 | `MIGRATION_CANCELLED` | Migration was cancelled | Start new migration if needed |

### P22xx — Bridge Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P2201 | `PREPARE_PHASE_FAILED` | Multi-hop prepare phase failed | Release locks and retry |
| P2202 | `COMMIT_PHASE_FAILED` | Multi-hop commit phase failed | Retry with exponential backoff |
| P2203 | `HOP_TIMEOUT` | Individual hop exceeded timeout | Check corridor status |
| P2204 | `RECEIPT_CHAIN_BROKEN` | Receipt sequence has gaps | Sync receipts from corridor |
| P2205 | `ATOMIC_COMMIT_VIOLATION` | Atomicity guarantee violated | Trigger dispute resolution |

### P23xx — Anchor Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P2301 | `CHAIN_NOT_SUPPORTED` | Target chain not configured | Add chain adapter |
| P2302 | `ANCHOR_SUBMISSION_FAILED` | Failed to submit to L1 | Retry with higher gas |
| P2303 | `CONFIRMATION_TIMEOUT` | Transaction not confirmed in time | Check chain status and resubmit |
| P2304 | `FINALITY_NOT_REACHED` | Insufficient confirmations | Wait for more blocks |
| P2305 | `REORG_DETECTED` | Chain reorganization detected | Resubmit checkpoint |
| P2306 | `CONTRACT_CALL_FAILED` | Smart contract call reverted | Check contract state and inputs |
| P2307 | `INSUFFICIENT_FUNDS` | Not enough ETH for gas | Fund anchor wallet |

---

## Layer 3: Network Coordination

### P30xx — Watcher Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P3001 | `INSUFFICIENT_COLLATERAL` | Watcher collateral below minimum | Deposit additional collateral |
| P3002 | `WATCHER_NOT_REGISTERED` | Watcher DID not in registry | Register watcher first |
| P3003 | `ATTESTATION_SIGNATURE_INVALID` | Attestation signature verification failed | Re-sign with correct key |
| P3004 | `DUPLICATE_ATTESTATION` | Attestation already submitted | No action needed |
| P3005 | `SLASHING_TRIGGERED` | Watcher slashed for violation | Review violation and appeal if invalid |
| P3006 | `BOND_LOCKED` | Bond currently locked | Wait for unlock period |
| P3007 | `QUORUM_SELECTION_FAILED` | Cannot select sufficient watchers | Lower quorum or wait for more watchers |

### P31xx — Security Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P3101 | `NONCE_ALREADY_USED` | Replay attack detected | Use fresh nonce |
| P3102 | `NONCE_EXPIRED` | Nonce outside valid time window | Generate new nonce |
| P3103 | `VERSION_MISMATCH` | Protocol version incompatible | Upgrade to compatible version |
| P3104 | `TIME_LOCK_ACTIVE` | Operation blocked by time lock | Wait for unlock time |
| P3105 | `RATE_LIMIT_EXCEEDED` | Too many requests | Implement backoff |
| P3106 | `SIGNATURE_INVALID` | Cryptographic signature invalid | Verify signing key and data |
| P3107 | `UNAUTHORIZED_CALLER` | Caller not authorized for operation | Check permissions |

### P32xx — Economic Attack Detection

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P3201 | `FRONT_RUNNING_DETECTED` | Front-running attack suspected | Use commit-reveal or private mempool |
| P3202 | `SANDWICH_ATTACK_DETECTED` | Sandwich attack pattern detected | Use MEV protection |
| P3203 | `WHALE_MANIPULATION` | Large position manipulation detected | Apply concentration limits |
| P3204 | `WASH_TRADING_DETECTED` | Wash trading pattern detected | Flag accounts for review |
| P3205 | `COLLUSION_SUSPECTED` | Coordinated watcher behavior | Increase quorum diversity |

---

## Layer 4: Operations

### P40xx — Configuration Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P4001 | `CONFIG_FILE_NOT_FOUND` | Configuration file missing | Create config file or use defaults |
| P4002 | `CONFIG_PARSE_ERROR` | Configuration file malformed | Fix YAML/JSON syntax |
| P4003 | `CONFIG_VALIDATION_FAILED` | Configuration values invalid | Check against schema |
| P4004 | `REQUIRED_CONFIG_MISSING` | Required configuration not set | Set required values |
| P4005 | `ENV_VAR_NOT_SET` | Required environment variable missing | Set environment variable |

### P41xx — Health Check Errors

| Code | Name | Description | Recovery |
|------|------|-------------|----------|
| P4101 | `DEPENDENCY_UNAVAILABLE` | Required dependency not reachable | Check dependency health |
| P4102 | `RESOURCE_EXHAUSTED` | System resource limit reached | Scale or free resources |
| P4103 | `DEGRADED_PERFORMANCE` | System operating below threshold | Investigate bottleneck |
| P4104 | `STARTUP_FAILED` | System failed to initialize | Check logs and config |

---

## Error Response Format

All PHOENIX errors follow RFC 7807 Problem Details format:

```json
{
  "type": "https://phoenix.momentum.inc/errors/P2101",
  "title": "Invalid Migration State",
  "status": 400,
  "detail": "Migration 'mig-abc123' is in state COMPLETED but expected TRANSIT",
  "instance": "/migrations/mig-abc123/advance",
  "code": "P2101",
  "timestamp": "2024-01-15T10:30:00Z",
  "correlation_id": "corr-xyz789",
  "context": {
    "migration_id": "mig-abc123",
    "current_state": "COMPLETED",
    "expected_states": ["TRANSIT"]
  },
  "recovery": {
    "strategy": "CHECK_STATE",
    "message": "Verify migration state before advancing. Migration may already be complete.",
    "docs": "https://docs.momentum.inc/phoenix/migrations#state-machine"
  }
}
```

---

## Recovery Strategies

### Automatic Recovery

| Strategy | Description | Applies To |
|----------|-------------|------------|
| `RETRY_WITH_BACKOFF` | Retry with exponential backoff | P2302, P2303, P4101 |
| `REFRESH_ATTESTATION` | Request fresh attestation | P1102, P1103 |
| `INCREASE_GAS` | Resubmit with higher gas | P2302, P2307 |
| `WAIT_FOR_QUORUM` | Wait for more attestations | P2103, P3007 |
| `COMPACT_STATE` | Reduce state size | P1006, P4102 |

### Manual Recovery

| Strategy | Description | Applies To |
|----------|-------------|------------|
| `MANUAL_INTERVENTION` | Operator action required | P2106, P2205 |
| `CONTACT_SUPPORT` | Escalate to support team | P3205, internal errors |
| `APPEAL_PROCESS` | Initiate dispute resolution | P3005 |

---

## Logging Error Events

All errors emit structured log events:

```json
{
  "level": "error",
  "timestamp": "2024-01-15T10:30:00.123Z",
  "logger": "phoenix.migration",
  "code": "P2101",
  "message": "Invalid migration state transition",
  "correlation_id": "corr-xyz789",
  "context": {
    "migration_id": "mig-abc123",
    "from_state": "TRANSIT",
    "to_state": "INITIATED",
    "actor_did": "did:key:z6Mk..."
  },
  "stack_trace": "..."
}
```

---

## See Also

- [Architecture Overview](./architecture/OVERVIEW.md)
- [Security Model](./architecture/SECURITY-MODEL.md)
- [Incident Response](./operators/INCIDENT-RESPONSE.md)
