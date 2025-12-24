# Conformance (normative)

Conformance is evaluated at three levels:

1. **Module conformance**
2. **Profile conformance**
3. **Zone deployment conformance**

## Module conformance

A module is conformant if:

- `module.yaml` validates against the module schema
- all `provides.path` artifacts exist
- any Akoma Ntoso XML under `src/akn/` validates against the Akoma schema (when schema is available)
- policy-to-code map exists when the module includes MUST/SHALL legal obligations (see policy-to-code completeness)
- if `src/policy-to-code/map.yaml` exists, it MUST validate against `schemas/policy-to-code.schema.json`
- if the module kind is `corridor`, `corridor.yaml` MUST validate against `schemas/corridor.schema.json`

## Profile conformance

A profile is conformant if:

- `profile.yaml` validates against the profile schema
- all referenced modules exist and match pinned versions
- variants exist and satisfy interface requirements
 - all transitive `depends_on` dependencies are present in the profile
 - any corridor IDs listed in `profile.yaml -> corridors` resolve to a corridor manifest

## Zone conformance

A zone deployment is conformant if:

- `zone.yaml` validates against the zone schema
- `stack.lock` validates against the lock schema and matches the resolved build
- corridor modules include trust anchors and key rotation policies
- corridor modules MUST include a Corridor Definition VC (definition_vc_path) that cryptographically binds the corridor manifest + security artifacts (hashes + signature)
- regulator console constraints are met (read-only, audited, revocable)

## Policy-to-code completeness (MUST clause mapping)

If a module includes legal text with normative MUST/SHALL obligations, the module MUST include a policy-to-code map such that:

- each MUST/SHALL clause is referenced by an entry with:
  - `legal_refs` referencing the clause anchor (`eId`) or stable URI, and
  - `attestation.type` describing the proof emitted/checked.

The reference conformance suite implements a best-effort extraction of MUST/SHALL statements from Akoma XML.

