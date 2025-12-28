# proof-key artifacts

This directory holds **content-addressed proof keys** for ZK proof systems.

Convention:

```text
dist/artifacts/proof-key/<digest>.*
```

This type is used for digests like:

- `receipt.zk.verifier_key_digest_sha256`

Suggested usage:

- store a verification key (or a verifying key bundle) as a JSON object or raw bytes
- compute `digest = SHA256(bytes)` (if raw) or `SHA256(JCS(json))` (if JSON)
- publish the artifact into this store so verifiers can resolve it by digest

Because ZK ecosystems differ, the precise on-disk format is intentionally open.
The digest must be computed deterministically.
