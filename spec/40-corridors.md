# Corridors (normative)

A **corridor** is a cross-border interoperability arrangement between zone nodes and external participants.

Corridors MUST define:

- settlement model and limits
- required attestations and trust anchors
- dispute escalation path
- key rotation and revocation policy
- data-sharing expectations (minimization)

Corridor manifests MUST conform to `schemas/corridor.schema.json`.

Trust anchors MUST be expressed as machine-readable artifacts conforming to:
- `schemas/trust-anchors.schema.json`
- `schemas/key-rotation.schema.json`

