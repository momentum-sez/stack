# Data classification (reference)

This scheme is used to minimize data exposure and support privacy-by-design.

## Classes

- **C0 Public**: safe to publish (aggregates, anonymized metrics)
- **C1 Internal**: operational logs without PII
- **C2 Confidential**: business confidential (contracts, financials)
- **C3 Restricted PII**: personal data (KYC docs, IDs)
- **C4 Regulator Access Data**: minimized supervisory data (attestations, hashes, timestamps)

## Rules

- Prefer storing C3 data outside the regulator console; expose C4 instead.
- Cross-border transfers of C3 require legal mechanism + DPIA.
- All access to C3/C4 MUST be audited.

