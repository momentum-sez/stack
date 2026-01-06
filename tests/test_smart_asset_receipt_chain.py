import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_smart_asset_receipt_chain_checkpoint_and_inclusion(tmp_path: Path):
    """End-to-end smoke test for the asset-local receipt chain.

    Covers:
      - receipt-init (signed)
      - state verify (chain integrity)
      - checkpoint (signed; MMR root)
      - inclusion-proof + verify-inclusion
    """

    from tools import smart_asset  # type: ignore
    from tools import msez  # type: ignore
    from tools.vc import generate_ed25519_jwk, load_ed25519_private_key_from_jwk  # type: ignore

    # --- signer key
    jwk = generate_ed25519_jwk()
    key_path = tmp_path / "k.jwk.json"
    _write_json(key_path, jwk)
    _priv, did = load_ed25519_private_key_from_jwk(jwk)

    # --- asset id (from genesis)
    genesis = smart_asset.build_genesis(
        stack_spec_version="0.4.31",
        asset_name="Test Asset",
        asset_class="contract",
        description="Receipt-chain test",
        created_at="2026-01-01T00:00:00Z",
        creator=did,
    )
    asset_id = genesis["asset_id"]

    receipts_dir = tmp_path / "receipts"
    receipts_dir.mkdir(parents=True, exist_ok=True)

    # --- receipt 0
    r0_path = receipts_dir / "r0.json"
    ns0 = msez.argparse.Namespace(
        asset_id=asset_id,
        sequence=0,
        prev_root="genesis",
        timestamp="2026-01-01T00:00:00Z",
        transition="",  # default noop transition
        lawpack_digest=[],
        ruleset_digest=[],
        transition_types_lock="",
        fill_transition_digests=False,
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
        purpose="core",
        out=str(r0_path),
    )
    assert msez.cmd_asset_state_receipt_init(ns0) == 0
    r0 = json.loads(r0_path.read_text(encoding="utf-8"))
    assert r0["asset_id"] == asset_id
    assert r0["sequence"] == 0
    assert "next_root" in r0

    # --- receipt 1
    r1_path = receipts_dir / "r1.json"
    ns1 = msez.argparse.Namespace(
        asset_id=asset_id,
        sequence=1,
        prev_root=r0["next_root"],
        timestamp="2026-01-01T00:00:01Z",
        transition="",
        lawpack_digest=[],
        ruleset_digest=[],
        transition_types_lock="",
        fill_transition_digests=False,
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
        purpose="core",
        out=str(r1_path),
    )
    assert msez.cmd_asset_state_receipt_init(ns1) == 0
    r1 = json.loads(r1_path.read_text(encoding="utf-8"))
    assert r1["sequence"] == 1

    # --- receipt 2
    r2_path = receipts_dir / "r2.json"
    ns2 = msez.argparse.Namespace(
        asset_id=asset_id,
        sequence=2,
        prev_root=r1["next_root"],
        timestamp="2026-01-01T00:00:02Z",
        transition="",
        lawpack_digest=[],
        ruleset_digest=[],
        transition_types_lock="",
        fill_transition_digests=False,
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
        purpose="core",
        out=str(r2_path),
    )
    assert msez.cmd_asset_state_receipt_init(ns2) == 0
    r2 = json.loads(r2_path.read_text(encoding="utf-8"))
    assert r2["sequence"] == 2

    # --- verify chain
    nsv = msez.argparse.Namespace(
        asset_id=asset_id,
        receipts=str(receipts_dir),
        purpose="core",
        genesis_root="",
        enforce_trust_anchors=False,
        trust_anchors="",
        enforce_transition_types=False,
        expected_transition_type_registry_digest="",
        require_artifacts=False,
        transitive_require_artifacts=False,
        checkpoint="",
        json=True,
    )
    assert msez.cmd_asset_state_verify(nsv) == 0

    # --- checkpoint
    cp_path = tmp_path / "cp.json"
    nscp = msez.argparse.Namespace(
        asset_id=asset_id,
        receipts=str(receipts_dir),
        purpose="core",
        genesis_root="",
        enforce_trust_anchors=False,
        trust_anchors="",
        out=str(cp_path),
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
    )
    assert msez.cmd_asset_state_checkpoint(nscp) == 0
    cp = json.loads(cp_path.read_text(encoding="utf-8"))
    assert cp["type"] == "SmartAssetReceiptChainCheckpoint"
    assert cp["asset_id"] == asset_id
    assert cp["receipt_count"] == 3

    # --- inclusion proof (seq=1)
    proof_path = tmp_path / "p.json"
    nsp = msez.argparse.Namespace(
        asset_id=asset_id,
        receipts=str(receipts_dir),
        purpose="core",
        genesis_root="",
        enforce_trust_anchors=False,
        trust_anchors="",
        sequence=1,
        checkpoint=str(cp_path),
        out=str(proof_path),
    )
    assert msez.cmd_asset_state_inclusion_proof(nsp) == 0

    # --- verify inclusion
    nsvp = msez.argparse.Namespace(
        asset_id=asset_id,
        receipt=str(r1_path),
        proof=str(proof_path),
        checkpoint=str(cp_path),
        enforce_trust_anchors=False,
        trust_anchors="",
    )
    assert msez.cmd_asset_state_verify_inclusion(nsvp) == 0
