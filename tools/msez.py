#!/usr/bin/env python3
"""MSEZ Stack tool (reference implementation) — v0.2

Capabilities:
- validate modules/profiles/zones against schemas
- validate Akoma Ntoso against XSD (when schemas present)
- fetch Akoma schemas
- render Akoma to HTML/PDF
- generate deterministic stack.lock from zone.yaml
- build a composed bundle directory

This tool is a **reference implementation**. Production implementations may differ while still conforming to the spec.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import sys
from datetime import datetime
from typing import Any, Dict, List, Tuple

import yaml
from jsonschema import Draft202012Validator
from lxml import etree

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]

def load_yaml(path: pathlib.Path) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))

def load_json(path: pathlib.Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()

def sha256_file(path: pathlib.Path) -> str:
    return sha256_bytes(path.read_bytes())

def schema_validator(schema_path: pathlib.Path) -> Draft202012Validator:
    schema = load_json(schema_path)
    return Draft202012Validator(schema)

def validate_with_schema(obj: Any, validator: Draft202012Validator) -> List[str]:
    errors = []
    for e in sorted(validator.iter_errors(obj), key=str):
        errors.append(f"{list(e.absolute_path)}: {e.message}")
    return errors

def find_modules(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p.parent for p in repo_root.glob("modules/**/module.yaml")]

def find_profiles(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p for p in repo_root.glob("profiles/**/profile.yaml")]

def find_zones(repo_root: pathlib.Path) -> List[pathlib.Path]:
    return [p for p in repo_root.glob("jurisdictions/**/zone.yaml")]

def validate_module(module_dir: pathlib.Path, validator: Draft202012Validator) -> Tuple[bool, List[str], Dict[str,Any]]:
    manifest_path = module_dir / "module.yaml"
    data = load_yaml(manifest_path)
    errors = validate_with_schema(data, validator)

    # Basic checks for provided artifact paths
    for prov in data.get("provides", []):
        rel = prov.get("path")
        if rel and not (module_dir / rel).exists():
            errors.append(f"provides path missing: {module_dir}/{rel}")
    return (len(errors) == 0, errors, data)

def load_akoma_schema(schema_dir: pathlib.Path) -> etree.XMLSchema | None:
    main_xsd = schema_dir / "akomantoso30.xsd"
    if not main_xsd.exists():
        return None
    try:
        doc = etree.parse(str(main_xsd))
        return etree.XMLSchema(doc)
    except Exception as ex:
        print("WARN: failed to load Akoma schema:", ex)
        return None

def validate_akoma_xml(module_dir: pathlib.Path, schema: etree.XMLSchema | None) -> List[str]:
    errs: List[str] = []
    for xml_file in module_dir.glob("src/akn/**/*.xml"):
        try:
            doc = etree.parse(str(xml_file))
            # Ensure there are stable eId anchors somewhere
            if not doc.xpath('//*[@eId]'):
                errs.append(f"No eId anchors found in {xml_file}")
            if schema is not None and not schema.validate(doc):
                for e in schema.error_log:
                    errs.append(f"{xml_file}: {e.message} (line {e.line})")
        except Exception as ex:
            errs.append(f"XML parse error in {xml_file}: {ex}")
    return errs

def cmd_fetch_akoma(args: argparse.Namespace) -> int:
    from tools.akoma.fetch_schemas import fetch  # type: ignore
    dest = REPO_ROOT / "tools" / "akoma" / "schemas"
    fetch(dest)
    print("Fetched Akoma schemas into", dest)
    return 0

def cmd_render(args: argparse.Namespace) -> int:
    from tools.akoma.render import main as render_main  # type: ignore
    # delegate to render.py
    sys.argv = ["render", args.xml, "--out-dir", args.out_dir] + (["--pdf"] if args.pdf else [])
    return render_main()

def resolve_module_by_id(module_id: str) -> Tuple[pathlib.Path, Dict[str,Any]] | None:
    for mdir in find_modules(REPO_ROOT):
        data = load_yaml(mdir / "module.yaml")
        if data.get("module_id") == module_id:
            return mdir, data
    return None

def cmd_validate(args: argparse.Namespace) -> int:
    module_schema = schema_validator(REPO_ROOT / "schemas" / "module.schema.json")
    profile_schema = schema_validator(REPO_ROOT / "schemas" / "profile.schema.json")
    zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")

    akoma_schema = load_akoma_schema(REPO_ROOT / "tools" / "akoma" / "schemas")

    if args.all_modules:
        ok = True
        for mdir in find_modules(REPO_ROOT):
            m_ok, m_errors, _ = validate_module(mdir, module_schema)
            m_errors.extend(validate_akoma_xml(mdir, akoma_schema))
            if not m_ok or m_errors:
                ok = False
                print(f"\nMODULE FAIL: {mdir}")
                for e in m_errors:
                    print("  -", e)
        if ok:
            print("OK: all modules validate")
            return 0
        return 2

    if args.all_profiles:
        ok = True
        for p in find_profiles(REPO_ROOT):
            profile = load_yaml(p)
            errors = validate_with_schema(profile, profile_schema)
            if errors:
                ok = False
                print(f"\nPROFILE FAIL: {p}")
                for e in errors:
                    print("  -", e)
        if ok:
            print("OK: all profiles validate")
            return 0
        return 2

    if args.all_zones:
        ok = True
        for z in find_zones(REPO_ROOT):
            zone = load_yaml(z)
            errors = validate_with_schema(zone, zone_schema)
            if errors:
                ok = False
                print(f"\nZONE FAIL: {z}")
                for e in errors:
                    print("  -", e)
        if ok:
            print("OK: all zones validate")
            return 0
        return 2

    # Default: validate a single profile path
    profile_path = pathlib.Path(args.profile)
    if not profile_path.is_absolute():
        profile_path = REPO_ROOT / profile_path
    if not profile_path.exists():
        print(f"ERROR: profile not found: {profile_path}")
        return 2

    profile = load_yaml(profile_path)
    errors = validate_with_schema(profile, profile_schema)
    if errors:
        print("PROFILE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    # Validate referenced modules
    ok = True
    for m in profile.get("modules", []):
        mid = m["module_id"]
        resolved = resolve_module_by_id(mid)
        if not resolved:
            ok = False
            print(f"Missing module: {mid}")
            continue
        mdir, data = resolved
        want_ver = str(m.get("version"))
        have_ver = str(data.get("version"))
        if want_ver and want_ver != have_ver:
            print(f"WARN: profile pins {mid}={want_ver} but module version is {have_ver}")
        m_ok, m_errors, _ = validate_module(mdir, module_schema)
        m_errors.extend(validate_akoma_xml(mdir, akoma_schema))
        if not m_ok or m_errors:
            ok = False
            print(f"MODULE FAIL: {mid}")
            for e in m_errors:
                print("  -", e)
    if ok:
        print("OK: profile and referenced modules validate")
        return 0
    return 2

def cmd_build(args: argparse.Namespace) -> int:
    profile_path = pathlib.Path(args.profile)
    if not profile_path.is_absolute():
        profile_path = REPO_ROOT / profile_path
    profile = load_yaml(profile_path)

    out_dir = pathlib.Path(args.out)
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    bundle_dir = out_dir / "bundle"
    if bundle_dir.exists():
        import shutil
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    for m in profile.get("modules", []):
        mid = m["module_id"]
        resolved = resolve_module_by_id(mid)
        if not resolved:
            print("ERROR: cannot build; missing module", mid)
            return 2
        found, _ = resolved
        target = bundle_dir / found.relative_to(REPO_ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        import shutil
        shutil.copytree(found, target)

    (bundle_dir / "profile.resolved.yaml").write_text(yaml.safe_dump(profile, sort_keys=False), encoding="utf-8")
    print("BUILD OK: wrote bundle to", bundle_dir)
    return 0

def digest_dir(path: pathlib.Path) -> str:
    # Deterministic directory digest: hash of relative paths + content hashes
    h = hashlib.sha256()
    files = sorted([p for p in path.rglob("*") if p.is_file()])
    for f in files:
        rel = str(f.relative_to(path)).encode("utf-8")
        h.update(rel)
        h.update(b"\0")
        h.update(f.read_bytes())
        h.update(b"\0")
    return h.hexdigest()

def cmd_lock(args: argparse.Namespace) -> int:
    zone_path = pathlib.Path(args.zone)
    if not zone_path.is_absolute():
        zone_path = REPO_ROOT / zone_path
    zone = load_yaml(zone_path)

    # validate zone schema first
    zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
    errors = validate_with_schema(zone, zone_schema)
    if errors:
        print("ZONE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    profile_id = zone["profile"]["profile_id"]
    profile_version = zone["profile"]["version"]

    # Find profile file by profile_id (best effort)
    profile_file = None
    for p in find_profiles(REPO_ROOT):
        prof = load_yaml(p)
        if prof.get("profile_id") == profile_id:
            profile_file = p
            profile = prof
            break
    if profile_file is None:
        print("ERROR: profile not found for profile_id", profile_id)
        return 2

    lock = {
        "stack_spec_version": "0.2.0",
        "generated_at": datetime.utcnow().replace(microsecond=0).isoformat() + "Z",
        "zone_id": zone["zone_id"],
        "profile": {"profile_id": profile_id, "version": profile_version},
        "modules": [],
        "overlays": [],
        "corridors": []
    }

    # module resolution
    for m in profile.get("modules", []):
        mid = m["module_id"]
        resolved = resolve_module_by_id(mid)
        if not resolved:
            print("ERROR: missing module", mid)
            return 2
        mdir, mdata = resolved
        entry = {
            "module_id": mid,
            "version": mdata.get("version"),
            "variant": m.get("variant", "default"),
            "params": m.get("params", {}),
            "manifest_sha256": sha256_file(mdir / "module.yaml"),
            "content_sha256": digest_dir(mdir),
            "provides": mdata.get("provides", [])
        }
        lock["modules"].append(entry)

    # overlays
    for ov in zone.get("overlays", []) or []:
        module_id = ov["module_id"]
        patch_hashes = []
        for patch in ov.get("patches", []):
            pp = zone_path.parent / patch
            if pp.exists():
                patch_hashes.append(sha256_file(pp))
            else:
                patch_hashes.append("MISSING")
        lock["overlays"].append({"module_id": module_id, "patches_sha256": patch_hashes})

    # corridors
    for cid in zone.get("corridors", []) or []:
        # best effort: locate corridor module by corridor_id prefix
        trust_hash = None
        rot_hash = None
        for mdir in find_modules(REPO_ROOT):
            mdata = load_yaml(mdir / "module.yaml")
            if mdata.get("kind") == "corridor":
                cy = mdir / "corridor.yaml"
                if cy.exists():
                    c = load_yaml(cy)
                    if c.get("corridor_id") == cid:
                        ta = mdir / "trust-anchors.yaml"
                        kr = mdir / "key-rotation.yaml"
                        trust_hash = sha256_file(ta) if ta.exists() else None
                        rot_hash = sha256_file(kr) if kr.exists() else None
                        break
        lock["corridors"].append({
            "corridor_id": cid,
            "trust_anchors_sha256": trust_hash or "",
            "key_rotation_sha256": rot_hash or ""
        })

    out_path = pathlib.Path(args.out) if args.out else zone_path.parent / (zone.get("lockfile_path") or "stack.lock")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(yaml.safe_dump(lock, sort_keys=False), encoding="utf-8")
    print("LOCK OK: wrote", out_path)
    return 0

def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    v = sub.add_parser("validate")
    v.add_argument("profile", nargs="?", default="profiles/digital-financial-center/profile.yaml")
    v.add_argument("--all-modules", action="store_true")
    v.add_argument("--all-profiles", action="store_true")
    v.add_argument("--all-zones", action="store_true")
    v.set_defaults(func=cmd_validate)

    b = sub.add_parser("build")
    b.add_argument("profile")
    b.add_argument("--out", default="dist")
    b.set_defaults(func=cmd_build)

    f = sub.add_parser("fetch-akoma-schemas")
    f.set_defaults(func=cmd_fetch_akoma)

    r = sub.add_parser("render")
    r.add_argument("xml")
    r.add_argument("--out-dir", default="dist/render")
    r.add_argument("--pdf", action="store_true")
    r.set_defaults(func=cmd_render)

    l = sub.add_parser("lock")
    l.add_argument("zone")
    l.add_argument("--out", default="")
    l.set_defaults(func=cmd_lock)

    args = ap.parse_args()
    return args.func(args)

if __name__ == "__main__":
    sys.exit(main())
