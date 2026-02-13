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


# ---------------------------------------------------------------------------
# Lazy re-exports from tools/msez.py (the monolith).
#
# The monolith coexists as tools/msez.py alongside this package.  Python
# resolves ``tools.msez`` to the package, so tests that do
# ``from tools.msez import schema_validator`` need the symbol here.
# We load the .py file explicitly (same technique as __main__.py) and
# re-export every symbol that downstream code depends on.
# ---------------------------------------------------------------------------
import argparse
import importlib.util as _ilu
import pathlib as _pl

_msez_py = _pl.Path(__file__).resolve().parent.parent / "msez.py"
_spec = _ilu.spec_from_file_location("tools._msez_cli", str(_msez_py))
_mod = _ilu.module_from_spec(_spec)
_spec.loader.exec_module(_mod)

# Constants
STACK_SPEC_VERSION = _mod.STACK_SPEC_VERSION

# File-writing helpers
write_canonical_json_file = _mod.write_canonical_json_file

# Schema validation
schema_validator = _mod.schema_validator
validate_with_schema = _mod.validate_with_schema

# Corridor state
corridor_state_genesis_root = _mod.corridor_state_genesis_root
corridor_state_next_root = _mod.corridor_state_next_root
corridor_expected_ruleset_digest_set = _mod.corridor_expected_ruleset_digest_set
_corridor_state_build_chain = _mod._corridor_state_build_chain

# Asset state
asset_state_genesis_root = _mod.asset_state_genesis_root
asset_state_next_root = _mod.asset_state_next_root

# Transition type registry
build_transition_type_registry_lock = _mod.build_transition_type_registry_lock
load_transition_type_registry_lock = _mod.load_transition_type_registry_lock
_build_transition_type_registry_mapping = _mod._build_transition_type_registry_mapping
resolve_transition_type_registry_lock_by_digest = _mod.resolve_transition_type_registry_lock_by_digest
transition_types_lock_cas_path = _mod.transition_types_lock_cas_path

# Artifact graph
build_artifact_graph_verify_report = _mod.build_artifact_graph_verify_report

# Corridor VC verification
verify_corridor_definition_vc = _mod.verify_corridor_definition_vc
verify_corridor_agreement_vc = _mod.verify_corridor_agreement_vc

# Transition type registry
TRANSITION_TYPES_SNAPSHOT_TAG = _mod.TRANSITION_TYPES_SNAPSHOT_TAG
transition_type_registry_snapshot_digest = _mod.transition_type_registry_snapshot_digest

# Internal helpers used by tests
_jcs_sha256_of_json_file = _mod._jcs_sha256_of_json_file
_compute_netting_and_legs = _mod._compute_netting_and_legs

# CLI commands (used by tests that invoke commands directly)
cmd_lock = _mod.cmd_lock
cmd_corridor_state_checkpoint = _mod.cmd_corridor_state_checkpoint
cmd_corridor_state_checkpoint_audit = _mod.cmd_corridor_state_checkpoint_audit
cmd_corridor_state_receipt_init = _mod.cmd_corridor_state_receipt_init
cmd_corridor_state_verify = _mod.cmd_corridor_state_verify
cmd_corridor_state_watcher_attest = _mod.cmd_corridor_state_watcher_attest
cmd_corridor_state_watcher_compare = _mod.cmd_corridor_state_watcher_compare
cmd_corridor_state_fork_alarm = _mod.cmd_corridor_state_fork_alarm
cmd_corridor_state_fork_inspect = _mod.cmd_corridor_state_fork_inspect
cmd_corridor_availability_attest = _mod.cmd_corridor_availability_attest
cmd_artifact_graph_verify = _mod.cmd_artifact_graph_verify
cmd_artifact_bundle_attest = _mod.cmd_artifact_bundle_attest
cmd_artifact_bundle_verify = _mod.cmd_artifact_bundle_verify
cmd_asset_state_receipt_init = _mod.cmd_asset_state_receipt_init
cmd_asset_state_checkpoint = _mod.cmd_asset_state_checkpoint
cmd_asset_state_verify = _mod.cmd_asset_state_verify
cmd_asset_state_fork_resolve = _mod.cmd_asset_state_fork_resolve
cmd_asset_state_inclusion_proof = _mod.cmd_asset_state_inclusion_proof
cmd_asset_state_verify_inclusion = _mod.cmd_asset_state_verify_inclusion
cmd_asset_anchor_verify = _mod.cmd_asset_anchor_verify
cmd_asset_module_init = _mod.cmd_asset_module_init

__all__ = [
    "__version__",
    "__spec_version__",
    "sha256_bytes",
    "sha256_file",
    "load_yaml",
    "load_json",
    "canonical_json_bytes",
    "write_canonical_json",
    "write_canonical_json_file",
    "REPO_ROOT",
    "STACK_SPEC_VERSION",
    "ZoneComposition",
    "JurisdictionLayer",
    "compose_zone",
    "schema_validator",
    "validate_with_schema",
    "corridor_state_genesis_root",
    "corridor_state_next_root",
    "corridor_expected_ruleset_digest_set",
    "_corridor_state_build_chain",
    "asset_state_genesis_root",
    "asset_state_next_root",
    "build_transition_type_registry_lock",
    "load_transition_type_registry_lock",
    "_build_transition_type_registry_mapping",
    "resolve_transition_type_registry_lock_by_digest",
    "transition_types_lock_cas_path",
    "build_artifact_graph_verify_report",
    "verify_corridor_definition_vc",
    "verify_corridor_agreement_vc",
    "cmd_lock",
    "cmd_corridor_state_checkpoint",
    "cmd_corridor_state_receipt_init",
    "cmd_corridor_state_verify",
    "cmd_corridor_state_watcher_attest",
    "cmd_corridor_state_watcher_compare",
    "cmd_corridor_state_fork_alarm",
    "cmd_corridor_state_fork_inspect",
    "cmd_corridor_availability_attest",
]
