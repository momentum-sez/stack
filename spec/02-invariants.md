# Invariants

These invariants are **non-negotiable** for conformance with the MSEZ Stack standard.

## Versioning

- Legal texts MUST use semantic versioning (MAJOR.MINOR.PATCH).
- Breaking changes MUST increment MAJOR.

## Modularity

- Modules MUST declare dependencies explicitly.
- Modules MUST be independently deployable where feasible.

## Overrides

- Jurisdiction deployments MUST use overlays rather than editing upstream module source.
- Overlay application MUST be deterministic and recorded in a lockfile.

## Dual-format accessibility

- Akoma Ntoso XML MUST be treated as the canonical source format for legal texts.
- Human-readable render outputs (HTML/PDF) MUST be build outputs derived from canonical source.

## Protocol integration hooks

- Modules SHOULD include policy-to-code mappings that connect legal requirements to:
  - Mass primitives,
  - attestations,
  - regulator console views,
  - and compliance controls.

## Regulator console

- Regulator access MUST be:
  - read-only,
  - role-gated,
  - audited (every request),
  - revocable,
  - and data-minimized (prefer attestations over raw PII).

