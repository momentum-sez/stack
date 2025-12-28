import pathlib

from tools.msez import load_yaml, load_json, schema_validator, validate_with_schema, sha256_bytes
from tools.lawpack import jcs_canonicalize


REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]


def test_transition_types_registry_schema_validation():
    reg_path = REPO_ROOT / "registries" / "transition-types.yaml"
    schema_path = REPO_ROOT / "schemas" / "transition-types.registry.schema.json"

    reg = load_yaml(reg_path)
    schema = schema_validator(schema_path)
    errs = validate_with_schema(reg, schema)
    assert errs == [], f"registry failed schema validation: {errs}"


def test_transition_types_registry_example_digests_match_files():
    reg = load_yaml(REPO_ROOT / "registries" / "transition-types.yaml")
    entries = {e.get("kind"): e for e in (reg.get("transition_types") or []) if isinstance(e, dict)}

    ex = entries.get("msez.example.transfer.v1")
    assert ex is not None, "missing example entry"

    schema_path = REPO_ROOT / str(ex.get("schema_path"))
    ruleset_path = REPO_ROOT / str(ex.get("ruleset_path"))

    schema_digest = sha256_bytes(jcs_canonicalize(load_json(schema_path)))
    ruleset_digest = sha256_bytes(jcs_canonicalize(load_json(ruleset_path)))

    assert ex.get("schema_digest_sha256") == schema_digest
    assert ex.get("ruleset_digest_sha256") == ruleset_digest
