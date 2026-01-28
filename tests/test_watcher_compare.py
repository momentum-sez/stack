import argparse
import json
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def _make_signed_watcher_vc(tmp_path: Path, corridor_id: str, receipt_count: int, final_state_root: str) -> Path:
    """Create a minimal signed watcher attestation VC for tests."""
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key
    from cryptography.hazmat.primitives import serialization
    from datetime import datetime, timezone

    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"

    # Use current time for freshness (avoid staleness issues in test suite)
    now_str = datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")

    vc = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "type": ["VerifiableCredential", "MSEZCorridorWatcherAttestationCredential"],
        "id": "urn:uuid:00000000-0000-0000-0000-000000000000",
        "issuer": did,
        "issuanceDate": now_str,
        "credentialSubject": {
            "corridor_id": corridor_id,
            "observed_at": now_str,
            "receipt_count": receipt_count,
            "final_state_root": final_state_root,
            # Minimal checkpoint commitment (digest only is allowed; ArtifactRef also allowed)
            "checkpoint_digest_sha256": "0" * 64,
            "no_fork_observed": True,
        },
    }
    add_ed25519_proof(vc, priv, vm)
    out = tmp_path / f"watcher_{receipt_count}_{final_state_root[:8]}.vc.json"
    out.write_text(json.dumps(vc, indent=2))
    return out


def test_watcher_compare_ok(tmp_path: Path):
    from tools.msez import REPO_ROOT, cmd_corridor_state_watcher_compare

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    # Two independent watchers reporting the same head => OK
    wdir = tmp_path / "watchers_ok"
    wdir.mkdir()
    _make_signed_watcher_vc(wdir, corridor_id, 10, "a" * 64)
    _make_signed_watcher_vc(wdir, corridor_id, 10, "a" * 64)

    args = argparse.Namespace(
        path=str(module_dir.relative_to(REPO_ROOT)),
        vcs=str(wdir),
        enforce_authority_registry=False,
        require_artifacts=False,
        fail_on_lag=False,
        json=False,
    )
    rc = cmd_corridor_state_watcher_compare(args)
    assert rc == 0


def test_watcher_compare_detects_fork_like_divergence(tmp_path: Path):
    from tools.msez import REPO_ROOT, cmd_corridor_state_watcher_compare

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    # Two independent watchers reporting different final_state_root for the SAME receipt_count
    wdir = tmp_path / "watchers_fork"
    wdir.mkdir()
    _make_signed_watcher_vc(wdir, corridor_id, 10, "b" * 64)
    _make_signed_watcher_vc(wdir, corridor_id, 10, "c" * 64)

    args = argparse.Namespace(
        path=str(module_dir.relative_to(REPO_ROOT)),
        vcs=str(wdir),
        enforce_authority_registry=False,
        require_artifacts=False,
        fail_on_lag=False,
        max_staleness="24h",  # Allow 24h staleness to prevent flakiness
        json=False,
    )
    rc = cmd_corridor_state_watcher_compare(args)
    assert rc == 2
