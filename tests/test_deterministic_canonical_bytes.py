import argparse
import json
import pathlib

import yaml

from tools.msez import (
    REPO_ROOT,
    canonical_json_bytes,
    cmd_corridor_state_checkpoint,
    cmd_corridor_state_receipt_init,
    cmd_corridor_state_verify,
    cmd_lock,
)


def _bytes_eq_canonical_file(raw: bytes, canonical: bytes) -> bool:
    """Test helper: accept canonical bytes with an optional trailing LF."""
    return raw == canonical or raw == canonical + b"\n"


def test_zone_lock_check_mode_enforces_canonical_bytes(tmp_path: pathlib.Path) -> None:
    zone_path = REPO_ROOT / "jurisdictions" / "_starter" / "zone.yaml"
    out_path = tmp_path / "stack.lock"

    # Generate a lockfile (now defaults to canonical-json output).
    rc = cmd_lock(
        argparse.Namespace(
            zone=str(zone_path),
            out=str(out_path),
            emit_artifactrefs=True,
        )
    )
    assert rc == 0
    assert out_path.exists()

    lock_obj = yaml.safe_load(out_path.read_text(encoding="utf-8"))
    assert isinstance(lock_obj, dict)

    raw = out_path.read_bytes()
    canon = canonical_json_bytes(lock_obj)
    assert _bytes_eq_canonical_file(raw, canon)

    # Check mode should pass without writing.
    rc = cmd_lock(
        argparse.Namespace(
            zone=str(zone_path),
            out=str(out_path),
            check=True,
            strict=True,
            emit_artifactrefs=True,
        )
    )
    assert rc == 0

    # Non-canonical formatting (same data) should fail check mode.
    out_path.write_text(json.dumps(lock_obj, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    rc = cmd_lock(
        argparse.Namespace(
            zone=str(zone_path),
            out=str(out_path),
            check=True,
            strict=True,
        )
    )
    assert rc != 0


def test_corridor_state_verify_can_enforce_canonical_receipts_and_checkpoint_bytes(tmp_path: pathlib.Path) -> None:
    # Use a real corridor module, but write all receipts/checkpoints to a temp directory.
    corridor_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    test_key = REPO_ROOT / "docs" / "examples" / "keys" / "dev.ed25519.jwk"
    assert test_key.exists()
    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir(parents=True, exist_ok=True)

    r0_path = receipts_dir / "corridor-receipt.0.json"
    rc = cmd_corridor_state_receipt_init(
        argparse.Namespace(
            path=str(corridor_dir),
            sequence=0,
            prev_root="genesis",
            out=str(r0_path),
            sign=True,
            key=str(test_key),
        )
    )
    assert rc == 0
    assert r0_path.exists()

    # Verify should succeed when receipts are canonical.
    rc = cmd_corridor_state_verify(
        argparse.Namespace(
            path=str(corridor_dir),
            receipts=str(receipts_dir),
            check_canonical_bytes=True,
        )
    )
    assert rc == 0

    # Rewrite receipt with pretty formatting but identical JSON content; chain verification still
    # passes, but canonical-bytes enforcement should fail.
    r0_obj = json.loads(r0_path.read_text(encoding="utf-8"))
    r0_path.write_text(json.dumps(r0_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    rc = cmd_corridor_state_verify(
        argparse.Namespace(
            path=str(corridor_dir),
            receipts=str(receipts_dir),
            check_canonical_bytes=True,
        )
    )
    assert rc != 0

    # Restore canonical receipt and generate a checkpoint.
    r0_path.write_bytes(canonical_json_bytes(r0_obj) + b"\n")
    checkpoint_path = tmp_path / "corridor.checkpoint.json"
    rc = cmd_corridor_state_checkpoint(
        argparse.Namespace(
            path=str(corridor_dir),
            receipts=str(receipts_dir),
            out=str(checkpoint_path),
            sign=True,
            key=str(test_key),
        )
    )
    assert rc == 0
    assert checkpoint_path.exists()

    rc = cmd_corridor_state_verify(
        argparse.Namespace(
            path=str(corridor_dir),
            receipts=str(receipts_dir),
            checkpoint=str(checkpoint_path),
            check_canonical_bytes=True,
        )
    )
    assert rc == 0

    # Reformat checkpoint with indentation (content-preserving) and ensure canonical-bytes enforcement fails.
    ck_obj = json.loads(checkpoint_path.read_text(encoding="utf-8"))
    checkpoint_path.write_text(json.dumps(ck_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    rc = cmd_corridor_state_verify(
        argparse.Namespace(
            path=str(corridor_dir),
            receipts=str(receipts_dir),
            checkpoint=str(checkpoint_path),
            check_canonical_bytes=True,
        )
    )
    assert rc != 0
