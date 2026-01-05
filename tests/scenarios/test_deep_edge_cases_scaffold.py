"""Deep edge-case scaffolds.

These represent adversarial, race, parsing, and protocol-evolution scenarios.

By default these are skipped (marker: scaffold). Enable with:
  MSEZ_RUN_SCAFFOLD=1 pytest -q
"""

from __future__ import annotations

import itertools
import os

import pytest


pytestmark = pytest.mark.scaffold


if os.environ.get("MSEZ_RUN_SCAFFOLD") != "1":
    pytest.skip("scaffold scenarios are disabled by default (set MSEZ_RUN_SCAFFOLD=1)", allow_module_level=True)


EDGE_CASES: list[tuple[str, str]] = [
    # Race conditions / concurrency
    ("race/simultaneous_receipt_proposals_same_sequence", "Deterministic tiebreak or halt on duplicate sequence"),
    ("race/receipt_during_checkpoint_ceremony", "Checkpoint ceremony handles new receipts deterministically"),
    ("race/key_rotation_mid_signature_collection", "Key rotation does not invalidate partially-collected signatures"),
    ("race/watcher_attestations_clock_skew", "Watcher quorum robust to clock skew / timestamp mismatch"),
    ("race/split_brain_partition_divergent_chains", "Partition leads to fork alarms and lifecycle halt"),

    # Cryptographic edge cases
    ("crypto/ed25519_noncanonical_signature_rejected", "Reject non-canonical Ed25519 signature encodings"),
    ("crypto/proof_with_unknown_suite", "Reject proofs with unknown cryptosuite"),
    ("crypto/did_urlencoded_special_chars", "DID parsing rejects url-encoded edge cases"),
    ("crypto/all_zero_digest_committed", "Treat all-zero digest as suspicious and policy-gated"),
    ("crypto/reused_nonce_like_behavior", "Detect identical signature bytes across different signing inputs"),

    # Data integrity / parsing
    ("data/unicode_normalization_payload", "Payload normalization handles NFC vs NFD correctly"),
    ("data/json_number_precision", "Reject unsafe integers or normalize via JCS rules"),
    ("data/null_vs_missing_semantics", "Distinguish null, missing, and empty string in canonicalization"),
    ("data/circular_artifact_reference", "Detect circular artifact references and fail require-artifacts"),
    ("data/invalid_date_time_formats", "Reject non-ISO8601 date-time fields"),

    # Watcher adversarial
    ("watcher/sybil_quorum_detection", "Detect likely Sybil watchers using same authority chain"),
    ("watcher/equivocation_two_attestations", "Watcher cannot issue contradictory attestations without slashing"),
    ("watcher/eclipse_attack", "Network partition / eclipse does not produce false finality"),
    ("watcher/bond_under_collateralized", "Reject watcher bonds without sufficient collateral evidence"),

    # Resource limits
    ("limits/sequence_overflow", "Reject sequence > max safe integer"),
    ("limits/receipt_chain_100k", "Verify receipt chain with 100k receipts within target time"),
    ("limits/watcher_compare_1000_watchers", "Compare scaling with 1000 watcher attestations"),

    # Protocol evolution
    ("evolution/old_client_new_schema", "Old client handles new optional fields without mis-verifying"),
    ("evolution/historical_receipts_new_validation", "Historical receipts verified under schema migration policy"),
]



def _expand(edge: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Expand deep edge cases into a large matrix.

    We keep this generator deterministic and purely combinatorial: it should be
    safe to run in CI when enabled, and it should give implementers a clear
    surface for systematically closing correctness gaps.
    """

    # Repeat each edge case across common ambiguity dimensions.
    time = ["t0", "t+1s", "t+5m", "t+24h", "t-5m", "t-24h"]
    encoding = ["nfc", "nfd", "nfkc", "nfkd"]
    net = ["no_partition", "partition", "split_brain", "eclipse"]
    byz = ["honest", "byzantine", "equivocating", "withholding"]

    out: list[tuple[str, str]] = []
    for sid, desc in edge:
        out.append((sid, desc))
        for t in time:
            out.append((f"{sid}/time/{t}", f"{desc} (time={t})"))
        for e in encoding:
            out.append((f"{sid}/encoding/{e}", f"{desc} (unicode={e})"))
        for n in net:
            out.append((f"{sid}/net/{n}", f"{desc} (net={n})"))
        for b in byz:
            out.append((f"{sid}/byz/{b}", f"{desc} (byz={b})"))

        # High-leverage intersections
        for n in net[:3]:
            for b in byz[:3]:
                out.append((f"{sid}/net/{n}/byz/{b}", f"{desc} (net={n}, byz={b})"))
    return out


SCENARIOS = _expand(EDGE_CASES)


@pytest.mark.parametrize("scenario_id, description", SCENARIOS)
def test_edge_case_scaffold(scenario_id: str, description: str) -> None:
    pytest.skip(f"Scaffold edge case not yet implemented: {scenario_id} — {description}")
