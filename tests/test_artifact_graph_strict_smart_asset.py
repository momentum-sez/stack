import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_artifact_graph_strict_verifies_smart_asset_artifacts(tmp_path: Path):
    """Regression: smart-asset-genesis/checkpoint/attestation are not raw-bytes content-addressed.

    - smart-asset-genesis digest == asset_id (sha256(JCS(genesis-without-asset_id)))
    - smart-asset-checkpoint digest == state_root_sha256 (declared)
    - smart-asset-attestation digest == sha256(JCS(attestation))
    """

    from tools import smart_asset  # type: ignore
    from tools.artifacts import store_artifact_file  # type: ignore
    from tools.msez import build_artifact_graph_verify_report  # type: ignore

    store_root = tmp_path / "artifacts"

    # --- genesis
    genesis = smart_asset.build_genesis(
        stack_spec_version="0.4.31",
        asset_name="Acme Bond",
        asset_class="security",
        description="Test asset",
        created_at="2026-01-01T00:00:00Z",
    )
    asset_id = genesis["asset_id"]
    gpath = tmp_path / "genesis.json"
    _write_json(gpath, genesis)
    store_artifact_file(
        artifact_type="smart-asset-genesis",
        digest_sha256=asset_id,
        src_path=gpath,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )

    rep_g = build_artifact_graph_verify_report(
        root_artifact_type="smart-asset-genesis",
        root_digest_sha256=asset_id,
        root_path=None,
        store_roots=[store_root],
        strict=True,
        max_nodes=256,
        max_depth=8,
        emit_edges=False,
    )
    assert rep_g["missing"] == []
    assert rep_g["digest_mismatches"] == []

    # --- checkpoint
    state = {"balance": 100, "owner": "did:key:alice"}
    state_root = smart_asset.state_root_from_state(state)
    ck = {
        "type": "SmartAssetCheckpoint",
        "asset_id": asset_id,
        "as_of": "2026-01-01T00:00:00Z",
        "state_root_sha256": state_root,
        "parents": [],
        "attachments": [],
    }
    ckpath = tmp_path / "checkpoint.json"
    _write_json(ckpath, ck)
    store_artifact_file(
        artifact_type="smart-asset-checkpoint",
        digest_sha256=state_root,
        src_path=ckpath,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )

    rep_ck = build_artifact_graph_verify_report(
        root_artifact_type="smart-asset-checkpoint",
        root_digest_sha256=state_root,
        root_path=None,
        store_roots=[store_root],
        strict=True,
        max_nodes=256,
        max_depth=8,
        emit_edges=False,
    )
    assert rep_ck["missing"] == []
    assert rep_ck["digest_mismatches"] == []

    # --- attestation
    att = {
        "type": "SmartAssetAttestation",
        "asset_id": asset_id,
        "issued_at": "2026-01-01T00:00:00Z",
        "issuer": "did:key:issuer",
        "kind": "kyc.passed.v1",
        "claims": {"tier": "standard"},
    }
    dg_att = smart_asset.sha256_hex(smart_asset.canonicalize_json(att))
    apath = tmp_path / "att.json"
    _write_json(apath, att)
    store_artifact_file(
        artifact_type="smart-asset-attestation",
        digest_sha256=dg_att,
        src_path=apath,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )

    rep_att = build_artifact_graph_verify_report(
        root_artifact_type="smart-asset-attestation",
        root_digest_sha256=dg_att,
        root_path=None,
        store_roots=[store_root],
        strict=True,
        max_nodes=256,
        max_depth=8,
        emit_edges=False,
    )
    assert rep_att["missing"] == []
    assert rep_att["digest_mismatches"] == []

    # --- receipt (digest == receipt.next_root, not raw bytes)
    from tools.msez import asset_state_genesis_root, asset_state_next_root  # type: ignore

    rcpt = {
        "type": "SmartAssetReceipt",
        "asset_id": asset_id,
        "sequence": 0,
        "timestamp": "2026-01-01T00:00:00Z",
        "prev_root": asset_state_genesis_root(asset_id, purpose="core"),
        "lawpack_digest_set": [],
        "ruleset_digest_set": [],
        "proof": [],
    }
    rcpt["next_root"] = asset_state_next_root(rcpt)

    rpath = tmp_path / "receipt.json"
    _write_json(rpath, rcpt)
    store_artifact_file(
        artifact_type="smart-asset-receipt",
        digest_sha256=rcpt["next_root"],
        src_path=rpath,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )

    rep_rcpt = build_artifact_graph_verify_report(
        root_artifact_type="smart-asset-receipt",
        root_digest_sha256=rcpt["next_root"],
        root_path=None,
        store_roots=[store_root],
        strict=True,
        max_nodes=256,
        max_depth=8,
        emit_edges=False,
    )
    assert rep_rcpt["missing"] == []
    assert rep_rcpt["digest_mismatches"] == []


