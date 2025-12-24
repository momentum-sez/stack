# Governance core

This module provides the minimum interoperability surface for governance/consent mechanisms in the MSEZ Stack.

It intentionally **does not** dictate political design. Instead it standardizes the common objects that any
governance engine needs to exchange:

- proposals
- votes / ballots
- delegations
- tallies / outcomes
- (optional) privacy-preserving eligibility proofs

Other governance modules should depend on this module and define mechanism-specific semantics.
