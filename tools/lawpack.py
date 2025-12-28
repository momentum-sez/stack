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

    module_manifest_path = module_dir / "module.yaml"
    sources_path = module_dir / "sources.yaml"
    akn_dir = module_dir / "src" / "akn"

    if not module_manifest_path.exists():
        raise FileNotFoundError(f"Missing module.yaml in {module_dir}")
    module_manifest = yaml.safe_load(module_manifest_path.read_text(encoding="utf-8")) or {}

    sources_manifest: Dict[str, Any] = {}
    if sources_path.exists():
        sources_manifest = yaml.safe_load(sources_path.read_text(encoding="utf-8")) or {}

    jurisdiction_id, domain = _infer_jurisdiction_and_domain(module_dir, sources_manifest)

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
            "tool_version": tool_version or "unknown",
            "inputs": [
                {
                    "module_id": str(module_manifest.get("module_id") or ""),
                    "module_version": str(module_manifest.get("version") or ""),
                    "sources_manifest_sha256": sha256_bytes(canonicalize_yaml(sources_path)) if sources_path.exists() else "",
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

    # Prepare output paths
    out_pack_dir = out_dir / jurisdiction_id / domain
    out_pack_dir.mkdir(parents=True, exist_ok=True)
    artifact_path = out_pack_dir / f"{lawpack_digest}.lawpack.zip"

    # Build zip in a temp dir to ensure consistent structure
    with tempfile.TemporaryDirectory() as td:
        tdir = pathlib.Path(td)

        (tdir / "lawpack.yaml").write_text(yaml.safe_dump(lawpack_obj, sort_keys=False), encoding="utf-8")
        (tdir / "index.json").write_text(json.dumps(index_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        (tdir / "digest.sha256").write_text(lawpack_digest + "\n", encoding="utf-8")

        akn_out = tdir / "akn"
        for xf in xml_files:
            rel = xf.relative_to(akn_dir)
            dest = akn_out / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(xf.read_bytes())

        if include_raw:
            raw_dir = module_dir / "src" / "raw"
            if raw_dir.exists():
                for rf in sorted(raw_dir.rglob("*")):
                    if rf.is_file():
                        rel = rf.relative_to(raw_dir)
                        dest = tdir / "raw" / rel
                        dest.parent.mkdir(parents=True, exist_ok=True)
                        dest.write_bytes(rf.read_bytes())

        with zipfile.ZipFile(artifact_path, "w") as zf:
            # Deterministic zip emission:
            # - stable file order
            # - fixed timestamps
            # - fixed permissions
            fixed_dt = (1980, 1, 1, 0, 0, 0)
            for fp in sorted(tdir.rglob("*")):
                if fp.is_dir():
                    continue
                arc = fp.relative_to(tdir).as_posix()
                zi = zipfile.ZipInfo(arc, date_time=fixed_dt)
                zi.compress_type = zipfile.ZIP_DEFLATED
                zi.create_system = 0
                zi.external_attr = (0o644 << 16)
                zf.writestr(zi, fp.read_bytes())

    artifact_sha256 = sha256_bytes(artifact_path.read_bytes())
    artifact_rel = normalize_relpath_for_lock(artifact_path, repo_root)

    lock_obj: Dict[str, Any] = {
        "lawpack_digest_sha256": lawpack_digest,        "jurisdiction_id": jurisdiction_id,
        "domain": domain,
        "as_of_date": as_of_date,
        "artifact_path": artifact_rel,
        "artifact_sha256": artifact_sha256,
        "components": {
            "lawpack_yaml_sha256": sha256_bytes(canonical_files["lawpack.yaml"]),
            "index_json_sha256": sha256_bytes(canonical_files["index.json"]),
            "akn_sha256": akn_sha_by_rel,
            "sources_sha256": sha256_bytes(canonicalize_yaml(sources_path)) if sources_path.exists() else sha256_bytes(b"{}"),
        },
        "provenance": {
            "module_manifest_path": "module.yaml",
            "sources_manifest_path": "sources.yaml" if sources_path.exists() else "",
            "raw_sources": raw_digests,
            "normalization": lawpack_obj.get("normalization"),
        },
    }

    (module_dir / "lawpack.lock.json").write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return lock_obj
