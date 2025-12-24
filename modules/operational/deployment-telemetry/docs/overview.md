# Telemetry overview

A conforming telemetry implementation SHOULD emit events that include:

- `zone_id` and `jurisdiction_id`
- `stack_spec_version`
- `stack_lock_sha256` (recommended)
- a privacy classification

Events are designed to be used by dashboards, regulators (read-only), and experimentation frameworks.
