import json
import pathlib
import sys


import yaml
from jsonschema import Draft202012Validator

REPO = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO))


def load_yaml(path: pathlib.Path):
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def load_json(path: pathlib.Path):
    return json.loads(path.read_text(encoding="utf-8"))


def test_corridor_definition_vcs_exist_and_verify():
    vc_schema = Draft202012Validator(load_json(REPO / "schemas" / "vc.corridor-definition.schema.json"))

    from tools.vc import verify_credential  # type: ignore

    for corridor_path in REPO.glob("modules/corridors/**/corridor.yaml"):
        mod_dir = corridor_path.parent
        manifest = load_yaml(corridor_path)

        vc_rel = manifest.get("definition_vc_path")
        assert vc_rel, f"{corridor_path} missing definition_vc_path"
        vc_path = mod_dir / vc_rel
        assert vc_path.exists(), f"Missing corridor VC at {vc_path}"

        vcj = load_json(vc_path)
        errors = [e.message for e in vc_schema.iter_errors(vcj)]
        assert not errors, f"{vc_path} does not validate VC schema: {errors}"

        results = verify_credential(vcj)
        assert results, f"{vc_path} has no proofs"
        bad = [r for r in results if not r.ok]
        assert not bad, f"{vc_path} has invalid proof(s): {[b.error for b in bad]}"
