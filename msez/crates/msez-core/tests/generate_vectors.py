#!/usr/bin/env python3
"""Generate cross-language test vectors for canonical digest verification.

This script computes canonical bytes and SHA-256 digests using the Python
jcs_canonicalize() implementation from tools/lawpack.py. The results can be
compared against the Rust implementation in msez-core to verify byte-identical
behavior across the language boundary.

Usage:
    python3 tests/generate_vectors.py

The script must be run from the repository root (where tools/ is accessible).

See also: msez/crates/msez-core/tests/cross_language.rs
"""
from __future__ import annotations

import hashlib
import json
import os
import sys

# Ensure tools/ is importable
repo_root = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "..", "..")
sys.path.insert(0, os.path.join(repo_root, "tools"))

from lawpack import jcs_canonicalize  # noqa: E402


# Same test vectors as in cross_language.rs
TEST_VECTORS = [
    '{"b":2,"a":1,"c":"hello"}',
    '{"z":26,"a":1}',
    '{}',
    '[]',
    '{"nested":{"z":1,"a":2},"top":true}',
    '{"arr":[3,2,1],"key":"value"}',
    '{"n":null,"b":false,"t":true,"i":42,"s":"text"}',
    '{"big":999999999999}',
    '{"neg":-42}',
    '{"empty":""}',
]


def main() -> None:
    print("Cross-language canonicalization test vectors")
    print("=" * 60)
    print()

    all_pass = True
    for i, input_json in enumerate(TEST_VECTORS):
        data = json.loads(input_json)
        canonical = jcs_canonicalize(data)
        canonical_str = canonical.decode("utf-8")
        digest = hashlib.sha256(canonical).hexdigest()

        print(f"Vector {i}:")
        print(f"  Input:     {input_json}")
        print(f"  Canonical: {canonical_str}")
        print(f"  SHA-256:   {digest}")
        print()

    print("=" * 60)
    print(f"Generated {len(TEST_VECTORS)} test vectors.")
    print("Compare these against the Rust integration test output.")

    # Also verify float rejection
    print()
    print("Float rejection tests:")
    float_inputs = [
        '{"x":1.5}',
        '{"x":3.14}',
        '{"x":0.1}',
    ]
    for inp in float_inputs:
        data = json.loads(inp)
        try:
            jcs_canonicalize(data)
            print(f"  FAIL: {inp} was NOT rejected (expected ValueError)")
            all_pass = False
        except (ValueError, TypeError):
            print(f"  OK:   {inp} correctly rejected")

    if not all_pass:
        sys.exit(1)


if __name__ == "__main__":
    main()
