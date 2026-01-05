import json
import os
import time
from datetime import datetime, timezone
from pathlib import Path

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization


def _now_rfc3339() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


@pytest.mark.perf
def test_watcher_compare_scaling(tmp_path: Path):
    """Perf harness: watcher-compare scaling with many watcher VCs.

    Skipped unless MSEZ_RUN_PERF=1.

    Configure:
      - MSEZ_PERF_WATCHERS: number of watcher VCs to generate (default: 100)

    Prints a timing measurement for operator benchmarking.
    """
    from tools.msez import REPO_ROOT, cmd_corridor_state_watcher_compare
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key

    n = int(os.environ.get('MSEZ_PERF_WATCHERS', '100'))

    corridor_id = 'org.momentum.msez.corridor.swift.iso20022-cross-border'
    final_state_root = 'a' * 64
    mmr_root = 'b' * 64
    genesis_root = 'c' * 64
    checkpoint_digest = 'd' * 64

    # Build attestation VC directory.
    vc_dir = tmp_path / 'watchers'
    vc_dir.mkdir(parents=True, exist_ok=True)

    for i in range(n):
        priv = Ed25519PrivateKey.generate()
        pub = priv.public_key().public_bytes(
            encoding=serialization.Encoding.Raw,
            format=serialization.PublicFormat.Raw,
        )
        did = did_key_from_ed25519_public_key(pub)
        vm = did + '#key-1'

        vc = {
            '@context': [
                'https://www.w3.org/2018/credentials/v1',
                'https://schemas.momentum-sez.org/contexts/msez/v1',
            ],
            'type': ['VerifiableCredential', 'MSEZCorridorWatcherAttestationCredential'],
            'issuer': did,
            'issuanceDate': _now_rfc3339(),
            'credentialSubject': {
                'corridor_id': corridor_id,
                'observed_at': _now_rfc3339(),
                'genesis_root': genesis_root,
                'receipt_count': 1000,
                'final_state_root': final_state_root,
                'mmr_root': mmr_root,
                'checkpoint_digest_sha256': checkpoint_digest,
                'no_fork_observed': True,
            },
        }
        add_ed25519_proof(vc, priv, vm)
        (vc_dir / f'watcher_{i:04d}.json').write_text(json.dumps(vc))

    # Minimal corridor module directory: use real swift module.
    corridor_module = REPO_ROOT / 'modules' / 'corridors' / 'swift'

    import argparse
    from io import StringIO
    import sys

    args = argparse.Namespace(
        path=str(corridor_module),
        vcs=str(vc_dir),
        enforce_authority_registry=False,
        require_artifacts=False,
        fail_on_lag=False,
        quorum_threshold='majority',
        require_quorum=False,
        max_staleness='24h',
        format='json',
        out='',
        json=True,
    )

    buf = StringIO()
    old = sys.stdout
    sys.stdout = buf
    try:
        t0 = time.perf_counter()
        rc = cmd_corridor_state_watcher_compare(args)
        dt = time.perf_counter() - t0
    finally:
        sys.stdout = old

    assert rc == 0
    out = json.loads(buf.getvalue())
    assert out.get('summary', {}).get('attestations_analyzed') == n
    print(f'watcher-compare analyzed {n} VCs in {dt:.3f}s ({(n/dt if dt>0 else 0):.1f} vcs/s)')
