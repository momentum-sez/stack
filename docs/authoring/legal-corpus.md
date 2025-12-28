# Legal corpus modules

The Momentum SEZ Stack ships **templates + schemas + tooling** to represent law as modular, versioned,
composable artifacts.

It does **not** ship a redistributed copy of “all laws everywhere” inside the core repository, because:

- many corpora are extremely large (and change frequently),
- licensing / redistribution rules vary by jurisdiction,
- we want deterministic provenance: every imported corpus must have a traceable source + digest.

## What is in this repo

This repo includes **placeholder jurisdiction corpus modules** under:

`modules/legal/jurisdictions/<jurisdiction_path>/<domain>/`

Each jurisdiction corpus module typically contains:

- `module.yaml` — module identity (module_id/version/license)
- `sources.yaml` — source URLs/references + normalization recipe notes
- `src/akn/**` — normalized Akoma Ntoso XML (placeholder content in scaffold)

The set of known jurisdictions is maintained in `registries/jurisdictions.yaml`.

## Lawpack supply chain (v0.4.1)

v0.4.1 introduces **Lawpacks**: content-addressed legal corpus artifacts (Akoma Ntoso + indices + provenance).

For a jurisdiction corpus module, run:

```bash
python tools/msez.py law ingest modules/legal/jurisdictions/us/ca/civil \
  --as-of-date 2025-01-01 \
  --fetch
```

This emits:

- `dist/lawpacks/<jurisdiction_id>/<domain>/<digest>.lawpack.zip`
- `<module_dir>/lawpack.lock.json`

Digest semantics, artifact format, and indexing are specified in `spec/96-lawpacks.md`.

### Fetching sources

`--fetch` is a best-effort scaffold:
it downloads declared `sources.yaml` URIs into `src/raw/` and records their digests in provenance.
High-fidelity HTML/PDF → Akoma Ntoso normalization is intentionally recipe-driven and pluggable.

### Including raw sources in the artifact

If licensing permits and you want a self-contained archive, add:

```bash
--include-raw
```

(By default, raw sources are not embedded in the zip.)

## Binding into stack.lock

Zones pin lawpacks in `stack.lock`, just like corridor/security artifacts:

```bash
python tools/msez.py lock jurisdictions/_starter/zone.yaml --out jurisdictions/_starter/stack.lock
```

The lock generator will emit a `lawpacks[]` section based on:

- `zone.yaml` `jurisdiction_id`
- optional `zone.yaml` `jurisdiction_stack` (layered governing law)
- optional `zone.yaml` `lawpack_domains` (defaults to `civil` + `financial`)
- discovered `lawpack.lock.json` files under `modules/legal/jurisdictions/...`

## Corridors and legal compatibility

Corridors can bind to lawpack digests via:

- **Corridor Definition VC**: declares required domains and optional allow-lists of compatible digests
- **Corridor Agreement VC**: each participant signs their pinned lawpack digest(s)

See `spec/40-corridors.md` and `spec/96-lawpacks.md`.
