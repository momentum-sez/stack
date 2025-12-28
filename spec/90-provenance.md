# Provenance and IP (normative)

To ensure the stack remains open and redistributable:

- Each module SHOULD include a provenance file describing sources and licenses.
- Build tooling SHOULD refuse to publish artifacts that are not redistributable without explicit rights.

Recommended file:

- `sources/provenance.yaml`



## Lawpack provenance (v0.4.1+)

Lawpacks carry explicit provenance for legal corpora:

- `lawpack.yaml` declares sources, licensing assertions, and the normalization recipe used to produce Akoma Ntoso.
- `lawpack.lock.json` binds the emitted artifact to its canonical digest and component hashes.

Zones pin lawpacks in `stack.lock` (see `spec/96-lawpacks.md` and `spec/95-lockfile.md`).

