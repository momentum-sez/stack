import pathlib

import yaml


def test_all_openapi_files_parse() -> None:
    repo_root = pathlib.Path(__file__).resolve().parents[1]
    apis_dir = repo_root / "apis"
    assert apis_dir.exists(), "repo missing apis/ directory"

    yaml_paths = sorted(p for p in apis_dir.glob("*.yaml") if p.is_file())
    assert yaml_paths, "no OpenAPI YAML files found under apis/"

    for p in yaml_paths:
        doc = yaml.safe_load(p.read_text(encoding="utf-8"))
        assert isinstance(doc, dict), f"OpenAPI file {p} did not parse to a mapping"
        assert "openapi" in doc, f"OpenAPI file {p} missing 'openapi' field"
        assert "info" in doc and isinstance(doc.get("info"), dict), f"OpenAPI file {p} missing 'info'"
        assert "paths" in doc and isinstance(doc.get("paths"), dict), f"OpenAPI file {p} missing 'paths'"