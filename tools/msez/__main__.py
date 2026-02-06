"""MSEZ CLI entry point — python -m tools.msez

Provides CLI access to MSEZ Stack functionality when invoked as a module.
Delegates to the legacy tools/msez.py for commands not implemented in the package.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


def _load_legacy_cli():
    """Load the legacy tools/msez.py module directly.

    Since both tools/msez.py and tools/msez/ exist, Python prefers the package.
    We use importlib to explicitly load the .py file.
    """
    # Path to the legacy CLI module
    tools_dir = Path(__file__).resolve().parent.parent
    legacy_path = tools_dir / "msez.py"

    if not legacy_path.exists():
        raise ImportError(f"Legacy CLI not found: {legacy_path}")

    # Load module with a distinct name to avoid conflicts
    spec = importlib.util.spec_from_file_location("_msez_legacy_cli", legacy_path)
    if spec is None or spec.loader is None:
        raise ImportError(f"Failed to create module spec for {legacy_path}")

    module = importlib.util.module_from_spec(spec)
    sys.modules["_msez_legacy_cli"] = module
    spec.loader.exec_module(module)

    return module


def main() -> int:
    """Main entry point — delegates to the legacy CLI."""
    legacy = _load_legacy_cli()
    return legacy.main()


if __name__ == "__main__":
    sys.exit(main())
