import hashlib
import json
import pathlib
import shutil
import zipfile

from tools.lawpack import ingest_lawpack


def test_lawpack_artifact_stores_canonical_akn_bytes(tmp_path: pathlib.Path):
    """The lawpack.zip should store Akoma Ntoso bytes in the canonical form.

    The lawpack index includes `document_sha256` computed over XML C14N bytes.
    For the index to be directly usable (and to make artifact bytes reproducible),
    the zip SHOULD store the canonical bytes, not a pretty-printed variant.
    """

    src_module = pathlib.Path("modules/legal/jurisdictions/ex/civil")
    module_dir = tmp_path / "module"
    shutil.copytree(src_module, module_dir)

    out_dir = tmp_path / "dist" / "lawpacks"

    lock = ingest_lawpack(
        module_dir=module_dir,
        out_dir=out_dir,
        as_of_date="2025-01-01",
        repo_root=tmp_path,
        fetch=False,
        include_raw=False,
        tool_version="test",
    )

    artifact_path = tmp_path / lock["artifact_path"]
    assert artifact_path.exists()

    with zipfile.ZipFile(artifact_path, "r") as zf:
        idx = json.loads(zf.read("index.json").decode("utf-8"))
        # Pick any document.
        doc_path, doc_info = next(iter(idx["documents"].items()))
        xml_bytes = zf.read(doc_path)

    assert hashlib.sha256(xml_bytes).hexdigest() == doc_info["document_sha256"]
