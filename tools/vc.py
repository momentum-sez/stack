#!/usr/bin/env python3
"""tools.vc

Minimal (but production-oriented) VC-style Ed25519 proofs used across the MSEZ
stack.

Profile / invariants:
- `did:key` identifiers (Ed25519 only)
- Proof object uses a compact JSON form with a raw Ed25519 signature encoded as
  base64url (`jws` field; no JOSE header)
- Signing input is the canonical JSON bytes of the object with `proof` removed.
  Canonicalization MUST be deterministic and consistent across the repo.

This module is intentionally strict about canonical bytes and proof shape; these
invariants are what allow CI to enforce byte-for-byte determinism in later
pipeline stages.
"""

from __future__ import annotations

import base64
import hashlib
import hmac
import json
import os
import pathlib
import re
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from typing import Any, Dict, List, Optional, Tuple, Union

from cryptography.hazmat.primitives import serialization
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey


# Base58 implementation (no external deps)
B58_ALPHABET = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
B58_MAP = {c: i for i, c in enumerate(B58_ALPHABET)}


def b58decode(s: str) -> bytes:
    if isinstance(s, str):
        s_bytes = s.encode("ascii")
    else:
        s_bytes = s
    num = 0
    for c in s_bytes:
        if c not in B58_MAP:
            raise ValueError("Invalid base58 character")
        num = num * 58 + B58_MAP[c]
    # Count leading zeros
    n_pad = 0
    for c in s_bytes:
        if c == B58_ALPHABET[0]:
            n_pad += 1
        else:
            break
    # Convert to bytes
    full = num.to_bytes((num.bit_length() + 7) // 8, "big") if num else b""
    return b"\x00" * n_pad + full


def b58encode(b: bytes) -> str:
    # Count leading zeros
    n_pad = 0
    for c in b:
        if c == 0:
            n_pad += 1
        else:
            break
    num = int.from_bytes(b, "big")
    out = bytearray()
    while num > 0:
        num, rem = divmod(num, 58)
        out.append(B58_ALPHABET[rem])
    out.extend(B58_ALPHABET[0] for _ in range(n_pad))
    out.reverse()
    return out.decode("ascii")


# ---------------------------------------------------------------------------
# Canonical bytes
# ---------------------------------------------------------------------------


def canonicalize_json(obj: Any) -> bytes:
    """Return canonical JSON bytes for hashing/signing.

    We share canonicalization rules with `tools.lawpack.jcs_canonicalize` to keep
    *one* repo-wide definition for byte-level determinism.

    Properties:
    - keys sorted
    - no insignificant whitespace
    - UTF-8
    - rejects floats (require decimal-as-string for monetary values)
    - coerces datetimes to RFC3339 strings
    """

    # Local import keeps module import lighter for callers that only need DID helpers.
    from tools.lawpack import jcs_canonicalize  # type: ignore

    return jcs_canonicalize(obj)


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def b64url_encode(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode("ascii")


def b64url_decode(s: str) -> bytes:
    pad = "=" * ((4 - len(s) % 4) % 4)
    return base64.urlsafe_b64decode((s + pad).encode("ascii"))


def signing_input(credential: Dict[str, Any]) -> bytes:
    """Canonical signing input for MSEZ proofs.

    The signing input excludes the `proof` field to enable multi-party co-signing.
    """

    signing_obj = {k: v for k, v in credential.items() if k != "proof"}
    return canonicalize_json(signing_obj)


# ---------------------------------------------------------------------------
# did:key (Ed25519)
# ---------------------------------------------------------------------------


def did_key_from_ed25519_public_key(pub: bytes) -> str:
    # multicodec 0xed01 + 32-byte pubkey (ed25519-pub)
    prefixed = bytes([0xED, 0x01]) + pub
    return "did:key:z" + b58encode(prefixed)


def ed25519_public_key_from_did_key(did: str) -> Ed25519PublicKey:
    """Parse a `did:key` (Ed25519) and return a cryptography public key."""

    if not did.startswith("did:key:z"):
        raise ValueError("Only did:key:z... supported")
    z = did[len("did:key:") :]
    if not z.startswith("z"):
        raise ValueError("did:key must be multibase base58btc (z...)")

    decoded = b58decode(z[1:])

    # Expect multicodec varint prefix for ed25519-pub (0xed01)
    if decoded.startswith(bytes([0xED, 0x01])):
        raw = decoded[2:]
    elif decoded.startswith(bytes([0xED])):
        # Compatibility path for malformed encodings that drop the varint continuation byte.
        raw = decoded[1:]
    else:
        raise ValueError("did:key multicodec prefix not recognized for Ed25519")

    if len(raw) != 32:
        raise ValueError(f"Ed25519 public key must be 32 bytes, got {len(raw)}")

    return Ed25519PublicKey.from_public_bytes(raw)


def normalize_verification_method(vm: str) -> str:
    """Strip fragment for key resolution (did:key:z...#... -> did:key:z...)."""

    return vm.split("#", 1)[0]


def base_did(did_or_vm: str) -> str:
    """Return base DID (strip fragment)."""

    return str(did_or_vm or "").split("#", 1)[0]


# ---------------------------------------------------------------------------
# Proof creation + verification
# ---------------------------------------------------------------------------

_PROOF_TYPE = "MsezEd25519Signature2025"
_ALLOWED_PROOF_PURPOSES = {"assertionMethod"}

_RFC3339_Z_RE = re.compile(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$")
_B64URL_RE = re.compile(r"^[A-Za-z0-9_-]*$")


def now_rfc3339() -> str:
    """RFC3339 timestamp used for `proof.created`.

    For deterministic builds (CI / reproducible playbooks), set `SOURCE_DATE_EPOCH`
    (seconds since Unix epoch). When unset, uses the current wall clock.
    """

    sde = os.environ.get("SOURCE_DATE_EPOCH")
    if sde is not None and str(sde).strip() != "":
        try:
            epoch = int(str(sde).strip(), 10)
        except ValueError as ex:
            raise ValueError("SOURCE_DATE_EPOCH must be an integer (seconds)") from ex
        dt = datetime.fromtimestamp(epoch, tz=timezone.utc)
    else:
        dt = datetime.now(timezone.utc)
    return dt.replace(microsecond=0).isoformat().replace("+00:00", "Z")


@dataclass
class ProofResult:
    verification_method: str
    ok: bool
    error: str = ""


def _proofs_as_list(proof: Any) -> List[Any]:
    """Normalize `proof` into a list without silently dropping malformed entries.

    CI should be able to detect *any* non-conformant proof objects, so we
    preserve items and validate them downstream.
    """

    if proof is None:
        return []
    if isinstance(proof, list):
        return list(proof)
    return [proof]


def _validate_proof_object(p: Dict[str, Any]) -> None:
    """Validate proof shape + `did:key` invariants.

    This does **not** verify the signature; it validates the proof fields so that
    we can produce tighter error messages and enforce invariants in CI.
    """

    # Bug #77: Use constant-time comparison for proof type to prevent timing attacks
    t = p.get("type")
    if not isinstance(t, str) or not hmac.compare_digest(t, _PROOF_TYPE):
        raise ValueError(f"Unsupported proof.type: {t!r} (expected {_PROOF_TYPE})")

    created = p.get("created")
    if not isinstance(created, str) or not _RFC3339_Z_RE.match(created):
        raise ValueError(
            "proof.created must be RFC3339 (seconds, Z) like '2025-01-01T00:00:00Z'"
        )

    vm = p.get("verificationMethod")
    if not isinstance(vm, str) or not vm.strip():
        raise ValueError("proof.verificationMethod must be a non-empty string")
    if not vm.startswith("did:key:"):
        raise ValueError("Only did:key verificationMethod is supported")

    pp = p.get("proofPurpose")
    if not isinstance(pp, str) or pp not in _ALLOWED_PROOF_PURPOSES:
        raise ValueError(f"Unsupported proof.proofPurpose: {pp!r}")

    jws = p.get("jws")
    if not isinstance(jws, str) or not jws:
        raise ValueError("proof.jws must be a non-empty base64url string")
    if not _B64URL_RE.match(jws):
        raise ValueError("proof.jws must be base64url (A-Z a-z 0-9 _ -) with no padding")


def verify_credential(credential: Dict[str, Any]) -> List[ProofResult]:
    """Verify all proofs in the object.

    Returns a ProofResult per proof.

    Notes:
    - Signature is verified against the canonical signing input (JCS bytes) of
      the object with `proof` removed.
    - Proofs must use `did:key` (Ed25519) and the repo's proof profile.
    """

    msg = signing_input(credential)
    results: List[ProofResult] = []
    for p in _proofs_as_list(credential.get("proof")):
        vm = str(p.get("verificationMethod") or "")
        try:
            _validate_proof_object(p)

            did = normalize_verification_method(vm)
            pub = ed25519_public_key_from_did_key(did)

            sig = b64url_decode(str(p.get("jws") or ""))
            if len(sig) != 64:
                raise ValueError(f"Ed25519 signature must be 64 bytes, got {len(sig)}")

            pub.verify(sig, msg)
            results.append(ProofResult(verification_method=vm, ok=True))
        except Exception as ex:
            results.append(ProofResult(verification_method=vm, ok=False, error=str(ex)))
    return results


def validate_credential(credential: Dict[str, Any]) -> List[str]:
    """Validate credential structure including date checks.

    Returns a list of error messages (empty list means valid).

    Checks:
    - Bug #78: issuanceDate must not be in the future
    - Bug #79: expirationDate is handled explicitly (missing = never expires)
    """
    errors: List[str] = []
    now = datetime.now(timezone.utc)

    # Bug #78: Reject VCs with issuance date in the future
    issuance_date = credential.get("issuanceDate")
    if issuance_date is not None:
        try:
            normalized = str(issuance_date).replace("Z", "+00:00")
            idt = datetime.fromisoformat(normalized)
            if idt.tzinfo is None:
                idt = idt.replace(tzinfo=timezone.utc)
            if idt > now + timedelta(seconds=60):  # 60s clock skew allowance
                errors.append(
                    f"issuanceDate is in the future: {issuance_date}"
                )
        except (ValueError, TypeError) as e:
            errors.append(f"Invalid issuanceDate: {e}")

    # Bug #79: Handle expirationDate explicitly
    expiration_date = credential.get("expirationDate")
    if expiration_date is not None:
        try:
            normalized = str(expiration_date).replace("Z", "+00:00")
            edt = datetime.fromisoformat(normalized)
            if edt.tzinfo is None:
                edt = edt.replace(tzinfo=timezone.utc)
            if edt < now:
                errors.append(
                    f"Credential has expired: {expiration_date}"
                )
        except (ValueError, TypeError) as e:
            errors.append(f"Invalid expirationDate: {e}")
    # When expirationDate is absent, the credential never expires.
    # Callers should apply their own policy for credentials without expiration.

    return errors


def add_ed25519_proof(
    credential: Dict[str, Any],
    private_key: Ed25519PrivateKey,
    verification_method: str,
    proof_purpose: str = "assertionMethod",
    created: Optional[str] = None,
) -> Dict[str, Any]:
    """Add a new Ed25519 proof to the object.

    The payload is *not* modified (signing input excludes `proof`), which enables
    multi-party co-signing.

    `created` defaults to `now_rfc3339()` which respects SOURCE_DATE_EPOCH.
    """

    created = created or now_rfc3339()
    sig = private_key.sign(signing_input(credential))

    proof_obj = {
        "type": _PROOF_TYPE,
        "created": created,
        "verificationMethod": verification_method,
        "proofPurpose": proof_purpose,
        "jws": b64url_encode(sig),
    }

    # Validate before mutating the object (tight feedback for callers).
    _validate_proof_object(proof_obj)

    existing = credential.get("proof")
    if existing is None:
        credential["proof"] = proof_obj
    elif isinstance(existing, list):
        credential["proof"].append(proof_obj)
    elif isinstance(existing, dict):
        credential["proof"] = [existing, proof_obj]
    else:
        credential["proof"] = [proof_obj]
    return credential


def load_ed25519_private_key_from_jwk(jwk: Dict[str, Any]) -> Tuple[Ed25519PrivateKey, str]:
    """Load an Ed25519 private key from an OKP JWK.

    Returns:
        (private_key, did:key) derived from the public key bytes in the JWK
        (the `x` member).
    """

    if jwk.get("kty") != "OKP" or jwk.get("crv") != "Ed25519":
        raise ValueError("Only OKP/Ed25519 JWK is supported")

    d = jwk.get("d")
    x = jwk.get("x")
    if not d or not x:
        raise ValueError("JWK must include both 'd' (private) and 'x' (public)")

    priv_bytes = b64url_decode(d)
    pub_bytes = b64url_decode(x)
    priv = Ed25519PrivateKey.from_private_bytes(priv_bytes)
    did = did_key_from_ed25519_public_key(pub_bytes)
    return priv, did

def load_proof_keypair(
    path: Union[str, pathlib.Path],
    default_kid: str = "key-1",
) -> Tuple[Ed25519PrivateKey, str]:
    """Load an Ed25519 keypair from a JSON file for use in proofs.

    Accepted file shapes:

    1) A private OKP JWK (Ed25519):

      {"kty":"OKP","crv":"Ed25519","x":"...","d":"...","kid":"key-1"}

    2) A wrapper object:

      {"private_jwk": <jwk>, "verificationMethod": "did:key:...#key-1"}

    Returns:
      (private_key, verification_method)
    """

    p = pathlib.Path(path)
    key_obj = json.loads(p.read_text(encoding="utf-8"))

    vm = ""
    jwk: Dict[str, Any]
    if isinstance(key_obj, dict) and ("private_jwk" in key_obj or "jwk" in key_obj):
        inner = key_obj.get("private_jwk") or key_obj.get("jwk")
        if not isinstance(inner, dict):
            raise ValueError("key file wrapper must contain a JWK object under 'private_jwk' or 'jwk'")
        jwk = inner
        vm = str(
            key_obj.get("verificationMethod")
            or key_obj.get("verification_method")
            or key_obj.get("vm")
            or ""
        ).strip()
    elif isinstance(key_obj, dict):
        jwk = key_obj
    else:
        raise ValueError("key file must be a JSON object")

    priv, did = load_ed25519_private_key_from_jwk(jwk)
    kid = str(jwk.get("kid") or default_kid).strip() or default_kid

    if not vm:
        vm = f"{did}#{kid}"
    return priv, vm


def public_jwk_from_private_jwk(jwk: Dict[str, Any]) -> Dict[str, Any]:
    """Return a public-only JWK (no 'd')."""

    out = dict(jwk)
    out.pop("d", None)
    return out


def generate_ed25519_jwk(kid: str = "key-1") -> Dict[str, Any]:
    """Generate a new Ed25519 OKP JWK keypair."""

    priv = Ed25519PrivateKey.generate()

    priv_bytes = priv.private_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PrivateFormat.Raw,
        encryption_algorithm=serialization.NoEncryption(),
    )
    pub_bytes = priv.public_key().public_bytes(
        encoding=serialization.Encoding.Raw,
        format=serialization.PublicFormat.Raw,
    )

    jwk = {
        "kty": "OKP",
        "crv": "Ed25519",
        "x": b64url_encode(pub_bytes),
        "d": b64url_encode(priv_bytes),
        "kid": kid,
    }
    return jwk
