import pathlib, yaml, json
from jsonschema import Draft202012Validator

REPO = pathlib.Path(__file__).resolve().parents[1]

def load_yaml(p): return yaml.safe_load(p.read_text(encoding="utf-8"))
def load_json(p): return json.loads(p.read_text(encoding="utf-8"))

def test_corridor_trust_anchors_and_key_rotation_validate():
    trust_v = Draft202012Validator(load_json(REPO / "schemas" / "trust-anchors.schema.json"))
    key_v = Draft202012Validator(load_json(REPO / "schemas" / "key-rotation.schema.json"))

    for m in REPO.glob("modules/corridors/**/module.yaml"):
        mod_dir = m.parent
        ta = mod_dir / "trust-anchors.yaml"
        kr = mod_dir / "key-rotation.yaml"
        assert ta.exists(), f"Missing trust-anchors.yaml in {mod_dir}"
        assert kr.exists(), f"Missing key-rotation.yaml in {mod_dir}"
        ta_data = load_yaml(ta)
        kr_data = load_yaml(kr)
        ta_errs = sorted(trust_v.iter_errors(ta_data), key=str)
        kr_errs = sorted(key_v.iter_errors(kr_data), key=str)
        assert not ta_errs, f"{ta} invalid: {[e.message for e in ta_errs[:5]]}"
        assert not kr_errs, f"{kr} invalid: {[e.message for e in kr_errs[:5]]}"
