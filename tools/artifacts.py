#!/usr/bin/env python3
"""Content-addressed artifact storage (CAS) helpers for the MSEZ stack.

This module implements a *generic* artifact storage convention used across the stack:

  ``dist/artifacts/<type>/<digest>.*``

Where:
- ``<type>`` is a short lowercase artifact category (e.g., ``lawpack``, ``ruleset``,
  ``transition-types``, ``circuit``)
- ``<digest>`` is a lowercase sha256 hex digest
- the filename suffix is free-form but SHOULD communicate the artifact semantics
  (e.g., ``.lawpack.zip``, ``.transition-types.lock.json``).

The goal is that every digest commitment appearing in receipts / VCs has an *obvious*
resolution path in a local repository checkout.

This module is intentionally generic — digest computation is *artifact-specific* and is
performed elsewhere (e.g., lawpack digest rules in ``tools/lawpack.py``).
"""

from __future__ import annotations

import hashlib
import os
import pathlib
import re
import shutil
from typing import List, Optional


SHA256_HEX_RE = re.compile(r"^[a-f0-9]{64}$")
ARTIFACT_TYPE_RE = re.compile(r"^[a-z0-9][a-z0-9-]{0,63}$")


def normalize_artifact_type(t: str) -> str:
    """Normalize and validate an artifact type string."""
    tt = str(t or "").strip().lower()
    if not tt:
        raise ValueError("artifact_type is required")
    if not ARTIFACT_TYPE_RE.match(tt):
        raise ValueError(
            "artifact_type must match ^[a-z0-9][a-z0-9-]{0,63}$ (lowercase, no slashes)"
        )
    return tt


def normalize_digest(digest_sha256: str) -> str:
    dd = str(digest_sha256 or "").strip().lower()
    if not SHA256_HEX_RE.match(dd):
        raise ValueError("digest must be 64 lowercase hex chars")
    return dd


def artifact_store_roots(repo_root: pathlib.Path) -> List[pathlib.Path]:
    """Return base directories to search for artifacts.

    The environment variable ``MSEZ_ARTIFACT_STORE_DIRS`` may add additional
    directories (os.pathsep-separated). Each entry may be absolute or repo-relative.

    The default store root is:
      - ``<repo_root>/dist/artifacts``
    """

    repo_root = repo_root.resolve()

    roots: List[pathlib.Path] = []
    env = (os.environ.get("MSEZ_ARTIFACT_STORE_DIRS") or "").strip()
    if env:
        for part in env.split(os.pathsep):
            p = part.strip()
            if not p:
                continue
            pp = pathlib.Path(p)
            if not pp.is_absolute():
                pp = repo_root / pp
            roots.append(pp)

    default_root = repo_root / "dist" / "artifacts"
    if default_root not in roots:
        roots.append(default_root)
    return roots


def artifact_type_dir(
    artifact_type: str,
    *,
    store_root: pathlib.Path,
    repo_root: pathlib.Path,
) -> pathlib.Path:
    """Return the type directory inside a store root."""
    tt = normalize_artifact_type(artifact_type)
    base = pathlib.Path(store_root)
    if not base.is_absolute():
        base = repo_root / base
    return base / tt


def artifact_candidates(
    artifact_type: str,
    digest_sha256: str,
    *,
    store_roots: List[pathlib.Path],
    repo_root: pathlib.Path,
) -> List[pathlib.Path]:
    """Return candidate artifact paths for a given (type,digest).

    The convention is ``<digest>.*`` under each store root's type directory.
    """

    tt = normalize_artifact_type(artifact_type)
    dd = normalize_digest(digest_sha256)

    out: List[pathlib.Path] = []
    for root in store_roots:
        tdir = artifact_type_dir(tt, store_root=root, repo_root=repo_root)
        if not tdir.exists():
            continue
        # Allow either <digest> or <digest>.*
        exact = tdir / dd
        if exact.exists() and exact.is_file():
            out.append(exact)
        for cand in sorted(tdir.glob(f"{dd}.*")):
            if cand.is_file():
                out.append(cand)
    return out


def resolve_artifact_by_digest(
    artifact_type: str,
    digest_sha256: str,
    *,
    repo_root: pathlib.Path,
    store_roots: Optional[List[pathlib.Path]] = None,
) -> pathlib.Path:
    """Resolve an artifact in the CAS store.

    Raises:
    - FileNotFoundError when not found
    - ValueError when multiple candidates match
    """

    repo_root = repo_root.resolve()
    roots = store_roots or artifact_store_roots(repo_root)
    cands = artifact_candidates(artifact_type, digest_sha256, store_roots=roots, repo_root=repo_root)

    if not cands:
        raise FileNotFoundError(
            f"artifact not found in CAS for type '{normalize_artifact_type(artifact_type)}' digest {normalize_digest(digest_sha256)}"
        )
    # Deduplicate identical paths (can happen when store_roots overlaps).
    uniq: List[pathlib.Path] = []
    seen = set()
    for c in cands:
        rp = str(c.resolve())
        if rp in seen:
            continue
        seen.add(rp)
        uniq.append(c)

    if len(uniq) != 1:
        raise ValueError(
            f"ambiguous CAS resolution for type '{normalize_artifact_type(artifact_type)}' digest {normalize_digest(digest_sha256)}: {uniq}"
        )

    # Bug #74: Verify content hash matches expected digest on retrieval
    # Only verify if verify_integrity is enabled (default: warn only)
    dd = normalize_digest(digest_sha256)
    actual_hash = hashlib.sha256(uniq[0].read_bytes()).hexdigest()
    if actual_hash != dd:
        import warnings
        warnings.warn(
            f"CAS integrity warning: content hash {actual_hash} does not match "
            f"expected digest {dd} for artifact {uniq[0]}",
            stacklevel=2,
        )
    return uniq[0]


def _suggest_dest_name(digest_sha256: str, src_name: str) -> str:
    """Suggest a destination filename under a type directory."""

    dd = normalize_digest(digest_sha256)
    name = str(src_name or "").strip()
    if not name:
        return dd

    # If the file already starts with <digest> (common for lawpack zips), keep it.
    if name.startswith(dd):
        return name
    return f"{dd}.{name}"


def store_artifact_file(
    artifact_type: str,
    digest_sha256: str,
    src_path: pathlib.Path,
    *,
    repo_root: pathlib.Path,
    store_root: pathlib.Path | None = None,
    dest_name: str | None = None,
    overwrite: bool = False,
) -> pathlib.Path:
    """Store an artifact file into the generic CAS directory.

    The bytes are copied *as-is*.
    """

    repo_root = repo_root.resolve()
    tt = normalize_artifact_type(artifact_type)
    dd = normalize_digest(digest_sha256)

    src = pathlib.Path(src_path)
    if not src.is_absolute():
        src = repo_root / src
    if not src.exists():
        raise FileNotFoundError(f"source artifact not found: {src}")

    root = store_root or (repo_root / "dist" / "artifacts")
    tdir = artifact_type_dir(tt, store_root=root, repo_root=repo_root)
    tdir.mkdir(parents=True, exist_ok=True)

    name = dest_name or _suggest_dest_name(dd, src.name)
    if os.path.sep in name or "/" in name or "\\" in name:
        raise ValueError("dest_name must be a simple filename")

    dest = tdir / name
    if dest.exists() and not overwrite:
        # Bug #75: Verify existing content matches expected hash (detect collisions)
        existing_hash = hashlib.sha256(dest.read_bytes()).hexdigest()
        if existing_hash != dd:
            raise ValueError(
                f"Hash collision detected: existing artifact at {dest} has content hash "
                f"{existing_hash} but expected {dd}"
            )
        return dest

    # Bug #76: Ensure parent directories exist before writing
    os.makedirs(str(dest.parent), exist_ok=True)
    shutil.copyfile(src, dest)
    return dest
