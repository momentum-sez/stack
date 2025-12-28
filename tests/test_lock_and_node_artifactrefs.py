from tools.msez import REPO_ROOT, schema_validator, validate_with_schema


def test_stack_lock_digest_fields_accept_artifactrefs():
    """v0.4.13+: stack.lock digest-bearing fields may carry ArtifactRef objects.

    This keeps zone lockfiles mechanically consistent with the ArtifactRef pattern used
    throughout receipts/VCs.
    """

    schema = schema_validator(REPO_ROOT / "schemas" / "stack.lock.schema.json")

    lock = {
        "stack_spec_version": "0.4.13",
        "generated_at": "2025-01-01T00:00:00Z",
        "zone_id": "org.example.zone.demo",
        "profile": {"profile_id": "org.example.profile", "version": "0.0.1"},
        "modules": [
            {
                "module_id": "org.example.module",
                "version": "0.0.1",
                "variant": "default",
                "manifest_sha256": "0" * 64,
            }
        ],
        "lawpacks": [
            {
                "jurisdiction_id": "us-ca",
                "domain": "civil",
                "lawpack_digest_sha256": {
                    "artifact_type": "lawpack",
                    "digest_sha256": "1" * 64,
                },
            }
        ],
        "corridors": [
            {
                "corridor_id": "org.example.corridor.demo",
                "corridor_manifest_sha256": {"artifact_type": "blob", "digest_sha256": "2" * 64},
                "trust_anchors_sha256": {"artifact_type": "blob", "digest_sha256": "3" * 64},
                "key_rotation_sha256": {"artifact_type": "blob", "digest_sha256": "4" * 64},
                "corridor_definition_vc_sha256": {"artifact_type": "blob", "digest_sha256": "5" * 64},
                "corridor_definition_signers": ["did:key:z6Mkexample"],
            }
        ],
    }

    errors = validate_with_schema(lock, schema)
    assert not errors, "\n".join(errors)


def test_node_descriptor_accepts_artifactrefs():
    """v0.4.13+: node.yaml digest-bearing fields may carry ArtifactRef objects."""

    schema = schema_validator(REPO_ROOT / "schemas" / "node.schema.json")

    node = {
        "node_id": "did:key:z6MkexampleNode",
        "zone_id": "org.example.zone.demo",
        "endpoints": {"api": "https://example.invalid/api"},
        "capabilities": {
            "zk": {
                "proof_systems": ["groth16"],
                "verifying_keys": [
                    {"artifact_type": "proof-key", "digest_sha256": "6" * 64, "key_id": "vk-1"}
                ],
            }
        },
        "attestations": [
            {"artifact_type": "vc", "digest_sha256": "7" * 64, "uri": "dist/artifacts/vc/7..."}
        ],
    }

    errors = validate_with_schema(node, schema)
    assert not errors, "\n".join(errors)
