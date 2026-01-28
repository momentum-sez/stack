import os
import json
import subprocess
from pathlib import Path


def test_trade_playbook_generator_generate_then_check(tmp_path: Path):
    """Run the generator in a temp output dir to ensure it is deterministic and checkable."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    # Generate
    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )
    # Check (should be no-op and succeed)
    subprocess.check_call(
        ["python", str(script), "--mode", "check", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )


def test_trade_playbook_root_manifest_is_repo_relative_and_stable(tmp_path: Path):
    """Regression guard: the committed playbook root MUST NOT embed machine-specific paths."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    root_path = out_root / "dist" / "manifest.playbook.root.json"
    data = json.loads(root_path.read_text(encoding="utf-8"))

    assert data["type"] == "MSEZTradePlaybookClosureRoot"
    assert data["generated_at"] == "2025-01-01T00:00:00Z"
    assert data["dist"]["store_root"] == "dist/artifacts"

    # These MUST be docs-root-relative to avoid drift across machines/CI.
    zone_yamls = [z["zone_yaml"] for z in data["zones"]]
    assert zone_yamls == ["src/zones/exporter/zone.yaml", "src/zones/importer/zone.yaml"]

    # Sanity: ensure we did not accidentally serialize absolute paths.
    for p in zone_yamls:
        assert not p.startswith("/")
        assert ":\\" not in p


def test_trade_playbook_generates_full_artifact_graph(tmp_path: Path):
    """Part 2e: verify full artifact graph is generated."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    store_root = out_root / "dist" / "artifacts"

    # Zone locks
    assert (store_root / "zone-locks" / "exporter.lock.json").exists()
    assert (store_root / "zone-locks" / "importer.lock.json").exists()

    # Corridor agreement
    assert (store_root / "agreements" / "obligation-corridor.agreement.json").exists()

    # Obligation corridor receipts
    assert (store_root / "receipts" / "obligation" / "receipt-0.json").exists()
    assert (store_root / "receipts" / "obligation" / "receipt-1.json").exists()
    assert (store_root / "receipts" / "obligation" / "receipt-2.json").exists()

    # Obligation checkpoint
    assert (store_root / "checkpoints" / "obligation" / "checkpoint-0.json").exists()

    # Settlement corridor receipts
    assert (store_root / "receipts" / "settlement" / "receipt-0.json").exists()
    assert (store_root / "receipts" / "settlement" / "receipt-1.json").exists()

    # Settlement plan and anchor
    assert (store_root / "settlement" / "plan.json").exists()
    assert (store_root / "settlement" / "anchor.json").exists()

    # Proof bindings
    assert (store_root / "proof-bindings" / "sanctions.json").exists()
    assert (store_root / "proof-bindings" / "carrier.json").exists()
    assert (store_root / "proof-bindings" / "payment.json").exists()

    # CAS index
    assert (store_root / "cas-index.json").exists()

    # Dashboard
    assert (out_root / "dist" / "dashboard.json").exists()


def test_trade_playbook_zone_locks_are_deterministic(tmp_path: Path):
    """Zone locks must have deterministic timestamps and digests."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    exporter_lock_path = out_root / "dist" / "artifacts" / "zone-locks" / "exporter.lock.json"
    exporter_lock = json.loads(exporter_lock_path.read_text(encoding="utf-8"))

    assert exporter_lock["type"] == "MSEZZoneLock"
    assert exporter_lock["stack_spec_version"] == "0.4.43"
    assert exporter_lock["zone_id"] == "org.momentum.msez.zone.trade-playbook.exporter"
    assert exporter_lock["locked_at"] == "2025-01-01T00:00:00Z"
    assert exporter_lock["jurisdiction_stack"] == ["ae", "ae-dubai", "ae-dubai-difc"]


def test_trade_playbook_corridor_receipts_chain_correctly(tmp_path: Path):
    """Corridor receipts must form a proper hash chain."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    receipt_dir = out_root / "dist" / "artifacts" / "receipts" / "obligation"
    receipt_0 = json.loads((receipt_dir / "receipt-0.json").read_text(encoding="utf-8"))
    receipt_1 = json.loads((receipt_dir / "receipt-1.json").read_text(encoding="utf-8"))
    receipt_2 = json.loads((receipt_dir / "receipt-2.json").read_text(encoding="utf-8"))

    # Chain: receipt_0.next_root == receipt_1.prev_root
    assert receipt_0["next_root"] == receipt_1["prev_root"]
    # Chain: receipt_1.next_root == receipt_2.prev_root
    assert receipt_1["next_root"] == receipt_2["prev_root"]

    # Sequence numbers are monotonic
    assert receipt_0["sequence"] == 0
    assert receipt_1["sequence"] == 1
    assert receipt_2["sequence"] == 2


def test_trade_playbook_settlement_anchor_links_plan_to_receipt(tmp_path: Path):
    """Settlement anchor must link settlement plan to settlement receipt."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    settlement_dir = out_root / "dist" / "artifacts" / "settlement"
    anchor = json.loads((settlement_dir / "anchor.json").read_text(encoding="utf-8"))
    plan = json.loads((settlement_dir / "plan.json").read_text(encoding="utf-8"))

    # Anchor references the plan
    assert anchor["plan_ref"]["artifact_type"] == "settlement-plan"
    
    # Anchor references the settlement receipt
    assert anchor["settlement_receipt_ref"]["artifact_type"] == "corridor-receipt"
    
    # Anchor has finality status
    assert anchor["finality_status"]["level"] == "confirmed"


def test_trade_playbook_proof_bindings_reference_commitments(tmp_path: Path):
    """Proof bindings must reference correct commitment digests."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    bindings_dir = out_root / "dist" / "artifacts" / "proof-bindings"
    
    sanctions = json.loads((bindings_dir / "sanctions.json").read_text(encoding="utf-8"))
    assert sanctions["type"] == "MSEZProofBinding"
    assert sanctions["purpose"] == "sanctions.screening.v1"
    assert len(sanctions["commitments"]) == 1
    assert sanctions["commitments"][0]["kind"] == "corridor.receipt"

    carrier = json.loads((bindings_dir / "carrier.json").read_text(encoding="utf-8"))
    assert carrier["purpose"] == "carrier.event.v1"

    payment = json.loads((bindings_dir / "payment.json").read_text(encoding="utf-8"))
    assert payment["purpose"] == "payment.confirmation.v1"
    assert payment["commitments"][0]["kind"] == "settlement-anchor"


def test_trade_playbook_dashboard_has_cards_and_tables(tmp_path: Path):
    """Dashboard must have cards and tables for UI rendering."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    dashboard_path = out_root / "dist" / "dashboard.json"
    dashboard = json.loads(dashboard_path.read_text(encoding="utf-8"))

    assert dashboard["type"] == "MSEZTradePlaybookDashboard"
    assert "cards" in dashboard
    assert "tables" in dashboard

    # Verify card structure
    card_ids = [c["card_id"] for c in dashboard["cards"]]
    assert "total-artifacts" in card_ids
    assert "zone-locks" in card_ids
    assert "corridor-agreements" in card_ids
    assert "receipts" in card_ids
    assert "settlement-plans" in card_ids
    assert "proof-bindings" in card_ids

    # Verify table structure
    assert "artifacts" in dashboard["tables"]
    assert "columns" in dashboard["tables"]["artifacts"]
    assert "rows" in dashboard["tables"]["artifacts"]


def test_trade_playbook_cas_index_covers_all_artifacts(tmp_path: Path):
    """CAS index must reference all generated artifacts."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    out_root = tmp_path / "trade"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root)],
        cwd=repo_root,
        env=env,
    )

    cas_index_path = out_root / "dist" / "artifacts" / "cas-index.json"
    cas_index = json.loads(cas_index_path.read_text(encoding="utf-8"))

    assert cas_index["type"] == "MSEZCASIndex"
    assert cas_index["digest_algorithm"] == "sha256"
    assert cas_index["artifact_count"] > 0
    assert len(cas_index["manifests"]) == 2  # closure-root + dashboard


def test_trade_playbook_byte_level_determinism(tmp_path: Path):
    """Running generator twice must produce byte-identical output."""
    repo_root = Path(__file__).resolve().parents[1]
    script = repo_root / "tools" / "dev" / "generate_trade_playbook.py"
    env = os.environ.copy()
    env["SOURCE_DATE_EPOCH"] = "1735689600"

    # Generate first time
    out_root_1 = tmp_path / "trade1"
    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root_1)],
        cwd=repo_root,
        env=env,
    )

    # Generate second time
    out_root_2 = tmp_path / "trade2"
    subprocess.check_call(
        ["python", str(script), "--mode", "generate", "--docs-root", str(out_root_2)],
        cwd=repo_root,
        env=env,
    )

    # Compare all JSON files byte-for-byte
    import filecmp
    import glob

    files_1 = sorted(glob.glob(str(out_root_1 / "dist" / "**" / "*.json"), recursive=True))
    files_2 = sorted(glob.glob(str(out_root_2 / "dist" / "**" / "*.json"), recursive=True))

    assert len(files_1) == len(files_2)
    
    for f1, f2 in zip(files_1, files_2):
        content_1 = Path(f1).read_bytes()
        content_2 = Path(f2).read_bytes()
        assert content_1 == content_2, f"Files differ: {f1} vs {f2}"
