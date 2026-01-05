# vc artifacts

This directory holds **content-addressed Verifiable Credentials (VCs)** referenced by digest.

Convention:

```text
dist/artifacts/vc/<digest>.*
```

Digest semantics (recommended):

- `vc_payload_digest_sha256 = SHA256(JCS(vc_without_proof))`

This matches the binding used throughout the corridor substrate:

- `definition_payload_sha256 = SHA256(JCS(definition_vc_without_proof))`

Notes:

- The digest is over the VC payload excluding the `proof` field, enabling multi-party co-signing
  without changing the payload digest.
- A VC file stored under this digest MAY contain one or more proofs.

Populate (reference tool):

```bash
python -m tools.msez artifact index-vcs
```
