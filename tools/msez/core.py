"""Core primitives for MSEZ Stack.

This module provides the foundational utilities used throughout the stack:
- Cryptographic hashing (SHA-256)
- Canonical JSON serialization (JCS/RFC8785)
- YAML/JSON loading with consistent encoding
- Path resolution relative to repository root

Design principles:
- Pure functions where possible
- No global mutable state
- Explicit error handling
- Type annotations throughout
"""

from __future__ import annotations

import hashlib
import json
import pathlib
import re
from typing import Any, Dict, List, Optional, Tuple, Union

import yaml

# Repository root, computed once at module load
REPO_ROOT = pathlib.Path(__file__).resolve().parents[2]


def sha256_bytes(data: bytes) -> str:
    """Compute SHA-256 hash of bytes, returning lowercase hex string."""
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: pathlib.Path) -> str:
    """Compute SHA-256 hash of file contents."""
    return sha256_bytes(pathlib.Path(path).read_bytes())


def load_yaml(path: pathlib.Path) -> Any:
    """Load YAML file with UTF-8 encoding."""
    return yaml.safe_load(pathlib.Path(path).read_text(encoding="utf-8"))


def load_json(path: pathlib.Path) -> Any:
    """Load JSON file with UTF-8 encoding."""
    return json.loads(pathlib.Path(path).read_text(encoding="utf-8"))


def canonical_json_bytes(obj: Any) -> bytes:
    """Serialize object to canonical JSON bytes (JCS/RFC8785 subset).

    Properties:
    - Keys sorted lexicographically
    - No whitespace
    - UTF-8 encoded
    - Floats rejected (use strings/ints for amounts)

    This ensures byte-for-byte reproducibility for cryptographic commitments.
    """
    def _reject_floats(o: Any, path: str = "") -> None:
        if isinstance(o, float):
            raise ValueError(f"Float not allowed in canonical JSON at {path}")
        if isinstance(o, dict):
            for k, v in o.items():
                _reject_floats(v, f"{path}.{k}")
        if isinstance(o, list):
            for i, v in enumerate(o):
                _reject_floats(v, f"{path}[{i}]")

    _reject_floats(obj)
    return json.dumps(
        obj,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
    ).encode("utf-8")


def write_canonical_json(path: pathlib.Path, obj: Any) -> str:
    """Write canonical JSON to file, returning the digest.

    Appends a trailing newline for POSIX compatibility.
    Returns the SHA-256 digest of the canonical bytes (without newline).
    """
    canonical = canonical_json_bytes(obj)
    pathlib.Path(path).write_bytes(canonical + b"\n")
    return sha256_bytes(canonical)


def resolve_path(
    relative: str,
    base: Optional[pathlib.Path] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> pathlib.Path:
    """Resolve a path relative to base or repo root.

    Resolution order:
    1. If absolute, return as-is
    2. If base provided and path exists relative to base, use that
    3. Fall back to repo root
    """
    p = pathlib.Path(relative)
    if p.is_absolute():
        return p

    if base is not None:
        candidate = pathlib.Path(base) / p
        if candidate.exists():
            return candidate

    return repo_root / p


def coerce_sha256(value: Any) -> str:
    """Extract SHA-256 digest from string or ArtifactRef dict.

    Handles both:
    - Raw hex string: "abc123..."
    - ArtifactRef object: {"digest_sha256": "abc123...", ...}

    Returns empty string if no valid digest found.
    """
    if isinstance(value, dict):
        return str(value.get("digest_sha256") or "").strip().lower()
    if isinstance(value, str):
        return value.strip().lower()
    return ""


def is_valid_sha256(digest: str) -> bool:
    """Check if string is a valid SHA-256 hex digest."""
    return bool(re.fullmatch(r"[a-f0-9]{64}", digest or ""))


def make_artifact_ref(
    artifact_type: str,
    digest_sha256: str,
    *,
    uri: str = "",
    display_name: str = "",
    media_type: str = "",
    byte_length: Optional[int] = None,
) -> Dict[str, Any]:
    """Construct an ArtifactRef object.

    ArtifactRefs are the universal typed digest commitment substrate.
    This helper ensures consistent structure across the codebase.
    """
    ref: Dict[str, Any] = {
        "artifact_type": artifact_type.strip(),
        "digest_sha256": digest_sha256.strip().lower(),
    }

    if uri:
        ref["uri"] = uri.strip()
    if display_name:
        ref["display_name"] = display_name.strip()
    if media_type:
        ref["media_type"] = media_type.strip()
    if byte_length is not None:
        ref["byte_length"] = int(byte_length)

    return ref


def parse_duration_seconds(duration: str) -> int:
    """Parse duration string to seconds.

    Supported formats:
    - Shorthand: "30s", "15m", "2h", "7d"
    - ISO8601 subset: "PT1H", "PT30M", "P1D"
    - Plain integer (seconds)

    Returns 0 for empty or unparseable input.
    """
    s = str(duration or "").strip()
    if not s:
        return 0

    # Plain integer
    if re.fullmatch(r"\d+", s):
        return int(s)

    # Shorthand: 30s, 15m, 2h, 7d
    m = re.fullmatch(r"(?i)(\d+)\s*([smhd])", s)
    if m:
        n, unit = int(m.group(1)), m.group(2).lower()
        return n * {"s": 1, "m": 60, "h": 3600, "d": 86400}[unit]

    # ISO8601 PnD
    m = re.fullmatch(r"(?i)P(\d+)D", s)
    if m:
        return int(m.group(1)) * 86400

    # ISO8601 PTnHnMnS
    m = re.fullmatch(r"(?i)PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?", s)
    if m:
        h = int(m.group(1) or 0)
        mi = int(m.group(2) or 0)
        sec = int(m.group(3) or 0)
        return h * 3600 + mi * 60 + sec

    return 0


# Timestamp utilities
def now_iso8601() -> str:
    """Return current UTC time in ISO8601 format."""
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def parse_iso8601(timestamp: str) -> Optional["datetime"]:
    """Parse ISO8601 timestamp string."""
    from datetime import datetime
    try:
        # Handle Z suffix
        s = timestamp.replace("Z", "+00:00")
        return datetime.fromisoformat(s)
    except (ValueError, AttributeError):
        return None
