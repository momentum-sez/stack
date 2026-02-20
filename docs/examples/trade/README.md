# Trade Playbook Example

This directory contains a jurisdiction-sharded trade corridor playbook skeleton.

The goal is to make the example read like an operator manual rather than a loose
collection of files:

- two zones (exporter/importer)
- jurisdiction stacks ("lawpack overlays") pinned via zone locks
- settlement rails (SWIFT ISO20022 + regulated stablecoin)
- trade flow instruments (invoice, BOL, letter of credit)

## Trade Playbook Profile

The bundle that defines the baseline modules and corridors for the playbook lives at:

- profiles/trade-playbook/profile.yaml

Zones in this playbook reference that profile so that the zone lock pins an operator-
reproducible module set.

## Zone Manifests

The playbook zone manifests are located here:

- docs/examples/trade/src/zones/exporter/zone.yaml
- docs/examples/trade/src/zones/importer/zone.yaml

Both files are full zone manifests (schemas/zone.schema.json) so they can be locked
and used as the stable substrate for agreements, receipts, checkpoints, and portable
witness bundles.

To generate locks:

```bash
mez lock docs/examples/trade/src/zones/exporter/zone.yaml
mez lock docs/examples/trade/src/zones/importer/zone.yaml
```

## Trade Flow API

The trade flow instruments runtime is implemented in `mez-corridor` with 4 archetypes
(Export, Import, LetterOfCredit, OpenAccount) and 10 transition types. API endpoints
are available at `/v1/trade/flows/*`. See the [Corridor Overview](../../corridors/overview.md)
for the full trade flow lifecycle documentation.
