"""Entry point for `python -m tools.msez`.

Delegates to the monolith CLI in tools/msez.py. This bridge ensures
`python -m tools.msez validate --all-modules` works from CI and the
command line.
"""
from __future__ import annotations

import importlib.util
import sys


def _main() -> int:
    # tools/msez.py coexists with tools/msez/ (this package).
    # Python resolves `tools.msez` to the package, so we need to
    # explicitly load the .py file via its file path.
    import pathlib

    msez_py = pathlib.Path(__file__).resolve().parent.parent / "msez.py"
    spec = importlib.util.spec_from_file_location("tools._msez_cli", str(msez_py))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod.main()


if __name__ == "__main__":
    sys.exit(_main())
