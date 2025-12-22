# Authoring modules

## Module checklist (MUST)

Each module MUST include:

- `module.yaml` manifest
- `README.md`
- `src/` source artifacts
- `tests/` conformance tests (at least manifest validation)

## Variants

If your module has policy choices, implement them as variants:

```
modules/<kind>/<name>/variants/<variant-name>/
```

Each variant SHOULD re-use shared source artifacts where possible.

## Dependencies

Declare dependencies in `module.yaml: depends_on`.
Dependencies MUST be explicit; do not rely on implicit build ordering.

