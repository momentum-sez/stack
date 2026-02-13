"""
Verify that mass_primitives.py uses the canonical jcs_canonicalize() from
tools.lawpack for ALL digest computation.

This test was added because the Feb 2026 audit discovered that
mass_primitives.py defined its own json_canonicalize() that accepted floats,
did not handle datetime objects, and did not coerce non-string dict keys —
producing different digests than the core layer's jcs_canonicalize().

This module must never regress.
"""

import hashlib
from datetime import datetime, timezone

import pytest

from tools.lawpack import jcs_canonicalize
from tools.mass_primitives import json_canonicalize, stack_digest


def test_mass_primitives_json_canonicalize_matches_jcs():
    """json_canonicalize() in mass_primitives must produce the same bytes as
    jcs_canonicalize() for identical input data."""
    data = {"b": 2, "a": 1, "c": "hello", "nested": {"z": True, "a": None}}
    core_bytes = jcs_canonicalize(data)
    mp_str = json_canonicalize(data)
    assert mp_str.encode("utf-8") == core_bytes, (
        f"Canonicalization mismatch:\n"
        f"  core:  {core_bytes!r}\n"
        f"  mass:  {mp_str.encode('utf-8')!r}"
    )


def test_mass_primitives_rejects_floats():
    """mass_primitives.json_canonicalize must reject floats, matching
    jcs_canonicalize behavior.  Floats are non-deterministic across
    implementations and must be represented as strings or integers."""
    with pytest.raises(ValueError, match="[Ff]loat"):
        json_canonicalize({"amount": 3.14})


def test_mass_primitives_handles_datetime():
    """mass_primitives.json_canonicalize must coerce datetime objects to UTC
    ISO8601 strings, matching jcs_canonicalize behavior."""
    dt = datetime(2026, 1, 15, 12, 0, 0, tzinfo=timezone.utc)
    data = {"ts": dt, "val": 42}
    core_bytes = jcs_canonicalize(data)
    mp_str = json_canonicalize(data)
    assert mp_str.encode("utf-8") == core_bytes
    assert "2026-01-15T12:00:00Z" in mp_str


def test_mass_primitives_coerces_non_string_keys():
    """mass_primitives.json_canonicalize must coerce non-string dict keys to
    strings, matching jcs_canonicalize behavior."""
    data = {1: "one", "b": 2}
    core_bytes = jcs_canonicalize(data)
    mp_str = json_canonicalize(data)
    assert mp_str.encode("utf-8") == core_bytes
    assert '"1"' in mp_str


def test_stack_digest_uses_canonical():
    """stack_digest() must produce the same digest as SHA256(jcs_canonicalize(data))."""
    data = {"asset_name": "TestAsset", "asset_class": "equity", "bindings": ["h1"]}
    expected = hashlib.sha256(jcs_canonicalize(data)).hexdigest()
    result = stack_digest(data)
    assert result.bytes_hex == expected, (
        f"stack_digest diverges from jcs_canonicalize:\n"
        f"  expected: {expected}\n"
        f"  got:      {result.bytes_hex}"
    )


def test_no_standalone_json_dumps_for_digests():
    """Scan mass_primitives.py for json.dumps used in digest computation
    contexts.  After the canonicalization fix, no digest path should bypass
    jcs_canonicalize."""
    import pathlib

    source = pathlib.Path("tools/mass_primitives.py").read_text()
    lines = source.split("\n")
    violations = []
    for i, line in enumerate(lines):
        if "json.dumps" in line and "sort_keys" in line:
            context = "\n".join(lines[max(0, i - 2):min(len(lines), i + 5)])
            if any(kw in context for kw in ["sha256", "hashlib", "digest", "canonical", "root"]):
                violations.append(f"mass_primitives.py:{i + 1}: {line.strip()}")
    assert not violations, (
        "Found json.dumps used for digest computation in mass_primitives.py:\n"
        + "\n".join(violations)
    )
