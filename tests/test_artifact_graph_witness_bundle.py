import json
import pathlib
import hashlib
import zipfile

from tools import artifacts as artifact_cas
from tools import msez


def _sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_artifact_graph_verify_emit_edges(tmp_path: pathlib.Path) -> None:
    """--emit-edges should include a machine-readable edge list linking discovered ArtifactRefs."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    # Store a blob
    blob_data = b"hello"
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
    root_obj = {
        "type": "TestRoot",
        "attachments": [
            {
                "artifact_type": "blob",
                "digest_sha256": blob_digest,
            }
        ],
    }
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

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type(
            "Args",
            (),
            {
                "type": "schema",
                "digest": root_digest,
                "path": "",
                "store_root": [str(store_root)],
                "strict": True,
                "emit_edges": True,
                "bundle": "",
                "bundle_max_bytes": 0,
                "max_nodes": 1000,
                "max_depth": 8,
                "out": str(report_path),
                "json": True,
            },
        )()
    )

    assert rc == 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["options"]["emit_edges"] is True
    assert "edges" in rep
    assert rep["stats"]["edges_total"] == len(rep["edges"])

    from_id = f"schema:{root_digest}"
    to_id = f"blob:{blob_digest}"
    assert any(e.get("from") == from_id and e.get("to") == to_id for e in rep["edges"])


def test_artifact_graph_verify_bundle_writes_zip(tmp_path: pathlib.Path) -> None:
    """--bundle should write a witness zip containing manifest + closure artifacts."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    blob_data = b"bundle-data"
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

    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert "bundle" in rep
    assert rep["bundle"]["included_files"] > 0

    with zipfile.ZipFile(bundle_path, "r") as zf:
        names = set(zf.namelist())

    assert "manifest.json" in names
    assert "README.txt" in names
    assert f"artifacts/schema/{root_digest}.schema.json" in names
    assert f"artifacts/blob/{blob_digest}.blob.bin" in names
