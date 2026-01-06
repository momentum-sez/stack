import argparse
import json
from pathlib import Path


def _write_json(p: Path, obj: object) -> None:
    p.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def test_smart_asset_genesis_checkpoint_and_registry_vc_validate_and_verify(tmp_path: Path):
    from tools import smart_asset  # type: ignore
    from tools.msez import REPO_ROOT, schema_validator, validate_with_schema  # type: ignore
    from tools.vc import generate_ed25519_jwk, load_ed25519_private_key_from_jwk, verify_credential  # type: ignore

    # --- genesis ---
    genesis = smart_asset.build_genesis(
        stack_spec_version="0.4.30",
        asset_name="Acme Bond",
        asset_class="security",
        description="Test non-tokenized instrument",
        created_at="2026-01-01T00:00:00Z",
    )
    gval = schema_validator(REPO_ROOT / "schemas" / "smart-asset.genesis.schema.json")
    assert not validate_with_schema(genesis, gval)

    gpath = tmp_path / "genesis.json"
    _write_json(gpath, genesis)

    # --- checkpoint ---
    state = {"balance": 100, "owner": "did:key:alice"}
    spath = tmp_path / "state.json"
    _write_json(spath, state)
    state_root = smart_asset.state_root_from_state(state)
    assert len(state_root) == 64

    # --- registry VC (signed) ---
    bindings = [
        {
            "harbor_id": "zone-a",
            "binding_status": "active",
            "shard_role": "primary",
            "lawpacks": [{"jurisdiction_id": "zone-a", "domain": "financial", "lawpack_digest_sha256": "a" * 64}],
            "compliance_profile": {
                "allowed_transition_kinds": ["transfer"],
                "required_attestations": {"transfer": ["kyc.passed.v1"]},
            },
        }
    ]
    bpath = tmp_path / "bindings.yaml"
    bpath.write_text("jurisdiction_bindings:\n  - harbor_id: zone-a\n    binding_status: active\n    shard_role: primary\n    lawpacks:\n      - jurisdiction_id: zone-a\n        domain: financial\n        lawpack_digest_sha256: " + ("a" * 64) + "\n    compliance_profile:\n      allowed_transition_kinds: [transfer]\n      required_attestations:\n        transfer: [kyc.passed.v1]\n", encoding="utf-8")

    jwk = generate_ed25519_jwk(kid="key-1")
    _priv, did = load_ed25519_private_key_from_jwk(jwk)
    kpath = tmp_path / "key.jwk.json"
    _write_json(kpath, jwk)

    out_vc = tmp_path / "registry.vc.json"
    args = argparse.Namespace(
        stack_spec_version="0.4.30",
        genesis=str(gpath),
        bindings=str(bpath),
        issuer=did,
        issuance_date="",
        id="",
        out=str(out_vc),
        sign=True,
        key=str(kpath),
        verification_method="",
        purpose="assertionMethod",
    )
    rc = smart_asset.cmd_asset_registry_init(args)
    assert rc == 0

    vcj = json.loads(out_vc.read_text(encoding="utf-8"))
    # Signed VC should validate against schema + have verifiable proof.
    vval = schema_validator(REPO_ROOT / "schemas" / "vc.smart-asset-registry.schema.json")
    assert not validate_with_schema(vcj, vval)

    proofs = verify_credential(vcj)
    assert any(p.ok for p in proofs)


def test_smart_asset_compliance_tensor_basic(tmp_path: Path):
    """Declarative compliance matrix: each harbor can require different attestations."""

    from tools import smart_asset  # type: ignore
    from tools.vc import now_rfc3339  # type: ignore
    from tools.artifacts import store_artifact_file  # type: ignore

    store_root = tmp_path / "artifacts"

    # Create two attestation artifacts in CAS.
    def mk_att(kind: str) -> dict:
        return {
            "type": "SmartAssetAttestation",
            "asset_id": "f" * 64,
            "issued_at": now_rfc3339(),
            "issuer": "did:key:issuer",
            "kind": kind,
        }

    att_kyc = mk_att("kyc.passed.v1")
    att_aml = mk_att("aml.passed.v1")
    att_kyc_path = tmp_path / "kyc.json"
    att_aml_path = tmp_path / "aml.json"
    _write_json(att_kyc_path, att_kyc)
    _write_json(att_aml_path, att_aml)

    dg_kyc = smart_asset.sha256_hex(smart_asset.canonicalize_json(att_kyc))
    dg_aml = smart_asset.sha256_hex(smart_asset.canonicalize_json(att_aml))

    store_artifact_file(
        artifact_type="smart-asset-attestation",
        digest_sha256=dg_kyc,
        src_path=att_kyc_path,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )
    store_artifact_file(
        artifact_type="smart-asset-attestation",
        digest_sha256=dg_aml,
        src_path=att_aml_path,
        repo_root=smart_asset.REPO_ROOT,
        store_root=store_root,
        overwrite=True,
    )

    registry_vc = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "type": ["VerifiableCredential", "MsezSmartAssetRegistryCredential"],
        "issuer": "did:key:issuer",
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "asset_id": "f" * 64,
            "stack_spec_version": "0.4.30",
            "asset_genesis": {
                "artifact_type": "smart-asset-genesis",
                "digest_sha256": "f" * 64,
                "uri": "urn:example:genesis",
                "media_type": "application/json",
            },
            "jurisdiction_bindings": [
                {
                    "harbor_id": "zone-a",
                    "binding_status": "active",
                    "shard_role": "primary",
                    "lawpacks": [],
                    "compliance_profile": {
                        "allowed_transition_kinds": ["transfer"],
                        "required_attestations": {"transfer": ["kyc.passed.v1"]},
                    },
                },
                {
                    "harbor_id": "zone-b",
                    "binding_status": "active",
                    "shard_role": "replica",
                    "lawpacks": [],
                    "compliance_profile": {
                        "allowed_transition_kinds": ["transfer"],
                        "required_attestations": {"transfer": ["kyc.passed.v1", "aml.passed.v1"]},
                    },
                },
            ],
        },
    }

    transition = {
        "type": "TransitionEnvelope",
        "transition_kind": "transfer",
        "timestamp": now_rfc3339(),
        "payload": {"to": "did:key:bob", "amount": 10},
        "attachments": [
            {
                "artifact_type": "smart-asset-attestation",
                "digest_sha256": dg_kyc,
                "uri": "urn:example:att:kyc",
                "media_type": "application/json",
            }
        ],
    }

    res = smart_asset.evaluate_transition_compliance(
        registry_vc=registry_vc,
        transition_envelope=transition,
        store_roots=[store_root],
    )
    by_harbor = {r.harbor_id: r for r in res}
    assert by_harbor["zone-a"].allowed is True
    assert by_harbor["zone-b"].allowed is False
    assert "aml.passed.v1" in by_harbor["zone-b"].missing_attestations

    # Add AML attestation -> both should pass.
    transition["attachments"].append(
        {
            "artifact_type": "smart-asset-attestation",
            "digest_sha256": dg_aml,
            "uri": "urn:example:att:aml",
            "media_type": "application/json",
        }
    )
    res2 = smart_asset.evaluate_transition_compliance(
        registry_vc=registry_vc,
        transition_envelope=transition,
        store_roots=[store_root],
    )
    by_harbor2 = {r.harbor_id: r for r in res2}
    assert by_harbor2["zone-a"].allowed is True
    assert by_harbor2["zone-b"].allowed is True
