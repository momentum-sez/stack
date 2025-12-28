import json
import pathlib
import shutil

from tools.lawpack import ingest_lawpack


def test_lawpack_ingest_is_deterministic(tmp_path: pathlib.Path):
    # Copy the small example corpus into a temp workspace so the test does not mutate the repo.
    src_module = pathlib.Path("modules/legal/jurisdictions/ex/civil")
    module_dir = tmp_path / "module"
    shutil.copytree(src_module, module_dir)

    out_dir = tmp_path / "dist" / "lawpacks"
    repo_root = tmp_path

    lock1 = ingest_lawpack(
        module_dir=module_dir,
        out_dir=out_dir,
        as_of_date="2025-01-01",
        repo_root=repo_root,
        fetch=False,
        include_raw=False,
        tool_version="test",
    )

    # Second run should produce the exact same digest and zip bytes.
    lock2 = ingest_lawpack(
        module_dir=module_dir,
        out_dir=out_dir,
        as_of_date="2025-01-01",
        repo_root=repo_root,
        fetch=False,
        include_raw=False,
        tool_version="test",
    )

    assert lock1["lawpack_digest_sha256"] == lock2["lawpack_digest_sha256"]
    assert lock1["artifact_sha256"] == lock2["artifact_sha256"]

    # Index must exist in the artifact, and contain at least one fragment entry for eId.
    artifact_rel = lock1["artifact_path"]
    artifact_path = repo_root / artifact_rel
    assert artifact_path.exists()

    import zipfile

    with zipfile.ZipFile(artifact_path, "r") as zf:
        idx = json.loads(zf.read("index.json").decode("utf-8"))
    assert idx["jurisdiction_id"] == "ex"
    assert idx["domain"] == "civil"
    assert "documents" in idx and idx["documents"]
    any_doc = next(iter(idx["documents"].values()))
    assert "fragments" in any_doc and isinstance(any_doc["fragments"], dict)
    assert any_doc["fragments"], "expected at least one eId fragment in the example corpus"
