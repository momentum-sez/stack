"""
Verify that security-critical schemas reject documents with injected extra fields
at locked-down levels while accepting them at intentionally extensible levels.

This test validates the TIER 1A schema hardening from the Feb 2026 audit.
"""
import json
import pathlib

import jsonschema

SCHEMAS_DIR = pathlib.Path("schemas")


def _load_schema(name: str) -> dict:
    """Load a schema by filename."""
    path = SCHEMAS_DIR / name
    return json.loads(path.read_text())


def _make_resolver():
    """Create a resolver that can resolve $ref across all schemas."""
    store = {}
    for f in SCHEMAS_DIR.glob("*.json"):
        try:
            schema = json.loads(f.read_text())
            if "$id" in schema:
                store[schema["$id"]] = schema
        except Exception:
            pass
    from jsonschema import RefResolver
    return RefResolver("", {}, store=store)


def _validate(schema, instance):
    """Validate instance against schema, return list of errors."""
    resolver = _make_resolver()
    validator = jsonschema.Draft202012Validator(schema, resolver=resolver)
    return sorted(validator.iter_errors(instance), key=str)


# ════════════════════════════════════════════════════════════════════════════
# ENVELOPE REJECTION TESTS — extra top-level fields must be rejected
# ════════════════════════════════════════════════════════════════════════════

class TestVCEnvelopeHardening:
    """VC envelope schemas must reject additional properties."""

    def test_vc_schema_rejects_extra_top_level_fields(self):
        schema = _load_schema("vc.schema.json")
        valid_vc = {
            "@context": ["https://www.w3.org/2018/credentials/v1"],
            "type": ["VerifiableCredential"],
            "issuer": "did:key:test",
            "issuanceDate": "2026-01-01T00:00:00Z",
            "credentialSubject": {"id": "test"},
            "proof": {
                "type": "Ed25519Signature2020",
                "created": "2026-01-01T00:00:00Z",
                "verificationMethod": "did:key:test#key-1",
                "proofPurpose": "assertionMethod",
                "proofValue": "abc123"
            },
            "INJECTED_FIELD": "malicious"
        }
        errors = _validate(schema, valid_vc)
        injection_errors = [e for e in errors if "INJECTED_FIELD" in str(e)]
        assert injection_errors, "VC schema should reject additional top-level fields"

    def test_vc_schema_allows_credential_subject_extension(self):
        schema = _load_schema("vc.schema.json")
        valid_vc = {
            "@context": ["https://www.w3.org/2018/credentials/v1"],
            "type": ["VerifiableCredential"],
            "issuer": "did:key:test",
            "issuanceDate": "2026-01-01T00:00:00Z",
            "credentialSubject": {
                "id": "test",
                "custom_claim": "allowed per W3C VC spec"
            },
            "proof": {
                "type": "Ed25519Signature2020",
                "created": "2026-01-01T00:00:00Z",
                "verificationMethod": "did:key:test#key-1",
                "proofPurpose": "assertionMethod",
                "proofValue": "abc123"
            }
        }
        errors = _validate(schema, valid_vc)
        subject_errors = [e for e in errors if "custom_claim" in str(e)]
        assert not subject_errors, "credentialSubject should accept extension fields"


class TestAttestationHardening:
    """Attestation schema must reject additional properties."""

    def test_attestation_rejects_extra_top_level_fields(self):
        schema = _load_schema("attestation.schema.json")
        instance = {
            "attestation_type": "compliance",
            "issuer": "did:key:test",
            "subject": "asset-001",
            "issued_at": "2026-01-01T00:00:00Z",
            "claims": {"status": "compliant"},
            "proof": {
                "type": "Ed25519Signature2020",
                "verification_method": "did:key:test#key-1",
                "signature": "abc123"
            },
            "INJECTED_FIELD": "malicious"
        }
        errors = _validate(schema, instance)
        assert any("INJECTED_FIELD" in str(e) for e in errors), \
            "Attestation schema should reject additional top-level fields"

    def test_attestation_proof_rejects_extra_fields(self):
        schema = _load_schema("attestation.schema.json")
        instance = {
            "attestation_type": "compliance",
            "issuer": "did:key:test",
            "subject": "asset-001",
            "issued_at": "2026-01-01T00:00:00Z",
            "claims": {"status": "compliant"},
            "proof": {
                "type": "Ed25519Signature2020",
                "verification_method": "did:key:test#key-1",
                "signature": "abc123",
                "INJECTED_PROOF_FIELD": "malicious"
            }
        }
        errors = _validate(schema, instance)
        assert any("INJECTED_PROOF_FIELD" in str(e) for e in errors), \
            "Attestation proof should reject additional fields"


class TestCorridorReceiptHardening:
    """Corridor receipt must reject extra top-level fields."""

    def test_corridor_receipt_additionalProperties_false(self):
        schema = _load_schema("corridor.receipt.schema.json")
        assert schema.get("additionalProperties") is False, \
            "corridor.receipt.schema.json must have additionalProperties: false at top level"


class TestCorridorCheckpointHardening:
    """Corridor checkpoint must reject extra top-level fields."""

    def test_corridor_checkpoint_additionalProperties_false(self):
        schema = _load_schema("corridor.checkpoint.schema.json")
        assert schema.get("additionalProperties") is False, \
            "corridor.checkpoint.schema.json must have additionalProperties: false at top level"


class TestCorridorForkResolutionHardening:
    """Corridor fork-resolution must reject extra top-level fields."""

    def test_corridor_fork_resolution_additionalProperties_false(self):
        schema = _load_schema("corridor.fork-resolution.schema.json")
        assert schema.get("additionalProperties") is False, \
            "corridor.fork-resolution.schema.json must have additionalProperties: false at top level"


class TestSmartAssetRegistryVCHardening:
    """Smart Asset Registry VC must reject extra top-level fields."""

    def test_vc_smart_asset_registry_additionalProperties_false(self):
        schema = _load_schema("vc.smart-asset-registry.schema.json")
        assert schema.get("additionalProperties") is False, \
            "vc.smart-asset-registry.schema.json must have additionalProperties: false at top level"
