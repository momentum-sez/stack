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


def _mk_receipt(*, corridor_id: str, seq: int, prev_root: str, ruleset_set: list[str], payload: dict, priv, vm: str) -> dict:
    payload_sha256 = hashlib.sha256(jcs_canonicalize(payload)).hexdigest()
    r = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": seq,
        "timestamp": "2025-01-01T00:00:00Z",
        "prev_root": prev_root,
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
    return r


def test_enforce_trust_anchors_accepts_authorized_signer(tmp_path: Path):
    src = REPO_ROOT / "modules" / "corridors" / "swift"
    module_dir = tmp_path / "swift"
    shutil.copytree(src, module_dir)

    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # signer DID
    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    # Rewrite trust-anchors.yaml to authorize our signer for corridor_receipt.
    ta_yaml = f"""trust_anchors:\n  - anchor_id: test\n    type: did\n    identifier: {did}\n    allowed_attestations:\n      - corridor_receipt\n"""
    (module_dir / "trust-anchors.yaml").write_text(ta_yaml, encoding="utf-8")

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()
    r = _mk_receipt(
        corridor_id=corridor_id,
        seq=0,
        prev_root=genesis,
        ruleset_set=ruleset_set,
        payload={"ok": True},
        priv=priv,
        vm=vm,
    )
    (receipts_dir / "r0.json").write_text(json.dumps(r), encoding="utf-8")

    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_dir,
        enforce_trust_anchors=True,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
        from_checkpoint_path=None,
    )
    assert errors == []
    assert result["receipt_count"] == 1


def test_enforce_trust_anchors_rejects_unauthorized_signer(tmp_path: Path):
    src = REPO_ROOT / "modules" / "corridors" / "swift"
    module_dir = tmp_path / "swift"
    shutil.copytree(src, module_dir)

    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # trust anchor DID (authorized)
    priv_auth = Ed25519PrivateKey.generate()
    pub_auth = priv_auth.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did_auth = did_key_from_ed25519_public_key(pub_auth)

    ta_yaml = f"""trust_anchors:\n  - anchor_id: test\n    type: did\n    identifier: {did_auth}\n    allowed_attestations:\n      - corridor_receipt\n"""
    (module_dir / "trust-anchors.yaml").write_text(ta_yaml, encoding="utf-8")

    # receipt signed by a different DID (unauthorized)
    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()
    r = _mk_receipt(
        corridor_id=corridor_id,
        seq=0,
        prev_root=genesis,
        ruleset_set=ruleset_set,
        payload={"ok": False},
        priv=priv,
        vm=vm,
    )
    (receipts_dir / "r0.json").write_text(json.dumps(r), encoding="utf-8")

    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_dir,
        enforce_trust_anchors=True,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
        from_checkpoint_path=None,
    )

    assert any("trust anchors" in e for e in errors)
