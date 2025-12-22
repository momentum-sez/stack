# Mass primitives mapping

The stack assumes a set of core programmable primitives that legal and regulatory modules can map to.

## Primitives

- **Entities** — incorporation, amendments, dissolution, registry state
- **Ownership** — cap tables, beneficial ownership, transfers, encumbrances
- **Instruments** — securities, tokens, accounts, contracts, funds
- **Identity** — KYC/KYB, identity passports, attestations, risk tiers
- **Consent** — signatures, resolutions, approvals, delegated authority

## Requirement mapping

Modules SHOULD provide `src/policy-to-code/map.yaml` to map requirements to:

- applicable legal references (eIds / URIs),
- primitive(s),
- attestations,
- regulator view(s),
- evidence / artifact references.

