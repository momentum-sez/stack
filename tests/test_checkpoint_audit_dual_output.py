import argparse
import json
import pathlib

import pytest

from tools import msez


def test_checkpoint_audit_failure_emits_stderr_and_machine_json(tmp_path: pathlib.Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    """On audit failure we want:

    - machine JSON on stdout (stable for CI/tooling)
    - human summary on stderr (developer/operator friendly)

    This matches the "dual-channel" invariant we want for production-grade CI gates.
    """

    # Determinism (not strictly required for this test, but makes it stable).
    monkeypatch.setenv("SOURCE_DATE_EPOCH", "1711929600")  # 2024-04-01T00:00:00Z

    corridor_dir = msez.REPO_ROOT / "modules" / "corridors" / "swift"
    assert corridor_dir.exists(), "expected swift corridor module to exist in repo"

    test_key = msez.REPO_ROOT / "docs" / "examples" / "keys" / "dev.ed25519.jwk"
    assert test_key.exists(), "expected dev test key to exist"

    # 1) Create a single valid receipt.
    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir(parents=True, exist_ok=True)
    receipt_path = receipts_dir / "corridor.receipt.0001.json"

    args_r = argparse.Namespace(
        path=str(corridor_dir),
        transition_type="swift.pacs008",
        transition_payload=json.dumps({"pacs008": {"msg_id": "TEST"}}),
        prev="",
        subject="did:example:alice",
        evidence="",
        out=str(receipt_path),
        require_artifacts=[],
        sign=True,
        key=str(test_key),
        verification_method="",
        purpose="assertionMethod",
        created="",
    )
    assert msez.cmd_corridor_state_receipt_init(args_r) == 0

    # 2) Create a valid checkpoint.
    checkpoint_path = tmp_path / "corridor.checkpoint.json"
    args_c = argparse.Namespace(
        path=str(corridor_dir),
        receipts=str(receipts_dir),
        fork_resolutions="",
        enforce_trust_anchors=False,
        format="canonical-json",
        out=str(checkpoint_path),
        sign=True,
        key=str(test_key),
        verification_method="",
        purpose="assertionMethod",
    )
    assert msez.cmd_corridor_state_checkpoint(args_c) == 0

    # 3) Corrupt the checkpoint bytes (non-canonical formatting), while keeping JSON semantics.
    checkpoint_obj = json.loads(checkpoint_path.read_text("utf-8"))
    checkpoint_path.write_text(json.dumps(checkpoint_obj, indent=2) + "\n", encoding="utf-8")

    # Clear stdout/stderr from setup (receipt/checkpoint writers) so the audit stdout is pure JSON.
    capsys.readouterr()

    # 4) Audit: should FAIL, and emit both stderr (human) and stdout (machine JSON).
    args_a = argparse.Namespace(
        path=str(corridor_dir),
        checkpoint=str(checkpoint_path),
        no_verify_proofs=False,
    )
    rc = msez.cmd_corridor_state_checkpoint_audit(args_a)
    out = capsys.readouterr()

    assert rc != 0
    assert out.err.strip() != ""
    assert "CHECKPOINT AUDIT FAILED" in out.err

    # stdout must be valid JSON.
    report = json.loads(out.out)
    assert report["type"] == "MSEZCorridorCheckpointAuditReport"
    assert report["ok"] is False
    assert report["checkpoint_path"].endswith("corridor.checkpoint.json")
    assert report["checks"]["canonical_bytes"]["ok"] is False
    assert any("non-canonical" in e for e in report["errors"])
