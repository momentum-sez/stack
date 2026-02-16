# Mass API OpenAPI Spec Snapshots

Committed snapshots of live Mass API OpenAPI 3.0 specs. These are the
**reference truth** for contract tests that validate Rust client types against
the actual Java/Spring Boot API schemas.

## Spec Sources

| File | Service | Live URL |
|------|---------|----------|
| `organization-info.openapi.json` | Entities | `organization-info.api.mass.inc/organization-info/v3/api-docs` |
| `investment-info.openapi.json` | Ownership (investments) | `investment-info-production-4f3779c81425.herokuapp.com/investment-info/v3/api-docs` |
| `treasury-info.openapi.json` | Fiscal | `treasury-info.api.mass.inc/treasury-info/v3/api-docs` |
| `consent-info.openapi.json` | Consent + Cap Tables | `consent.api.mass.inc/consent-info/v3/api-docs` |
| `templating-engine.openapi.json` | Document Templating | `templating-engine-prod-5edc768c1f80.herokuapp.com/templating-engine/v3/api-docs` |

## Refreshing Specs

Run the refresh script from the repository root:

```bash
./msez/crates/msez-mass-client/scripts/refresh-specs.sh
```

This fetches all five specs from the live APIs and overwrites the files in this
directory. After refreshing:

1. Run `cargo test -p msez-mass-client contract_test` to check for drift
2. Review any test failures — they indicate field renames, type changes, or
   new required fields in the Java services
3. Update the Rust client types in `msez-mass-client/src/` to match
4. Commit the updated specs alongside the Rust type fixes

## Staleness Check

The contract test suite includes `#[ignore]`d staleness tests that fetch live
specs and compare against these snapshots:

```bash
cargo test -p msez-mass-client contract_staleness -- --ignored
```

If any spec has drifted, the test prints a message indicating which service
changed. Run the refresh script and review the diff.

## Notes

- These specs may require VPN or network access to the Mass API hosts
- The `consent-info` service hosts both consent AND cap table/ownership endpoints
- All specs use Spring Boot's `/v3/api-docs` (SpringDoc OpenAPI) convention
