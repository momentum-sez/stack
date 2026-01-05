#!/usr/bin/env python3
"""
Minimal Verifiable Credential (VC) helpers for the MSEZ stack.

Design goals:
- No network dependency for verification (supports did:key)
- Deterministic signing input (JCS-style canonical JSON: sorted keys, no whitespace)
- Supports multiple proofs (multi-party co-signing) by signing the credential payload *excluding* the proof field

This is intentionally a *minimal* profile:
- It is compatible with the W3C VC JSON shape (context/type/issuer/issuanceDate/credentialSubject/proof),
  but does not attempt full JSON-LD processing.
"""
from __future__ import annotations

import base64
import json
import hashlib
import pathlib
from dataclasses import dataclass
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple, Union

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey, Ed25519PublicKey
from cryptography.hazmat.primitives import serialization


B58_ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
_B58_INDEX = {c: i for i, c in enumerate(B58_ALPHABET)}


def b64url_encode(b: bytes) -> str:
    return base64.urlsafe_b64encode(b).rstrip(b"=").decode("ascii")


def b64url_decode(s: str) -> bytes:
    pad = "=" * ((4 - (len(s) % 4)) % 4)
    return base64.urlsafe_b64decode((s + pad).encode("ascii"))


def b58encode(b: bytes) -> str:
    n = int.from_bytes(b, "big")
    res = ""
    while n > 0:
        n, rem = divmod(n, 58)
        res = B58_ALPHABET[rem] + res

    # preserve leading zeros
    pad = 0
    for byte in b:
        if byte == 0:
            pad += 1
        else:
            break
    return ("1" * pad) + (res or "")


def b58decode(s: str) -> bytes:
    n = 0
    for ch in s:
        n = (n * 58) + _B58_INDEX[ch]
    full = n.to_bytes((n.bit_length() + 7) // 8, "big") if n > 0 else b""
    pad = 0
    for ch in s:
        if ch == "1":
            pad += 1
        else:
            break
    return (b"\x00" * pad) + full


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def canonicalize_json(obj: Any) -> bytes:
    """
    Canonicalize as bytes for signing:
    - UTF-8
    - sorted keys
    - no whitespace

    NOTE: This is not a full RFC 8785 implementation for floating-point edge cases.
    MSEZ credential subjects SHOULD avoid floats; use strings for amounts/limits.
    """
    return json.dumps(obj, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def signing_input(credential: Dict[str, Any]) -> bytes:
    """Return canonical bytes over the credential payload, excluding `proof`."""
    payload = {k: v for k, v in credential.items() if k != "proof"}
    return canonicalize_json(payload)


def did_key_from_ed25519_public_key(pubkey_bytes: bytes) -> str:
    """
    did:key for Ed25519 public keys:
    - multicodec prefix: 0xED 0x01
    - multibase base58btc, prefix 'z'
    """
    mc = b"\xed\x01" + pubkey_bytes
    return "did:key:z" + b58encode(mc)


def ed25519_public_key_from_did_key(did: str) -> Ed25519PublicKey:
    if not did.startswith("did:key:"):
        raise ValueError("Only did:key is supported for offline verification")
    mb = did.split("did:key:", 1)[1]
    if not mb.startswith("z"):
        raise ValueError("did:key multibase must be base58btc (z...)")
    raw = b58decode(mb[1:])
    if len(raw) < 2 or raw[0:2] != b"\xed\x01":
        raise ValueError("did:key multicodec prefix is not Ed25519 (0xED01)")
    pub = raw[2:]
    if len(pub) != 32:
        raise ValueError("Ed25519 public key must be 32 bytes")
    return Ed25519PublicKey.from_public_bytes(pub)


def normalize_verification_method(vm: str) -> str:
    """Strip fragment for key resolution (did:key:z...#... -> did:key:z...)."""
    return vm.split("#", 1)[0]



def base_did(did_or_vm: str) -> str:
    """Return the base DID (strip any fragment).

    This helper is used across the stack when comparing issuer DIDs and
    verificationMethod values.
    """
    return str(did_or_vm or "").split("#", 1)[0]

def now_rfc3339() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


@dataclass
class ProofResult:
    verification_method: str
    ok: bool
    error: str = ""


def _proofs_as_list(proof: Any) -> List[Dict[str, Any]]:
    if proof is None:
        return []
    if isinstance(proof, list):
        return [p for p in proof if isinstance(p, dict)]
    if isinstance(proof, dict):
        return [proof]
    return []


def verify_credential(credential: Dict[str, Any]) -> List[ProofResult]:
    """
    Verify all proofs in the credential.
    Returns a list of ProofResult objects (one per proof).
    """
    msg = signing_input(credential)
    results: List[ProofResult] = []
    for p in _proofs_as_list(credential.get("proof")):
        vm = str(p.get("verificationMethod") or "")
        try:
            did = normalize_verification_method(vm)
            pub = ed25519_public_key_from_did_key(did)
            sig = b64url_decode(str(p.get("jws") or ""))
            pub.verify(sig, msg)
            results.append(ProofResult(verification_method=vm, ok=True))
        except Exception as ex:
            results.append(ProofResult(verification_method=vm, ok=False, error=str(ex)))
    return results


def add_ed25519_proof(
    credential: Dict[str, Any],
    private_key: Ed25519PrivateKey,
    verification_method: str,
    proof_purpose: str = "assertionMethod",
    created: Optional[str] = None,
) -> Dict[str, Any]:
    """
    Add a new Ed25519 proof to the credential, without modifying the credential payload
    (signing input excludes `proof`, enabling multi-party co-signing).
    """
    created = created or now_rfc3339()
    sig = private_key.sign(signing_input(credential))
    proof_obj = {
        "type": "MsezEd25519Signature2025",
        "created": created,
        "verificationMethod": verification_method,
        "proofPurpose": proof_purpose,
        "jws": b64url_encode(sig),
    }

    existing = credential.get("proof")
    if existing is None:
        credential["proof"] = [proof_obj]
    elif isinstance(existing, list):
        credential["proof"].append(proof_obj)
    elif isinstance(existing, dict):
        credential["proof"] = [existing, proof_obj]
    else:
        credential["proof"] = [proof_obj]
    return credential


def load_ed25519_private_key_from_jwk(jwk: Dict[str, Any]) -> Tuple[Ed25519PrivateKey, str]:
    """
    Load an Ed25519 private key from an OKP JWK.
    Returns (private_key, did:key) derived from the public key.
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


def load_proof_keypair(path: Union[str, pathlib.Path], default_kid: str = "key-1") -> Tuple[Ed25519PrivateKey, str]:
    """Load an Ed25519 keypair from a JSON file for use in VC proofs.

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
    """Generate a new Ed25519 OKP JWK keypair (includes private 'd' and public 'x')."""
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
