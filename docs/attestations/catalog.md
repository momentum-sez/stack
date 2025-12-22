# Attestation types catalog (reference)

This catalog defines common attestation types used across modules and corridors.

> Attestation semantics must be stable over time. If you change semantics, bump MAJOR version.

## Core identity and ownership

- `kyc_verified` — a natural person passed KYC (tiered)
- `kyb_verified` — an entity passed KYB verification
- `bo_verified` — beneficial owners verified to required standard
- `pep_screened` — PEP screening performed (and outcomes captured)
- `sanctions_screened` — sanctions screening performed

## Licensing and supervision

- `license_valid` — license is active and not suspended/revoked
- `periodic_return_filed` — periodic supervisory report filed

## AML/CFT program

- `aml_program_attested` — MLRO appointed, RBA completed, policies in place
- `travel_rule_ready` — travel rule capability exists for virtual asset transfers (if applicable)

## Payments and infrastructure

- `api_security_attested` — API gateway / security baseline attested (mTLS, logging, etc.)
- `incident_reporting_ready` — incident response plan exists and tested

## Safeguarding / reserves

- `safeguarding_attested` — client asset safeguarding controls in place
- `reserves_attested` — reserve backing attestations (stablecoin or custody)

