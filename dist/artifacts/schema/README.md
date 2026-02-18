# schema artifacts

This directory holds **content-addressed JSON Schemas** referenced by digest.

Convention:

```text
dist/artifacts/schema/<digest>.*
```

Digest semantics (recommended):

- `schema_digest_sha256 = SHA256(JCS(schema_json))`

Where **JCS** is the JSON Canonicalization Scheme (RFC 8785).

This is the digest model used by:
- `transition_types[*].schema_digest_sha256` in transition type registries / lock snapshots.

Populate (reference tool):

```bash
python -m tools.mez artifact index-schemas
```
