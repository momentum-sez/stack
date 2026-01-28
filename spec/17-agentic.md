# Chapter 17: Agentic Execution Framework

**MSEZ Stack Specification v0.4.42**

This chapter specifies the Agentic Execution Framework, enabling autonomous asset behavior through environment monitoring, policy-driven trigger evaluation, and deterministic action scheduling.

## Overview

Smart Assets in the MASS Protocol can exhibit autonomous behavior through the Agentic Execution Framework. This framework enables assets to respond to environmental changes without requiring explicit human intervention for each transition, while maintaining full auditability and determinism.

The framework consists of three primary components:

1. **Environment Monitors** — observe external conditions and emit triggers
2. **Policy Evaluation Engine** — processes triggers against defined policies
3. **Action Scheduler** — schedules and executes authorized transitions

## Definition 17.1: Agentic Trigger

An **Agentic Trigger** is an environmental event that may cause autonomous state transitions. Triggers are emitted by environment monitors when observed conditions change.

```
AgenticTrigger := {
    trigger_type: TriggerType,
    data: Map<String, Any>,
    timestamp: ISO8601DateTime
}
```

### Trigger Types

The specification defines the following trigger categories:

**Regulatory Environment Triggers:**
- `sanctions_list_update` — Sanctions list modified (OFAC/EU/UN)
- `license_status_change` — License validity changed
- `guidance_update` — Regulatory guidance published or modified
- `compliance_deadline` — Compliance deadline approaching or passed

**Arbitration Triggers:**
- `dispute_filed` — New dispute filed against asset
- `ruling_received` — Arbitration ruling delivered
- `appeal_period_expired` — Appeal window closed
- `enforcement_due` — Ruling enforcement required

**Corridor Triggers:**
- `corridor_state_change` — Corridor receipt/checkpoint activity
- `settlement_anchor_available` — Settlement finality achieved
- `watcher_quorum_reached` — Watcher attestation threshold met

**Asset Lifecycle Triggers:**
- `checkpoint_due` — Checkpoint creation threshold reached
- `key_rotation_due` — Key rotation schedule triggered
- `governance_vote_resolved` — Governance decision finalized

## Definition 17.2: Agentic Policy

An **Agentic Policy** defines a mapping from triggers to authorized transitions.

```
AgenticPolicy := {
    policy_id: String,
    trigger_type: TriggerType,
    condition: Optional<Predicate>,
    action: TransitionKind,
    authorization_requirement: AuthorizationLevel
}
```

### Authorization Levels

- `automatic` — No additional authorization required
- `quorum` — Requires quorum signature threshold
- `unanimous` — Requires all governors to sign
- `governance` — Requires formal governance vote

### Condition Predicates

Policies may include condition predicates that must evaluate to true for the policy to match:

```
Predicate := 
    | { type: "threshold", field: String, threshold: Number }
    | { type: "equals", field: String, value: Any }
    | { type: "contains", field: String, item: Any }
    | { type: "not_equals", field: String, value: Any }
```

## Theorem 17.1: Agentic Determinism

**Statement:** Given identical trigger events and environment state, agentic execution is deterministic and produces identical state transitions.

**Proof Sketch:** The policy evaluation engine processes policies in a deterministic order (sorted by policy_id). Each condition predicate evaluates deterministically over immutable trigger data. Action scheduling uses deterministic timestamps. Therefore, the complete trigger → evaluation → action flow is reproducible.

**Implications:**
- Audit trails can be replayed and verified
- Multiple nodes will reach identical conclusions
- Testing can rely on deterministic behavior

## Definition 17.3: Standard Policy Library

The specification provides a standard library of common policies:

| Policy ID | Trigger | Condition | Action |
|-----------|---------|-----------|--------|
| `sanctions_auto_halt` | `sanctions_list_update` | `affected_parties contains self` | `HALT` |
| `license_expiry_alert` | `license_status_change` | `new_status == expired` | `HALT` |
| `ruling_enforcement` | `ruling_received` | — | `ARBITRATION_ENFORCE` |
| `checkpoint_auto` | `checkpoint_due` | `receipts_since_last >= 100` | `UPDATE_MANIFEST` |
| `sanctions_freeze` | `sanctions_list_update` | `new_sanctioned == true` | `HALT` |
| `license_suspend` | `license_status_change` | `new_status == suspended` | `HALT` |
| `corridor_failover` | `corridor_state_change` | `change_type == fork_detected` | `HALT` |
| `key_rotation_enforce` | `key_rotation_due` | — | `UPDATE_MANIFEST` |
| `dispute_filed_halt` | `dispute_filed` | — | `HALT` |

## Definition 17.4: Environment Monitor

An **Environment Monitor** continuously observes external conditions and emits triggers when state changes occur.

```
EnvironmentMonitor := {
    monitor_id: String,
    monitor_type: MonitorType,
    mode: PollingMode,
    poll_interval_seconds: Integer,
    poll(): Optional<State>,
    detect_changes(old: State, new: State): List<AgenticTrigger>
}
```

### Monitor Types

**SanctionsListMonitor**
- Watches OFAC, EU, UN sanctions lists
- Tracks specific entities for status changes
- Emits `sanctions_list_update` triggers

**LicenseStatusMonitor**
- Tracks license validity windows
- Monitors approaching expiry dates
- Emits `license_status_change` triggers

**CorridorStateMonitor**
- Observes corridor receipt and checkpoint activity
- Detects forks and settlement anchors
- Emits `corridor_state_change` and `settlement_anchor_available` triggers

**GuidanceUpdateMonitor**
- Tracks regulatory guidance documents
- Monitors compliance deadlines
- Emits `guidance_update` and `compliance_deadline` triggers

**CheckpointDueMonitor**
- Tracks receipt counts since last checkpoint
- Monitors time since last checkpoint
- Emits `checkpoint_due` triggers

## Definition 17.5: Policy Evaluation Engine

The **Policy Evaluation Engine** processes triggers against registered policies.

```
PolicyEvaluator := {
    policies: Map<String, AgenticPolicy>,
    evaluate(trigger: AgenticTrigger, environment: State): List<EvaluationResult>,
    schedule_actions(results: List<EvaluationResult>, asset_id: String): List<ScheduledAction>
}
```

### Evaluation Algorithm

```
EVALUATE(trigger, environment):
    results = []
    FOR policy_id IN sorted(policies.keys()):
        policy = policies[policy_id]
        IF policy.enabled AND policy.trigger_type == trigger.trigger_type:
            condition_met = EVALUATE_CONDITION(policy.condition, trigger, environment)
            results.append(EvaluationResult(policy, condition_met))
    RETURN results
```

## Definition 17.6: Agentic Execution Engine

The **Agentic Execution Engine** coordinates monitors, evaluation, and action scheduling.

```
AgenticExecutionEngine := {
    monitor_registry: MonitorRegistry,
    policy_evaluator: PolicyEvaluator,
    process_trigger(trigger: AgenticTrigger, asset_id: String): List<ScheduledAction>,
    execute_pending_actions(): List<ExecutionResult>
}
```

### Trigger Processing Flow

```
1. Monitor detects state change
2. Monitor emits AgenticTrigger
3. Engine routes trigger to PolicyEvaluator
4. PolicyEvaluator evaluates against all registered policies
5. Matching policies produce ScheduledActions
6. Actions are executed based on authorization requirements
7. All steps are recorded in audit trail
```

## Definition 17.7: Audit Trail

Every agentic execution step is recorded in an immutable audit trail:

```
AuditTrailEntry := {
    entry_id: String,
    entry_type: AuditEntryType,
    timestamp: ISO8601DateTime,
    asset_id: Optional<String>,
    trigger_data: Optional<AgenticTrigger>,
    evaluation_result: Optional<EvaluationResult>,
    action_data: Optional<ScheduledAction>
}
```

### Entry Types

- `trigger_received` — Trigger was received by engine
- `policy_evaluated` — Policy was evaluated against trigger
- `action_scheduled` — Action was scheduled for execution
- `action_executed` — Action was successfully executed
- `action_failed` — Action execution failed
- `action_cancelled` — Action was cancelled

## Security Considerations

### Authorization Enforcement

Actions requiring authorization beyond `automatic` MUST NOT execute until the authorization requirement is satisfied. The engine MUST verify signatures before execution.

### Rate Limiting

Monitors SHOULD implement rate limiting to prevent trigger flooding. The recommended minimum poll interval is 60 seconds for most monitor types.

### Audit Trail Integrity

Audit trail entries MUST be immutable once written. Implementations SHOULD consider anchoring audit trail digests to corridor receipts for additional integrity guarantees.

## Schema References

- `schemas/agentic.environment-monitor.schema.json`
- `schemas/agentic.trigger.schema.json`
- `schemas/agentic.policy.schema.json`
- `schemas/agentic.policy-evaluation.schema.json`
- `schemas/agentic.action-schedule.schema.json`
- `schemas/agentic.audit-trail.schema.json`

## Implementation Notes

### Determinism Requirements

To satisfy Theorem 17.1:

1. Policy evaluation order MUST be deterministic (sort by policy_id)
2. Condition evaluation MUST be pure (no side effects)
3. Timestamp generation MUST use consistent precision
4. Action scheduling MUST be reproducible given same inputs

### Monitor Lifecycle

Monitors follow a state machine:

```
STOPPED → STARTING → RUNNING → STOPPING → STOPPED
                  ↓
                ERROR
                  ↓
                STOPPED
```

The `PAUSED` state is available for temporary suspension without full restart.

### Testing

The agentic framework supports trigger simulation for testing:

```bash
msez agent trigger emit checkpoint_due --payload '{"receipts_since_last": 150}'
```

This allows testing policy behavior without actual environment changes.
