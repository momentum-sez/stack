import json
import pathlib
import hashlib
import zipfile

from tools import artifacts as artifact_cas
from tools import msez


def _sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_artifact_graph_witness_bundle_dir_root_includes_root_dir(tmp_path: pathlib.Path) -> None:
    # Arrange: a CAS root containing one blob
    store_root = tmp_path / "store"
    store_root.mkdir(parents=True, exist_ok=True)

    blob_path = tmp_path / "hello.txt"
    blob_path.write_text("hello", encoding="utf-8")
    dg = _sha256_bytes(blob_path.read_bytes())
    artifact_cas.store_artifact_file("blob", dg, blob_path, repo_root=msez.REPO_ROOT, store_root=store_root)

    # Arrange: a directory root containing structured files with embedded ArtifactRefs
    root_dir = tmp_path / "asset-module"
    (root_dir / "state").mkdir(parents=True, exist_ok=True)

    root_obj = {
        "type": "ExampleRoot",
        "payload": "hello",
        "artifact_ref": {"artifact_type": "blob", "digest_sha256": dg},
    }
    (root_dir / "asset.yaml").write_text("asset_id: test\n", encoding="utf-8")
    (root_dir / "state" / "receipt.json").write_text(json.dumps(root_obj, indent=2) + "\n", encoding="utf-8")

    bundle_path = tmp_path / "witness.zip"

    # Act
    rc = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "",
                "digest": "",
                "path": str(root_dir),
                "strict": True,
                "store_root": [str(store_root)],
                "out": "",
                "json": True,
                "max_nodes": 100,
                "max_depth": 8,
                "emit_edges": True,
                "bundle": str(bundle_path),
                "bundle_max_bytes": 0,
                "from_bundle": "",
            },
        )()
    )

    # Assert
    assert rc == 0
    assert bundle_path.exists()

    with zipfile.ZipFile(bundle_path, "r") as zf:
        names = set(zf.namelist())
        assert "manifest.json" in names
        assert "README.txt" in names
        assert f"root/{root_dir.name}/asset.yaml" in names
        assert f"root/{root_dir.name}/state/receipt.json" in names
        assert any(n.startswith("artifacts/blob/") for n in names)

        manifest = json.loads(zf.read("manifest.json").decode("utf-8"))
        assert manifest.get("type") == "MSEZArtifactGraphVerifyReport"
        assert manifest.get("root", {}).get("mode") == "dir"


def test_artifact_graph_from_bundle_dir_root_infers_root(tmp_path: pathlib.Path) -> None:
    # Arrange: build a witness bundle with a directory root
    store_root = tmp_path / "store"
    store_root.mkdir(parents=True, exist_ok=True)

    blob_path = tmp_path / "hello.txt"
    blob_path.write_text("hello", encoding="utf-8")
    dg = _sha256_bytes(blob_path.read_bytes())
    artifact_cas.store_artifact_file("blob", dg, blob_path, repo_root=msez.REPO_ROOT, store_root=store_root)

    root_dir = tmp_path / "asset-module"
    (root_dir / "state").mkdir(parents=True, exist_ok=True)
    (root_dir / "asset.yaml").write_text("asset_id: test\n", encoding="utf-8")
    (root_dir / "state" / "receipt.json").write_text(
        json.dumps({"artifact_ref": {"artifact_type": "blob", "digest_sha256": dg}}, indent=2) + "\n",
        encoding="utf-8",
    )

    bundle_path = tmp_path / "witness.zip"

    build_rc = msez.cmd_artifact_graph_verify(
        type(
            "BuildArgs",
            (),
            {
                "type": "",
                "digest": "",
                "path": str(root_dir),
                "strict": True,
                "store_root": [str(store_root)],
                "out": "",
                "json": True,
                "max_nodes": 100,
                "max_depth": 8,
                "emit_edges": False,
                "bundle": str(bundle_path),
                "bundle_max_bytes": 0,
                "from_bundle": "",
            },
        )()
    )
    assert build_rc == 0

    # Act: verify from bundle without passing an explicit root (root inferred from manifest)
    verify_rc = msez.cmd_artifact_graph_verify(
        type(
            "VerifyArgs",
            (),
            {
                "type": "",
                "digest": "",
                "path": "",
                "strict": True,
                "store_root": [],
                "out": "",
                "json": True,
                "max_nodes": 100,
                "max_depth": 8,
                "emit_edges": False,
                "bundle": "",
                "bundle_max_bytes": 0,
                "from_bundle": str(bundle_path),
            },
        )()
    )

    # Assert
    assert verify_rc == 0
