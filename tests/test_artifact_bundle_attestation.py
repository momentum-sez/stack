import json
import pathlib
import hashlib

from tools import artifacts as artifact_cas
from tools import msez
from tools import vc as vc_tools


def _sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _make_minimal_witness_bundle(tmp_path: pathlib.Path) -> pathlib.Path:
    """Create a minimal witness bundle by storing a schema -> blob reference in CAS and bundling it."""
    store_root = tmp_path / "artifacts"
    store_root.mkdir(parents=True, exist_ok=True)

    # Store a blob
    blob_data = b"bundle-attest"
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
    return bundle_path


def test_artifact_bundle_attest_and_verify_signed(tmp_path: pathlib.Path) -> None:
    bundle_path = _make_minimal_witness_bundle(tmp_path)

    # Generate a signing key
    jwk = vc_tools.generate_ed25519_jwk(kid="key-1")
    key_path = tmp_path / "dev.ed25519.jwk"
    key_path.write_text(json.dumps(jwk, indent=2) + "\n", encoding="utf-8")
    _priv, did = vc_tools.load_ed25519_private_key_from_jwk(jwk)

    out_vc = tmp_path / "bundle.attestation.vc.json"
    rc = msez.cmd_artifact_bundle_attest(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "issuer": did,
                "id": "",
                "statement": "test",
                "out": str(out_vc),
                "sign": True,
                "key": str(key_path),
                "verification_method": "",
                "purpose": "assertionMethod",
            },
        )()
    )
    assert rc == 0
    assert out_vc.exists()

    vcj = json.loads(out_vc.read_text(encoding="utf-8"))
    assert "proof" in vcj

    # Verify: digest match + proof ok.
    rc2 = msez.cmd_artifact_bundle_verify(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "vc": str(out_vc),
                "require_proof": True,
                "json": True,
            },
        )()
    )
    assert rc2 == 0


def test_artifact_bundle_verify_allows_unsigned_with_flag(tmp_path: pathlib.Path) -> None:
    bundle_path = _make_minimal_witness_bundle(tmp_path)

    out_vc = tmp_path / "bundle.attestation.vc.unsigned.json"
    rc = msez.cmd_artifact_bundle_attest(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "issuer": "did:example:issuer",
                "id": "",
                "statement": "unsigned",
                "out": str(out_vc),
                "sign": False,
                "key": "",
                "verification_method": "",
                "purpose": "assertionMethod",
            },
        )()
    )
    assert rc == 0
    assert out_vc.exists()

    # By default, require_proof=True should fail.
    rc_fail = msez.cmd_artifact_bundle_verify(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "vc": str(out_vc),
                "require_proof": True,
                "json": True,
            },
        )()
    )
    assert rc_fail != 0

    # With --no-require-proof semantics, allow unsigned.
    rc_ok = msez.cmd_artifact_bundle_verify(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "vc": str(out_vc),
                "require_proof": False,
                "json": True,
            },
        )()
    )
    assert rc_ok == 0


def test_artifact_bundle_verify_fails_on_manifest_tamper(tmp_path: pathlib.Path) -> None:
    bundle_path = _make_minimal_witness_bundle(tmp_path)

    jwk = vc_tools.generate_ed25519_jwk(kid="key-1")
    key_path = tmp_path / "dev.ed25519.jwk"
    key_path.write_text(json.dumps(jwk, indent=2) + "\n", encoding="utf-8")
    _priv, did = vc_tools.load_ed25519_private_key_from_jwk(jwk)

    out_vc = tmp_path / "bundle.attestation.vc.json"
    rc = msez.cmd_artifact_bundle_attest(
        type(
            "Args",
            (),
            {
                "bundle": str(bundle_path),
                "issuer": did,
                "id": "",
                "statement": "tamper",
                "out": str(out_vc),
                "sign": True,
                "key": str(key_path),
                "verification_method": "",
                "purpose": "assertionMethod",
            },
        )()
    )
    assert rc == 0

    # Tamper with manifest.json inside the bundle.
    import zipfile

    tampered = tmp_path / "witness.tampered.zip"
    with zipfile.ZipFile(bundle_path, "r") as zin, zipfile.ZipFile(tampered, "w", compression=zipfile.ZIP_DEFLATED) as zout:
        for item in zin.infolist():
            data = zin.read(item.filename)
            if item.filename == "manifest.json":
                obj = json.loads(data.decode("utf-8"))
                obj["tampered"] = True
                data = json.dumps(obj, indent=2).encode("utf-8")
            zout.writestr(item, data)

    rc2 = msez.cmd_artifact_bundle_verify(
        type(
            "Args",
            (),
            {
                "bundle": str(tampered),
                "vc": str(out_vc),
                "require_proof": True,
                "json": True,
            },
        )()
    )
    assert rc2 != 0
