# Smart Asset module

This directory is an **operator-friendly, jurisdiction-shardable** container for running a Smart Asset without requiring a blockchain.

**Key files**

- `asset.yaml` — operational manifest (purposes, trust anchors, replication targets, expected transition registry lock)
- `trust-anchors.yaml` — authorized receipt/checkpoint signers
- `key-rotation.yaml` — suggested key rotation policy

**State directories**

- `state/receipts/` — `SmartAssetReceipt` JSON files (append-only)
- `state/fork-resolutions/` — `ForkResolution` JSON files (canonical-chain selection for forks)
- `state/checkpoints/` — `SmartAssetReceiptChainCheckpoint` JSON files (MMR head commitments)
- `state/proofs/` — `SmartAssetReceiptInclusionProof` JSON files

**CAS**

- `dist/artifacts/` can be used as a local content-addressed store root for any resolved artifacts.

**CLI**

Once initialized, you can operate on the module directory directly:

```bash
# create a signed receipt
python3 -m tools.mez asset state receipt-init modules/smart-assets/<asset_id> \
  --sequence 0 --prev-root genesis --sign --key keys/operator.jwk.json

# verify the chain
python3 -m tools.mez asset state verify modules/smart-assets/<asset_id>
```


**Portable audit packets (witness bundles)**

Export a self-contained witness bundle (zip) that includes the module state + referenced artifacts:

```bash
python3 -m tools.mez asset module witness-bundle modules/smart-assets/<asset_id> \
  --out /tmp/asset-module.witness.zip --json

# offline / air-gapped verification
python3 -m tools.mez artifact graph verify --from-bundle /tmp/asset-module.witness.zip --strict --json
```


## Multi-jurisdiction scope hints (optional)

When issuing receipts in a multi-harbor / sharded compliance configuration, you can embed *scope hints* into the receipt
(these fields are committed into `next_root` and therefore are audit-stable):

```bash
mez asset state receipt-init \
  --asset-id EXAMPLE_ASSET \
  --sequence 2 \
  --transition transition.json \
  --jurisdiction-scope quorum \
  --harbor-id ae-adgm \
  --harbor-id us-de \
  --harbor-quorum 2 \
  --prev-receipt receipt-1.json \
  --sign --key ./keys/asset-operator.jwk.json --verification-method did:key:...
```

## Rule evaluation evidence (optional)

Harbors (or other evaluators) can emit portable evidence artifacts that are attachable to any transition envelope:

```bash
mez asset rule-eval-evidence-init \
  --transition transition.json \
  --harbor-id ae-adgm \
  --result pass \
  --sign --key ./keys/harbor.jwk.json --verification-method did:key:... \
  --store
```

The resulting artifact (type `rule-eval-evidence`) can be referenced from a transition envelope via `attachments`, and will be
pulled into witness bundles automatically.
