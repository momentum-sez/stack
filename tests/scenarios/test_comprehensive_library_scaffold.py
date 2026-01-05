"""Scenario scaffolds derived from the comprehensive test library.

These are intentionally skipped by default.

Enable with:
  MSEZ_RUN_SCAFFOLD=1 pytest -q
"""

from __future__ import annotations

import itertools
import os

import pytest


pytestmark = pytest.mark.scaffold


if os.environ.get("MSEZ_RUN_SCAFFOLD") != "1":
    pytest.skip("scaffold scenarios are disabled by default (set MSEZ_RUN_SCAFFOLD=1)", allow_module_level=True)


BASE_SCENARIOS: list[tuple[str, str]] = [
    # Receipt Chain Integrity (examples)
    ("receipt/genesis_root_mismatch", "Reject receipt[0].prev_root != genesis_root"),
    ("receipt/next_root_mismatch", "Reject receipt where computed next_root != declared next_root"),
    ("receipt/sequence_non_monotonic", "Reject non-monotonic receipt sequence"),
    ("receipt/replay_duplicate_digest", "Detect replayed receipt by identical digest with different timestamp"),
    ("receipt/proof_invalid_signature", "Reject invalid Ed25519 proof"),
    ("receipt/proof_wrong_vm", "Reject proof with verificationMethod mismatch"),
    ("receipt/unknown_corridor_id", "Reject receipt.corridor_id that doesn't match module corridor_id"),

    # Fork Detection & Resolution
    ("fork/same_prev_two_next", "Detect fork: two receipts share prev_root but diverge next_root"),
    ("fork/watchers_diverge_same_height", "Watcher quorum flags divergent final_state_root at same receipt_count"),
    ("fork/resolution_selects_canonical", "fork-resolution VC selects canonical head and halts other branches"),
    ("fork/resolution_invalid_authority", "Reject fork-resolution VC signed by non-authorized authority registry"),
    ("fork/auto_halt_on_alarm", "Lifecycle policy transitions OPERATIONAL->HALTED on fork alarm"),

    # Checkpoint Operations
    ("checkpoint/build_from_receipts", "Checkpoint from receipts produces deterministic head_commitment"),
    ("checkpoint/mmr_inclusion_proof", "MMR inclusion proof verifies against checkpoint.mmr_root"),
    ("checkpoint/invalid_mmr_path", "Invalid MMR path rejected"),
    ("checkpoint/epoch_policy_violation", "Checkpoint policy enforces min_receipts / max_lag"),
    ("checkpoint/receipt_inclusion_privacy", "Inclusion proof does not reveal non-included receipts"),

    # Lifecycle State Machine
    ("lifecycle/proposed_to_operational_requires_threshold", "Require corridor agreement threshold to activate"),
    ("lifecycle/operational_to_halted_requires_alarm", "Require fork-alarm evidence for HALTED"),
    ("lifecycle/halted_to_operational_requires_resolution", "Require fork-resolution VC to resume"),
    ("lifecycle/deprecated_blocks_new_receipts", "Reject new receipts when corridor is DEPRECATED"),

    # Finality Semantics
    ("finality/proposed", "Finality level: proposed"),
    ("finality/receipt_signed", "Finality level: receipt_signed"),
    ("finality/checkpoint_signed", "Finality level: checkpoint_signed"),
    ("finality/watcher_quorum", "Finality level: watcher_quorum"),
    ("finality/l1_anchored", "Finality level: l1_anchored"),
    ("finality/legally_recognized", "Finality level: legally_recognized"),

    # Lawpacks
    ("lawpack/lock_digest_matches", "lawpack.lock digest matches canonical content digest"),
    ("lawpack/availability_attestation", "Lawpack availability attestations require each party holds copies"),
    ("lawpack/migration_successor_chain", "Lawpack succession VC links L1->L2 and preserves provenance"),

    # Authority Registry
    ("authority/registry_chain_valid", "Treaty->National->Zone chain verifies and scopes actions"),
    ("authority/key_rotation", "Key rotation VC updates authority keys w/ ceremony policy"),

    # Dispute & Arbitration
    ("dispute/file_claim", "Dispute claim VC created and validated"),
    ("dispute/arbitration_award", "Arbitration award VC validated against arbitrator registry"),

    # Integration (stubs)
    ("integration/swift_iso20022_stub", "SWIFT ISO 20022 adapter stub validates payload schema"),
    ("integration/usdc_circle_stub", "USDC adapter stub validates idempotency and receipts"),
]


def _expand_variations(base: list[tuple[str, str]]) -> list[tuple[str, str]]:
    """Create a *large* matrix of concrete variants from the base scenarios.

    This is a scaffold generator meant to surface real-world ambiguity classes:
      - digest formatting and boundary cases
      - unicode normalization and encoding pitfalls
      - clock skew and sequencing semantics
      - network partitions and delayed delivery

    These scenarios are intentionally numerous (order-of-magnitude expansion).
    """

    digests = [
        "all_zero_digest",
        "all_ff_digest",
        "non_hex_digest",
        "uppercase_hex",
        "short_digest",
        "long_digest",
        "leading_space",
        "trailing_space",
        "embedded_newline",
        "0x_prefixed",
        "mixed_case",
        "unicode_homoglyphs",
    ]
    encodings = [
        "utf8",
        "utf8_nfc",
        "utf8_nfd",
        "utf8_nfkc",
        "utf8_nfkd",
        "latin1",
        "windows1252",
        "utf16le",
    ]
    clocks = [
        "clock_skew_-5m",
        "clock_skew_+5m",
        "clock_skew_+24h",
        "clock_skew_-24h",
        "clock_monotonic",
        "clock_backward_jump",
        "clock_forward_jump",
        "timestamp_missing",
        "timestamp_null",
        "timestamp_non_iso",
    ]
    network = [
        "partition_none",
        "partition_split_brain",
        "partition_eclipse",
        "delayed_delivery_1s",
        "delayed_delivery_5m",
        "delayed_delivery_24h",
    ]

    out: list[tuple[str, str]] = []
    for sid, desc in base:
        # Base case
        out.append((sid, desc))

        # Single-axis variants
        for d in digests:
            out.append((f"{sid}/digest/{d}", f"{desc} (digest variant: {d})"))
        for e in encodings:
            out.append((f"{sid}/encoding/{e}", f"{desc} (encoding variant: {e})"))
        for c in clocks:
            out.append((f"{sid}/time/{c}", f"{desc} (time variant: {c})"))
        for n in network:
            out.append((f"{sid}/net/{n}", f"{desc} (network variant: {n})"))

        # High-leverage intersections (limited cross product to avoid infinite blow-up)
        for d, e in itertools.product(digests[:4], encodings[:4]):
            out.append((
                f"{sid}/digest/{d}/encoding/{e}",
                f"{desc} (digest {d} + encoding {e})",
            ))
        for c, n in itertools.product(clocks[:3], network[:3]):
            out.append((
                f"{sid}/time/{c}/net/{n}",
                f"{desc} (time {c} + network {n})",
            ))

    return out


SCENARIOS = _expand_variations(BASE_SCENARIOS)


@pytest.mark.parametrize("scenario_id, description", SCENARIOS)
def test_scenario_scaffold(scenario_id: str, description: str) -> None:
    pytest.skip(f"Scaffold scenario not yet implemented: {scenario_id} — {description}")
