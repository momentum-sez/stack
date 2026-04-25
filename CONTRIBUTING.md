# Contributing

## Adding a Jurisdiction

1. Fork this repo
2. Copy an example profile from `examples/` to start
3. Edit `zone.yaml` with your jurisdiction's details
4. Add jurisdiction-specific operation overrides in `operations/`
5. Run `make validate`
6. Open a PR if contributing upstream

## Adding an Operation

1. Create `operations/{primitive}/your-operation.yaml`
2. Follow the schema in `schemas/operation.schema.json`
3. Ensure all `depends_on` references point to valid step IDs (no cycles)
4. Run `make validate`

## Reporting Issues

- Configuration bugs: open a GitHub issue with your `zone.yaml` (redact secrets)
- Runtime bugs: file against the runtime image referenced in `deploy/`
- Security issues: see [SECURITY.md](SECURITY.md)

## Code of Conduct

Be respectful. This project serves sovereign institutions. Contributions
should be precise, well-tested, and documented.
