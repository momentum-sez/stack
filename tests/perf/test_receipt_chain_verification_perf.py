import hashlib
import json
import os
import time
from pathlib import Path

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization


@pytest.mark.perf
def test_receipt_chain_verification_time(tmp_path: Path):
    """Perf harness: verify a long receipt chain end-to-end.

    Skipped unless MSEZ_RUN_PERF=1.

    Configure:
      - MSEZ_PERF_RECEIPTS: number of receipts to generate (default: 10_000; try 100_000)

    Notes:
      This test is intentionally non-assertive on runtime budgets by default because
      CI environments vary. It prints throughput metrics for operator benchmarking.
    """
    from tools.msez import (
        REPO_ROOT,
        corridor_state_genesis_root,
        corridor_expected_ruleset_digest_set,
        corridor_state_next_root,
        _corridor_state_build_chain,
    )
    from tools.lawpack import jcs_canonicalize
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key

    count = int(os.environ.get("MSEZ_PERF_RECEIPTS", "10000"))

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # Use a single signer key to focus the benchmark on verification + chain logic.
    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir(parents=True, exist_ok=True)

    prev = genesis
    for i in range(count):
        payload = {"i": i}
        payload_sha256 = hashlib.sha256(jcs_canonicalize(payload)).hexdigest()
        r = {
            "type": "MSEZCorridorStateReceipt",
            "corridor_id": "org.momentum.msez.corridor.swift.iso20022-cross-border",
            "sequence": i,
            "timestamp": "2025-01-01T00:00:00Z",
            "prev_root": prev,
            "lawpack_digest_set": [],
            "ruleset_digest_set": ruleset_set,
            "transition": {
                "type": "MSEZTransitionEnvelope",
                "kind": "generic",
                "payload": payload,
                "payload_sha256": payload_sha256,
            },
        }
        r["next_root"] = corridor_state_next_root(r)
        add_ed25519_proof(r, priv, vm)
        (receipts_dir / f"{i:08d}.json").write_text(json.dumps(r))
        prev = str(r["next_root"])

    t0 = time.perf_counter()
    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_dir,
        enforce_trust_anchors=False,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
        from_checkpoint_path=None,
    )
    dt = time.perf_counter() - t0

    assert not errors
    assert int(result.get("receipt_count") or 0) == count

    # Print operator-friendly metrics.
    rate = (count / dt) if dt > 0 else 0.0
    print(f"verified {count} receipts in {dt:.3f}s ({rate:.1f} receipts/s)")

    # Optional: enforce a soft budget if provided.
    budget_ms = os.environ.get("MSEZ_PERF_BUDGET_MS", "").strip()
    if budget_ms:
        budget = float(budget_ms) / 1000.0
        assert dt <= budget, f"verification exceeded budget: {dt:.3f}s > {budget:.3f}s"
