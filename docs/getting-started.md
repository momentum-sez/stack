# Getting started

This repository is structured as a **standard + library**:

- `spec/` is normative: it defines *how* the system works
- `modules/` are reusable building blocks
- `profiles/` are deployable bundles ("styles")
- `jurisdictions/` contain overlays for a real deployment

## Quick commands

```bash
python -m pip install -r tools/requirements.txt
python -m tools.msez validate profiles/digital-financial-center/profile.yaml
python -m tools.msez validate --all-modules
python -m tools.msez build profiles/digital-financial-center/profile.yaml --out dist/
```

## How to adopt

1. Choose a profile under `profiles/`
2. Create a new jurisdiction folder under `jurisdictions/<your-id>/`
3. Add overlays under `jurisdictions/<your-id>/overlays/` rather than editing upstream modules
4. Commit `zone.yaml` + `stack.lock`

