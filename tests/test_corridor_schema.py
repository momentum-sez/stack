import json
import pathlib

import yaml
from jsonschema import Draft202012Validator

REPO = pathlib.Path(__file__).resolve().parents[1]


def load_yaml(path: pathlib.Path):
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def test_corridor_manifests_validate_against_schema():
    schema_path = REPO / "schemas" / "corridor.schema.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    validator = Draft202012Validator(schema)

    for corridor_path in REPO.glob("modules/**/corridor.yaml"):
        data = load_yaml(corridor_path)
        errors = [e.message for e in validator.iter_errors(data)]
        assert not errors, f"Invalid corridor manifest {corridor_path}: {errors}"
