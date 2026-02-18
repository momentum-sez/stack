# United States - South Carolina — civil corpus (placeholder)

This module is a **placeholder** for the civil legal corpus for **United States - South Carolina** (`us-sc`).

## Why is it empty?

The core `momentum-ez/stack` repository ships **schemas, templates, and tooling** — not a full redistributed copy of every jurisdiction's statutes and regulations.

Reasons:
- licensing / redistribution constraints vary by jurisdiction,
- corpora are very large and update frequently,
- we require provenance + hashing for deterministic compilation.

## How to populate

1. Add authoritative references to `sources.yaml`.
2. Use the law ingestion pipeline (planned) to fetch, normalize, and convert to Akoma Ntoso (`src/akn/main.xml` and/or additional documents).

Until populated, this module exists so profiles and zones can reference a stable `module_id` and later bind to a specific corpus digest in `stack.lock`.
