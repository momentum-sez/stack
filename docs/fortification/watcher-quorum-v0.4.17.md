# Watcher quorum and compact head commitments (v0.4.17)

This note explains the operational intent behind the v0.4.17 watcher upgrades.

## Why watcher quorum matters

Receipt verification is the ground truth, but it is expensive to distribute receipts everywhere and to keep every observer perfectly synced.

Watcher attestations let you detect fork-like divergence *cheaply*: multiple independent parties sign what they believe the corridor head is.

By adding a **quorum policy** (K-of-N watchers), operations can treat watcher signals as a first-class liveness gate while keeping receipts private and localized.

## Compact head commitments

Attestations now include `head_commitment_digest_sha256`, a deterministic digest over stable head fields.

This enables gossip and caching systems to dedupe identical heads even if:

- the underlying checkpoint JSON differs in timestamps
- watchers attach different URIs or metadata

## Operational recommendation

For production corridors:

1. configure an authority registry allow-list for watchers
2. set a watcher quorum policy in the corridor agreement (`state_channel.watcher_quorum`)
3. run `watcher-compare` continuously and alert on:
   - fork-like divergence (critical)
   - quorum loss (liveness)

Example:

```bash
python3 tools/msez.py corridor state watcher-compare modules/corridors/<corridor> \
  --vcs ./watcher-attestations \
  --quorum-threshold 3/5 \
  --require-quorum \
  --max-staleness 1h
```

This produces an auditable, machine-readable report and supports instant fork alarms without receipt transport.