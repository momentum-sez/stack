import json
import pathlib
import hashlib

from tools import artifacts as artifact_cas
from tools import msez


def _sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_artifact_graph_verify_from_bundle_infers_root(tmp_path: pathlib.Path) -> None:
    """--from-bundle should allow fully-offline verification using artifacts in the witness bundle.

    This is the primary intended UX:
      1) create a witness bundle for some root commitment
      2) ship the zip
      3) verifier runs: `msez artifact graph verify --from-bundle witness.zip --strict`

    The verifier should infer the root from manifest.json when no explicit root is provided.
    """

    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    # Store a blob.
    blob_data = b"bundle-offline"
    blob_digest = _sha256_bytes(blob_data)
    blob_path = tmp_path / "blob.bin"
    blob_path.write_bytes(blob_data)

    artifact_cas.store_artifact_file(
        "blob",
        blob_digest,
        blob_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{blob_digest}.blob.bin",
        overwrite=True,
    )

    # Root structured artifact contains a typed ArtifactRef to the blob.
    root_obj = {"attachments": [{"artifact_type": "blob", "digest_sha256": blob_digest}]}
    root_path = tmp_path / "root.schema.json"
    root_path.write_text(json.dumps(root_obj, indent=2) + "\n", encoding="utf-8")
    root_digest = msez._jcs_sha256_of_json_file(root_path)

    artifact_cas.store_artifact_file(
        "schema",
        root_digest,
        root_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{root_digest}.schema.json",
        overwrite=True,
    )

    # Create witness bundle.
    bundle_path = tmp_path / "witness.zip"
    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "schema",
                "digest": root_digest,
                "path": "",
                "from_bundle": "",
                "store_root": [str(store_root)],
                "strict": True,
                "emit_edges": False,
                "bundle": str(bundle_path),
                "bundle_max_bytes": 0,
                "max_nodes": 1000,
                "max_depth": 8,
                "out": str(report_path),
                "json": True,
            },
        )()
    )
    assert rc == 0
    assert bundle_path.exists()

    # Verify purely from the bundle (no store roots, no root args).
    report2_path = tmp_path / "report2.json"
    rc2 = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "",
                "digest": "",
                "path": "",
                "from_bundle": str(bundle_path),
                "store_root": [],
                "strict": True,
                "emit_edges": False,
                "bundle": "",
                "bundle_max_bytes": 0,
                "max_nodes": 1000,
                "max_depth": 8,
                "out": str(report2_path),
                "json": True,
            },
        )()
    )

    assert rc2 == 0
    rep2 = json.loads(report2_path.read_text(encoding="utf-8"))
    assert rep2.get("input_bundle", {}).get("root_inferred_from_manifest") is True
    assert rep2["stats"]["missing_total"] == 0
    assert rep2["stats"]["digest_mismatch_total"] == 0
