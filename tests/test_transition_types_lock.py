import json
import pathlib


def test_transition_types_lock_schema_and_digest():
    # Validate the committed transition-types.lock.json against schema and digest.
    from tools.msez import (
        REPO_ROOT,
        load_json,
        load_yaml,
        schema_validator,
        validate_with_schema,
        load_transition_type_registry_lock,
        build_transition_type_registry_lock,
        _build_transition_type_registry_mapping,
    )

    lock_path = REPO_ROOT / "registries" / "transition-types.lock.json"
    assert lock_path.exists(), "registries/transition-types.lock.json must be committed"

    lock_obj = load_json(lock_path)
    schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.lock.schema.json")
    errs = validate_with_schema(lock_obj, schema)
    assert not errs, "\n".join(errs)

    # Loader recomputes and validates snapshot digest.
    _lock2, mapping, digest = load_transition_type_registry_lock(lock_path)
    assert digest == lock_obj.get("snapshot_digest_sha256")
    assert "msez.example.transfer.v1" in mapping

    # Rebuild snapshot digest from the YAML registry and ensure it matches.
    reg_path = REPO_ROOT / "registries" / "transition-types.yaml"
    reg_obj = load_yaml(reg_path)
    reg_map = _build_transition_type_registry_mapping(reg_path.parent, reg_obj, label=str(reg_path))
    rebuilt = build_transition_type_registry_lock(reg_path=reg_path, reg_obj=reg_obj, reg_map=reg_map)
    assert rebuilt.get("snapshot_digest_sha256") == lock_obj.get("snapshot_digest_sha256")
