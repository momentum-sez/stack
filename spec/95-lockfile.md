# Lockfile semantics (normative)

Deployments MUST record a resolved, deterministic lockfile called `stack.lock`.

## Purpose

The lockfile ensures:

- deterministic builds,
- reproducible deployments,
- upgrade diffs,
- auditability of which legal/regulatory artifacts were active at a point in time.

## Required contents

`stack.lock` MUST include:

- `stack_spec_version`
- `generated_at`
- `zone_id`
- `profile` (id + version)
- resolved module list:
  - `module_id`, `version`, `variant`
  - parameter values
  - content digests (at least manifest digest)
- overlays:
  - patch digests
- corridors:
  - corridor ids
  - trust anchors digest
  - key rotation digest

Lockfiles MUST validate against `schemas/stack.lock.schema.json`.

