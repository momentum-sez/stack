import pathlib, json, yaml
from jsonschema import Draft202012Validator

REPO = pathlib.Path(__file__).resolve().parents[1]

def load_json(p): return json.loads(p.read_text(encoding="utf-8"))
def load_yaml(p): return yaml.safe_load(p.read_text(encoding="utf-8"))

def validator(schema_name):
    return Draft202012Validator(load_json(REPO / "schemas" / schema_name))

def test_all_module_manifests_validate():
    v = validator("module.schema.json")
    for p in REPO.glob("modules/**/module.yaml"):
        data = load_yaml(p)
        errs = sorted(v.iter_errors(data), key=str)
        assert not errs, f"{p} errors: {[e.message for e in errs[:5]]}"

def test_all_profiles_validate():
    v = validator("profile.schema.json")
    for p in REPO.glob("profiles/**/profile.yaml"):
        data = load_yaml(p)
        errs = sorted(v.iter_errors(data), key=str)
        assert not errs, f"{p} errors: {[e.message for e in errs[:5]]}"

def test_all_zones_validate():
    v = validator("zone.schema.json")
    for p in REPO.glob("jurisdictions/**/zone.yaml"):
        data = load_yaml(p)
        errs = sorted(v.iter_errors(data), key=str)
        assert not errs, f"{p} errors: {[e.message for e in errs[:5]]}"
