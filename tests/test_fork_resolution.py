import json
from datetime import datetime, timezone
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def _now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


def _sign(obj: dict) -> None:
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key
    from cryptography.hazmat.primitives import serialization

    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"
    add_ed25519_proof(obj, priv, vm)


def _mk_receipt(corridor_id: str, seq: int, prev_root: str, ruleset_set: list[str], payload: dict) -> dict:
    from tools.msez import corridor_state_next_root
    from tools.lawpack import jcs_canonicalize
    import hashlib

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
    _sign(r)
    return r


def test_fork_detected_without_resolution(tmp_path: Path):
    from tools.msez import REPO_ROOT, corridor_state_genesis_root, corridor_expected_ruleset_digest_set, _corridor_state_build_chain

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # Two conflicting receipts at the same (seq=0, prev_root=genesis)
    r1 = _mk_receipt(
        "org.momentum.msez.corridor.swift.iso20022-cross-border",
        0,
        genesis,
        ruleset_set,
        {"x": 1},
    )
    r2 = _mk_receipt(
        "org.momentum.msez.corridor.swift.iso20022-cross-border",
        0,
        genesis,
        ruleset_set,
        {"x": 2},
    )
    assert r1["next_root"] != r2["next_root"]

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()
    (receipts_dir / "r1.json").write_text(json.dumps(r1, indent=2))
    (receipts_dir / "r2.json").write_text(json.dumps(r2, indent=2))

    result, warnings, errors = _corridor_state_build_chain(
        module_dir.relative_to(REPO_ROOT),
        receipts_dir,
        enforce_trust_anchors=False,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=None,
    )
    assert errors
    assert any("fork detected" in e for e in errors)


def test_fork_resolved_with_resolution_artifact(tmp_path: Path):
    from tools.msez import REPO_ROOT, corridor_state_genesis_root, corridor_expected_ruleset_digest_set, _corridor_state_build_chain

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    r1 = _mk_receipt(
        "org.momentum.msez.corridor.swift.iso20022-cross-border",
        0,
        genesis,
        ruleset_set,
        {"x": 1},
    )
    r2 = _mk_receipt(
        "org.momentum.msez.corridor.swift.iso20022-cross-border",
        0,
        genesis,
        ruleset_set,
        {"x": 2},
    )

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir()
    (receipts_dir / "r1.json").write_text(json.dumps(r1, indent=2))
    (receipts_dir / "r2.json").write_text(json.dumps(r2, indent=2))

    # Fork resolution VC selects r1.next_root
    fork_res_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorForkResolutionCredential"],
        "issuer": "did:key:z6Mk-authority",
        "issuanceDate": _now(),
        "credentialSubject": {
            "corridor_id": "org.momentum.msez.corridor.swift.iso20022-cross-border",
            "sequence": 0,
            "prev_root": genesis,
            "candidate_next_roots": [r1["next_root"], r2["next_root"]],
            "chosen_next_root": r1["next_root"],
            "reason": "canonical selection",
            "decided_at": _now(),
        },
    }
    _sign(fork_res_vc)

    fork_dir = tmp_path / "fork"
    fork_dir.mkdir()
    (fork_dir / "fork-resolution.json").write_text(json.dumps(fork_res_vc, indent=2))

    result, warnings, errors = _corridor_state_build_chain(
        module_dir.relative_to(REPO_ROOT),
        receipts_dir,
        enforce_trust_anchors=False,
        enforce_transition_types=False,
        require_artifacts=False,
        fork_resolutions_path=fork_dir,
    )
    assert not errors
    assert result.get("receipt_count") == 1
    assert result.get("final_state_root") == r1["next_root"]
    assert any("fork resolved" in w for w in warnings)
