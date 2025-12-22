# Canonical Repository Layout

This repository is organized as follows:

- `spec/` — normative specification documents
- `modules/` — module sources (model laws, regs, policy-as-code, forms, workflows)
- `profiles/` — profiles (bundles of modules + parameters)
- `schemas/` — JSON Schema definitions for manifests and payloads
- `apis/` — OpenAPI specifications
- `tools/` — validators/builders/publishers
- `registries/` — registries for IDs and compatibility

The `stack` repository is intended to be **vendored** (copied or pinned) by an implementing jurisdiction and then customized via overlays.

