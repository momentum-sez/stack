"""Smart Asset utilities for the MSEZ Stack.

This module implements a *non-blockchain* Smart Asset reference layer:

 - asset_id = sha256(JCS(genesis_without_asset_id))
 - deterministic state_root_sha256 = sha256(JCS(state))
 - lightweight compliance evaluation across jurisdictional bindings

The primitives here are intentionally simple and file/CAS-friendly so that
production deployments can substitute richer engines (ZK, TEEs, policy-as-code)
while keeping the same commitment surfaces (digests + ArtifactRefs).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
import tempfile
from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional, Set

import yaml

from tools import artifacts as artifact_cas
from tools.lawpack import jcs_canonicalize as canonicalize_json  # type: ignore
from tools.vc import (
    now_rfc3339,
    add_ed25519_proof,
    load_ed25519_private_key_from_jwk,
)  # type: ignore


REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def sha256_hex(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _load_yaml(path: pathlib.Path) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def asset_id_from_genesis(genesis: Dict[str, Any]) -> str:
    """Compute asset_id = sha256(JCS(genesis_without_asset_id)).

    The genesis document may optionally include an informational `asset_id` field.
    When computing the digest commitment, that derived field is **excluded**.
    """

    g = dict(genesis)
    g.pop("asset_id", None)
    return sha256_hex(canonicalize_json(g))


def state_root_from_state(state: Any) -> str:
    """Compute state_root_sha256 = sha256(JCS(state))."""
    return sha256_hex(canonicalize_json(state))


def build_genesis(
    *,
    stack_spec_version: str,
    asset_name: str,
    asset_class: str,
    description: str = "",
    creator: str = "",
    home_harbor_id: str = "",
    initial_metadata: Optional[Dict[str, Any]] = None,
    created_at: Optional[str] = None,
) -> Dict[str, Any]:
    created_at = created_at or now_rfc3339()
    g: Dict[str, Any] = {
        "type": "SmartAssetGenesis",
        "stack_spec_version": stack_spec_version,
        "created_at": created_at,
        "asset_name": asset_name,
        "asset_class": asset_class,
    }
    if description:
        g["description"] = description
    if creator:
        g["creator"] = creator
    if home_harbor_id:
        g["home_harbor_id"] = home_harbor_id
    if initial_metadata:
        g["initial_metadata"] = initial_metadata
    # asset_id is derived; we include it as an informational convenience.
    g["asset_id"] = asset_id_from_genesis(g)
    return g


def _coerce_path(p: str) -> pathlib.Path:
    path = pathlib.Path(p)
    if not path.is_absolute():
        path = REPO_ROOT / path
    return path


def _write_json(path: pathlib.Path, obj: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def _maybe_store_artifact(
    *,
    store_root: pathlib.Path,
    artifact_type: str,
    digest: str,
    src_path: pathlib.Path,
    overwrite: bool = False,
) -> pathlib.Path:
    store_root.mkdir(parents=True, exist_ok=True)
    out_path = artifact_cas.store_artifact_file(
        artifact_type=artifact_type,
        digest_sha256=digest,
        src_path=src_path,
        repo_root=REPO_ROOT,
        store_root=store_root,
        overwrite=overwrite,
    )
    return out_path


def _as_list(x: Any) -> List[Any]:
    if x is None:
        return []
    if isinstance(x, list):
        return x
    return [x]


def _extract_attestation_kinds_from_artifact(
    *,
    artifact_ref: Dict[str, Any],
    store_roots: List[pathlib.Path],
) -> Set[str]:
    """Resolve an artifact ref and return attestation kinds if it is a SmartAssetAttestation."""
    at = str(artifact_ref.get("artifact_type") or "").strip()
    dg = str(artifact_ref.get("digest_sha256") or "").strip()
    if not at or not dg:
        return set()

    # We only parse structured attestations from known artifact types.
    if at not in {"smart-asset-attestation", "smart-asset.attestation"}:
        return set()

    try:
        resolved = artifact_cas.resolve_artifact_by_digest(
            artifact_type=at,
            digest_sha256=dg,
            repo_root=REPO_ROOT,
            store_roots=store_roots,
        )
    except Exception:
        return set()

    try:
        obj = json.loads(resolved.read_text(encoding="utf-8"))
    except Exception:
        return set()

    if not isinstance(obj, dict):
        return set()
    if obj.get("type") != "SmartAssetAttestation":
        return set()
    k = str(obj.get("kind") or "").strip()
    return {k} if k else set()


@dataclass
class BindingComplianceResult:
    harbor_id: str
    binding_status: str
    shard_role: str
    allowed: bool
    reasons: List[str]
    missing_attestations: List[str]
    present_attestations: List[str]


def evaluate_transition_compliance(
    *,
    registry_vc: Dict[str, Any],
    transition_envelope: Dict[str, Any],
    store_roots: List[pathlib.Path],
) -> List[BindingComplianceResult]:
    """Evaluate a transition against every jurisdiction binding.

    This is a deliberately *declarative* evaluator:
    - allowed_transition_kinds gates on transition kind
    - required_attestations gates on presence of SmartAssetAttestation kinds

    Supported input shapes:
    - Legacy Smart Asset envelope: {"type":"TransitionEnvelope","transition_kind":...}
    - Stack-standard envelope: {"type":"MSEZTransitionEnvelope","kind":...}
    - Corridor receipt wrapper: {"type":"MSEZCorridorStateReceipt","transition":{...}}
    """

    # Allow callers to pass a corridor receipt and evaluate its embedded transition.
    env = transition_envelope
    if isinstance(env, dict) and str(env.get("type") or "") == "MSEZCorridorStateReceipt":
        inner = env.get("transition")
        if isinstance(inner, dict):
            env = inner

    def _transition_kind(e: Dict[str, Any]) -> str:
        # Backward compatible (TransitionEnvelope) + stack-standard (MSEZTransitionEnvelope)
        for k in ("transition_kind", "kind"):
            v = str(e.get(k) or "").strip()
            if v:
                return v
        return "noop"

    subj = registry_vc.get("credentialSubject") if isinstance(registry_vc, dict) else None
    if not isinstance(subj, dict):
        raise ValueError("registry_vc must be a VC JSON object with credentialSubject")

    bindings = subj.get("jurisdiction_bindings")
    if not isinstance(bindings, list):
        raise ValueError("registry_vc.credentialSubject.jurisdiction_bindings must be an array")

    t_kind = _transition_kind(env)

    # Collect attestation kinds from referenced artifacts.
    present_attestations: Set[str] = set()
    for att in _as_list(env.get("attachments")):
        if isinstance(att, dict) and "artifact_type" in att and "digest_sha256" in att:
            present_attestations |= _extract_attestation_kinds_from_artifact(
                artifact_ref=att,
                store_roots=store_roots,
            )

    out: List[BindingComplianceResult] = []

    for b in bindings:
        if not isinstance(b, dict):
            continue

        harbor_id = str(b.get("harbor_id") or "").strip()
        binding_status = str(b.get("binding_status") or "").strip() or "active"
        shard_role = str(b.get("shard_role") or "").strip() or "primary"
        reasons: List[str] = []
        missing: List[str] = []

        if binding_status != "active":
            out.append(
                BindingComplianceResult(
                    harbor_id=harbor_id,
                    binding_status=binding_status,
                    shard_role=shard_role,
                    allowed=False,
                    reasons=[f"binding_status={binding_status}"],
                    missing_attestations=[],
                    present_attestations=sorted(present_attestations),
                )
            )
            continue

        cp = b.get("compliance_profile")
        if not isinstance(cp, dict):
            cp = {}

        allowed_kinds = cp.get("allowed_transition_kinds")
        if isinstance(allowed_kinds, list) and allowed_kinds:
            allow_set = {str(x).strip() for x in allowed_kinds if isinstance(x, str) and x.strip()}
            if allow_set and t_kind not in allow_set:
                reasons.append(f"transition_kind '{t_kind}' not allowed")

        req_map = cp.get("required_attestations")
        default_req = cp.get("default_required_attestations")

        required: Set[str] = set()
        if isinstance(default_req, list):
            required |= {str(x).strip() for x in default_req if isinstance(x, str) and x.strip()}
        if isinstance(req_map, dict):
            v = req_map.get(t_kind)
            if isinstance(v, list):
                required |= {str(x).strip() for x in v if isinstance(x, str) and x.strip()}

        for rk in sorted(required):
            if rk and rk not in present_attestations:
                missing.append(rk)
        if missing:
            reasons.append("missing required attestations")

        allowed = not reasons
        out.append(
            BindingComplianceResult(
                harbor_id=harbor_id,
                binding_status=binding_status,
                shard_role=shard_role,
                allowed=allowed,
                reasons=reasons,
                missing_attestations=missing,
                present_attestations=sorted(present_attestations),
            )
        )

    return out


# ---------------------------------------------------------------------------
# CLI commands (wired in tools/msez.py)


def cmd_asset_genesis_init(args: argparse.Namespace) -> int:
    out_path = _coerce_path(args.out) if args.out else None
    created_at = args.created_at.strip() if getattr(args, "created_at", "") else ""
    g = build_genesis(
        stack_spec_version=args.stack_spec_version,
        asset_name=args.asset_name,
        asset_class=args.asset_class,
        description=args.description or "",
        creator=args.creator or "",
        home_harbor_id=args.home_harbor_id or "",
        initial_metadata=_load_yaml(_coerce_path(args.metadata)) if args.metadata else None,
        created_at=created_at or None,
    )

    if out_path:
        _write_json(out_path, g)
        print("Wrote genesis to", out_path)

    print("asset_id", g.get("asset_id"))

    if args.store:
        store_root = _coerce_path(args.store_root) if args.store_root else (REPO_ROOT / "dist" / "artifacts")
        # Store as content-addressed artifact where digest == asset_id.
        if not out_path:
            # Write to a temporary path under dist/tmp
            tmp = REPO_ROOT / "dist" / "tmp" / f"smart-asset.genesis.{g['asset_id']}.json"
            _write_json(tmp, g)
            out_path = tmp
        stored = _maybe_store_artifact(
            store_root=store_root,
            artifact_type="smart-asset-genesis",
            digest=str(g["asset_id"]),
            src_path=out_path,
            overwrite=args.overwrite,
        )
        print("Stored artifact", stored)

    return 0


def cmd_asset_genesis_hash(args: argparse.Namespace) -> int:
    gp = _coerce_path(args.genesis)
    g = _load_json(gp)
    if not isinstance(g, dict):
        raise ValueError("genesis must be a JSON object")
    aid = asset_id_from_genesis(g)
    print(aid)
    return 0


def _bindings_from_file(path: pathlib.Path) -> List[Dict[str, Any]]:
    obj = _load_yaml(path) if path.suffix.lower() in {".yaml", ".yml"} else _load_json(path)
    if isinstance(obj, dict) and isinstance(obj.get("jurisdiction_bindings"), list):
        obj = obj.get("jurisdiction_bindings")
    if not isinstance(obj, list):
        raise ValueError("bindings file must be a list (or an object with jurisdiction_bindings: [...])")
    out: List[Dict[str, Any]] = []
    for b in obj:
        if isinstance(b, dict):
            out.append(b)
    if not out:
        raise ValueError("bindings file contains no bindings")
    return out


def cmd_asset_registry_init(args: argparse.Namespace) -> int:
    genesis_path = _coerce_path(args.genesis)
    g = _load_json(genesis_path)
    if not isinstance(g, dict):
        raise ValueError("genesis must be a JSON object")
    asset_id = str(g.get("asset_id") or "").strip() or asset_id_from_genesis(g)

    bindings_path = _coerce_path(args.bindings)
    bindings = _bindings_from_file(bindings_path)

    quorum_policy_obj: Optional[Dict[str, Any]] = None
    qp_path_str = str(getattr(args, "quorum_policy", "") or "").strip()
    if qp_path_str:
        qp_path = _coerce_path(qp_path_str)
        if qp_path.suffix.lower() in (".yaml", ".yml"):
            qp_loaded = _load_yaml(qp_path)
        else:
            qp_loaded = _load_json(qp_path)
        if not isinstance(qp_loaded, dict):
            raise ValueError("quorum_policy file must contain a JSON/YAML object")
        quorum_policy_obj = qp_loaded


    vc_id = args.id.strip() if getattr(args, "id", "") else ""
    if not vc_id:
        vc_id = f"urn:msez:vc:smart-asset-registry:{asset_id}"

    issuer = args.issuer.strip()
    issuance_date = args.issuance_date.strip() if args.issuance_date else now_rfc3339()

    # Genesis can be stored/committed by ArtifactRef, even if the VC itself lives in CAS(v...)
    genesis_ref = {
        "artifact_type": "smart-asset-genesis",
        "digest_sha256": asset_id,
        "uri": str(args.genesis),
        "media_type": "application/json",
    }

    vcj: Dict[str, Any] = {
        "@context": ["https://www.w3.org/2018/credentials/v1"],
        "id": vc_id,
        "type": ["VerifiableCredential", "MsezSmartAssetRegistryCredential"],
        "issuer": issuer,
        "issuanceDate": issuance_date,
        "credentialSubject": {
            "asset_id": asset_id,
            "stack_spec_version": args.stack_spec_version,
            "asset_genesis": genesis_ref,
            "asset_name": g.get("asset_name"),
            "asset_class": g.get("asset_class"),
            "jurisdiction_bindings": bindings,
        },
    }

    if quorum_policy_obj:
        vcj["credentialSubject"]["quorum_policy"] = quorum_policy_obj

    out_path = _coerce_path(args.out) if args.out else genesis_path.with_name("smart-asset.registry.vc.json")

    if args.sign:
        # Defer to msez vc sign semantics (OKP/Ed25519 JWK file).
        from tools.vc import load_ed25519_private_key_from_jwk, add_ed25519_proof  # type: ignore

        key_path = _coerce_path(args.key)
        jwk = _load_json(key_path)
        priv, did = load_ed25519_private_key_from_jwk(jwk)
        vm = args.verification_method.strip() if args.verification_method else (did + "#key-1")
        add_ed25519_proof(vcj, priv, vm, proof_purpose=args.purpose)
        _write_json(out_path, vcj)
        print("Wrote signed registry VC to", out_path)
    else:
        _write_json(out_path, vcj)
        print("Wrote unsigned registry VC to", out_path)

    return 0


def cmd_asset_checkpoint_build(args: argparse.Namespace) -> int:
    state_path = _coerce_path(args.state)
    state = _load_json(state_path)
    sr = state_root_from_state(state)

    # Normalize + validate parents (DAG semantics). Parents are checkpoint digests (sha256 hex).
    parents_in: List[str] = []
    for p in _as_list(args.parent):
        if not p:
            continue
        d = str(p).strip().lower()
        if not d:
            continue
        if not artifact_cas.SHA256_HEX_RE.match(d):
            raise ValueError(f"invalid --parent digest (expected 64 lowercase hex): {p}")
        parents_in.append(d)
    parents = sorted(set(parents_in))

    ck: Dict[str, Any] = {
        "type": "SmartAssetCheckpoint",
        "asset_id": args.asset_id,
        "as_of": args.as_of.strip() if args.as_of else now_rfc3339(),
        "state_root_sha256": sr,
        "parents": parents,
        "attachments": [],
    }
    if args.notes:
        ck["notes"] = args.notes

    out_path = _coerce_path(args.out) if args.out else state_path.with_name(f"smart-asset.checkpoint.{sr}.json")
    _write_json(out_path, ck)
    print("Wrote checkpoint to", out_path)
    print("state_root_sha256", sr)

    if args.store:
        store_root = _coerce_path(args.store_root) if args.store_root else (REPO_ROOT / "dist" / "artifacts")
        stored = _maybe_store_artifact(
            store_root=store_root,
            artifact_type="smart-asset-checkpoint",
            digest=sr,
            src_path=out_path,
            overwrite=args.overwrite,
        )
        print("Stored artifact", stored)
    return 0


def cmd_asset_attestation_init(args: argparse.Namespace) -> int:
    att: Dict[str, Any] = {
        "type": "SmartAssetAttestation",
        "asset_id": args.asset_id,
        "issued_at": args.issued_at.strip() if args.issued_at else now_rfc3339(),
        "issuer": args.issuer,
        "kind": args.kind,
    }
    if args.claims:
        claims_path = _coerce_path(args.claims)
        claims_obj = _load_json(claims_path) if claims_path.suffix.lower() == ".json" else _load_yaml(claims_path)
        att["claims"] = claims_obj
        att["claims_digest_sha256"] = sha256_hex(canonicalize_json(claims_obj))

    out_path = _coerce_path(args.out) if args.out else (REPO_ROOT / "dist" / "tmp" / f"smart-asset.attestation.{args.kind}.json")
    _write_json(out_path, att)
    print("Wrote attestation to", out_path)
    dg = sha256_hex(canonicalize_json(att))
    print("attestation_digest_sha256", dg)

    if args.store:
        store_root = _coerce_path(args.store_root) if args.store_root else (REPO_ROOT / "dist" / "artifacts")
        stored = _maybe_store_artifact(
            store_root=store_root,
            artifact_type="smart-asset-attestation",
            digest=dg,
            src_path=out_path,
            overwrite=args.overwrite,
        )
        print("Stored artifact", stored)
    return 0



def verify_receipt_chain_continuity(receipts: List[Dict[str, Any]]) -> List[str]:
    """Verify that a list of corridor receipts forms a continuous hash chain.

    BUG FIX #100: Each receipt's previous_hash must match the SHA-256 digest
    of the canonicalized prior receipt to ensure chain integrity.  Returns a
    list of error strings (empty if the chain is valid).
    """
    errors: List[str] = []
    if not receipts:
        return errors

    for i in range(1, len(receipts)):
        prev = receipts[i - 1]
        curr = receipts[i]

        # Compute expected hash from the previous receipt
        expected_hash = sha256_hex(canonicalize_json(prev))
        actual_prev_hash = curr.get("previous_hash") or curr.get("prev_root") or curr.get("prev_hash")

        if actual_prev_hash is None:
            errors.append(
                f"receipt[{i}]: missing previous_hash/prev_root field"
            )
        elif actual_prev_hash != expected_hash:
            errors.append(
                f"receipt[{i}]: chain break - previous_hash={actual_prev_hash!r} "
                f"does not match hash of receipt[{i - 1}]={expected_hash!r}"
            )

        # Verify sequence numbers are strictly increasing (if present)
        prev_seq = prev.get("seq")
        curr_seq = curr.get("seq")
        if prev_seq is not None and curr_seq is not None:
            if curr_seq != prev_seq + 1:
                errors.append(
                    f"receipt[{i}]: sequence gap - expected seq={prev_seq + 1}, got {curr_seq}"
                )

    return errors


def _extract_transition_envelope(obj: Dict[str, Any]) -> Dict[str, Any]:
    """Unwrap a transition envelope from either a bare envelope or a corridor receipt."""
    if not isinstance(obj, dict):
        raise ValueError("transition object must be a JSON object")

    ttype = str(obj.get("type") or "").strip()
    if ttype in ("MSEZCorridorReceipt", "MSEZCorridorStateReceipt"):
        tr = obj.get("transition")
        if isinstance(tr, dict):
            return tr
    # Fallback: assume obj itself is an envelope
    return obj


def _transition_kind_from_envelope(env: Dict[str, Any]) -> str:
    # TransitionEnvelope (legacy) may use transition_kind; MSEZTransitionEnvelope uses kind.
    k = env.get("transition_kind") or env.get("kind") or env.get("transitionKind")
    return str(k or "").strip()


def _load_optional_obj(path_str: str) -> Any:
    p = _coerce_path(path_str)
    if p.suffix.lower() in (".yaml", ".yml"):
        return _load_yaml(p)
    return _load_json(p)


def _evaluate_quorum_policy(
    registry_vc: Dict[str, Any],
    transition_envelope: Dict[str, Any],
    results: List[SmartAssetComplianceResult],
) -> Optional[Dict[str, Any]]:
    """Evaluate an optional quorum policy attached to the registry VC.

    Returns a JSON-serializable dict with decision details, or None if no policy exists.
    """
    cs = registry_vc.get("credentialSubject") if isinstance(registry_vc, dict) else None
    if not isinstance(cs, dict):
        return None
    qp = cs.get("quorum_policy")
    if not isinstance(qp, dict):
        return None

    env = _extract_transition_envelope(transition_envelope)
    kind = _transition_kind_from_envelope(env) or "(unknown-kind)"

    rule: Optional[Dict[str, Any]] = None
    by_kind = qp.get("by_transition_kind")
    if isinstance(by_kind, dict):
        rk = by_kind.get(kind)
        if isinstance(rk, dict):
            rule = rk
    if rule is None:
        rd = qp.get("default")
        if isinstance(rd, dict):
            rule = rd
    if rule is None:
        return None

    mode = str(rule.get("mode") or "all_active").strip()
    if mode not in ("all_active", "quorum", "any_active"):
        mode = "all_active"

    min_harbors = 0
    if mode == "quorum":
        try:
            min_harbors = int(rule.get("min_harbors") or 0)
        except Exception:
            min_harbors = 0
        if min_harbors < 1:
            min_harbors = 1

    eligible_roles = rule.get("eligible_shard_roles")
    if isinstance(eligible_roles, str):
        eligible_roles = [eligible_roles]
    roles_set: Optional[Set[str]] = None
    if isinstance(eligible_roles, list):
        roles_set = {str(x) for x in eligible_roles if str(x).strip()}

    allowlist = rule.get("harbor_ids")
    if isinstance(allowlist, str):
        allowlist = [allowlist]
    allow_set: Optional[Set[str]] = None
    if isinstance(allowlist, list):
        allow_set = {str(x) for x in allowlist if str(x).strip()}

    required = rule.get("required_harbor_ids")
    if isinstance(required, str):
        required = [required]
    required_set: Set[str] = set()
    if isinstance(required, list):
        required_set = {str(x) for x in required if str(x).strip()}

    # Active-only candidates.
    candidates = [r for r in results if r.binding_status == "active"]
    if roles_set is not None:
        candidates = [r for r in candidates if r.shard_role in roles_set]
    if allow_set is not None:
        candidates = [r for r in candidates if r.harbor_id in allow_set]

    eligible_harbors = [r.harbor_id for r in candidates]
    allowed_harbors = [r.harbor_id for r in candidates if r.allowed]

    required_missing = sorted([hid for hid in required_set if hid not in allowed_harbors])
    required_ok = len(required_missing) == 0

    ok = False
    reason = ""

    if not candidates:
        ok = False
        reason = "no eligible active harbors"
    elif not required_ok:
        ok = False
        reason = f"required_harbor_ids not satisfied: {','.join(required_missing)}"
    elif mode == "any_active":
        ok = len(allowed_harbors) >= 1
        reason = f"allowed={len(allowed_harbors)} of eligible={len(eligible_harbors)}"
    elif mode == "quorum":
        ok = len(allowed_harbors) >= min_harbors
        reason = f"allowed={len(allowed_harbors)} of eligible={len(eligible_harbors)} min_harbors={min_harbors}"
    else:  # all_active
        ok = len(allowed_harbors) == len(eligible_harbors)
        reason = f"allowed={len(allowed_harbors)} of eligible={len(eligible_harbors)}"

    return {
        "transition_kind": kind,
        "mode": mode,
        "min_harbors": min_harbors if mode == "quorum" else None,
        "eligible_shard_roles": sorted(list(roles_set)) if roles_set is not None else None,
        "eligible_harbors": eligible_harbors,
        "allowed_harbors": allowed_harbors,
        "required_harbor_ids": sorted(list(required_set)) if required_set else [],
        "ok": bool(ok),
        "reason": reason,
    }


def cmd_asset_compliance_eval(args: argparse.Namespace) -> int:
    reg_path = _coerce_path(args.registry)
    tr_path = _coerce_path(args.transition)
    reg = _load_json(reg_path)
    tr = _load_json(tr_path)
    if not isinstance(reg, dict) or not isinstance(tr, dict):
        raise ValueError("registry and transition must be JSON objects")

    store_roots = [_coerce_path(args.store_root)] if getattr(args, "store_root", "") else [REPO_ROOT / "dist" / "artifacts"]
    # Allow multiple --store-root in future by accepting comma-separated for now.
    if getattr(args, "extra_store_root", ""):
        for s in str(args.extra_store_root).split(","):
            if s.strip():
                store_roots.append(_coerce_path(s.strip()))

    results = evaluate_transition_compliance(
        registry_vc=reg,
        transition_envelope=tr,
        store_roots=store_roots,
    )

    quorum = _evaluate_quorum_policy(reg, tr, results)

    # Default behavior (no policy): require all *active* bindings to allow.
    active_results = [r for r in results if r.binding_status == "active"]
    ok_all_active = bool(active_results) and all(r.allowed for r in active_results)

    ok = quorum["ok"] if quorum is not None else ok_all_active

    if getattr(args, "json", False):
        out = {
            "ok": bool(ok),
            "registry": str(args.registry),
            "transition": str(args.transition),
            "results": [r.__dict__ for r in results],
            "quorum": quorum,
        }
        print(json.dumps(out, indent=2, ensure_ascii=False))
        return 0 if ok else 2

    # Human-friendly report
    for r in results:
        if r.binding_status != "active":
            status = "SKIP"
        else:
            status = "OK" if r.allowed else "FAIL"

        line = f"{status} harbor={r.harbor_id} shard_role={r.shard_role} binding_status={r.binding_status}"
        if r.missing_attestations:
            line += f" missing={','.join(r.missing_attestations)}"
        if r.reasons:
            line += f" reasons={';'.join(r.reasons)}"
        print(line)

    if quorum is not None:
        q_status = "OK" if quorum.get("ok") else "FAIL"
        line = (
            f"{q_status} quorum mode={quorum.get('mode')} "
            f"eligible={len(quorum.get('eligible_harbors') or [])} "
            f"allowed={len(quorum.get('allowed_harbors') or [])}"
        )
        if quorum.get("mode") == "quorum":
            line += f" min_harbors={quorum.get('min_harbors')}"
        if quorum.get("required_harbor_ids"):
            line += f" required={','.join(quorum.get('required_harbor_ids') or [])}"
        if quorum.get("reason"):
            line += f" reason={quorum.get('reason')}"
        print(line)
    else:
        # Clarify the default decision rule.
        print(f"{'OK' if ok else 'FAIL'} default_policy=all_active eligible={len(active_results)} allowed={sum(1 for r in active_results if r.allowed)}")

    return 0 if ok else 2


def cmd_asset_rule_eval_evidence_init(args: argparse.Namespace) -> int:
    """Create a rule evaluation evidence artifact (optionally sign + store).

    This produces a portable JSON object that can be attached to a transition envelope
    (as an ArtifactRef) and carried inside witness bundles.
    """
    tr_path = _coerce_path(args.transition)
    tr_obj = _load_json(tr_path)
    if not isinstance(tr_obj, dict):
        raise ValueError("transition must be a JSON object")

    env = _extract_transition_envelope(tr_obj)
    kind = _transition_kind_from_envelope(env) or "(unknown-kind)"

    # Prefer committed digests when present; otherwise fall back to hashing embedded payload.
    payload_sha = str(env.get("payload_sha256") or "").strip()
    if not payload_sha:
        payload = env.get("payload")
        if payload is not None:
            payload_sha = sha256_hex(canonicalize_json(payload))

    evidence: Dict[str, Any] = {
        "type": "MSEZRuleEvaluationEvidence",
        "stack_spec_version": getattr(args, "stack_spec_version", ""),
        "evaluated_at": args.evaluated_at.strip() if getattr(args, "evaluated_at", "") else now_rfc3339(),
        "harbor_id": args.harbor_id.strip(),
        "transition_kind": kind,
        "result": args.result,
    }
    if getattr(args, "jurisdiction_id", ""):
        evidence["jurisdiction_id"] = str(args.jurisdiction_id).strip()
    if payload_sha:
        evidence["payload_sha256"] = payload_sha

    for field in ("schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"):
        v = env.get(field)
        if v:
            evidence[field] = v

    if getattr(args, "notes", ""):
        evidence["notes"] = str(args.notes)

    if getattr(args, "violations", ""):
        vio_loaded = _load_optional_obj(str(args.violations))
        evidence["violations"] = vio_loaded

    # Compute semantic digest: sha256(JCS(evidence_without_proof)).
    tmp = dict(evidence)
    tmp.pop("proof", None)
    digest = sha256_hex(canonicalize_json(tmp))

    # Optionally sign (Ed25519 JWK)
    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            raise ValueError("--key is required when --sign is set")
        jwk_obj = _load_json(_coerce_path(key_path))
        priv = load_ed25519_private_key_from_jwk(jwk_obj)
        vm = str(getattr(args, "verification_method", "") or "").strip() or str(args.harbor_id)
        purpose = str(getattr(args, "purpose", "assertionMethod") or "assertionMethod")
        evidence["proof"] = [
            add_ed25519_proof(evidence, priv, verification_method=vm, proof_purpose=purpose)
        ]

    # Output
    out_path = _coerce_path(args.out) if getattr(args, "out", "") else None
    if out_path:
        out_path.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    else:
        print(json.dumps(evidence, indent=2, ensure_ascii=False))

    # Store in CAS if requested.
    if getattr(args, "store", False):
        store_root = _coerce_path(getattr(args, "store_root", "") or "") if getattr(args, "store_root", "") else (REPO_ROOT / "dist" / "artifacts")
        # We need a file path to store; if no out_path was given, materialize a temp file.
        src_path = out_path
        if src_path is None:
            src_path = pathlib.Path(tempfile.gettempdir()) / f"rule-eval-evidence.{digest}.json"
            src_path.write_text(json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        stored = artifact_cas.store_artifact_file(
            artifact_type="rule-eval-evidence",
            digest_sha256=digest,
            src_path=src_path,
            store_root=store_root,
            overwrite=getattr(args, "overwrite", False),
        )
        print(f"Stored artifact {stored}", file=sys.stderr)

    print(f"rule-eval-evidence digest_sha256={digest}", file=sys.stderr)
    return 0
