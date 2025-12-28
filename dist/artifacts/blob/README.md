# blob artifacts

This directory holds **raw byte blobs** referenced by digest.

Convention:

```text
dist/artifacts/blob/<digest>.*
```

This type is used for digest commitments that refer to **raw bytes** (unstructured payloads) but must still be resolvable
for “commitment completeness”, notably:

- `transition.attachments[*]` when `artifact_type = "blob"` (or when `artifact_type` is omitted for legacy compatibility)
- `receipt.zk.proof_sha256`

Digest semantics:

- `digest_sha256 = SHA256(bytes)`

Where `bytes` are the exact raw bytes of the referenced file/object (PDF, JSON, proof bytes, manifest, etc.).

Because blobs are intentionally unstructured, the on-disk filename suffix is free-form.
Suggested practice is to preserve a meaningful extension (e.g., `.pdf`, `.json`, `.bin`) while ensuring
the leading `<digest>` matches the SHA256 of the stored bytes.
