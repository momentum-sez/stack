# Corridors overview

A **corridor** is a cross-border interoperability arrangement between zone nodes.

This stack provides corridor modules for:

- SWIFT / ISO20022 messaging (`modules/corridors/swift/`)
- stablecoin settlement corridors (`modules/corridors/stablecoin-settlement/`)
- open-banking / account-to-account corridors (`modules/corridors/open-banking/`)

Each corridor module contains:

- a `corridor.yaml` manifest (machine-readable)
- a playbook (`docs/playbook.md`) for legal, compliance, and ops steps
- a `required-attestations.yaml` list (what proofs are required for corridor eligibility)

