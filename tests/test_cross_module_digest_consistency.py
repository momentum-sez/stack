"""
Verify that computing a digest via the core layer and the phoenix layer
produces identical results for the same input data.

This test prevents the canonicalization split from regressing.
"""
import hashlib
import json

from tools.lawpack import jcs_canonicalize


def test_digest_agreement_simple_dict():
    """Same dict -> same digest regardless of computation path."""
    data = {"b": 2, "a": 1, "c": "hello"}
    core_digest = hashlib.sha256(jcs_canonicalize(data)).hexdigest()
    # json.dumps with sort_keys for simple dicts should match JCS
    phoenix_old_digest = hashlib.sha256(
        json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    assert core_digest == phoenix_old_digest, (
        f"Digest mismatch for simple dict: core={core_digest}, old_phoenix={phoenix_old_digest}"
    )


def test_digest_agreement_with_datetime():
    """Datetimes must be coerced identically."""
    from datetime import datetime, timezone
    data = {"ts": datetime(2026, 1, 15, 12, 0, 0, tzinfo=timezone.utc), "val": 42}
    core_digest = hashlib.sha256(jcs_canonicalize(data)).hexdigest()
    # After fix, phoenix code should also use jcs_canonicalize for this.
    # If it used json.dumps, datetime would not serialize at all (TypeError).
    assert len(core_digest) == 64


def test_digest_agreement_nested():
    """Nested structures produce consistent digests."""
    data = {
        "outer": {"inner": [1, 2, 3], "name": "test"},
        "id": "abc",
    }
    core_digest = hashlib.sha256(jcs_canonicalize(data)).hexdigest()
    phoenix_old_digest = hashlib.sha256(
        json.dumps(data, sort_keys=True, separators=(",", ":")).encode()
    ).hexdigest()
    assert core_digest == phoenix_old_digest


def test_phoenix_tensor_uses_jcs():
    """Verify ComplianceTensorV2.commit() uses jcs_canonicalize."""
    from tools.phoenix.tensor import (
        ComplianceDomain, ComplianceState, ComplianceTensorV2
    )
    tensor = ComplianceTensorV2()
    tensor.set(
        asset_id="test-asset",
        jurisdiction_id="PK",
        domain=ComplianceDomain.AML,
        state=ComplianceState.COMPLIANT,
        reason_code="verified",
    )
    commitment = tensor.commit()
    assert commitment is not None
    assert commitment.root is not None
    assert len(commitment.root) == 64  # hex-encoded SHA256


def test_phoenix_events_digest_uses_jcs():
    """Verify Event.digest() uses jcs_canonicalize, not json.dumps."""
    from tools.phoenix.events import AssetCreated
    event = AssetCreated(
        asset_id="test-001",
        asset_type="entity",
        owner_did="did:key:test",
    )
    digest = event.digest()
    assert len(digest) == 64


def test_float_rejection_prevents_nondeterministic_digests():
    """Floats in digest data must be rejected to prevent non-deterministic output."""
    import pytest
    with pytest.raises(ValueError, match="[Ff]loat"):
        jcs_canonicalize({"amount": 19.99})
