import argparse

import yaml

from tools import msez


def test_lock_emit_artifactrefs(tmp_path):
    zone_path = msez.REPO_ROOT / "jurisdictions" / "_starter" / "zone.yaml"
    out_path = tmp_path / "stack.lock"

    args = argparse.Namespace(
        zone=str(zone_path),
        out=str(out_path),
        emit_artifactrefs=True,
    )

    rc = msez.cmd_lock(args)
    assert rc == 0

    lock = yaml.safe_load(out_path.read_text(encoding="utf-8"))

    assert lock.get("lawpacks"), "expected lawpacks in starter lock"
    for lp in lock["lawpacks"]:
        ref = lp.get("lawpack_digest_sha256")
        assert isinstance(ref, dict)
        assert ref.get("artifact_type") == "lawpack"
        assert isinstance(ref.get("digest_sha256"), str)
        assert len(ref.get("digest_sha256")) == 64

    assert lock.get("corridors"), "expected corridors in starter lock"
    c0 = lock["corridors"][0]
    for k in ("corridor_manifest_sha256", "trust_anchors_sha256", "key_rotation_sha256", "corridor_definition_vc_sha256"):
        ref = c0.get(k)
        assert isinstance(ref, dict)
        assert ref.get("artifact_type") == "blob"
        assert isinstance(ref.get("digest_sha256"), str)
        assert len(ref.get("digest_sha256")) == 64
