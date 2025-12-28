import argparse
import json
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey


def test_typed_attachments_are_resolved_in_require_artifacts_mode(tmp_path: Path):
    """Corridor receipts can carry typed attachment artifact references (v0.4.10+).

    In --require-artifacts mode, verifiers should resolve attachments using
    (artifact_type, digest_sha256), while legacy attachments without artifact_type
    default to blob.
    """

    from tools.msez import (
        REPO_ROOT,
        corridor_expected_ruleset_digest_set,
        corridor_state_genesis_root,
        corridor_state_next_root,
        cmd_corridor_state_verify,
    )
    from tools.lawpack import jcs_canonicalize  # type: ignore
    from tools.vc import add_ed25519_proof, did_key_from_ed25519_public_key
    import hashlib

    module_dir = REPO_ROOT / "modules" / "corridors" / "swift"

    genesis = corridor_state_genesis_root(module_dir)
    ruleset_set = corridor_expected_ruleset_digest_set(module_dir)

    # Compute a payload commitment
    payload = {"example": "payload"}
    payload_sha256 = hashlib.sha256(jcs_canonicalize(payload)).hexdigest()

    # A legacy blob attachment (digest exists under dist/artifacts/blob/...)
    legacy_blob_digest = "d6e1b186ddd511ee8b2d28beb530bc2b1acdda18e643f30d22f29bd5332ed5a0"

    # Typed artifacts already present in the reference CAS store.
    schema_digest = "28249476f011e934f7615a506a37f1e4bf9ba634b4e335194460d6a6296b9efa"
    vc_digest = "bc671170cc5263feb53fe332d2c0f59f49a7ef6a6f86499fde41b2bb7b02cde5"

    receipt = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": "org.momentum.msez.corridor.swift.iso20022-cross-border",
        "sequence": 0,
        "timestamp": "2025-01-01T00:00:00Z",
        "prev_root": genesis,
        "lawpack_digest_set": [],
        "ruleset_digest_set": ruleset_set,
        "transition": {
            "type": "MSEZTransitionEnvelope",
            "kind": "generic",
            "payload": payload,
            "payload_sha256": payload_sha256,
            "attachments": [
                # Legacy attachment (no artifact_type) -> defaults to blob
                {"uri": "ipfs://example/legacy.txt", "digest_sha256": legacy_blob_digest},
                # Typed attachments (uri is optional)
                {"artifact_type": "schema", "digest_sha256": schema_digest},
                {"artifact_type": "vc", "digest_sha256": vc_digest},
            ],
        },
    }

    receipt["next_root"] = corridor_state_next_root(receipt)

    # Sign receipt with an ephemeral did:key.
    priv = Ed25519PrivateKey.generate()
    from cryptography.hazmat.primitives import serialization
    pub = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )
    did = did_key_from_ed25519_public_key(pub)
    vm = did + "#key-1"
    add_ed25519_proof(receipt, priv, vm)

    receipts_path = tmp_path / "receipt0.json"
    receipts_path.write_text(json.dumps(receipt, indent=2))

    args = argparse.Namespace(
        path=str(module_dir.relative_to(REPO_ROOT)),
        receipts=str(receipts_path),
        enforce_transition_types=False,
        enforce_trust_anchors=False,
        require_artifacts=True,
        json=False,
    )

    rc = cmd_corridor_state_verify(args)
    assert rc == 0
