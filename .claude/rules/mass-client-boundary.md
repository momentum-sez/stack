---
paths:
  - "mez/crates/mez-mass-client/**"
  - "mez/crates/mez-mass-stub/**"
description: "Rules for the Mass API boundary — the sole gateway to Mass services"
---

# Mass Client Boundary Rules

`mez-mass-client` depends ONLY on `mez-core` (for identifier newtypes). Never import tensors, corridors, packs, VCs, or any other domain crate.

No other crate may make HTTP calls to Mass APIs directly. All Mass communication flows through this crate.

## Mass API URLs

| Primitive | Service | Convention |
|-----------|---------|-----------|
| Entities | organization-info.api.mass.inc | `{base}/organization-info/api/v1/{resource}` |
| Ownership | investment-info (Heroku) | `{base}/investment-info/api/v1/{resource}` |
| Fiscal | treasury-info.api.mass.inc | `{base}/treasury-info/api/v1/{resource}` |
| Identity | Split across consent-info + org-info | No dedicated service yet |
| Consent | consent.api.mass.inc | `{base}/consent-info/api/v1/{resource}` |

## mez-mass-stub

Standalone dev server using DashMap (no Postgres). For testing without live Mass APIs. Depends on mez-core + mez-mass-client only.
