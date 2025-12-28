# dist/artifacts

This directory is the **generic content-addressed store (CAS)** for all stack artifacts.

Convention:

```text
dist/artifacts/<type>/<digest>.*
```

Where:
- `<type>` is a lowercase artifact category (example set: `lawpack`, `ruleset`, `transition-types`, `circuit`, `schema`, `vc`, `checkpoint`, `proof-key`, `blob`).
- `<digest>` is a `sha256` hex string (lowercase).
- the filename suffix is free-form but SHOULD communicate semantics (e.g., `.lawpack.zip`, `.schema.json`, `.vc.json`).

Use the CLI:

```bash
python tools/msez.py artifact resolve <type> <digest>
python tools/msez.py artifact store <type> <digest> <path>
```

Index helpers (optional):

```bash
python tools/msez.py artifact index-rulesets
python tools/msez.py artifact index-lawpacks
python tools/msez.py artifact index-schemas
python tools/msez.py artifact index-vcs
```

This CAS exists so that any digest commitment appearing in receipts / VCs has an **obvious resolution path** in a repository checkout.
