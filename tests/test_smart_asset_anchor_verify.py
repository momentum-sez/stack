import argparse
import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_smart_asset_checkpoint_can_be_anchored_in_corridor_receipt_and_checkpoint(tmp_path: Path):
    """Integration: asset checkpoint digest attached to a corridor receipt proven-included by an MMR checkpoint."""

    from tools import smart_asset  # type: ignore
    from tools import msez  # type: ignore
    from tools.lawpack import jcs_canonicalize  # type: ignore
    from tools.mmr import build_inclusion_proof, mmr_root_from_next_roots  # type: ignore
    from tools.vc import add_ed25519_proof, generate_ed25519_jwk, load_ed25519_private_key_from_jwk, now_rfc3339  # type: ignore

    # --- minimal corridor module
    corridor_id = "test-corridor"
    corridor_mod = tmp_path / "corridor"
    corridor_mod.mkdir(parents=True, exist_ok=True)
    (corridor_mod / "corridor.yaml").write_text(f"corridor_id: {corridor_id}\n", encoding="utf-8")

    # --- smart asset checkpoint
    state = {"balance": 100, "owner": "did:key:alice"}
    state_root = smart_asset.state_root_from_state(state)
    asset_id = "a" * 64
    asset_ck = {
        "type": "SmartAssetCheckpoint",
        "asset_id": asset_id,
        "as_of": "2026-01-01T00:00:00Z",
        "state_root_sha256": state_root,
        "parents": [],
        "attachments": [],
    }
    asset_ck_path = tmp_path / "asset.checkpoint.json"
    _write_json(asset_ck_path, asset_ck)

    # --- corridor receipt with typed attachment to the asset checkpoint digest
    payload = {"note": "anchor asset checkpoint"}
    payload_sha = msez.sha256_bytes(jcs_canonicalize(payload))
    transition = {
        "type": "MSEZTransitionEnvelope",
        "kind": "msez.asset.checkpoint.anchor.v1",
        "timestamp": now_rfc3339(),
        "payload": payload,
        "payload_sha256": payload_sha,
        "attachments": [
            {
                "artifact_type": "smart-asset-checkpoint",
                "digest_sha256": state_root,
                "uri": str(asset_ck_path),
                "media_type": "application/json",
            }
        ],
    }

    receipt = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": 0,
        "timestamp": now_rfc3339(),
        "prev_root": "0" * 64,
        "lawpack_digest_set": [],
        "ruleset_digest_set": [],
        "transition": transition,
    }
    receipt["next_root"] = msez.corridor_state_next_root(receipt)

    # Sign receipt
    jwk = generate_ed25519_jwk(kid="key-1")
    priv, did = load_ed25519_private_key_from_jwk(jwk)
    add_ed25519_proof(receipt, priv, did + "#key-1")

    receipt_path = tmp_path / "corridor.receipt.json"
    _write_json(receipt_path, receipt)

    # --- corridor checkpoint (MMR root over one receipt)
    mmr = mmr_root_from_next_roots([receipt["next_root"]])
    checkpoint = {
        "type": "MSEZCorridorStateCheckpoint",
        "corridor_id": corridor_id,
        "timestamp": now_rfc3339(),
        "genesis_root": "0" * 64,
        "final_state_root": receipt["next_root"],
        "receipt_count": 1,
        "lawpack_digest_set": [],
        "ruleset_digest_set": [],
        "mmr": {"size": mmr["size"], "root": mmr["root"], "peaks": mmr.get("peaks")},
    }
    add_ed25519_proof(checkpoint, priv, did + "#key-1")

    checkpoint_path = tmp_path / "corridor.checkpoint.json"
    _write_json(checkpoint_path, checkpoint)

    # --- inclusion proof for leaf 0
    base = build_inclusion_proof([receipt["next_root"]], 0)
    proof = {
        "type": "MSEZCorridorReceiptInclusionProof",
        "corridor_id": corridor_id,
        "generated_at": now_rfc3339(),
        "leaf_index": 0,
        "receipt_next_root": receipt["next_root"],
        "leaf_hash": base["leaf_hash"],
        "peak_index": base["peak_index"],
        "peak_height": base["peak_height"],
        "path": base["path"],
        "peaks": base["peaks"],
        "mmr": {"size": base["size"], "root": base["root"]},
    }
    proof_path = tmp_path / "corridor.inclusion-proof.0.json"
    _write_json(proof_path, proof)

    # --- verify anchor (this calls corridor inclusion verification + attachment check)
    args = argparse.Namespace(
        path=str(corridor_mod),
        receipt=str(receipt_path),
        proof=str(proof_path),
        checkpoint=str(checkpoint_path),
        asset_checkpoint=str(asset_ck_path),
        state_root="",
        enforce_trust_anchors=False,
    )
    rc = msez.cmd_asset_anchor_verify(args)
    assert rc == 0
