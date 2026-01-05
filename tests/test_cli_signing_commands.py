import argparse
import json
from pathlib import Path


def _write_jwk(tmp_path: Path, *, kid: str = "key-1") -> tuple[Path, str]:
    """Write a private Ed25519 JWK to disk and return (path, did)."""
    from tools.vc import generate_ed25519_jwk, load_ed25519_private_key_from_jwk  # type: ignore

    jwk = generate_ed25519_jwk(kid=kid)
    _priv, did = load_ed25519_private_key_from_jwk(jwk)
    p = tmp_path / "key.jwk.json"
    p.write_text(json.dumps(jwk, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return p, did


def test_cli_watcher_attest_signs_vc(tmp_path: Path):
    """Ensure `corridor state watcher-attest --sign` produces a verifiable VC."""
    from tools.msez import REPO_ROOT, cmd_corridor_state_watcher_attest
    from tools.vc import verify_credential  # type: ignore

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    checkpoint = {
        "type": "MSEZCorridorCheckpoint",
        "corridor_id": corridor_id,
        "genesis_root": "a" * 64,
        "receipt_count": 7,
        "final_state_root": "b" * 64,
        "mmr": {"size": 7, "root": "c" * 64, "peaks": []},
        "proof": [],
    }
    cp_path = tmp_path / "checkpoint.json"
    cp_path.write_text(json.dumps(checkpoint, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    key_path, did = _write_jwk(tmp_path)

    out_path = tmp_path / "watcher.vc.json"
    args = argparse.Namespace(
        path=str(module_dir.relative_to(REPO_ROOT)),
        checkpoint=str(cp_path),
        issuer=did,
        id="",
        observed_at="",
        finality_level="",
        no_fork_observed=True,
        store_artifacts=False,
        sign=True,
        key=str(key_path),
        out=str(out_path),
    )
    rc = cmd_corridor_state_watcher_attest(args)
    assert rc == 0
    assert out_path.exists()

    vc = json.loads(out_path.read_text(encoding="utf-8"))
    res = verify_credential(vc)
    assert any(r.ok for r in res)


def test_cli_fork_alarm_signs_vc(tmp_path: Path):
    """Ensure `corridor state fork-alarm --sign` produces a verifiable VC."""
    from tools.msez import REPO_ROOT, cmd_corridor_state_fork_alarm
    from tools.vc import verify_credential  # type: ignore

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"
    corridor_id = "org.momentum.msez.corridor.swift.iso20022-cross-border"

    # Two conflicting receipts at the same (sequence, prev_root)
    receipt_a = {
        "type": "MSEZCorridorReceipt",
        "corridor_id": corridor_id,
        "sequence": 10,
        "prev_root": "0" * 64,
        "transition": {"kind": "test", "payload_sha256": "1" * 64},
        "next_root": "a" * 64,
        "proof": [],
    }
    receipt_b = dict(receipt_a)
    receipt_b["next_root"] = "b" * 64

    ra = tmp_path / "receipt_a.json"
    rb = tmp_path / "receipt_b.json"
    ra.write_text(json.dumps(receipt_a, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    rb.write_text(json.dumps(receipt_b, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    key_path, did = _write_jwk(tmp_path)
    out_path = tmp_path / "fork_alarm.vc.json"

    args = argparse.Namespace(
        path=str(module_dir.relative_to(REPO_ROOT)),
        receipt_a=str(ra),
        receipt_b=str(rb),
        issuer=did,
        id="",
        detected_at="",
        store_artifacts=False,
        sign=True,
        key=str(key_path),
        out=str(out_path),
    )
    rc = cmd_corridor_state_fork_alarm(args)
    assert rc == 0
    assert out_path.exists()

    vc = json.loads(out_path.read_text(encoding="utf-8"))
    res = verify_credential(vc)
    assert any(r.ok for r in res)


def test_cli_availability_attest_signs_vc(tmp_path: Path):
    """Ensure `corridor availability attest --sign` produces a verifiable VC.

    This constructs a minimal corridor module with party-specific agreement VCs
    pinning lawpacks.
    """
    from tools.msez import REPO_ROOT, cmd_corridor_availability_attest
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key, verify_credential  # type: ignore
    from tools.vc import now_rfc3339  # type: ignore
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    from cryptography.hazmat.primitives import serialization

    module_dir = tmp_path / "corridor_mod"
    module_dir.mkdir()
    corridor_id = "org.momentum.msez.corridor.fixture.availability"

    # Create two zone authority DIDs.
    za = Ed25519PrivateKey.generate()
    zb = Ed25519PrivateKey.generate()
    za_pub = za.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    zb_pub = zb.public_key().public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
    did_a = did_key_from_ed25519_public_key(za_pub)
    did_b = did_key_from_ed25519_public_key(zb_pub)
    vm_a = did_a + "#key-1"
    vm_b = did_b + "#key-1"

    # Minimal trust anchors.
    (module_dir / "trust-anchors.yaml").write_text(
        """version: 1
trust_anchors:
  - anchor_id: zone-a
    type: did
    identifier: """ + did_a + """
    allowed_attestations: [corridor_agreement]
  - anchor_id: zone-b
    type: did
    identifier: """ + did_b + """
    allowed_attestations: [corridor_agreement]
""",
        encoding="utf-8",
    )
    (module_dir / "key-rotation.yaml").write_text("version: 1\npolicy: {}\n", encoding="utf-8")

    # Minimal definition VC (digest is what matters for agreement binding).
    def_vc = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": "urn:msez:vc:corridor-definition:fixture.availability",
        "type": ["VerifiableCredential", "MSEZCorridorDefinitionCredential"],
        "issuer": did_a,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "ruleset": "msez.corridor.verification.v1",
            "artifacts": {},
            "version": "0.4.20",
        },
        "proof": [],
    }
    (module_dir / "corridor.vc.json").write_text(json.dumps(def_vc, indent=2) + "\n", encoding="utf-8")

    # Agreement VCs with party-specific pins.
    from tools.vc import signing_input  # type: ignore
    from tools.lawpack import sha256_bytes  # type: ignore

    def_digest = sha256_bytes(signing_input(def_vc))
    pinned_digest = "d" * 64

    ag_a = {
        "@context": def_vc["@context"],
        "id": "urn:msez:vc:corridor-agreement:fixture.availability:a",
        "type": ["VerifiableCredential", "MSEZCorridorAgreementCredential"],
        "issuer": did_a,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "definition_vc_id": def_vc["id"],
            "definition_payload_sha256": def_digest,
            "participants": [
                {"id": did_a, "role": "zone_authority", "name": "Zone A"},
                {"id": did_b, "role": "zone_authority", "name": "Zone B"},
            ],
            "activation": {"thresholds": [{"role": "zone_authority", "required": 2, "of": 2}]},
            "terms": {"reference": "urn:msez:fixture:terms"},
            "party": {"id": did_a, "role": "zone_authority", "name": "Zone A"},
            "commitment": "agree",
            "pinned_lawpacks": [
                {"jurisdiction_id": "fixture", "domain": "civil", "lawpack_digest_sha256": pinned_digest}
            ],
            "version": "0.4.20",
            "maintainer": "fixture",
        },
    }
    add_ed25519_proof(ag_a, za, vm_a)
    (module_dir / "agreement.a.vc.json").write_text(json.dumps(ag_a, indent=2) + "\n", encoding="utf-8")

    ag_b = json.loads(json.dumps(ag_a))
    ag_b["id"] = "urn:msez:vc:corridor-agreement:fixture.availability:b"
    ag_b["issuer"] = did_b
    ag_b["credentialSubject"]["party"] = {"id": did_b, "role": "zone_authority", "name": "Zone B"}
    add_ed25519_proof(ag_b, zb, vm_b)
    (module_dir / "agreement.b.vc.json").write_text(json.dumps(ag_b, indent=2) + "\n", encoding="utf-8")

    # corridor.yaml
    (module_dir / "corridor.yaml").write_text(
        """corridor_id: """ + corridor_id + """
participants:
  - """ + did_a + """
  - """ + did_b + """
settlement: {type: fiat-correspondent, currency: USD}
recognition: {passporting: []}
attestations: {required: []}
dispute_resolution: {method: arbitration}
trust_anchors_path: trust-anchors.yaml
key_rotation_path: key-rotation.yaml
definition_vc_path: corridor.vc.json
agreement_vc_path:
  - agreement.a.vc.json
  - agreement.b.vc.json
verification_ruleset: msez.corridor.verification.v1
""",
        encoding="utf-8",
    )

    # Availability VC signer (separate key)
    key_path, did = _write_jwk(tmp_path)
    out_path = tmp_path / "availability.vc.json"

    args = argparse.Namespace(
        path=str(module_dir),
        issuer=did,
        id="",
        as_of="",
        endpoint=[],
        notes="",
        sign=True,
        key=str(key_path),
        out=str(out_path),
    )

    rc = cmd_corridor_availability_attest(args)
    assert rc == 0
    assert out_path.exists()
    vc = json.loads(out_path.read_text(encoding="utf-8"))
    res = verify_credential(vc)
    assert any(r.ok for r in res)
