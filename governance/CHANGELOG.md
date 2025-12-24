# Changelog

## 0.3.0 — Unreleased

### New in v0.3 (corridor cryptographic substrate)

- Corridor Agreement VC participant-specific party semantics (`credentialSubject.party`, `party_terms`, `commitment`)
- Multiple agreement VC files (per-party) supported and validated (threshold activation)
- `msez corridor status` CLI for human + JSON activation status
- `stack.lock` includes corridor agreement digests + best-effort activation status when configured

## 0.2.0 — 2025-12-22

### New in v0.2

- Full normative spec additions (mission, invariants, rubric, architecture, lockfile semantics, provenance)
- Zone manifest (`zone.yaml`) schema and starter jurisdiction
- Lockfile schema and `msez lock` generator
- Corridor trust anchors + key rotation as machine-readable artifacts and schemas
- Conformance test suite (pytest), including policy-to-code completeness checks for MUST/SHALL clauses
- Akoma schema validation support (XSD fetch + validation)
- Akoma render pipeline (XSLT -> HTML, optional PDF)
- Expanded taxonomy: stub modules for legal/regulatory/financial/licensing/corridors/ops
- CI workflow expanded to fetch Akoma schemas and run conformance suite

## 0.1.0 — 2025-12-21

- Initial expanded skeleton: Akoma templates, licensing pack, corridors, docs, CI, validator
