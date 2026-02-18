# liquid-democracy (baseline)

This module defines the mechanism-specific semantics for **liquid democracy (delegation) configuration and semantics.**

It is intentionally a thin specification scaffold so different implementations can remain interoperable.

## Inputs

- proposal object: `mez.gov.proposal.v1`
- votes/ballots: `mez.gov.vote.v1`

## Output

- tally/outcome: `mez.gov.tally.v1`

## Notes

- This module does **not** mandate a particular on-chain / off-chain implementation.
- Zones should bind the outcome to audited events (see `modules/operational/audit-logging`) and, when relevant,
  to legal/regulatory artifacts via overlays + lockfiles.
