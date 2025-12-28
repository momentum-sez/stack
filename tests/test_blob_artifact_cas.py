import hashlib


def _sha256(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def test_blob_artifact_cas_resolves():
    """A raw byte blob can be resolved via the generic artifact CAS store."""
    from tools.msez import REPO_ROOT
    from tools import artifacts as artifact_cas

    blob_bytes = b"MSEZ example blob artifact (attachments/proofs)\n"
    digest = _sha256(blob_bytes)

    # The repository includes this blob under dist/artifacts/blob/<digest>.*
    assert digest == "d6e1b186ddd511ee8b2d28beb530bc2b1acdda18e643f30d22f29bd5332ed5a0"

    resolved = artifact_cas.resolve_artifact_by_digest("blob", digest, repo_root=REPO_ROOT)
    assert resolved.exists()
    assert resolved.read_bytes() == blob_bytes
