# Conformance tests

This folder contains the **reference conformance suite**.

Run:

```bash
pip install -r tools/requirements.txt
pytest -q
```

## Optional suites

Some suites are *intentionally* skipped by default to keep CI fast and deterministic.

Enable them via environment flags:

```bash
# Slow correctness suites
MEZ_RUN_SLOW=1 pytest -q

# Performance/benchmark suites
MEZ_RUN_PERF=1 pytest -q

# Large scenario scaffolds (roadmap test matrix; mostly TODO stubs)
MEZ_RUN_SCAFFOLD=1 pytest -q
```

CI runs this suite on every push/PR.

