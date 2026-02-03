"""MSEZ Stack Tool — v0.4.44 GENESIS

A modular, composable Special Economic Zone infrastructure toolkit.

Architecture:
    msez/
    ├── __init__.py      # Package entry, version, public API
    ├── core.py          # Primitives: sha256, JSON, YAML, paths
    ├── schema.py        # JSON Schema validation infrastructure
    ├── artifacts.py     # Content-addressed artifact storage
    ├── corridors.py     # Settlement corridor state machines
    ├── assets.py        # Smart asset lifecycle management
    ├── composition.py   # Multi-jurisdiction composition engine
    ├── validators.py    # Module/profile/zone validators
    └── cli.py           # Command-line interface

The composition engine enables deployments like:
    "Deploy NY civic code with Delaware corporate law, ADGM digital
     asset clearance/settlement/securities, and AI arbitration"

Reference implementation. Production deployments may differ while
conforming to the specification.
"""

__version__ = "0.4.44"
__spec_version__ = "0.4.44"

from tools.msez.core import (
    sha256_bytes,
    sha256_file,
    load_yaml,
    load_json,
    canonical_json_bytes,
    write_canonical_json,
    REPO_ROOT,
)

from tools.msez.composition import (
    ZoneComposition,
    JurisdictionLayer,
    compose_zone,
)

__all__ = [
    "__version__",
    "__spec_version__",
    "sha256_bytes",
    "sha256_file",
    "load_yaml",
    "load_json",
    "canonical_json_bytes",
    "write_canonical_json",
    "REPO_ROOT",
    "ZoneComposition",
    "JurisdictionLayer",
    "compose_zone",
]
