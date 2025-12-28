# lawpack artifacts

This directory is the **content-addressed store** for lawpack artifacts.

Convention:

```text
dist/artifacts/lawpack/<digest>.lawpack.zip
```

Where `<digest>` is the lawpack digest (`lawpack_digest_sha256`) computed over canonicalized content
(metadata + index + Akoma Ntoso docs), as defined in `spec/96-lawpacks.md`.

Populate from any locally built lawpacks:

```bash
python tools/msez.py artifact index-lawpacks
```
