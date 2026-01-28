#!/usr/bin/env python3
"""Lawpack supply chain helpers for the MSEZ Stack.

A *lawpack* is a content-addressed artifact that packages a jurisdictional legal corpus
snapshot (typically normalized to Akoma Ntoso) plus deterministic indices and provenance.

This module provides:
- deterministic canonicalization (JCS for JSON/YAML, Exclusive XML C14N for XML)
- eId fragment indexing for Akoma Ntoso documents
- lawpack digest computation
- lawpack.zip emission + lawpack.lock.json generation

NOTE: Fetching/normalization of raw sources is implemented as a pluggable scaffold. High-fidelity
HTML/PDF -> AKN conversion is intentionally left to recipe implementations.
"""

from __future__ import annotations

import hashlib
import io
import json
import mimetypes
import os
import pathlib
import re
import tempfile
import urllib.request
import zipfile
from datetime import datetime, timezone, date
from typing import Any, Dict, List, Optional, Tuple

import yaml
from lxml import etree


SHA256_RE = re.compile(r"^[0-9a-f]{64}$")


def _ensure_json_compatible(obj: Any, *, path: str = "$", context: str = "manifest") -> None:
    """Enforce that a parsed YAML/JSON object is JSON-compatible.

    Why this exists:
      - YAML has implicit typing (timestamps/dates), which can differ across parsers.
      - The stack's reproducibility story depends on manifests being portable across
        implementations/languages.

    Strict mode uses this to reject:
      - floats (non-deterministic canonicalization edge-cases)
      - datetime/date objects (YAML implicit timestamps)
      - non-string mapping keys
      - exotic YAML tags (sets, binaries, etc.)

    When this raises, the fix is usually to *quote* values that YAML may treat as
    dates/timestamps (e.g., "2025-01-01") and to avoid floats.
    """
    if obj is None:
        return
    if isinstance(obj, (str, bool, int)):
        return
    if isinstance(obj, float):
        raise ValueError(f"{context}: {path}: floats are not allowed; use strings or integers")
    if isinstance(obj, (datetime, date)):
        # This typically comes from unquoted YAML scalars like 2025-01-01.
        raise ValueError(
            f"{context}: {path}: YAML timestamp/date detected ({type(obj).__name__}); quote it to force a string"
        )
    if isinstance(obj, list):
        for i, x in enumerate(obj):
            _ensure_json_compatible(x, path=f"{path}[{i}]", context=context)
        return
    if isinstance(obj, tuple):
        raise ValueError(f"{context}: {path}: tuples are not allowed; use YAML sequences (lists)")
    if isinstance(obj, dict):
        for k, v in obj.items():
            if not isinstance(k, str):
                raise ValueError(
                    f"{context}: {path}: non-string key {k!r} ({type(k).__name__}); keys must be strings"
                )
            key_path = f"{path}.{k}" if path else k
            _ensure_json_compatible(v, path=key_path, context=context)
        return
    raise ValueError(f"{context}: {path}: unsupported type {type(obj).__name__}; use JSON-compatible YAML")


def _load_yaml_manifest(path: pathlib.Path, *, strict: bool, context: str) -> Dict[str, Any]:
    """Load a YAML manifest and, in strict mode, enforce JSON-compatible typing."""
    try:
        obj = yaml.safe_load(path.read_text(encoding="utf-8"))
    except Exception as ex:
        raise ValueError(f"Unable to parse {context} YAML at {path}: {ex}") from ex

    if obj is None:
        obj = {}
    if not isinstance(obj, dict):
        raise ValueError(f"{context}: expected a mapping/object at top-level in {path}")

    if strict:
        _ensure_json_compatible(obj, path="$", context=context)

    return obj


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def now_rfc3339() -> str:
    return datetime.now(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")


def _coerce_json_types(obj: Any) -> Any:
    """Coerce Python objects into strict JSON types.

    - YAML loaders may produce datetime/date objects — convert them to ISO strings.
    - Floats are rejected to avoid non-JCS number edge cases (use strings for amounts).
    """
    if obj is None:
        return None
    if isinstance(obj, (str, bool, int)):
        return obj
    if isinstance(obj, float):
        raise ValueError("Floats are not allowed in JCS canonicalization. Use strings or integers.")
    if isinstance(obj, (datetime, date)):
        if isinstance(obj, datetime):
            if obj.tzinfo is None:
                obj = obj.replace(tzinfo=timezone.utc)
            return obj.astimezone(timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
        return obj.isoformat()
    if isinstance(obj, list):
        return [_coerce_json_types(x) for x in obj]
    if isinstance(obj, tuple):
        return [_coerce_json_types(x) for x in obj]
    if isinstance(obj, dict):
        out: Dict[str, Any] = {}
        for k, v in obj.items():
            out[str(k)] = _coerce_json_types(v)
        return out
    return str(obj)


def jcs_canonicalize(obj: Any) -> bytes:
    """Canonicalize JSON using a JCS-like subset (RFC 8785 compatible for objects without floats)."""
    clean = _coerce_json_types(obj)
    return json.dumps(clean, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def canonicalize_yaml(path: pathlib.Path) -> bytes:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    return jcs_canonicalize(data)


def canonicalize_json(path: pathlib.Path) -> bytes:
    data = json.loads(path.read_text(encoding="utf-8"))
    return jcs_canonicalize(data)


def canonicalize_xml(path: pathlib.Path) -> bytes:
    """Canonicalize XML via Exclusive XML C14N (no comments)."""
    parser = etree.XMLParser(resolve_entities=False, no_network=True, remove_blank_text=False, huge_tree=True)
    tree = etree.parse(str(path), parser)
    buf = io.BytesIO()
    tree.write_c14n(buf, exclusive=True, with_comments=False)
    return buf.getvalue()


def canonicalize_xml_bytes(xml_bytes: bytes) -> bytes:
    """Canonicalize XML bytes via Exclusive XML C14N (no comments)."""
    parser = etree.XMLParser(resolve_entities=False, no_network=True, remove_blank_text=False, huge_tree=True)
    root = etree.fromstring(xml_bytes, parser)
    tree = etree.ElementTree(root)
    buf = io.BytesIO()
    tree.write_c14n(buf, exclusive=True, with_comments=False)
    return buf.getvalue()


def _compute_lawpack_digest_from_zip(zip_path: pathlib.Path) -> str:
    """Recompute a lawpack digest from an on-disk lawpack.zip.

    This is used for strict verification / CI to ensure the emitted artifact bytes
    actually correspond to the digest computed from the module's inputs.
    """
    canonical_files: Dict[str, bytes] = {}
    with zipfile.ZipFile(zip_path, "r") as zf:
        names = set(zf.namelist())

        if "lawpack.yaml" not in names:
            raise ValueError("lawpack.zip missing lawpack.yaml")
        if "index.json" not in names:
            raise ValueError("lawpack.zip missing index.json")

        lawpack_obj = yaml.safe_load(zf.read("lawpack.yaml").decode("utf-8"))
        canonical_files["lawpack.yaml"] = jcs_canonicalize(lawpack_obj)

        index_obj = json.loads(zf.read("index.json").decode("utf-8"))
        canonical_files["index.json"] = jcs_canonicalize(index_obj)

        akn_names = sorted([n for n in names if n.startswith("akn/") and n.endswith(".xml")])
        if not akn_names:
            raise ValueError("lawpack.zip missing akn/*.xml")
        for n in akn_names:
            canonical_files[n] = canonicalize_xml_bytes(zf.read(n))

    return compute_lawpack_digest(canonical_files)


def find_akn_xml_files(akn_dir: pathlib.Path) -> List[pathlib.Path]:
    if not akn_dir.exists():
        return []
    out: List[pathlib.Path] = []
    for p in sorted(akn_dir.rglob("*.xml")):
        if p.is_file():
            out.append(p)
    return out


def build_eid_index(xml_path: pathlib.Path) -> Dict[str, Any]:
    """Build an eId index for a single Akoma Ntoso XML document.

    The index maps `eId` → canonical fragment hash and best-effort byte offsets.
    Offsets may be null when the fragment canonicalization does not appear as an exact substring
    of the full-document canonicalization (namespace context edge cases).
    """
    parser = etree.XMLParser(resolve_entities=False, no_network=True, remove_blank_text=False, huge_tree=True)
    tree = etree.parse(str(xml_path), parser)
    root = tree.getroot()

    full_c14n = canonicalize_xml(xml_path)

    fragments: Dict[str, Any] = {}
    for el in root.xpath('//*[@eId]'):
        eid = str(el.get("eId") or "").strip()
        if not eid:
            continue

        frag_c14n = etree.tostring(el, method="c14n", exclusive=True, with_comments=False)
        frag_hash = sha256_bytes(frag_c14n)
        xpath = tree.getpath(el)

        start = full_c14n.find(frag_c14n)
        if start >= 0:
            end = start + len(frag_c14n)
        else:
            start = None
            end = None

        fragments[eid] = {
            "sha256": frag_hash,
            "xpath": xpath,
            "byte_start": start,
            "byte_end": end,
        }

    return {
        "document_sha256": sha256_bytes(full_c14n),
        "fragments": fragments,
    }


def compute_lawpack_digest(canonical_files: Dict[str, bytes]) -> str:
    """Compute the lawpack digest over canonicalized file bytes.

    Digest definition (v1):
    SHA256( b"msez-lawpack-v1\0" + Σ(sorted(path) (path + "\0" + canonical_bytes + "\0")) )
    """
    h = hashlib.sha256()
    h.update(b"msez-lawpack-v1\0")
    for relpath in sorted(canonical_files.keys()):
        h.update(relpath.encode("utf-8"))
        h.update(b"\0")
        h.update(canonical_files[relpath])
        h.update(b"\0")
    return h.hexdigest()


def _infer_jurisdiction_and_domain(module_dir: pathlib.Path, sources_manifest: Dict[str, Any]) -> Tuple[str, str]:
    jid = str((sources_manifest or {}).get("jurisdiction_id") or "").strip()
    domain = str((sources_manifest or {}).get("domain") or "").strip()
    if jid and domain:
        return (jid, domain)

    # Derive from modules/legal/jurisdictions/<jid path>/<domain>
    parts = list(module_dir.parts)
    try:
        i = parts.index("jurisdictions")
    except ValueError:
        i = -1
    if i >= 0:
        if not domain:
            domain = parts[-1]
        if not jid and len(parts) >= i + 2:
            jid = "-".join(parts[i + 1 : -1])
    return (jid or "unknown", domain or "unknown")


def _fetch_source(uri: str, out_path: pathlib.Path) -> Tuple[str, str]:
    """Fetch a source URI into out_path and return (sha256, media_type)."""
    out_path.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(uri) as resp:
        data = resp.read()
        ct = resp.headers.get("Content-Type") or ""
    out_path.write_bytes(data)
    media_type = ct.split(";", 1)[0].strip() if ct else (mimetypes.guess_type(str(out_path))[0] or "application/octet-stream")
    return (sha256_bytes(data), media_type)


def normalize_relpath_for_lock(path: pathlib.Path, repo_root: pathlib.Path) -> str:
    try:
        return path.resolve().relative_to(repo_root.resolve()).as_posix()
    except Exception:
        return path.as_posix()


def parse_lawpack_ref(s: str) -> Dict[str, str]:
    """Parse a compact lawpack ref string: <jurisdiction_id>:<domain>:<sha256>"""
    parts = [p.strip() for p in s.split(":") if p.strip()]
    if len(parts) != 3:
        raise ValueError("lawpack ref must be '<jurisdiction_id>:<domain>:<sha256>'")
    jid, domain, digest = parts
    if not SHA256_RE.match(digest):
        raise ValueError("lawpack digest must be 64 lowercase hex chars")
    return {"jurisdiction_id": jid, "domain": domain, "lawpack_digest_sha256": digest}


def ingest_lawpack(
    module_dir: pathlib.Path,
    out_dir: pathlib.Path,
    as_of_date: str,
    *,
    repo_root: pathlib.Path,
    fetch: bool = False,
    include_raw: bool = False,
    tool_version: str = "",
    strict: bool = False,
    skip_existing: bool = False,
) -> Dict[str, Any]:
    """Ingest a jurisdiction corpus module into a content-addressed lawpack.zip.

    Inputs (by convention):
    - module.yaml (for license + description)
    - sources.yaml (for sources list + jurisdiction_id/domain)
    - src/akn/**/*.xml (normalized Akoma Ntoso documents)

    Outputs:
    - dist/lawpacks/<jurisdiction_id>/<domain>/<digest>.lawpack.zip
    - <module_dir>/lawpack.lock.json

    Returns the lock object.
    """
    module_dir = module_dir.resolve()
    repo_root = repo_root.resolve()
    out_dir = out_dir.resolve()

    # CI / reproducibility guardrails.
    if strict and fetch:
        raise ValueError("strict mode cannot be combined with fetch=True (non-deterministic retrieved_at); run fetch separately or disable strict")

    as_of_date = str(as_of_date or "").strip()
    if not as_of_date:
        raise ValueError("as_of_date is required (YYYY-MM-DD)")
    if strict:
        try:
            datetime.strptime(as_of_date, "%Y-%m-%d")
        except Exception as ex:
            raise ValueError(f"as_of_date must be YYYY-MM-DD (got {as_of_date!r})") from ex

    module_manifest_path = module_dir / "module.yaml"
    sources_path = module_dir / "sources.yaml"
    akn_dir = module_dir / "src" / "akn"

    if not module_manifest_path.exists():
        raise FileNotFoundError(f"Missing module.yaml in {module_dir}")
    module_manifest = _load_yaml_manifest(module_manifest_path, strict=strict, context="module.yaml")

    sources_manifest: Dict[str, Any] = {}
    if sources_path.exists():
        sources_manifest = _load_yaml_manifest(sources_path, strict=strict, context="sources.yaml")

    jurisdiction_id, domain = _infer_jurisdiction_and_domain(module_dir, sources_manifest)

    # Deterministic manifest digests (for provenance + CI verification).
    # NOTE: This is computed from the *parsed* YAML structure (JSON-compatible),
    # then canonicalized via JCS (RFC 8785). Comments/formatting do not affect it.
    sources_manifest_sha256 = sha256_bytes(jcs_canonicalize(sources_manifest)) if sources_path.exists() else ""
    sources_sha256 = sources_manifest_sha256 if sources_path.exists() else sha256_bytes(b"{}")
    module_manifest_sha256 = sha256_bytes(jcs_canonicalize(module_manifest))

    # Gather and (optionally) fetch raw sources
    sources: List[Dict[str, Any]] = []
    raw_digests: Dict[str, str] = {}
    if isinstance(sources_manifest.get("sources"), list):
        for s in sources_manifest.get("sources") or []:
            if not isinstance(s, dict):
                continue
            src = dict(s)
            src_id = str(src.get("source_id") or src.get("id") or "").strip() or "source"
            uri = str(src.get("uri") or src.get("url") or "").strip()
            if not uri:
                uri = str(src.get("reference") or "").strip()
            src["source_id"] = src_id
            if uri:
                src["uri"] = uri

            if fetch and uri and (uri.startswith("http://") or uri.startswith("https://")):
                raw_dir = module_dir / "src" / "raw"
                ext = pathlib.Path(uri.split("?", 1)[0]).suffix
                raw_path = raw_dir / f"{src_id}{ext or ''}"
                digest, media_type = _fetch_source(uri, raw_path)
                raw_digests[src_id] = digest
                src.setdefault("retrieved_at", now_rfc3339())
                src.setdefault("sha256", digest)
                src.setdefault("media_type", media_type)

            sources.append(src)

    # Read normalized Akoma Ntoso docs (required for v0.4.1 reference implementation)
    xml_files = find_akn_xml_files(akn_dir)
    if not xml_files:
        raise FileNotFoundError(f"No Akoma Ntoso XML found under {akn_dir}. Expected src/akn/**/*.xml")

    index_obj: Dict[str, Any] = {
        "index_version": "1",
        "jurisdiction_id": jurisdiction_id,
        "domain": domain,
        "documents": {},
    }

    akn_sha_by_rel: Dict[str, str] = {}
    canonical_files: Dict[str, bytes] = {}

    # Strict CI check mode: if a lock already exists and skip_existing is enabled,
    # prefer the tool_version recorded in the existing lock when computing digests.
    #
    # Rationale: the digest definition includes `lawpack.yaml`, and `lawpack.yaml` includes
    # the normalization.tool_version string. Without this override, `--check` would start
    # failing every time the CLI tool_version string advances, even if the underlying
    # lawpack content has not changed.
    lock_path = module_dir / "lawpack.lock.json"
    locked_tool_version: str = ""
    if strict and skip_existing and lock_path.exists():
        try:
            existing_lock = json.loads(lock_path.read_text(encoding="utf-8"))
            if isinstance(existing_lock, dict):
                norm = (existing_lock.get("provenance") or {}).get("normalization") or {}
                locked_tool_version = str(norm.get("tool_version") or "").strip()
        except Exception:
            locked_tool_version = ""
    effective_tool_version = locked_tool_version or (tool_version or "")

    # lawpack.yaml content (as an object; written later)
    lawpack_obj: Dict[str, Any] = {
        "lawpack_format_version": "1",
        "jurisdiction_id": jurisdiction_id,
        "domain": domain,
        "as_of_date": as_of_date,
        "sources": sources,
        "license": str(module_manifest.get("license") or sources_manifest.get("license") or "NOASSERTION"),
        "normalization": {
            "recipe_id": str((sources_manifest.get("normalization") or {}).get("recipe_id") or "msez.law.normalization.v1"),
            "tool": "msez",
            "tool_version": effective_tool_version or "unknown",
            "inputs": [
                {
                    "module_id": str(module_manifest.get("module_id") or ""),
                    "module_version": str(module_manifest.get("version") or ""),
                    "sources_manifest_sha256": sources_manifest_sha256,
                }
            ],
            "notes": str((sources_manifest.get("normalization") or {}).get("notes") or ""),
        },
    }

    canonical_files["lawpack.yaml"] = jcs_canonicalize(lawpack_obj)

    # Canonicalize AKN docs and index them
    for xf in xml_files:
        pack_path = "akn/" + xf.relative_to(akn_dir).as_posix()
        c14n = canonicalize_xml(xf)
        canonical_files[pack_path] = c14n
        akn_sha_by_rel[pack_path] = sha256_bytes(c14n)
        index_obj["documents"][pack_path] = build_eid_index(xf)

    canonical_files["index.json"] = jcs_canonicalize(index_obj)

    lawpack_digest = compute_lawpack_digest(canonical_files)

    # Prepare output paths (no I/O yet; strict+skip_existing should be side-effect free).
    out_pack_dir = out_dir / jurisdiction_id / domain
    artifact_path = out_pack_dir / f"{lawpack_digest}.lawpack.zip"
    artifact_rel = normalize_relpath_for_lock(artifact_path, repo_root)
    lock_path = module_dir / "lawpack.lock.json"

    expected_lawpack_yaml_sha256 = sha256_bytes(canonical_files["lawpack.yaml"])
    expected_index_json_sha256 = sha256_bytes(canonical_files["index.json"])

    # Fast/strict path: validate and reuse existing outputs.
    if skip_existing:
        if not lock_path.exists():
            if strict:
                raise FileNotFoundError(
                    f"strict+skip_existing requested but {lock_path} does not exist; run without --skip-existing to generate it"
                )
        else:
            try:
                existing_lock = json.loads(lock_path.read_text(encoding="utf-8"))
            except Exception as ex:
                if strict:
                    raise ValueError(f"Unable to parse existing lock at {lock_path}: {ex}") from ex
                existing_lock = {}

            if isinstance(existing_lock, dict) and existing_lock:
                # 1) Digest + artifact binding.
                digest_ok = str(existing_lock.get("lawpack_digest_sha256") or "").strip().lower() == lawpack_digest
                artifact_rel_ok = str(existing_lock.get("artifact_path") or "").strip() == artifact_rel

                # 2) Resolve on-disk artifact path from lock (do not assume out_dir).
                existing_art_rel = str(existing_lock.get("artifact_path") or "").strip()
                existing_art_path = pathlib.Path(existing_art_rel) if existing_art_rel else pathlib.Path()
                if existing_art_rel and not existing_art_path.is_absolute():
                    existing_art_path = repo_root / existing_art_path
                artifact_exists = bool(existing_art_rel) and existing_art_path.exists()

                # 3) Artifact sha256 and strict digest recomputation.
                sha_ok = False
                zip_digest_ok = False
                raw_ok = True
                if digest_ok and artifact_rel_ok and artifact_exists:
                    actual_artifact_sha = sha256_bytes(existing_art_path.read_bytes())
                    expected_artifact_sha = str(existing_lock.get("artifact_sha256") or "").strip().lower()
                    sha_ok = (expected_artifact_sha == actual_artifact_sha) if expected_artifact_sha else True

                    try:
                        zip_digest_ok = _compute_lawpack_digest_from_zip(existing_art_path) == lawpack_digest
                    except Exception as ex:
                        zip_digest_ok = False
                        if strict:
                            raise ValueError(
                                f"Strict digest recomputation failed for {existing_art_path}: {ex}"
                            ) from ex

                    # Raw inclusion expectation (portable audit packets often include raw).
                    try:
                        with zipfile.ZipFile(existing_art_path, "r") as zf:
                            names = set(zf.namelist())
                            has_raw = any(n.startswith("raw/") for n in names)
                            if include_raw:
                                raw_dir = module_dir / "src" / "raw"
                                if raw_dir.exists():
                                    for rf in sorted(raw_dir.rglob("*")):
                                        if rf.is_file():
                                            rel = rf.relative_to(raw_dir).as_posix()
                                            zname = f"raw/{rel}"
                                            if zname not in names:
                                                raw_ok = False
                                                break
                                            if zf.read(zname) != rf.read_bytes():
                                                raw_ok = False
                                                break
                            else:
                                if has_raw:
                                    raw_ok = False
                    except Exception as ex:
                        raw_ok = False
                        if strict:
                            raise ValueError(f"Unable to inspect raw/ in {existing_art_path}: {ex}") from ex

                # 4) Component digests: lock must match recomputed canonical values.
                comp_ok = True
                comps = existing_lock.get("components")
                if not isinstance(comps, dict):
                    comp_ok = False
                else:
                    if str(comps.get("lawpack_yaml_sha256") or "").strip().lower() != expected_lawpack_yaml_sha256:
                        comp_ok = False
                    if str(comps.get("index_json_sha256") or "").strip().lower() != expected_index_json_sha256:
                        comp_ok = False
                    # sources_sha256 is required by schema; tolerate empty only for legacy locks.
                    sources_lock = str(comps.get("sources_sha256") or "").strip().lower()
                    if sources_lock and sources_lock != sources_sha256:
                        comp_ok = False
                    akn_lock = comps.get("akn_sha256")
                    if not isinstance(akn_lock, dict):
                        comp_ok = False
                    else:
                        for k, v in akn_sha_by_rel.items():
                            if str(akn_lock.get(k) or "").strip().lower() != v:
                                comp_ok = False
                                break
                    # Optional: module manifest digest (newer locks only).
                    mm_lock = str(comps.get("module_manifest_sha256") or "").strip().lower()
                    if mm_lock and mm_lock != module_manifest_sha256:
                        comp_ok = False

                if digest_ok and artifact_rel_ok and artifact_exists and sha_ok and zip_digest_ok and raw_ok and comp_ok:
                    return existing_lock

                if strict:
                    raise ValueError(
                        "strict+skip_existing verification failed; "
                        f"digest_ok={digest_ok}, artifact_rel_ok={artifact_rel_ok}, artifact_exists={artifact_exists}, "
                        f"sha_ok={sha_ok}, zip_digest_ok={zip_digest_ok}, raw_ok={raw_ok}, components_ok={comp_ok}"
                    )

    # Ensure output directory exists only when we are about to write.
    out_pack_dir.mkdir(parents=True, exist_ok=True)

    # Build zip in a temp dir to ensure consistent structure.
    with tempfile.TemporaryDirectory() as td:
        tdir = pathlib.Path(td)

        # Emit canonical metadata files as deterministic JSON (JSON is a YAML subset).
        #
        # Rationale: YAML emitters may produce slightly different byte sequences across
        # platforms / library versions, which breaks strict reproducibility of the
        # lawpack.zip bytes (even when the logical content is identical). By writing
        # RFC 8785 (JCS) canonical JSON bytes, we guarantee deterministic artifacts.
        (tdir / "lawpack.yaml").write_bytes(canonical_files["lawpack.yaml"])
        (tdir / "index.json").write_bytes(canonical_files["index.json"])
        (tdir / "digest.sha256").write_text(lawpack_digest + "\n", encoding="utf-8")

        # Emit canonicalized Akoma Ntoso bytes.
        #
        # The index hashes and offsets are computed over the canonical XML C14N bytes,
        # so the artifact must store the same representation to make the index directly
        # usable by verifiers and operators.
        for xf in xml_files:
            rel = xf.relative_to(akn_dir).as_posix()
            pack_path = f"akn/{rel}"
            dest = tdir / pack_path
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(canonical_files[pack_path])

        if include_raw:
            raw_dir = module_dir / "src" / "raw"
            if raw_dir.exists():
                for rf in sorted(raw_dir.rglob("*")):
                    if rf.is_file():
                        rel = rf.relative_to(raw_dir)
                        dest = tdir / "raw" / rel
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        dest.write_bytes(rf.read_bytes())

        # Deterministic zip emission:
        # - stable file order
        # - fixed timestamps
        # - fixed permissions
        fixed_dt = (1980, 1, 1, 0, 0, 0)

        fd, tmp_name = tempfile.mkstemp(prefix=f"{lawpack_digest}.", suffix=".lawpack.zip.tmp", dir=str(out_pack_dir))
        os.close(fd)
        tmp_artifact_path = pathlib.Path(tmp_name)
        tmp_path_live = tmp_artifact_path
        try:
            with zipfile.ZipFile(tmp_artifact_path, "w") as zf:
                for fp in sorted(tdir.rglob("*")):
                    if fp.is_dir():
                        continue
                    arc = fp.relative_to(tdir).as_posix()
                    zi = zipfile.ZipInfo(arc, date_time=fixed_dt)
                    zi.compress_type = zipfile.ZIP_DEFLATED
                    zi.create_system = 0
                    zi.external_attr = (0o644 << 16)
                    zf.writestr(zi, fp.read_bytes())
            tmp_artifact_path.replace(artifact_path)
            tmp_path_live = None
        finally:
            if tmp_path_live is not None and tmp_path_live.exists():
                tmp_path_live.unlink()

    artifact_sha256 = sha256_bytes(artifact_path.read_bytes())

    lock_obj: Dict[str, Any] = {
        "lawpack_digest_sha256": lawpack_digest,
        "jurisdiction_id": jurisdiction_id,
        "domain": domain,
        "as_of_date": as_of_date,
        "artifact_path": artifact_rel,
        "artifact_sha256": artifact_sha256,
        "components": {
            "lawpack_yaml_sha256": expected_lawpack_yaml_sha256,
            "index_json_sha256": expected_index_json_sha256,
            "akn_sha256": akn_sha_by_rel,
            "sources_sha256": sources_sha256,
            "module_manifest_sha256": module_manifest_sha256,
        },
        "provenance": {
            "module_manifest_path": "module.yaml",
            "sources_manifest_path": "sources.yaml" if sources_path.exists() else "",
            "raw_sources": raw_digests,
            "normalization": lawpack_obj.get("normalization"),
        },
    }

    # Atomic write to avoid partially-written locks in interrupted runs.
    tmp_lock_path = lock_path.with_name(lock_path.name + ".tmp")
    tmp_lock_path.write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    tmp_lock_path.replace(lock_path)

    return lock_obj
