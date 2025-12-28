# circuit artifacts

This directory holds **content-addressed ZK circuit artifacts** referenced by digest in:
- transition type registry entries (`zk_circuit_digest_sha256`), and
- per-receipt overrides (`transition.zk_proof.circuit_digest_sha256`).

Convention:

```text
dist/artifacts/circuit/<digest>.*
```

The concrete circuit format is ecosystem-defined (e.g., `.r1cs`, `.wasm`, `.zkey`, `.json`),
but the **digest MUST be a sha256 commitment** pinned in receipts and/or registries.

Store artifacts:

```bash
python tools/msez.py artifact store circuit <digest> <path>
```
