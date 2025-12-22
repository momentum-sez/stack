# Module system (normative)

A **module** is the atomic unit of composition in the stack.

## Module manifest

Each module MUST include a `module.yaml` that conforms to `schemas/module.schema.json`.

Key required properties:

- `module_id` (globally unique)
- `version` (semver)
- `kind` (legal, regulatory, licensing, financial, corridor, operational, api, data)
- `license`
- `depends_on` (explicit dependencies)
- `provides` (interfaces + artifact paths)
- `variants` (hot-swappable choices)

## Variants

A module MAY ship multiple variants. Variants MUST remain compatible within a MAJOR version for the interfaces they provide.

## Dependencies

Dependencies MUST be explicit. Build tools MUST refuse to produce a bundle with unresolved dependencies.

## Provenance

Modules SHOULD document provenance for any included legal text or external sources. If redistribution rights are unclear,
deployments SHOULD reference the source rather than include it.

