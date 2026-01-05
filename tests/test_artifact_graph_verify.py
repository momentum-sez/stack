import json
import pathlib
import hashlib

from tools import artifacts as artifact_cas
from tools import msez


def _sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_artifact_graph_verify_reports_missing(tmp_path: pathlib.Path) -> None:
    """graph verify should report missing referenced artifacts in closure."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    missing_blob = "f" * 64

    # Root structured artifact contains a typed ArtifactRef to a missing blob.
    root_obj = {
        "type": "TestRoot",
        "attachments": [
            {
                "artifact_type": "blob",
                "digest_sha256": missing_blob,
            }
        ],
    }
    root_path = tmp_path / "schema.json"
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
        type("Args", (), {
            "type": "schema",
            "digest": root_digest,
            "path": "",
            "store_root": [str(store_root)],
            "strict": False,
            "max_nodes": 1000,
            "max_depth": 8,
            "out": str(report_path),
            "json": True,
        })()
    )

    assert rc != 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["stats"]["missing_total"] == 1
    assert rep["missing"][0]["artifact_type"] == "blob"
    assert rep["missing"][0]["digest_sha256"] == missing_blob


def test_artifact_graph_verify_strict_detects_digest_mismatch(tmp_path: pathlib.Path) -> None:
    """--strict should flag when on-disk artifact content does not match its digest commitment."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    data = b"hello world"
    real_digest = _sha256_bytes(data)
    wrong_digest = "0" * 64

    blob_path = tmp_path / "blob.bin"
    blob_path.write_bytes(data)

    # Intentionally store under the wrong digest to simulate a tampered CAS entry.
    artifact_cas.store_artifact_file(
        "blob",
        wrong_digest,
        blob_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{wrong_digest}.blob.bin",
        overwrite=True,
    )

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type("Args", (), {
            "type": "blob",
            "digest": wrong_digest,
            "path": "",
            "store_root": [str(store_root)],
            "strict": True,
            "max_nodes": 100,
            "max_depth": 4,
            "out": str(report_path),
            "json": True,
        })()
    )

    assert rc != 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["stats"]["digest_mismatch_total"] == 1
    mm = rep["digest_mismatches"][0]
    assert mm["artifact_type"] == "blob"
    assert mm["digest_sha256"] == wrong_digest
    assert mm["computed_digest_sha256"] == real_digest


def test_artifact_graph_verify_local_file_root(tmp_path: pathlib.Path) -> None:
    """graph verify should accept a local JSON root file (no CAS root) and traverse refs."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    # Store one referenced blob; leave the other missing.
    present_data = b"present"
    present_digest = _sha256_bytes(present_data)
    present_path = tmp_path / "present.bin"
    present_path.write_bytes(present_data)

    artifact_cas.store_artifact_file(
        "blob",
        present_digest,
        present_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{present_digest}.blob.bin",
        overwrite=True,
    )

    missing_digest = "e" * 64

    root = {
        "attachments": [
            {"artifact_type": "blob", "digest_sha256": present_digest},
            {"artifact_type": "blob", "digest_sha256": missing_digest},
        ]
    }
    root_path = tmp_path / "root.json"
    root_path.write_text(json.dumps(root, indent=2) + "\n", encoding="utf-8")

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type("Args", (), {
            "type": "",
            "digest": "",
            "path": str(root_path),
            "store_root": [str(store_root)],
            "strict": False,
            "max_nodes": 1000,
            "max_depth": 8,
            "out": str(report_path),
            "json": True,
        })()
    )

    assert rc != 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["root"]["mode"] == "file"
    assert rep["stats"]["missing_total"] == 1


def test_artifact_graph_verify_transition_types_commitment_root(tmp_path: pathlib.Path) -> None:
    """transition-types lock digests are commitment roots; verify should follow referenced digests."""

    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    # Minimal schema artifact
    schema_obj = {"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object"}
    schema_path = tmp_path / "payload.schema.json"
    schema_path.write_text(json.dumps(schema_obj, indent=2) + "\n", encoding="utf-8")
    schema_digest = msez._jcs_sha256_of_json_file(schema_path)
    artifact_cas.store_artifact_file(
        "schema",
        schema_digest,
        schema_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{schema_digest}.schema.json",
        overwrite=True,
    )

    # Minimal ruleset artifact
    ruleset_obj = {"id": "rs1", "version": 1}
    ruleset_path = tmp_path / "ruleset.json"
    ruleset_path.write_text(json.dumps(ruleset_obj, indent=2) + "\n", encoding="utf-8")
    from tools.lawpack import jcs_canonicalize  # type: ignore

    ruleset_digest = _sha256_bytes(jcs_canonicalize(ruleset_obj))
    artifact_cas.store_artifact_file(
        "ruleset",
        ruleset_digest,
        ruleset_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{ruleset_digest}.ruleset.json",
        overwrite=True,
    )

    # Build a minimal transition-types lock with raw digest strings (legacy form).
    snapshot = {
        "tag": msez.TRANSITION_TYPES_SNAPSHOT_TAG,
        "registry_version": 1,
        "transition_types": [
            {
                "kind": "demo.v1",
                "schema_digest_sha256": schema_digest,
                "ruleset_digest_sha256": ruleset_digest,
            }
        ],
    }
    lock_obj = {
        "transition_types_lock_version": 1,
        "generated_at": "2026-01-01T00:00:00Z",
        "snapshot": snapshot,
        "snapshot_digest_sha256": msez.transition_type_registry_snapshot_digest(snapshot),
    }
    lock_digest = lock_obj["snapshot_digest_sha256"]

    lock_path = tmp_path / "transition-types.lock.json"
    lock_path.write_text(json.dumps(lock_obj, indent=2) + "\n", encoding="utf-8")

    artifact_cas.store_artifact_file(
        "transition-types",
        lock_digest,
        lock_path,
        repo_root=msez.REPO_ROOT,
        store_root=store_root,
        dest_name=f"{lock_digest}.transition-types.lock.json",
        overwrite=True,
    )

    report_path = tmp_path / "report.json"
    rc = msez.cmd_artifact_graph_verify(
        type("Args", (), {
            "type": "transition-types",
            "digest": lock_digest,
            "path": "",
            "store_root": [str(store_root)],
            "strict": True,
            "max_nodes": 1000,
            "max_depth": 8,
            "out": str(report_path),
            "json": True,
        })()
    )

    assert rc == 0
    rep = json.loads(report_path.read_text(encoding="utf-8"))
    assert rep["stats"]["missing_total"] == 0
    assert rep["stats"]["digest_mismatch_total"] == 0
    # Ensure we traversed at least schema + ruleset.
    types = {(n["artifact_type"], n["digest_sha256"]) for n in rep["nodes"]}
    assert ("transition-types", lock_digest) in types
    assert ("schema", schema_digest) in types
    assert ("ruleset", ruleset_digest) in types
