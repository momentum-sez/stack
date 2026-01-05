# Performance harness

The MSEZ stack includes **performance-oriented tests** under `tests/perf/`.

These are *skipped by default* so CI and normal local test runs stay fast.

## Running perf tests

Enable perf tests via an environment variable:

```bash
MSEZ_RUN_PERF=1 pytest -q
```

### Tuning workload sizes

Some perf tests take a tunable workload size from env vars:

- `MSEZ_PERF_RECEIPTS` — number of receipts to verify (default: `10000`).
- `MSEZ_PERF_WATCHERS` — number of watcher attestations to compare (default: `100`).

Example (100k receipt chain):

```bash
MSEZ_RUN_PERF=1 MSEZ_PERF_RECEIPTS=100000 pytest -q -k receipt_chain_verification_time -s
```

## Notes

- These tests print timing / throughput to stdout. They are intended as a **regression guard** and a **tuning aid**, not as a strict benchmark.
- If you need stable numbers, pin CPU frequency scaling and run on a quiet machine.
