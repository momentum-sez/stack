import pathlib
import re
import sys

import yaml

REPO = pathlib.Path(__file__).resolve().parents[1]

# Import the tool's declared stack spec version to keep tests aligned with version bumps.
sys.path.insert(0, str(REPO))
from tools.msez import STACK_SPEC_VERSION  # type: ignore


def load_yaml(path: pathlib.Path):
    return yaml.safe_load(path.read_text(encoding="utf-8"))


SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+].*)?$")


def semver_tuple(version: str):
    m = SEMVER_RE.match(str(version).strip())
    assert m, f"Not a semver: {version}"
    return (int(m.group(1)), int(m.group(2)), int(m.group(3)))


def semver_satisfies(version: str, constraint: str) -> bool:
    v = semver_tuple(version)
    for part in [p.strip() for p in str(constraint).split(",") if p.strip()]:
        if part.startswith(">="):
            op, req = ">=", part[2:]
        elif part.startswith("<="):
            op, req = "<=", part[2:]
        elif part.startswith(">"):
            op, req = ">", part[1:]
        elif part.startswith("<"):
            op, req = "<", part[1:]
        elif part.startswith("=="):
            op, req = "==", part[2:]
        elif part.startswith("="):
            op, req = "==", part[1:]
        else:
            raise AssertionError(f"Unsupported constraint fragment: {part}")
        r = semver_tuple(req)
        if op == ">=" and not (v >= r):
            return False
        if op == ">" and not (v > r):
            return False
        if op == "<=" and not (v <= r):
            return False
        if op == "<" and not (v < r):
            return False
        if op == "==" and not (v == r):
            return False
    return True


def iter_dep_specs(dep_list):
    out = []
    if not dep_list or not isinstance(dep_list, list):
        return out
    for d in dep_list:
        if isinstance(d, str):
            out.append((d, None))
        elif isinstance(d, dict) and d.get("module_id"):
            out.append((str(d["module_id"]), d.get("constraint")))
    return out


def test_profiles_resolve_modules_variants_and_dependencies():
    # Build module index
    module_index = {}
    for mpath in REPO.glob("modules/**/module.yaml"):
        data = load_yaml(mpath)
        module_index[data["module_id"]] = data

    # Build corridor index
    corridor_ids = set()
    for mpath in REPO.glob("modules/**/module.yaml"):
        mdir = mpath.parent
        mdata = load_yaml(mpath)
        if mdata.get("kind") != "corridor":
            continue
        cpath = mdir / "corridor.yaml"
        if cpath.exists():
            corridor_ids.add(load_yaml(cpath).get("corridor_id"))

    for ppath in REPO.glob("profiles/**/profile.yaml"):
        profile = load_yaml(ppath)
        assert profile.get("stack_spec_version") == STACK_SPEC_VERSION, f"{ppath} has wrong stack_spec_version"
        mods = profile.get("modules") or []
        assert isinstance(mods, list)
        profile_module_ids = {m.get("module_id") for m in mods if isinstance(m, dict)}

        for m in mods:
            assert isinstance(m, dict)
            mid = m.get("module_id")
            assert mid in module_index, f"{ppath} references missing module {mid}"
            mdata = module_index[mid]
            # variant exists
            assert m.get("variant") in (mdata.get("variants") or []), f"{ppath} pins unknown variant for {mid}"
            # version pin matches manifest (repo currently holds one version per module)
            assert str(m.get("version")) == str(mdata.get("version")), f"{ppath} pins {mid} version {m.get('version')} but manifest is {mdata.get('version')}"
            # dependencies satisfied
            for dep_id, constraint in iter_dep_specs(mdata.get("depends_on")):
                assert dep_id in profile_module_ids, f"{ppath} missing dependency: {mid} depends_on {dep_id}"
                if constraint:
                    dep_version = None
                    for mm in mods:
                        if mm.get("module_id") == dep_id:
                            dep_version = mm.get("version")
                            break
                    assert dep_version is not None
                    assert semver_satisfies(str(dep_version), str(constraint)), f"{ppath} pins {dep_id} version {dep_version} which violates constraint '{constraint}' (required by {mid})"

        for cid in profile.get("corridors", []) or []:
            assert cid in corridor_ids, f"{ppath} references unknown corridor_id {cid}"
