import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_smart_asset_receipt_chain_fork_can_be_resolved_with_vc(tmp_path: Path):
    """Smoke test for v0.4.33 smart-asset fork-resolution support.

    Scenario:
      - receipt 0 (genesis -> r0.next_root)
      - two competing receipts at sequence=1 with the same prev_root (fork)
      - verify should fail without a fork-resolution artifact
      - verify should pass once a fork-resolution VC is provided
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
        asset_name="Fork Test Asset",
        asset_class="contract",
        description="Fork-resolution test",
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

    # --- receipt 0
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
    r0 = json.loads(r0_path.read_text(encoding="utf-8"))

    # --- two competing receipts at seq=1 (fork)
    r1a_path = module_dir / "state/receipts/smart-asset.receipt.1.a.json"
    ns1a = msez.argparse.Namespace(
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
        out=str(r1a_path),
    )
    assert msez.cmd_asset_state_receipt_init(ns1a) == 0
    r1a = json.loads(r1a_path.read_text(encoding="utf-8"))

    r1b_path = module_dir / "state/receipts/smart-asset.receipt.1.b.json"
    ns1b = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        purpose="",
        sequence=1,
        prev_root=r0["next_root"],
        timestamp="2026-01-01T00:00:01.500Z",
        transition="",
        lawpack_digest=[],
        ruleset_digest=[],
        transition_types_lock="",
        fill_transition_digests=False,
        sign=True,
        key=str(key_path),
        verification_method="",
        proof_purpose="assertionMethod",
        out=str(r1b_path),
    )
    assert msez.cmd_asset_state_receipt_init(ns1b) == 0
    r1b = json.loads(r1b_path.read_text(encoding="utf-8"))

    assert r1a["next_root"] != r1b["next_root"]

    # --- verify should fail (fork detected)
    nsv_fail = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        receipts="",
        fork_resolutions="",
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
    assert msez.cmd_asset_state_verify(nsv_fail) != 0

    # --- write a fork-resolution VC selecting r1a
    fr_path = module_dir / "state/fork-resolutions/fork-resolution.seq1.json"
    ns_fr = msez.argparse.Namespace(
        path=str(module_dir),
        asset_id="",
        purpose="",
        sequence=1,
        prev_root=r0["next_root"],
        chosen_next_root=r1a["next_root"],
        candidate_next_root=[r1a["next_root"], r1b["next_root"]],
        resolved_at="2026-01-01T00:00:02Z",
        issuer=did,
        id="",
        out=str(fr_path),
    )
    assert msez.cmd_asset_state_fork_resolve(ns_fr) == 0
    assert fr_path.exists()

    # --- verify should now succeed
    nsv_ok = nsv_fail
    assert msez.cmd_asset_state_verify(nsv_ok) == 0
