#!/usr/bin/env python3
"""MSEZ Stack tool (reference implementation) — v0.4.23

Capabilities:
- validate modules/profiles/zones against schemas
- validate Akoma Ntoso against XSD (when schemas present)
- fetch Akoma schemas
- render Akoma to HTML/PDF
- generate deterministic stack.lock from zone.yaml
- build a composed bundle directory
- check policy-to-code coverage for MUST/SHALL clauses
- diff two lockfiles for upgrade impact analysis
- publish rendered artifacts (Akoma -> HTML/PDF) for distribution
- sign/verify Verifiable Credentials (VC) for corridor integrity
- verify corridor cryptographic bindings (manifest + security artifacts + VC)
- corridor state channels (genesis root + signed receipts + state root verification)

This tool is a **reference implementation**. Production implementations may differ while still conforming to the spec.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import uuid
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

import yaml
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012
from lxml import etree

# Common helpers used across CLI subcommands. Imported at module load so that
# individual commands can remain small and tests can import `cmd_*` functions
# without tripping `NameError` on shared utilities.
from tools.vc import now_rfc3339  # type: ignore
REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]

# Ensure imports like `tools.akoma.render` work even when this file is executed
# as a script (sys.path[0] becomes the tools/ directory).
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

# Local (repo) imports (after sys.path fix)
from tools import artifacts as artifact_cas
STACK_SPEC_VERSION = "0.4.23"

# Templating: we intentionally support two simple placeholder syntaxes used in v0.4 (safe placeholder subset)
# - {{ VAR_NAME }} (common in Akoma templates)
# - ${var_name}    (common in YAML corridor manifests)
RE_JINJA_LITE = re.compile(r"\{\{\s*([A-Za-z0-9_]+)\s*\}\}")
RE_DOLLAR = re.compile(r"\$\{\s*([A-Za-z0-9_]+)\s*\}")

def load_yaml(path: pathlib.Path) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))

def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()

def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())



def coerce_corridor_module_dir(path: pathlib.Path) -> pathlib.Path:
    """Coerce a corridor module directory from either a module dir or a corridor.yaml path.

    Many CLI commands accept either:
      - a corridor module directory (containing corridor.yaml), or
      - a path directly to a corridor.yaml file.

    This helper also tries a REPO_ROOT-relative resolution when given a relative path
    that does not exist from the current working directory.
    """
    cand = pathlib.Path(str(path))
    candidates = [cand]
    if not cand.is_absolute():
        candidates.append(REPO_ROOT / cand)

    for pth in candidates:
        try:
            if pth.is_file() and pth.name == 'corridor.yaml':
                return pth.parent
            if pth.is_dir() and (pth / 'corridor.yaml').exists():
                return pth
        except Exception:
            continue

    raise FileNotFoundError(f'Corridor module not found or missing corridor.yaml: {path}')


def load_trust_anchors(path: pathlib.Path) -> List[Dict[str, Any]]:
    """Load trust anchors YAML and return a normalized list.

    The canonical trust-anchors.yaml shape is:
      version: 1
      trust_anchors:
        - anchor_id: ...
          type: did
          identifier: did:key:...#key-1
          allowed_attestations: [...]

    For receipt enforcement we normalize to:
      {"did": "did:key:...", "allowed_attestations": [...], "raw": {..}}

    This helper is best-effort: schema validation is performed elsewhere.
    """
    pth = pathlib.Path(path)
    if not pth.is_absolute() and not pth.exists():
        pth = REPO_ROOT / pth

    try:
        obj = load_yaml(pth)
    except Exception:
        return []

    anchors = []
    if isinstance(obj, dict):
        anchors = obj.get('trust_anchors') or []

    if not isinstance(anchors, list):
        return []

    out: List[Dict[str, Any]] = []
    for a in anchors:
        if not isinstance(a, dict):
            continue
        ident = str(a.get('identifier') or a.get('did') or '').strip()
        did = ident.split('#', 1)[0].strip()
        if not did:
            continue
        allowed = a.get('allowed_attestations') or []
        if not isinstance(allowed, list):
            allowed = []
        out.append({
            'did': did,
            'allowed_attestations': [str(x) for x in allowed if str(x).strip()],
            'raw': a,
        })

    return out


def parse_duration_to_seconds(s: str) -> int:
    """Parse a small subset of duration syntaxes into seconds.

    Supported:
    - shorthand:  "30s", "15m", "2h", "7d"
    - ISO8601-ish: "PT1H", "PT30M", "P1D" (minimal subset)

    Returns 0 when the input is empty or unparseable.
    """
    raw = str(s or "").strip()
    if not raw:
        return 0

    # Shorthand forms.
    if re.fullmatch(r"\d+", raw):
        try:
            return int(raw)
        except Exception:
            return 0
    m = re.fullmatch(r"(?i)\s*(\d+)\s*([smhd])\s*", raw)
    if m:
        n = int(m.group(1))
        unit = m.group(2).lower()
        if unit == "s":
            return n
        if unit == "m":
            return n * 60
        if unit == "h":
            return n * 3600
        if unit == "d":
            return n * 86400

    # ISO8601-ish subset.
    # PnD
    m = re.fullmatch(r"(?i)\s*P(\d+)D\s*", raw)
    if m:
        return int(m.group(1)) * 86400
    # PTnH, PTnM, PTnS, or combinations like PT1H30M
    m = re.fullmatch(r"(?i)\s*PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?\s*", raw)
    if m:
        h = int(m.group(1) or 0)
        mi = int(m.group(2) or 0)
        sec = int(m.group(3) or 0)
        return (h * 3600) + (mi * 60) + sec

    return 0


def corridor_head_commitment_digest(
    *,
    corridor_id: str,
    genesis_root: str,
    receipt_count: int,
    final_state_root: str,
    mmr_root: str,
) -> str:
    """Compute a deterministic digest for a corridor head.

    This digest is designed for:
    - gossip-layer dedupe of identical heads even when auxiliary fields (like checkpoint timestamps)
      differ across checkpoints,
    - cheap cross-watcher comparison without transporting receipts.

    Digest is SHA256(JCS(head_commitment)).
    """
    try:
        from tools.lawpack import jcs_canonicalize  # type: ignore
    except Exception:
        # Fallback to local canonicalization rule.
        from tools.vc import canonicalize_json as jcs_canonicalize  # type: ignore

    head = {
        "corridor_id": str(corridor_id or "").strip(),
        "genesis_root": str(genesis_root or "").strip().lower(),
        "receipt_count": int(receipt_count or 0),
        "final_state_root": str(final_state_root or "").strip().lower(),
        "mmr_root": str(mmr_root or "").strip().lower(),
    }
    return sha256_bytes(jcs_canonicalize(head))

def _coerce_sha256(value: Any) -> str:
    """Coerce either a raw sha256 hex string or an ArtifactRef-like dict into a digest string.

    This enables backward-compatible schemas where a field may be either:
    - a sha256 hex string, or
    - an object containing {digest_sha256: <hex>} (e.g., an ArtifactRef).
    """
    if isinstance(value, dict):
        return str(value.get("digest_sha256") or "").strip().lower()
    if isinstance(value, str):
        return value.strip().lower()
    return ""


def _verified_base_dids(verify_results: Any) -> Set[str]:
    """Extract base DIDs for all successfully-verified proofs.

    The reference verifier (tools.vc.verify_credential) returns a list of ProofResult
    objects. Older/alternate toolchains may return dict-like structures; we accept
    those best-effort to keep the stack resilient.

    Returns:
      A set of base DIDs (did:key:..., did:web:..., etc).
    """
    out: Set[str] = set()
    if not verify_results:
        return out

    # Legacy dict shape: {"verified": [{"verificationMethod": "..."}], ...}
    if isinstance(verify_results, dict):
        for ok in (verify_results.get("verified") or []):
            if not isinstance(ok, dict):
                continue
            vm = str(ok.get("verificationMethod") or ok.get("verification_method") or "")
            did = vm.split("#", 1)[0] if vm else ""
            if did:
                out.add(did)
        return out

    # Preferred list shape: [ProofResult, ...]
    if isinstance(verify_results, list):
        for r in verify_results:
            try:
                if isinstance(r, dict):
                    ok = bool(r.get("ok") or r.get("verified") or r.get("valid"))
                    vm = str(r.get("verificationMethod") or r.get("verification_method") or "")
                else:
                    ok = bool(getattr(r, "ok", False))
                    vm = str(getattr(r, "verification_method", "") or getattr(r, "verificationMethod", "") or "")
                if not ok:
                    continue
                did = vm.split("#", 1)[0] if vm else ""
                if did:
                    out.add(did)
            except Exception:
                continue
    return out


def make_artifact_ref(
    artifact_type: str,
    digest_sha256: str,
    *,
    uri: str = "",
    display_name: str = "",
    media_type: str = "",
    byte_length: int | None = None,
) -> Dict[str, Any]:
    """Construct a minimally-populated ArtifactRef object.

    We use ArtifactRefs as the universal typed digest commitment substrate.
    Keeping this helper centralized makes it harder for different commands
    to drift in how they emit typed references.
    """
    d = str(digest_sha256 or "").strip().lower()
    out: Dict[str, Any] = {
        "artifact_type": str(artifact_type or "").strip(),
        "digest_sha256": d,
    }
    u = str(uri or "").strip()
    if u:
        out["uri"] = u
    dn = str(display_name or "").strip()
    if dn:
        out["display_name"] = dn
    mt = str(media_type or "").strip()
    if mt:
        out["media_type"] = mt
    if byte_length is not None:
        try:
            out["byte_length"] = int(byte_length)
        except Exception:
            pass
    return out



# --- ArtifactRef discovery (transitive completeness) -------------------------


def _iter_embedded_artifactrefs(obj: Any):
    """Yield (artifact_type, digest_sha256) tuples from embedded ArtifactRef-shaped dicts.

    We treat ArtifactRefs as the universal typed commitment substrate. This helper is used
    by stronger verification modes (e.g., --transitive-require-artifacts) to recursively
    require that *all* committed artifacts are resolvable via CAS.

    NOTE: This intentionally only detects typed ArtifactRefs (artifact_type + digest_sha256).
    Untyped digests (legacy) are handled elsewhere as `blob` commitments.
    """

    if isinstance(obj, dict):
        at = obj.get("artifact_type")
        dg = obj.get("digest_sha256")
        if isinstance(at, str) and isinstance(dg, str):
            at_s = at.strip()
            dg_s = dg.strip().lower()
            if at_s and _coerce_sha256(dg_s):
                yield (at_s, dg_s)

        for v in obj.values():
            yield from _iter_embedded_artifactrefs(v)
        return

    if isinstance(obj, list):
        for it in obj:
            yield from _iter_embedded_artifactrefs(it)
        return


def _require_transition_types_lock_transitive(
    registry_digest_sha256: str,
    *,
    errors: List[str],
    label: str,
    repo_root: pathlib.Path = REPO_ROOT,
) -> None:
    """Require that a transition type registry lock snapshot resolves *transitively*.

    When corridor receipts commit to a `transition_type_registry_digest_sha256`, that digest
    acts as a *commitment root* for the transition type universe at that point in time.

    `--require-artifacts` verifies that the registry lock *itself* can be resolved. This helper
    powers the stronger `--transitive-require-artifacts` mode, which additionally requires that
    every schema/ruleset/circuit digest referenced by the lock snapshot is present in artifact
    CAS.

    This makes "commitment completeness" mechanically checkable: a verifier can refuse to accept
    receipts that reference registry snapshots whose transitive dependencies cannot be resolved.
    """

    dd = str(registry_digest_sha256 or "").strip().lower()
    if not dd:
        return

    # 1) Ensure the snapshot artifact exists.
    try:
        lock_path = artifact_cas.resolve_artifact_by_digest(
            "transition-types", dd, repo_root=repo_root
        )
    except FileNotFoundError:
        errors.append(f"missing artifact for {label}: transition-types:{dd}")
        return
    except Exception as e:
        errors.append(f"artifact resolver error for {label}: transition-types:{dd}: {e}")
        return

    # 2) Load and schema-validate the lock (so we don't parse arbitrary JSON shapes).
    try:
        lock_obj = load_json(pathlib.Path(lock_path))
    except Exception as e:
        errors.append(f"failed to load transition-types lock for {label}: {lock_path}: {e}")
        return

    try:
        v = schema_validator(repo_root / "schemas" / "transition-types.lock.schema.json")
        ve = list(v.iter_errors(lock_obj))
        if ve:
            errors.append(f"invalid transition-types.lock schema for {label}: {lock_path}: {ve[0].message}")
            return
    except Exception as e:
        errors.append(f"failed to validate transition-types.lock for {label}: {lock_path}: {e}")
        return

    snapshot = lock_obj.get("snapshot") if isinstance(lock_obj, dict) else None
    types = (snapshot or {}).get("transition_types") if isinstance(snapshot, dict) else None
    if not isinstance(types, list):
        errors.append(f"transition-types.lock missing snapshot.transition_types list: {lock_path}")
        return

    # 3) Require all referenced artifacts (deduplicated).
    seen: Set[Tuple[str, str]] = set()
    for t in types:
        if not isinstance(t, dict):
            continue
        kind = str(t.get("kind") or "").strip()
        for field, expected_type in [
            ("schema_digest_sha256", "schema"),
            ("ruleset_digest_sha256", "ruleset"),
            ("zk_circuit_digest_sha256", "circuit"),
        ]:
            val = t.get(field)
            if not val:
                continue
            dig = _coerce_sha256(val)
            if not dig:
                continue

            atype = expected_type
            if isinstance(val, dict) and val.get("artifact_type"):
                atype = str(val.get("artifact_type") or "").strip() or expected_type
                if atype != expected_type:
                    errors.append(
                        f"transition-types.lock {lock_path} kind={kind}: {field} artifact_type={atype} expected {expected_type}"
                    )
                    continue

            key = (atype, dig)
            if key in seen:
                continue
            seen.add(key)

            try:
                resolved_path = artifact_cas.resolve_artifact_by_digest(atype, dig, repo_root=repo_root)
            except FileNotFoundError:
                errors.append(
                    f"missing artifact referenced by transition-types.lock {lock_path} kind={kind}: {atype}:{dig} ({field})"
                )
                continue
            except Exception as e:
                errors.append(
                    f"artifact resolver error for transition-types.lock {lock_path} kind={kind}: {atype}:{dig} ({field}): {e}"
                )
                continue

            # Optional deeper completeness: if a referenced ruleset artifact embeds ArtifactRefs
            # (e.g., proof keys, circuits, subordinate schemas), require those to resolve as well.
            if atype == "ruleset":
                try:
                    ruleset_obj = load_json(pathlib.Path(resolved_path))
                except Exception:
                    ruleset_obj = None

                if ruleset_obj is not None:
                    nested_seen: set[tuple[str, str]] = set()
                    for at2, d2 in _iter_embedded_artifactrefs(ruleset_obj):
                        key2 = (str(at2), str(d2))
                        if key2 in nested_seen:
                            continue
                        nested_seen.add(key2)
                        try:
                            _ = artifact_cas.resolve_artifact_by_digest(at2, d2, repo_root=repo_root)
                        except FileNotFoundError:
                            errors.append(
                                f"missing artifact referenced transitively by ruleset {resolved_path} (from transition-types.lock {lock_path} kind={kind}): {at2}:{d2}"
                            )
                        except Exception as e:
                            errors.append(
                                f"artifact resolver error for transitive ref in ruleset {resolved_path} (from transition-types.lock {lock_path} kind={kind}): {at2}:{d2}: {e}"
                            )


def load_authority_registry(
    module_dir: pathlib.Path,
    corridor_cfg: Dict[str, Any],
) -> tuple[Dict[str, set[str]], list[str]]:
    """Load + verify an optional Authority Registry VC chain referenced by a corridor module.

    This provides an external signer authorization layer intended to mitigate trust-anchor
    circularity.

    corridor.yaml MAY include an `authority_registry_vc_path` as either:
      - a single VC path (string), OR
      - an ordered chain (array of VC paths) representing hierarchical delegation
        (e.g., treaty body → national authority → zone authority).

    Verifiers SHOULD validate:
      - each registry VC has at least one valid proof
      - the issuer of each *child* registry is authorized by the parent registry for
        the `authority_registry` attestation kind (or wildcard '*').

    Returns:
      (allowed_by_attestation, errors)

    Where allowed_by_attestation maps attestation names (e.g. "corridor_definition") to a
    set of base DIDs authorized for that attestation.

    NOTE: The effective allowlist returned is the *leaf* registry's allowlist. Parent
    registries are used to validate delegation, not to widen the allowlist.
    """

    rel_raw: Any = corridor_cfg.get("authority_registry_vc_path")
    paths: List[str] = []
    if isinstance(rel_raw, str):
        if rel_raw.strip():
            paths = [rel_raw.strip()]
    elif isinstance(rel_raw, list):
        paths = [str(x).strip() for x in rel_raw if isinstance(x, str) and str(x).strip()]

    if not paths:
        return ({}, [])

    errs: list[str] = []

    # Import helpers for DID normalization + VC proof verification.
    # These are optional: if unavailable, we still perform schema validation.
    try:
        from tools.vc import base_did, verify_credential  # type: ignore
    except Exception as ex:  # pragma: no cover
        base_did = lambda s: str(s or "").split("#", 1)[0]  # type: ignore
        verify_credential = None  # type: ignore
        errs.append(f"authority registry: missing verifier: {ex}")

    def _issuer_id(vcj: Dict[str, Any]) -> str:
        iss = vcj.get("issuer")
        if isinstance(iss, str):
            return iss
        if isinstance(iss, dict):
            return str(iss.get("id") or "")
        return ""

    def _allowed_map(vcj: Dict[str, Any]) -> Dict[str, set[str]]:
        """Extract allowed attestations from a registry VC."""
        allowed: Dict[str, set[str]] = {}
        authorities = ((vcj.get("credentialSubject") or {}) if isinstance(vcj.get("credentialSubject"), dict) else {}).get("authorities")
        if not isinstance(authorities, list):
            authorities = []

        for a in authorities:
            if not isinstance(a, dict):
                continue
            did = base_did(a.get("did") or a.get("id") or "")
            if not did:
                continue

            att = a.get("allowed_attestations")
            if isinstance(att, str):
                att_list = [att]
            elif isinstance(att, list):
                att_list = [x for x in att if isinstance(x, str)]
            else:
                att_list = []

            for name in att_list:
                n = str(name).strip()
                if not n:
                    continue
                allowed.setdefault(n, set()).add(did)

            # Support wildcard authorization in registries.
            if any(str(x).strip() == "*" for x in att_list):
                allowed.setdefault("*", set()).add(did)

        return allowed

    # Load configured chain (root → leaf)
    chain: List[Tuple[str, Dict[str, Any]]] = []
    for rel in paths:
        vc_path = module_dir / rel
        if not vc_path.exists():
            errs.append(f"{rel}: authority registry VC not found")
            continue
        try:
            vcj = load_json(vc_path)
            if isinstance(vcj, dict):
                chain.append((rel, vcj))
            else:
                errs.append(f"{rel}: authority registry VC must be a JSON object")
        except Exception as ex:
            errs.append(f"{rel}: failed to parse authority registry VC: {ex}")

    if not chain:
        return ({}, errs)

    # If the leaf VC declares a parent_registry_ref and only one VC was configured,
    # attempt to extend the chain by resolving parents via CAS or local paths.
    if len(chain) == 1:
        try:
            from tools.vc import base_did, verify_credential  # type: ignore
            leaf_rel, leaf_vc = chain[0]
            cur = leaf_vc
            seen: set[str] = set()
            # Bound recursion depth to avoid cycles.
            for _ in range(8):
                cs = cur.get("credentialSubject") if isinstance(cur.get("credentialSubject"), dict) else {}
                pref = (cs or {}).get("parent_registry_ref")
                if not pref:
                    break
                # Resolve parent VC path
                parent_path: Optional[pathlib.Path] = None
                if isinstance(pref, dict):
                    dd = str(pref.get("digest_sha256") or "").strip().lower()
                    if dd:
                        try:
                            parent_path = artifact_cas.resolve_artifact_by_digest("vc", dd, repo_root=REPO_ROOT)
                        except Exception as ex:
                            errs.append(f"{leaf_rel}: failed to resolve parent_registry_ref digest {dd}: {ex}")
                elif isinstance(pref, str):
                    cand = pathlib.Path(pref)
                    if not cand.is_absolute():
                        cand = module_dir / cand
                    if cand.exists():
                        parent_path = cand

                if not parent_path:
                    break
                key = str(parent_path.resolve())
                if key in seen:
                    errs.append(f"{leaf_rel}: parent_registry_ref cycle detected at {parent_path}")
                    break
                seen.add(key)

                parent_vc = load_json(parent_path)
                if isinstance(parent_vc, dict):
                    # Prepend parent
                    chain.insert(0, (str(parent_path.relative_to(module_dir)) if str(parent_path).startswith(str(module_dir)) else str(parent_path), parent_vc))
                    cur = parent_vc
                else:
                    errs.append(f"{leaf_rel}: parent registry VC must be a JSON object")
                    break
        except Exception:
            # Best-effort: do not fail if parent resolution cannot run.
            pass

    # Validate each registry VC (schema + crypto)
    schema = schema_validator(REPO_ROOT / "schemas" / "vc.authority-registry.schema.json")
    try:
        from tools.vc import verify_credential  # type: ignore
    except Exception as ex:
        verify_credential = None  # type: ignore
        errs.append(f"authority registry: missing verifier: {ex}")

    for rel, vc in chain:
        for e in validate_with_schema(vc, schema):
            errs.append(f"{rel}: {e}")
        if verify_credential is not None:
            results = verify_credential(vc)
            if not results or not any(r.ok for r in results):
                errs.append(f"{rel}: authority registry VC has no valid proof")

    # Enforce delegation chaining: issuer(child) must be authorized by parent for authority_registry.
    if len(chain) > 1:
        try:
            from tools.vc import base_did  # type: ignore
        except Exception:
            base_did = lambda s: str(s).split("#", 1)[0]  # type: ignore

        for i in range(1, len(chain)):
            parent_rel, parent_vc = chain[i - 1]
            child_rel, child_vc = chain[i]

            parent_allowed = _allowed_map(parent_vc)
            delegates = set()
            delegates |= parent_allowed.get("authority_registry", set())
            delegates |= parent_allowed.get("*", set())
            if not delegates:
                errs.append(
                    f"{parent_rel}: registry chain cannot be validated (no 'authority_registry' delegates declared); required to validate child {child_rel}"
                )
                continue

            issuer = base_did(_issuer_id(child_vc))
            if issuer not in delegates:
                errs.append(
                    f"{child_rel}: issuer {issuer} is not authorized by parent registry {parent_rel} for 'authority_registry' delegation"
                )

    # Effective allowlist is the leaf registry.
    leaf_rel, leaf_vc = chain[-1]
    allowed = _allowed_map(leaf_vc)
    if not allowed:
        errs.append(f"{leaf_rel}: authority registry contains no allowed attestations")

    return (allowed, errs)


_SCHEMA_REGISTRY: Optional[Registry] = None


def _schema_registry(repo_root: pathlib.Path = REPO_ROOT) -> Registry:
    """Build an in-memory registry of known schemas keyed by $id.

    This enables offline validation of schemas that use $ref across the stack.
    """
    global _SCHEMA_REGISTRY
    if _SCHEMA_REGISTRY is not None:
        return _SCHEMA_REGISTRY

    reg = Registry()
    schemas_dir = repo_root / 'schemas'
    if schemas_dir.exists():
        for sp in sorted(schemas_dir.glob('*.schema.json')):
            try:
                sj = load_json(sp)
            except Exception:
                continue
            sid = sj.get('$id')
            if not sid or not isinstance(sid, str):
                continue
            try:
                reg = reg.with_resource(sid, Resource.from_contents(sj, default_specification=DRAFT202012))
            except Exception:
                # Fallback: accept unknown metaschemas/annotation-only docs
                try:
                    reg = reg.with_resource(sid, Resource.from_contents(sj))
                except Exception:
                    pass

    _SCHEMA_REGISTRY = reg
    return reg


def schema_validator(schema_path: pathlib.Path) -> Draft202012Validator:
    schema = load_json(schema_path)
    return Draft202012Validator(schema, registry=_schema_registry())

def validate_with_schema(obj: Any, validator: Draft202012Validator) -> List[str]:
    errors = []
    for e in sorted(validator.iter_errors(obj), key=str):
        errors.append(f"{list(e.absolute_path)}: {e.message}")
    return errors

def find_modules(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p.parent for p in repo_root.glob("modules/**/module.yaml")]

def find_profiles(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p for p in repo_root.glob("profiles/**/profile.yaml")]

def find_zones(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p for p in repo_root.glob("jurisdictions/**/zone.yaml")]

def build_module_index(repo_root: pathlib.Path) -> Dict[str, Tuple[pathlib.Path, Dict[str, Any]]]:
    """Build an in-memory index of module_id -> (module_dir, manifest)."""
    index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]] = {}
    for mdir in find_modules(repo_root):
        try:
            data = load_yaml(mdir / "module.yaml")
        except Exception:
            continue
        mid = data.get("module_id")
        if mid:
            index[str(mid)] = (mdir, data)
    return index

def build_profile_index(repo_root: pathlib.Path) -> Dict[str, pathlib.Path]:
    """Build an in-memory index of profile_id -> profile.yaml path."""
    idx: Dict[str, pathlib.Path] = {}
    for p in find_profiles(repo_root):
        try:
            prof = load_yaml(p)
        except Exception:
            continue
        pid = prof.get("profile_id")
        if pid:
            idx[str(pid)] = p
    return idx

def build_corridor_index(repo_root: pathlib.Path) -> Dict[str, pathlib.Path]:
    """Index corridor_id -> module_dir for corridor modules."""
    idx: Dict[str, pathlib.Path] = {}
    for mdir in find_modules(repo_root):
        try:
            mdata = load_yaml(mdir / "module.yaml")
        except Exception:
            continue
        if mdata.get("kind") != "corridor":
            continue
        cy = mdir / "corridor.yaml"
        if not cy.exists():
            continue
        try:
            c = load_yaml(cy)
        except Exception:
            continue
        cid = c.get("corridor_id")
        if cid:
            idx[str(cid)] = mdir
    return idx

def validate_module(module_dir: pathlib.Path, validator: Draft202012Validator) -> Tuple[bool, List[str], Dict[str,Any]]:
    manifest_path = module_dir / "module.yaml"
    data = load_yaml(manifest_path)
    errors = validate_with_schema(data, validator)

    # Basic checks for provided artifact paths
    for prov in data.get("provides", []):
        rel = prov.get("path")
        if rel and not (module_dir / rel).exists():
            errors.append(f"provides path missing: {module_dir}/{rel}")
    return (len(errors) == 0, errors, data)

def load_akoma_schema(schema_dir: pathlib.Path) -> etree.XMLSchema | None:
    main_xsd = schema_dir / "akomantoso30.xsd"
    if not main_xsd.exists():
        return None
    try:
        doc = etree.parse(str(main_xsd))
        return etree.XMLSchema(doc)
    except Exception as ex:
        print("WARN: failed to load Akoma schema:", ex)
        return None

def validate_akoma_xml(module_dir: pathlib.Path, schema: etree.XMLSchema | None) -> List[str]:
    errs: List[str] = []
    for xml_file in module_dir.glob("src/akn/**/*.xml"):
        try:
            doc = etree.parse(str(xml_file))
            # Ensure there are stable eId anchors somewhere
            if not doc.xpath('//*[@eId]'):
                errs.append(f"No eId anchors found in {xml_file}")
            if schema is not None and not schema.validate(doc):
                for e in schema.error_log:
                    errs.append(f"{xml_file}: {e.message} (line {e.line})")
        except Exception as ex:
            errs.append(f"XML parse error in {xml_file}: {ex}")
    return errs

def cmd_fetch_akoma(args: argparse.Namespace) -> int:
    from tools.akoma.fetch_schemas import fetch  # type: ignore
    dest = REPO_ROOT / "tools" / "akoma" / "schemas"
    fetch(dest)
    print("Fetched Akoma schemas into", dest)
    return 0

def cmd_render(args: argparse.Namespace) -> int:
    from tools.akoma.render import main as render_main  # type: ignore
    # delegate to render.py
    sys.argv = ["render", args.xml, "--out-dir", args.out_dir] + (["--pdf"] if args.pdf else [])
    return render_main()

def resolve_module_by_id(
    module_id: str,
    module_index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]] | None = None,
) -> Tuple[pathlib.Path, Dict[str, Any]] | None:
    """Resolve a module by module_id.

    Pass a prebuilt index for speed/consistency.
    """
    idx = module_index or build_module_index(REPO_ROOT)
    return idx.get(module_id)

def extract_dep_ids(dep_list: Any) -> List[str]:
    """Extract dependency module IDs from depends_on supporting legacy string and object forms."""
    deps: List[str] = []
    if not dep_list:
        return deps
    if not isinstance(dep_list, list):
        return deps
    for d in dep_list:
        if isinstance(d, str):
            deps.append(d)
        elif isinstance(d, dict) and d.get("module_id"):
            deps.append(str(d["module_id"]))
    return deps

def iter_dep_specs(dep_list: Any) -> List[Tuple[str, str | None]]:
    """Return (module_id, constraint) pairs from a depends_on list."""
    out: List[Tuple[str, str | None]] = []
    if not dep_list or not isinstance(dep_list, list):
        return out
    for d in dep_list:
        if isinstance(d, str):
            out.append((d, None))
        elif isinstance(d, dict):
            mid = d.get("module_id")
            if mid:
                out.append((mid, d.get("constraint")))
    return out

# Minimal SemVer constraint evaluation (supports comma-separated comparisons)
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+].*)?$")

def _semver_tuple(v: str) -> Tuple[int, int, int]:
    m = SEMVER_RE.match(v.strip())
    if not m:
        raise ValueError(f"Invalid semver: {v!r}")
    return int(m.group(1)), int(m.group(2)), int(m.group(3))

def semver_satisfies(version: str, constraint: str) -> bool:
    """Return True if `version` satisfies a constraint string.

    Supported forms:
    - ">=0.1.0"
    - ">=0.1.0,<0.2.0"
    - "==0.2.0" / "=0.2.0"
    """
    v = _semver_tuple(version)
    for raw in [p.strip() for p in constraint.split(",") if p.strip()]:
        op = None
        rhs = None
        for candidate in (">=", "<=", "==", "=", ">", "<"):
            if raw.startswith(candidate):
                op = "==" if candidate == "=" else candidate
                rhs = raw[len(candidate):].strip()
                break
        if op is None or rhs is None:
            raise ValueError(f"Unsupported constraint fragment: {raw!r}")
        r = _semver_tuple(rhs)
        if op == ">=" and not (v >= r):
            return False
        if op == ">" and not (v > r):
            return False
        if op == "<=" and not (v <= r):
            return False
        if op == "<" and not (v < r):
            return False
        if op == "==" and not (v == r):
            return False
    return True

def build_template_context(base: Dict[str, Any]) -> Dict[str, Any]:
    """Return a template context that includes multiple key aliases.

    - original keys (as provided)
    - UPPERCASE aliases for snake_case keys
    """
    ctx: Dict[str, Any] = dict(base)
    for k, v in list(base.items()):
        if isinstance(k, str):
            ctx[k.upper()] = v
    return ctx

def render_text_template(text: str, ctx: Dict[str, Any], strict: bool = False) -> str:
    """Render text by replacing {{VAR}} and ${var} placeholders.

    This intentionally keeps behavior deterministic and dependency-free.
    """
    def repl_jinja(m: re.Match) -> str:
        key = m.group(1)
        if key in ctx:
            return str(ctx[key])
        if strict:
            raise KeyError(f"Missing template variable: {key}")
        return m.group(0)

    def repl_dollar(m: re.Match) -> str:
        key = m.group(1)
        if key in ctx:
            return str(ctx[key])
        # allow ${foo} to fall back to ${FOO} if present
        ukey = key.upper()
        if ukey in ctx:
            return str(ctx[ukey])
        if strict:
            raise KeyError(f"Missing template variable: {key}")
        return m.group(0)

    text = RE_JINJA_LITE.sub(repl_jinja, text)
    text = RE_DOLLAR.sub(repl_dollar, text)
    return text

def render_templates_in_dir(
    root: pathlib.Path,
    ctx: Dict[str, Any],
    strict: bool = False,
    include_globs: Tuple[str, ...] = (
        "**/*.xml",
        "**/*.yaml",
        "**/*.yml",
        "**/*.md",
    ),
) -> List[str]:
    """Render templates in-place under root.

    Returns a list of files modified (relative paths).
    """
    touched: List[str] = []
    for pat in include_globs:
        for p in root.glob(pat):
            if not p.is_file():
                continue
            # skip module manifests (metadata)
            if p.name in {"module.yaml", "profile.yaml", "zone.yaml", "stack.lock"}:
                continue
            try:
                raw = p.read_text(encoding="utf-8")
            except Exception:
                continue
            if "{{" not in raw and "${" not in raw:
                continue
            try:
                rendered = render_text_template(raw, ctx, strict=strict)
            except KeyError as ex:
                raise
            if rendered != raw:
                p.write_text(rendered, encoding="utf-8")
                touched.append(str(p.relative_to(root)))
    return touched

def validate_policy_to_code_map(
    map_path: pathlib.Path,
    validator: Draft202012Validator,
) -> List[str]:
    """Validate a policy-to-code map against its JSON Schema."""
    try:
        data = load_yaml(map_path)
    except Exception as ex:
        return [f"Failed to parse {map_path}: {ex}"]
    return validate_with_schema(data, validator)

def validate_profile_semantics(
    profile: Dict[str, Any],
    module_index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]],
    corridor_index: Dict[str, pathlib.Path],
) -> List[str]:
    errs: List[str] = []
    if profile.get("stack_spec_version") != STACK_SPEC_VERSION:
        errs.append(
            f"profile.stack_spec_version={profile.get('stack_spec_version')} does not match tool STACK_SPEC_VERSION={STACK_SPEC_VERSION}"
        )

    modules = profile.get("modules") or []
    if not isinstance(modules, list):
        return ["profile.modules must be a list"]
    profile_module_ids = {m.get("module_id") for m in modules if isinstance(m, dict)}

    for m in modules:
        if not isinstance(m, dict):
            errs.append("profile.modules contains a non-object entry")
            continue
        mid = m.get("module_id")
        if not mid:
            errs.append("profile.modules entry missing module_id")
            continue
        if mid not in module_index:
            errs.append(f"Missing module referenced by profile: {mid}")
            continue
        mdir, mdata = module_index[mid]
        want_ver = str(m.get("version"))
        have_ver = str(mdata.get("version"))
        if want_ver and want_ver != have_ver:
            errs.append(f"Version pin mismatch for {mid}: profile pins {want_ver}, module manifest is {have_ver}")

        want_variant = str(m.get("variant"))
        variants = mdata.get("variants") or []
        if want_variant and want_variant not in variants:
            errs.append(f"Variant '{want_variant}' not declared by module {mid}. Available: {variants}")

        # dependencies must be explicit in the profile for determinism
        for dep_id, constraint in iter_dep_specs(mdata.get("depends_on")):
            if dep_id not in profile_module_ids:
                errs.append(
                    f"Unresolved dependency: {mid} depends_on {dep_id} (missing from profile)"
                )
                continue
            if constraint:
                dep_ver = mods_by_id.get(dep_id, {}).get("version")
                if dep_ver and not semver_satisfies(dep_ver, constraint):
                    errs.append(
                        f"Dependency constraint not satisfied: {mid} requires {dep_id} {constraint}, but profile pins {dep_ver}"
                    )

    # corridor ids in profile should be resolvable (optional but recommended)
    for cid in profile.get("corridors", []) or []:
        if cid not in corridor_index:
            errs.append(f"Profile references unknown corridor_id: {cid}")
    return errs

def validate_zone_semantics(
    zone: Dict[str, Any],
    profile_index: Dict[str, pathlib.Path],
    corridor_index: Dict[str, pathlib.Path],
) -> List[str]:
    errs: List[str] = []
    prof = zone.get("profile") or {}
    pid = prof.get("profile_id")
    if pid and pid not in profile_index:
        errs.append(f"Zone references unknown profile_id: {pid}")

    for cid in zone.get("corridors", []) or []:
        if cid not in corridor_index:
            errs.append(f"Zone enables unknown corridor_id: {cid}")
    return errs

def cmd_validate(args: argparse.Namespace) -> int:
    module_schema = schema_validator(REPO_ROOT / "schemas" / "module.schema.json")
    profile_schema = schema_validator(REPO_ROOT / "schemas" / "profile.schema.json")
    zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
    corridor_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.schema.json")
    policy_schema = schema_validator(REPO_ROOT / "schemas" / "policy-to-code.schema.json")

    akoma_schema = load_akoma_schema(REPO_ROOT / "tools" / "akoma" / "schemas")

    module_index = build_module_index(REPO_ROOT)
    profile_index = build_profile_index(REPO_ROOT)
    corridor_index = build_corridor_index(REPO_ROOT)

    if args.all_modules:
        ok = True
        for mdir in find_modules(REPO_ROOT):
            m_ok, m_errors, mdata = validate_module(mdir, module_schema)
            m_errors.extend(validate_akoma_xml(mdir, akoma_schema))

            # Validate policy-to-code maps when present
            map_path = mdir / "src" / "policy-to-code" / "map.yaml"
            if map_path.exists():
                for e in validate_policy_to_code_map(map_path, policy_schema):
                    m_errors.append(f"{map_path.relative_to(mdir)}: {e}")

            # Validate corridor manifests for corridor modules
            if mdata.get("kind") == "corridor":
                cy = mdir / "corridor.yaml"
                if not cy.exists():
                    m_errors.append("Missing corridor.yaml")
                else:
                    cdata = load_yaml(cy)
                    c_errs = validate_with_schema(cdata, corridor_schema)
                    for e in c_errs:
                        m_errors.append(f"corridor.yaml: {e}")


                # Corridor VC binding (cryptographically meaningful corridor definitions)
                m_errors.extend(verify_corridor_definition_vc(mdir))
                m_errors.extend(verify_corridor_agreement_vc(mdir))
            if not m_ok or m_errors:
                ok = False
                print(f"\nMODULE FAIL: {mdir}")
                for e in m_errors:
                    print("  -", e)
        if ok:
            print("OK: all modules validate")
            return 0
        return 2

    if args.all_profiles:
        ok = True
        for p in find_profiles(REPO_ROOT):
            profile = load_yaml(p)
            errors = validate_with_schema(profile, profile_schema)
            errors.extend(validate_profile_semantics(profile, module_index, corridor_index))
            if errors:
                ok = False
                print(f"\nPROFILE FAIL: {p}")
                for e in errors:
                    print("  -", e)
        if ok:
            print("OK: all profiles validate")
            return 0
        return 2

    if args.all_zones:
        ok = True
        for z in find_zones(REPO_ROOT):
            zone = load_yaml(z)
            errors = validate_with_schema(zone, zone_schema)
            errors.extend(validate_zone_semantics(zone, profile_index, corridor_index))
            if errors:
                ok = False
                print(f"\nZONE FAIL: {z}")
                for e in errors:
                    print("  -", e)
        if ok:
            print("OK: all zones validate")
            return 0
        return 2

    # Optional: validate a specific zone file
    if getattr(args, "zone", None):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        if not zone_path.exists():
            print(f"ERROR: zone not found: {zone_path}")
            return 2
        zone = load_yaml(zone_path)
        errors = validate_with_schema(zone, zone_schema)
        errors.extend(validate_zone_semantics(zone, profile_index, corridor_index))
        if errors:
            print(f"ZONE FAIL: {zone_path}")
            for e in errors:
                print("  -", e)
            return 2
        print("OK: zone validates")
        return 0

    # Default: validate a single profile path
    profile_path = pathlib.Path(args.profile)
    if not profile_path.is_absolute():
        profile_path = REPO_ROOT / profile_path
    if not profile_path.exists():
        print(f"ERROR: profile not found: {profile_path}")
        return 2

    profile = load_yaml(profile_path)
    errors = validate_with_schema(profile, profile_schema)
    errors.extend(validate_profile_semantics(profile, module_index, corridor_index))
    if errors:
        print("PROFILE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    # Validate referenced modules (schema + Akoma + policy-to-code map)
    ok = True
    for m in profile.get("modules", []):
        if not isinstance(m, dict):
            ok = False
            print("Invalid profile.modules entry (not an object)")
            continue
        mid = m.get("module_id")
        if not mid:
            ok = False
            print("Missing module_id in profile.modules entry")
            continue
        resolved = resolve_module_by_id(mid, module_index)
        if not resolved:
            ok = False
            print(f"Missing module: {mid}")
            continue
        mdir, _ = resolved

        m_ok, m_errors, mdata = validate_module(mdir, module_schema)
        m_errors.extend(validate_akoma_xml(mdir, akoma_schema))

        map_path = mdir / "src" / "policy-to-code" / "map.yaml"
        if map_path.exists():
            for e in validate_policy_to_code_map(map_path, policy_schema):
                m_errors.append(f"{map_path.relative_to(mdir)}: {e}")

        # corridor manifest validation (if corridor module)
        if mdata.get("kind") == "corridor":
            cy = mdir / "corridor.yaml"
            if cy.exists():
                c_errs = validate_with_schema(load_yaml(cy), corridor_schema)
                for e in c_errs:
                    m_errors.append(f"corridor.yaml: {e}")

        if not m_ok or m_errors:
            ok = False
            print(f"MODULE FAIL: {mid}")
            for e in m_errors:
                print("  -", e)

    if ok:
        print("OK: profile and referenced modules validate")
        return 0
    return 2

def cmd_build(args: argparse.Namespace) -> int:
    """Compose a deterministic bundle.

    Build modes:
    - Profile build: `msez build profiles/.../profile.yaml`
    - Zone build (recommended): `msez build --zone jurisdictions/.../zone.yaml`
      This applies overlays and params_overrides from the zone manifest.
    """

    module_index = build_module_index(REPO_ROOT)
    profile_index = build_profile_index(REPO_ROOT)
    corridor_index = build_corridor_index(REPO_ROOT)

    # Load zone/profile
    zone = None
    zone_path = None
    if getattr(args, "zone", ""):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        if not zone_path.exists():
            print(f"ERROR: zone not found: {zone_path}")
            return 2
        zone = load_yaml(zone_path)

        # schema + semantics
        zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
        zerrs = validate_with_schema(zone, zone_schema)
        zerrs.extend(validate_zone_semantics(zone, profile_index, corridor_index))
        if zerrs:
            print("ZONE FAIL:")
            for e in zerrs:
                print("  -", e)
            return 2

        pid = zone["profile"]["profile_id"]
        prof_path = profile_index.get(pid)
        if not prof_path:
            print(f"ERROR: profile not found for profile_id {pid}")
            return 2
        profile = load_yaml(prof_path)
    else:
        if not getattr(args, "profile", ""):
            print("ERROR: provide either a profile path or --zone")
            return 2
        profile_path = pathlib.Path(args.profile)
        if not profile_path.is_absolute():
            profile_path = REPO_ROOT / profile_path
        if not profile_path.exists():
            print(f"ERROR: profile not found: {profile_path}")
            return 2
        profile = load_yaml(profile_path)

    # Validate profile before building
    profile_schema = schema_validator(REPO_ROOT / "schemas" / "profile.schema.json")
    perrs = validate_with_schema(profile, profile_schema)
    perrs.extend(validate_profile_semantics(profile, module_index, corridor_index))
    if perrs:
        print("PROFILE FAIL:")
        for e in perrs:
            print("  -", e)
        return 2

    # Prepare output
    out_dir = pathlib.Path(args.out)
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    bundle_dir = out_dir / "bundle"
    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    # Copy module directories into bundle
    for m in profile.get("modules", []):
        if not isinstance(m, dict):
            print("ERROR: invalid profile.modules entry")
            return 2
        mid = m.get("module_id")
        resolved = resolve_module_by_id(mid, module_index)
        if not resolved:
            print("ERROR: cannot build; missing module", mid)
            return 2
        found, _ = resolved
        target = bundle_dir / found.relative_to(REPO_ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(found, target)

    # Apply overlays (zone build only)
    if zone and zone_path:
        for ov in zone.get("overlays", []) or []:
            for patch in ov.get("patches", []) or []:
                pp = (zone_path.parent / patch).resolve()
                if not pp.exists():
                    print(f"ERROR: overlay patch not found: {pp}")
                    return 2
                try:
                    subprocess.run(
                        [
                            "git",
                            "apply",
                            "--unsafe-paths",
                            "--directory",
                            str(bundle_dir),
                            str(pp),
                        ],
                        check=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                    )
                except subprocess.CalledProcessError as ex:
                    print("ERROR: failed to apply overlay patch:", pp)
                    if ex.stderr:
                        print(ex.stderr.strip())
                    return 2

    # Render templated artifacts (in-place in the bundle)
    strict_render = bool(getattr(args, "strict_render", False))
    if not getattr(args, "no_render", False):
        base_ctx: Dict[str, Any] = {}
        if zone:
            base_ctx.update(
                {
                    "zone_id": zone.get("zone_id"),
                    "zone_name": zone.get("zone_name"),
                    "jurisdiction_id": zone.get("jurisdiction_id"),
                }
            )
        for m in profile.get("modules", []):
            if not isinstance(m, dict):
                continue
            mid = m.get("module_id")
            resolved = resolve_module_by_id(mid, module_index)
            if not resolved:
                continue
            mdir, mdata = resolved
            bundle_mod_dir = bundle_dir / mdir.relative_to(REPO_ROOT)

            # Resolve parameter values: defaults -> profile params -> zone params_overrides
            params: Dict[str, Any] = {}
            for pname, pspec in (mdata.get("parameters") or {}).items():
                if isinstance(pspec, dict) and "default" in pspec:
                    params[pname] = pspec.get("default")
            params.update(m.get("params", {}) or {})
            if zone:
                overrides = (zone.get("params_overrides") or {}).get(mid) or {}
                if isinstance(overrides, dict):
                    params.update(overrides)

            ctx = build_template_context({**base_ctx, **params})
            try:
                render_templates_in_dir(bundle_mod_dir, ctx, strict=strict_render)
            except KeyError as ex:
                print(f"ERROR: templating failed for {mid}: {ex}")
                return 2

    # Write resolved profile + optional lockfile
    (bundle_dir / "profile.resolved.yaml").write_text(
        yaml.safe_dump(profile, sort_keys=False), encoding="utf-8"
    )

    if zone and zone_path:
        # Emit a lockfile alongside the bundle for reproducible deployments
        lock_out = out_dir / "stack.lock"
        try:
            zone_arg = str(zone_path.relative_to(REPO_ROOT))
        except Exception:
            zone_arg = str(zone_path)
        try:
            out_arg = str(lock_out.relative_to(REPO_ROOT))
        except Exception:
            out_arg = str(lock_out)
        rc = cmd_lock(argparse.Namespace(zone=zone_arg, out=out_arg))
        if rc != 0:
            return rc

    print("BUILD OK: wrote bundle to", bundle_dir)
    return 0

def digest_dir(path: pathlib.Path) -> str:
    # Deterministic directory digest: hash of relative paths + content hashes
    h = hashlib.sha256()
    files = sorted([p for p in path.rglob("*") if p.is_file()])
    for f in files:
        rel = str(f.relative_to(path)).encode("utf-8")
        h.update(rel)
        h.update(b"\0")
        h.update(f.read_bytes())
        h.update(b"\0")
    return h.hexdigest()

def cmd_lock(args: argparse.Namespace) -> int:
    emit_artifactrefs = bool(getattr(args, "emit_artifactrefs", False))

    zone_path = pathlib.Path(args.zone)
    if not zone_path.is_absolute():
        zone_path = REPO_ROOT / zone_path
    zone = load_yaml(zone_path)

    # validate zone schema first
    zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
    errors = validate_with_schema(zone, zone_schema)
    if errors:
        print("ZONE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    profile_id = zone["profile"]["profile_id"]
    profile_version = zone["profile"]["version"]

    # Find profile file by profile_id (best effort)
    profile_file = None
    for p in find_profiles(REPO_ROOT):
        prof = load_yaml(p)
        if prof.get("profile_id") == profile_id:
            profile_file = p
            profile = prof
            break
    if profile_file is None:
        print("ERROR: profile not found for profile_id", profile_id)
        return 2

    lock = {
        "stack_spec_version": STACK_SPEC_VERSION,
        "generated_at": datetime.utcnow().replace(microsecond=0).isoformat() + "Z",
        "zone_id": zone["zone_id"],
        "profile": {"profile_id": profile_id, "version": profile_version},
        "modules": [],
        "lawpacks": [],
        "overlays": [],
        "corridors": []
    }

    # module resolution
    for m in profile.get("modules", []):
        mid = m["module_id"]
        resolved = resolve_module_by_id(mid)
        if not resolved:
            print("ERROR: missing module", mid)
            return 2
        mdir, mdata = resolved
        entry = {
            "module_id": mid,
            "version": mdata.get("version"),
            "variant": m.get("variant", "default"),
            "params": m.get("params", {}),
            "manifest_sha256": sha256_file(mdir / "module.yaml"),
            "content_sha256": digest_dir(mdir),
            "provides": mdata.get("provides", [])
        }
        lock["modules"].append(entry)

    # overlays
    for ov in zone.get("overlays", []) or []:
        module_id = ov["module_id"]
        patch_hashes = []
        for patch in ov.get("patches", []):
            pp = zone_path.parent / patch
            if pp.exists():
                patch_hashes.append(sha256_file(pp))
            else:
                patch_hashes.append("MISSING")
        lock["overlays"].append({"module_id": module_id, "patches_sha256": patch_hashes})

    # lawpacks (jurisdictional legal corpus pins)
    # Zones may optionally declare a jurisdiction_stack (multiple governing layers), and lawpack_domains.
    jurisdiction_stack = zone.get("jurisdiction_stack") or [zone.get("jurisdiction_id")]
    if not isinstance(jurisdiction_stack, list) or not jurisdiction_stack:
        jurisdiction_stack = [zone.get("jurisdiction_id")]
    lawpack_domains = zone.get("lawpack_domains") or ["civil", "financial"]
    if not isinstance(lawpack_domains, list) or not lawpack_domains:
        lawpack_domains = ["civil", "financial"]

    for jid in jurisdiction_stack:
        if not jid:
            continue
        for dom in lawpack_domains:
            if not dom:
                continue
            # expected module location: modules/legal/jurisdictions/<jid segments>/<domain>
            mdir = REPO_ROOT / "modules" / "legal" / "jurisdictions"
            for seg in str(jid).split("-"):
                mdir = mdir / seg
            mdir = mdir / str(dom)
            lp_lock_path = mdir / "lawpack.lock.json"

            entry = {
                "jurisdiction_id": str(jid),
                "domain": str(dom),
                "lawpack_digest_sha256": "MISSING",
                "lawpack_lock_path": str(lp_lock_path.relative_to(REPO_ROOT)),
                "lawpack_lock_sha256": "MISSING",
                "lawpack_artifact_path": "",
                "as_of_date": "",
            }

            if lp_lock_path.exists():
                try:
                    lp_lock = load_json(lp_lock_path)
                    entry["lawpack_digest_sha256"] = str(lp_lock.get("lawpack_digest_sha256") or "MISSING")
                    entry["lawpack_lock_sha256"] = sha256_file(lp_lock_path)
                    entry["lawpack_artifact_path"] = str(lp_lock.get("artifact_path") or "")


                    # Prefer the canonical CAS convention when present:
                    # dist/artifacts/lawpack/<digest>.lawpack.zip
                    dg = str(entry.get("lawpack_digest_sha256") or "").strip().lower()
                    if SHA256_HEX_RE.match(dg):
                        cas_candidate = REPO_ROOT / "dist" / "artifacts" / "lawpack" / f"{dg}.lawpack.zip"
                        if cas_candidate.exists():
                            try:
                                entry["lawpack_artifact_path"] = str(cas_candidate.relative_to(REPO_ROOT))
                            except Exception:
                                entry["lawpack_artifact_path"] = str(cas_candidate)
                    entry["as_of_date"] = str(lp_lock.get("as_of_date") or "")
                except Exception:
                    pass

            # Optional v0.4.14+ emission: use ArtifactRef as the default digest substrate.
            dg_final = str(entry.get("lawpack_digest_sha256") or "").strip().lower()
            if emit_artifactrefs and SHA256_HEX_RE.match(dg_final):
                # Resolution hint: prefer CAS path if known, otherwise point to lawpack.lock.json.
                hint = str(entry.get("lawpack_artifact_path") or "").strip() or str(entry.get("lawpack_lock_path") or "").strip()
                entry["lawpack_digest_sha256"] = make_artifact_ref(
                    "lawpack",
                    dg_final,
                    uri=hint,
                )

            lock["lawpacks"].append(entry)

    # corridors
    for cid in zone.get("corridors", []) or []:
        # best effort: locate corridor module by corridor_id
        trust_hash = ""
        rot_hash = ""
        manifest_hash = ""
        vc_hash = ""
        trust_uri = ""
        rot_uri = ""
        manifest_uri = ""
        vc_uri = ""
        signers: List[str] = []

        agreement_hashes: List[str] = []
        agreement_uris: List[str] = []
        agreement_signers: List[str] = []
        activated: bool | None = None

        for mdir in find_modules(REPO_ROOT):
            mdata = load_yaml(mdir / "module.yaml")
            if mdata.get("kind") != "corridor":
                continue

            cy = mdir / "corridor.yaml"
            if not cy.exists():
                continue

            c = load_yaml(cy)
            if c.get("corridor_id") != cid:
                continue

            # Digests for security artifacts + corridor manifest
            ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
            kr_rel = str(c.get("key_rotation_path") or "key-rotation.yaml")
            ta = mdir / ta_rel
            kr = mdir / kr_rel
            trust_hash = sha256_file(ta) if ta.exists() else "MISSING"
            rot_hash = sha256_file(kr) if kr.exists() else "MISSING"
            manifest_hash = sha256_file(cy) if cy.exists() else "MISSING"

            # URI hints (repo-relative) for typed ArtifactRef emission
            try:
                trust_uri = str(ta.relative_to(REPO_ROOT))
            except Exception:
                trust_uri = str(ta)
            try:
                rot_uri = str(kr.relative_to(REPO_ROOT))
            except Exception:
                rot_uri = str(kr)
            try:
                manifest_uri = str(cy.relative_to(REPO_ROOT))
            except Exception:
                manifest_uri = str(cy)

            # Corridor Definition VC (required in v0.3+; optional in older stacks)
            vc_rel = (c.get("definition_vc_path") or "").strip()
            vc_path = (mdir / vc_rel) if vc_rel else None
            if vc_path and vc_path.exists():
                vc_hash = sha256_file(vc_path)
                try:
                    vc_uri = str(vc_path.relative_to(REPO_ROOT))
                except Exception:
                    vc_uri = str(vc_path)
                try:
                    vcj = load_json(vc_path)
                    pr = vcj.get("proof")
                    if isinstance(pr, dict):
                        signers = [str(pr.get("verificationMethod") or "")]
                    elif isinstance(pr, list):
                        signers = [str(x.get("verificationMethod") or "") for x in pr if isinstance(x, dict)]
                    signers = sorted({s.split("#", 1)[0] for s in signers if s})
                except Exception:
                    signers = []
            else:
                vc_hash = "MISSING" if vc_rel else ""

            # Corridor Agreement VC(s) (optional)
            for rel in _agreement_paths(c):
                ap = mdir / rel
                if ap.exists():
                    agreement_hashes.append(sha256_file(ap))
                    try:
                        agreement_uris.append(str(ap.relative_to(REPO_ROOT)))
                    except Exception:
                        agreement_uris.append(str(ap))
                    try:
                        avcj = load_json(ap)
                        pr = avcj.get("proof")
                        if isinstance(pr, dict):
                            agreement_signers.append(str(pr.get("verificationMethod") or ""))
                        elif isinstance(pr, list):
                            agreement_signers.extend([str(x.get("verificationMethod") or "") for x in pr if isinstance(x, dict)])
                    except Exception:
                        pass
                else:
                    agreement_hashes.append("MISSING")
                    agreement_uris.append(str(rel))

            agreement_signers = sorted({s.split("#", 1)[0] for s in agreement_signers if s})

            # Activation status (thresholds) — when agreement VC(s) exist
            asummary: Dict[str, Any] = {}
            aerrs: List[str] = []
            if agreement_hashes:
                try:
                    aerrs, asummary = corridor_agreement_summary(mdir)
                    activated = bool(asummary.get("activated")) and not aerrs
                    # Prefer signed_parties if available (participant-specific agreement VCs)
                    signed_parties = asummary.get("signed_parties") or []
                    if signed_parties:
                        agreement_signers = list(signed_parties)
                except Exception:
                    activated = False

            break

        entry: Dict[str, Any] = {
            "corridor_id": cid,
            "corridor_manifest_sha256": manifest_hash or "",
            "trust_anchors_sha256": trust_hash or "",
            "key_rotation_sha256": rot_hash or "",
            "corridor_definition_vc_sha256": vc_hash or "",
            "corridor_definition_signers": signers,
        }

        # Optional v0.4.14+ emission: use ArtifactRef as the default digest substrate.
        if emit_artifactrefs:
            mh = str(entry.get("corridor_manifest_sha256") or "").strip().lower()
            th = str(entry.get("trust_anchors_sha256") or "").strip().lower()
            rh = str(entry.get("key_rotation_sha256") or "").strip().lower()
            vh = str(entry.get("corridor_definition_vc_sha256") or "").strip().lower()
            if SHA256_HEX_RE.match(mh):
                entry["corridor_manifest_sha256"] = make_artifact_ref("blob", mh, uri=manifest_uri)
            if SHA256_HEX_RE.match(th):
                entry["trust_anchors_sha256"] = make_artifact_ref("blob", th, uri=trust_uri)
            if SHA256_HEX_RE.match(rh):
                entry["key_rotation_sha256"] = make_artifact_ref("blob", rh, uri=rot_uri)
            if SHA256_HEX_RE.match(vh):
                entry["corridor_definition_vc_sha256"] = make_artifact_ref("blob", vh, uri=vc_uri)

        if agreement_hashes:
            if emit_artifactrefs:
                refs: List[Any] = []
                for i, dg in enumerate(agreement_hashes):
                    dgn = str(dg or "").strip().lower()
                    if SHA256_HEX_RE.match(dgn):
                        uri = agreement_uris[i] if i < len(agreement_uris) else ""
                        refs.append(make_artifact_ref("blob", dgn, uri=uri))
                    else:
                        refs.append(dg)
                entry["corridor_agreement_vc_sha256"] = refs
            else:
                entry["corridor_agreement_vc_sha256"] = agreement_hashes
            entry["corridor_agreement_signers"] = agreement_signers
            entry["corridor_activated"] = bool(activated) if activated is not None else False
            # Optional: content-addressed agreement set digest + per-file payload hashes
            aset = (asummary or {}).get("agreement_set_sha256")
            if aset:
                entry["corridor_agreement_set_sha256"] = aset
            ap = (asummary or {}).get("agreement_payload_sha256_by_path")
            if isinstance(ap, dict) and ap:
                entry["corridor_agreement_payload_sha256_by_path"] = ap
            blockers = (asummary or {}).get("blocked_parties") or []
            if isinstance(blockers, list) and blockers:
                entry["corridor_activation_blockers"] = [
                    f"{b.get('id')}:{b.get('commitment')}" for b in blockers if isinstance(b, dict)
                ]

        lock["corridors"].append(entry)


    out_path = pathlib.Path(args.out) if args.out else zone_path.parent / (zone.get("lockfile_path") or "stack.lock")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(yaml.safe_dump(lock, sort_keys=False), encoding="utf-8")
    print("LOCK OK: wrote", out_path)
    return 0

def cmd_check_coverage(args: argparse.Namespace) -> int:
    """Check that all MUST/SHALL clauses in legal Akoma sources are mapped.

    This is the CLI equivalent of tests/test_policy_to_code_completeness.py.
    """
    policy_schema = schema_validator(REPO_ROOT / "schemas" / "policy-to-code.schema.json")
    module_index = build_module_index(REPO_ROOT)

    # Determine scope: all modules, a profile, or a zone.
    module_ids: List[str] = []
    if getattr(args, "zone", ""):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        zone = load_yaml(zone_path)
        pid = (zone.get("profile") or {}).get("profile_id")
        if not pid:
            print("ERROR: zone missing profile.profile_id")
            return 2
        # resolve profile
        prof_path = build_profile_index(REPO_ROOT).get(pid)
        if not prof_path:
            print("ERROR: profile not found for", pid)
            return 2
        profile = load_yaml(prof_path)
        module_ids = [m.get("module_id") for m in (profile.get("modules") or []) if isinstance(m, dict) and m.get("module_id")]
    elif getattr(args, "profile", ""):
        prof_path = pathlib.Path(args.profile)
        if not prof_path.is_absolute():
            prof_path = REPO_ROOT / prof_path
        profile = load_yaml(prof_path)
        module_ids = [m.get("module_id") for m in (profile.get("modules") or []) if isinstance(m, dict) and m.get("module_id")]
    else:
        module_ids = list(module_index.keys())

    must_re = re.compile(r"\b(MUST|SHALL)\b")

    def extract_must_eids(xml_path: pathlib.Path) -> List[str]:
        doc = etree.parse(str(xml_path))
        eids: List[str] = []
        for el in doc.xpath('//*[@eId]'):
            text = " ".join([t.strip() for t in el.xpath(".//text()") if str(t).strip()])
            if must_re.search(text):
                eids.append(str(el.get("eId")))
        return eids

    def map_covers_eid(map_data: Any, eid: str) -> bool:
        if not isinstance(map_data, list):
            return False
        for entry in map_data:
            if not isinstance(entry, dict):
                continue
            for lr in entry.get("legal_refs", []) or []:
                if isinstance(lr, dict) and lr.get("eId") == eid:
                    return True
                if isinstance(lr, str) and eid in lr:
                    return True
        return False

    ok = True
    for mid in module_ids:
        if mid not in module_index:
            # ignore missing in scope (e.g., stale profile)
            continue
        mdir, _ = module_index[mid]
        akn_dir = mdir / "src" / "akn"
        if not akn_dir.exists():
            continue
        must_eids: List[str] = []
        for xml in akn_dir.rglob("*.xml"):
            must_eids.extend(extract_must_eids(xml))
        if not must_eids:
            continue

        map_path = mdir / "src" / "policy-to-code" / "map.yaml"
        if not map_path.exists():
            ok = False
            print(f"FAIL: {mid} has MUST/SHALL clauses but no policy-to-code map at {map_path.relative_to(mdir)}")
            continue
        map_data = load_yaml(map_path)
        map_errors = validate_with_schema(map_data, policy_schema)
        if map_errors:
            ok = False
            print(f"FAIL: {mid} policy-to-code map schema errors:")
            for e in map_errors:
                print("  -", e)
            continue

        missing = [eid for eid in must_eids if not map_covers_eid(map_data, eid)]
        if missing:
            ok = False
            print(f"FAIL: {mid} missing policy-to-code entries for eIds: {missing}")

    if ok:
        print("OK: policy-to-code coverage checks passed")
        return 0
    return 2


def cmd_law_list(args: argparse.Namespace) -> int:
    """List jurisdictional legal corpus modules (placeholders or populated)."""
    out: List[Dict[str, Any]] = []
    for mp in REPO_ROOT.glob("modules/legal/jurisdictions/**/module.yaml"):
        mod_dir = mp.parent
        rel = mod_dir.relative_to(REPO_ROOT)
        parts = rel.parts
        # modules/legal/jurisdictions/<jid_path...>/<domain>
        if len(parts) < 5:
            continue
        domain = parts[-1]
        jid_parts = parts[3:-1]
        jurisdiction_id = "-".join(jid_parts)
        if args.jurisdiction and args.jurisdiction != jurisdiction_id:
            continue
        if args.domain and args.domain != domain:
            continue
        manifest = load_yaml(mp)
        out.append({
            "jurisdiction_id": jurisdiction_id,
            "domain": domain,
            "module_id": manifest.get("module_id"),
            "path": str(rel),
            "version": manifest.get("version"),
            "license": manifest.get("license"),
        })

    out = sorted(out, key=lambda r: (r["jurisdiction_id"], r["domain"]))
    if args.json:
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        for r in out:
            print(f"{r['jurisdiction_id']:>14}  {r['domain']:<10}  {r['module_id']}  ({r['path']})")
    return 0


def cmd_law_coverage(args: argparse.Namespace) -> int:
    """Report whether the scaffolded corpus modules exist for each registry jurisdiction."""
    reg_path = REPO_ROOT / "registries" / "jurisdictions.yaml"
    if not reg_path.exists():
        print("registries/jurisdictions.yaml not found")
        return 2
    reg = load_yaml(reg_path) or []
    domains = ["civil", "financial"]
    rows: List[Dict[str, Any]] = []
    for e in reg:
        if not isinstance(e, dict):
            continue
        jid = e.get("jurisdiction_id", "")
        name = e.get("name", "")
        if not jid:
            continue
        base = REPO_ROOT / "modules" / "legal" / "jurisdictions"
        for seg in jid.split("-"):
            base = base / seg
        row = {"jurisdiction_id": jid, "name": name}
        for d in domains:
            row[d] = (base / d / "module.yaml").exists()
        rows.append(row)

    rows = sorted(rows, key=lambda r: r["jurisdiction_id"])
    if args.json:
        print(json.dumps(rows, indent=2, sort_keys=True))
    else:
        print(f"{'jurisdiction_id':<18}  {'civil':<5}  {'financial':<9}  name")
        print("-" * 88)
        for r in rows:
            civ = "yes" if r['civil'] else "no"
            fin = "yes" if r['financial'] else "no"
            print(f"{r['jurisdiction_id']:<18}  {civ:<5}  {fin:<9}  {r['name']}")
    return 0


def cmd_law_ingest(args: argparse.Namespace) -> int:
    """Ingest a jurisdiction corpus module into a content-addressed lawpack.zip.

    This builds:
      - dist/lawpacks/<jurisdiction_id>/<domain>/<digest>.lawpack.zip (implementation output)
      - dist/artifacts/lawpack/<digest>.lawpack.zip (canonical CAS copy; see spec/97-artifacts.md)
      - <module_dir>/lawpack.lock.json

    The lock entry is later pinned into stack.lock via `msez lock`.
    """
    from tools.lawpack import ingest_lawpack  # type: ignore

    module_dir = pathlib.Path(args.module)
    if not module_dir.is_absolute():
        module_dir = REPO_ROOT / module_dir

    out_dir = pathlib.Path(args.out_dir) if args.out_dir else (REPO_ROOT / "dist" / "lawpacks")
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir

    as_of_date = str(args.as_of_date or "").strip()
    if not as_of_date:
        print("ERROR: --as-of-date is required (YYYY-MM-DD)", file=sys.stderr)
        return 2

    try:
        lock_obj = ingest_lawpack(
            module_dir=module_dir,
            out_dir=out_dir,
            as_of_date=as_of_date,
            repo_root=REPO_ROOT,
            fetch=bool(getattr(args, "fetch", False)),
            include_raw=bool(getattr(args, "include_raw", False)),
            tool_version=STACK_SPEC_VERSION,
        )
    except Exception as ex:
        print(f"ERROR: law ingest failed: {ex}", file=sys.stderr)
        return 2

    # v0.4.7+: store a canonical CAS copy: dist/artifacts/lawpack/<digest>.lawpack.zip
    cas_artifact_path: str = ""
    try:
        digest = str(lock_obj.get("lawpack_digest_sha256") or "").strip().lower()
        apath = str(lock_obj.get("artifact_path") or "").strip()
        if digest and apath:
            src = pathlib.Path(apath)
            if not src.is_absolute():
                src = REPO_ROOT / src
            if src.exists():
                cas_path = artifact_cas.store_artifact_file(
                    "lawpack",
                    digest,
                    src,
                    repo_root=REPO_ROOT,
                    store_root=None,
                    dest_name=None,
                    overwrite=False,
                )
                try:
                    cas_artifact_path = str(cas_path.relative_to(REPO_ROOT))
                except Exception:
                    cas_artifact_path = str(cas_path)
    except Exception as ex:
        print(f"WARN: unable to store lawpack artifact in dist/artifacts: {ex}", file=sys.stderr)
        cas_artifact_path = ""

    if args.json:
        out = dict(lock_obj)
        if cas_artifact_path:
            out["cas_artifact_path"] = cas_artifact_path
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        print("LAWPACK OK:")
        print("  jurisdiction_id:", lock_obj.get("jurisdiction_id"))
        print("  domain:", lock_obj.get("domain"))
        print("  as_of_date:", lock_obj.get("as_of_date"))
        print("  digest:", lock_obj.get("lawpack_digest_sha256"))
        print("  artifact_path:", lock_obj.get("artifact_path"))
        if cas_artifact_path:
            print("  cas_artifact_path:", cas_artifact_path)
        print("  lock_path:", (module_dir / "lawpack.lock.json").relative_to(REPO_ROOT))
    return 0


def cmd_law_attest_init(args: argparse.Namespace) -> int:
    """Initialize a Lawpack Validity Attestation VC skeleton.

    This does not add a proof; use `msez vc sign` to sign the generated VC.
    """

    jurisdiction_id = str(getattr(args, "jurisdiction_id", "") or "").strip()
    domain = str(getattr(args, "domain", "") or "").strip()
    as_of_date = str(getattr(args, "as_of_date", "") or "").strip()
    issuer = str(getattr(args, "issuer", "") or "").strip() or "did:key:REPLACE_ME"

    dg = _coerce_sha256(getattr(args, "lawpack_digest", ""))
    if not SHA256_HEX_RE.match(dg):
        print("ERROR: --lawpack-digest must be a 64-hex sha256 digest", file=sys.stderr)
        return 2

    vc: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1.jsonld",
            "https://schemas.momentum-sez.org/contexts/msez/lawpack/v1.jsonld",
        ],
        "id": f"urn:uuid:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "MSEZLawpackAttestationCredential"],
        "issuer": {"id": issuer},
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "jurisdiction_id": jurisdiction_id,
            "domain": domain,
            "as_of_date": as_of_date,
            "lawpack": make_artifact_ref("lawpack", dg),
            "assertion": {
                "status": "asserted_valid",
                "statement": (
                    "I attest that the referenced lawpack digest corresponds to a legally valid body of law "
                    "for the stated jurisdiction and domain as-of the specified date."
                ),
                "sources": [],
                "evidence": [],
            },
        },
    }

    out_path = pathlib.Path(str(getattr(args, "out", "") or "") or f"lawpack-attestation.{dg[:8]}.vc.json")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(vc, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out_path.relative_to(REPO_ROOT)))
    return 0


def cmd_diff(args: argparse.Namespace) -> int:
    """Diff two stack.lock files for upgrade impact analysis."""
    a_path = pathlib.Path(args.a)
    b_path = pathlib.Path(args.b)
    if not a_path.is_absolute():
        a_path = REPO_ROOT / a_path
    if not b_path.is_absolute():
        b_path = REPO_ROOT / b_path
    a = load_yaml(a_path)
    b = load_yaml(b_path)

    a_mods = {m["module_id"]: m for m in (a.get("modules") or []) if isinstance(m, dict) and m.get("module_id")}
    b_mods = {m["module_id"]: m for m in (b.get("modules") or []) if isinstance(m, dict) and m.get("module_id")}

    added = sorted(set(b_mods) - set(a_mods))
    removed = sorted(set(a_mods) - set(b_mods))
    common = sorted(set(a_mods) & set(b_mods))

    changed: List[str] = []
    for mid in common:
        am = a_mods[mid]
        bm = b_mods[mid]
        fields = ["version", "variant", "manifest_sha256", "content_sha256"]
        for f in fields:
            if str(am.get(f, "")) != str(bm.get(f, "")):
                changed.append(mid)
                break
        else:
            # params diff
            if (am.get("params") or {}) != (bm.get("params") or {}):
                changed.append(mid)

    print(f"Diff: {a_path} -> {b_path}")
    if added:
        print("\nAdded modules:")
        for mid in added:
            print("  +", mid)
    if removed:
        print("\nRemoved modules:")
        for mid in removed:
            print("  -", mid)
    if changed:
        print("\nChanged modules:")
        for mid in sorted(set(changed)):
            am = a_mods[mid]
            bm = b_mods[mid]
            print(f"  ~ {mid}")
            for f in ["version", "variant", "manifest_sha256", "content_sha256"]:
                av = str(am.get(f, ""))
                bv = str(bm.get(f, ""))
                if av != bv:
                    print(f"      {f}: {av} -> {bv}")
            if (am.get("params") or {}) != (bm.get("params") or {}):
                print("      params: changed")

    # overlays + corridors (best effort)
    if (a.get("overlays") or []) != (b.get("overlays") or []):
        print("\nOverlays: changed")
    if (a.get("corridors") or []) != (b.get("corridors") or []):
        print("\nCorridors: changed")

    return 0

def cmd_publish(args: argparse.Namespace) -> int:
    """Publish rendered artifacts (Akoma -> HTML/PDF) from a repo or bundle directory."""
    root = pathlib.Path(args.path)
    if not root.is_absolute():
        root = REPO_ROOT / root
    if not root.exists():
        print(f"ERROR: path not found: {root}")
        return 2

    out_dir = pathlib.Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    from tools.akoma.render import render_html, render_pdf_from_html, render_pdf_text

    xslt_path = REPO_ROOT / "tools" / "akoma" / "xslt" / "akn2html.xsl"
    if not xslt_path.exists():
        print(f"ERROR: missing XSLT at {xslt_path}")
        return 2

    xml_files = sorted({p for p in root.glob("**/src/akn/**/*.xml") if p.is_file()})
    if not xml_files:
        print("No Akoma XML files found under", root)
        return 0

    wrote = 0
    for xml in xml_files:
        try:
            rel = xml.relative_to(root)
        except Exception:
            rel = pathlib.Path(xml.name)
        html_out = out_dir / rel.with_suffix(".html")
        html_out.parent.mkdir(parents=True, exist_ok=True)
        try:
            render_html(xml, xslt_path, html_out)
        except Exception as ex:
            print(f"WARN: failed to render {xml}: {ex}")
            continue
        wrote += 1
        if args.pdf:
            pdf_out = out_dir / rel.with_suffix(".pdf")
            pdf_out.parent.mkdir(parents=True, exist_ok=True)
            if not render_pdf_from_html(html_out, pdf_out):
                render_pdf_text(xml, pdf_out)
        
    print(f"PUBLISH OK: rendered {wrote} document(s) to {out_dir}")
    return 0



# ---------------------------------------------------------------------------
# Verifiable Credentials (VC) and corridor cryptographic verification
# ---------------------------------------------------------------------------

def _as_list(x: Any) -> List[Any]:
    if x is None:
        return []
    if isinstance(x, list):
        return x
    return [x]

def verify_corridor_definition_vc(module_dir: pathlib.Path) -> List[str]:
    """
    Verify that a corridor module is cryptographically bound:
    - corridor.yaml validates
    - trust-anchors/key-rotation validate
    - corridor definition VC verifies cryptographically
    - VC subject binds to sha256(corridor.yaml / trust-anchors.yaml / key-rotation.yaml)
    - VC signer(s) are authorized by trust-anchors.yaml for `corridor_definition`
    """
    errs: List[str] = []
    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        return ["Missing corridor.yaml"]

    corridor_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.schema.json")
    c = load_yaml(corridor_path)
    errs.extend([f"corridor.yaml: {e}" for e in validate_with_schema(c, corridor_schema)])

    # Security artifacts
    ta_path = module_dir / (c.get("trust_anchors_path") or "trust-anchors.yaml")
    kr_path = module_dir / (c.get("key_rotation_path") or "key-rotation.yaml")
    if not ta_path.exists():
        errs.append(f"Missing {ta_path.name}")
        return errs
    if not kr_path.exists():
        errs.append(f"Missing {kr_path.name}")
        return errs

    trust_schema = schema_validator(REPO_ROOT / "schemas" / "trust-anchors.schema.json")
    keyrot_schema = schema_validator(REPO_ROOT / "schemas" / "key-rotation.schema.json")
    errs.extend([f"{ta_path.name}: {e}" for e in validate_with_schema(load_yaml(ta_path), trust_schema)])
    errs.extend([f"{kr_path.name}: {e}" for e in validate_with_schema(load_yaml(kr_path), keyrot_schema)])

    # Definition VC
    vc_rel = (c.get("definition_vc_path") or "").strip()
    if not vc_rel:
        errs.append("corridor.yaml: missing definition_vc_path")
        return errs
    vc_path = module_dir / vc_rel
    if not vc_path.exists():
        errs.append(f"Missing {vc_rel}")
        return errs

    vc_schema = schema_validator(REPO_ROOT / "schemas" / "vc.corridor-definition.schema.json")
    vcj = load_json(vc_path)
    errs.extend([f"{vc_rel}: {e}" for e in validate_with_schema(vcj, vc_schema)])

    # Cryptographic verification (supports did:key offline)
    try:
        from tools.vc import verify_credential  # type: ignore
        results = verify_credential(vcj)
        ok_methods = [r.verification_method for r in results if r.ok]
        bad = [r for r in results if not r.ok]
        if not ok_methods:
            errs.append(f"{vc_rel}: no valid proofs")
        for r in bad:
            errs.append(f"{vc_rel}: invalid proof for {r.verification_method}: {r.error}")
    except Exception as ex:
        errs.append(f"{vc_rel}: VC verification error: {ex}")
        ok_methods = []

    # Binding checks: artifact sha256 pins
    subj = (vcj.get("credentialSubject") or {})
    if isinstance(subj, dict):
        art = subj.get("artifacts") or {}
        if isinstance(art, dict):
            for name, spec in art.items():
                if not isinstance(spec, dict):
                    continue

                # Legacy form: {path, sha256}
                if ("sha256" in spec) or ("path" in spec):
                    rel = str(spec.get("path") or "").strip()
                    expected = str(spec.get("sha256") or "").strip().lower()
                # ArtifactRef form: {artifact_type, digest_sha256, uri?, ...}
                else:
                    rel = str(spec.get("uri") or spec.get("path") or "").strip()
                    expected = str(spec.get("digest_sha256") or "").strip().lower()
                    if not rel:
                        # Name-based defaults for the standard corridor package layout.
                        if name == "corridor_manifest":
                            rel = "corridor.yaml"
                        elif name == "trust_anchors":
                            rel = str(ta_path.name)
                        elif name == "key_rotation":
                            rel = str(kr_path.name)

                if not rel or not expected:
                    continue

                # If the hint is a non-local URI, we can't validate on-disk binding here.
                if "://" in rel or rel.startswith("ipfs:") or rel.startswith("urn:"):
                    continue

                fpath = module_dir / rel
                if not fpath.exists():
                    errs.append(f"{vc_rel}: artifacts.{name}.path missing: {rel}")
                    continue
                actual = sha256_file(fpath)
                if actual != expected:
                    errs.append(f"{vc_rel}: artifacts.{name}.sha256 mismatch (VC vs file)")

        vc_cid = subj.get("corridor_id")
        if vc_cid and vc_cid != c.get("corridor_id"):
            errs.append(f"{vc_rel}: corridor_id mismatch (VC={vc_cid} vs manifest={c.get('corridor_id')})")

    # Optional external authority registry constraint (mitigates trust-anchor circularity).
    reg_allowed, reg_errs = load_authority_registry(module_dir, c)
    if reg_errs:
        errs.extend(reg_errs)
    reg_def_allowed = set(reg_allowed.get("corridor_definition", set())) | set(reg_allowed.get("*", set()))

    # Authorization checks against trust anchors
    try:
        ta = load_yaml(ta_path)
        anchors = ta.get("trust_anchors") or []
        allowed = set()
        for a in anchors:
            if not isinstance(a, dict):
                continue
            if "corridor_definition" in (a.get("allowed_attestations") or []):
                allowed.add(str(a.get("identifier") or "").split("#", 1)[0])

        # If a registry is present, the corridor module trust-anchor set must be a subset of it.
        if reg_def_allowed:
            for did in allowed:
                if did and did not in reg_def_allowed:
                    errs.append(f"{vc_rel}: trust anchor {did} is not authorized by authority-registry for corridor_definition")

            # Enforce that corridor_definition signers are drawn from the registry-constrained set.
            allowed = allowed.intersection(reg_def_allowed)

        for vm in ok_methods:
            did = str(vm).split("#", 1)[0]
            if allowed and did not in allowed:
                errs.append(f"{vc_rel}: signer {did} is not authorized for corridor_definition in trust-anchors.yaml")
    except Exception:
        # best-effort; schema validation already covers shape
        pass

    return errs


def _agreement_paths(c: Any) -> List[str]:
    """Return agreement VC path(s) from corridor.yaml supporting string or list forms."""
    if c is None or not isinstance(c, dict):
        return []
    av = c.get("agreement_vc_path")
    if not av:
        return []
    if isinstance(av, str):
        s = av.strip()
        return [s] if s else []
    if isinstance(av, list):
        out: List[str] = []
        for item in av:
            if isinstance(item, str) and item.strip():
                out.append(item.strip())
        return out
    return []

def corridor_agreement_summary(module_dir: pathlib.Path) -> Tuple[List[str], Dict[str, Any]]:
    """Compute Corridor Agreement VC status.

    Returns (errors, summary) where summary is a machine-readable dict with:
    - corridor_id
    - has_agreement
    - agreement_paths
    - definition_payload_sha256
    - participants (as provided in the VC)
    - agreement_pattern ('party-specific' or 'multi-signer')
    - signed_parties_all (unique DIDs that produced at least one valid proof)
    - signed_parties (unique DIDs counted for threshold evaluation; commitment-aware)
    - party_commitments (participant DID -> commitment verb, when party-specific)
    - blocked_parties (list of parties whose commitment blocks activation)
    - thresholds (per-role evaluation)
    - activated (True when threshold rules are satisfied and no verification errors exist)

    Notes:
    - If no agreement VC is configured, this returns (corridor.yaml schema errors, {{has_agreement: False}}).
    - Participant-specific agreement VCs MAY set credentialSubject.party. When present, validators:
        - require the party.id DID to have signed that VC,
        - require that each party appears at most once across agreement_vc_path (status lock),
        - interpret credentialSubject.commitment as that party's current status.
    - Activation thresholds are evaluated over *affirmative* commitments only, using
      activation.accept_commitments (default: ['agree']).
    """

    errs: List[str] = []
    corridor_path = module_dir / 'corridor.yaml'
    if not corridor_path.exists():
        return (['Missing corridor.yaml'], {'has_agreement': False, 'activated': False})

    corridor_schema = schema_validator(REPO_ROOT / 'schemas' / 'corridor.schema.json')
    c = load_yaml(corridor_path)
    errs.extend([f"corridor.yaml: {e}" for e in validate_with_schema(c, corridor_schema)])

    agreement_paths = _agreement_paths(c)
    summary: Dict[str, Any] = {
        'corridor_id': c.get('corridor_id'),
        'has_agreement': bool(agreement_paths),
        'agreement_paths': agreement_paths,
        'activated': False,
    }

    if not agreement_paths:
        return (errs, summary)

    # Security artifacts (trust anchors) for signer authorization
    ta_path = module_dir / (c.get('trust_anchors_path') or 'trust-anchors.yaml')
    if not ta_path.exists():
        errs.append(f"Missing {ta_path.name}")
        return (errs, summary)

    # Corridor Definition VC binding: hash the canonical payload excluding proof
    def_rel = (c.get('definition_vc_path') or '').strip()
    if not def_rel:
        errs.append('corridor.yaml: missing definition_vc_path')
        return (errs, summary)
    def_path = module_dir / def_rel
    if not def_path.exists():
        errs.append(f"Missing {def_rel}")
        return (errs, summary)

    try:
        def_vcj = load_json(def_path)
        from tools.vc import signing_input  # type: ignore
        def_payload_sha256 = sha256_bytes(signing_input(def_vcj))
        def_lawpack_compat = ((def_vcj.get('credentialSubject') or {}).get('lawpack_compatibility'))
        def_vc_id = str(def_vcj.get('id') or '').strip()
        summary['definition_payload_sha256'] = def_payload_sha256
        if def_vc_id:
            summary['definition_vc_id'] = def_vc_id
    except Exception as ex:
        errs.append(f"{def_rel}: unable to load Corridor Definition VC for agreement binding: {ex}")
        return (errs, summary)

    # Load trust anchors for corridor_agreement authorization
    allowed_agreement = set()
    try:
        ta = load_yaml(ta_path)
        for a in (ta.get('trust_anchors') or []):
            if not isinstance(a, dict):
                continue
            if 'corridor_agreement' in (a.get('allowed_attestations') or []):
                ident = str(a.get('identifier') or '').split('#', 1)[0]
                if ident:
                    allowed_agreement.add(ident)
    except Exception:
        pass

    # Optional external authority registry constraint (mitigates trust-anchor circularity).
    reg_allowed, reg_errs = load_authority_registry(module_dir, c)
    if reg_errs:
        errs.extend(reg_errs)
    reg_ag_allowed = set(reg_allowed.get("corridor_agreement", set())) | set(reg_allowed.get("*", set()))
    if reg_ag_allowed:
        for did in sorted(list(allowed_agreement)):
            if did and did not in reg_ag_allowed:
                errs.append(
                    f"{ta_path.name}: trust anchor {did} is not authorized by authority-registry for corridor_agreement"
                )
        allowed_agreement = allowed_agreement.intersection(reg_ag_allowed)

    if not allowed_agreement:
        errs.append(
            f"{ta_path.name}: no trust anchors authorize corridor_agreement (allowed_attestations includes 'corridor_agreement')"
        )

    base_subject: Dict[str, Any] | None = None
    agreement_pattern: str | None = None  # 'party-specific' or 'multi-signer'
    all_valid_signers: set[str] = set()
    signed_parties_all: set[str] = set()
    agreement_payload_sha256_by_path: Dict[str, str] = {}

    # Participant-specific state lock + commitments
    seen_party_ids: set[str] = set()
    party_by_vc: Dict[str, str] = {}
    party_role_by_vc: Dict[str, str] = {}
    party_commitment_by_party: Dict[str, str] = {}
    party_path_by_party: Dict[str, str] = {}
    pinned_lawpacks_by_party: Dict[str, List[Dict[str, Any]]] = {}

    vc_schema = schema_validator(REPO_ROOT / 'schemas' / 'vc.corridor-agreement.schema.json')

    for rel in agreement_paths:
        vc_path = module_dir / rel
        if not vc_path.exists():
            errs.append(f"Missing {rel}")
            continue

        vcj = load_json(vc_path)
        # Deterministic payload hash (excludes proofs)
        try:
            from tools.vc import signing_input  # type: ignore
            agreement_payload_sha256_by_path[rel] = sha256_bytes(signing_input(vcj))
        except Exception as ex:
            errs.append(f"{rel}: unable to compute agreement payload sha256: {ex}")
        errs.extend([f"{rel}: {e}" for e in validate_with_schema(vcj, vc_schema)])

        # Cryptographic verification
        try:
            from tools.vc import verify_credential  # type: ignore
            results = verify_credential(vcj)
            ok_methods = [r.verification_method for r in results if r.ok]
            bad = [r for r in results if not r.ok]

            if not ok_methods:
                errs.append(f"{rel}: no valid proofs")
            for r in bad:
                errs.append(f"{rel}: invalid proof for {r.verification_method}: {r.error}")

            ok_dids = [str(vm).split('#', 1)[0] for vm in ok_methods if vm]
            for did in ok_dids:
                all_valid_signers.add(did)
        except Exception as ex:
            errs.append(f"{rel}: VC verification error: {ex}")
            ok_dids = []

        subj = vcj.get('credentialSubject') or {}
        if not isinstance(subj, dict):
            errs.append(f"{rel}: credentialSubject must be an object")
            continue

        issuer_did = str(vcj.get('issuer') or '').split('#', 1)[0]
        commitment = str(subj.get('commitment') or '').strip() or 'agree'
        commitment_norm = commitment.strip().lower()

        # Corridor id binding
        vc_cid = subj.get('corridor_id')
        if vc_cid and vc_cid != c.get('corridor_id'):
            errs.append(f"{rel}: corridor_id mismatch (VC={vc_cid} vs manifest={c.get('corridor_id')})")

        # Definition VC binding
        expected_hash = str(subj.get('definition_payload_sha256') or '').strip()
        if expected_hash and expected_hash != def_payload_sha256:
            errs.append(f"{rel}: definition_payload_sha256 mismatch (VC vs corridor definition payload)")

        expected_def_id = str(subj.get('definition_vc_id') or '').strip()
        if expected_def_id and def_vc_id and expected_def_id != def_vc_id:
            errs.append(f"{rel}: definition_vc_id mismatch (VC={expected_def_id} vs definition={def_vc_id})")

        # Authorization checks (best-effort; for offline did:key, allowed list should include did:key)
        if allowed_agreement:
            for did in ok_dids:
                if did and did not in allowed_agreement:
                    errs.append(f"{rel}: signer {did} is not authorized for corridor_agreement in {ta_path.name}")

        # Participant-specific party semantics
        party_obj = subj.get('party')
        if party_obj is not None:
            if agreement_pattern == 'multi-signer':
                errs.append(
                    f"agreement: mixed agreement patterns (party-specific + multi-signer) in agreement_vc_path; choose one pattern"
                )
            agreement_pattern = agreement_pattern or 'party-specific'

            if not isinstance(party_obj, dict):
                errs.append(f"{rel}: credentialSubject.party must be an object")
            else:
                party_id = str(party_obj.get('id') or '').split('#', 1)[0]
                party_role = str(party_obj.get('role') or '').strip()

                if not party_id:
                    errs.append(f"{rel}: credentialSubject.party.id is required for participant-specific agreement VCs")
                else:
                    if party_id in seen_party_ids:
                        errs.append(
                            f"agreement: duplicate party {party_id} across agreement_vc_path (status lock violation); include only one current VC per party"
                        )
                    else:
                        seen_party_ids.add(party_id)

                    party_by_vc[rel] = party_id
                    if party_role:
                        party_role_by_vc[rel] = party_role

                    party_commitment_by_party[party_id] = commitment_norm
                    party_path_by_party[party_id] = rel

                    # Lawpack pins (v0.4.1+): participant attests to the exact legal corpus digests in force.
                    pinned = subj.get("pinned_lawpacks")
                    if pinned is None:
                        pinned_lawpacks_by_party[party_id] = []
                    elif isinstance(pinned, list):
                        pinned_lawpacks_by_party[party_id] = pinned
                    else:
                        errs.append(f"{rel}: credentialSubject.pinned_lawpacks must be an array when present")

                    # For participant-specific agreement VCs, issuer SHOULD equal party.id (keeps provenance clear)
                    if issuer_did and issuer_did != party_id:
                        errs.append(
                            f"{rel}: issuer {issuer_did} must match credentialSubject.party.id {party_id} for participant-specific agreement VCs"
                        )

                    if party_id and party_id not in ok_dids:
                        errs.append(f"{rel}: party {party_id} did not sign this agreement VC")
                    if party_id and party_id in ok_dids:
                        signed_parties_all.add(party_id)

        else:
            if agreement_pattern == 'party-specific':
                errs.append(
                    f"agreement: mixed agreement patterns (party-specific + multi-signer) in agreement_vc_path; choose one pattern"
                )
            agreement_pattern = agreement_pattern or 'multi-signer'
            # Legacy/multi-signer agreement: count all signer DIDs.
            for did in ok_dids:
                if did:
                    signed_parties_all.add(did)

        # Ensure multiple agreement VC subjects do not conflict (base fields)
        if base_subject is None:
            base_subject = subj
        else:
            keys = ('corridor_id', 'definition_payload_sha256', 'participants', 'activation', 'terms')
            # In multi-signer mode, commitment is a global property and must be consistent.
            if agreement_pattern == 'multi-signer':
                keys = keys + ('commitment',)
            for k in keys:
                if subj.get(k) != base_subject.get(k):
                    errs.append(f"{rel}: agreement VC subject.{k} does not match other agreement VC(s)")
                    break

            # definition_vc_id is optional; only enforce when both are present
            a_def = str(base_subject.get('definition_vc_id') or '').strip()
            b_def = str(subj.get('definition_vc_id') or '').strip()
            if a_def and b_def and a_def != b_def:
                errs.append(f"{rel}: agreement VC subject.definition_vc_id does not match other agreement VC(s)")

    summary['agreement_pattern'] = agreement_pattern or 'unknown'
    summary['signed_parties_all'] = sorted(signed_parties_all)
    if agreement_payload_sha256_by_path:
        summary['agreement_payload_sha256_by_path'] = {
            k: agreement_payload_sha256_by_path[k] for k in sorted(agreement_payload_sha256_by_path.keys())
        }
        digest_obj = {
            'corridor_id': summary.get('corridor_id'),
            'definition_payload_sha256': summary.get('definition_payload_sha256'),
            'agreement_payload_sha256': sorted(agreement_payload_sha256_by_path.values()),
        }
        digest_bytes = json.dumps(digest_obj, sort_keys=True, separators=(',', ':')).encode('utf-8')
        summary['agreement_set_sha256'] = sha256_bytes(digest_bytes)

    # If we never got a usable subject, stop here
    if base_subject is None:
        return (errs, summary)

    participants = base_subject.get('participants') or []
    summary['participants'] = participants

    role_by_did: Dict[str, str] = {}
    if isinstance(participants, list):
        for p in participants:
            if not isinstance(p, dict):
                continue
            pid = str(p.get('id') or '').split('#', 1)[0]
            role = str(p.get('role') or '').strip()
            if pid and role:
                role_by_did[pid] = role

    # Expose the participant role mapping for downstream verifiers (e.g. receipt threshold enforcement).
    summary['role_by_did'] = {k: role_by_did[k] for k in sorted(role_by_did.keys())}

    # If corridor.yaml participants are specified (non-empty), require exact match with VC participants.
    corridor_participants = c.get('participants') or []
    if isinstance(corridor_participants, list) and corridor_participants:
        cset = {str(x).split('#', 1)[0] for x in corridor_participants if x}
        vset = set(role_by_did.keys())
        if cset != vset:
            missing = sorted(cset - vset)
            extra = sorted(vset - cset)
            if missing:
                errs.append(f"agreement: corridor.yaml participants not present in agreement VC participants: {missing}")
            if extra:
                errs.append(f"agreement: agreement VC participants not present in corridor.yaml participants: {extra}")

    # All valid signers MUST be listed as participants (keeps activation semantics unambiguous)
    for did in sorted(all_valid_signers):
        if role_by_did and did not in role_by_did:
            errs.append(f"agreement: signer {did} is not listed in credentialSubject.participants")

    # If party fields are present, ensure party role matches participant list
    for rel, party_id in party_by_vc.items():
        if not party_id:
            continue
        expected_role = role_by_did.get(party_id)
        declared_role = party_role_by_vc.get(rel, '')
        if expected_role and declared_role and expected_role != declared_role:
            errs.append(
                f"{rel}: party role mismatch for {party_id} (party.role={declared_role} vs participants.role={expected_role})"
            )
        if role_by_did and party_id not in role_by_did:
            errs.append(f"{rel}: party {party_id} is not listed in credentialSubject.participants")

    activation = base_subject.get('activation') or {}
    thresholds = []
    accept_commitments = ['agree']
    if isinstance(activation, dict):
        thresholds = activation.get('thresholds') or []
        ac = activation.get('accept_commitments')
        if isinstance(ac, list):
            norm = [str(x).strip().lower() for x in ac if str(x).strip()]
            if norm:
                accept_commitments = norm

    if not isinstance(thresholds, list) or not thresholds:
        errs.append('agreement: missing activation.thresholds')
        return (errs, summary)

    # v0.4.14+: receipt signing thresholds (fork resistance)
    #
    # Receipt signing policy lives in credentialSubject.state_channel.receipt_signing.
    # If absent, we fall back to activation.thresholds (backward compatible) but verifiers
    # SHOULD prefer unanimity for bilateral corridors (2-of-2) to prevent valid forks.
    summary['activation_thresholds'] = thresholds
    rs_obj = None
    if isinstance(base_subject.get('state_channel'), dict):
        rs_obj = (base_subject.get('state_channel') or {}).get('receipt_signing')
    if rs_obj is None:
        rs_obj = base_subject.get('receipt_signing')

    rs_thresholds: List[Dict[str, Any]] = []
    if isinstance(rs_obj, dict):
        t = rs_obj.get('thresholds')
        if isinstance(t, list):
            rs_thresholds = [x for x in t if isinstance(x, dict)]
    if not rs_thresholds:
        rs_thresholds = [x for x in thresholds if isinstance(x, dict)]
    summary['receipt_signing_thresholds'] = rs_thresholds

    # v0.4.15+: checkpoint finality + sync policy
    ck_obj = None
    ck_policy: Dict[str, Any] = {}
    if isinstance(base_subject.get('state_channel'), dict):
        ck_obj = (base_subject.get('state_channel') or {}).get('checkpointing')
    if isinstance(ck_obj, dict):
        ck_policy = {k: v for k, v in ck_obj.items()}

    ck_thresholds: List[Dict[str, Any]] = []
    if isinstance(ck_obj, dict):
        t = ck_obj.get('thresholds')
        if isinstance(t, list):
            ck_thresholds = [x for x in t if isinstance(x, dict)]

    # Default to receipt signing thresholds if no explicit checkpoint thresholds.
    if not ck_thresholds:
        ck_thresholds = list(rs_thresholds)

    summary['checkpointing_policy'] = ck_policy
    summary['checkpoint_signing_thresholds'] = ck_thresholds

    accept_set = set(accept_commitments)

    # Commitment-aware signer set (what counts toward thresholds)
    signed_parties: set[str] = set()
    blocked_parties: List[Dict[str, Any]] = []

    if (agreement_pattern or '') == 'party-specific':
        # Any non-affirmative party commitment blocks activation (even if thresholds would be satisfied).
        for party_id, comm in sorted(party_commitment_by_party.items()):
            if comm not in accept_set:
                blocked_parties.append(
                    {'id': party_id, 'commitment': comm, 'path': party_path_by_party.get(party_id, '')}
                )
                errs.append(
                    f"agreement: party {party_id} commitment '{comm}' blocks activation (accept_commitments={accept_commitments})"
                )
        # Count only affirmative commitments toward thresholds.
        for did in signed_parties_all:
            comm = party_commitment_by_party.get(did, 'agree')
            if comm in accept_set:
                signed_parties.add(did)

        summary['party_commitments'] = {k: v for k, v in sorted(party_commitment_by_party.items())}
    else:
        # Multi-signer agreement: treat subject.commitment as global (default 'agree').
        global_comm = str(base_subject.get('commitment') or '').strip().lower() or 'agree'
        summary['commitment'] = global_comm
        if global_comm not in accept_set:
            errs.append(
                f"agreement: commitment '{global_comm}' blocks activation (accept_commitments={accept_commitments})"
            )
        else:
            signed_parties = set(signed_parties_all)

    summary['blocked_parties'] = blocked_parties

    # Threshold evaluation
    threshold_summaries: List[Dict[str, Any]] = []
    activated = True

    for t in thresholds:
        if not isinstance(t, dict):
            continue
        role = str(t.get('role') or '').strip()
        required = t.get('required')
        of_val = t.get('of') if 'of' in t else None

        try:
            req_n = int(required)
        except Exception:
            errs.append(f"agreement: invalid threshold.required for role '{role}'")
            activated = False
            continue

        participants_with_role = [d for d, r in role_by_did.items() if r == role]
        signed_with_role = [d for d in signed_parties if role_by_did.get(d) == role]

        if of_val is not None:
            try:
                of_n = int(of_val)
                if of_n != len(participants_with_role):
                    errs.append(
                        f"agreement: threshold 'of' mismatch for role '{role}' (of={of_n} vs participants={len(participants_with_role)})"
                    )
                    activated = False
            except Exception:
                errs.append(f"agreement: invalid threshold.of for role '{role}'")
                activated = False

        if len(signed_with_role) < req_n:
            errs.append(
                f"agreement: activation threshold not met for role '{role}' ({len(signed_with_role)} signed, requires {req_n})"
            )
            activated = False

        threshold_summaries.append(
            {
                'role': role,
                'required': req_n,
                'of': int(of_val) if of_val is not None else None,
                'participants': len(participants_with_role),
                'signed': len(signed_with_role),
                'satisfied': len(signed_with_role) >= req_n,
            }
        )

    summary['signed_parties'] = sorted(signed_parties)
    summary['thresholds'] = threshold_summaries

    signed_by_role: Dict[str, List[str]] = {}
    for did in signed_parties:
        role = role_by_did.get(did)
        if role:
            signed_by_role.setdefault(role, []).append(did)
    summary['signed_by_role'] = {r: sorted(dids) for r, dids in signed_by_role.items()}

    summary['pinned_lawpacks_by_party'] = pinned_lawpacks_by_party

    # Lawpack compatibility enforcement (v0.4.1+)
    if isinstance(def_lawpack_compat, dict):
        req_domains = def_lawpack_compat.get("required_domains") or []
        allowed_list = def_lawpack_compat.get("allowed") or []
        allowed_map: Dict[Tuple[str, str], set] = {}

        if isinstance(allowed_list, list):
            for ent in allowed_list:
                if not isinstance(ent, dict):
                    continue
                jid = str(ent.get("jurisdiction_id") or "").strip()
                dom = str(ent.get("domain") or "").strip()
                digests = ent.get("digests_sha256") or []
                if jid and dom and isinstance(digests, list):
                    allowed_map[(jid, dom)] = set(str(d).strip() for d in digests if isinstance(d, str) and d.strip())

        # Required domain coverage (only meaningful for party-specific agreements)
        if isinstance(req_domains, list) and req_domains and agreement_pattern == "party-specific":
            for pid in sorted(role_by_did.keys()):
                pinned = pinned_lawpacks_by_party.get(pid, [])
                present = set()
                for lp in pinned:
                    if isinstance(lp, dict):
                        present.add(str(lp.get("domain") or "").strip())
                for dom in req_domains:
                    if str(dom) not in present:
                        errs.append(f"lawpacks: party {pid} missing pinned lawpack for required domain '{dom}'")

        # Allowlist enforcement (only if provided)
        if allowed_map:
            for pid, pinned in pinned_lawpacks_by_party.items():
                for lp in pinned:
                    if not isinstance(lp, dict):
                        continue
                    jid = str(lp.get("jurisdiction_id") or "").strip()
                    dom = str(lp.get("domain") or "").strip()
                    digest = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if not (jid and dom and digest):
                        continue
                    allowed = allowed_map.get((jid, dom))
                    if allowed is None:
                        errs.append(f"lawpacks: {pid} pinned {jid}/{dom} digest {digest} but definition has no allowlist entry for that jurisdiction/domain")
                    elif digest not in allowed:
                        errs.append(f"lawpacks: {pid} pinned {jid}/{dom} digest {digest} which is not in definition allowlist")


    summary['activated'] = bool(agreement_paths) and activated and (len(errs) == 0)

    return (errs, summary)


def verify_corridor_agreement_vc(module_dir: pathlib.Path) -> List[str]:
    """Verify Corridor Agreement VC(s) if configured.

    This is the strict CLI/test-facing validator wrapper around :func:`corridor_agreement_summary`.

    If `agreement_vc_path` is omitted, returns only corridor.yaml schema errors (if any).
    """

    errs, _summary = corridor_agreement_summary(module_dir)
    return errs



# --- Corridor state channels (v0.4.3+) ----------------------------------------

SHA256_HEX_RE = re.compile(r"^[a-f0-9]{64}$")


def _normalize_digest_set(values):
    """Normalize a digest list into a sorted unique list.

    This is used to ensure digest sets are stable (order-independent) prior to hashing.
    """
    out: List[str] = []
    seen = set()
    for v in (values or []):
        # Backwards compatible: accept either raw digest strings or ArtifactRef-like
        # objects carrying {digest_sha256: <hex>}.
        s = _coerce_sha256(v)
        if not s:
            continue
        if not SHA256_HEX_RE.match(s):
            raise ValueError(f"invalid sha256 digest: {v}")
        if s in seen:
            continue
        seen.add(s)
        out.append(s)
    out.sort()
    return out


def _load_rulesets_registry() -> dict:
    """Load registries/rulesets.yaml mapping ruleset_id -> descriptor path."""
    rp = REPO_ROOT / "registries" / "rulesets.yaml"
    if not rp.exists():
        return {}
    reg = load_yaml(rp) or {}
    mapping = {}
    for ent in (reg.get("rulesets") or []):
        if not isinstance(ent, dict):
            continue
        rid = str(ent.get("ruleset_id") or "").strip()
        path = str(ent.get("path") or "").strip()
        if rid and path:
            mapping[rid] = path
    return mapping


def ruleset_descriptor_digest_sha256(ruleset_id: str) -> str:
    """Compute the sha256 digest of a ruleset descriptor (content-addressed)."""
    rid = str(ruleset_id or "").strip()
    if not rid:
        raise ValueError("ruleset_id is required")

    reg = _load_rulesets_registry()
    rel = reg.get(rid)
    if not rel:
        raise ValueError(f"ruleset_id not found in registries/rulesets.yaml: {rid}")

    path = pathlib.Path(rel)
    if not path.is_absolute():
        path = REPO_ROOT / path
    if not path.exists():
        raise FileNotFoundError(f"ruleset descriptor missing: {rel}")

    data = load_json(path)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(data))


def corridor_expected_ruleset_digest_set(module_dir: pathlib.Path) -> List[str]:
    """Return the ruleset digest set that MUST govern corridor state receipts.

    By default this includes:
    - the corridor verification ruleset (corridor.yaml verification_ruleset)
    - the corridor state transition ruleset (corridor.yaml state_channel.transition_ruleset or default)
    """
    c = load_yaml(module_dir / "corridor.yaml")
    verification_ruleset = str(c.get("verification_ruleset") or "msez.corridor.verification.v1").strip()
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}
    transition_ruleset = str(state_cfg.get("transition_ruleset") or "msez.corridor.state-transition.v2").strip()

    digests = [
        ruleset_descriptor_digest_sha256(verification_ruleset),
        ruleset_descriptor_digest_sha256(transition_ruleset),
    ]
    return _normalize_digest_set(digests)


def corridor_expected_lawpack_digest_set(module_dir: pathlib.Path) -> List[str]:
    """Return the union of pinned lawpack digests from the activated agreement set.

    This is best-effort when no agreement is configured.
    """
    try:
        errs, summary = corridor_agreement_summary(module_dir)
        # If agreements exist but are invalid, keep deterministic behavior by still using what we can parse.
        pinned_by_party = (summary or {}).get("pinned_lawpacks_by_party") or {}
        digests = []
        if isinstance(pinned_by_party, dict):
            for _party, pinned in pinned_by_party.items():
                if not isinstance(pinned, list):
                    continue
                for lp in pinned:
                    if not isinstance(lp, dict):
                        continue
                    d = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if d:
                        digests.append(d)
        return _normalize_digest_set(digests)
    except Exception:
        return []


def _jcs_sha256_of_json_file(path: pathlib.Path) -> str:
    """Compute SHA256(JCS(json)) for a JSON file.

    Used for content-addressed digests where insignificant whitespace/ordering should not matter.
    """
    data = load_json(path)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(data))


def _resolve_path_repo_or_module(module_dir: pathlib.Path, rel: str) -> pathlib.Path:
    """Resolve a path that may be repo-relative or module-relative.

    Resolution order (v0.4.7+):
    1) absolute paths
    2) module-relative (to support overlays/bundles overriding shared names)
    3) repo-relative (shared artifacts like schemas/rulesets)

    This bias toward module-relative paths is intentional: it prevents accidental capture
    of a repo-root file when verifying a built bundle directory.
    """
    p = pathlib.Path(str(rel))
    if p.is_absolute():
        return p
    p_mod = module_dir / p
    if p_mod.exists():
        return p_mod
    p_repo = REPO_ROOT / p
    if p_repo.exists():
        return p_repo
    return p_mod


def _build_transition_type_registry_mapping(base_dir: pathlib.Path, reg_obj: Dict[str, Any], *, label: str) -> Dict[str, Dict[str, Any]]:
    """Build a `kind -> entry` mapping from a transition type registry object.

    Best-effort fills missing digest fields from referenced artifacts.

    Parameters:
      base_dir: directory to resolve module-relative paths (fallback when repo-relative does not exist)
      reg_obj: loaded registry object (already schema-validated)
      label: human label used in error messages
    """
    mapping: Dict[str, Dict[str, Any]] = {}
    for ent in (reg_obj.get("transition_types") or []):
        if not isinstance(ent, dict):
            continue
        kind = str(ent.get("kind") or "").strip()
        if not kind:
            continue
        if kind in mapping:
            raise ValueError(f"duplicate transition kind in registry ({label}): {kind}")

        # Copy and best-effort fill missing digests from referenced artifacts.
        e = dict(ent)

        # Schema digest: compute from schema_path when digest omitted.
        if not _coerce_sha256(e.get("schema_digest_sha256")):
            sp = str(e.get("schema_path") or "").strip()
            if sp:
                try:
                    s_path = _resolve_path_repo_or_module(base_dir, sp)
                    if s_path.exists():
                        e["schema_digest_sha256"] = _jcs_sha256_of_json_file(s_path)
                except Exception:
                    pass

        # Ruleset digest: prefer ruleset_id lookup; fall back to ruleset_path.
        if not _coerce_sha256(e.get("ruleset_digest_sha256")):
            rid = str(e.get("ruleset_id") or "").strip()
            if rid:
                try:
                    e["ruleset_digest_sha256"] = ruleset_descriptor_digest_sha256(rid)
                except Exception:
                    pass
            rp = str(e.get("ruleset_path") or "").strip()
            if rp and not _coerce_sha256(e.get("ruleset_digest_sha256")):
                try:
                    r_path = _resolve_path_repo_or_module(base_dir, rp)
                    if r_path.exists():
                        e["ruleset_digest_sha256"] = _jcs_sha256_of_json_file(r_path)
                except Exception:
                    pass

        # ZK circuit digest is not auto-computed unless a local path is provided (future-proof).

        mapping[kind] = e

    return mapping


def corridor_transition_type_registry(module_dir: pathlib.Path) -> Tuple[Optional[pathlib.Path], Dict[str, Any], Dict[str, Dict[str, Any]]]:
    """Load the corridor's transition type registry (if configured).

    The registry maps `transition.kind` -> optional digest references:
    - schema_digest_sha256 (payload format)
    - ruleset_digest_sha256 (validation semantics)
    - zk_circuit_digest_sha256 (proof-carrying transitions)

    The registry path is taken from corridor.yaml:
      state_channel.transition_type_registry_path

    Returns (registry_path, registry_object, mapping).
    """
    c = load_yaml(module_dir / "corridor.yaml")
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}
    rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
    if not rel:
        return None, {}, {}

    reg_path = _resolve_path_repo_or_module(module_dir, rel)
    if not reg_path.exists():
        raise FileNotFoundError(f"transition type registry not found: {rel}")

    reg_obj = load_yaml(reg_path) or {}
    reg_schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.registry.schema.json")
    verrs = validate_with_schema(reg_obj, reg_schema)
    if verrs:
        raise ValueError(f"transition type registry invalid ({rel}): {verrs[0]}")

    mapping = _build_transition_type_registry_mapping(module_dir, reg_obj, label=rel)
    return reg_path, reg_obj, mapping


def fill_transition_envelope_from_registry(env: Dict[str, Any], entry: Dict[str, Any]) -> Dict[str, Any]:
    """Fill a transition envelope's digest references from a registry entry.

    If the envelope already specifies a digest reference that disagrees with the registry entry,
    this function raises ValueError.
    """
    out = dict(env)
    for field in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
        expected = _coerce_sha256(entry.get(field))
        if not expected:
            continue
        actual = _coerce_sha256(out.get(field))
        if actual and actual != expected:
            raise ValueError(f"transition.{field} mismatch for kind '{out.get('kind')}'")
        out[field] = expected
    return out


def verify_transition_envelope_against_registry(
    env: Dict[str, Any],
    entry: Dict[str, Any],
    *,
    enforce: bool = False,
    allow_overrides: bool = False,
) -> List[str]:
    """Verify that a transition envelope's digest references are consistent with a registry entry.

    By default this is *non-strict*: if a registry defines a digest but the envelope omits it,
    verification passes. Set enforce=True to require presence.
    """
    errs: List[str] = []
    kind = str(env.get("kind") or "").strip()
    for field in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
        expected = _coerce_sha256(entry.get(field))
        actual = _coerce_sha256(env.get(field))
        if expected:
            if actual and actual != expected:
                if not allow_overrides:
                    errs.append(f"transition.{field} mismatch for kind '{kind}'")
            elif (not actual) and enforce:
                errs.append(f"transition.{field} missing for kind '{kind}' (required by registry)")
    return errs


TRANSITION_TYPES_SNAPSHOT_TAG = "msez.transition-types.registry.snapshot.v1"


def transition_type_registry_snapshot(registry_version: int, reg_map: Dict[str, Dict[str, Any]]) -> Dict[str, Any]:
    """Build the canonical transition-type registry snapshot used for content addressing.

    The snapshot intentionally commits only to semantics-relevant fields:
    - kind
    - schema_digest_sha256
    - ruleset_digest_sha256
    - zk_circuit_digest_sha256

    Any descriptive/provenance metadata should live outside the snapshot.
    """
    items: List[Dict[str, Any]] = []
    for kind in sorted(reg_map.keys()):
        e = reg_map[kind] or {}
        item: Dict[str, Any] = {"kind": kind}
        for f in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
            v = str(e.get(f) or "").strip().lower()
            if v:
                item[f] = v
        items.append(item)

    return {
        "tag": TRANSITION_TYPES_SNAPSHOT_TAG,
        "registry_version": int(registry_version or 1),
        "transition_types": items,
    }


def transition_type_registry_snapshot_digest(snapshot: Dict[str, Any]) -> str:
    """Compute SHA256(JCS(snapshot))."""
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(snapshot))


def build_transition_type_registry_lock(
    *,
    reg_path: pathlib.Path,
    reg_obj: Dict[str, Any],
    reg_map: Dict[str, Dict[str, Any]],
) -> Dict[str, Any]:
    """Build a transition type registry lock object.

    The lock contains:
    - provenance (source registry path + sha256 of canonicalized YAML)
    - snapshot (canonical semantics)
    - snapshot_digest_sha256 (content address)
    """
    from tools.lawpack import canonicalize_yaml  # type: ignore

    registry_version = int((reg_obj or {}).get("version") or 1)
    snapshot = transition_type_registry_snapshot(registry_version, reg_map)
    digest = transition_type_registry_snapshot_digest(snapshot)

    # Prefer a repo-relative path when possible (helps portability).
    try:
        rel = os.path.relpath(reg_path, REPO_ROOT)
    except Exception:
        rel = str(reg_path)

    lock_obj: Dict[str, Any] = {
        "transition_types_lock_version": 1,
        "generated_at": datetime.utcnow().replace(microsecond=0).isoformat() + "Z",
        "source": {
            "registry_path": rel,
            "registry_sha256": sha256_bytes(canonicalize_yaml(reg_path)),
        },
        "snapshot": snapshot,
        "snapshot_digest_sha256": digest,
    }
    return lock_obj


def load_transition_type_registry_lock(lock_path: pathlib.Path) -> Tuple[Dict[str, Any], Dict[str, Dict[str, Any]], str]:
    """Load and validate a transition type registry lock.

    Returns (lock_object, mapping, snapshot_digest_sha256).
    """
    lock_obj = load_json(lock_path) or {}
    schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.lock.schema.json")
    errs = validate_with_schema(lock_obj, schema)
    if errs:
        raise ValueError(f"transition type registry lock invalid ({lock_path}): {errs[0]}")

    snap = lock_obj.get("snapshot")
    if not isinstance(snap, dict):
        raise ValueError("transition type registry lock missing snapshot")
    if str(snap.get("tag") or "") != TRANSITION_TYPES_SNAPSHOT_TAG:
        raise ValueError("transition type registry lock snapshot.tag mismatch")

    digest = transition_type_registry_snapshot_digest(snap)
    expected = str(lock_obj.get("snapshot_digest_sha256") or "").strip().lower()
    if expected and digest != expected:
        raise ValueError("transition type registry lock snapshot_digest_sha256 mismatch")
    lock_obj["snapshot_digest_sha256"] = digest

    mapping: Dict[str, Dict[str, Any]] = {}
    for ent in (snap.get("transition_types") or []):
        if not isinstance(ent, dict):
            continue
        kind = str(ent.get("kind") or "").strip()
        if not kind:
            continue
        if kind in mapping:
            raise ValueError(f"duplicate transition kind in lock snapshot: {kind}")
        mapping[kind] = dict(ent)
    return lock_obj, mapping, digest



TRANSITION_TYPES_LOCK_CAS_SUFFIX = ".transition-types.lock.json"


def transition_types_lock_store_dirs(repo_root: pathlib.Path = REPO_ROOT) -> List[pathlib.Path]:
    """Return directories to search for content-addressed Transition Type Registry lock snapshots.

    Canonical CAS location (v0.4.7+):
      - dist/artifacts/transition-types

    The generic artifact store roots may be extended via:
      - MSEZ_ARTIFACT_STORE_DIRS (os.pathsep-separated)

    For backwards compatibility, this function also honors:
      - MSEZ_TRANSITION_TYPES_STORE_DIRS (type directories; legacy)

    Each store directory is expected to contain files named:

      <digest>.transition-types.lock.json

    where <digest> is the lock's snapshot_digest_sha256.
    """
    repo_root = repo_root.resolve()

    dirs: List[pathlib.Path] = []

    # Legacy override: directories that already point at transition-type lock files.
    legacy_env = (os.environ.get("MSEZ_TRANSITION_TYPES_STORE_DIRS") or "").strip()
    if legacy_env:
        for part in legacy_env.split(os.pathsep):
            p = part.strip()
            if not p:
                continue
            pp = pathlib.Path(p)
            if not pp.is_absolute():
                pp = repo_root / pp
            dirs.append(pp)

    # New generic store roots (dist/artifacts/*)
    for root in artifact_cas.artifact_store_roots(repo_root):
        # artifact_store_roots returns absolute paths.
        dirs.append(pathlib.Path(root) / "transition-types")

    # Legacy default (pre-v0.4.7)
    dirs.append(repo_root / "dist" / "registries" / "transition-types")

    # Deduplicate while preserving order.
    out: List[pathlib.Path] = []
    seen = set()
    for d in dirs:
        try:
            key = str(d.resolve())
        except Exception:
            key = str(d)
        if key in seen:
            continue
        seen.add(key)
        out.append(d)
    return out


def transition_types_lock_cas_path(
    digest_sha256: str,
    *,
    store_dir: Optional[pathlib.Path] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> pathlib.Path:
    """Return the canonical content-addressed path for a transition type registry lock snapshot.

    Canonical location (v0.4.7+):
      dist/artifacts/transition-types/<digest>.transition-types.lock.json

    ``store_dir`` may be used to override the *type directory*.
    """
    d = str(digest_sha256 or "").strip().lower()
    if not SHA256_HEX_RE.match(d):
        raise ValueError("digest must be 64 lowercase hex chars")

    base = store_dir or (repo_root / "dist" / "artifacts" / "transition-types")
    if not base.is_absolute():
        base = repo_root / base

    return base / f"{d}{TRANSITION_TYPES_LOCK_CAS_SUFFIX}"


def store_transition_type_registry_lock_to_cas(
    lock_path: pathlib.Path,
    *,
    store_dir: Optional[pathlib.Path] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> Tuple[pathlib.Path, str]:
    """Store a Transition Type Registry lock file into the content-addressed registry store.

    Returns (cas_path, snapshot_digest_sha256).
    """
    if not lock_path.exists():
        raise FileNotFoundError(f"lock not found: {lock_path}")

    lock_obj, _mapping, digest = load_transition_type_registry_lock(lock_path)

    cas_path = transition_types_lock_cas_path(digest, store_dir=store_dir, repo_root=repo_root)
    cas_path.parent.mkdir(parents=True, exist_ok=True)

    # Write a normalized JSON representation (stable formatting; digest is over snapshot, not file bytes).
    cas_path.write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return cas_path, digest


def resolve_transition_type_registry_lock_by_digest(
    digest_sha256: str,
    *,
    module_dir: Optional[pathlib.Path] = None,
    store_dirs: Optional[List[pathlib.Path]] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> pathlib.Path:
    """Resolve a Transition Type Registry lock snapshot by digest.

    Search order:
    1) content-addressed store(s) (default: dist/artifacts/transition-types)
    2) module-local conventional names (best-effort)
    3) repo fallback (registries/transition-types.lock.json) when it matches the digest
    """
    d = str(digest_sha256 or "").strip().lower()
    if not SHA256_HEX_RE.match(d):
        raise ValueError("digest must be 64 lowercase hex chars")

    dirs = store_dirs or transition_types_lock_store_dirs(repo_root)

    # 1) CAS store(s)
    for sd in dirs:
        sdp = sd
        if not sdp.is_absolute():
            sdp = repo_root / sdp
        cand = sdp / f"{d}{TRANSITION_TYPES_LOCK_CAS_SUFFIX}"
        if cand.exists():
            return cand

    # 2) module-local conventional names
    if module_dir is not None:
        for rel in ["transition-types.lock.json", "registries/transition-types.lock.json"]:
            cand = module_dir / rel
            if cand.exists():
                try:
                    _lock_obj, _mapping, dig = load_transition_type_registry_lock(cand)
                    if dig == d:
                        return cand
                except Exception:
                    pass

    # 3) repo fallback
    fallback = repo_root / "registries" / "transition-types.lock.json"
    if fallback.exists():
        try:
            _lock_obj, _mapping, dig = load_transition_type_registry_lock(fallback)
            if dig == d:
                return fallback
        except Exception:
            pass

    raise FileNotFoundError(f"transition type registry lock not found for digest {d}")


def corridor_transition_type_registry_snapshot(module_dir: pathlib.Path) -> Tuple[Optional[str], Dict[str, Dict[str, Any]]]:
    """Return (transition_type_registry_digest_sha256, mapping) best-effort.

    Preference order:
    1) transition_type_registry_lock_path (JSON lock)
    2) transition_type_registry_path (YAML registry -> derived snapshot)
    """
    c = load_yaml(module_dir / "corridor.yaml")
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}

    reg_rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
    lock_rel = str(state_cfg.get("transition_type_registry_lock_path") or "").strip()

    # Derive a default lock name when a registry is configured.
    if not lock_rel and reg_rel:
        rp = pathlib.Path(reg_rel)
        if rp.suffix.lower() in {".yaml", ".yml"}:
            lock_rel = str(rp.with_suffix(".lock.json"))
        else:
            lock_rel = reg_rel + ".lock.json"

    if lock_rel:
        lp = _resolve_path_repo_or_module(module_dir, lock_rel)
        if lp.exists():
            lock_obj, mapping, digest = load_transition_type_registry_lock(lp)
            return digest, mapping

    if reg_rel:
        _rp, reg_obj, mapping = corridor_transition_type_registry(module_dir)
        if mapping:
            registry_version = int((reg_obj or {}).get("version") or 1)
            snap = transition_type_registry_snapshot(registry_version, mapping)
            digest = transition_type_registry_snapshot_digest(snap)
            return digest, mapping

    return None, {}


def cmd_registry_transition_types_lock(args: argparse.Namespace) -> int:
    """Generate a content-addressed Transition Type Registry lock file.

    The lock captures a deterministic snapshot (kind -> digests) and emits:
    - snapshot (canonical JSON)
    - snapshot_digest_sha256 = sha256(JCS(snapshot))
    """
    reg_arg = str(getattr(args, "registry", "") or "").strip()
    if not reg_arg:
        print("registry path is required", file=sys.stderr)
        return 2

    reg_path = pathlib.Path(reg_arg)
    if not reg_path.is_absolute():
        reg_path = REPO_ROOT / reg_path
    if not reg_path.exists():
        print(f"Registry not found: {reg_path}", file=sys.stderr)
        return 2

    try:
        reg_obj = load_yaml(reg_path)
    except Exception as ex:
        print(f"ERROR: invalid YAML: {ex}", file=sys.stderr)
        return 2

    schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.registry.schema.json")
    errs = validate_with_schema(reg_obj, schema)
    if errs:
        for e in errs:
            print("  -", e, file=sys.stderr)
        return 2

    try:
        reg_map = _build_transition_type_registry_mapping(reg_path.parent, reg_obj, label=str(reg_path))
        lock_obj = build_transition_type_registry_lock(reg_path=reg_path, reg_obj=reg_obj, reg_map=reg_map)
    except Exception as ex:
        print(f"ERROR: unable to build transition type registry lock: {ex}", file=sys.stderr)
        return 2

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if reg_path.suffix.lower() in {".yaml", ".yml"}:
            out_path = reg_path.with_suffix(".lock.json")
        else:
            out_path = pathlib.Path(str(reg_path) + ".lock.json")

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # v0.4.6+: store lock snapshots; v0.4.7+: canonical store is dist/artifacts/transition-types
    cas_path: Optional[pathlib.Path] = None
    if not bool(getattr(args, 'no_store', False)):
        try:
            sd_arg = str(getattr(args, 'store_dir', '') or '').strip()
            sd_path: Optional[pathlib.Path] = pathlib.Path(sd_arg) if sd_arg else None
            if sd_path is not None and not sd_path.is_absolute():
                sd_path = REPO_ROOT / sd_path
            cas_path, _dig = store_transition_type_registry_lock_to_cas(out_path, store_dir=sd_path, repo_root=REPO_ROOT)
        except Exception as ex:
            print(f"WARN: unable to store registry lock snapshot in content-addressed store: {ex}", file=sys.stderr)
            cas_path = None

    if getattr(args, "json", False):
        print(
            json.dumps(
                {
                    "path": str(out_path),
                    "snapshot_digest_sha256": str(lock_obj.get("snapshot_digest_sha256") or ""),
                    "stored_path": str(cas_path) if cas_path is not None else "",
                },
                indent=2,
            )
        )
    else:
        print(str(out_path))
    return 0



def cmd_registry_transition_types_store(args: argparse.Namespace) -> int:
    """Store a transition type registry lock snapshot in the content-addressed registry store."""
    lock_arg = str(getattr(args, "lock", "") or "").strip()
    if not lock_arg:
        print("lock path is required", file=sys.stderr)
        return 2

    lock_path = pathlib.Path(lock_arg)
    if not lock_path.is_absolute():
        lock_path = REPO_ROOT / lock_path
    if not lock_path.exists():
        print(f"Lock not found: {lock_path}", file=sys.stderr)
        return 2

    sd_arg = str(getattr(args, "store_dir", "") or "").strip()
    sd_path: Optional[pathlib.Path] = pathlib.Path(sd_arg) if sd_arg else None
    if sd_path is not None and not sd_path.is_absolute():
        sd_path = REPO_ROOT / sd_path

    try:
        cas_path, digest = store_transition_type_registry_lock_to_cas(lock_path, store_dir=sd_path, repo_root=REPO_ROOT)
    except Exception as ex:
        print(f"ERROR: unable to store lock snapshot: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "json", False):
        print(json.dumps({"snapshot_digest_sha256": digest, "path": str(cas_path)}, indent=2))
    else:
        print(str(cas_path))
    return 0


def cmd_registry_transition_types_resolve(args: argparse.Namespace) -> int:
    """Resolve a transition type registry lock snapshot by digest."""
    dig = str(getattr(args, "digest", "") or "").strip().lower()
    if not dig:
        print("digest is required", file=sys.stderr)
        return 2

    # Start from default dirs (env + dist/...)
    dirs = transition_types_lock_store_dirs(REPO_ROOT)

    # Optional additional search dirs (searched first)
    extra = getattr(args, "store_dir", None) or []
    for sd in reversed(list(extra)):
        s = str(sd or "").strip()
        if not s:
            continue
        pth = pathlib.Path(s)
        if not pth.is_absolute():
            pth = REPO_ROOT / pth
        if pth not in dirs:
            dirs.insert(0, pth)

    try:
        lp = resolve_transition_type_registry_lock_by_digest(dig, module_dir=None, store_dirs=dirs, repo_root=REPO_ROOT)
    except Exception as ex:
        print(f"ERROR: unable to resolve lock snapshot: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "show", False):
        try:
            obj = load_json(lp)
            print(json.dumps(obj, indent=2, ensure_ascii=False))
        except Exception as ex:
            print(f"ERROR: unable to read lock at {lp}: {ex}", file=sys.stderr)
            return 2
        return 0

    if getattr(args, "json", False):
        # Validate by loading (also recomputes digest)
        try:
            _lock_obj, _mapping, digest = load_transition_type_registry_lock(lp)
        except Exception as ex:
            print(f"ERROR: invalid lock snapshot at {lp}: {ex}", file=sys.stderr)
            return 2
        print(json.dumps({"snapshot_digest_sha256": digest, "path": str(lp)}, indent=2))
    else:
        print(str(lp))
    return 0


def corridor_state_genesis_root(module_dir: pathlib.Path) -> str:
    """Compute the corridor genesis root binding the state channel to corridor substrate.

    genesis_root = SHA256(JCS({
      tag, corridor_id, definition_payload_sha256, agreement_set_sha256,
      lawpack_digest_set, ruleset_digest_set
    }))
    """
    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        raise FileNotFoundError("Missing corridor.yaml")
    c = load_yaml(corridor_path)
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        raise ValueError("corridor.yaml missing corridor_id")

    def_rel = str((c or {}).get("definition_vc_path") or "").strip()
    if not def_rel:
        raise ValueError("corridor.yaml missing definition_vc_path")
    def_path = module_dir / def_rel
    if not def_path.exists():
        raise FileNotFoundError(f"Missing Corridor Definition VC: {def_rel}")

    from tools.vc import signing_input  # type: ignore
    def_vcj = load_json(def_path)
    definition_payload_sha256 = sha256_bytes(signing_input(def_vcj))

    # Agreement set digest is optional (corridors may be single-operator). When no agreement is configured,
    # agreement_set_sha256 is set to "" and the genesis still binds to definition+digest-sets.
    agreement_set_sha256 = ""
    try:
        _errs, summary = corridor_agreement_summary(module_dir)
        agreement_set_sha256 = str((summary or {}).get("agreement_set_sha256") or "").strip()
    except Exception:
        agreement_set_sha256 = ""

    lawpack_digest_set = corridor_expected_lawpack_digest_set(module_dir)
    ruleset_digest_set = corridor_expected_ruleset_digest_set(module_dir)

    from tools.lawpack import jcs_canonicalize  # type: ignore
    payload = {
        "tag": "msez.corridor.state.genesis.v1",
        "corridor_id": corridor_id,
        "definition_payload_sha256": definition_payload_sha256,
        "agreement_set_sha256": agreement_set_sha256,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
    }
    return sha256_bytes(jcs_canonicalize(payload))


def corridor_state_next_root(receipt: Dict[str, Any]) -> str:
    """Compute next_root for a corridor receipt (excludes proof + next_root)."""
    if not isinstance(receipt, dict):
        raise ValueError("receipt must be an object")
    tmp = dict(receipt)
    tmp.pop("proof", None)
    tmp.pop("next_root", None)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(tmp))


def cmd_corridor_state_genesis_root(args: argparse.Namespace) -> int:
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    try:
        root = corridor_state_genesis_root(module_dir)
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "json", False):
        out = {
            "corridor": str(module_dir),
            "genesis_root": root,
        }
        print(json.dumps(out, indent=2))
    else:
        print(root)
    return 0


def _load_transition_envelope(path: str) -> Dict[str, Any]:
    """Load a transition input and coerce it into a typed transition envelope.

    v0.4.4+ RECOMMENDS a typed envelope of the form:

      {
        "type": "MSEZTransitionEnvelope",
        "kind": "...",
        "schema": "...",                    # optional (URI)
        "schema_digest_sha256": "<sha256>", # optional (payload schema)
        "ruleset_digest_sha256": "<sha256>",# optional (transition validation semantics)
        "zk_circuit_digest_sha256": "<sha256>", # optional (proof-carrying transitions)
        "payload": { ... },            # optional (may be omitted for privacy)
        "payload_sha256": "<sha256>",  # required when payload omitted
        "attachments": [...],          # optional
        "meta": {...}                  # optional
      }

    Backward compatible input forms:
    - empty path => noop envelope
    - legacy object with top-level 'kind' => treated as {kind, payload=<remaining fields>}
    - any other JSON value => treated as payload with kind='generic'
    """
    from tools.lawpack import jcs_canonicalize  # type: ignore

    if not path:
        payload: Any = {}
        payload_sha256 = sha256_bytes(jcs_canonicalize(payload))
        return {
            "type": "MSEZTransitionEnvelope",
            "kind": "noop",
            "payload": payload,
            "payload_sha256": payload_sha256,
        }

    pp = pathlib.Path(path)
    if not pp.is_absolute():
        pp = REPO_ROOT / pp
    if not pp.exists():
        raise FileNotFoundError(f"transition file not found: {path}")

    data = load_json(pp)

    # If already an envelope, normalize/complete it.
    if isinstance(data, dict) and str(data.get("type") or "") == "MSEZTransitionEnvelope":
        env = dict(data)
    elif isinstance(data, dict) and "kind" in data and "payload" in data:
        env = {"type": "MSEZTransitionEnvelope", **data}
    elif isinstance(data, dict) and "kind" in data:
        kind = str(data.get("kind") or "generic").strip() or "generic"
        payload = {k: v for (k, v) in data.items() if k != "kind"}
        env = {"type": "MSEZTransitionEnvelope", "kind": kind, "payload": payload}
    else:
        env = {"type": "MSEZTransitionEnvelope", "kind": "generic", "payload": data}

    kind = str(env.get("kind") or "").strip()
    if not kind:
        raise ValueError("transition envelope missing kind")

    # If payload is present, compute and (re)write payload_sha256.
    if "payload" in env:
        payload_sha256 = sha256_bytes(jcs_canonicalize(env.get("payload")))
        existing = str(env.get("payload_sha256") or "").strip().lower()
        if existing and existing != payload_sha256:
            raise ValueError("transition envelope payload_sha256 does not match computed SHA256(JCS(payload))")
        env["payload_sha256"] = payload_sha256
    else:
        # No payload embedded: require payload_sha256.
        ps = str(env.get("payload_sha256") or "").strip().lower()
        if not SHA256_HEX_RE.match(ps):
            raise ValueError("transition envelope must include payload_sha256 when payload is omitted")
        env["payload_sha256"] = ps

    env["type"] = "MSEZTransitionEnvelope"
    env["kind"] = kind
    return env


def cmd_corridor_state_receipt_init(args: argparse.Namespace) -> int:
    """Create a corridor state receipt and optionally sign it.

    By default:
    - prev_root is the corridor genesis_root
    - lawpack_digest_set is derived from the activated agreement-set (if present)
    - ruleset_digest_set is derived from the corridor verification + state-transition rulesets
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    # digest sets
    try:
        lp_set = getattr(args, "lawpack_digest", None) or []
        if lp_set:
            lawpack_digest_set = _normalize_digest_set(lp_set)
        else:
            lawpack_digest_set = corridor_expected_lawpack_digest_set(module_dir)

        rs_set = getattr(args, "ruleset_digest", None) or []
        if rs_set:
            ruleset_digest_set = _normalize_digest_set(rs_set)
        else:
            ruleset_digest_set = corridor_expected_ruleset_digest_set(module_dir)
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    # prev root
    prev_root_arg = str(getattr(args, "prev_root", "") or "").strip().lower()
    try:
        if not prev_root_arg or prev_root_arg in {"genesis", "genesis_root"}:
            prev_root = corridor_state_genesis_root(module_dir)
        else:
            if not SHA256_HEX_RE.match(prev_root_arg):
                raise ValueError("--prev-root must be 64 lowercase hex chars or 'genesis'")
            prev_root = prev_root_arg
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    try:
        seq = int(getattr(args, "sequence", 0))
        if seq < 0:
            raise ValueError("sequence must be >= 0")
    except Exception:
        print("--sequence must be a non-negative integer", file=sys.stderr)
        return 2

    ts = str(getattr(args, "timestamp", "") or "").strip()
    if not ts:
        ts = now_rfc3339()

    try:
        transition = _load_transition_envelope(str(getattr(args, "transition", "") or "").strip())
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    # Optional Transition Type Registry (v0.4.4+) and Registry Lock (v0.4.5+):
    # If corridor.yaml configures a registry and/or lock, receipts MAY commit to the registry snapshot digest
    # (transition_type_registry_digest_sha256). When this digest is present, receipts can avoid repeating
    # per-transition digest references.
    fill_transition_digests = bool(getattr(args, "fill_transition_digests", False))
    ttr_digest: str = ""
    ttr_map: Dict[str, Dict[str, Any]] = {}
    try:
        _d, _m = corridor_transition_type_registry_snapshot(module_dir)
        ttr_digest = str(_d or "").strip().lower()
        ttr_map = _m or {}
        if ttr_map:
            kind = str(transition.get("kind") or "").strip()
            if kind and kind in ttr_map and fill_transition_digests:
                transition = fill_transition_envelope_from_registry(transition, ttr_map[kind])
                # If the transition references a ruleset digest, ensure it is included in the receipt-level ruleset_digest_set.
                trd = _coerce_sha256(transition.get("ruleset_digest_sha256"))
                if trd:
                    ruleset_digest_set = _normalize_digest_set(list(ruleset_digest_set) + [trd])
    except Exception as ex:
        print(f"ERROR: transition type registry: {ex}", file=sys.stderr)
        return 2

    receipt: Dict[str, Any] = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": seq,
        "timestamp": ts,
        "prev_root": prev_root,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "transition": transition,
    }

    # Bind the receipt to the transition type registry snapshot (single commitment) when configured.
    if ttr_digest:
        receipt["transition_type_registry_digest_sha256"] = ttr_digest

    # Compute next_root deterministically and attach
    try:
        receipt["next_root"] = corridor_state_next_root(receipt)
    except Exception as ex:
        print(f"ERROR: unable to compute next_root: {ex}", file=sys.stderr)
        return 2

    # Optional signing
    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)

        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            vm = f"{did}#key-1"
        add_ed25519_proof(receipt, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        out_path = pathlib.Path(f"corridor-receipt.{seq}.json")
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def _collect_receipt_paths(path: pathlib.Path) -> List[pathlib.Path]:
    if path.is_dir():
        return sorted([p for p in path.glob("*.json") if p.is_file()])
    return [path]



def _proofs_to_list(proof_val: Any) -> List[Dict[str, Any]]:
    """Normalize a Linked Data Proof field to a list."""
    if proof_val is None:
        return []
    if isinstance(proof_val, list):
        return [p for p in proof_val if isinstance(p, dict)]
    if isinstance(proof_val, dict):
        return [proof_val]
    return []


def _merge_proofs(receipt_obj: Dict[str, Any], other_proof_val: Any) -> None:
    """Merge proofs from another receipt instance into receipt_obj in-place."""
    base = _proofs_to_list(receipt_obj.get("proof"))
    addl = _proofs_to_list(other_proof_val)
    seen: Set[Tuple[str, str]] = set()
    merged: List[Dict[str, Any]] = []
    for p in base + addl:
        vm = str(p.get("verificationMethod") or "")
        jws = str(p.get("jws") or "")
        key = (vm, jws)
        if key in seen:
            continue
        seen.add(key)
        merged.append(p)
    if not merged:
        return
    # Preserve legacy "proof": {..} shape when there's only one proof.
    receipt_obj["proof"] = merged[0] if len(merged) == 1 else merged


def _corridor_id_from_module(module_dir: pathlib.Path) -> str:
    corridor_yaml = module_dir / "corridor.yaml"
    try:
        data = load_yaml(corridor_yaml)
        cid = str(data.get("corridor_id") or "").strip()
        if cid:
            return cid
    except Exception:
        pass
    return module_dir.name


def _load_fork_resolution_map(
    fork_resolutions_path: Optional[pathlib.Path],
    *,
    expected_corridor_id: Optional[str] = None,
) -> Tuple[Dict[Tuple[int, str], str], List[str]]:
    """Load fork-resolution artifacts (raw or VC-wrapped) into a map.

    Returns:
      (resolution_map, errors)
      where resolution_map[(sequence, prev_root)] = chosen_next_root
    """
    res_map: Dict[Tuple[int, str], str] = {}
    errors: List[str] = []
    if not fork_resolutions_path:
        return res_map, errors

    p = fork_resolutions_path
    paths: List[pathlib.Path] = []
    if p.is_dir():
        paths = sorted([x for x in p.glob("*.json") if x.is_file()])
    else:
        paths = [p]

    for fp in paths:
        try:
            obj = load_json(fp)
        except Exception as e:
            errors.append(f"fork-resolution load failed: {fp}: {e}")
            continue

        subj = None
        if isinstance(obj, dict) and obj.get("type") == "MSEZCorridorForkResolution":
            subj = obj
        elif isinstance(obj, dict) and isinstance(obj.get("credentialSubject"), dict):
            subj = obj.get("credentialSubject")
        else:
            errors.append(f"fork-resolution unsupported shape (expected raw or VC): {fp}")
            continue

        cid = str(subj.get("corridor_id") or "").strip()
        if expected_corridor_id and cid and cid != expected_corridor_id:
            errors.append(
                f"fork-resolution corridor_id mismatch: {fp}: expected {expected_corridor_id}, got {cid}"
            )

        try:
            seq = int(subj.get("sequence"))
        except Exception:
            errors.append(f"fork-resolution missing/invalid sequence: {fp}")
            continue

        prev_root = str(subj.get("prev_root") or "").strip()
        chosen_next_root = str(subj.get("chosen_next_root") or "").strip()
        if not prev_root or not chosen_next_root:
            errors.append(f"fork-resolution missing prev_root/ chosen_next_root: {fp}")
            continue

        key = (seq, prev_root)
        if key in res_map and res_map[key] != chosen_next_root:
            errors.append(
                f"fork-resolution duplicate conflict for (seq={seq}, prev_root={prev_root}): {fp} chooses {chosen_next_root}, previously {res_map[key]}"
            )
            continue
        res_map[key] = chosen_next_root

    return res_map, errors


def _load_fork_resolution_map_with_sources(
    fork_resolutions_path: Optional[pathlib.Path],
    *,
    expected_corridor_id: Optional[str] = None,
) -> Tuple[Dict[Tuple[int, str], str], Dict[Tuple[int, str], List[str]], List[str]]:
    """Load fork-resolution artifacts and retain source paths.

    This is useful for *forensics* tooling (e.g., fork-inspect) where the operator
    wants to see which artifact(s) claimed a specific resolution.

    Returns:
      (resolution_map, sources_map, errors)
      where:
        resolution_map[(sequence, prev_root)] = chosen_next_root
        sources_map[(sequence, prev_root)] = ["/path/to/vc.json", ...]
    """
    res_map: Dict[Tuple[int, str], str] = {}
    sources: Dict[Tuple[int, str], List[str]] = {}
    errors: List[str] = []
    if not fork_resolutions_path:
        return res_map, sources, errors

    p = fork_resolutions_path
    paths: List[pathlib.Path] = []
    if p.is_dir():
        paths = sorted([x for x in p.glob("*.json") if x.is_file()])
    else:
        paths = [p]

    for fp in paths:
        try:
            obj = load_json(fp)
        except Exception as e:
            errors.append(f"fork-resolution load failed: {fp}: {e}")
            continue

        subj = None
        if isinstance(obj, dict) and obj.get("type") == "MSEZCorridorForkResolution":
            subj = obj
        elif isinstance(obj, dict) and isinstance(obj.get("credentialSubject"), dict):
            subj = obj.get("credentialSubject")
        else:
            errors.append(f"fork-resolution unsupported shape (expected raw or VC): {fp}")
            continue

        cid = str(subj.get("corridor_id") or "").strip()
        if expected_corridor_id and cid and cid != expected_corridor_id:
            errors.append(
                f"fork-resolution corridor_id mismatch: {fp}: expected {expected_corridor_id}, got {cid}"
            )

        try:
            seq = int(subj.get("sequence"))
        except Exception:
            errors.append(f"fork-resolution missing/invalid sequence: {fp}")
            continue

        prev_root = str(subj.get("prev_root") or "").strip()
        chosen_next_root = str(subj.get("chosen_next_root") or "").strip()
        if not prev_root or not chosen_next_root:
            errors.append(f"fork-resolution missing prev_root/ chosen_next_root: {fp}")
            continue

        key = (seq, prev_root)
        if key in res_map and res_map[key] != chosen_next_root:
            errors.append(
                f"fork-resolution duplicate conflict for (seq={seq}, prev_root={prev_root}): {fp} chooses {chosen_next_root}, previously {res_map[key]}"
            )
            continue

        res_map[key] = chosen_next_root
        sources.setdefault(key, []).append(str(fp))

    return res_map, sources, errors


def _group_receipt_candidates(
    receipt_rows: List[Tuple[pathlib.Path, Dict[str, Any], Set[str]]]
) -> Dict[Tuple[int, str, str], Dict[str, Any]]:
    """Group duplicate receipts (same seq, prev_root, next_root) and merge proofs/signers."""
    grouped: Dict[Tuple[int, str, str], Dict[str, Any]] = {}
    for rp, receipt, ok_dids in receipt_rows:
        seq = int(receipt.get("sequence"))
        prev_root = str(receipt.get("prev_root"))
        next_root = str(receipt.get("next_root"))
        key = (seq, prev_root, next_root)

        if key not in grouped:
            grouped[key] = {
                "receipt": dict(receipt),
                "signers": set(ok_dids),
                "paths": [str(rp)],
            }
        else:
            grouped[key]["paths"].append(str(rp))
            grouped[key]["signers"].update(ok_dids)
            _merge_proofs(grouped[key]["receipt"], receipt.get("proof"))
    return grouped


def _select_canonical_chain(
    candidates_by_seq_prev: Dict[Tuple[int, str], List[Dict[str, Any]]],
    *,
    start_seq: int,
    start_prev_root: str,
    fork_resolution_map: Optional[Dict[Tuple[int, str], str]] = None,
) -> Tuple[List[Dict[str, Any]], List[str], List[str]]:
    """Select a canonical chain of receipts, resolving forks using fork_resolution_map."""
    chain: List[Dict[str, Any]] = []
    warnings: List[str] = []
    errors: List[str] = []

    expected_seq = start_seq
    expected_prev = start_prev_root

    while True:
        cands = candidates_by_seq_prev.get((expected_seq, expected_prev), [])
        if not cands:
            break

        if len(cands) == 1:
            chosen = cands[0]
        else:
            # Fork at (seq, prev_root): choose by resolution artifact.
            chosen_next = None
            if fork_resolution_map is not None:
                chosen_next = fork_resolution_map.get((expected_seq, expected_prev))
            if not chosen_next:
                errors.append(
                    f"fork detected at sequence {expected_seq} prev_root={expected_prev} (candidates={len(cands)}). Provide --fork-resolutions to select the canonical receipt."
                )
                break
            matches = [c for c in cands if str(c['receipt'].get('next_root')) == chosen_next]
            if len(matches) != 1:
                errors.append(
                    f"fork-resolution selects next_root={chosen_next} for (seq={expected_seq}, prev_root={expected_prev}) but it was not found among candidates: {[str(c['receipt'].get('next_root')) for c in cands]}"
                )
                break
            chosen = matches[0]
            warnings.append(
                f"fork resolved at seq={expected_seq} prev_root={expected_prev} -> next_root={chosen_next}"
            )

        chain.append(chosen["receipt"])
        expected_prev = str(chosen["receipt"].get("next_root"))
        expected_seq += 1

    # Post-condition sanity: if there are receipts at the expected sequence but with a different prev_root,
    # flag as a warning (unreachable branch).
    orphan_same_seq = [
        k
        for k in candidates_by_seq_prev.keys()
        if k[0] == expected_seq and k[1] != expected_prev
    ]
    if orphan_same_seq:
        warnings.append(
            f"unreachable receipts exist at sequence {expected_seq} with non-matching prev_root values (possible alternate fork branches): {sorted(list({k[1] for k in orphan_same_seq}))}"
        )

    return chain, warnings, errors


def _corridor_state_build_chain(
    module_dir: pathlib.Path,
    receipts_path: pathlib.Path,
    *,
    enforce_trust_anchors: bool = False,
    enforce_receipt_threshold: bool = False,
    enforce_checkpoint_policy: bool = False,
    enforce_transition_types: bool = False,
    require_artifacts: bool = False,
    transitive_require_artifacts: bool = False,
    fork_resolutions_path: Optional[pathlib.Path] = None,
    from_checkpoint_path: Optional[pathlib.Path] = None,
) -> Tuple[Dict[str, Any], List[str], List[str]]:
    """Core corridor receipt verification and canonicalization (fork-aware).

    Returns:
      (result, warnings, errors)
    where result contains:
      corridor_id, genesis_root, receipts (selected canonical receipts), receipt_count, final_state_root, mmr
    """
    warnings: List[str] = []
    errors: List[str] = []

    corridor_id = _corridor_id_from_module(module_dir)

    receipt_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.receipt.schema.json")
    checkpoint_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.checkpoint.schema.json")

    # Corridor invariants
    expected_genesis_root = corridor_state_genesis_root(module_dir)

    expected_ruleset_set: List[str] = []
    expected_lawpack_set: List[str] = []
    try:
        expected_ruleset_set = corridor_expected_ruleset_digest_set(module_dir)
    except Exception as e:
        errors.append(f"failed to load expected ruleset_digest_set: {e}")
    try:
        expected_lawpack_set = corridor_expected_lawpack_digest_set(module_dir)
    except Exception:
        expected_lawpack_set = []

    # Transition type registry snapshot (optional but strongly recommended).
    corridor_ttr_digest, _corridor_ttr = corridor_transition_type_registry_snapshot(module_dir)

    # Trust anchors (optional enforcement)
    allowed_receipt_signers: Set[str] = set()
    if enforce_trust_anchors:
        try:
            c_cfg = load_yaml(module_dir / "corridor.yaml")
            ta_rel = str((c_cfg or {}).get("trust_anchors_path") or "trust-anchors.yaml")
        except Exception:
            ta_rel = "trust-anchors.yaml"

        trust_anchors = load_trust_anchors(module_dir / ta_rel)
        allowed_receipt_signers = set(
            ta["did"]
            for ta in trust_anchors
            if "corridor_receipt" in (ta.get("allowed_attestations") or [])
            or "*" in (ta.get("allowed_attestations") or [])
        )
        if not allowed_receipt_signers:
            errors.append(
                f"--enforce-trust-anchors was set but {ta_rel} contains no anchors permitting corridor_receipt"
            )

    # Optional receipt threshold policy.
    receipt_thresholds: List[Dict[str, Any]] = []
    checkpoint_thresholds: List[Dict[str, Any]] = []
    checkpointing_policy: Dict[str, Any] = {}
    role_by_did: Dict[str, str] = {}
    if enforce_receipt_threshold or enforce_checkpoint_policy:
        agreement_errs, agreement_summary = corridor_agreement_summary(module_dir)
        errors.extend(agreement_errs)
        receipt_thresholds = agreement_summary.get("receipt_signing_thresholds") or []
        checkpoint_thresholds = agreement_summary.get("checkpoint_signing_thresholds") or []
        checkpointing_policy = agreement_summary.get("checkpointing_policy") or {}
        role_by_did = agreement_summary.get("role_by_did") or {}

    # Fork resolution map (optional)
    fork_map, fork_errs = _load_fork_resolution_map(
        fork_resolutions_path, expected_corridor_id=corridor_id
    )
    errors.extend(fork_errs)

    # Artifact resolver (for --require-artifacts)
    import tools.artifacts as artifact_cas  # local import to keep startup fast

    def _require_artifact(artifact_type: str, digest: str, label: str) -> None:
        if not digest:
            return
        try:
            artifact_cas.resolve_artifact_by_digest(
                artifact_type, digest, repo_root=REPO_ROOT
            )
        except FileNotFoundError:
            errors.append(f"missing required artifact {label}: {artifact_type}:{digest}")
        except Exception as e:
            errors.append(f"artifact resolve error for {label}: {artifact_type}:{digest}: {e}")

    # Load + validate receipts
    receipt_rows: List[Tuple[pathlib.Path, Dict[str, Any], Set[str]]] = []
    receipt_paths = _collect_receipt_paths(receipts_path)
    if not receipt_paths:
        errors.append(f"no receipts found under: {receipts_path}")
        return {}, warnings, errors

    # Preload transition-type-registry lock (if present)
    ttr_lock_path = module_dir / "transition-type-registry.lock.json"
    locked_ttr_digest: Optional[str] = None
    if ttr_lock_path.exists():
        try:
            lock_obj = load_json(ttr_lock_path)
            locked_ttr_digest = str(lock_obj.get("digest_sha256") or "")
        except Exception as e:
            errors.append(f"failed to load transition-type-registry.lock.json: {e}")

    # Cache to avoid re-scanning the same registry lock multiple times when
    # verifying many receipts.
    transitive_checked_ttr_digests: set[str] = set()

    for rp in receipt_paths:
        try:
            receipt = load_json(rp)
        except Exception as e:
            errors.append(f"failed to load receipt {rp}: {e}")
            continue

        # Schema check
        ve = list(receipt_validator.iter_errors(receipt))
        if ve:
            errors.append(f"invalid receipt schema: {rp}: {ve[0].message}")
            continue

        # next_root integrity
        computed_next = corridor_state_next_root(receipt)
        if receipt.get("next_root") != computed_next:
            errors.append(
                f"next_root mismatch: {rp}: declared {receipt.get('next_root')} computed {computed_next}"
            )
            continue

        # Expected digest sets (ArtifactRef-aware)
        if expected_ruleset_set:
            declared_rulesets_raw = receipt.get("ruleset_digest_set") or []
            declared_rulesets = _normalize_digest_set(declared_rulesets_raw)
            missing = [x for x in expected_ruleset_set if x not in declared_rulesets]
            if missing:
                errors.append(f"ruleset_digest_set missing expected entries {missing}: {rp}")
                continue
        if expected_lawpack_set:
            declared_lawpacks_raw = receipt.get("lawpack_digest_set") or []
            declared_lawpacks = _normalize_digest_set(declared_lawpacks_raw)
            if set(declared_lawpacks) != set(expected_lawpack_set):
                errors.append(
                    f"lawpack_digest_set mismatch: {rp}: expected {expected_lawpack_set} got {declared_lawpacks}"
                )
                continue

        # Transition-type-registry binding checks (optional enforcement)
        if enforce_transition_types:
            receipt_ttr_digest = _coerce_sha256(receipt.get("transition_type_registry_digest_sha256"))
            if locked_ttr_digest:
                if receipt_ttr_digest and receipt_ttr_digest != locked_ttr_digest:
                    errors.append(
                        f"receipt transition_type_registry_digest_sha256 does not match lock digest: {rp}"
                    )
                    continue
            else:
                if receipt_ttr_digest and corridor_ttr_digest and receipt_ttr_digest != corridor_ttr_digest:
                    errors.append(
                        f"receipt transition_type_registry_digest_sha256 does not match corridor registry digest: {rp}"
                    )
                    continue

        # Require artifacts (best-effort coverage)
        if require_artifacts:
            # digest sets may contain raw digest strings OR ArtifactRefs.
            for entry in receipt.get("lawpack_digest_set") or []:
                dd = _coerce_sha256(entry)
                if not dd:
                    continue
                at = "lawpack"
                if isinstance(entry, dict) and entry.get("artifact_type"):
                    at = str(entry.get("artifact_type") or at)
                _require_artifact(at, dd, f"lawpack_digest_set[{dd}]")
            for entry in receipt.get("ruleset_digest_set") or []:
                dd = _coerce_sha256(entry)
                if not dd:
                    continue
                at = "ruleset"
                if isinstance(entry, dict) and entry.get("artifact_type"):
                    at = str(entry.get("artifact_type") or at)
                _require_artifact(at, dd, f"ruleset_digest_set[{dd}]")
            # Transition type registry snapshot: in transitive mode we treat the registry digest
            # as a *commitment root* and ensure all referenced schema/ruleset/circuit digests are
            # present in CAS.
            ttr_entry = receipt.get("transition_type_registry_digest_sha256")
            ttr_dd = _coerce_sha256(ttr_entry)
            if ttr_dd:
                if isinstance(ttr_entry, dict) and ttr_entry.get("artifact_type"):
                    at = str(ttr_entry.get("artifact_type") or "transition-types")
                    if at and at != "transition-types":
                        errors.append(
                            f"transition_type_registry_digest_sha256 ArtifactRef has artifact_type={at} (expected transition-types): {rp}"
                        )
                if not transitive_require_artifacts:
                    _require_artifact("transition-types", ttr_dd, "transition_type_registry_digest_sha256")

            if transitive_require_artifacts:
                effective = ttr_dd or (locked_ttr_digest or "") or (corridor_ttr_digest or "")
                if effective and effective not in transitive_checked_ttr_digests:
                    transitive_checked_ttr_digests.add(effective)
                    _require_transition_types_lock_transitive(
                        effective,
                        errors=errors,
                        label="transition_type_registry_digest_sha256 (transitive)",
                        repo_root=REPO_ROOT,
                    )

            # transition envelope artifacts
            env = receipt.get("transition") or {}
            for field, atype in [
                ("schema_digest_sha256", "schema"),
                ("ruleset_digest_sha256", "ruleset"),
                ("zk_circuit_digest_sha256", "circuit"),
            ]:
                val = env.get(field)
                if isinstance(val, str):
                    _require_artifact(atype, val, f"transition.{field}")
                elif isinstance(val, dict) and val.get("digest_sha256"):
                    _require_artifact(atype, str(val.get("digest_sha256")), f"transition.{field}.digest_sha256")

            for att in env.get("attachments") or []:
                # Preferred: typed ArtifactRef {artifact_type, digest_sha256, uri?, ...}
                if isinstance(att, dict):
                    dd = _coerce_sha256(att)
                    if not dd:
                        continue
                    at = str(att.get("artifact_type") or "blob")
                    _require_artifact(at, dd, f"transition.attachments[{at}]")
                    continue

                # Legacy form: raw digest string treated as a blob.
                if isinstance(att, str):
                    dd = att.strip().lower()
                    if dd:
                        _require_artifact("blob", dd, "transition.attachments[legacy]")

        # Signature verification (ProofResult-aware)
        from tools.vc import verify_credential

        vres = verify_credential(receipt)
        ok_dids: Set[str] = _verified_base_dids(vres)

        if not ok_dids:
            errors.append(f"receipt signature verification failed (no valid proofs): {rp}")
            continue

        if enforce_trust_anchors and allowed_receipt_signers:
            if not (ok_dids & allowed_receipt_signers):
                errors.append(
                    f"receipt signer not in trust anchors: {rp}: signers={sorted(ok_dids)}"
                )
                continue

        receipt_rows.append((rp, receipt, ok_dids))

    if errors:
        return {}, warnings, errors

    # Bootstrap from checkpoint (optional)
    start_seq = 0
    start_prev_root = expected_genesis_root
    base_receipt_count = 0
    base_checkpoint_peaks = None

    if from_checkpoint_path:
        try:
            ck = load_json(from_checkpoint_path)
        except Exception as e:
            errors.append(f"failed to load from-checkpoint: {from_checkpoint_path}: {e}")
            return {}, warnings, errors

        ve = list(checkpoint_validator.iter_errors(ck))
        if ve:
            errors.append(f"invalid from-checkpoint schema: {from_checkpoint_path}: {ve[0].message}")
            return {}, warnings, errors

        from tools.vc import verify_credential
        ck_v = verify_credential(ck)
        ck_ok_dids = _verified_base_dids(ck_v)
        if not ck_ok_dids:
            errors.append(f"from-checkpoint signature verification failed: {from_checkpoint_path}")
            return {}, warnings, errors

        if str(ck.get("corridor_id")) != corridor_id:
            errors.append(
                f"from-checkpoint corridor_id mismatch: expected {corridor_id} got {ck.get('corridor_id')}"
            )
            return {}, warnings, errors
        if str(ck.get("genesis_root")) != expected_genesis_root:
            errors.append(
                f"from-checkpoint genesis_root mismatch: expected {expected_genesis_root} got {ck.get('genesis_root')}"
            )
            return {}, warnings, errors

        base_receipt_count = int(ck.get("receipt_count") or 0)
        start_seq = base_receipt_count
        start_prev_root = str(ck.get("final_state_root"))

        # Prefer incremental MMR update if peaks are present.
        mmr_obj = ck.get("mmr") or {}
        base_checkpoint_peaks = mmr_obj.get("peaks")

    # Merge duplicates and build candidates by (seq, prev_root)
    grouped = _group_receipt_candidates(receipt_rows)

    candidates_by_seq_prev: Dict[Tuple[int, str], List[Dict[str, Any]]] = {}
    for (_seq, _prev, _next), cand in grouped.items():
        key = (_seq, _prev)
        candidates_by_seq_prev.setdefault(key, []).append(cand)

    # Apply receipt signing threshold policy at the logical-receipt level (post-merge).
    if enforce_receipt_threshold and receipt_thresholds:
        pruned: Dict[Tuple[int, str], List[Dict[str, Any]]] = {}
        for key, cand_list in candidates_by_seq_prev.items():
            kept: List[Dict[str, Any]] = []
            for cand in cand_list:
                signers = cand.get("signers") or set()
                ok = True
                for t in receipt_thresholds:
                    role = str(t.get("role") or "")
                    req = int(t.get("required") or 0)
                    if req <= 0:
                        continue
                    count = sum(1 for d in signers if role_by_did.get(d) == role)
                    if count < req:
                        ok = False
                        break
                if ok:
                    kept.append(cand)
            if kept:
                pruned[key] = kept
        candidates_by_seq_prev = pruned

    # Select canonical chain
    chain_receipts, chain_warnings, chain_errors = _select_canonical_chain(
        candidates_by_seq_prev,
        start_seq=start_seq,
        start_prev_root=start_prev_root,
        fork_resolution_map=fork_map if fork_map else None,
    )
    warnings.extend(chain_warnings)
    errors.extend(chain_errors)
    if errors:
        return {}, warnings, errors

    # Ensure the chain starts where expected.
    if not chain_receipts:
        if start_seq == 0:
            # We had receipts, but none could start at (0, genesis_root).
            has_seq0 = any(k[0] == 0 for k in candidates_by_seq_prev.keys())
            if has_seq0:
                errors.append(
                    f"no canonical chain starts at (sequence=0, prev_root=genesis_root={expected_genesis_root}); receipts may target a different genesis_root"
                )
            else:
                errors.append(
                    "receipt set does not include an initial receipt (sequence=0); cannot verify from genesis"
                )
            return {}, warnings, errors
        else:
            # Head remains at checkpoint iff there are no receipts at/after start_seq.
            has_after = any(k[0] >= start_seq for k in candidates_by_seq_prev.keys())
            if has_after:
                errors.append(
                    f"no receipts connect to from-checkpoint head at (sequence={start_seq}, prev_root={start_prev_root})"
                )
                return {}, warnings, errors

    # Warn about unreachable receipts beyond the selected head sequence.
    head_seq = start_seq + len(chain_receipts)
    unreachable = sorted({k for k in candidates_by_seq_prev.keys() if k[0] > head_seq})
    if unreachable:
        warnings.append(
            f"unreachable receipts exist beyond the selected head (head_seq={head_seq}); first few unreachable keys: {unreachable[:5]}"
        )

    # Compute MMR state. If from_checkpoint was provided but no peaks exist, attempt full recompute
    # if receipts cover sequence 0..head; otherwise fail.
    from tools.mmr import mmr_root_from_next_roots, append_peaks, bag_peaks, peaks_from_json, peaks_to_json

    total_receipt_count = base_receipt_count + len(chain_receipts)
    final_state_root = start_prev_root if not chain_receipts else str(chain_receipts[-1].get("next_root"))

    mmr_state: Optional[Dict[str, Any]] = None

    if from_checkpoint_path and base_receipt_count > 0:
        # Try full recompute if we have seq0 receipts.
        try_full = any(int(r.get("sequence")) == 0 for (_rp, r, _d) in receipt_rows)
        if try_full:
            full_chain, full_warn, full_err = _select_canonical_chain(
                candidates_by_seq_prev,
                start_seq=0,
                start_prev_root=expected_genesis_root,
                fork_resolution_map=fork_map if fork_map else None,
            )
            if not full_err and len(full_chain) >= total_receipt_count:
                full_next_roots = [str(r.get("next_root")) for r in full_chain[:total_receipt_count]]
                mmr_full = mmr_root_from_next_roots(full_next_roots)
                mmr_state = {
                    "type": "sha256-mmr",
                    "algorithm": "sha256",
                    "size": total_receipt_count,
                    "root": mmr_full.get("root"),
                    "peaks": mmr_full.get("peaks"),
                }
                warnings.extend([w for w in full_warn if w not in warnings])

        if mmr_state is None:
            if not base_checkpoint_peaks:
                errors.append(
                    "from-checkpoint does not include mmr.peaks and full receipt history was not available; cannot compute head MMR root"
                )
                return {}, warnings, errors
            try:
                peaks = peaks_from_json(base_checkpoint_peaks)
                # Append leaf hashes for the new receipts
                leaf_hashes = [str(r.get("next_root")) for r in chain_receipts]
                new_peaks = append_peaks(peaks, leaf_hashes)
                mmr_root = bag_peaks(new_peaks)
                mmr_state = {
                    "type": "sha256-mmr",
                    "algorithm": "sha256",
                    "size": total_receipt_count,
                    "root": mmr_root,
                    "peaks": peaks_to_json(new_peaks),
                }
            except Exception as e:
                errors.append(f"failed to compute incremental MMR from checkpoint peaks: {e}")
                return {}, warnings, errors

    if mmr_state is None:
        next_roots = [str(r.get("next_root")) for r in chain_receipts]
        mmr_full = mmr_root_from_next_roots(next_roots)
        mmr_state = {
            "type": "sha256-mmr",
            "algorithm": "sha256",
            "size": total_receipt_count,
            "root": mmr_full.get("root"),
            "peaks": mmr_full.get("peaks"),
        }

    result = {
        "corridor_id": corridor_id,
        "genesis_root": expected_genesis_root,
        "receipts": chain_receipts,
        "receipt_count": total_receipt_count,
        "final_state_root": final_state_root,
        "mmr": mmr_state,
        "transition_type_registry_digest_sha256": corridor_ttr_digest,
    }

    return result, warnings, errors


def cmd_corridor_state_verify(args: argparse.Namespace) -> int:
    """Verify a corridor receipt chain (fork-aware).

    Supports:
      - duplicate receipts (same payload, different proofs) via proof/signature union
      - forks (same (sequence, prev_root), different next_root) via --fork-resolutions
      - bootstrap from a signed checkpoint via --from-checkpoint
    """
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    receipts_path = pathlib.Path(args.receipts)

    fork_resolutions_path = getattr(args, "fork_resolutions", None)
    if fork_resolutions_path:
        fork_resolutions_path = pathlib.Path(fork_resolutions_path)

    from_checkpoint_path = getattr(args, "from_checkpoint", None)
    if from_checkpoint_path:
        from_checkpoint_path = pathlib.Path(from_checkpoint_path)

    enforce_trust = bool(getattr(args, "enforce_trust_anchors", False))
    enforce_transition_types = bool(getattr(args, "enforce_transition_types", False))
    require_artifacts = bool(getattr(args, "require_artifacts", False))
    transitive_require_artifacts = bool(getattr(args, "transitive_require_artifacts", False))
    if transitive_require_artifacts:
        require_artifacts = True
    enforce_receipt_threshold = bool(getattr(args, "enforce_receipt_threshold", False))
    enforce_checkpoint_policy = bool(getattr(args, "enforce_checkpoint_policy", False))

    result, warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_path,
        enforce_trust_anchors=enforce_trust,
        enforce_transition_types=enforce_transition_types,
        require_artifacts=require_artifacts,
        transitive_require_artifacts=transitive_require_artifacts,
        enforce_receipt_threshold=enforce_receipt_threshold,
        enforce_checkpoint_policy=enforce_checkpoint_policy,
        fork_resolutions_path=fork_resolutions_path,
        from_checkpoint_path=from_checkpoint_path,
    )

    # Optional head-checkpoint verification
    checkpoint_path = getattr(args, "checkpoint", None)
    if enforce_checkpoint_policy:
        agreement_errs, agreement_summary = corridor_agreement_summary(module_dir)
        errors.extend(agreement_errs)
        policy = agreement_summary.get("checkpointing_policy") or {}
        mode = str(policy.get("mode") or "").strip().lower()
        if mode == "required" and not checkpoint_path:
            errors.append(
                "--enforce-checkpoint-policy is set and corridor checkpointing policy mode=required, but no --checkpoint was provided"
            )

    checkpoint_verified = None
    if checkpoint_path and result and not errors:
        ck_path = pathlib.Path(checkpoint_path)
        checkpoint_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.checkpoint.schema.json")
        try:
            ck = load_json(ck_path)
        except Exception as e:
            errors.append(f"failed to load checkpoint: {ck_path}: {e}")
            ck = None

        if ck is not None:
            ve = list(checkpoint_validator.iter_errors(ck))
            if ve:
                errors.append(f"invalid checkpoint schema: {ck_path}: {ve[0].message}")
            else:
                from tools.vc import verify_credential
                ck_v = verify_credential(ck)
                ck_ok_dids = _verified_base_dids(ck_v)
                checkpoint_verified = bool(ck_ok_dids)
                if not checkpoint_verified:
                    errors.append(f"checkpoint signature verification failed: {ck_path}")

                # Content matching checks
                if str(ck.get("corridor_id")) != result.get("corridor_id"):
                    errors.append("checkpoint corridor_id mismatch")
                if str(ck.get("genesis_root")) != result.get("genesis_root"):
                    errors.append("checkpoint genesis_root mismatch")
                if int(ck.get("receipt_count") or 0) != int(result.get("receipt_count") or 0):
                    errors.append("checkpoint receipt_count mismatch")
                if str(ck.get("final_state_root")) != result.get("final_state_root"):
                    errors.append("checkpoint final_state_root mismatch")
                mmr = ck.get("mmr") or {}
                if str(mmr.get("root")) != str((result.get("mmr") or {}).get("root")):
                    errors.append("checkpoint mmr.root mismatch")
                if int(mmr.get("size") or 0) != int(result.get("receipt_count") or 0):
                    errors.append("checkpoint mmr.size mismatch")

                # Optional checkpoint signing threshold enforcement
                if enforce_checkpoint_policy and checkpoint_verified:
                    agreement_errs, agreement_summary = corridor_agreement_summary(module_dir)
                    errors.extend(agreement_errs)
                    role_by_did = agreement_summary.get("role_by_did") or {}
                    ck_thresholds = agreement_summary.get("checkpoint_signing_thresholds") or []
                    ok_dids: Set[str] = set(ck_ok_dids)
                    for t in ck_thresholds:
                        role = str(t.get("role") or "")
                        req = int(t.get("required") or 0)
                        if req <= 0:
                            continue
                        count = sum(1 for d in ok_dids if role_by_did.get(d) == role)
                        if count < req:
                            errors.append(
                                f"checkpoint signing threshold not met for role={role}: required={req}, got={count}"
                            )

    # Output
    if getattr(args, "json", False):
        out = dict(result or {})
        if warnings:
            out["warnings"] = warnings
        if checkpoint_path:
            out["checkpoint_verified"] = checkpoint_verified
        if errors:
            out["errors"] = errors
        print(json.dumps(out, indent=2))
    else:
        if errors:
            print("STATE VERIFY FAILED")
            for e in errors:
                print(f"  - {e}")
            if warnings:
                print("WARNINGS:")
                for w in warnings:
                    print(f"  - {w}")
        else:
            print("STATE VERIFY OK")
            print(f"corridor_id: {result.get('corridor_id')}")
            print(f"genesis_root: {result.get('genesis_root')}")
            print(f"receipt_count: {result.get('receipt_count')}")
            print(f"final_state_root: {result.get('final_state_root')}")
            mmr = result.get("mmr") or {}
            print(f"mmr_root: {mmr.get('root')}")
            if checkpoint_path:
                print(f"checkpoint_verified: {bool(checkpoint_verified)}")
            if warnings:
                print("WARNINGS:")
                for w in warnings:
                    print(f"  - {w}")

    return 2 if errors else 0


def cmd_corridor_state_fork_inspect(args: argparse.Namespace) -> int:
    """Inspect receipts for forks, duplicates, and resolution coverage.

    This is an incident-response / forensic command. Unlike `msez corridor state verify`, it does
    not require a fully-resolved canonical chain. Instead it emits a structured report describing:
      - fork points (same (sequence, prev_root) with multiple next_root candidates)
      - candidate receipts at each fork point (signers, file paths)
      - whether fork-resolution artifacts cover the fork points
      - an optional canonical head computation when resolutions are complete

    By default this command verifies receipt signatures (did:key offline). Use --no-verify-proofs
    for quick triage when signatures are not available (signer sets will be empty).
    """
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    corridor_id = _corridor_id_from_module(module_dir)

    receipts_path = pathlib.Path(str(getattr(args, "receipts", "") or "").strip())
    if not str(receipts_path):
        print("--receipts is required", file=sys.stderr)
        return 2
    if not receipts_path.is_absolute():
        receipts_path = REPO_ROOT / receipts_path

    fork_resolutions_path = None
    fr = str(getattr(args, "fork_resolutions", "") or "").strip()
    if fr:
        fp = pathlib.Path(fr)
        if not fp.is_absolute():
            fp = REPO_ROOT / fp
        fork_resolutions_path = fp

    from_checkpoint_path = None
    fc = str(getattr(args, "from_checkpoint", "") or "").strip()
    if fc:
        cp = pathlib.Path(fc)
        if not cp.is_absolute():
            cp = REPO_ROOT / cp
        from_checkpoint_path = cp

    verify_proofs = not bool(getattr(args, "no_verify_proofs", False))
    enforce_trust = bool(getattr(args, "enforce_trust_anchors", False)) and verify_proofs
    enforce_transition_types = bool(getattr(args, "enforce_transition_types", False))
    require_artifacts = bool(getattr(args, "require_artifacts", False))
    transitive_require_artifacts = bool(getattr(args, "transitive_require_artifacts", False))
    if transitive_require_artifacts:
        require_artifacts = True

    warnings: List[str] = []
    errors: List[str] = []

    # Corridor invariants
    genesis_root = corridor_state_genesis_root(module_dir)

    # Optional checkpoint bootstrap
    start_seq = 0
    start_prev_root = genesis_root
    checkpoint_info: Optional[Dict[str, Any]] = None
    if from_checkpoint_path:
        try:
            ck = load_json(from_checkpoint_path)
        except Exception as e:
            errors.append(f"failed to load from-checkpoint: {from_checkpoint_path}: {e}")
            ck = None

        if ck is not None:
            ck_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.checkpoint.schema.json")
            ve = list(ck_validator.iter_errors(ck))
            if ve:
                errors.append(f"invalid from-checkpoint schema: {from_checkpoint_path}: {ve[0].message}")
            else:
                # Verify checkpoint proofs when possible.
                ck_ok = False
                ck_signers: List[str] = []
                if verify_proofs:
                    try:
                        from tools.vc import verify_credential  # type: ignore
                        vres = verify_credential(ck)
                        ck_dids = sorted(_verified_base_dids(vres))
                        ck_ok = bool(ck_dids)
                        ck_signers = ck_dids
                    except Exception as e:
                        warnings.append(f"checkpoint proof verification failed: {from_checkpoint_path}: {e}")

                start_seq = int((ck or {}).get("receipt_count") or 0)
                start_prev_root = str((ck or {}).get("final_state_root") or "").strip().lower() or start_prev_root
                checkpoint_info = {
                    "path": str(from_checkpoint_path),
                    "valid_proofs": bool(ck_ok) if verify_proofs else None,
                    "signers": ck_signers,
                    "receipt_count": start_seq,
                    "final_state_root": start_prev_root,
                    "mmr_root": str(((ck or {}).get("mmr") or {}).get("root") or "").strip().lower(),
                }

    # Fork-resolution artifacts (optional)
    fork_map: Dict[Tuple[int, str], str] = {}
    fork_sources: Dict[Tuple[int, str], List[str]] = {}
    if fork_resolutions_path:
        fork_map, fork_sources, fork_errs = _load_fork_resolution_map_with_sources(
            fork_resolutions_path, expected_corridor_id=corridor_id
        )
        errors.extend(fork_errs)

    # Transition type registry snapshot (optional but recommended)
    corridor_ttr_digest, _corridor_ttr = corridor_transition_type_registry_snapshot(module_dir)

    allowed_receipt_signers: Set[str] = set()
    if enforce_trust:
        try:
            c_cfg = load_yaml(module_dir / "corridor.yaml")
            ta_rel = str((c_cfg or {}).get("trust_anchors_path") or "trust-anchors.yaml")
        except Exception:
            ta_rel = "trust-anchors.yaml"
        trust_anchors = load_trust_anchors(module_dir / ta_rel)
        allowed_receipt_signers = set(
            ta["did"]
            for ta in trust_anchors
            if "corridor_receipt" in (ta.get("allowed_attestations") or []) or "*" in (ta.get("allowed_attestations") or [])
        )

    import tools.artifacts as artifact_cas  # local import

    # Cache to avoid re-reading registry locks for every receipt.
    transitive_checked_ttr: set[str] = set()

    def _require_artifact(artifact_type: str, digest: str, label: str) -> None:
        if not digest:
            return
        try:
            artifact_cas.resolve_artifact_by_digest(artifact_type, digest, repo_root=REPO_ROOT)
        except Exception as e:
            if isinstance(e, FileNotFoundError):
                errors.append(f"missing artifact for {label}: {artifact_type}:{digest}")
            else:
                errors.append(f"artifact resolver error for {label}: {artifact_type}:{digest}: {e}")

    receipt_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.receipt.schema.json")

    receipt_rows: List[Tuple[pathlib.Path, Dict[str, Any], Set[str]]] = []
    invalid_receipts: List[Dict[str, Any]] = []

    for rp in _collect_receipt_paths(receipts_path):
        try:
            receipt = load_json(rp)
        except Exception as e:
            invalid_receipts.append({"path": str(rp), "error": f"json parse failed: {e}"})
            continue

        ve = list(receipt_validator.iter_errors(receipt))
        if ve:
            invalid_receipts.append({"path": str(rp), "error": f"schema invalid: {ve[0].message}"})
            continue

        # corridor_id check
        if str(receipt.get("corridor_id") or "").strip() != corridor_id:
            invalid_receipts.append({"path": str(rp), "error": "corridor_id mismatch"})
            continue

        # next_root recomputation
        try:
            expected_next = corridor_state_next_root(receipt)
            if str(receipt.get("next_root") or "").strip().lower() != expected_next:
                invalid_receipts.append({"path": str(rp), "error": "next_root mismatch"})
                continue
        except Exception as e:
            invalid_receipts.append({"path": str(rp), "error": f"next_root recompute failed: {e}"})
            continue

        # Optional transition type registry digest check (if present + corridor pinned)
        if enforce_transition_types and corridor_ttr_digest:
            r_ttr = _coerce_sha256(receipt.get("transition_type_registry_digest_sha256"))
            if r_ttr and corridor_ttr_digest and r_ttr != corridor_ttr_digest:
                invalid_receipts.append({"path": str(rp), "error": "transition_type_registry_digest mismatch"})
                continue

        # Artifact completeness check
        if require_artifacts:
            # typed digest-sets
            for entry in receipt.get("lawpack_digest_set") or []:
                if isinstance(entry, str):
                    _require_artifact("lawpack", entry, f"lawpack_digest_set[{entry}]")
                elif isinstance(entry, dict) and entry.get("digest_sha256"):
                    _require_artifact(str(entry.get("artifact_type") or "lawpack"), str(entry.get("digest_sha256")), "lawpack_digest_set")

            for entry in receipt.get("ruleset_digest_set") or []:
                if isinstance(entry, str):
                    _require_artifact("ruleset", entry, f"ruleset_digest_set[{entry}]")
                elif isinstance(entry, dict) and entry.get("digest_sha256"):
                    _require_artifact(str(entry.get("artifact_type") or "ruleset"), str(entry.get("digest_sha256")), "ruleset_digest_set")

            # Transition type registry digest: in transitive mode we treat this as a commitment root.
            r_ttr = _coerce_sha256(receipt.get("transition_type_registry_digest_sha256"))
            if r_ttr and not transitive_require_artifacts:
                _require_artifact("transition-types", r_ttr, "transition_type_registry_digest_sha256")

            if transitive_require_artifacts:
                effective = r_ttr or (corridor_ttr_digest or "")
                if effective and effective not in transitive_checked_ttr:
                    transitive_checked_ttr.add(effective)
                    _require_transition_types_lock_transitive(
                        effective,
                        errors=errors,
                        label="transition_type_registry_digest_sha256 (transitive)",
                        repo_root=REPO_ROOT,
                    )

            env = receipt.get("transition") or {}
            for field, atype in [
                ("schema_digest_sha256", "schema"),
                ("ruleset_digest_sha256", "ruleset"),
                ("zk_circuit_digest_sha256", "circuit"),
            ]:
                val = env.get(field)
                if isinstance(val, str):
                    _require_artifact(atype, val, f"transition.{field}")
                elif isinstance(val, dict) and val.get("digest_sha256"):
                    _require_artifact(str(val.get("artifact_type") or atype), str(val.get("digest_sha256")), f"transition.{field}")

            for att in env.get("attachments") or []:
                if isinstance(att, dict) and att.get("digest_sha256"):
                    _require_artifact(str(att.get("artifact_type") or "blob"), str(att.get("digest_sha256")), "transition.attachments")

        # Signature verification (optional)
        ok_dids: Set[str] = set()
        if verify_proofs:
            try:
                from tools.vc import verify_credential  # type: ignore
                vres = verify_credential(receipt)
                ok_dids = _verified_base_dids(vres)
            except Exception as e:
                invalid_receipts.append({"path": str(rp), "error": f"proof verification failed: {e}"})
                continue

            if not ok_dids:
                invalid_receipts.append({"path": str(rp), "error": "no valid proofs"})
                continue

            if enforce_trust and allowed_receipt_signers:
                if not (ok_dids & allowed_receipt_signers):
                    invalid_receipts.append({"path": str(rp), "error": f"signer not in trust anchors: {sorted(ok_dids)}"})
                    continue

        receipt_rows.append((rp, receipt, ok_dids))

    grouped = _group_receipt_candidates(receipt_rows)

    candidates_by_seq_prev: Dict[Tuple[int, str], List[Dict[str, Any]]] = {}
    for (seq, prev_root, _next_root), data in grouped.items():
        candidates_by_seq_prev.setdefault((seq, prev_root), []).append(data)

    # Enumerate forks
    fork_points: List[Dict[str, Any]] = []
    for (seq, prev_root), cands in sorted(candidates_by_seq_prev.items(), key=lambda kv: (kv[0][0], kv[0][1])):
        if len(cands) <= 1:
            continue

        point: Dict[str, Any] = {
            "sequence": int(seq),
            "prev_root": str(prev_root),
            "candidates": [],
        }

        for c in cands:
            rec = c.get("receipt") or {}
            point["candidates"].append(
                {
                    "next_root": str(rec.get("next_root") or ""),
                    "signer_count": len(c.get("signers") or []),
                    "signers": sorted(list(c.get("signers") or [])),
                    "paths": list(c.get("paths") or []),
                }
            )

        chosen = fork_map.get((int(seq), str(prev_root)))
        if chosen:
            point["chosen_next_root"] = chosen
            point["resolution_sources"] = fork_sources.get((int(seq), str(prev_root)), [])
            point["resolved"] = any(str(cc.get("next_root") or "") == chosen for cc in point["candidates"])
        else:
            point["resolved"] = False

        fork_points.append(point)

    total_forks = len(fork_points)
    resolved_forks = sum(1 for fp in fork_points if bool(fp.get("resolved")))
    unresolved_forks = total_forks - resolved_forks

    # Canonical chain head (best-effort)
    canonical_head: Optional[Dict[str, Any]] = None
    if candidates_by_seq_prev:
        chain, sel_warn, sel_err = _select_canonical_chain(
            candidates_by_seq_prev,
            start_seq=start_seq,
            start_prev_root=start_prev_root,
            fork_resolution_map=fork_map if fork_map else None,
        )
        warnings.extend(sel_warn)
        # do not treat selection errors as fatal for the report
        if sel_err:
            warnings.extend(sel_err)

        if chain:
            receipt_count = start_seq + len(chain)
            final_state_root = str(chain[-1].get("next_root") or "").strip().lower()
            canonical_head = {
                "receipt_count": receipt_count,
                "final_state_root": final_state_root,
                "last_sequence": receipt_count - 1,
            }
        else:
            # With no receipts selected, the checkpoint head (if any) is still informative.
            if checkpoint_info is not None:
                canonical_head = {
                    "receipt_count": int(checkpoint_info.get("receipt_count") or 0),
                    "final_state_root": str(checkpoint_info.get("final_state_root") or "").strip().lower(),
                    "last_sequence": int(checkpoint_info.get("receipt_count") or 0) - 1,
                }

    recommendations: List[str] = []
    if unresolved_forks:
        recommendations.append(
            "UNRESOLVED_FORKS: issue fork-resolution VC(s) selecting a canonical receipt per fork point; HALT corridor lifecycle until resolved."
        )
    if invalid_receipts:
        recommendations.append(
            "INVALID_RECEIPTS_PRESENT: review invalid receipt files; forensics may require signature/key rotation checks and artifact completeness checks."
        )

    report: Dict[str, Any] = {
        "type": "MSEZCorridorForkInspectReport",
        "corridor_id": corridor_id,
        "genesis_root": genesis_root,
        "from_checkpoint": checkpoint_info,
        "receipts": {
            "total": len(_collect_receipt_paths(receipts_path)),
            "valid": len(receipt_rows),
            "invalid": len(invalid_receipts),
            "invalid_details": invalid_receipts,
        },
        "forks": {
            "total": total_forks,
            "resolved": resolved_forks,
            "unresolved": unresolved_forks,
            "points": fork_points,
        },
        "canonical_head": canonical_head,
        "recommendations": recommendations,
        "warnings": warnings,
        "errors": errors,
    }

    # Validate report shape
    try:
        v = schema_validator(REPO_ROOT / "schemas" / "corridor.fork-inspect-report.schema.json")
        ve = list(v.iter_errors(report))
        if ve:
            warnings.append(f"fork-inspect report schema self-check failed: {ve[0].message}")
    except Exception:
        pass

    fmt = str(getattr(args, "format", "") or "text").strip().lower()
    out_path = str(getattr(args, "out", "") or "").strip()

    rendered = ""
    if fmt == "json":
        rendered = json.dumps(report, indent=2, ensure_ascii=False)
    else:
        # human-friendly text
        rendered_lines = []
        rendered_lines.append(f"corridor_id: {corridor_id}")
        rendered_lines.append(f"genesis_root: {genesis_root}")
        if checkpoint_info:
            rendered_lines.append(f"from_checkpoint: {checkpoint_info.get('path')}")
            rendered_lines.append(f"  checkpoint_receipt_count: {checkpoint_info.get('receipt_count')}")
            rendered_lines.append(f"  checkpoint_final_state_root: {checkpoint_info.get('final_state_root')}")

        rendered_lines.append(f"receipts_total: {report['receipts']['total']}")
        rendered_lines.append(f"receipts_valid: {report['receipts']['valid']}")
        rendered_lines.append(f"receipts_invalid: {report['receipts']['invalid']}")
        rendered_lines.append(f"fork_points: {total_forks} (resolved={resolved_forks}, unresolved={unresolved_forks})")

        for fp in fork_points:
            rendered_lines.append(f"- fork sequence={fp.get('sequence')} prev_root={fp.get('prev_root')}")
            if fp.get('chosen_next_root'):
                rendered_lines.append(f"    chosen_next_root: {fp.get('chosen_next_root')} (resolved={fp.get('resolved')})")
            for cand in fp.get('candidates') or []:
                rendered_lines.append(
                    f"    candidate next_root={cand.get('next_root')} signers={cand.get('signer_count')} paths={len(cand.get('paths') or [])}"
                )

        if canonical_head:
            rendered_lines.append(f"canonical_head_receipt_count: {canonical_head.get('receipt_count')}")
            rendered_lines.append(f"canonical_head_final_state_root: {canonical_head.get('final_state_root')}")

        if recommendations:
            rendered_lines.append("RECOMMENDATIONS:")
            for r in recommendations:
                rendered_lines.append(f"  - {r}")

        if warnings:
            rendered_lines.append("WARNINGS:")
            for w in warnings:
                rendered_lines.append(f"  - {w}")

        if errors:
            rendered_lines.append("ERRORS:")
            for e in errors:
                rendered_lines.append(f"  - {e}")

        rendered = "\n".join(rendered_lines)

    if out_path:
        op = pathlib.Path(out_path)
        if not op.is_absolute():
            op = REPO_ROOT / op
        op.parent.mkdir(parents=True, exist_ok=True)
        op.write_text(rendered + "\n", encoding="utf-8")
        print(str(op))
    else:
        print(rendered)

    # Exit code convention: 0 for report emission, 2 for fatal parse errors.
    return 0




def cmd_corridor_state_propose(args: argparse.Namespace) -> int:
    """Generate an unsigned receipt proposal (MSEZCorridorReceiptProposal).

    This is a convenience helper for pre-signature negotiation. The proposal includes
    `computed_next_root`, which is the hash commitment that would become the receipt's
    `next_root` once the receipt is signed and published.
    """
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    corridor_id = _corridor_id_from_module(module_dir)

    # Digest sets default to corridor expectations unless explicitly overridden.
    lp_override = getattr(args, "lawpack_digest", None)
    rs_override = getattr(args, "ruleset_digest", None)

    lawpack_digest_set = _normalize_digest_set(lp_override) if lp_override else _normalize_digest_set(
        corridor_expected_lawpack_digest_set(module_dir)
    )
    ruleset_digest_set = _normalize_digest_set(rs_override) if rs_override else _normalize_digest_set(
        corridor_expected_ruleset_digest_set(module_dir)
    )

    # Transition type registry snapshot
    _ttr_digest, _ttr = corridor_transition_type_registry_snapshot(module_dir)
    ttr_digest = str(_ttr_digest or "").strip().lower()

    # prev_root default: genesis_root
    prev_root = str(getattr(args, "prev_root", "") or "").strip().lower()
    if not prev_root:
        prev_root = corridor_state_genesis_root(module_dir)

    # Timestamp default: now()
    ts = str(getattr(args, "timestamp", "") or "").strip()
    if not ts:
        ts = datetime.utcnow().replace(microsecond=0).isoformat() + "Z"

    seq = int(getattr(args, "sequence", 0))

    transition_path = str(getattr(args, "transition", "") or "").strip()
    if not transition_path:
        print("--transition is required", file=sys.stderr)
        return 2
    tpath = pathlib.Path(transition_path)
    if not tpath.is_absolute():
        tpath = REPO_ROOT / tpath

    transition = _load_transition_envelope(tpath)

    # Optional digest auto-fill based on corridor module state.
    if bool(getattr(args, "fill_transition_digests", False)):
        if not transition.get("ruleset_digest_sha256") and ruleset_digest_set:
            # Best-effort: pick the first expected ruleset digest as the envelope's ruleset digest.
            transition["ruleset_digest_sha256"] = ruleset_digest_set[0]
        if not transition.get("transition_type_registry_digest_sha256") and ttr_digest:
            transition["transition_type_registry_digest_sha256"] = ttr_digest

    # Build the receipt template (without next_root / proof) and compute the next_root commitment.
    receipt_template: Dict[str, Any] = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": seq,
        "timestamp": ts,
        "prev_root": prev_root,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "transition": transition,
    }
    if ttr_digest:
        receipt_template["transition_type_registry_digest_sha256"] = ttr_digest

    computed_next_root = corridor_state_next_root(receipt_template)

    # Proposal artifact
    import uuid
    from tools.vc import now_rfc3339

    proposal: Dict[str, Any] = {
        "type": "MSEZCorridorReceiptProposal",
        "proposal_id": str(getattr(args, "proposal_id", "") or "").strip() or f"urn:uuid:{uuid.uuid4()}",
        "corridor_id": corridor_id,
        "proposed_at": now_rfc3339(),
        "proposed_by": str(getattr(args, "proposed_by", "") or "").strip(),
        "sequence": seq,
        "timestamp": ts,
        "prev_root": prev_root,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "transition": transition,
                "computed_next_root": computed_next_root,
    }
    if ttr_digest:
        proposal["transition_type_registry_digest_sha256"] = ttr_digest
    if not proposal["proposed_by"]:
        proposal.pop("proposed_by", None)

    notes = str(getattr(args, "notes", "") or "").strip()
    if notes:
        proposal["notes"] = notes

    # Validate
    v = schema_validator(REPO_ROOT / "schemas" / "corridor.receipt-proposal.schema.json")
    ve = list(v.iter_errors(proposal))
    if ve:
        print(f"Invalid proposal schema: {ve[0].message}", file=sys.stderr)
        return 2

    out_path = str(getattr(args, "out", "") or "").strip()
    if out_path:
        op = pathlib.Path(out_path)
        if not op.is_absolute():
            op = REPO_ROOT / op
        op.parent.mkdir(parents=True, exist_ok=True)
        op.write_text(json.dumps(proposal, indent=2) + "\n")
        print(str(op))
    else:
        print(json.dumps(proposal, indent=2))
    return 0


def cmd_corridor_state_fork_resolve(args: argparse.Namespace) -> int:
    """Generate an unsigned fork-resolution VC selecting the canonical receipt for a fork point."""
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    corridor_id = _corridor_id_from_module(module_dir)

    seq = int(getattr(args, "sequence", 0))
    prev_root = str(getattr(args, "prev_root", "") or "").strip().lower()
    chosen_next_root = str(getattr(args, "chosen_next_root", "") or "").strip().lower()
    issuer = str(getattr(args, "issuer", "") or "").strip()

    if not prev_root or not chosen_next_root or not issuer:
        print("--prev-root, --chosen-next-root, and --issuer are required", file=sys.stderr)
        return 2

    from tools.vc import now_rfc3339
    import uuid

    resolved_at = str(getattr(args, "resolved_at", "") or "").strip() or now_rfc3339()

    candidates: List[Dict[str, Any]] = []
    for c in (getattr(args, "candidate_next_root", []) or []):
        croot = str(c or "").strip().lower()
        if croot:
            candidates.append({"next_root": croot})

    subject: Dict[str, Any] = {
        "type": "MSEZCorridorForkResolution",
        "corridor_id": corridor_id,
        "sequence": seq,
        "prev_root": prev_root,
        "chosen_next_root": chosen_next_root,
        "resolved_at": resolved_at,
    }
    if candidates:
        subject["candidates"] = candidates

    vc_id = str(getattr(args, "id", "") or "").strip() or f"urn:uuid:{uuid.uuid4()}"
    vc_obj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorForkResolutionCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": subject,
    }

    # Validate the credentialSubject shape (the VC wrapper is unsigned until `msez vc sign` is applied).
    v = schema_validator(REPO_ROOT / "schemas" / "corridor.fork-resolution.schema.json")
    ve = list(v.iter_errors(subject))
    if ve:
        print(f"Invalid fork-resolution subject schema: {ve[0].message}", file=sys.stderr)
        return 2

    out_path = str(getattr(args, "out", "") or "").strip()
    if out_path:
        op = pathlib.Path(out_path)
        if not op.is_absolute():
            op = REPO_ROOT / op
        op.parent.mkdir(parents=True, exist_ok=True)
        op.write_text(json.dumps(vc_obj, indent=2) + "\n")
        print(str(op))
    else:
        print(json.dumps(vc_obj, indent=2))
    return 0


def cmd_corridor_state_anchor(args: argparse.Namespace) -> int:
    """Generate an unsigned corridor-anchor VC referencing a head commitment and anchoring metadata."""
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    corridor_id = _corridor_id_from_module(module_dir)

    issuer = str(getattr(args, "issuer", "") or "").strip()
    if not issuer:
        print("--issuer is required", file=sys.stderr)
        return 2

    head_commitment = str(getattr(args, "head_commitment", "") or "").strip().lower()

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    fork_resolutions_arg = str(getattr(args, "fork_resolutions", "") or "").strip()
    from_checkpoint_arg = str(getattr(args, "from_checkpoint", "") or "").strip()

    if not head_commitment:
        if not receipts_arg:
            print("Provide either --head-commitment or --receipts to compute it", file=sys.stderr)
            return 2

        rpath = pathlib.Path(receipts_arg)
        if not rpath.is_absolute():
            rpath = REPO_ROOT / rpath

        fork_path = None
        if fork_resolutions_arg:
            fp = pathlib.Path(fork_resolutions_arg)
            if not fp.is_absolute():
                fp = REPO_ROOT / fp
            fork_path = fp

        ck_path = None
        if from_checkpoint_arg:
            cp = pathlib.Path(from_checkpoint_arg)
            if not cp.is_absolute():
                cp = REPO_ROOT / cp
            ck_path = cp

        chain_res, _warn, errs = _corridor_state_build_chain(
            module_dir,
            rpath,
            fork_resolutions_path=fork_path,
            from_checkpoint_path=ck_path,
        )
        if errs:
            print("Cannot compute head commitment:", file=sys.stderr)
            for e in errs:
                print(f"  - {e}", file=sys.stderr)
            return 2

        mmr_root = str((chain_res.get("mmr") or {}).get("root") or "")
        head_commitment = corridor_head_commitment_digest(
            corridor_id=corridor_id,
            genesis_root=str(chain_res.get("genesis_root") or ""),
            receipt_count=int(chain_res.get("receipt_count") or 0),
            final_state_root=str(chain_res.get("final_state_root") or ""),
            mmr_root=mmr_root,
        )

    # Chain metadata
    network = str(getattr(args, "network", "") or "").strip()
    if not network:
        print("--network is required", file=sys.stderr)
        return 2

    chain_obj: Dict[str, Any] = {"network": network}
    for field in ["chain_id", "tx_hash", "block_hash"]:
        val = str(getattr(args, field, "") or "").strip()
        if val:
            chain_obj[field] = val
    bn = getattr(args, "block_number", None)
    if bn is not None:
        try:
            chain_obj["block_number"] = int(bn)
        except Exception:
            pass
    bt = str(getattr(args, "block_timestamp", "") or "").strip()
    if bt:
        chain_obj["block_timestamp"] = bt

    from tools.vc import now_rfc3339
    import uuid

    anchored_at = str(getattr(args, "anchored_at", "") or "").strip() or now_rfc3339()

    subject: Dict[str, Any] = {
        "type": "MSEZCorridorAnchor",
        "corridor_id": corridor_id,
        "anchored_at": anchored_at,
        "head_commitment_digest_sha256": head_commitment,
        "chain": chain_obj,
    }

    # Optional checkpoint reference as ArtifactRef
    ck_digest = str(getattr(args, "checkpoint_digest", "") or "").strip().lower()
    ck_uri = str(getattr(args, "checkpoint_uri", "") or "").strip()
    if ck_digest:
        subject["checkpoint_ref"] = make_artifact_ref("checkpoint", ck_digest, uri=ck_uri)

    vc_id = str(getattr(args, "id", "") or "").strip() or f"urn:uuid:{uuid.uuid4()}"
    vc_obj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorAnchorCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": subject,
    }

    # Validate the credentialSubject shape (the VC wrapper is unsigned until `msez vc sign` is applied).
    v = schema_validator(REPO_ROOT / "schemas" / "corridor.anchor.schema.json")
    ve = list(v.iter_errors(subject))
    if ve:
        print(f"Invalid anchor subject schema: {ve[0].message}", file=sys.stderr)
        return 2

    out_path = str(getattr(args, "out", "") or "").strip()
    if out_path:
        op = pathlib.Path(out_path)
        if not op.is_absolute():
            op = REPO_ROOT / op
        op.parent.mkdir(parents=True, exist_ok=True)
        op.write_text(json.dumps(vc_obj, indent=2) + "\n")
        print(str(op))
    else:
        print(json.dumps(vc_obj, indent=2))
    return 0


def cmd_corridor_state_finality_status(args: argparse.Namespace) -> int:
    """Compute a corridor finality status object for the current head.

    This is primarily an *output* helper for tooling; it does not sign anything.
    """
    module_dir = coerce_corridor_module_dir(pathlib.Path(args.path))
    corridor_id = _corridor_id_from_module(module_dir)

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required to compute the head (use --from-checkpoint for scalable sync)", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath

    fork_resolutions_arg = str(getattr(args, "fork_resolutions", "") or "").strip()
    fork_path = None
    if fork_resolutions_arg:
        fp = pathlib.Path(fork_resolutions_arg)
        if not fp.is_absolute():
            fp = REPO_ROOT / fp
        fork_path = fp

    from_checkpoint_arg = str(getattr(args, "from_checkpoint", "") or "").strip()
    from_ck_path = None
    if from_checkpoint_arg:
        cp = pathlib.Path(from_checkpoint_arg)
        if not cp.is_absolute():
            cp = REPO_ROOT / cp
        from_ck_path = cp

    chain_res, warnings, errs = _corridor_state_build_chain(
        module_dir,
        rpath,
        fork_resolutions_path=fork_path,
        from_checkpoint_path=from_ck_path,
    )
    if errs:
        print("Cannot compute corridor head:", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        return 2

    mmr_root = str((chain_res.get("mmr") or {}).get("root") or "")
    receipt_count = int(chain_res.get("receipt_count") or 0)
    final_state_root = str(chain_res.get("final_state_root") or "")
    genesis_root = str(chain_res.get("genesis_root") or "")

    head_commitment = corridor_head_commitment_digest(
        corridor_id=corridor_id,
        genesis_root=genesis_root,
        receipt_count=receipt_count,
        final_state_root=final_state_root,
        mmr_root=mmr_root,
    )

    # Finality ranking helper
    order = {
        "proposed": 0,
        "receipt_signed": 1,
        "checkpoint_signed": 2,
        "watcher_quorum": 3,
        "l1_anchored": 4,
        "legally_recognized": 5,
    }

    finality = "receipt_signed" if receipt_count > 0 else "proposed"

    evidence: Dict[str, Any] = {}

    # Optional checkpoint
    checkpoint_path = str(getattr(args, "checkpoint", "") or "").strip()
    if checkpoint_path:
        ck_path = pathlib.Path(checkpoint_path)
        if not ck_path.is_absolute():
            ck_path = REPO_ROOT / ck_path
        try:
            ck = load_json(ck_path)
            ck_validator = schema_validator(REPO_ROOT / "schemas" / "corridor.checkpoint.schema.json")
            ve = list(ck_validator.iter_errors(ck))
            if ve:
                warnings.append(f"checkpoint schema invalid: {ck_path}: {ve[0].message}")
            else:
                from tools.vc import verify_credential
                ck_v = verify_credential(ck)
                if not _verified_base_dids(ck_v):
                    warnings.append(f"checkpoint signature not verified: {ck_path}")
                else:
                    # Match head
                    mmr = ck.get("mmr") or {}
                    if (
                        str(ck.get("corridor_id")) == corridor_id
                        and str(ck.get("genesis_root")) == genesis_root
                        and int(ck.get("receipt_count") or 0) == receipt_count
                        and str(ck.get("final_state_root")) == final_state_root
                        and str(mmr.get("root")) == mmr_root
                    ):
                        finality = "checkpoint_signed" if order[finality] < order["checkpoint_signed"] else finality
                        # add checkpoint ref (digest of file bytes)
                        ck_digest = sha256_bytes(ck_path.read_bytes())
                        evidence["checkpoint"] = make_artifact_ref("checkpoint", ck_digest, uri=str(ck_path))
                    else:
                        warnings.append("checkpoint does not match computed head; ignoring for finality level")
        except Exception as e:
            warnings.append(f"failed to load/parse checkpoint: {checkpoint_path}: {e}")

    # Optional watcher compare/quorum report
    watcher_report_path = str(getattr(args, "watcher_report", "") or "").strip()
    if watcher_report_path:
        wp = pathlib.Path(watcher_report_path)
        if not wp.is_absolute():
            wp = REPO_ROOT / wp
        try:
            wr = load_json(wp)
            # Supported shapes:
            #  (1) corridor_watcher_compare report: { corridors:[{corridor_id, quorum:{finality_level,...}}] }
            #  (2) direct: { corridor_id, finality_level }
            lvl = None
            if isinstance(wr, dict) and isinstance(wr.get("corridors"), list):
                for c in wr.get("corridors"):
                    if isinstance(c, dict) and str(c.get("corridor_id")) == corridor_id:
                        q = c.get("quorum") or {}
                        if isinstance(q, dict):
                            lvl = q.get("finality_level")
                        break
            elif isinstance(wr, dict) and str(wr.get("corridor_id")) == corridor_id:
                lvl = wr.get("finality_level")

            if lvl and str(lvl) in order:
                if order[str(lvl)] > order[finality]:
                    finality = str(lvl)
                evidence["watcher_quorum"] = {"source": str(wp), "finality_level": str(lvl)}
        except Exception as e:
            warnings.append(f"failed to load watcher report: {watcher_report_path}: {e}")

    # Optional anchors
    anchors_arg = str(getattr(args, "anchors", "") or "").strip()
    if anchors_arg:
        ap = pathlib.Path(anchors_arg)
        if not ap.is_absolute():
            ap = REPO_ROOT / ap
        anchor_files: List[pathlib.Path] = []
        if ap.is_dir():
            anchor_files = sorted([x for x in ap.glob("*.json") if x.is_file()])
        else:
            anchor_files = [ap]
        valid_anchors: List[Dict[str, Any]] = []
        for af in anchor_files:
            try:
                aobj = load_json(af)
                subj = aobj.get("credentialSubject") if isinstance(aobj, dict) else None
                if isinstance(subj, dict) and str(subj.get("head_commitment_digest_sha256")) == head_commitment:
                    # Best-effort verify signature
                    from tools.vc import verify_credential
                    av = verify_credential(aobj)
                    if _verified_base_dids(av):
                        ad = sha256_bytes(af.read_bytes())
                        valid_anchors.append(make_artifact_ref("vc", ad, uri=str(af)))
            except Exception:
                continue
        if valid_anchors:
            evidence["anchors"] = valid_anchors
            if order["l1_anchored"] > order[finality]:
                finality = "l1_anchored"

    # Optional arbitration awards (legal recognition)
    awards_arg = str(getattr(args, "arbitration_awards", "") or "").strip()
    if awards_arg:
        ap = pathlib.Path(awards_arg)
        if not ap.is_absolute():
            ap = REPO_ROOT / ap
        award_files: List[pathlib.Path] = []
        if ap.is_dir():
            award_files = sorted([x for x in ap.glob("*.json") if x.is_file()])
        else:
            award_files = [ap]
        valid_awards: List[Dict[str, Any]] = []
        for af in award_files:
            try:
                aobj = load_json(af)
                subj = aobj.get("credentialSubject") if isinstance(aobj, dict) else None
                if isinstance(subj, dict) and str(subj.get("head_commitment_digest_sha256")) == head_commitment:
                    from tools.vc import verify_credential
                    av = verify_credential(aobj)
                    if _verified_base_dids(av):
                        ad = sha256_bytes(af.read_bytes())
                        valid_awards.append(make_artifact_ref("vc", ad, uri=str(af)))
            except Exception:
                continue
        if valid_awards:
            evidence["arbitration_awards"] = valid_awards
            if order["legally_recognized"] > order[finality]:
                finality = "legally_recognized"

    from tools.vc import now_rfc3339
    status_obj: Dict[str, Any] = {
        "type": "MSEZCorridorFinalityStatus",
        "corridor_id": corridor_id,
        "as_of": now_rfc3339(),
        "receipt_count": receipt_count,
        "final_state_root": final_state_root,
        "mmr_root": mmr_root,
        "head_commitment_digest_sha256": head_commitment,
        "finality_level": finality,
        "evidence": evidence,
    }
    if warnings:
        status_obj["warnings"] = warnings

    v = schema_validator(REPO_ROOT / "schemas" / "corridor.finality-status.schema.json")
    ve = list(v.iter_errors(status_obj))
    if ve:
        print(f"Invalid finality-status schema: {ve[0].message}", file=sys.stderr)
        return 2

    out_path = str(getattr(args, "out", "") or "").strip()
    if out_path:
        op = pathlib.Path(out_path)
        if not op.is_absolute():
            op = REPO_ROOT / op
        op.parent.mkdir(parents=True, exist_ok=True)
        op.write_text(json.dumps(status_obj, indent=2) + "\n")
        print(str(op))
    else:
        print(json.dumps(status_obj, indent=2))
    return 0


# --- Receipt accumulator (fork-aware canonical chain) -------------------------


def _corridor_state_load_verified_receipts(
    module_dir: pathlib.Path,
    receipts_path: pathlib.Path,
    enforce_trust: bool = False,
    fork_resolutions_path: Optional[pathlib.Path] = None,
) -> Tuple[str, List[Dict[str, Any]], str]:
    """Internal helper used by checkpoint/inclusion tools.

    Returns:
      (genesis_root, canonical_receipts, final_state_root)
    """
    result, _warnings, errors = _corridor_state_build_chain(
        module_dir,
        receipts_path,
        enforce_trust_anchors=enforce_trust,
        fork_resolutions_path=fork_resolutions_path,
    )
    if errors:
        raise ValueError("; ".join(errors))
    return (
        str(result.get("genesis_root")),
        list(result.get("receipts") or []),
        str(result.get("final_state_root")),
    )

def cmd_corridor_state_checkpoint(args: argparse.Namespace) -> int:
    """Create a signed checkpoint committing to the corridor state head and receipt MMR root."""
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk  # type: ignore
    from tools.mmr import mmr_root_from_next_roots  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath
    if not rpath.exists():
        print(f"Receipts path not found: {rpath}", file=sys.stderr)
        return 2

    fork_resolutions_arg = str(getattr(args, "fork_resolutions", "") or "").strip()
    fork_resolutions_path: Optional[pathlib.Path] = None
    if fork_resolutions_arg:
        fp = pathlib.Path(fork_resolutions_arg)
        if not fp.is_absolute():
            fp = REPO_ROOT / fp
        fork_resolutions_path = fp

    try:
        genesis_root, receipts, final_root = _corridor_state_load_verified_receipts(
            module_dir,
            rpath,
            enforce_trust=bool(getattr(args, "enforce_trust_anchors", False)),
            fork_resolutions_path=fork_resolutions_path,
        )
    except Exception as ex:
        print(f"STATE FAIL: {ex}", file=sys.stderr)
        return 2

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    # Prefer the digest sets as actually carried in receipts (after normalization).
    lawpack_digest_set = _normalize_digest_set((receipts[0].get("lawpack_digest_set") or []))
    ruleset_digest_set = _normalize_digest_set((receipts[0].get("ruleset_digest_set") or []))

    next_roots = [str(r.get("next_root") or "").strip().lower() for r in receipts]
    mmr_info = mmr_root_from_next_roots(next_roots)

    checkpoint: Dict[str, Any] = {
        "type": "MSEZCorridorStateCheckpoint",
        "corridor_id": corridor_id,
        "timestamp": now_rfc3339(),
        "genesis_root": genesis_root,
        "final_state_root": final_root,
        "receipt_count": len(receipts),
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "mmr": {
            "type": "MSEZReceiptMMR",
            "algorithm": "sha256",
            "size": mmr_info["size"],
            "root": mmr_info["root"],
            "peaks": mmr_info.get("peaks", []),
        },
    }

    # Optional signing
    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)

        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            vm = f"{did}#key-1"
        add_ed25519_proof(checkpoint, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out = str(getattr(args, "out", "") or "").strip() or "corridor.checkpoint.json"
    out_path = pathlib.Path(out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(checkpoint, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_state_inclusion_proof(args: argparse.Namespace) -> int:
    """Generate an MMR inclusion proof for a receipt sequence (leaf index)."""
    from tools.vc import now_rfc3339  # type: ignore
    from tools.mmr import build_inclusion_proof  # type: ignore
    from tools.lawpack import jcs_canonicalize  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath
    if not rpath.exists():
        print(f"Receipts path not found: {rpath}", file=sys.stderr)
        return 2

    fork_resolutions_arg = str(getattr(args, "fork_resolutions", "") or "").strip()
    fork_resolutions_path: Optional[pathlib.Path] = None
    if fork_resolutions_arg:
        fp = pathlib.Path(fork_resolutions_arg)
        if not fp.is_absolute():
            fp = REPO_ROOT / fp
        fork_resolutions_path = fp

    try:
        _genesis_root, receipts, _final_root = _corridor_state_load_verified_receipts(
            module_dir,
            rpath,
            enforce_trust=bool(getattr(args, "enforce_trust_anchors", False)),
            fork_resolutions_path=fork_resolutions_path,
        )
    except Exception as ex:
        print(f"STATE FAIL: {ex}", file=sys.stderr)
        return 2

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    seq_arg = getattr(args, "sequence", None)
    if seq_arg is None:
        print("--sequence is required", file=sys.stderr)
        return 2
    try:
        seq = int(seq_arg)
        if seq < 0:
            raise ValueError()
    except Exception:
        print("--sequence must be a non-negative integer", file=sys.stderr)
        return 2

    next_roots = [str(r.get("next_root") or "").strip().lower() for r in receipts]
    if seq >= len(next_roots):
        print(f"--sequence out of range (have {len(next_roots)} receipts)", file=sys.stderr)
        return 2

    base = build_inclusion_proof(next_roots, seq)

    proof_obj: Dict[str, Any] = {
        "type": "MSEZCorridorReceiptInclusionProof",
        "corridor_id": corridor_id,
        "generated_at": now_rfc3339(),
        "mmr": {
            "type": "MSEZReceiptMMR",
            "algorithm": "sha256",
            "size": base["size"],
            "root": base["root"],
        },
        "leaf_index": base["leaf_index"],
        "receipt_next_root": base["receipt_next_root"],
        "leaf_hash": base["leaf_hash"],
        "peak_index": base["peak_index"],
        "peak_height": base["peak_height"],
        "path": base["path"],
        "peaks": base["peaks"],
        "computed_peak_root": base.get("computed_peak_root", ""),
    }

    # Optional checkpoint binding: include digest of a checkpoint payload (excluding proof)
    cp_path = str(getattr(args, "checkpoint", "") or "").strip()
    if cp_path:
        cpp = pathlib.Path(cp_path)
        if not cpp.is_absolute():
            cpp = REPO_ROOT / cpp
        if not cpp.exists():
            print(f"Checkpoint not found: {cpp}", file=sys.stderr)
            return 2
        cp = load_json(cpp)
        # Verify it matches the MMR root/size.
        try:
            cp_mmr = (cp or {}).get("mmr") or {}
            if str(cp_mmr.get("root") or "").strip().lower() != str(base["root"]).strip().lower():
                print("Checkpoint mmr.root does not match computed proof root", file=sys.stderr)
                return 2
            if int(cp_mmr.get("size") or 0) != int(base["size"]):
                print("Checkpoint mmr.size does not match computed proof size", file=sys.stderr)
                return 2
        except Exception:
            print("Invalid checkpoint format", file=sys.stderr)
            return 2

        tmp = dict(cp)
        tmp.pop("proof", None)
        checkpoint_digest = sha256_bytes(jcs_canonicalize(tmp))
        proof_obj["checkpoint_digest_sha256"] = checkpoint_digest
        proof_obj["checkpoint_path"] = str(cpp)
        proof_obj["checkpoint_mmr_root"] = str((cp_mmr or {}).get("root") or "")
        proof_obj["checkpoint_receipt_count"] = int((cp_mmr or {}).get("size") or 0)
        proof_obj["checkpoint_final_state_root"] = str((cp or {}).get("final_state_root") or "")
        proof_obj["checkpoint_ref"] = {
            "artifact_type": "checkpoint",
            "digest_sha256": checkpoint_digest,
            "uri": str(cpp),
            "media_type": "application/json",
        }

    out = str(getattr(args, "out", "") or "").strip() or f"corridor.inclusion-proof.{seq}.json"
    out_path = pathlib.Path(out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(proof_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_state_verify_inclusion(args: argparse.Namespace) -> int:
    """Verify an inclusion proof against a signed checkpoint and a receipt.

    Validates:
    - Receipt signature(s)
    - Checkpoint signature(s)
    - Receipt next_root recomputation
    - MMR inclusion proof: receipt.next_root included in checkpoint.mmr.root

    If --enforce-trust-anchors is set, requires at least one valid signature from a trust anchor
    authorized for:
    - corridor_receipt (receipt)
    - corridor_checkpoint (checkpoint) OR corridor_receipt (fallback)
    """
    from tools.vc import verify_credential  # type: ignore
    from tools.mmr import verify_inclusion_proof  # type: ignore

    # Resolve corridor module
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    enforce_trust = bool(getattr(args, "enforce_trust_anchors", False))
    allowed_receipt_signers = set()
    allowed_checkpoint_signers = set()

    if enforce_trust:
        try:
            ta_rel = str((c or {}).get("trust_anchors_path") or "trust-anchors.yaml")
            ta = load_yaml(module_dir / ta_rel)
            for a in (ta.get("trust_anchors") or []):
                if not isinstance(a, dict):
                    continue
                ident = str(a.get("identifier") or "").split("#", 1)[0]
                if not ident:
                    continue
                allowed = set(a.get("allowed_attestations") or [])
                if "corridor_receipt" in allowed:
                    allowed_receipt_signers.add(ident)
                # corridor_checkpoint is preferred; corridor_receipt is accepted for backwards compatibility.
                if "corridor_checkpoint" in allowed or "corridor_receipt" in allowed:
                    allowed_checkpoint_signers.add(ident)
        except Exception:
            allowed_receipt_signers = set()
            allowed_checkpoint_signers = set()

    # Inputs
    receipt_path = pathlib.Path(str(getattr(args, "receipt", "") or "").strip())
    proof_path = pathlib.Path(str(getattr(args, "proof", "") or "").strip())
    checkpoint_path = pathlib.Path(str(getattr(args, "checkpoint", "") or "").strip())

    for name, pp in [("--receipt", receipt_path), ("--proof", proof_path), ("--checkpoint", checkpoint_path)]:
        if not str(pp):
            print(f"{name} is required", file=sys.stderr)
            return 2
        if not pp.is_absolute():
            pp2 = REPO_ROOT / pp
        else:
            pp2 = pp
        if not pp2.exists():
            print(f"{name} not found: {pp2}", file=sys.stderr)
            return 2

    # Normalize absolute paths
    if not receipt_path.is_absolute():
        receipt_path = REPO_ROOT / receipt_path
    if not proof_path.is_absolute():
        proof_path = REPO_ROOT / proof_path
    if not checkpoint_path.is_absolute():
        checkpoint_path = REPO_ROOT / checkpoint_path

    receipt = load_json(receipt_path)
    proof = load_json(proof_path)
    checkpoint = load_json(checkpoint_path)

    # Corridor id binding (best-effort)
    if corridor_id:
        for obj_name, obj in [("receipt", receipt), ("checkpoint", checkpoint), ("proof", proof)]:
            obj_cid = str((obj or {}).get("corridor_id") or "").strip()
            if obj_cid and obj_cid != corridor_id:
                print(f"FAIL: {obj_name}.corridor_id does not match corridor.yaml", file=sys.stderr)
                return 2

    # Verify receipt proof(s)
    rres = verify_credential(receipt)
    if not rres or any(not r.ok for r in rres):
        print("FAIL: receipt signature invalid", file=sys.stderr)
        return 2
    if enforce_trust and allowed_receipt_signers:
        ok_dids = {str(r.verification_method).split('#', 1)[0] for r in rres if r.ok and r.verification_method}
        if not (ok_dids & allowed_receipt_signers):
            print("FAIL: receipt not signed by an allowed trust anchor", file=sys.stderr)
            return 2

    # Verify checkpoint proof(s)
    cres = verify_credential(checkpoint)
    if not cres or any(not r.ok for r in cres):
        print("FAIL: checkpoint signature invalid", file=sys.stderr)
        return 2
    if enforce_trust and allowed_checkpoint_signers:
        ok_dids = {str(r.verification_method).split('#', 1)[0] for r in cres if r.ok and r.verification_method}
        if not (ok_dids & allowed_checkpoint_signers):
            print("FAIL: checkpoint not signed by an allowed trust anchor", file=sys.stderr)
            return 2

    # Verify receipt next_root
    computed = corridor_state_next_root(receipt)
    if computed != str(receipt.get("next_root") or ""):
        print("FAIL: receipt next_root mismatch", file=sys.stderr)
        return 2

    # Ensure proof binds to this receipt digest
    if str(proof.get("receipt_next_root") or "").strip().lower() != str(receipt.get("next_root") or "").strip().lower():
        print("FAIL: proof receipt_next_root does not match receipt", file=sys.stderr)
        return 2

    # Optional: sequence binding
    try:
        if int(proof.get("leaf_index") or -1) != int(receipt.get("sequence") or -2):
            print("FAIL: proof.leaf_index does not match receipt.sequence", file=sys.stderr)
            return 2
    except Exception:
        pass

    # Ensure proof binds to checkpoint root
    cp_mmr = (checkpoint or {}).get("mmr") or {}
    pr_mmr = (proof or {}).get("mmr") or {}
    if str(cp_mmr.get("root") or "").strip().lower() != str(pr_mmr.get("root") or "").strip().lower():
        print("FAIL: proof mmr.root does not match checkpoint", file=sys.stderr)
        return 2
    if int(cp_mmr.get("size") or 0) != int(pr_mmr.get("size") or 0):
        print("FAIL: proof mmr.size does not match checkpoint", file=sys.stderr)
        return 2
    try:
        if int(checkpoint.get("receipt_count") or 0) and int(checkpoint.get("receipt_count") or 0) != int(cp_mmr.get("size") or 0):
            print("FAIL: checkpoint receipt_count does not match checkpoint.mmr.size", file=sys.stderr)
            return 2
    except Exception:
        pass

    # Optional proof binding: checkpoint digest commitment (v0.4.11+)
    expected_cp_digest = ""
    if isinstance((proof or {}).get("checkpoint_ref"), dict):
        expected_cp_digest = _coerce_sha256((proof or {}).get("checkpoint_ref"))
    if not expected_cp_digest:
        expected_cp_digest = str((proof or {}).get("checkpoint_digest_sha256") or "").strip().lower()
    if expected_cp_digest:
        try:
            from tools.lawpack import jcs_canonicalize  # type: ignore
            tmp = dict(checkpoint)
            tmp.pop("proof", None)
            computed_cp_digest = sha256_bytes(jcs_canonicalize(tmp))
            if computed_cp_digest != expected_cp_digest:
                print("FAIL: checkpoint digest does not match proof commitment", file=sys.stderr)
                return 2
        except Exception:
            pass

    # Optional informational bindings
    try:
        if (proof or {}).get("checkpoint_final_state_root") and (checkpoint or {}).get("final_state_root"):
            if str((proof or {}).get("checkpoint_final_state_root") or "").strip().lower() != str((checkpoint or {}).get("final_state_root") or "").strip().lower():
                print("FAIL: proof checkpoint_final_state_root does not match checkpoint", file=sys.stderr)
                return 2
    except Exception:
        pass

    # Verify MMR inclusion proof
    mmr_proof = {
        "size": int(pr_mmr.get("size") or 0),
        "root": str(pr_mmr.get("root") or "").strip().lower(),
        "leaf_index": int(proof.get("leaf_index") or 0),
        "receipt_next_root": str(proof.get("receipt_next_root") or "").strip().lower(),
        "leaf_hash": str(proof.get("leaf_hash") or "").strip().lower(),
        "peak_index": int(proof.get("peak_index") or 0),
        "peak_height": int(proof.get("peak_height") or 0),
        "path": proof.get("path"),
        "peaks": proof.get("peaks"),
    }

    if not verify_inclusion_proof(mmr_proof):
        print("FAIL: invalid inclusion proof", file=sys.stderr)
        return 2

    print("OK")
    return 0


def cmd_corridor_state_watcher_attest(args: argparse.Namespace) -> int:
    """Create a Corridor Watcher Attestation VC referencing a checkpoint digest.

    This is intended to be cheap, externally publishable evidence of the corridor head.
    Multiple watcher attestations can be compared to detect forks quickly.
    """

    from tools.vc import now_rfc3339, add_ed25519_proof, load_proof_keypair  # type: ignore

    # Resolve corridor module
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        print("WATCHER FAIL: corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    checkpoint_path = pathlib.Path(str(getattr(args, "checkpoint", "") or "").strip())
    if not str(checkpoint_path):
        print("--checkpoint is required", file=sys.stderr)
        return 2
    if not checkpoint_path.is_absolute():
        checkpoint_path = REPO_ROOT / checkpoint_path
    if not checkpoint_path.exists():
        print(f"WATCHER FAIL: checkpoint not found: {checkpoint_path}", file=sys.stderr)
        return 2

    checkpoint = load_json(checkpoint_path)
    if str((checkpoint or {}).get("corridor_id") or "").strip() != corridor_id:
        print("WATCHER FAIL: checkpoint corridor_id does not match corridor.yaml", file=sys.stderr)
        return 2

    # Compute checkpoint payload digest (excluding proof) using JCS (aligns with inclusion-proof verification)
    try:
        from tools.lawpack import jcs_canonicalize  # type: ignore
        tmp = dict(checkpoint)
        tmp.pop("proof", None)
        cp_digest = sha256_bytes(jcs_canonicalize(tmp))
    except Exception as ex:
        print(f"WATCHER FAIL: unable to compute checkpoint digest: {ex}", file=sys.stderr)
        return 2

    out_vc = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": str(getattr(args, "id", "") or "").strip() or f"urn:msez:vc:corridor-watcher:{corridor_id}:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "MSEZCorridorWatcherAttestationCredential"],
        "issuer": str(getattr(args, "issuer", "") or "").strip(),
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "genesis_root": str(checkpoint.get("genesis_root") or "").strip().lower(),
            "checkpoint_digest_sha256": {
                "artifact_type": "checkpoint",
                "digest_sha256": cp_digest,
                "uri": os.path.relpath(checkpoint_path, module_dir),
            },
            "receipt_count": int(checkpoint.get("receipt_count") or 0),
            "final_state_root": str(checkpoint.get("final_state_root") or "").strip().lower(),
            "mmr_root": str(((checkpoint.get("mmr") or {}) or {}).get("root") or "").strip().lower(),
            "observed_at": str(getattr(args, "observed_at", "") or "").strip() or now_rfc3339(),
            "no_fork_observed": bool(getattr(args, "no_fork_observed", False)),
        },
    }

    # Optional explicit finality claim.
    fl = str(getattr(args, "finality_level", "") or "").strip()
    if fl:
        out_vc["credentialSubject"]["finality_level"] = fl
    else:
        # Best-effort: if the checkpoint carries at least one valid signature, we can
        # conservatively label it as checkpoint_signed.
        try:
            from tools.vc import verify_credential  # type: ignore
            if any(r.ok for r in verify_credential(checkpoint)):
                out_vc["credentialSubject"]["finality_level"] = "checkpoint_signed"
        except Exception:
            pass

    # Compute deterministic head commitment digest for gossip-friendly dedupe.
    try:
        out_vc["credentialSubject"]["head_commitment_digest_sha256"] = corridor_head_commitment_digest(
            corridor_id=corridor_id,
            genesis_root=str(out_vc["credentialSubject"].get("genesis_root") or ""),
            receipt_count=int(out_vc["credentialSubject"].get("receipt_count") or 0),
            final_state_root=str(out_vc["credentialSubject"].get("final_state_root") or ""),
            mmr_root=str(out_vc["credentialSubject"].get("mmr_root") or ""),
        )
    except Exception:
        # Best-effort; schema treats this field as optional.
        pass

    if not out_vc["issuer"]:
        print("WATCHER FAIL: --issuer is required", file=sys.stderr)
        return 2

    # Optional: store checkpoint into the local artifact CAS for resolvability
    if bool(getattr(args, "store_artifacts", False)):
        try:
            artifact_cas.store_artifact_file(
                artifact_type="checkpoint",
                digest_sha256=cp_digest,
                src_file=checkpoint_path,
                repo_root=REPO_ROOT,
            )
        except Exception as ex:
            print(f"WATCHER WARN: unable to store checkpoint in CAS: {ex}", file=sys.stderr)

    if bool(getattr(args, "sign", False)):
        key_path = pathlib.Path(str(getattr(args, "key", "") or "").strip())
        if not str(key_path):
            print("--key is required with --sign", file=sys.stderr)
            return 2
        if not key_path.is_absolute():
            key_path = REPO_ROOT / key_path
        priv, vm = load_proof_keypair(key_path)
        add_ed25519_proof(out_vc, priv, vm)

    out_path = pathlib.Path(str(getattr(args, "out", "") or "").strip() or f"corridor.watcher.{corridor_id}.vc.unsigned.json")
    if not out_path.is_absolute():
        out_path = module_dir / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out_vc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("Wrote watcher attestation VC to", out_path)
    return 0



def cmd_corridor_state_watcher_compare(args: argparse.Namespace) -> int:
    """Compare watcher attestation VCs and flag divergent heads.

    This aggregator is designed to be *cheap*:
    - it does not require corridor receipts,
    - it can optionally enforce a watcher allow-list (authority registry),
    - it can evaluate a quorum threshold (K-of-N watchers agree on a head) using only
      watcher attestations.

    Fork-like divergence rule:
      If >=2 watchers attest to the same `receipt_count` but different `final_state_root`,
      treat as a **critical** divergence.
    """

    # Resolve module directory
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        print(f"WATCHER-COMPARE FAIL: missing corridor.yaml in {module_dir}", file=sys.stderr)
        return 2
    cfg = load_yaml(corridor_path)
    corridor_id = str(cfg.get("corridor_id") or "").strip()
    if not corridor_id:
        print("WATCHER-COMPARE FAIL: corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    # Load optional authority registry chain (watcher allow-list)
    reg_allowed, reg_errs = load_authority_registry(module_dir, cfg if isinstance(cfg, dict) else {})
    allowed_watchers = set(reg_allowed.get("corridor_watcher_attestation", set())) | set(reg_allowed.get("*", set()))
    enforce_registry = bool(getattr(args, "enforce_authority_registry", False))
    if enforce_registry and reg_errs:
        print("WATCHER-COMPARE FAIL: authority registry errors:", file=sys.stderr)
        for e in reg_errs:
            print("  -", e, file=sys.stderr)
        return 2

    # Determine quorum policy defaults (optionally from Corridor Agreement VCs).
    quorum_from_agreement: Dict[str, Any] = {}
    quorum_policy_warnings: List[str] = []

    def _load_watcher_quorum_policy_from_agreements() -> Dict[str, Any]:
        out: Dict[str, Any] = {}
        paths = _agreement_paths(cfg if isinstance(cfg, dict) else {})
        if not paths:
            return out
        found: List[Dict[str, Any]] = []
        for rel in paths:
            pth = (module_dir / rel).resolve()
            if not pth.exists():
                continue
            try:
                vcj = load_json(pth)
            except Exception:
                continue
            cs = (vcj.get("credentialSubject") or {}) if isinstance(vcj, dict) else {}
            sc = (cs.get("state_channel") or {}) if isinstance(cs, dict) else {}
            wq = (sc.get("watcher_quorum") or {}) if isinstance(sc, dict) else {}
            if not isinstance(wq, dict) or not wq:
                continue

            pol: Dict[str, Any] = {}
            mode = str(wq.get("mode") or "").strip() or "optional"
            if mode not in ("optional", "required"):
                mode = "optional"
            pol["mode"] = mode

            # Accept either string threshold (majority, 3/5) or required/of fields.
            thr = str(wq.get("threshold") or "").strip()
            req = int(wq.get("required") or 0) if str(wq.get("required") or "").strip() else 0
            ofv = int(wq.get("of") or 0) if str(wq.get("of") or "").strip() else 0
            if thr:
                pol["threshold"] = thr
            elif req and ofv:
                pol["threshold"] = f"{req}/{ofv}"
            elif req:
                # If only required is specified, treat denominator as dynamic.
                pol["threshold"] = f"{req}/0"

            ms = wq.get("max_staleness")
            if isinstance(ms, int):
                pol["max_staleness_seconds"] = int(ms)
            elif isinstance(ms, str):
                sec = parse_duration_to_seconds(ms)
                if sec:
                    pol["max_staleness_seconds"] = sec
            else:
                if wq.get("max_staleness_seconds") is not None:
                    try:
                        pol["max_staleness_seconds"] = int(wq.get("max_staleness_seconds") or 0)
                    except Exception:
                        pass

            found.append(pol)

        if not found:
            return out

        # If multiple agreements define the policy, require consistency.
        base = found[0]
        for other in found[1:]:
            if other != base:
                quorum_policy_warnings.append(
                    "agreement watcher_quorum policies differ across agreement_vc_path; using the first policy"
                )
                break
        return base

    quorum_from_agreement = _load_watcher_quorum_policy_from_agreements()

    # Collect VC files
    src = pathlib.Path(getattr(args, "vcs", "") or "")
    if not src.is_absolute():
        src = REPO_ROOT / src
    src = src.resolve()
    vc_paths: List[pathlib.Path] = []
    if src.is_file():
        vc_paths = [src]
    elif src.is_dir():
        vc_paths = sorted([p for p in src.glob("*.json") if p.is_file()])
    else:
        print(f"WATCHER-COMPARE FAIL: not found: {src}", file=sys.stderr)
        return 2

    schema_path = REPO_ROOT / "schemas/vc.corridor-watcher-attestation.schema.json"
    schema = schema_validator(schema_path)

    try:
        from tools.vc import base_did, verify_credential  # type: ignore
    except Exception as ex:
        print(f"WATCHER-COMPARE FAIL: cannot import VC verifier: {ex}", file=sys.stderr)
        return 2

    require_artifacts = bool(getattr(args, "require_artifacts", False))

    # Staleness policy
    max_staleness_seconds = parse_duration_to_seconds(str(getattr(args, "max_staleness", "") or ""))
    if not max_staleness_seconds:
        try:
            max_staleness_seconds = int(quorum_from_agreement.get("max_staleness_seconds") or 0)
        except Exception:
            max_staleness_seconds = 0
    if not max_staleness_seconds:
        max_staleness_seconds = 3600  # default 1h

    # Quorum threshold policy
    threshold_str = str(getattr(args, "quorum_threshold", "") or "").strip() or str(quorum_from_agreement.get("threshold") or "").strip() or "majority"
    require_quorum = bool(getattr(args, "require_quorum", False)) or (str(quorum_from_agreement.get("mode") or "") == "required")

    # Output format
    fmt = str(getattr(args, "format", "") or "").strip() or "text"
    if bool(getattr(args, "json", False)):
        fmt = "json"

    entries: List[Dict[str, Any]] = []
    errors: List[str] = []
    warns: List[str] = []

    def _issuer_id(vcj: Dict[str, Any]) -> str:
        iss = vcj.get("issuer")
        if isinstance(iss, str):
            return iss
        if isinstance(iss, dict):
            return str(iss.get("id") or "")
        return ""

    def _short(h: str, n: int = 12) -> str:
        hh = str(h or "").strip()
        return (hh[:n] + "…") if len(hh) > n else hh

    from datetime import datetime, timezone

    def _parse_dt(s: str) -> datetime | None:
        ss = str(s or "").strip()
        if not ss:
            return None
        try:
            if ss.endswith("Z"):
                ss = ss[:-1] + "+00:00"
            dt = datetime.fromisoformat(ss)
            if dt.tzinfo is None:
                dt = dt.replace(tzinfo=timezone.utc)
            return dt.astimezone(timezone.utc)
        except Exception:
            return None

    now = datetime.now(timezone.utc)

    for vp in vc_paths:
        try:
            vc = load_json(vp)
        except Exception as ex:
            warns.append(f"{vp}: unreadable JSON: {ex}")
            continue
        if not isinstance(vc, dict):
            warns.append(f"{vp}: VC must be a JSON object")
            continue

        verrs = validate_with_schema(vc, schema)
        if verrs:
            warns.append(f"{vp}: schema invalid (likely missing proof; use --sign): {verrs[0]}")
            continue

        results = verify_credential(vc)
        ok_methods = [r.verification_method for r in results if getattr(r, "ok", False)]
        if not ok_methods:
            warns.append(f"{vp}: invalid VC signature(s)")
            continue

        signers = sorted({base_did(vm) for vm in ok_methods if base_did(vm)})
        if enforce_registry and allowed_watchers:
            if not (set(signers) & allowed_watchers):
                warns.append(f"{vp}: signer(s) {signers} not authorized for corridor_watcher_attestation by authority registry")
                continue

        cs = vc.get("credentialSubject") or {}
        if not isinstance(cs, dict):
            warns.append(f"{vp}: credentialSubject must be an object")
            continue

        cs_cid = str(cs.get("corridor_id") or "").strip()
        if cs_cid and cs_cid != corridor_id:
            warns.append(f"{vp}: corridor_id mismatch (VC={cs_cid} vs module={corridor_id})")
            continue

        try:
            receipt_count = int(cs.get("receipt_count") or 0)
        except Exception:
            receipt_count = 0
        final_state_root = str(cs.get("final_state_root") or "").strip().lower()
        mmr_root = str(cs.get("mmr_root") or "").strip().lower()
        genesis_root = str(cs.get("genesis_root") or "").strip().lower()
        observed_at = str(cs.get("observed_at") or "").strip()
        checkpoint_digest = _coerce_sha256(cs.get("checkpoint_digest_sha256"))
        provided_head_digest = str(cs.get("head_commitment_digest_sha256") or "").strip().lower()

        if receipt_count <= 0 or not final_state_root:
            warns.append(f"{vp}: missing receipt_count/final_state_root")
            continue

        # Staleness computation (for liveness monitoring)
        dt_obs = _parse_dt(observed_at)
        staleness_seconds = 0
        stale = False
        if dt_obs is None:
            stale = True
        else:
            staleness_seconds = int((now - dt_obs).total_seconds())
            stale = (max_staleness_seconds > 0) and (staleness_seconds > max_staleness_seconds)

        # Head commitment digest (stable dedupe key)
        computed_head_digest = ""
        try:
            computed_head_digest = corridor_head_commitment_digest(
                corridor_id=corridor_id,
                genesis_root=genesis_root,
                receipt_count=receipt_count,
                final_state_root=final_state_root,
                mmr_root=mmr_root,
            )
        except Exception:
            computed_head_digest = ""

        head_digest = provided_head_digest or computed_head_digest
        head_digest_mismatch = bool(provided_head_digest and computed_head_digest and provided_head_digest != computed_head_digest)

        # Optional commitment completeness: ensure referenced checkpoint is locally resolvable.
        checkpoint_resolved_path = ""
        if require_artifacts and checkpoint_digest:
            try:
                resolved = artifact_cas.resolve_artifact_by_digest("checkpoint", checkpoint_digest, repo_root=REPO_ROOT)
                checkpoint_resolved_path = str(resolved)
            except Exception as ex:
                errors.append(f"{vp}: missing committed checkpoint artifact (digest {checkpoint_digest}): {ex}")

        issuer = base_did(_issuer_id(vc))
        watcher_did = issuer or (signers[0] if signers else "")

        if head_digest_mismatch:
            warns.append(f"{vp}: head_commitment_digest_sha256 does not match computed value; using computed")
            if computed_head_digest:
                head_digest = computed_head_digest

        entries.append(
            {
                "path": str(vp),
                "watcher_did": watcher_did,
                "issuer": issuer,
                "signers": signers,
                "proof_ok": True,
                "authorized": (not enforce_registry) or (not allowed_watchers) or bool(set(signers) & allowed_watchers),
                "observed_at": observed_at,
                "stale": stale,
                "staleness_seconds": staleness_seconds,
                "finality_level": str(cs.get("finality_level") or "").strip(),
                "receipt_count": receipt_count,
                "final_state_root": final_state_root,
                "mmr_root": mmr_root,
                "genesis_root": genesis_root,
                "head_commitment_digest_sha256": head_digest,
                "checkpoint_digest_sha256": checkpoint_digest,
                "checkpoint_resolved_path": checkpoint_resolved_path,
                "included_in_quorum": False,
            }
        )

    if not entries:
        print("WATCHER-COMPARE FAIL: no valid watcher attestations found", file=sys.stderr)
        for w in warns[:10]:
            print("  WARN:", w, file=sys.stderr)
        return 2

    # Consider only *fresh* attestations for fork/quorum analysis.
    fresh_entries = [e for e in entries if not bool(e.get("stale"))]
    by_count: Dict[int, List[Dict[str, Any]]] = {}
    for e in fresh_entries:
        by_count.setdefault(int(e.get("receipt_count") or 0), []).append(e)

    fork_points: List[Dict[str, Any]] = []
    for rc, ents in sorted(by_count.items(), key=lambda x: x[0]):
        by_root: Dict[str, List[Dict[str, Any]]] = {}
        for e in ents:
            by_root.setdefault(str(e.get("final_state_root") or ""), []).append(e)
        if len(by_root) > 1:
            fork_points.append(
                {
                    "receipt_count": rc,
                    "branches": [
                        {
                            "final_state_root": fr,
                            "watchers": sorted({s for ee in el for s in (ee.get("signers") or [])}),
                            "vc_paths": [str(ee.get("path") or "") for ee in el],
                        }
                        for fr, el in sorted(by_root.items(), key=lambda x: x[0])
                    ],
                }
            )

    fork_detected = bool(fork_points)
    lag_detected = len(by_count) > 1

    # Quorum computation: group by head_commitment_digest_sha256.
    by_head: Dict[str, List[Dict[str, Any]]] = {}
    for e in fresh_entries:
        hd = str(e.get("head_commitment_digest_sha256") or "").strip().lower()
        if not hd:
            continue
        by_head.setdefault(hd, []).append(e)

    # Determine watcher universe size (N) for K-of-N thresholds.
    if enforce_registry and allowed_watchers:
        watcher_universe = len(allowed_watchers)
    else:
        watcher_universe = len(sorted({str(e.get("watcher_did") or "") for e in entries if str(e.get("watcher_did") or "")}))

    def _parse_threshold(thr: str, universe: int) -> Tuple[int, int, str]:
        t = str(thr or "").strip().lower() or "majority"
        ofv = int(universe or 0)
        if t in ("majority", "maj"):
            req = (ofv // 2) + 1 if ofv > 0 else 0
            return (req, ofv, "majority")
        m = re.fullmatch(r"\s*(\d+)\s*/\s*(\d+)\s*", t)
        if m:
            req = int(m.group(1))
            den = int(m.group(2))
            return (req, den if den > 0 else ofv, f"{req}/{den}")
        # Fallback: treat as majority
        req = (ofv // 2) + 1 if ofv > 0 else 0
        return (req, ofv, "majority")

    quorum_required, quorum_of, quorum_label = _parse_threshold(threshold_str, watcher_universe)

    agreed_head: Dict[str, Any] = {}
    agreeing_watchers = 0
    best_head_digest = ""
    for hd, ents in by_head.items():
        wset = sorted({str(e.get("watcher_did") or "") for e in ents if str(e.get("watcher_did") or "")})
        if len(wset) > agreeing_watchers:
            agreeing_watchers = len(wset)
            best_head_digest = hd
            # Use the max receipt_count among this group (should be identical, but be defensive)
            sample = sorted(ents, key=lambda x: int(x.get("receipt_count") or 0), reverse=True)[0]
            agreed_head = {
                "receipt_count": int(sample.get("receipt_count") or 0),
                "final_state_root": str(sample.get("final_state_root") or ""),
                "mmr_root": str(sample.get("mmr_root") or ""),
                "head_commitment_digest_sha256": hd,
            }

    quorum_reached = (agreeing_watchers >= quorum_required) if quorum_required else False
    if fork_detected:
        # Forks override any apparent quorum.
        quorum_reached = False

    # Mark entries that match the agreed head (for dashboards/debug)
    if best_head_digest:
        for e in entries:
            if str(e.get("head_commitment_digest_sha256") or "").strip().lower() == best_head_digest and not bool(e.get("stale")):
                e["included_in_quorum"] = True

    # Divergences list (machine-friendly)
    divergences: List[Dict[str, Any]] = []
    if fork_detected:
        divergences.append(
            {
                "type": "fork_like_divergence",
                "severity": "critical",
                "message": "Multiple watcher heads at the same receipt_count with different final_state_root",
                "details": {"fork_points": fork_points},
            }
        )
    if lag_detected and not fork_detected:
        divergences.append(
            {
                "type": "lag_divergence",
                "severity": "warn",
                "message": "Watchers are out-of-sync on receipt_count (lag)",
                "details": {"receipt_counts": sorted(list(by_count.keys()))},
            }
        )

    # Informational: checkpoint digest divergence for the same head digest (expected if timestamp fields differ)
    for hd, ents in by_head.items():
        cds = sorted({str(e.get("checkpoint_digest_sha256") or "") for e in ents if str(e.get("checkpoint_digest_sha256") or "")})
        if len(cds) > 1:
            divergences.append(
                {
                    "type": "checkpoint_digest_divergence",
                    "severity": "info",
                    "message": "Watchers reference different checkpoint digests for the same head (likely timestamp/proof metadata differences)",
                    "details": {"head_commitment_digest_sha256": hd, "checkpoint_digests": cds},
                }
            )

    if not fresh_entries:
        divergences.append(
            {
                "type": "no_fresh_attestations",
                "severity": "warn",
                "message": "No watcher attestations are within the staleness window",
                "details": {"max_staleness_seconds": max_staleness_seconds},
            }
        )

    if quorum_policy_warnings:
        for w in quorum_policy_warnings:
            divergences.append({"type": "policy_warning", "severity": "info", "message": w, "details": {}})

    # Summary
    distinct_watchers = len(sorted({str(e.get("watcher_did") or "") for e in entries if str(e.get("watcher_did") or "")}))
    rc_vals = sorted([int(e.get("receipt_count") or 0) for e in entries])
    summary: Dict[str, Any] = {
        "total_attestations": len(vc_paths),
        "valid_attestations": len(entries),
        "fresh_attestations": len(fresh_entries),
        "distinct_watchers": distinct_watchers,
        "max_receipt_count": max(rc_vals) if rc_vals else 0,
        "min_receipt_count": min(rc_vals) if rc_vals else 0,
    }

    from tools.vc import now_rfc3339  # type: ignore
    report: Dict[str, Any] = {
        "type": "MSEZWatcherCompareResult",
        "corridor_id": corridor_id,
        "analysis_timestamp": now_rfc3339(),
        "summary": summary,
        "attestations": entries,
        "divergences": divergences,
        "quorum": {
            "threshold": quorum_label if quorum_label else threshold_str,
            "required": int(quorum_required),
            "of": int(quorum_of),
            "max_staleness_seconds": int(max_staleness_seconds),
            "fresh_watchers": len(sorted({str(e.get("watcher_did") or "") for e in fresh_entries if str(e.get("watcher_did") or "")})),
            "agreeing_watchers": int(agreeing_watchers),
            "reached": bool(quorum_reached),
            "finality_level": "watcher_quorum" if quorum_reached else "checkpoint_signed" if fresh_entries else "receipt_signed",
            "agreed_head": agreed_head,
            "require_quorum": bool(require_quorum),
        },

        # Legacy keys for backward compatibility with v0.4.16 JSON
        "fork_detected": fork_detected,
        "lag_detected": lag_detected,
        "max_receipt_count": summary.get("max_receipt_count", 0),
        "fork_points": fork_points,
        "warnings": warns,
        "errors": errors,
    }

    # Emit report
    out_path = str(getattr(args, "out", "") or "").strip()
    payload = json.dumps(report, indent=2, ensure_ascii=False) if fmt == "json" else ""

    if fmt == "json":
        if out_path:
            op = pathlib.Path(out_path)
            if not op.is_absolute():
                op = module_dir / op
            op.parent.mkdir(parents=True, exist_ok=True)
            op.write_text(payload + "\n", encoding="utf-8")
            print("Wrote watcher-compare report to", op)
        else:
            print(payload)
    else:
        print(f"WATCHER-COMPARE: corridor_id={corridor_id}")
        if enforce_registry and allowed_watchers:
            print(f"  authority-registry: enforcing watcher allow-list (n={len(allowed_watchers)})")
        print(f"  staleness window: {max_staleness_seconds}s")
        print(f"  inputs: {len(vc_paths)} file(s) | valid: {len(entries)} | fresh: {len(fresh_entries)}")
        print("  heads observed (fresh):")
        for rc, ents in sorted(by_count.items(), key=lambda x: x[0]):
            roots: Dict[str, List[str]] = {}
            for e in ents:
                roots.setdefault(e.get("final_state_root") or "", []).append(e.get("watcher_did") or "")
            for fr, watchers in sorted(roots.items(), key=lambda x: x[0]):
                uniq = sorted({w for w in watchers if w})
                print(f"    - receipt_count={rc} final_state_root={_short(fr)} watchers={uniq}")

        if fork_detected:
            print("  RESULT: ALARM (fork-like divergence detected)")
            for fp in fork_points:
                print(f"    * receipt_count={fp.get('receipt_count')}: {len(fp.get('branches') or [])} competing roots")
        else:
            if lag_detected:
                print("  RESULT: OK (no fork detected) — NOTE: watchers are out-of-sync on receipt_count")
            else:
                print("  RESULT: OK (watchers agree)")

        print(f"  QUORUM: threshold={threshold_str} required={quorum_required} of={quorum_of} agreeing={agreeing_watchers} reached={quorum_reached}")
        if agreed_head:
            print(f"    - agreed receipt_count={agreed_head.get('receipt_count')} head={_short(agreed_head.get('final_state_root') or '')}")

        if require_quorum and not quorum_reached:
            print("  QUORUM RESULT: FAIL (require_quorum enabled)")

        if errors:
            print("  ERRORS:")
            for e in errors:
                print("    -", e)
        if warns:
            print("  WARNINGS:")
            for w in warns[:10]:
                print("    -", w)

        if out_path:
            # Allow writing a text report to file (useful for operators)
            op = pathlib.Path(out_path)
            if not op.is_absolute():
                op = module_dir / op
            op.parent.mkdir(parents=True, exist_ok=True)
            op.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
            print("  (also wrote JSON report to", op, ")")

    # Exit policy
    if errors:
        return 2
    if fork_detected:
        return 2
    if lag_detected and bool(getattr(args, "fail_on_lag", False)):
        return 1
    if require_quorum and not quorum_reached:
        return 1
    return 0


def cmd_corridor_state_fork_alarm(args: argparse.Namespace) -> int:
    """Create a Corridor Fork Alarm VC with evidence references.

    The fork alarm asserts two conflicting receipts exist for the same (sequence, prev_root).
    """

    from tools.vc import now_rfc3339, add_ed25519_proof, load_proof_keypair  # type: ignore

    # Resolve corridor module
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        print("FORK FAIL: corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    ra = pathlib.Path(str(getattr(args, "receipt_a", "") or "").strip())
    rb = pathlib.Path(str(getattr(args, "receipt_b", "") or "").strip())
    if not str(ra) or not str(rb):
        print("--receipt-a and --receipt-b are required", file=sys.stderr)
        return 2
    if not ra.is_absolute():
        ra = REPO_ROOT / ra
    if not rb.is_absolute():
        rb = REPO_ROOT / rb
    if not ra.exists() or not rb.exists():
        print("FORK FAIL: receipt file not found", file=sys.stderr)
        return 2

    a = load_json(ra)
    b = load_json(rb)

    for obj_name, obj in [("receipt-a", a), ("receipt-b", b)]:
        if str((obj or {}).get("corridor_id") or "").strip() != corridor_id:
            print(f"FORK FAIL: {obj_name}.corridor_id does not match corridor.yaml", file=sys.stderr)
            return 2

    seq_a = int((a or {}).get("sequence") or -1)
    seq_b = int((b or {}).get("sequence") or -1)
    if seq_a != seq_b:
        print("FORK FAIL: receipts have different sequence", file=sys.stderr)
        return 2

    prev_a = str((a or {}).get("prev_root") or "").strip().lower()
    prev_b = str((b or {}).get("prev_root") or "").strip().lower()
    if prev_a != prev_b:
        print("FORK FAIL: receipts have different prev_root", file=sys.stderr)
        return 2

    next_a = str((a or {}).get("next_root") or "").strip().lower()
    next_b = str((b or {}).get("next_root") or "").strip().lower()
    if next_a == next_b:
        print("FORK FAIL: receipts do not conflict (same next_root)", file=sys.stderr)
        return 2

    da = sha256_bytes(ra.read_bytes())
    db = sha256_bytes(rb.read_bytes())

    out_vc = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": str(getattr(args, "id", "") or "").strip() or f"urn:msez:vc:corridor-fork-alarm:{corridor_id}:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "MSEZCorridorForkAlarmCredential"],
        "issuer": str(getattr(args, "issuer", "") or "").strip(),
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "sequence": seq_a,
            "prev_root": prev_a,
            "next_root_a": next_a,
            "next_root_b": next_b,
            "receipt_a": {
                "artifact_type": "blob",
                "digest_sha256": da,
                "uri": os.path.relpath(ra, module_dir),
            },
            "receipt_b": {
                "artifact_type": "blob",
                "digest_sha256": db,
                "uri": os.path.relpath(rb, module_dir),
            },
            "detected_at": str(getattr(args, "detected_at", "") or "").strip() or now_rfc3339(),
        },
    }

    if not out_vc["issuer"]:
        print("FORK FAIL: --issuer is required", file=sys.stderr)
        return 2

    if bool(getattr(args, "store_artifacts", False)):
        try:
            artifact_cas.store_artifact_file("blob", da, ra, repo_root=REPO_ROOT)
            artifact_cas.store_artifact_file("blob", db, rb, repo_root=REPO_ROOT)
        except Exception as ex:
            print(f"FORK WARN: unable to store evidence blobs in CAS: {ex}", file=sys.stderr)

    if bool(getattr(args, "sign", False)):
        key_path = pathlib.Path(str(getattr(args, "key", "") or "").strip())
        if not str(key_path):
            print("--key is required with --sign", file=sys.stderr)
            return 2
        if not key_path.is_absolute():
            key_path = REPO_ROOT / key_path
        priv, vm = load_proof_keypair(key_path)
        add_ed25519_proof(out_vc, priv, vm)

    out_path = pathlib.Path(str(getattr(args, "out", "") or "").strip() or f"corridor.fork-alarm.{corridor_id}.vc.unsigned.json")
    if not out_path.is_absolute():
        out_path = module_dir / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(out_vc, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("Wrote fork alarm VC to", out_path)
    return 0



def cmd_vc_keygen(args: argparse.Namespace) -> int:
    """Generate an Ed25519 OKP JWK (did:key compatible)."""
    from tools.vc import generate_ed25519_jwk, public_jwk_from_private_jwk, b64url_decode, did_key_from_ed25519_public_key  # type: ignore

    out_path = pathlib.Path(args.out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)

    jwk = generate_ed25519_jwk(kid=getattr(args, "kid", "key-1") or "key-1")
    out_path.write_text(json.dumps(jwk, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # Optional public-only JWK output
    pub_out = getattr(args, "public_out", "") or ""
    if pub_out:
        pub_path = pathlib.Path(pub_out)
        if not pub_path.is_absolute():
            pub_path = REPO_ROOT / pub_path
        pub_path.parent.mkdir(parents=True, exist_ok=True)
        pub_path.write_text(json.dumps(public_jwk_from_private_jwk(jwk), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # Print derived did:key for convenience
    pub_bytes = b64url_decode(jwk["x"])
    did = did_key_from_ed25519_public_key(pub_bytes)
    print(did)
    return 0

def cmd_vc_sign(args: argparse.Namespace) -> int:
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    key_path = pathlib.Path(args.key)
    if not key_path.is_absolute():
        key_path = REPO_ROOT / key_path

    vcj = load_json(cred_path)
    jwk = load_json(key_path)

    from tools.vc import load_ed25519_private_key_from_jwk, add_ed25519_proof  # type: ignore
    priv, did = load_ed25519_private_key_from_jwk(jwk)

    vm = args.verification_method.strip() if args.verification_method else (did + "#key-1")
    add_ed25519_proof(vcj, priv, vm, proof_purpose=args.purpose)

    out_path = pathlib.Path(args.out) if args.out else cred_path.with_name(cred_path.stem + ".signed.json")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("Wrote signed VC to", out_path)
    return 0


def cmd_vc_verify(args: argparse.Namespace) -> int:
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    vcj = load_json(cred_path)

    from tools.vc import verify_credential  # type: ignore
    results = verify_credential(vcj)
    ok = True
    for r in results:
        if r.ok:
            print("OK  ", r.verification_method)
        else:
            ok = False
            print("FAIL", r.verification_method, "-", r.error)
    if not results:
        print("FAIL: no proofs present")
        return 2
    return 0 if ok else 2



def cmd_vc_payload_hash(args: argparse.Namespace) -> int:
    """Print sha256 of the canonical VC payload excluding `proof`.

    This is the signing input hash used for Corridor Agreement VC binding to a Corridor Definition VC.
    """
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    vcj = load_json(cred_path)

    from tools.vc import signing_input  # type: ignore
    print(sha256_bytes(signing_input(vcj)))
    return 0


def cmd_corridor_verify(args: argparse.Namespace) -> int:
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent
    errs = verify_corridor_definition_vc(module_dir)
    errs.extend(verify_corridor_agreement_vc(module_dir))
    if errs:
        print("CORRIDOR FAIL:", module_dir)
        for e in errs:
            print("  -", e)
        return 2
    # If an agreement VC is configured, corridor verify implies activation thresholds are met.
    c = load_yaml(module_dir / "corridor.yaml")
    has_agreement = bool(_agreement_paths(c))
    if has_agreement:
        print("OK: corridor verified + activated:", module_dir)
    else:
        print("OK: corridor verified (no agreement VC configured):", module_dir)
    return 0



def cmd_corridor_vc_init_definition(args: argparse.Namespace) -> int:
    """Initialize (or update) a Corridor Definition VC template from corridor.yaml + artifact hashes.

    This is an authoring helper: it produces either an unsigned VC template or a signed VC, depending on --sign.
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk, did_key_from_ed25519_public_key, b64url_decode  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        print(f"Missing corridor.yaml in {module_dir}", file=sys.stderr)
        return 2

    c = load_yaml(corridor_path)
    corridor_id = str(c.get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    issuer = str(getattr(args, "issuer", "") or "").strip()
    if not issuer:
        print("--issuer is required", file=sys.stderr)
        return 2

    ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
    kr_rel = str(c.get("key_rotation_path") or "key-rotation.yaml")
    ta_path = module_dir / ta_rel
    kr_path = module_dir / kr_rel
    if not ta_path.exists():
        print(f"Missing {ta_rel}", file=sys.stderr)
        return 2
    if not kr_path.exists():
        print(f"Missing {kr_rel}", file=sys.stderr)
        return 2

    ruleset = str(getattr(args, "ruleset", "") or c.get("verification_ruleset") or "msez.corridor.verification.v1").strip()
    vc_id = str(getattr(args, "id", "") or "").strip() or f"urn:msez:vc:corridor-definition:{corridor_id}"
    maintainer = str(getattr(args, "maintainer", "") or "").strip()
    version = str(getattr(args, "version", "") or "").strip()

    vcj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorDefinitionCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "ruleset": ruleset,
            "artifacts": {
                "corridor_manifest": {"path": "corridor.yaml", "sha256": sha256_file(corridor_path)},
                "trust_anchors": {"path": ta_rel, "sha256": sha256_file(ta_path)},
                "key_rotation": {"path": kr_rel, "sha256": sha256_file(kr_path)},
            },
        },
    }

    # Optional transition type registry artifact pins (v0.4.4+) and lock pin (v0.4.5+)
    try:
        state_cfg = c.get("state_channel") if isinstance(c, dict) else None
        if not isinstance(state_cfg, dict):
            state_cfg = {}

        ttr_rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
        ttl_rel = str(state_cfg.get("transition_type_registry_lock_path") or "").strip()

        # Derive a default lock name when a registry is configured.
        if not ttl_rel and ttr_rel:
            rp = pathlib.Path(ttr_rel)
            if rp.suffix.lower() in {".yaml", ".yml"}:
                ttl_rel = str(rp.with_suffix(".lock.json"))
            else:
                ttl_rel = ttr_rel + ".lock.json"

        # Pin the lock artifact first (preferred).
        if ttl_rel:
            ttl_path = _resolve_path_repo_or_module(module_dir, ttl_rel)
            if not ttl_path.exists():
                print(f"Missing {ttl_rel}", file=sys.stderr)
                return 2
            vcj["credentialSubject"]["artifacts"]["transition_type_registry_lock"] = {
                "path": ttl_rel,
                "sha256": sha256_file(ttl_path),
            }

        # Optionally also pin the human-authored registry YAML.
        if ttr_rel:
            ttr_path = _resolve_path_repo_or_module(module_dir, ttr_rel)
            if not ttr_path.exists():
                print(f"Missing {ttr_rel}", file=sys.stderr)
                return 2
            vcj["credentialSubject"]["artifacts"]["transition_type_registry"] = {
                "path": ttr_rel,
                "sha256": sha256_file(ttr_path),
            }
    except Exception as ex:
        print(f"ERROR: unable to pin transition type registry artifacts: {ex}", file=sys.stderr)
        return 2

    # Ruleset digest pins (v0.4.3+)
    # Provides content-addressed identifiers for the verifier logic governing this corridor.
    try:
        state_cfg = c.get("state_channel") if isinstance(c, dict) else None
        if not isinstance(state_cfg, dict):
            state_cfg = {}
        transition_ruleset = str(state_cfg.get("transition_ruleset") or "msez.corridor.state-transition.v2").strip()
        vcj["credentialSubject"]["ruleset_digest_set"] = _normalize_digest_set([
            ruleset_descriptor_digest_sha256(ruleset),
            ruleset_descriptor_digest_sha256(transition_ruleset),
        ])
    except Exception:
        # Best-effort: allow authoring even if ruleset registry is missing.
        pass

    # Lawpack compatibility scaffold (v0.4.1+)
    # By default we require each participant to pin (and sign) civil + financial lawpacks.
    # To additionally *constrain* which digests are compatible, pass --allow-lawpack one or more times.
    from tools.lawpack import parse_lawpack_ref  # type: ignore
    req_domains = getattr(args, "require_domain", None) or ["civil", "financial"]
    allow_refs = getattr(args, "allow_lawpack", None) or []
    lawpack_compat: Dict[str, Any] = {"required_domains": req_domains}

    if allow_refs:
        allowed_map: Dict[Tuple[str, str], set] = {}
        for ref in allow_refs:
            lr = parse_lawpack_ref(ref)
            key = (lr["jurisdiction_id"], lr["domain"])
            allowed_map.setdefault(key, set()).add(lr["lawpack_digest_sha256"])
        allowed_list: List[Dict[str, Any]] = []
        for (jid, dom), digests in sorted(allowed_map.items(), key=lambda x: (x[0][0], x[0][1])):
            allowed_list.append({"jurisdiction_id": jid, "domain": dom, "digests_sha256": sorted(digests)})
        lawpack_compat["allowed"] = allowed_list

    vcj["credentialSubject"]["lawpack_compatibility"] = lawpack_compat

    if version:
        vcj["credentialSubject"]["version"] = version
    if maintainer:
        vcj["credentialSubject"]["maintainer"] = maintainer

    # Output path
    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if getattr(args, "sign", False):
            out_path = module_dir / (str(c.get("definition_vc_path") or "corridor.vc.json"))
        else:
            out_path = module_dir / "corridor.vc.unsigned.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # Optional signing
    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)
        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            # If issuer is did:key and matches derived did, use issuer#key-1; else use derived did#key-1
            issuer_base = issuer.split("#", 1)[0]
            vm_did = issuer_base if issuer_base.startswith("did:key:") else did
            vm = f"{vm_did}#key-1"
        add_ed25519_proof(vcj, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))
    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_vc_init_agreement(args: argparse.Namespace) -> int:
    """Initialize a Corridor Agreement VC template from a corridor package.

    Generates participants from trust-anchors.yaml entries that authorize 'corridor_agreement'.
    Defaults to unanimous thresholds per role (safe default); edit the output for alternative governance.
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk, signing_input  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        print(f"Missing corridor.yaml in {module_dir}", file=sys.stderr)
        return 2
    c = load_yaml(corridor_path)
    corridor_id = str(c.get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    party = str(getattr(args, "party", "") or "").strip()
    issuer = str(getattr(args, "issuer", "") or "").strip()
    if not issuer:
        issuer = party
    if not issuer:
        print("--issuer is required (or --party for participant-specific agreement)", file=sys.stderr)
        return 2

    # Resolve Corridor Definition VC and compute payload hash
    def_rel = str(c.get("definition_vc_path") or "corridor.vc.json")
    def_path = module_dir / def_rel
    if not def_path.exists():
        print(f"Missing {def_rel}", file=sys.stderr)
        return 2
    def_vcj = load_json(def_path)
    def_vc_id = str(def_vcj.get("id") or "").strip()
    def_payload_sha256 = sha256_bytes(signing_input(def_vcj))

    # Participants sourced from trust anchors authorized for corridor_agreement
    ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
    ta_path = module_dir / ta_rel
    if not ta_path.exists():
        print(f"Missing {ta_rel}", file=sys.stderr)
        return 2
    ta = load_yaml(ta_path)

    participants: List[Dict[str, Any]] = []
    for a in (ta.get("trust_anchors") or []):
        if not isinstance(a, dict):
            continue
        if "corridor_agreement" not in (a.get("allowed_attestations") or []):
            continue
        did = str(a.get("identifier") or "").split("#", 1)[0]
        if not did:
            continue
        role = str((a.get("metadata") or {}).get("role") or "participant").strip() or "participant"
        name = str(a.get("anchor_id") or "").strip()
        pobj: Dict[str, Any] = {"id": did, "role": role}
        if name:
            pobj["name"] = name
        participants.append(pobj)

    if not participants:
        print(f"{ta_rel}: no trust anchors authorize corridor_agreement; cannot scaffold participants", file=sys.stderr)
        return 2

    # Default activation thresholds: unanimity per role (safe default).
    by_role: Dict[str, List[Dict[str, Any]]] = {}
    for p in participants:
        by_role.setdefault(str(p.get("role") or "participant"), []).append(p)

    thresholds: List[Dict[str, Any]] = []
    for role, plist in sorted(by_role.items()):
        n = len(plist)
        thresholds.append(
            {
                "role": role,
                "required": n,
                "of": n,
                "description": f"all {role} participants must sign ({n}-of-{n})",
            }
        )

    accept_commitments = []
    ac_arg = getattr(args, "accept_commitments", "") or ""
    if ac_arg:
        accept_commitments = [x.strip() for x in ac_arg.split(",") if x.strip()]
    if not accept_commitments:
        accept_commitments = ["agree"]

    terms_ref = str(getattr(args, "terms_ref", "") or "").strip() or "urn:msez:terms:TODO"
    maintainer = str(getattr(args, "maintainer", "") or "").strip()
    version = str(getattr(args, "version", "") or "").strip()
    commitment = str(getattr(args, "commitment", "") or "").strip() or "agree"

    # For participant-specific agreement VCs, role must be provided or derivable.
    party_role = str(getattr(args, "role", "") or "").strip()
    if party and not party_role:
        for p0 in participants:
            if str(p0.get("id") or "") == party:
                party_role = str(p0.get("role") or "").strip()
                break
    if party and not party_role:
        print("--role is required when --party is set (and cannot be inferred from trust anchors)", file=sys.stderr)
        return 2

    # ID / output naming
    vc_id = str(getattr(args, "id", "") or "").strip()
    if not vc_id:
        if party:
            slug = re.sub(r"[^a-zA-Z0-9]+", "-", party).strip("-")[:32] or "party"
            vc_id = f"urn:msez:vc:corridor-agreement:{corridor_id}:{slug}"
        else:
            vc_id = f"urn:msez:vc:corridor-agreement:{corridor_id}"

    subj: Dict[str, Any] = {
        "corridor_id": corridor_id,
        "definition_vc_id": def_vc_id,
        "definition_payload_sha256": def_payload_sha256,
        "participants": participants,
        "activation": {
            "thresholds": thresholds,
            "effectiveFrom": now_rfc3339(),
            "accept_commitments": accept_commitments,
        },
        # v0.4.14+: state-channel receipt signing policy.
        # Default is unanimity per role (safe against forks unless a signer equivocates).
        "state_channel": {
            "receipt_signing": {
                "thresholds": thresholds,
                "effectiveFrom": now_rfc3339(),
                "description": "receipt signing thresholds for corridor state transitions",
            }
        },
        "terms": {"reference": terms_ref},
    }
    if version:
        subj["version"] = version
    if maintainer:
        subj["maintainer"] = maintainer

    if party:
        subj["party"] = {"id": party, "role": party_role}
        subj["commitment"] = commitment
        subj["party_terms"] = {}

        # Participant-specific lawpack pins (v0.4.1+)
        from tools.lawpack import parse_lawpack_ref  # type: ignore
        pinned: List[Dict[str, Any]] = []

        # 1) pull from a zone's stack.lock (recommended)
        zl = str(getattr(args, "from_zone_lock", "") or "").strip()
        if zl:
            zl_path = pathlib.Path(zl)
            if not zl_path.is_absolute():
                zl_path = REPO_ROOT / zl_path
            try:
                zl_obj = load_yaml(zl_path)
                for lp in (zl_obj.get("lawpacks") or []):
                    if not isinstance(lp, dict):
                        continue
                    digest = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if not digest or digest == "MISSING":
                        continue
                    pinned.append({
                        "jurisdiction_id": str(lp.get("jurisdiction_id") or ""),
                        "domain": str(lp.get("domain") or ""),
                        "lawpack_digest_sha256": digest,
                        "lawpack_lock_sha256": str(lp.get("lawpack_lock_sha256") or ""),
                        "as_of_date": str(lp.get("as_of_date") or ""),
                    })
            except Exception as ex:
                print(f"WARNING: could not load zone lock {zl_path}: {ex}", file=sys.stderr)

        # 2) explicit refs
        for ref in (getattr(args, "pinned_lawpack", None) or []):
            try:
                pinned.append(parse_lawpack_ref(ref))
            except Exception as ex:
                print(f"WARNING: ignoring invalid --pinned-lawpack '{ref}': {ex}", file=sys.stderr)

        # de-dup (jurisdiction_id, domain, digest)
        seen = set()
        dedup: List[Dict[str, Any]] = []
        for lp in pinned:
            key = (str(lp.get("jurisdiction_id") or ""), str(lp.get("domain") or ""), _coerce_sha256(lp.get("lawpack_digest_sha256")))
            if key in seen:
                continue
            seen.add(key)
            dedup.append(lp)
        subj["pinned_lawpacks"] = dedup

    vcj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorAgreementCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": subj,
    }

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if party:
            slug = re.sub(r"[^a-zA-Z0-9]+", "-", party).strip("-")[:32] or "party"
            out_name = f"corridor.agreement.{slug}.vc.json" if getattr(args, "sign", False) else f"corridor.agreement.{slug}.unsigned.json"
        else:
            out_name = "corridor.agreement.vc.json" if getattr(args, "sign", False) else "corridor.agreement.unsigned.json"
        out_path = module_dir / out_name

    out_path.parent.mkdir(parents=True, exist_ok=True)

    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)
        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            vm_did = issuer.split("#", 1)[0] if issuer.startswith("did:key:") else did
            vm = f"{vm_did}#key-1"
        add_ed25519_proof(vcj, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0

def cmd_corridor_status(args: argparse.Namespace) -> int:
    '''Show corridor VC and agreement activation status.'''
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    definition_errors = verify_corridor_definition_vc(module_dir)
    agreement_errors, agreement_summary = corridor_agreement_summary(module_dir)

    out = {
        'module_dir': str(module_dir),
        'corridor_id': agreement_summary.get('corridor_id'),
        'definition_ok': len(definition_errors) == 0,
        'definition_errors': definition_errors,
        'agreement': agreement_summary,
        'agreement_errors': agreement_errors,
        'activated': bool(agreement_summary.get('activated')),
    }

    if getattr(args, 'json', False):
        print(json.dumps(out, indent=2, ensure_ascii=False))
    else:
        cid = out.get('corridor_id') or module_dir.name
        print(f"Corridor: {cid}")
        print("  Definition VC:", "OK" if out['definition_ok'] else "FAIL")
        for e in definition_errors:
            print("    -", e)

        if agreement_summary.get('has_agreement'):
            print("  Agreement:", "ACTIVATED" if out['activated'] else "NOT ACTIVE")
            pattern = agreement_summary.get('agreement_pattern') or ''
            if pattern:
                print(f"    Pattern: {pattern}")
            aset = agreement_summary.get('agreement_set_sha256') or ''
            if aset:
                print(f"    Agreement set sha256: {aset}")
            signed = agreement_summary.get('signed_parties') or []
            signed_all = agreement_summary.get('signed_parties_all') or []
            participants = agreement_summary.get('participants') or []
            print(f"    Signed parties (affirmative): {len(signed)}/{len(participants)} (total signed: {len(signed_all)})")
            blocked = agreement_summary.get('blocked_parties') or []
            if blocked:
                print("    Blocked parties:")
                for b in blocked:
                    if not isinstance(b, dict):
                        continue
                    pid = b.get('id')
                    comm = b.get('commitment')
                    path = b.get('path')
                    tail = f" ({path})" if path else ""
                    print(f"      - {pid}: {comm}{tail}")
            for th in (agreement_summary.get('thresholds') or []):
                if not isinstance(th, dict):
                    continue
                role = th.get('role')
                signed_n = th.get('signed')
                req_n = th.get('required')
                sat = th.get('satisfied')
                print(f"    - threshold[{role}]: {signed_n}/{req_n} => {'OK' if sat else 'MISSING'}")
            for e in agreement_errors:
                print("    -", e)
        else:
            print("  Agreement: not configured")

    # Exit non-zero if definition fails OR if agreement configured but not activated.
    if not out['definition_ok']:
        return 2
    if agreement_summary.get('has_agreement') and not out['activated']:
        return 2
    return 0





def cmd_corridor_availability_attest(args: argparse.Namespace) -> int:
    """Create an Artifact Availability Attestation VC for a corridor.

    Primary intent: lawpack availability attestations for operational resilience.
    """

    from tools.vc import now_rfc3339, add_ed25519_proof, load_proof_keypair  # type: ignore

    module_dir = pathlib.Path(args.path).resolve()
    cfg = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str(cfg.get("corridor_id") or "")
    if not corridor_id:
        print("AVAIL FAIL: corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    # Compute the union of required (pinned) lawpacks.
    expected_lawpacks = corridor_expected_lawpack_digest_set(module_dir)
    if not expected_lawpacks:
        print("AVAIL FAIL: no pinned lawpacks found for corridor (agreement VCs missing or empty)", file=sys.stderr)
        return 2

    artifacts: List[Dict[str, Any]] = []
    for d in expected_lawpacks:
        artifacts.append({"artifact_type": "lawpack", "digest_sha256": d})

    now = now_rfc3339()
    vc: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": args.id or f"urn:msez:vc:artifact-availability:{corridor_id}:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "MSEZArtifactAvailabilityCredential"],
        "issuer": args.issuer,
        "issuanceDate": now,
        "credentialSubject": {
            "corridor_id": corridor_id,
            "as_of": args.as_of or now,
            "artifacts": artifacts,
            "service_endpoints": args.endpoint or [],
            "notes": args.notes or "",
        },
    }

    if args.sign:
        priv, vm = load_proof_keypair(pathlib.Path(args.key))
        add_ed25519_proof(vc, priv, vm, proof_purpose="assertionMethod")

    out_path = pathlib.Path(args.out) if args.out else (module_dir / "corridor.availability.vc.json")
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(vc, indent=2), encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_availability_verify(args: argparse.Namespace) -> int:
    """Verify that a set of Availability VCs cover the corridor's pinned lawpacks."""

    module_dir = pathlib.Path(args.path).resolve()
    cfg = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str(cfg.get("corridor_id") or "")
    if not corridor_id:
        print("AVAIL FAIL: corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    summary = corridor_agreement_summary(module_dir, cfg)
    parties = sorted(set((summary.get("participants") or [])))
    parties_base: set[str] = set()
    try:
        from tools.vc import base_did  # type: ignore
    except Exception:
        base_did = lambda s: str(s or "").split("#", 1)[0]  # type: ignore

    for p in parties:
        parties_base.add(base_did(p))

    expected_lawpacks = corridor_expected_lawpack_digest_set(module_dir, cfg)
    if not expected_lawpacks:
        print("AVAIL FAIL: no pinned lawpacks found for corridor (agreement VCs missing or empty)", file=sys.stderr)
        return 2

    src = pathlib.Path(args.vcs).resolve()
    vc_paths: List[pathlib.Path] = []
    if src.is_file():
        vc_paths = [src]
    elif src.is_dir():
        vc_paths = sorted([p for p in src.glob("*.json") if p.is_file()])
    else:
        print(f"AVAIL FAIL: not found: {src}", file=sys.stderr)
        return 2

    schema_path = REPO_ROOT / "schemas/vc.artifact-availability.schema.json"
    schema = schema_validator(schema_path)

    try:
        from tools.vc import verify_credential  # type: ignore
    except Exception as ex:
        print(f"AVAIL FAIL: cannot verify VC proofs: {ex}", file=sys.stderr)
        return 2

    by_issuer: Dict[str, set[str]] = {}

    for p in vc_paths:
        try:
            vc = load_json(p)
        except Exception as ex:
            print(f"AVAIL WARN: skip unreadable JSON {p}: {ex}", file=sys.stderr)
            continue

        verrs = validate_with_schema(vc, schema)
        if verrs:
            print(f"AVAIL WARN: schema invalid {p}", file=sys.stderr)
            continue

        results = verify_credential(vc)
        if not results or not any(r.ok for r in results):
            print(f"AVAIL WARN: bad signature {p}", file=sys.stderr)
            continue

        issuer = vc.get("issuer")
        issuer_id = issuer.get("id") if isinstance(issuer, dict) else issuer
        issuer_id = base_did(issuer_id)

        cs = vc.get("credentialSubject") or {}
        # If corridor_id is present, require it matches.
        cs_cid = cs.get("corridor_id")
        if cs_cid and str(cs_cid) != corridor_id:
            print(f"AVAIL WARN: corridor_id mismatch in {p}", file=sys.stderr)
            continue

        artifacts = cs.get("artifacts")
        if not isinstance(artifacts, list):
            continue

        digs: set[str] = set()
        for a in artifacts:
            if not isinstance(a, dict):
                continue
            if str(a.get("artifact_type") or "") != "lawpack":
                continue
            d = a.get("digest_sha256")
            if isinstance(d, str) and d:
                digs.add(d)

        if digs:
            by_issuer.setdefault(issuer_id, set()).update(digs)

    # Require each participant to attest availability for the full lawpack set.
    missing_parties: List[str] = []
    for party in sorted(parties_base):
        got = by_issuer.get(party, set())
        if got != expected_lawpacks:
            missing = sorted(expected_lawpacks - got)
            print(f"AVAIL FAIL: {party} missing {len(missing)} lawpacks", file=sys.stderr)
            for d in missing[:10]:
                print(f"  - {d}", file=sys.stderr)
            missing_parties.append(party)

    if missing_parties:
        return 2

    print("AVAIL OK")
    return 0

# --- Generic artifact CAS (v0.4.7+) ------------------------------------------

ARTIFACT_TYPES_KNOWN = [
    "lawpack",
    "ruleset",
    "transition-types",
    "circuit",
    "schema",
    "vc",
    "checkpoint",
    "proof-key",
    "blob",
]


def resolve_artifact_commitment(
    artifact_type: str,
    digest_sha256: str,
    *,
    repo_root: pathlib.Path = REPO_ROOT,
    store_roots: Optional[List[pathlib.Path]] = None,
    module_dir: Optional[pathlib.Path] = None,
) -> pathlib.Path:
    """Resolve an artifact commitment (type + sha256 digest) to a concrete path.

    Primary resolution uses the generic CAS convention:

      dist/artifacts/<type>/<digest>.*

    For backwards compatibility, this function provides best-effort legacy fallbacks:
    - transition-types: dist/artifacts/transition-types + (legacy fallbacks)
    - lawpack: dist/lawpacks/**/<digest>.lawpack.zip
    - ruleset: scan rulesets declared in registries/rulesets.yaml and compare digests

    The intent is that any digest commitment appearing in receipts/VCs has an obvious
    resolution path.
    """

    repo_root = repo_root.resolve()
    at = artifact_cas.normalize_artifact_type(artifact_type)
    dg = artifact_cas.normalize_digest(digest_sha256)

    roots = store_roots or artifact_cas.artifact_store_roots(repo_root)

    # 1) Generic CAS lookup.
    try:
        return artifact_cas.resolve_artifact_by_digest(at, dg, repo_root=repo_root, store_roots=roots)
    except FileNotFoundError:
        pass

    # 2) Legacy fallbacks (type-specific).
    if at == "transition-types":
        return resolve_transition_type_registry_lock_by_digest(dg, module_dir=module_dir, repo_root=repo_root)

    if at == "lawpack":
        legacy_root = repo_root / "dist" / "lawpacks"
        if legacy_root.exists():
            for cand in sorted(legacy_root.rglob(f"{dg}.lawpack.zip")):
                if cand.is_file():
                    return cand
        raise FileNotFoundError(f"lawpack artifact not found for digest {dg}")

    if at == "ruleset":
        # Ruleset digests are SHA256(JCS(ruleset_descriptor_json)).
        reg = _load_rulesets_registry()
        for rid, rel in reg.items():
            try:
                dig = ruleset_descriptor_digest_sha256(rid)
            except Exception:
                continue
            if dig != dg:
                continue
            p = pathlib.Path(rel)
            if not p.is_absolute():
                p = repo_root / p
            if p.exists():
                return p
        raise FileNotFoundError(f"ruleset descriptor not found for digest {dg}")

    raise FileNotFoundError(f"artifact not found for type '{at}' digest {dg}")


def cmd_artifact_store(args: argparse.Namespace) -> int:
    at = str(getattr(args, "type", "") or "").strip()
    digest = str(getattr(args, "digest", "") or "").strip()
    src = pathlib.Path(str(getattr(args, "path", "") or "").strip())
    if not (at and digest and str(src)):
        print("type, digest and path are required", file=sys.stderr)
        return 2

    store_root = str(getattr(args, "store_root", "") or "").strip()
    name = str(getattr(args, "name", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    dest = artifact_cas.store_artifact_file(
        at,
        digest,
        src,
        repo_root=REPO_ROOT,
        store_root=pathlib.Path(store_root) if store_root else None,
        dest_name=name or None,
        overwrite=overwrite,
    )

    if getattr(args, "json", False):
        print(json.dumps({"stored": dest.as_posix(), "type": artifact_cas.normalize_artifact_type(at), "digest": artifact_cas.normalize_digest(digest)}, indent=2))
    else:
        print(dest.as_posix())
    return 0


def cmd_artifact_resolve(args: argparse.Namespace) -> int:
    at = str(getattr(args, "type", "") or "").strip()
    digest = str(getattr(args, "digest", "") or "").strip()
    if not (at and digest):
        print("type and digest are required", file=sys.stderr)
        return 2

    extra_roots = getattr(args, "store_root", None) or []
    roots = []
    for r in extra_roots:
        rr = str(r or "").strip()
        if not rr:
            continue
        roots.append(pathlib.Path(rr))

    p = resolve_artifact_commitment(
        at,
        digest,
        repo_root=REPO_ROOT,
        store_roots=roots if roots else None,
    )

    if getattr(args, "show", False):
        # Only show text-like artifacts.
        if p.suffix.lower() in {".json", ".yaml", ".yml", ".md", ".txt"}:
            print(p.read_text(encoding="utf-8"))
        else:
            print(p.as_posix())
    else:
        if getattr(args, "json", False):
            print(json.dumps({"path": p.as_posix(), "type": artifact_cas.normalize_artifact_type(at), "digest": artifact_cas.normalize_digest(digest)}, indent=2))
        else:
            print(p.as_posix())
    return 0


def cmd_artifact_index_rulesets(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/ruleset with content-addressed copies of declared rulesets."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    reg = _load_rulesets_registry()
    if not reg:
        print("No rulesets found in registries/rulesets.yaml", file=sys.stderr)
        return 2

    stored = []
    for rid, rel in sorted(reg.items()):
        try:
            digest = ruleset_descriptor_digest_sha256(rid)
        except Exception as ex:
            print(f"WARN: could not digest ruleset {rid}: {ex}", file=sys.stderr)
            continue
        src = pathlib.Path(rel)
        if not src.is_absolute():
            src = REPO_ROOT / src
        if not src.exists():
            print(f"WARN: missing ruleset file for {rid}: {src}", file=sys.stderr)
            continue
        dest = artifact_cas.store_artifact_file(
            "ruleset",
            digest,
            src,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({"ruleset_id": rid, "digest": digest, "path": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} ruleset artifacts")
    return 0


def cmd_artifact_index_lawpacks(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/lawpack by copying any local dist/lawpacks/**/*.lawpack.zip files."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    legacy_root = REPO_ROOT / "dist" / "lawpacks"
    if not legacy_root.exists():
        print("dist/lawpacks not found", file=sys.stderr)
        return 2

    stored = []
    for lp in sorted(legacy_root.rglob("*.lawpack.zip")):
        name = lp.name
        digest = name.split(".", 1)[0].strip().lower()
        if not SHA256_HEX_RE.match(digest):
            continue
        dest = artifact_cas.store_artifact_file(
            "lawpack",
            digest,
            lp,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({"digest": digest, "src": lp.as_posix(), "dest": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} lawpack artifacts")
    return 0


def cmd_artifact_index_schemas(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/schema with content-addressed copies of JSON Schemas."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    schema_dir = REPO_ROOT / "schemas"
    if not schema_dir.exists():
        print("schemas/ not found", file=sys.stderr)
        return 2

    stored = []
    for sp in sorted(schema_dir.rglob("*.schema.json")):
        if not sp.is_file():
            continue
        try:
            digest = _jcs_sha256_of_json_file(sp)
        except Exception as ex:
            print(f"WARN: could not digest schema {sp}: {ex}", file=sys.stderr)
            continue

        dest = artifact_cas.store_artifact_file(
            "schema",
            digest,
            sp,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({
            "schema": sp.relative_to(REPO_ROOT).as_posix(),
            "digest": digest,
            "path": dest.as_posix(),
        })

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} schema artifacts")
    return 0


def cmd_artifact_index_vcs(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/vc with content-addressed copies of VCs (payload digest)."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    from tools.vc import signing_input  # type: ignore

    search_roots = [
        REPO_ROOT / "modules",
        REPO_ROOT / "docs" / "examples" / "vc",
        REPO_ROOT / "tests" / "fixtures",
    ]

    stored = []
    seen_paths = set()

    for root in search_roots:
        if not root.exists():
            continue
        for vp in sorted(root.rglob("*.vc.json")):
            if not vp.is_file():
                continue
            try:
                rel = vp.relative_to(REPO_ROOT).as_posix()
            except Exception:
                rel = vp.as_posix()
            if rel in seen_paths:
                continue
            seen_paths.add(rel)

            try:
                vcj = load_json(vp)
                digest = sha256_bytes(signing_input(vcj))
            except Exception as ex:
                print(f"WARN: could not digest VC {vp}: {ex}", file=sys.stderr)
                continue

            dest = artifact_cas.store_artifact_file(
                "vc",
                digest,
                vp,
                repo_root=REPO_ROOT,
                store_root=pathlib.Path(store_root) if store_root else None,
                overwrite=overwrite,
            )
            stored.append({"vc": rel, "digest": digest, "path": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} VC artifacts")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    v = sub.add_parser("validate")
    v.add_argument("profile", nargs="?", default="profiles/digital-financial-center/profile.yaml")
    v.add_argument("--zone", default="", help="Validate a zone.yaml (overrides profile path)")
    v.add_argument("--all-modules", action="store_true")
    v.add_argument("--all-profiles", action="store_true")
    v.add_argument("--all-zones", action="store_true")
    v.set_defaults(func=cmd_validate)

    b = sub.add_parser("build")
    b.add_argument("profile", nargs="?", default="")
    b.add_argument("--zone", default="", help="Build from a zone.yaml (recommended)")
    b.add_argument("--out", default="dist")
    b.add_argument("--strict-render", action="store_true", help="Fail build when template variables are missing")
    b.add_argument("--no-render", action="store_true", help="Skip template rendering step")
    b.set_defaults(func=cmd_build)

    f = sub.add_parser("fetch-akoma-schemas")
    f.set_defaults(func=cmd_fetch_akoma)

    r = sub.add_parser("render")
    r.add_argument("xml")
    r.add_argument("--out-dir", default="dist/render")
    r.add_argument("--pdf", action="store_true")
    r.set_defaults(func=cmd_render)

    l = sub.add_parser("lock")
    l.add_argument("zone")
    l.add_argument("--out", default="")
    l.add_argument(
        "--emit-artifactrefs",
        action="store_true",
        help=(
            "Emit typed ArtifactRef objects for digest-bearing fields (lawpacks + corridor artifacts) "
            "instead of legacy raw sha256 strings."
        ),
    )
    l.set_defaults(func=cmd_lock)

    vc = sub.add_parser("vc")
    vc_sub = vc.add_subparsers(dest="vc_cmd", required=True)

    vcs = vc_sub.add_parser("sign")
    vcs.add_argument("credential", help="Path to an unsigned VC JSON")
    vcs.add_argument("--key", required=True, help="Private key as OKP/Ed25519 JWK (with d,x)")
    vcs.add_argument("--verification-method", default="", help="Optional verificationMethod override (default: did:key derived from JWK + '#key-1')")
    vcs.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    vcs.add_argument("--out", default="", help="Output path (default: <input>.signed.json)")
    vcs.set_defaults(func=cmd_vc_sign)

    vcv = vc_sub.add_parser("verify")
    vcv.add_argument("credential", help="Path to a signed VC JSON")
    vcv.set_defaults(func=cmd_vc_verify)

    vch = vc_sub.add_parser("payload-hash")
    vch.add_argument("credential", help="Path to a VC JSON (signed or unsigned)")
    vch.set_defaults(func=cmd_vc_payload_hash)

    vck = vc_sub.add_parser("keygen")
    vck.add_argument("--out", required=True, help="Output path for private OKP/Ed25519 JWK")
    vck.add_argument("--public-out", default="", help="Optional output path for public-only JWK (no 'd')")
    vck.add_argument("--kid", default="key-1", help="JWK key id (kid)")
    vck.set_defaults(func=cmd_vc_keygen)

    cor = sub.add_parser("corridor")
    cor_sub = cor.add_subparsers(dest="corridor_cmd", required=True)

    corv = cor_sub.add_parser("verify")
    corv.add_argument("path", help="Corridor module directory or corridor.yaml path")
    corv.set_defaults(func=cmd_corridor_verify)

    cdef = cor_sub.add_parser("vc-init-definition")
    cdef.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cdef.add_argument("--issuer", required=True, help="Issuer DID for the Corridor Definition VC")
    cdef.add_argument("--id", default="", help="Credential id (default: urn:msez:vc:corridor-definition:<corridor_id>)")
    cdef.add_argument("--ruleset", default="", help="Verification ruleset override (default: corridor.yaml verification_ruleset)")
    cdef.add_argument("--require-domain", dest="require_domain", action="append", default=[], help="Require a lawpack domain for corridor participants (repeatable; default: civil+financial)")
    cdef.add_argument("--allow-lawpack", dest="allow_lawpack", action="append", default=[], help="Allow a specific lawpack digest (<jurisdiction_id>:<domain>:<sha256>) (repeatable)")
    cdef.add_argument("--version", default="", help="credentialSubject.version")
    cdef.add_argument("--maintainer", default="", help="credentialSubject.maintainer")
    cdef.add_argument("--out", default="", help="Output path (default: corridor.vc.unsigned.json or corridor.vc.json when --sign)")
    cdef.add_argument("--sign", action="store_true", help="Sign the generated VC")
    cdef.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cdef.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cdef.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cdef.set_defaults(func=cmd_corridor_vc_init_definition)

    cagr = cor_sub.add_parser("vc-init-agreement")
    cagr.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cagr.add_argument("--issuer", default="", help="Issuer DID (defaults to --party when provided)")
    cagr.add_argument("--party", default="", help="Party DID for participant-specific agreement VC (optional)")
    cagr.add_argument("--role", default="", help="Party role for --party (required if not inferable from trust-anchors)")
    cagr.add_argument("--commitment", default="agree", help="Party commitment verb (default: agree)")
    cagr.add_argument("--accept-commitments", dest="accept_commitments", default="", help="Comma-separated affirmative commitment verbs (default: agree)")
    cagr.add_argument("--terms-ref", default="", help="Terms reference (default: urn:msez:terms:TODO)")
    cagr.add_argument("--from-zone-lock", dest="from_zone_lock", default="", help="Populate pinned_lawpacks from a zone stack.lock")
    cagr.add_argument("--pinned-lawpack", dest="pinned_lawpack", action="append", default=[], help="Pinned lawpack ref (<jurisdiction_id>:<domain>:<sha256>) (repeatable)")
    cagr.add_argument("--id", default="", help="Credential id override")
    cagr.add_argument("--version", default="", help="credentialSubject.version")
    cagr.add_argument("--maintainer", default="", help="credentialSubject.maintainer")
    cagr.add_argument("--out", default="", help="Output path")
    cagr.add_argument("--sign", action="store_true", help="Sign the generated VC")
    cagr.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cagr.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cagr.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cagr.set_defaults(func=cmd_corridor_vc_init_agreement)

    cors = cor_sub.add_parser("status")
    cors.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cors.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    cors.set_defaults(func=cmd_corridor_status)

    cav = cor_sub.add_parser("availability-attest", help="Create a lawpack artifact availability attestation VC for this corridor")
    cav.add_argument("path", help="Corridor module directory")
    cav.add_argument("--issuer", required=True, help="Issuer DID (the party attesting to availability)")
    cav.add_argument("--as-of", dest="as_of", default="", help="Timestamp for the attestation (RFC3339; default: now)")
    cav.add_argument(
        "--endpoint",
        dest="endpoints",
        action="append",
        default=[],
        help="Service endpoint URI (repeatable)",
    )
    cav.add_argument("--id", default="", help="Optional VC id (default: generated urn:uuid)")
    cav.add_argument("--out", default="", help="Output path (default: <module>/corridor.availability.vc.unsigned.json)")
    cav.add_argument("--sign", action="store_true", help="Sign the attestation VC")
    cav.add_argument("--key", default="", help="Proof key JSON path (required if --sign)")
    cav.set_defaults(func=cmd_corridor_availability_attest)

    cavv = cor_sub.add_parser("availability-verify", help="Verify availability attestations cover all corridor lawpacks")
    cavv.add_argument("path", help="Corridor module directory")
    cavv.add_argument("--vcs", required=True, help="Directory or file containing availability VC JSONs")
    cavv.add_argument(
        "--require-all-parties",
        dest="require_all_parties",
        action="store_true",
        help="Require an availability VC from every corridor participant",
    )
    cavv.add_argument(
        "--no-require-all-parties",
        dest="require_all_parties",
        action="store_false",
        help="Do not require every participant to attest",
    )
    cavv.set_defaults(require_all_parties=True)
    cavv.add_argument(
        "--require-all-lawpacks",
        dest="require_all_lawpacks",
        action="store_true",
        help="Require each party's attestation to cover the full corridor lawpack digest set",
    )
    cavv.add_argument(
        "--no-require-all-lawpacks",
        dest="require_all_lawpacks",
        action="store_false",
        help="Allow partial coverage (informational)",
    )
    cavv.set_defaults(require_all_lawpacks=True)
    cavv.set_defaults(func=cmd_corridor_availability_verify)



    # Corridor state channels (verifiable receipts)
    cstate = cor_sub.add_parser("state")
    cstate_sub = cstate.add_subparsers(dest="corridor_state_cmd", required=True)

    cg = cstate_sub.add_parser("genesis-root", help="Compute corridor genesis_root")
    cg.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cg.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    cg.set_defaults(func=cmd_corridor_state_genesis_root)

    cri = cstate_sub.add_parser("receipt-init", help="Create a corridor state receipt and optionally sign it")
    cri.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cri.add_argument("--sequence", type=int, default=0, help="Receipt sequence number (default: 0)")
    cri.add_argument("--prev-root", default="genesis", help="prev_root (default: genesis). Use 64-hex or 'genesis'")
    cri.add_argument("--timestamp", default="", help="RFC3339 timestamp override")
    cri.add_argument("--transition", default="", help="Path to a JSON transition object (default: noop)")
    cri.add_argument(
        "--fill-transition-digests",
        action="store_true",
        help="Populate transition envelope digest references from the configured transition type registry (default: omit and rely on the registry snapshot digest commitment)",
    )
    cri.add_argument("--lawpack-digest", dest="lawpack_digest", action="append", default=[], help="Lawpack digest sha256 to include (repeatable; default: derive from agreement-set)")
    cri.add_argument("--ruleset-digest", dest="ruleset_digest", action="append", default=[], help="Ruleset digest sha256 to include (repeatable; default: derive from corridor rulesets)")
    cri.add_argument("--out", default="", help="Output path (default: corridor-receipt.<seq>.json)")
    cri.add_argument("--sign", action="store_true", help="Sign the receipt")
    cri.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cri.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cri.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cri.set_defaults(func=cmd_corridor_state_receipt_init)

    csv = cstate_sub.add_parser("verify", help="Verify a corridor receipt chain and print the final root")
    csv.add_argument("path", help="Corridor module directory or corridor.yaml path")
    csv.add_argument("--receipts", required=True, help="Receipt file or directory containing receipt JSON files")
    csv.add_argument(
        "--from-checkpoint",
        dest="from_checkpoint",
        default="",
        help="Optional signed checkpoint to bootstrap verification (verify only receipts at or after checkpoint.receipt_count)",
    )
    csv.add_argument("--fork-resolutions", default="", help="Optional fork-resolution artifacts (file or dir) to select canonical receipts at fork points")
    csv.add_argument(
        "--checkpoint",
        dest="checkpoint",
        default="",
        help="Optional signed head checkpoint to verify against the computed final_root / receipt_count",
    )
    csv.add_argument(
        "--enforce-checkpoint-policy",
        action="store_true",
        help="Enforce checkpoint finality policy derived from Corridor Agreement VC(s) (state_channel.checkpointing.*) when present; when mode=required, --checkpoint must be provided",
    )
    csv.add_argument("--enforce-trust-anchors", action="store_true", help="Require at least one valid proof from a trust anchor authorized for corridor_receipt")
    csv.add_argument(
        "--enforce-receipt-threshold",
        action="store_true",
        help=(
            "Enforce receipt signing thresholds derived from Corridor Agreement VC(s) (recommended for fork resistance). "
            "Defaults to the agreement's state_channel.receipt_signing thresholds when present; otherwise falls back to activation thresholds."
        ),
    )
    csv.add_argument(
        "--enforce-transition-types",
        action="store_true",
        help="Require that receipt transitions resolve against the corridor's transition type registry; when a receipt does not bind to a registry snapshot digest, require explicit per-transition digest references",
    )
    csv.add_argument(
        "--require-artifacts",
        action="store_true",
        help="Fail verification if any digest commitment in receipts cannot be resolved via the artifact CAS (dist/artifacts/<type>/<digest>.* or configured store roots)",
    )
    csv.add_argument(
        "--transitive-require-artifacts",
        action="store_true",
        help=(
            "Stronger artifact completeness: treat transition_type_registry_digest_sha256 as a commitment root and "
            "require that all schema/ruleset/circuit digests referenced by the registry lock are present in CAS (and any nested ArtifactRefs embedded in referenced ruleset artifacts). "
            "Implies --require-artifacts."
        ),
    )
    csv.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    csv.set_defaults(func=cmd_corridor_state_verify)

    cfi = cstate_sub.add_parser(
        "fork-inspect",
        help="Inspect receipts for forks/duplicates and resolution coverage (forensics)",
    )
    cfi.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cfi.add_argument(
        "--receipts",
        required=True,
        help="Directory containing receipt JSON files",
    )
    cfi.add_argument(
        "--fork-resolutions",
        default="",
        help="Optional fork-resolution artifact (VC) file/dir; if provided, report indicates which forks are resolved",
    )
    cfi.add_argument(
        "--from-checkpoint",
        default="",
        help="Optional checkpoint to bootstrap analysis (starts from receipt_count/final_state_root)",
    )
    cfi.add_argument(
        "--enforce-trust-anchors",
        action="store_true",
        help="When verifying proofs, require receipt signers to be in trust-anchors.yaml for corridor_receipt",
    )
    cfi.add_argument(
        "--enforce-transition-types",
        action="store_true",
        help="Require transition_type_registry_digest_sha256 (if present) matches corridor pinned snapshot",
    )
    cfi.add_argument(
        "--require-artifacts",
        action="store_true",
        help="Fail report generation if any committed digest cannot be resolved via dist/artifacts CAS",
    )
    cfi.add_argument(
        "--transitive-require-artifacts",
        action="store_true",
        help=(
            "Stronger artifact completeness: treat transition_type_registry_digest_sha256 as a commitment root and "
            "require that all schema/ruleset/circuit digests referenced by the registry lock are present in CAS (and any nested ArtifactRefs embedded in referenced ruleset artifacts). "
            "Implies --require-artifacts."
        ),
    )
    cfi.add_argument(
        "--no-verify-proofs",
        action="store_true",
        help="Skip cryptographic proof verification (signer sets will be empty)",
    )
    cfi.add_argument(
        "--format",
        choices=["text", "json"],
        default="text",
        help="Output format",
    )
    cfi.add_argument(
        "--out",
        default="",
        help="Optional output path; default prints to stdout",
    )
    cfi.set_defaults(func=cmd_corridor_state_fork_inspect)


    # corridor state propose
    csp = cstate_sub.add_parser("propose", help="Generate an unsigned receipt proposal (MSEZCorridorReceiptProposal)")
    csp.add_argument("--path", required=True, help="Corridor module directory")
    csp.add_argument("--sequence", type=int, required=True, help="Receipt sequence to propose")
    csp.add_argument("--prev-root", default="", help="Previous state root (default: corridor genesis_root)")
    csp.add_argument("--timestamp", default="", help="Receipt timestamp (RFC3339/ISO8601; default: now)")
    csp.add_argument("--transition", required=True, help="Transition envelope JSON file")
    csp.add_argument("--fill-transition-digests", action="store_true", help="Fill missing transition envelope digests from corridor module expectations")
    csp.add_argument("--lawpack-digest", action="append", default=[], help="Override expected lawpack digests (repeatable)")
    csp.add_argument("--ruleset-digest", action="append", default=[], help="Override expected ruleset digests (repeatable)")
    csp.add_argument("--proposed-by", default="", help="Optional DID or identifier of the proposer")
    csp.add_argument("--proposal-id", default="", help="Optional proposal id (default: urn:uuid:...)")
    csp.add_argument("--notes", default="", help="Optional human-readable notes")
    csp.add_argument("--out", default="", help="Write proposal JSON to a file (default: stdout)")
    csp.set_defaults(func=cmd_corridor_state_propose)

    # corridor state fork-resolve
    cfr = cstate_sub.add_parser("fork-resolve", help="Generate an unsigned fork-resolution VC selecting the canonical receipt at a fork point")
    cfr.add_argument("--path", required=True, help="Corridor module directory")
    cfr.add_argument("--sequence", type=int, required=True, help="Fork sequence to resolve")
    cfr.add_argument("--prev-root", required=True, help="Fork prev_root value")
    cfr.add_argument("--chosen-next-root", required=True, help="Chosen next_root value for the canonical receipt")
    cfr.add_argument("--candidate-next-root", action="append", default=[], help="Optional candidate next_root values (repeatable)")
    cfr.add_argument("--issuer", required=True, help="DID issuer for the fork-resolution VC (to be signed later)")
    cfr.add_argument("--resolved-at", default="", help="Resolution timestamp (default: now)")
    cfr.add_argument("--id", default="", help="VC id (default: urn:uuid:...)")
    cfr.add_argument("--out", default="", help="Write VC JSON to a file (default: stdout)")
    cfr.set_defaults(func=cmd_corridor_state_fork_resolve)

    # corridor state anchor
    can = cstate_sub.add_parser("anchor", help="Generate an unsigned corridor-anchor VC")
    can.add_argument("--path", required=True, help="Corridor module directory")
    can.add_argument("--issuer", required=True, help="DID issuer for the anchor VC (to be signed later)")
    can.add_argument("--head-commitment", default="", help="Head commitment digest (if omitted, computed from --receipts)")
    can.add_argument("--receipts", default="", help="Receipt file or directory to compute the head commitment")
    can.add_argument("--fork-resolutions", default="", help="Optional fork-resolution artifacts (file or dir) used when computing from receipts")
    can.add_argument("--from-checkpoint", dest="from_checkpoint", default="", help="Optional signed checkpoint JSON to bootstrap head computation")
    can.add_argument("--network", required=True, help="Anchor chain network identifier")
    can.add_argument("--chain-id", default="", help="Anchor chain id")
    can.add_argument("--tx-hash", default="", help="Anchor transaction hash")
    can.add_argument("--block-number", type=int, default=None, help="Anchor block number")
    can.add_argument("--block-hash", default="", help="Anchor block hash")
    can.add_argument("--block-timestamp", default="", help="Anchor block timestamp (RFC3339)")
    can.add_argument("--anchored-at", default="", help="Anchor event timestamp (default: now)")
    can.add_argument("--checkpoint-digest", default="", help="Optional checkpoint digest_sha256 to reference as an ArtifactRef")
    can.add_argument("--checkpoint-uri", default="", help="Optional URI for checkpoint artifact reference")
    can.add_argument("--id", default="", help="VC id (default: urn:uuid:...)")
    can.add_argument("--out", default="", help="Write VC JSON to a file (default: stdout)")
    can.set_defaults(func=cmd_corridor_state_anchor)

    # corridor state finality-status
    cfs = cstate_sub.add_parser("finality-status", help="Compute a corridor finality status object for the current head")
    cfs.add_argument("--path", required=True, help="Corridor module directory")
    cfs.add_argument("--receipts", required=True, help="Receipt file or directory containing receipt JSON files")
    cfs.add_argument("--fork-resolutions", default="", help="Optional fork-resolution artifacts (file or dir) used when computing the head")
    cfs.add_argument("--from-checkpoint", dest="from_checkpoint", default="", help="Optional signed checkpoint JSON to bootstrap head computation")
    cfs.add_argument("--checkpoint", default="", help="Optional signed checkpoint JSON (if it matches the computed head, finality can be upgraded)")
    cfs.add_argument("--watcher-report", default="", help="Optional watcher compare/quorum report JSON (may upgrade finality)")
    cfs.add_argument("--anchors", default="", help="Optional anchor VC file or directory (may upgrade finality)")
    cfs.add_argument("--arbitration-awards", dest="arbitration_awards", default="", help="Optional arbitration award VC file or directory (may upgrade finality)")
    cfs.add_argument("--out", default="", help="Write finality status JSON to a file (default: stdout)")
    cfs.set_defaults(func=cmd_corridor_state_finality_status)
    ccp = cstate_sub.add_parser("checkpoint", help="Create a corridor state checkpoint (MMR root) and optionally sign it")
    ccp.add_argument("path", help="Corridor module directory or corridor.yaml path")
    ccp.add_argument("--receipts", required=True, help="Receipt file or directory containing receipt JSON files")
    ccp.add_argument("--fork-resolutions", default="", help="Optional fork-resolution artifacts (file or dir) to select canonical receipts at fork points")
    ccp.add_argument("--enforce-trust-anchors", action="store_true", help="Require at least one valid proof from a trust anchor authorized for corridor_receipt")
    ccp.add_argument("--out", default="", help="Output path (default: corridor.checkpoint.json)")
    ccp.add_argument("--sign", action="store_true", help="Sign the checkpoint")
    ccp.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    ccp.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    ccp.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    ccp.set_defaults(func=cmd_corridor_state_checkpoint)

    cpr = cstate_sub.add_parser("proof", help="Generate an MMR inclusion proof for a receipt sequence")
    cpr.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cpr.add_argument("--receipts", required=True, help="Receipt directory containing receipt JSON files")
    cpr.add_argument("--fork-resolutions", default="", help="Optional fork-resolution artifacts (file or dir) to select canonical receipts at fork points")
    cpr.add_argument("--sequence", required=True, type=int, help="Receipt sequence number (leaf index) to prove")
    cpr.add_argument("--checkpoint", default="", help="Optional checkpoint path; when provided, the proof is checked against it")
    cpr.add_argument("--enforce-trust-anchors", action="store_true", help="Verify receipt signatures and require trust anchors")
    cpr.add_argument("--out", default="", help="Output path (default: corridor.inclusion-proof.<seq>.json)")
    cpr.set_defaults(func=cmd_corridor_state_inclusion_proof)

    civ = cstate_sub.add_parser("verify-inclusion", help="Verify an inclusion proof against a signed checkpoint")
    civ.add_argument("path", help="Corridor module directory or corridor.yaml path")
    civ.add_argument("--receipt", required=True, help="Receipt JSON file")
    civ.add_argument("--proof", required=True, help="Inclusion proof JSON file")
    civ.add_argument("--checkpoint", required=True, help="Signed checkpoint JSON file")
    civ.add_argument("--enforce-trust-anchors", action="store_true", help="Require receipt/checkpoint signatures from trust anchors")
    civ.set_defaults(func=cmd_corridor_state_verify_inclusion)

    cwa = cstate_sub.add_parser("watcher-attest", help="Create a watcher attestation VC referencing a signed checkpoint")
    cwa.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cwa.add_argument("--checkpoint", required=True, help="Signed checkpoint JSON file to attest to")
    cwa.add_argument("--issuer", required=True, help="Issuer DID (watcher DID)")
    cwa.add_argument("--observed-at", dest="observed_at", default="", help="Observed timestamp (RFC3339; default: now)")
    cwa.add_argument("--no-fork-observed", dest="no_fork_observed", action="store_true", help="Include no_fork_observed=true in credentialSubject")
    cwa.add_argument(
        "--finality-level",
        dest="finality_level",
        default="",
        help="Optional finality-level claim to include (e.g., checkpoint_signed)",
    )
    cwa.add_argument("--store-artifacts", dest="store_artifacts", action="store_true", help="Store checkpoint in artifact CAS (type checkpoint)")
    cwa.add_argument("--id", default="", help="Optional VC id (default: generated urn:uuid)")
    cwa.add_argument("--out", default="", help="Output path (default: corridor.watcher-attestation.vc.unsigned.json)")
    cwa.add_argument("--sign", action="store_true", help="Sign the watcher attestation VC")
    cwa.add_argument("--key", default="", help="Proof key JSON path (required if --sign)")
    cwa.set_defaults(func=cmd_corridor_state_watcher_attest)


    cwc = cstate_sub.add_parser(
        "watcher-compare",
        help="Compare watcher attestation VCs and flag divergent heads (instant fork alarms without receipts)",
    )
    cwc.add_argument("path", help="Path to corridor module directory")
    cwc.add_argument("--vcs", required=True, help="Watcher attestation VC file or directory")
    cwc.add_argument(
        "--enforce-authority-registry",
        action="store_true",
        help="Require watcher signers to be authorized by the corridor authority-registry chain (if configured)",
    )
    cwc.add_argument(
        "--require-artifacts",
        action="store_true",
        help="Fail if committed checkpoint digests cannot be resolved via the artifact CAS",
    )
    cwc.add_argument(
        "--fail-on-lag",
        action="store_true",
        help="Return non-zero when watchers disagree on receipt_count (lag/out-of-sync)",
    )
    cwc.add_argument(
        "--quorum-threshold",
        dest="quorum_threshold",
        default="",
        help="Optional quorum threshold for liveness monitoring (e.g., 'majority' or '3/5')",
    )
    cwc.add_argument(
        "--require-quorum",
        dest="require_quorum",
        action="store_true",
        help="Return non-zero when the quorum threshold is not reached (in addition to fork/lag policies)",
    )
    cwc.add_argument(
        "--max-staleness",
        dest="max_staleness",
        default="",
        help="Ignore watcher attestations older than this age (e.g., '1h', '24h', 'PT1H'); default: 1h",
    )
    cwc.add_argument(
        "--format",
        dest="format",
        default="text",
        choices=["text", "json"],
        help="Output format (default: text)",
    )
    cwc.add_argument("--out", default="", help="Write report to this path (default: stdout)")
    cwc.add_argument("--json", action="store_true", help="Alias for --format json")
    cwc.set_defaults(func=cmd_corridor_state_watcher_compare)


    cfa = cstate_sub.add_parser("fork-alarm", help="Create a watcher fork-alarm VC from two conflicting receipts")
    cfa.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cfa.add_argument("--receipt-a", required=True, help="First conflicting receipt JSON file")
    cfa.add_argument("--receipt-b", required=True, help="Second conflicting receipt JSON file")
    cfa.add_argument("--issuer", required=True, help="Issuer DID (watcher DID)")
    cfa.add_argument("--detected-at", dest="detected_at", default="", help="Detected timestamp (RFC3339; default: now)")
    cfa.add_argument("--store-artifacts", dest="store_artifacts", action="store_true", help="Store receipts in artifact CAS (type blob)")
    cfa.add_argument("--id", default="", help="Optional VC id (default: generated urn:uuid)")
    cfa.add_argument("--out", default="", help="Output path (default: corridor.fork-alarm.vc.unsigned.json)")
    cfa.add_argument("--sign", action="store_true", help="Sign the fork-alarm VC")
    cfa.add_argument("--key", default="", help="Proof key JSON path (required if --sign)")
    cfa.set_defaults(func=cmd_corridor_state_fork_alarm)


    c = sub.add_parser("check-coverage")
    c.add_argument("--profile", default="", help="Restrict check to modules in a profile.yaml")
    c.add_argument("--zone", default="", help="Restrict check to modules in the zone's profile")
    c.set_defaults(func=cmd_check_coverage)


    law = sub.add_parser("law")
    law_sub = law.add_subparsers(dest="law_cmd", required=True)

    ll = law_sub.add_parser("list")
    ll.add_argument("--jurisdiction", default="", help="Filter by jurisdiction_id (e.g., us-ca)")
    ll.add_argument("--domain", default="", help="Filter by domain (civil|financial)")
    ll.add_argument("--json", action="store_true")
    ll.set_defaults(func=cmd_law_list)

    lc = law_sub.add_parser("coverage")
    lc.add_argument("--json", action="store_true")
    lc.set_defaults(func=cmd_law_coverage)


    li = law_sub.add_parser("ingest")
    li.add_argument("module", help="Path to a jurisdiction corpus module directory (e.g., modules/legal/jurisdictions/us/ca/civil)")
    li.add_argument("--as-of-date", dest="as_of_date", required=True, help="Snapshot date (YYYY-MM-DD)")
    li.add_argument("--out-dir", dest="out_dir", default="dist/lawpacks", help="Output directory for lawpack artifacts")
    li.add_argument("--fetch", action="store_true", help="Fetch declared sources into src/raw (best-effort)")
    li.add_argument("--include-raw", dest="include_raw", action="store_true", help="Include src/raw files inside the lawpack.zip (license permitting)")
    li.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    li.set_defaults(func=cmd_law_ingest)

    latt = law_sub.add_parser("attest-init", help="Initialize a Lawpack Validity Attestation VC skeleton")
    latt.add_argument("--jurisdiction-id", required=True)
    latt.add_argument("--domain", required=True)
    latt.add_argument("--lawpack-digest", required=True)
    latt.add_argument("--as-of-date", required=True, help="Snapshot date (YYYY-MM-DD)")
    latt.add_argument("--issuer", default="", help="Issuer DID (e.g., did:key:z6Mk...)")
    latt.add_argument("--out", default="", help="Output path for the VC JSON")
    latt.set_defaults(func=cmd_law_attest_init)



    art = sub.add_parser("artifact")
    art_sub = art.add_subparsers(dest="artifact_cmd", required=True)

    astore = art_sub.add_parser("store", help="Store an artifact into the content-addressed CAS")
    astore.add_argument("type", help="Artifact type (e.g., lawpack|ruleset|transition-types|circuit|schema|vc|checkpoint|proof-key|blob)")
    astore.add_argument("digest", help="sha256 digest (64-hex)")
    astore.add_argument("path", help="Source file path")
    astore.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    astore.add_argument("--name", default="", help="Optional destination filename override")
    astore.add_argument("--overwrite", action="store_true", help="Overwrite existing file if present")
    astore.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    astore.set_defaults(func=cmd_artifact_store)

    ares = art_sub.add_parser("resolve", help="Resolve an artifact by (type,digest) via CAS")
    ares.add_argument("type", help="Artifact type")
    ares.add_argument("digest", help="sha256 digest (64-hex)")
    ares.add_argument("--store-root", dest="store_root", action="append", default=[], help="Additional store root to search (repeatable)")
    ares.add_argument("--show", action="store_true", help="Print text artifacts instead of just the path")
    ares.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ares.set_defaults(func=cmd_artifact_resolve)

    air = art_sub.add_parser("index-rulesets", help="Populate dist/artifacts/ruleset with content-addressed ruleset descriptors")
    air.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    air.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    air.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    air.set_defaults(func=cmd_artifact_index_rulesets)

    ail = art_sub.add_parser("index-lawpacks", help="Populate dist/artifacts/lawpack by copying local dist/lawpacks/**/*.lawpack.zip")
    ail.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    ail.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    ail.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ail.set_defaults(func=cmd_artifact_index_lawpacks)


    ais = art_sub.add_parser("index-schemas", help="Populate dist/artifacts/schema with content-addressed JSON Schemas")
    ais.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    ais.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    ais.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ais.set_defaults(func=cmd_artifact_index_schemas)

    aiv = art_sub.add_parser("index-vcs", help="Populate dist/artifacts/vc with content-addressed VCs (payload digests)")
    aiv.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    aiv.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    aiv.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    aiv.set_defaults(func=cmd_artifact_index_vcs)

    reg = sub.add_parser("registry")
    reg_sub = reg.add_subparsers(dest="registry_cmd", required=True)

    rtl = reg_sub.add_parser("transition-types-lock", help="Generate a transition type registry lock (transition-types.lock.json)")
    rtl.add_argument("registry", help="Path to a transition type registry YAML (transition-types.yaml)")
    rtl.add_argument("--out", default="", help="Output path (default: <registry>.lock.json)")
    rtl.add_argument("--no-store", action="store_true", help="Do not write a content-addressed copy into dist/artifacts/transition-types")
    rtl.add_argument("--store-dir", default="", help="Content-addressed store directory (default: dist/artifacts/transition-types)")
    rtl.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rtl.set_defaults(func=cmd_registry_transition_types_lock)

    rts = reg_sub.add_parser("transition-types-store", help="Store a transition type registry lock snapshot by digest")
    rts.add_argument("lock", help="Path to a transition-types.lock.json")
    rts.add_argument("--store-dir", default="", help="Store directory (default: dist/artifacts/transition-types)")
    rts.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rts.set_defaults(func=cmd_registry_transition_types_store)

    rtr = reg_sub.add_parser("transition-types-resolve", help="Resolve a transition type registry lock snapshot by digest")
    rtr.add_argument("digest", help="snapshot_digest_sha256 (64-hex)")
    rtr.add_argument("--store-dir", action="append", default=[], help="Additional search directory (repeatable)")
    rtr.add_argument("--show", action="store_true", help="Print the lock JSON instead of the path")
    rtr.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rtr.set_defaults(func=cmd_registry_transition_types_resolve)

    d = sub.add_parser("diff")
    d.add_argument("a", help="First stack.lock path")
    d.add_argument("b", help="Second stack.lock path")
    d.set_defaults(func=cmd_diff)

    p = sub.add_parser("publish")
    p.add_argument("path", help="Repo, bundle, or module directory")
    p.add_argument("--out-dir", default="dist/publish")
    p.add_argument("--pdf", action="store_true")
    p.set_defaults(func=cmd_publish)

    args = ap.parse_args()
    return args.func(args)

if __name__ == "__main__":
    sys.exit(main())
