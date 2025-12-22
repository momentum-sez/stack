# Standard structure

This repository defines an **open standard** in the same spirit as widely adopted engineering standards:

- normative specification text with MUST/SHOULD/MAY language,
- registries for stable identifiers,
- schemas for machine validation,
- conformance tests.

## Normative keywords

The keywords **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL**
are to be interpreted as described in RFC 2119 and RFC 8174.

## Normative vs informative

- `spec/` is normative unless explicitly marked informative.
- `docs/` is informative guidance.

## Stability expectations

- Registries and interface IDs SHOULD be stable.
- Compatibility is governed by semantic versioning at:
  - Stack spec level,
  - module level,
  - interface level.

