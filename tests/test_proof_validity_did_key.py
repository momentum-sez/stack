import copy

import pytest

from tools.vc import (
    add_ed25519_proof,
    canonicalize_json,
    generate_ed25519_jwk,
    load_ed25519_private_key_from_jwk,
    signing_input,
    verify_credential,
)


def _fresh_signed_object() -> dict:
    jwk = generate_ed25519_jwk()
    priv, did = load_ed25519_private_key_from_jwk(jwk)
    obj = {"id": "urn:example:obj:1", "hello": "world"}
    signed = add_ed25519_proof(obj, priv, verification_method=f"{did}#key-1")
    return signed


def test_verify_credential_rejects_unsupported_proof_type_even_if_signature_matches():
    signed = _fresh_signed_object()
    ok = verify_credential(signed)
    assert any(r.ok for r in ok)

    bad = copy.deepcopy(signed)
    bad["proof"]["type"] = "Ed25519Signature2020"
    res = verify_credential(bad)
    assert len(res) == 1
    assert res[0].ok is False
    assert "Unsupported proof.type" in (res[0].error or "")


def test_verify_credential_rejects_non_did_key_verification_method():
    signed = _fresh_signed_object()

    bad = copy.deepcopy(signed)
    bad["proof"]["verificationMethod"] = "did:example:123#key-1"

    res = verify_credential(bad)
    assert len(res) == 1
    assert res[0].ok is False
    assert "only did:key" in ((res[0].error or "").lower())


def test_signing_input_and_canonicalize_json_reject_floats_for_determinism():
    with pytest.raises(ValueError):
        canonicalize_json({"x": 1.0})

    with pytest.raises(ValueError):
        signing_input({"x": 1.0})


def test_verify_credential_rejects_truncated_signature():
    signed = _fresh_signed_object()

    bad = copy.deepcopy(signed)
    bad["proof"]["jws"] = bad["proof"]["jws"][:-2]

    res = verify_credential(bad)
    assert len(res) == 1
    assert res[0].ok is False
    # Either we fail b64 decoding or we fail the expected signature length check.
    assert (
        "Invalid base64url" in (res[0].error or "")
        or "must be 64 bytes" in (res[0].error or "")
    )
