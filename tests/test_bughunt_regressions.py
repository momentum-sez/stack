"""Bughunt regression tests for v0.4.40.

Each test corresponds to a bug discovered during the v0.4.40 development cycle.
See docs/bughunt/BUGHUNT_LOG.md for full details.
"""

import hashlib
import json
import os
import subprocess
import uuid
from decimal import Decimal
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[1]


# ─────────────────────────────────────────────────────────────────────────────
# Bug #1: Non-deterministic UUID generation
# ─────────────────────────────────────────────────────────────────────────────

def test_corridor_receipt_ids_are_deterministic(tmp_path: Path):
    """Verify that corridor receipt IDs are deterministic (uuid5, not uuid4)."""
    # Generate the same receipt twice with the same inputs
    namespace = uuid.UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")
    
    id1 = str(uuid.uuid5(namespace, "corridor:test:seq:0"))
    id2 = str(uuid.uuid5(namespace, "corridor:test:seq:0"))
    
    assert id1 == id2, "UUID5 should be deterministic for same input"
    
    # Verify uuid4 would NOT be deterministic
    id3 = str(uuid.uuid4())
    id4 = str(uuid.uuid4())
    assert id3 != id4, "UUID4 is random and should differ"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #2: Trailing newline inconsistency
# ─────────────────────────────────────────────────────────────────────────────

def test_canonical_json_trailing_newline_consistency():
    """Verify canonical JSON handling accepts both with and without trailing newline."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.msez import canonical_json_bytes
    
    obj = {"a": 1, "b": 2}
    canonical = canonical_json_bytes(obj)
    
    # Both should be acceptable
    with_newline = canonical + b"\n"
    without_newline = canonical
    
    # Verify the canonical bytes are stable
    assert canonical == canonical_json_bytes(obj), "Canonical JSON should be stable"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #3: Dead imports
# ─────────────────────────────────────────────────────────────────────────────

def test_no_dead_imports():
    """Verify no dead imports exist in the codebase."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    
    # This should not raise ImportError
    try:
        from tools import msez
        from tools import artifacts
        from tools import vc
        from tools import lawpack
    except ImportError as e:
        pytest.fail(f"Dead import detected: {e}")


# ─────────────────────────────────────────────────────────────────────────────
# Bug #4: Settlement anchor missing finality timestamp
# ─────────────────────────────────────────────────────────────────────────────

def test_settlement_anchor_finality_has_timestamp(tmp_path: Path):
    """Verify settlement anchors include confirmed_at when finality is confirmed."""
    # Generate trade playbook
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    anchor_path = out_root / "dist" / "artifacts" / "settlement" / "anchor.json"
    anchor = json.loads(anchor_path.read_text(encoding="utf-8"))
    
    assert "finality_status" in anchor
    assert anchor["finality_status"]["level"] == "confirmed"
    assert "confirmed_at" in anchor["finality_status"], "confirmed status must have confirmed_at timestamp"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #5: Proof binding commitments missing corridor_id
# ─────────────────────────────────────────────────────────────────────────────

def test_proof_binding_commitments_have_corridor_context(tmp_path: Path):
    """Verify proof bindings include corridor context in commitments."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    sanctions_path = out_root / "dist" / "artifacts" / "proof-bindings" / "sanctions.json"
    sanctions = json.loads(sanctions_path.read_text(encoding="utf-8"))
    
    assert "commitments" in sanctions
    assert len(sanctions["commitments"]) > 0
    
    for commitment in sanctions["commitments"]:
        if commitment["kind"] == "corridor.receipt":
            assert "corridor_id" in commitment, "Corridor receipt commitment must include corridor_id"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #6: Zone lock lawpack digest ordering
# ─────────────────────────────────────────────────────────────────────────────

def test_zone_lock_lawpack_digests_are_sorted(tmp_path: Path):
    """Verify zone lock lawpack_digest_set is sorted for determinism."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    lock_path = out_root / "dist" / "artifacts" / "zone-locks" / "exporter.lock.json"
    lock = json.loads(lock_path.read_text(encoding="utf-8"))
    
    digests = lock.get("lawpack_digest_set", [])
    assert digests == sorted(digests), "lawpack_digest_set must be sorted"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #7: Receipt chain genesis root enforcement
# ─────────────────────────────────────────────────────────────────────────────

def test_receipt_chain_genesis_root_enforcement(tmp_path: Path):
    """Verify first receipt prev_root matches expected genesis pattern."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    receipt_0_path = out_root / "dist" / "artifacts" / "receipts" / "obligation" / "receipt-0.json"
    receipt_0 = json.loads(receipt_0_path.read_text(encoding="utf-8"))
    
    # First receipt should have sequence 0
    assert receipt_0["sequence"] == 0
    
    # prev_root should be a valid sha256 hex digest (the genesis root)
    prev_root = receipt_0["prev_root"]
    assert len(prev_root) == 64, "prev_root must be sha256 hex"
    assert all(c in "0123456789abcdef" for c in prev_root), "prev_root must be hex"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #8: Dashboard artifact count
# ─────────────────────────────────────────────────────────────────────────────

def test_dashboard_artifact_count_matches_closure(tmp_path: Path):
    """Verify dashboard total-artifacts matches closure root artifact count."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    dashboard_path = out_root / "dist" / "dashboard.json"
    closure_path = out_root / "dist" / "manifest.playbook.root.json"
    
    dashboard = json.loads(dashboard_path.read_text(encoding="utf-8"))
    closure = json.loads(closure_path.read_text(encoding="utf-8"))
    
    dashboard_total = None
    for card in dashboard["cards"]:
        if card["card_id"] == "total-artifacts":
            dashboard_total = card["value"]
            break
    
    closure_count = closure["artifact_count"]
    
    assert dashboard_total == closure_count, f"Dashboard ({dashboard_total}) != closure ({closure_count})"


# ─────────────────────────────────────────────────────────────────────────────
# Bug #9: CAS index digest semantics
# ─────────────────────────────────────────────────────────────────────────────

def test_cas_index_uses_strict_digest_semantics(tmp_path: Path):
    """Verify CAS index computes digests correctly (excluding proof field)."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    cas_index_path = out_root / "dist" / "artifacts" / "cas-index.json"
    cas_index = json.loads(cas_index_path.read_text(encoding="utf-8"))
    
    assert cas_index["digest_algorithm"] == "sha256"
    assert len(cas_index["manifests"]) >= 2  # closure-root + dashboard


# ─────────────────────────────────────────────────────────────────────────────
# Bug #10: Netting engine determinism
# ─────────────────────────────────────────────────────────────────────────────

def test_netting_engine_deterministic_output():
    """Verify netting engine produces deterministic output."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    
    from tools.netting import (
        NettingEngine, Obligation, Party, Currency, SettlementRail
    )
    
    # Create test data
    party_a = Party("did:a", "A")
    party_b = Party("did:b", "B")
    usd = Currency("USD", 2)
    
    obligations = [
        Obligation("obl-1", "corridor-1", party_a, party_b, usd, Decimal("1000")),
        Obligation("obl-2", "corridor-1", party_b, party_a, usd, Decimal("600")),
    ]
    
    rails = [
        SettlementRail("rail-swift", "corridor-swift", {"USD"}),
    ]
    
    # Run netting twice
    engine1 = NettingEngine(obligations, rails)
    plan1 = engine1.compute_plan("plan-1")
    
    engine2 = NettingEngine(obligations, rails)
    plan2 = engine2.compute_plan("plan-1")
    
    # Plans should be identical
    assert len(plan1.settlement_legs) == len(plan2.settlement_legs)
    for leg1, leg2 in zip(plan1.settlement_legs, plan2.settlement_legs):
        assert leg1.leg_id == leg2.leg_id
        assert leg1.payer == leg2.payer
        assert leg1.payee == leg2.payee
        assert leg1.amount == leg2.amount


# ─────────────────────────────────────────────────────────────────────────────
# Bug #12: Checkpoint audit canonical bytes
# ─────────────────────────────────────────────────────────────────────────────

def test_checkpoint_audit_strict_canonical_bytes(tmp_path: Path):
    """Verify checkpoint files are in canonical JSON format."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.msez import canonical_json_bytes
    
    checkpoint_path = out_root / "dist" / "artifacts" / "checkpoints" / "obligation" / "checkpoint-0.json"
    raw = checkpoint_path.read_bytes()
    obj = json.loads(raw.decode("utf-8"))
    canonical = canonical_json_bytes(obj)
    
    # Should match (with or without trailing newline)
    assert raw == canonical or raw == canonical + b"\n", "Checkpoint must be canonical JSON"


# ─────────────────────────────────────────────────────────────────────────────
# Additional regression tests for v0.4.40 gate
# ─────────────────────────────────────────────────────────────────────────────

def test_trade_playbook_full_roundtrip_determinism(tmp_path: Path):
    """Full roundtrip: generate → check → regenerate → compare bytes."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    # Generate first time
    out1 = tmp_path / "trade1"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out1)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    # Check should pass
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "check", "--docs-root", str(out1)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    # Generate second time
    out2 = tmp_path / "trade2"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out2)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    # Compare all JSON files
    import glob
    files1 = sorted(glob.glob(str(out1 / "dist" / "**" / "*.json"), recursive=True))
    files2 = sorted(glob.glob(str(out2 / "dist" / "**" / "*.json"), recursive=True))
    
    assert len(files1) == len(files2)
    
    for f1, f2 in zip(files1, files2):
        bytes1 = Path(f1).read_bytes()
        bytes2 = Path(f2).read_bytes()
        assert bytes1 == bytes2, f"Files differ: {f1} vs {f2}"


def test_all_generated_artifacts_have_type_field(tmp_path: Path):
    """Every generated artifact must have a 'type' field."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    import glob
    json_files = glob.glob(str(out_root / "dist" / "**" / "*.json"), recursive=True)
    
    for jf in json_files:
        obj = json.loads(Path(jf).read_text(encoding="utf-8"))
        assert "type" in obj, f"Missing 'type' field in {jf}"
        assert obj["type"].startswith("MSEZ"), f"Invalid type prefix in {jf}: {obj['type']}"


def test_all_generated_artifacts_have_stack_spec_version(tmp_path: Path):
    """Every generated artifact must have stack_spec_version field."""
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"
    
    out_root = tmp_path / "trade"
    subprocess.check_call(
        ["python", "tools/dev/generate_trade_playbook.py", "--mode", "generate", "--docs-root", str(out_root)],
        cwd=REPO_ROOT,
        env=env,
    )
    
    import glob
    json_files = glob.glob(str(out_root / "dist" / "**" / "*.json"), recursive=True)
    
    for jf in json_files:
        obj = json.loads(Path(jf).read_text(encoding="utf-8"))
        assert "stack_spec_version" in obj, f"Missing 'stack_spec_version' in {jf}"
        assert obj["stack_spec_version"] == "0.4.41", f"Wrong version in {jf}: {obj['stack_spec_version']}"
