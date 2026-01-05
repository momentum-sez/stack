# Incident Response

This document describes a practical response playbook for corridor integrity incidents.

The stack provides several primitives that are designed to make incident response **fast** and **verifiable**:

- watcher attestations and quorum monitoring (cheap fork alarms)
- fork alarm credentials (evidence-backed, when receipts are available)
- checkpoint finality policy enforcement
- key rotation artifacts and ceremonies

## 1) Detect divergence (cheap)

Run watcher compare:

```bash
python3 -m tools.msez corridor state watcher-compare modules/corridors/<corridor> \
  --vcs ./watcher-attestations \
  --quorum-threshold majority \
  --require-quorum \
  --max-staleness 1h
```

Interpretation:

- **FORK-LIKE**: same `receipt_count`, different `final_state_root` → treat as critical
- **LAG**: different `receipt_count` only → treat as out-of-sync; investigate propagation

## 2) Halt the corridor

If a fork-like divergence is detected, operators SHOULD halt receipt production until the incident is resolved.

## 3) Collect evidence

If possible, obtain:

- the conflicting receipts
- the last known-good checkpoint
- watcher attestations supporting each branch

Store them as content-addressed artifacts.

## 4) Receipt-level fork forensics (when receipts exist)

If you can collect receipts (even from multiple operators / watchers), generate a fork report:

```bash
python3 -m tools.msez corridor state fork-inspect modules/corridors/<corridor>   --receipts path/to/receipts   --format json   --out fork-report.json
```

If you already have fork resolution artifacts (VCs or raw `MSEZCorridorForkResolution` JSON), include them to check coverage and compute the canonical head:

```bash
python3 -m tools.msez corridor state fork-inspect modules/corridors/<corridor>   --receipts path/to/receipts   --fork-resolutions path/to/fork-resolutions   --format text
```

Interpretation:

- `forks.unresolved > 0`: halt and resolve before producing new receipts.
- `forks.resolved == forks.total`: the report includes a `canonical_head` you can checkpoint against.

## 5) Issue a fork alarm VC (when receipts exist)

If you can identify two receipts with the same `(sequence, prev_root)` but different `next_root`, issue a fork alarm VC:

```bash
python3 -m tools.msez corridor state fork-alarm \
  --corridor modules/corridors/<corridor> \
  --receipt-a path/to/receiptA.json \
  --receipt-b path/to/receiptB.json \
  --key keys/watcher-or-operator.jwk.json \
  --out dist/artifacts/vc/
```

## 6) Contain key compromise

If key compromise is suspected:

- rotate keys using the configured key rotation policy
- update the authority registry chain as required
- publish the updated artifacts and new digests

## 7) Resume with a checkpoint

Once the authoritative branch is determined (via operator consensus, arbitration, or external anchoring), resume operations by producing a checkpoint and requiring downstream consumers to sync from that checkpoint.

## 8) Publish a post-mortem

Publish a short post-mortem including:

- what happened and which final_state_root is authoritative
- which artifact digests contain the evidence
- what mitigations were added (watcher policy changes, key rotations)

This makes the incident machine-verifiable for auditors and downstream integrations.
