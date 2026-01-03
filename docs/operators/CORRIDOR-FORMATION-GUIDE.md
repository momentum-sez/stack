# Corridor Formation Guide

This guide describes an end-to-end corridor formation workflow:

1. define a corridor module
2. issue a corridor definition VC
3. issue participant-specific corridor agreement VCs (thresholded)
4. start the corridor state channel (receipts)
5. add checkpoints + watchers for scalability and integrity

It assumes both zones already have pinned lawpacks and an authority registry chain.

## 0) Create a corridor module directory

Create `modules/corridors/<corridor-name>/` with at minimum:

- `corridor.yaml`
- `dist/artifacts/` (for pinned artifacts and outputs)
- `security/` (trust anchors, key rotation policies)

Validate it:

```bash
python3 tools/msez.py corridor validate modules/corridors/example-corridor
```

## 1) Issue a Corridor Definition VC

Create an unsigned corridor definition, then sign it:

```bash
python3 tools/msez.py vc sign \
  --in docs/examples/vc/unsigned.corridor-definition.json \
  --key keys/zone-authority.jwk.json \
  --out dist/artifacts/vc/
```

Pin the resulting VC digest in the corridor module (and in zone lockfiles).

## 2) Issue participant-specific Corridor Agreement VCs

Each corridor participant issues a party-specific agreement VC.

Agreements can include:

- pinned lawpacks (party-specific)
- checkpoint policy
- watcher quorum policy

## 3) Start the state channel

The state channel is an append-only sequence of signed receipts.

Receipts SHOULD be produced by authorized receipt signers and MUST include:

- `(prev_root, next_root)`
- the committed lawpack/ruleset digest sets
- optional transition registry digest commitments

## 4) Produce checkpoints

Checkpoints summarize the corridor head and MMR root for fast sync.

```bash
python3 tools/msez.py corridor state checkpoint \
  --corridor modules/corridors/example-corridor \
  --out dist/artifacts/checkpoint/
```

## 5) Add watchers

Independent watchers can issue watcher attestation VCs committing to the observed head.

```bash
python3 tools/msez.py corridor state watcher-attest \
  --corridor modules/corridors/example-corridor \
  --checkpoint dist/artifacts/checkpoint/<digest>.checkpoint.json \
  --key keys/watcher.jwk.json \
  --out dist/artifacts/vc/
```

Aggregate attestations to detect forks cheaply:

```bash
python3 tools/msez.py corridor state watcher-compare modules/corridors/example-corridor \
  --vcs ./watcher-attestations \
  --quorum-threshold majority \
  --require-quorum \
  --max-staleness 1h
```

See `docs/operators/INCIDENT-RESPONSE.md` for how to react to alarms.
