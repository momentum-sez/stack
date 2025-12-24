# zk-participation (baseline)

This module defines the mechanism-specific semantics for **privacy-preserving participation patterns (zk eligibility, anonymity sets).**

It is intentionally a thin specification scaffold so different implementations can remain interoperable.

## Inputs

- proposal object: `msez.gov.proposal.v1`
- votes/ballots: `msez.gov.vote.v1`

## Output

- tally/outcome: `msez.gov.tally.v1`

## Notes

- This module does **not** mandate a particular on-chain / off-chain implementation.
- Zones should bind the outcome to audited events (see `modules/operational/audit-logging`) and, when relevant,
  to legal/regulatory artifacts via overlays + lockfiles.
