# Architecture overview

The MEZ Stack is a layered system. Deployments SHOULD implement all layers relevant to their target profile.

## Layer A — Enabling Authority (public law)

- enabling act / charter
- delegation of authority and rulemaking
- administrative procedure and appeals
- boundary definitions (physical and/or digital)

## Layer B — Legal operating system (private law)

- civil and commercial rules
- entity registry, property registries
- dispute resolution mechanisms

## Layer C — Regulatory supervision

- AML/CFT and sanctions
- market conduct and consumer protection
- cybersecurity and data protection
- financial supervision framework

## Layer D — Financial infrastructure

- domestic payments
- domestic banking adapters
- safeguarding and treasury controls
- settlement (fiat and/or token rails)

## Layer E — Corridors (cross-border)

- correspondent banking corridors (SWIFT/ISO20022)
- stablecoin settlement corridors
- open-banking / A2A corridors
- passporting / mutual recognition corridors

## Layer F — Observability & control plane

- regulator console (read-only, audited)
- attestation streams
- audit logging, incident response, transparency reporting

## Layer G — Civic governance & diffusion (optional)

- consent mechanisms (voting, delegation, quadratic mechanisms)
- privacy-preserving participation patterns (e.g., ZK eligibility proofs)
- network diffusion and experimentation (telemetry, success metrics, A/B tests)


In v0.4.1, zones and corridors additionally bind to **Lawpacks**: content-addressed legal corpus snapshots (Akoma Ntoso + indices) pinned in `stack.lock` and referenced by corridor VCs (see `spec/96-lawpacks.md`).
