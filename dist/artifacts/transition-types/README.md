# transition-types artifacts

This directory is a **content-addressed store** for **Transition Type Registry lock snapshots**.

Convention:

```text
dist/artifacts/transition-types/<digest>.transition-types.lock.json
```

Where `<digest>` is the snapshot digest (`snapshot_digest_sha256`) computed as:

```
SHA256(JCS(snapshot))
```

Corridor receipts can commit to a registry snapshot by including:
- `transition_type_registry_digest_sha256`

This makes receipt verification possible against historical snapshots even if the corridor module updates its registry.
