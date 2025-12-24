# Security & Privacy Baseline

This document defines baseline security and privacy expectations for MSEZ stacks.

## Data minimization

- Prefer attestations and hashes over raw personal data in shared systems.
- Regulator Console SHOULD consume C4 "Regulator Access Data" wherever feasible.

## Auditability

- Every privileged access MUST produce an audit event.
- Audit events MUST be immutable in storage (append-only) and retained per policy.

## Key management

- Attestation issuers MUST support key rotation.
- Trust anchors MUST be documented per corridor and per regulator.

### Corridor integrity binding

- Corridor manifests SHOULD be distributed with a signed Corridor Definition VC (see `spec/40-corridors.md`).
- Validators SHOULD refuse to activate corridor modules whose VC hash bindings do not match the shipped artifacts.


## Cross-border data sharing

- Corridors MUST document data-sharing purpose, legal basis, and minimization.
- Deployments SHOULD maintain DPIA artifacts for C3/C4 transfers.
