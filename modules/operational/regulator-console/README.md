# Regulator Console (Operational Module)

This module provides the **control plane** requirements for regulator access:

- read-only access
- role-based authorization
- full audit logging for every query
- revocation capabilities
- data minimization: prefer attestations over raw PII

Artifacts:
- OpenAPI spec: `api/regulator-console.openapi.yaml`
- OPA access policy: `policy/access.rego`
- Audit event schema: `schemas/audit-event.schema.json`

