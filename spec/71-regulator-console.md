# Regulator console (normative)

A conformant regulator console MUST provide:

- read-only access
- role-based access control
- full audit logging for every request attempt (allow or deny)
- revocation mechanisms (disable roles/keys)
- data minimization: attestations and hashes should be preferred over raw PII

The canonical API surface is defined in `modules/operational/regulator-console/api/regulator-console.openapi.yaml`.

