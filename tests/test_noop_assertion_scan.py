"""
Detect tests that contain no-op assertions (assert True, assert not False, etc).

These assertions provide zero verification value and may mask missing test
logic. This was identified in the Feb 2026 audit as a test quality concern.
"""
import pathlib

# Known no-op assertions that are intentional smoke tests (verify no crash).
# Adding to this list requires justification in the commit message.
KNOWN_EXCEPTIONS = {
    ("test_trade_corridors.py", 932),  # Smoke test: verify negative amount doesn't crash
}


def test_no_noop_assertions():
    """Detect tests that assert True/False without meaningful checks."""
    violations = []
    for f in sorted(pathlib.Path("tests").glob("test_*.py")):
        for i, line in enumerate(f.read_text().split("\n"), 1):
            stripped = line.strip()
            if stripped in ("assert True", "assert not False", "assert 1"):
                if (f.name, i) not in KNOWN_EXCEPTIONS:
                    violations.append(f"{f.name}:{i}: {stripped}")
    assert not violations, f"No-op assertions found:\n" + "\n".join(violations)
