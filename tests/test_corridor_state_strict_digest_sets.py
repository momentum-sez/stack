import json
import hashlib
import shutil
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization

from tools.lawpack import jcs_canonicalize
from tools.msez import (
    REPO_ROOT,
    _corridor_state_build_chain,
    corridor_expected_ruleset_digest_set,
    corridor_state_genesis_root,
    corridor_state_next_root,
)
from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key


def _mk_receipt(
    *,
    corridor_id: str,
    seq: int,
    prev_root: str,
    lawpack_set: list[str],
    ruleset_set: list[str],
    payload: dict,
    priv,
    vm: str,
) -> dict:
    payload_sha256 = hashlib.sha256(jcs_canonicalize(payload)).hexdigest()
    r = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": seq,
        "timestamp": "2025-01-01T00:00:00Z",
        "prev_root": prev_root,
        "lawpack_digest_set": lawpack_set,
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
    return r


def _mk_signer():
    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"
    return priv, vm


def test_corridor_receipt_rejects_extra_ruleset_digests(tmp_path: Path):
    src = REPO_ROOT / "modules" / "corridors" / "swift"
    module_dir = tmp_path / "swift"
    shutil.copytree(src, module_dir)

    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # Extra digest that is syntactically valid but not part of the corridor substrate.
    ruleset_set_extra = list(ruleset_set) + ["0" * 64]

    priv, vm = _mk_signer()
    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()

    r = _mk_receipt(
        corridor_id=corridor_id,
        seq=0,
        prev_root=genesis,
        lawpack_set=[],
        ruleset_set=ruleset_set_extra,
        payload={"ok": True},
        priv=priv,
        vm=vm,
    )
    (receipts_dir / "r0.json").write_text(json.dumps(r), encoding="utf-8")

    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_dir,
        enforce_trust_anchors=False,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
        from_checkpoint_path=None,
    )

    assert result == {}
    assert any("ruleset_digest_set mismatch" in e for e in errors)


def test_corridor_receipt_rejects_extra_lawpack_digests(tmp_path: Path):
    src = REPO_ROOT / "modules" / "corridors" / "swift"
    module_dir = tmp_path / "swift"
    shutil.copytree(src, module_dir)

    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # Extra digest that is syntactically valid but not expected for this corridor module.
    lawpack_set_extra = ["f" * 64]

    priv, vm = _mk_signer()
    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()

    r = _mk_receipt(
        corridor_id=corridor_id,
        seq=0,
        prev_root=genesis,
        lawpack_set=lawpack_set_extra,
        ruleset_set=ruleset_set,
        payload={"ok": True},
        priv=priv,
        vm=vm,
    )
    (receipts_dir / "r0.json").write_text(json.dumps(r), encoding="utf-8")

    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_dir,
        enforce_trust_anchors=False,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
        from_checkpoint_path=None,
    )

    assert result == {}
    assert any("lawpack_digest_set mismatch" in e for e in errors)
