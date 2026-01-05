import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def _now() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace('+00:00', 'Z')


def _sign_vc(vc: dict) -> None:
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key
    from cryptography.hazmat.primitives import serialization

    priv = Ed25519PrivateKey.generate()
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"
    add_ed25519_proof(vc, priv, vm)


def test_operational_to_halted_requires_fork_alarm(tmp_path: Path):
    from tools.msez import REPO_ROOT
    from tools.lifecycle import apply_lifecycle_transition, load_state_machine, default_state_machine_path

    sm = load_state_machine(default_state_machine_path(REPO_ROOT))

    lifecycle = {
        "type": "MSEZCorridorLifecycle",
        "corridor_id": "test.corridor",
        "state": "OPERATIONAL",
        "since": "2025-01-01T00:00:00Z",
    }

    transition_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorLifecycleTransitionCredential"],
        "issuer": "did:key:z6Mk-test",
        "issuanceDate": _now(),
        "credentialSubject": {
            "type": "MSEZCorridorLifecycleTransition",
            "corridor_id": "test.corridor",
            "from_state": "OPERATIONAL",
            "to_state": "HALTED",
            "transitioned_at": _now(),
            "reason": "fork alarm",
        },
    }
    _sign_vc(transition_vc)

    updated, errs = apply_lifecycle_transition(
        lifecycle,
        transition_vc,
        state_machine=sm,
        evidence=[],
        repo_root=REPO_ROOT,
    )
    assert errs
    assert "missing required evidence VC types" in errs[0]

    fork_alarm_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorForkAlarmCredential"],
        "issuer": "did:key:z6Mk-watcher",
        "issuanceDate": _now(),
        "credentialSubject": {
            "corridor_id": "test.corridor",
            "detected_at": _now(),
            "sequence": 0,
            "prev_root": "0" * 64,
            "next_root_a": "1" * 64,
            "next_root_b": "2" * 64,
        },
    }
    _sign_vc(fork_alarm_vc)

    updated, errs = apply_lifecycle_transition(
        lifecycle,
        transition_vc,
        state_machine=sm,
        evidence=[fork_alarm_vc],
        repo_root=REPO_ROOT,
    )
    assert not errs
    assert updated["state"] == "HALTED"


def test_halted_to_operational_requires_fork_resolution(tmp_path: Path):
    from tools.msez import REPO_ROOT
    from tools.lifecycle import apply_lifecycle_transition, load_state_machine, default_state_machine_path

    sm = load_state_machine(default_state_machine_path(REPO_ROOT))

    lifecycle = {
        "type": "MSEZCorridorLifecycle",
        "corridor_id": "test.corridor",
        "state": "HALTED",
        "since": "2025-01-01T00:00:00Z",
    }

    transition_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorLifecycleTransitionCredential"],
        "issuer": "did:key:z6Mk-test",
        "issuanceDate": _now(),
        "credentialSubject": {
            "type": "MSEZCorridorLifecycleTransition",
            "corridor_id": "test.corridor",
            "from_state": "HALTED",
            "to_state": "OPERATIONAL",
            "transitioned_at": _now(),
            "reason": "fork resolved",
        },
    }
    _sign_vc(transition_vc)

    updated, errs = apply_lifecycle_transition(
        lifecycle,
        transition_vc,
        state_machine=sm,
        evidence=[],
        repo_root=REPO_ROOT,
    )
    assert errs
    assert "missing required evidence VC types" in errs[0]

    fork_res_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MSEZCorridorForkResolutionCredential"],
        "issuer": "did:key:z6Mk-authority",
        "issuanceDate": _now(),
        "credentialSubject": {
            "corridor_id": "test.corridor",
            "sequence": 0,
            "prev_root": "0" * 64,
            "candidate_next_roots": ["1" * 64, "2" * 64],
            "chosen_next_root": "1" * 64,
            "reason": "canonical selection",
            "decided_at": _now(),
        },
    }
    _sign_vc(fork_res_vc)

    updated, errs = apply_lifecycle_transition(
        lifecycle,
        transition_vc,
        state_machine=sm,
        evidence=[fork_res_vc],
        repo_root=REPO_ROOT,
    )
    assert not errs
    assert updated["state"] == "OPERATIONAL"
