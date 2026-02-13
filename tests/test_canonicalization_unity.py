"""
Verify that ALL digest-computing paths in the stack use jcs_canonicalize.

This test exists because the Feb 2026 audit discovered that the phoenix layer
used json.dumps(sort_keys=True) while the core layer used jcs_canonicalize(),
producing different digests for identical data. This must never regress.
"""
import ast
import pathlib

PHOENIX_DIR = pathlib.Path("tools/phoenix")
ALLOWED_MODULES = {"cli.py", "config.py"}  # Non-digest uses are acceptable here


def test_no_json_dumps_in_digest_paths():
    """Ensure phoenix modules use jcs_canonicalize for all digest computation."""
    violations = []
    for py_file in sorted(PHOENIX_DIR.glob("*.py")):
        if py_file.name in ALLOWED_MODULES:
            continue
        source = py_file.read_text()
        # Look for json.dumps used directly in digest computation
        lines = source.split("\n")
        for i, line in enumerate(lines):
            if "json.dumps" in line and "sort_keys" in line:
                # Skip lines that are in display/serialization methods (to_json, to_str)
                func_context = "\n".join(lines[max(0, i - 5):i + 1])
                if "def to_json" in func_context or "def __str__" in func_context:
                    continue
                # Check if this json.dumps feeds into a digest computation
                context = "\n".join(lines[max(0, i - 2):min(len(lines), i + 5)])
                if any(kw in context for kw in ["sha256", "hashlib", "digest", "canonical", "commitment"]):
                    violations.append(f"{py_file.name}:{i+1}: {line.strip()}")
    assert not violations, (
        f"Found json.dumps used for digest computation instead of jcs_canonicalize:\n"
        + "\n".join(violations)
    )


def test_no_json_dumps_encode_sha256_pattern():
    """Detect the specific anti-pattern: json.dumps(...).encode() followed by sha256."""
    violations = []
    for py_file in sorted(PHOENIX_DIR.glob("*.py")):
        if py_file.name in ALLOWED_MODULES:
            continue
        source = py_file.read_text()
        lines = source.split("\n")
        for i, line in enumerate(lines):
            stripped = line.strip()
            # Direct pattern: json.dumps(...).encode() in a sha256 call
            if "json.dumps" in stripped and ".encode()" in stripped and "sha256" in stripped:
                violations.append(f"{py_file.name}:{i+1}: {stripped}")
    assert not violations, (
        f"Found json.dumps().encode() used directly in sha256 computation:\n"
        + "\n".join(violations)
    )


def test_jcs_canonicalize_returns_bytes():
    """Verify jcs_canonicalize returns bytes, not str."""
    from tools.lawpack import jcs_canonicalize
    result = jcs_canonicalize({"b": 2, "a": 1})
    assert isinstance(result, bytes), f"Expected bytes, got {type(result)}"
    # Verify sort order
    assert result == b'{"a":1,"b":2}'


def test_jcs_canonicalize_rejects_floats():
    """Verify floats are rejected per the canonicalization spec."""
    from tools.lawpack import jcs_canonicalize
    import pytest
    with pytest.raises(ValueError, match="[Ff]loat"):
        jcs_canonicalize({"price": 19.99})


def test_jcs_canonicalize_coerces_datetime():
    """Verify datetime objects are coerced to UTC ISO8601 with Z suffix."""
    from tools.lawpack import jcs_canonicalize
    from datetime import datetime, timezone
    import json

    dt = datetime(2026, 1, 15, 12, 0, 0, tzinfo=timezone.utc)
    result = jcs_canonicalize({"ts": dt})
    parsed = json.loads(result)
    assert parsed["ts"] == "2026-01-15T12:00:00Z"
