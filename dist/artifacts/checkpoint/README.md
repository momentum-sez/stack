# checkpoint artifacts

This directory holds **content-addressed corridor state checkpoints**.

Convention:

```text
dist/artifacts/checkpoint/<digest>.*
```

Digest semantics (recommended):

- `checkpoint_digest_sha256 = SHA256(JCS(checkpoint_without_proof))`

This matches the digest model used by:

- `MSEZCorridorReceiptInclusionProof.checkpoint_digest_sha256`

Notes:

- The digest is computed over the checkpoint payload excluding `proof`.
- A checkpoint file stored under this digest MAY include one or more signatures/proofs.

Populate:

- use `python -m tools.msez corridor state checkpoint ...` to create a checkpoint
- then store it with `python -m tools.msez artifact store checkpoint <digest> <path>`
