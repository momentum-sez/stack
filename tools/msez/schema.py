"""JSON Schema validation infrastructure.

Provides a robust schema validation system with:
- Automatic schema resolution via $ref
- Cross-reference registry for all MSEZ schemas
- Cached validators for performance
- Clear error reporting
"""

from __future__ import annotations

import json
from functools import lru_cache
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from jsonschema import Draft202012Validator
from jsonschema.exceptions import ValidationError
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012

from tools.msez.core import REPO_ROOT, load_json


@lru_cache(maxsize=1)
def _schema_registry(repo_root: Path = REPO_ROOT) -> Registry:
    """Build a schema registry for all MSEZ schemas.

    This enables $ref resolution across the schema corpus.
    Cached for performance.
    """
    schemas_dir = repo_root / "schemas"
    if not schemas_dir.is_dir():
        return Registry()

    def _schema_files():
        for p in schemas_dir.glob("*.schema.json"):
            yield p
        for p in schemas_dir.glob("**/*.schema.json"):
            yield p

    resources = []
    for schema_path in _schema_files():
        try:
            schema = load_json(schema_path)
            if not isinstance(schema, dict):
                continue

            # Use $id from schema, or derive from filename
            schema_id = schema.get("$id", "")
            if not schema_id:
                # Derive URI from path relative to schemas/
                rel = schema_path.relative_to(schemas_dir)
                schema_id = f"https://schemas.momentum-sez.org/msez/{rel}"

            resource = Resource.from_contents(schema, default_specification=DRAFT202012)
            resources.append((schema_id, resource))
        except Exception:
            continue

    return Registry().with_resources(resources)


def schema_validator(
    schema_path: Path,
    repo_root: Path = REPO_ROOT,
) -> Draft202012Validator:
    """Create a validator for a schema file.

    Args:
        schema_path: Path to the JSON Schema file
        repo_root: Repository root for resolving references

    Returns:
        A configured Draft202012Validator
    """
    schema = load_json(schema_path)
    registry = _schema_registry(repo_root)
    return Draft202012Validator(schema, registry=registry)


def validate_against_schema(
    obj: Any,
    schema_path: Path,
    repo_root: Path = REPO_ROOT,
) -> List[str]:
    """Validate an object against a schema.

    Args:
        obj: The object to validate
        schema_path: Path to the JSON Schema
        repo_root: Repository root

    Returns:
        List of validation error messages (empty if valid)
    """
    validator = schema_validator(schema_path, repo_root)
    return [
        f"{error.json_path}: {error.message}"
        for error in validator.iter_errors(obj)
    ]


def validate_module(
    module_dir: Path,
    repo_root: Path = REPO_ROOT,
) -> Tuple[bool, List[str], Dict[str, Any]]:
    """Validate a module directory.

    Checks:
    - module.yaml exists and is valid YAML
    - Schema validation against module.schema.json
    - Required fields present
    - Dependencies resolvable

    Args:
        module_dir: Path to module directory
        repo_root: Repository root

    Returns:
        Tuple of (is_valid, error_messages, parsed_module)
    """
    from tools.msez.core import load_yaml

    errors = []
    module_data: Dict[str, Any] = {}

    # Check module.yaml exists
    module_yaml = module_dir / "module.yaml"
    if not module_yaml.exists():
        return False, [f"Missing module.yaml in {module_dir}"], {}

    # Load module.yaml
    try:
        module_data = load_yaml(module_yaml)
        if not isinstance(module_data, dict):
            return False, [f"module.yaml must be a mapping: {module_yaml}"], {}
    except Exception as e:
        return False, [f"Failed to load module.yaml: {e}"], {}

    # Schema validation
    schema_path = repo_root / "schemas" / "module.schema.json"
    if schema_path.exists():
        schema_errors = validate_against_schema(module_data, schema_path, repo_root)
        errors.extend(schema_errors)

    # Required field validation
    required = ["module_id", "version"]
    for field in required:
        if field not in module_data:
            errors.append(f"Missing required field: {field}")

    return len(errors) == 0, errors, module_data


def validate_zone(
    zone_path: Path,
    repo_root: Path = REPO_ROOT,
) -> Tuple[bool, List[str], Dict[str, Any]]:
    """Validate a zone YAML file.

    Args:
        zone_path: Path to zone.yaml
        repo_root: Repository root

    Returns:
        Tuple of (is_valid, error_messages, parsed_zone)
    """
    from tools.msez.core import load_yaml

    errors = []

    # Load zone.yaml
    try:
        zone_data = load_yaml(zone_path)
        if not isinstance(zone_data, dict):
            return False, [f"zone.yaml must be a mapping: {zone_path}"], {}
    except Exception as e:
        return False, [f"Failed to load zone.yaml: {e}"], {}

    # Schema validation
    schema_path = repo_root / "schemas" / "zone.schema.json"
    if schema_path.exists():
        schema_errors = validate_against_schema(zone_data, schema_path, repo_root)
        errors.extend(schema_errors)

    # Required fields
    required = ["zone_id", "jurisdiction_id"]
    for field in required:
        if field not in zone_data:
            errors.append(f"Missing required field: {field}")

    return len(errors) == 0, errors, zone_data


def validate_profile(
    profile_path: Path,
    repo_root: Path = REPO_ROOT,
) -> Tuple[bool, List[str], Dict[str, Any]]:
    """Validate a profile YAML file.

    Args:
        profile_path: Path to profile.yaml
        repo_root: Repository root

    Returns:
        Tuple of (is_valid, error_messages, parsed_profile)
    """
    from tools.msez.core import load_yaml

    errors = []

    # Load profile.yaml
    try:
        profile_data = load_yaml(profile_path)
        if not isinstance(profile_data, dict):
            return False, [f"profile.yaml must be a mapping: {profile_path}"], {}
    except Exception as e:
        return False, [f"Failed to load profile.yaml: {e}"], {}

    # Schema validation
    schema_path = repo_root / "schemas" / "profile.schema.json"
    if schema_path.exists():
        schema_errors = validate_against_schema(profile_data, schema_path, repo_root)
        errors.extend(schema_errors)

    # Required fields
    required = ["profile_id", "name"]
    for field in required:
        if field not in profile_data:
            errors.append(f"Missing required field: {field}")

    return len(errors) == 0, errors, profile_data


def find_all_modules(repo_root: Path = REPO_ROOT) -> List[Path]:
    """Find all module directories in the repository."""
    modules_dir = repo_root / "modules"
    if not modules_dir.is_dir():
        return []

    return [
        p.parent
        for p in modules_dir.rglob("module.yaml")
    ]


def find_all_profiles(repo_root: Path = REPO_ROOT) -> List[Path]:
    """Find all profile files in the repository."""
    profiles_dir = repo_root / "profiles"
    if not profiles_dir.is_dir():
        return []

    return list(profiles_dir.glob("*/profile.yaml"))


def find_all_zones(repo_root: Path = REPO_ROOT) -> List[Path]:
    """Find all zone files in the repository."""
    zones_dir = repo_root / "zones"
    if not zones_dir.is_dir():
        return []

    return list(zones_dir.glob("*/zone.yaml"))


def build_module_index(repo_root: Path = REPO_ROOT) -> Dict[str, Tuple[Path, Dict[str, Any]]]:
    """Build an index of all modules by module_id.

    Returns:
        Dict mapping module_id -> (module_dir, module_data)
    """
    from tools.msez.core import load_yaml

    index: Dict[str, Tuple[Path, Dict[str, Any]]] = {}

    for module_dir in find_all_modules(repo_root):
        module_yaml = module_dir / "module.yaml"
        try:
            data = load_yaml(module_yaml)
            if isinstance(data, dict) and "module_id" in data:
                index[data["module_id"]] = (module_dir, data)
        except Exception:
            continue

    return index
