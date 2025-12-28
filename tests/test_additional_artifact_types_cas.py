import hashlib
import json


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_schema_artifact_cas_resolves():
    """A schema digest referenced by the transition type registry should be resolvable via the CAS store."""
    from tools.msez import REPO_ROOT
    from tools.lawpack import jcs_canonicalize
    from tools import artifacts as artifact_cas

    schema_path = REPO_ROOT / "schemas" / "transition.payload.example.transfer.v1.schema.json"
    assert schema_path.exists()

    schema_obj = json.loads(schema_path.read_text(encoding="utf-8"))
    digest = _sha256(jcs_canonicalize(schema_obj))

    # This digest is referenced by registries/transition-types.yaml in this repository.
    assert digest == "28249476f011e934f7615a506a37f1e4bf9ba634b4e335194460d6a6296b9efa"

    resolved = artifact_cas.resolve_artifact_by_digest("schema", digest, repo_root=REPO_ROOT)
    assert resolved.exists()


def test_vc_artifact_cas_resolves():
    """A VC (by payload digest) can be stored and resolved via the CAS store."""
    from tools.msez import REPO_ROOT
    from tools.vc import sha256_bytes, signing_input
    from tools import artifacts as artifact_cas

    vc_path = (
        REPO_ROOT
        / "modules"
        / "corridors"
        / "stablecoin-settlement"
        / "corridor.vc.json"
    )
    assert vc_path.exists()

    vc_obj = json.loads(vc_path.read_text(encoding="utf-8"))
    digest = sha256_bytes(signing_input(vc_obj))

    # The repo includes a CAS copy for verifier convenience.
    resolved = artifact_cas.resolve_artifact_by_digest("vc", digest, repo_root=REPO_ROOT)
    assert resolved.exists()
