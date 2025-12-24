# Network diffusion and experimental propagation (v0.4+ scaffold)

This section defines minimal, general-purpose scaffolding for safely propagating modules, profiles, and corridor
patterns across a network of zones.

The goal is to combine:

- **determinism** (lockfiles, content hashes)
- **measurement** (telemetry + success metrics)
- **safe rollout** (A/B tests, staged adoption)

## Operational modules

The reference repository includes placeholder operational modules for:

- `operational/deployment-telemetry` — defines telemetry event schemas and collection conventions
- `operational/success-metric-registry` — defines how success metrics are named, versioned, and reported
- `operational/ab-testing-framework` — defines experiment descriptors and audit expectations

These modules are intentionally thin: they standardize *interfaces* and *data models* so that different vendors can
implement them without breaking interoperability.

## Telemetry events (normative-ish)

A conforming telemetry system SHOULD:

- emit structured events that include:
  - `zone_id`, `jurisdiction_id`
  - `stack.lock` digest (or `stack_spec_version` + module set digests)
  - event type, timestamp, and privacy classification
- avoid leaking personal data by default; prefer aggregation and minimization

## Success metrics

Zones SHOULD track outcome metrics such as:

- time to business registration
- time to license issuance
- cost of compliance
- payment settlement latency
- dispute resolution cycle time
- corridor activation time and failure causes

Metrics MUST be:

- **named** (stable identifiers)
- **versioned** (schema evolution)
- **auditable** (measurement method recorded)

## Experiments and staged rollout

Experiments SHOULD be represented as content-addressed descriptors that declare:

- hypothesis and scope
- treatment/control arms
- safety constraints
- metric bindings
- rollback criteria

Where experiments modify legal/regulatory artifacts, they MUST be reflected via overlays and pinned in `stack.lock`.

