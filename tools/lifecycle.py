"""Corridor lifecycle state machine utilities.

This module provides a minimal but strict reference implementation for applying
lifecycle transitions to a corridor instance, gated by a state machine policy.

Design goals:
- deterministic validation (schema + policy)
- evidence-gated transitions (fork alarm / fork resolution, etc.)
- ArtifactRef-friendly evidence plumbing (VCs referenced by digest and/or uri)

The lifecycle mechanism is intentionally generic: the state machine defines the
meaning of states, and transition rules can reference evidence types without
hard-coding corridor semantics.
"""

from __future__ import annotations

import json
import pathlib
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Sequence, Set, Tuple

from tools.msez import REPO_ROOT, load_json, schema_validator


FINALITY_ORDER = {
    "proposed": 0,
    "receipt_signed": 1,
    "checkpoint_signed": 2,
    "watcher_quorum": 3,
    "l1_anchored": 4,
    "legally_recognized": 5,
}


def default_state_machine_path(repo_root: pathlib.Path = REPO_ROOT) -> pathlib.Path:
    return repo_root / "governance" / "corridor.lifecycle.state-machine.v1.json"


def _vc_types(vc: Dict[str, Any]) -> Set[str]:
    t = vc.get("type")
    out: Set[str] = set()
    if isinstance(t, list):
        for x in t:
            if isinstance(x, str):
                out.add(x)
    elif isinstance(t, str):
        out.add(t)
    return out


def _is_vc(obj: Any) -> bool:
    return isinstance(obj, dict) and isinstance(obj.get("credentialSubject"), dict) and "proof" in obj


def load_state_machine(path: pathlib.Path) -> Dict[str, Any]:
    sm = load_json(path)
    v = schema_validator(REPO_ROOT / "schemas" / "corridor.state-machine.schema.json")
    errs = list(v.iter_errors(sm))
    if errs:
        raise ValueError(f"invalid corridor state-machine: {path}: {errs[0].message}")
    return sm


def load_lifecycle(path: pathlib.Path) -> Dict[str, Any]:
    obj = load_json(path)
    v = schema_validator(REPO_ROOT / "schemas" / "corridor.lifecycle.schema.json")
    errs = list(v.iter_errors(obj))
    if errs:
        raise ValueError(f"invalid corridor lifecycle: {path}: {errs[0].message}")
    return obj


def _resolve_evidence_vcs(
    evidence: Sequence[Any],
    *,
    repo_root: pathlib.Path = REPO_ROOT,
) -> Tuple[List[Dict[str, Any]], List[str]]:
    """Resolve evidence inputs into concrete VC objects.

    Evidence entries may be:
    - a VC object
    - an ArtifactRef (dict with digest_sha256 and optional uri)

    Returns: (vcs, errors)
    """
    errors: List[str] = []
    vcs: List[Dict[str, Any]] = []

    import tools.artifacts as artifact_cas

    for e in evidence or []:
        if _is_vc(e):
            vcs.append(e)
            continue

        if isinstance(e, dict) and e.get("digest_sha256"):
            uri = str(e.get("uri") or "").strip()
            if uri:
                p = pathlib.Path(uri)
                if p.exists():
                    try:
                        vcs.append(json.loads(p.read_text(encoding="utf-8")))
                        continue
                    except Exception as ex:
                        errors.append(f"failed to load evidence VC from uri={uri}: {ex}")
                        continue

            at = str(e.get("artifact_type") or "vc")
            digest = str(e.get("digest_sha256"))
            try:
                apath = artifact_cas.resolve_artifact_by_digest(at, digest, repo_root=repo_root)
                if apath:
                    vcs.append(json.loads(pathlib.Path(apath).read_text(encoding="utf-8")))
                else:
                    errors.append(f"evidence artifact not found: {at}:{digest}")
            except FileNotFoundError:
                errors.append(f"evidence artifact missing: {at}:{digest}")
            except Exception as ex:
                errors.append(f"evidence resolve error for {at}:{digest}: {ex}")
            continue

        errors.append("unsupported evidence entry (expected VC or ArtifactRef)")

    return vcs, errors


def apply_lifecycle_transition(
    lifecycle: Dict[str, Any],
    transition_vc: Dict[str, Any],
    *,
    state_machine: Dict[str, Any],
    evidence: Optional[Sequence[Any]] = None,
    finality_status: Optional[Dict[str, Any]] = None,
    repo_root: pathlib.Path = REPO_ROOT,
    verify_signatures: bool = True,
) -> Tuple[Dict[str, Any], List[str]]:
    """Apply a lifecycle transition VC to a lifecycle record.

    Returns: (updated_lifecycle, errors)
    """
    errors: List[str] = []

    # Validate lifecycle schema
    lc_v = schema_validator(REPO_ROOT / "schemas" / "corridor.lifecycle.schema.json")
    lc_errs = list(lc_v.iter_errors(lifecycle))
    if lc_errs:
        return lifecycle, [f"invalid lifecycle schema: {lc_errs[0].message}"]

    # Validate transition VC schema
    tr_v = schema_validator(REPO_ROOT / "schemas" / "vc.corridor-lifecycle-transition.schema.json")
    tr_errs = list(tr_v.iter_errors(transition_vc))
    if tr_errs:
        return lifecycle, [f"invalid lifecycle transition VC schema: {tr_errs[0].message}"]

    # Optional signature verification
    if verify_signatures:
        from tools.vc import verify_credential

        results = verify_credential(transition_vc)
        if not any(getattr(r, "ok", False) for r in results):
            return lifecycle, ["transition VC signature not verified"]

    subj = transition_vc.get("credentialSubject") or {}
    corridor_id = str(subj.get("corridor_id") or "")
    if corridor_id != str(lifecycle.get("corridor_id")):
        return lifecycle, ["transition corridor_id does not match lifecycle"]

    from_state = str(subj.get("from_state") or "")
    to_state = str(subj.get("to_state") or "")

    if from_state != str(lifecycle.get("state")):
        return lifecycle, [f"from_state mismatch: lifecycle={lifecycle.get('state')} transition={from_state}"]

    # Find transition rule
    rules = state_machine.get("allowed_transitions") or []
    rule = None
    for r in rules:
        if str(r.get("from")) == from_state and str(r.get("to")) == to_state:
            rule = r
            break
    if not rule:
        return lifecycle, [f"transition not allowed by state machine: {from_state} -> {to_state}"]

    # Enforce finality threshold (if required)
    req_finality = str(rule.get("requires_finality_level") or "").strip()
    if req_finality:
        if not finality_status or not isinstance(finality_status, dict):
            return lifecycle, [f"transition requires finality level '{req_finality}' but no finality_status was provided"]
        lvl = str(finality_status.get("finality_level") or "").strip()
        if FINALITY_ORDER.get(lvl, -1) < FINALITY_ORDER.get(req_finality, 10**9):
            return lifecycle, [f"finality_level '{lvl}' is below required '{req_finality}'"]

    # Resolve evidence
    evidence_inputs: List[Any] = []
    if evidence:
        evidence_inputs.extend(list(evidence))
    if subj.get("evidence"):
        if isinstance(subj.get("evidence"), list):
            evidence_inputs.extend(list(subj.get("evidence")))

    ev_vcs, ev_errs = _resolve_evidence_vcs(evidence_inputs, repo_root=repo_root)
    errors.extend(ev_errs)
    if errors:
        return lifecycle, errors

    if verify_signatures:
        from tools.vc import verify_credential

        for i, ev in enumerate(ev_vcs):
            results = verify_credential(ev)
            if not any(getattr(r, "ok", False) for r in results):
                return lifecycle, [f"evidence VC signature not verified (index={i})"]

    # Evidence gating by VC type(s)
    required_types = rule.get("requires_evidence_vc_types") or []
    if required_types:
        present_types: Set[str] = set()
        for ev in ev_vcs:
            present_types.update(_vc_types(ev))
        missing = [t for t in required_types if t not in present_types]
        if missing:
            return lifecycle, [f"missing required evidence VC types: {missing}"]

    # Apply transition
    updated = dict(lifecycle)
    updated["state"] = to_state
    transitioned_at = str(subj.get("transitioned_at") or "")
    if transitioned_at:
        updated["since"] = transitioned_at

    # Append history
    hist = list(updated.get("history") or [])
    hist.append(
        {
            "from": from_state,
            "to": to_state,
            "transitioned_at": transitioned_at,
            "transition_id": subj.get("transition_id"),
            "vc_id": transition_vc.get("id"),
        }
    )
    updated["history"] = hist

    return updated, []
