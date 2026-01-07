import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_smart_asset_module_init_and_module_aware_state_commands(tmp_path: Path):
    """End-to-end smoke test for the v0.4.32 Smart Asset module directory UX.

    Covers:
      - msez asset module init (scaffold)
      - asset state subcommands using <path> module dir defaults
        (receipt-init, verify, checkpoint, inclusion-proof, verify-inclusion)
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
        stack_spec_version=msez.STACK_SPEC_VERSION,
        asset_name="Test Asset",
        asset_class="contract",
        description="Module-dir receipt-chain test",
        created_at="2026-01-01T00:00:00Z",
        creator=did,
    )
    asset_id = genesis["asset_id"]

    # --- scaffold asset module
    out_dir = tmp_path / "smart-assets"
    ns_init = msez.argparse.Namespace(
        asset_id=asset_id,
        out_dir=str(out_dir),
        template=str(msez.REPO_ROOT / "modules/smart-assets/_template"),
        purpose=["core"],
        transition_types_lock="",
        expected_transition_type_registry_digest="",
        force=False,
        json=True,
    )
    assert msez.cmd_asset_module_init(ns_init) == 0

    module_dir = out_dir / asset_id
    assert (module_dir / "asset.yaml").exists()
    asset_yaml_txt = (module_dir / "asset.yaml").read_text(encoding="utf-8")
    assert asset_id in asset_yaml_txt
    assert "{{" not in asset_yaml_txt  # templates should be rendered

    # --- receipt 0 (default out path inside module)
    ns0 = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        purpose="",
        sequence=0,
        prev_root="genesis",
        timestamp="2026-01-01T00:00:00Z",
        transition="",
        lawpack_digest=[],
        ruleset_digest=[],
        transition_types_lock="",
        fill_transition_digests=False,
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
        out="",
    )
    assert msez.cmd_asset_state_receipt_init(ns0) == 0

    r0_path = module_dir / "state/receipts/smart-asset.receipt.0.json"
    assert r0_path.exists()
    r0 = json.loads(r0_path.read_text(encoding="utf-8"))
    assert r0["asset_id"] == asset_id
    assert r0["sequence"] == 0

    # --- receipt 1
    ns1 = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        purpose="",
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
        out="",
    )
    assert msez.cmd_asset_state_receipt_init(ns1) == 0

    r1_path = module_dir / "state/receipts/smart-asset.receipt.1.json"
    assert r1_path.exists()
    r1 = json.loads(r1_path.read_text(encoding="utf-8"))
    assert r1["sequence"] == 1

    # --- verify chain (derive receipts dir from module)
    nsv = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        receipts="",
        purpose="",
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

    # --- checkpoint (default out path inside module)
    nscp = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        receipts="",
        purpose="",
        genesis_root="",
        enforce_trust_anchors=False,
        trust_anchors="",
        out="",
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
    )
    assert msez.cmd_asset_state_checkpoint(nscp) == 0

    cp_path = module_dir / "state/checkpoints/smart-asset.receipt-chain.checkpoint.json"
    assert cp_path.exists()

    # --- inclusion proof (seq=1; default out path inside module)
    nsp = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        receipts="",
        purpose="",
        genesis_root="",
        enforce_trust_anchors=False,
        trust_anchors="",
        sequence=1,
        checkpoint=str(cp_path),
        out="",
    )
    assert msez.cmd_asset_state_inclusion_proof(nsp) == 0

    proof_path = module_dir / "state/proofs/smart-asset.receipt.1.inclusion-proof.json"
    assert proof_path.exists()

    # --- verify inclusion (derive asset_id from module)
    nsvp = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        receipt=str(r1_path),
        proof=str(proof_path),
        checkpoint=str(cp_path),
        enforce_trust_anchors=False,
        trust_anchors="",
    )
    assert msez.cmd_asset_state_verify_inclusion(nsvp) == 0
