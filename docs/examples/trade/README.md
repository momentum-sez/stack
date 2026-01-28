# Trade Playbook Example

This directory contains a jurisdiction-sharded trade corridor playbook skeleton.

The goal is to make the example read like an operator manual rather than a loose
collection of files:

- two zones (exporter/importer)
- jurisdiction stacks ("lawpack overlays") pinned via zone locks
- settlement rails (SWIFT ISO20022 + regulated stablecoin)
- room for obligation corridors (invoice/BOL/LC) and proof-bindings

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
./msez zone lock --zone docs/examples/trade/src/zones/exporter/zone.yaml
./msez zone lock --zone docs/examples/trade/src/zones/importer/zone.yaml
```

## Deterministic generation

The deterministic generator entrypoint is:

- tools/dev/generate_trade_playbook.py

At this stage, the generator only scaffolds the on-disk layout and root manifest.
Subsequent parts of the .40 gate expand it to generate corridor instances, receipts,
checkpoints, settlement anchors, proof-bindings, and a complete CAS closure.
