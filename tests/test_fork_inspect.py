import json
import hashlib
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization

from tools.lawpack import jcs_canonicalize
from tools.msez import (
    REPO_ROOT,
    corridor_expected_ruleset_digest_set,
    corridor_state_genesis_root,
    corridor_state_next_root,
    cmd_corridor_state_fork_inspect,
)
from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key


def _mk_receipt(
    *,
    corridor_id: str,
    sequence: int,
    prev_root: str,
    ruleset_digest_set: list[str],
    payload: dict,
    signer_priv: Ed25519PrivateKey,
    verification_method: str,
) -> dict:
    payload_sha256 = hashlib.sha256(jcs_canonicalize(payload)).hexdigest()
    r = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": sequence,
        "timestamp": "2025-01-01T00:00:00Z",
        "prev_root": prev_root,
        "lawpack_digest_set": [],
        "ruleset_digest_set": ruleset_digest_set,
        "transition": {
            "type": "MSEZTransitionEnvelope",
            "kind": "generic",
            "payload": payload,
            "payload_sha256": payload_sha256,
        },
    }
    r["next_root"] = corridor_state_next_root(r)
    add_ed25519_proof(r, signer_priv, verification_method)
    return r


def test_fork_inspect_detects_unresolved_fork(tmp_path: Path, capsys):
    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()

    r_a = _mk_receipt(
        corridor_id=corridor_id,
        sequence=0,
        prev_root=genesis,
        ruleset_digest_set=ruleset_set,
        payload={"x": 1},
        signer_priv=priv,
        verification_method=vm,
    )
    r_b = _mk_receipt(
        corridor_id=corridor_id,
        sequence=0,
        prev_root=genesis,
        ruleset_digest_set=ruleset_set,
        payload={"x": 2},
        signer_priv=priv,
        verification_method=vm,
    )

    (receipts_dir / "a.json").write_text(json.dumps(r_a), encoding="utf-8")
    (receipts_dir / "b.json").write_text(json.dumps(r_b), encoding="utf-8")

    class Args:
        path = str(module_dir)
        receipts = str(receipts_dir)
        fork_resolutions = ""
        from_checkpoint = ""
        enforce_trust_anchors = False
        enforce_transition_types = False
        require_artifacts = False
        no_verify_proofs = False
        format = "json"
        out = ""

    rc = cmd_corridor_state_fork_inspect(Args())
    assert rc == 0

    out = capsys.readouterr().out.strip()
    report = json.loads(out)
    assert report["corridor_id"] == corridor_id
    assert report["forks"]["total"] == 1
    assert report["forks"]["unresolved"] == 1
    assert len(report["forks"]["points"][0]["candidates"]) == 2


def test_fork_inspect_marks_resolved_with_fork_resolution(tmp_path: Path, capsys):
    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()

    r_a = _mk_receipt(
        corridor_id=corridor_id,
        sequence=0,
        prev_root=genesis,
        ruleset_digest_set=ruleset_set,
        payload={"x": 1},
        signer_priv=priv,
        verification_method=vm,
    )
    r_b = _mk_receipt(
        corridor_id=corridor_id,
        sequence=0,
        prev_root=genesis,
        ruleset_digest_set=ruleset_set,
        payload={"x": 2},
        signer_priv=priv,
        verification_method=vm,
    )

    (receipts_dir / "a.json").write_text(json.dumps(r_a), encoding="utf-8")
    (receipts_dir / "b.json").write_text(json.dumps(r_b), encoding="utf-8")

    # Minimal fork-resolution payload (not a VC) is accepted by loader.
    fork_dir = tmp_path / "forks"
    fork_dir.mkdir()
    fr = {
        "type": "MSEZCorridorForkResolution",
        "corridor_id": corridor_id,
        "resolved_at": "2025-01-01T00:00:00Z",
        "sequence": 0,
        "prev_root": genesis,
        "chosen_next_root": r_a["next_root"],
        "candidate_next_roots": [r_a["next_root"], r_b["next_root"]],
        "notes": "test",
    }
    (fork_dir / "fr.json").write_text(json.dumps(fr), encoding="utf-8")

    class Args:
        path = str(module_dir)
        receipts = str(receipts_dir)
        fork_resolutions = str(fork_dir)
        from_checkpoint = ""
        enforce_trust_anchors = False
        enforce_transition_types = False
        require_artifacts = False
        no_verify_proofs = False
        format = "json"
        out = ""

    rc = cmd_corridor_state_fork_inspect(Args())
    assert rc == 0

    report = json.loads(capsys.readouterr().out.strip())
    assert report["forks"]["total"] == 1
    assert report["forks"]["resolved"] == 1
    assert report["forks"]["unresolved"] == 0
    fp = report["forks"]["points"][0]
    assert fp["chosen_next_root"] == r_a["next_root"]
    assert fp["resolved"] is True
    assert report["canonical_head"]["receipt_count"] == 1
    assert report["canonical_head"]["final_state_root"] == r_a["next_root"]
