#!/usr/bin/env python3
"""MSEZ Stack tool (reference implementation) — v0.4.0

Capabilities:
- validate modules/profiles/zones against schemas
- validate Akoma Ntoso against XSD (when schemas present)
- fetch Akoma schemas
- render Akoma to HTML/PDF
- generate deterministic stack.lock from zone.yaml
- build a composed bundle directory
- check policy-to-code coverage for MUST/SHALL clauses
- diff two lockfiles for upgrade impact analysis
- publish rendered artifacts (Akoma -> HTML/PDF) for distribution
- sign/verify Verifiable Credentials (VC) for corridor integrity
- verify corridor cryptographic bindings (manifest + security artifacts + VC)

This tool is a **reference implementation**. Production implementations may differ while still conforming to the spec.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import sys
from datetime import datetime
from typing import Any, Dict, List, Tuple

import yaml
from jsonschema import Draft202012Validator
from lxml import etree

REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]

# Ensure imports like `tools.akoma.render` work even when this file is executed
# as a script (sys.path[0] becomes the tools/ directory).
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))
STACK_SPEC_VERSION = "0.4.0"

# Templating: we intentionally support two simple placeholder syntaxes used in v0.4 (safe placeholder subset)
# - {{ VAR_NAME }} (common in Akoma templates)
# - ${var_name}    (common in YAML corridor manifests)
RE_JINJA_LITE = re.compile(r"\{\{\s*([A-Za-z0-9_]+)\s*\}\}")
RE_DOLLAR = re.compile(r"\$\{\s*([A-Za-z0-9_]+)\s*\}")

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

def build_module_index(repo_root: pathlib.Path) -> Dict[str, Tuple[pathlib.Path, Dict[str, Any]]]:
    """Build an in-memory index of module_id -> (module_dir, manifest)."""
    index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]] = {}
    for mdir in find_modules(repo_root):
        try:
            data = load_yaml(mdir / "module.yaml")
        except Exception:
            continue
        mid = data.get("module_id")
        if mid:
            index[str(mid)] = (mdir, data)
    return index

def build_profile_index(repo_root: pathlib.Path) -> Dict[str, pathlib.Path]:
    """Build an in-memory index of profile_id -> profile.yaml path."""
    idx: Dict[str, pathlib.Path] = {}
    for p in find_profiles(repo_root):
        try:
            prof = load_yaml(p)
        except Exception:
            continue
        pid = prof.get("profile_id")
        if pid:
            idx[str(pid)] = p
    return idx

def build_corridor_index(repo_root: pathlib.Path) -> Dict[str, pathlib.Path]:
    """Index corridor_id -> module_dir for corridor modules."""
    idx: Dict[str, pathlib.Path] = {}
    for mdir in find_modules(repo_root):
        try:
            mdata = load_yaml(mdir / "module.yaml")
        except Exception:
            continue
        if mdata.get("kind") != "corridor":
            continue
        cy = mdir / "corridor.yaml"
        if not cy.exists():
            continue
        try:
            c = load_yaml(cy)
        except Exception:
            continue
        cid = c.get("corridor_id")
        if cid:
            idx[str(cid)] = mdir
    return idx

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

def resolve_module_by_id(
    module_id: str,
    module_index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]] | None = None,
) -> Tuple[pathlib.Path, Dict[str, Any]] | None:
    """Resolve a module by module_id.

    Pass a prebuilt index for speed/consistency.
    """
    idx = module_index or build_module_index(REPO_ROOT)
    return idx.get(module_id)

def extract_dep_ids(dep_list: Any) -> List[str]:
    """Extract dependency module IDs from depends_on supporting legacy string and object forms."""
    deps: List[str] = []
    if not dep_list:
        return deps
    if not isinstance(dep_list, list):
        return deps
    for d in dep_list:
        if isinstance(d, str):
            deps.append(d)
        elif isinstance(d, dict) and d.get("module_id"):
            deps.append(str(d["module_id"]))
    return deps

def iter_dep_specs(dep_list: Any) -> List[Tuple[str, str | None]]:
    """Return (module_id, constraint) pairs from a depends_on list."""
    out: List[Tuple[str, str | None]] = []
    if not dep_list or not isinstance(dep_list, list):
        return out
    for d in dep_list:
        if isinstance(d, str):
            out.append((d, None))
        elif isinstance(d, dict):
            mid = d.get("module_id")
            if mid:
                out.append((mid, d.get("constraint")))
    return out

# Minimal SemVer constraint evaluation (supports comma-separated comparisons)
SEMVER_RE = re.compile(r"^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:[-+].*)?$")

def _semver_tuple(v: str) -> Tuple[int, int, int]:
    m = SEMVER_RE.match(v.strip())
    if not m:
        raise ValueError(f"Invalid semver: {v!r}")
    return int(m.group(1)), int(m.group(2)), int(m.group(3))

def semver_satisfies(version: str, constraint: str) -> bool:
    """Return True if `version` satisfies a constraint string.

    Supported forms:
    - ">=0.1.0"
    - ">=0.1.0,<0.2.0"
    - "==0.2.0" / "=0.2.0"
    """
    v = _semver_tuple(version)
    for raw in [p.strip() for p in constraint.split(",") if p.strip()]:
        op = None
        rhs = None
        for candidate in (">=", "<=", "==", "=", ">", "<"):
            if raw.startswith(candidate):
                op = "==" if candidate == "=" else candidate
                rhs = raw[len(candidate):].strip()
                break
        if op is None or rhs is None:
            raise ValueError(f"Unsupported constraint fragment: {raw!r}")
        r = _semver_tuple(rhs)
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

def build_template_context(base: Dict[str, Any]) -> Dict[str, Any]:
    """Return a template context that includes multiple key aliases.

    - original keys (as provided)
    - UPPERCASE aliases for snake_case keys
    """
    ctx: Dict[str, Any] = dict(base)
    for k, v in list(base.items()):
        if isinstance(k, str):
            ctx[k.upper()] = v
    return ctx

def render_text_template(text: str, ctx: Dict[str, Any], strict: bool = False) -> str:
    """Render text by replacing {{VAR}} and ${var} placeholders.

    This intentionally keeps behavior deterministic and dependency-free.
    """
    def repl_jinja(m: re.Match) -> str:
        key = m.group(1)
        if key in ctx:
            return str(ctx[key])
        if strict:
            raise KeyError(f"Missing template variable: {key}")
        return m.group(0)

    def repl_dollar(m: re.Match) -> str:
        key = m.group(1)
        if key in ctx:
            return str(ctx[key])
        # allow ${foo} to fall back to ${FOO} if present
        ukey = key.upper()
        if ukey in ctx:
            return str(ctx[ukey])
        if strict:
            raise KeyError(f"Missing template variable: {key}")
        return m.group(0)

    text = RE_JINJA_LITE.sub(repl_jinja, text)
    text = RE_DOLLAR.sub(repl_dollar, text)
    return text

def render_templates_in_dir(
    root: pathlib.Path,
    ctx: Dict[str, Any],
    strict: bool = False,
    include_globs: Tuple[str, ...] = (
        "**/*.xml",
        "**/*.yaml",
        "**/*.yml",
        "**/*.md",
    ),
) -> List[str]:
    """Render templates in-place under root.

    Returns a list of files modified (relative paths).
    """
    touched: List[str] = []
    for pat in include_globs:
        for p in root.glob(pat):
            if not p.is_file():
                continue
            # skip module manifests (metadata)
            if p.name in {"module.yaml", "profile.yaml", "zone.yaml", "stack.lock"}:
                continue
            try:
                raw = p.read_text(encoding="utf-8")
            except Exception:
                continue
            if "{{" not in raw and "${" not in raw:
                continue
            try:
                rendered = render_text_template(raw, ctx, strict=strict)
            except KeyError as ex:
                raise
            if rendered != raw:
                p.write_text(rendered, encoding="utf-8")
                touched.append(str(p.relative_to(root)))
    return touched

def validate_policy_to_code_map(
    map_path: pathlib.Path,
    validator: Draft202012Validator,
) -> List[str]:
    """Validate a policy-to-code map against its JSON Schema."""
    try:
        data = load_yaml(map_path)
    except Exception as ex:
        return [f"Failed to parse {map_path}: {ex}"]
    return validate_with_schema(data, validator)

def validate_profile_semantics(
    profile: Dict[str, Any],
    module_index: Dict[str, Tuple[pathlib.Path, Dict[str, Any]]],
    corridor_index: Dict[str, pathlib.Path],
) -> List[str]:
    errs: List[str] = []
    if profile.get("stack_spec_version") != STACK_SPEC_VERSION:
        errs.append(
            f"profile.stack_spec_version={profile.get('stack_spec_version')} does not match tool STACK_SPEC_VERSION={STACK_SPEC_VERSION}"
        )

    modules = profile.get("modules") or []
    if not isinstance(modules, list):
        return ["profile.modules must be a list"]
    profile_module_ids = {m.get("module_id") for m in modules if isinstance(m, dict)}

    for m in modules:
        if not isinstance(m, dict):
            errs.append("profile.modules contains a non-object entry")
            continue
        mid = m.get("module_id")
        if not mid:
            errs.append("profile.modules entry missing module_id")
            continue
        if mid not in module_index:
            errs.append(f"Missing module referenced by profile: {mid}")
            continue
        mdir, mdata = module_index[mid]
        want_ver = str(m.get("version"))
        have_ver = str(mdata.get("version"))
        if want_ver and want_ver != have_ver:
            errs.append(f"Version pin mismatch for {mid}: profile pins {want_ver}, module manifest is {have_ver}")

        want_variant = str(m.get("variant"))
        variants = mdata.get("variants") or []
        if want_variant and want_variant not in variants:
            errs.append(f"Variant '{want_variant}' not declared by module {mid}. Available: {variants}")

        # dependencies must be explicit in the profile for determinism
        for dep_id, constraint in iter_dep_specs(mdata.get("depends_on")):
            if dep_id not in profile_module_ids:
                errs.append(
                    f"Unresolved dependency: {mid} depends_on {dep_id} (missing from profile)"
                )
                continue
            if constraint:
                dep_ver = mods_by_id.get(dep_id, {}).get("version")
                if dep_ver and not semver_satisfies(dep_ver, constraint):
                    errs.append(
                        f"Dependency constraint not satisfied: {mid} requires {dep_id} {constraint}, but profile pins {dep_ver}"
                    )

    # corridor ids in profile should be resolvable (optional but recommended)
    for cid in profile.get("corridors", []) or []:
        if cid not in corridor_index:
            errs.append(f"Profile references unknown corridor_id: {cid}")
    return errs

def validate_zone_semantics(
    zone: Dict[str, Any],
    profile_index: Dict[str, pathlib.Path],
    corridor_index: Dict[str, pathlib.Path],
) -> List[str]:
    errs: List[str] = []
    prof = zone.get("profile") or {}
    pid = prof.get("profile_id")
    if pid and pid not in profile_index:
        errs.append(f"Zone references unknown profile_id: {pid}")

    for cid in zone.get("corridors", []) or []:
        if cid not in corridor_index:
            errs.append(f"Zone enables unknown corridor_id: {cid}")
    return errs

def cmd_validate(args: argparse.Namespace) -> int:
    module_schema = schema_validator(REPO_ROOT / "schemas" / "module.schema.json")
    profile_schema = schema_validator(REPO_ROOT / "schemas" / "profile.schema.json")
    zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
    corridor_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.schema.json")
    policy_schema = schema_validator(REPO_ROOT / "schemas" / "policy-to-code.schema.json")

    akoma_schema = load_akoma_schema(REPO_ROOT / "tools" / "akoma" / "schemas")

    module_index = build_module_index(REPO_ROOT)
    profile_index = build_profile_index(REPO_ROOT)
    corridor_index = build_corridor_index(REPO_ROOT)

    if args.all_modules:
        ok = True
        for mdir in find_modules(REPO_ROOT):
            m_ok, m_errors, mdata = validate_module(mdir, module_schema)
            m_errors.extend(validate_akoma_xml(mdir, akoma_schema))

            # Validate policy-to-code maps when present
            map_path = mdir / "src" / "policy-to-code" / "map.yaml"
            if map_path.exists():
                for e in validate_policy_to_code_map(map_path, policy_schema):
                    m_errors.append(f"{map_path.relative_to(mdir)}: {e}")

            # Validate corridor manifests for corridor modules
            if mdata.get("kind") == "corridor":
                cy = mdir / "corridor.yaml"
                if not cy.exists():
                    m_errors.append("Missing corridor.yaml")
                else:
                    cdata = load_yaml(cy)
                    c_errs = validate_with_schema(cdata, corridor_schema)
                    for e in c_errs:
                        m_errors.append(f"corridor.yaml: {e}")


                # Corridor VC binding (cryptographically meaningful corridor definitions)
                m_errors.extend(verify_corridor_definition_vc(mdir))
                m_errors.extend(verify_corridor_agreement_vc(mdir))
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
            errors.extend(validate_profile_semantics(profile, module_index, corridor_index))
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
            errors.extend(validate_zone_semantics(zone, profile_index, corridor_index))
            if errors:
                ok = False
                print(f"\nZONE FAIL: {z}")
                for e in errors:
                    print("  -", e)
        if ok:
            print("OK: all zones validate")
            return 0
        return 2

    # Optional: validate a specific zone file
    if getattr(args, "zone", None):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        if not zone_path.exists():
            print(f"ERROR: zone not found: {zone_path}")
            return 2
        zone = load_yaml(zone_path)
        errors = validate_with_schema(zone, zone_schema)
        errors.extend(validate_zone_semantics(zone, profile_index, corridor_index))
        if errors:
            print(f"ZONE FAIL: {zone_path}")
            for e in errors:
                print("  -", e)
            return 2
        print("OK: zone validates")
        return 0

    # Default: validate a single profile path
    profile_path = pathlib.Path(args.profile)
    if not profile_path.is_absolute():
        profile_path = REPO_ROOT / profile_path
    if not profile_path.exists():
        print(f"ERROR: profile not found: {profile_path}")
        return 2

    profile = load_yaml(profile_path)
    errors = validate_with_schema(profile, profile_schema)
    errors.extend(validate_profile_semantics(profile, module_index, corridor_index))
    if errors:
        print("PROFILE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    # Validate referenced modules (schema + Akoma + policy-to-code map)
    ok = True
    for m in profile.get("modules", []):
        if not isinstance(m, dict):
            ok = False
            print("Invalid profile.modules entry (not an object)")
            continue
        mid = m.get("module_id")
        if not mid:
            ok = False
            print("Missing module_id in profile.modules entry")
            continue
        resolved = resolve_module_by_id(mid, module_index)
        if not resolved:
            ok = False
            print(f"Missing module: {mid}")
            continue
        mdir, _ = resolved

        m_ok, m_errors, mdata = validate_module(mdir, module_schema)
        m_errors.extend(validate_akoma_xml(mdir, akoma_schema))

        map_path = mdir / "src" / "policy-to-code" / "map.yaml"
        if map_path.exists():
            for e in validate_policy_to_code_map(map_path, policy_schema):
                m_errors.append(f"{map_path.relative_to(mdir)}: {e}")

        # corridor manifest validation (if corridor module)
        if mdata.get("kind") == "corridor":
            cy = mdir / "corridor.yaml"
            if cy.exists():
                c_errs = validate_with_schema(load_yaml(cy), corridor_schema)
                for e in c_errs:
                    m_errors.append(f"corridor.yaml: {e}")

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
    """Compose a deterministic bundle.

    Build modes:
    - Profile build: `msez build profiles/.../profile.yaml`
    - Zone build (recommended): `msez build --zone jurisdictions/.../zone.yaml`
      This applies overlays and params_overrides from the zone manifest.
    """

    module_index = build_module_index(REPO_ROOT)
    profile_index = build_profile_index(REPO_ROOT)
    corridor_index = build_corridor_index(REPO_ROOT)

    # Load zone/profile
    zone = None
    zone_path = None
    if getattr(args, "zone", ""):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        if not zone_path.exists():
            print(f"ERROR: zone not found: {zone_path}")
            return 2
        zone = load_yaml(zone_path)

        # schema + semantics
        zone_schema = schema_validator(REPO_ROOT / "schemas" / "zone.schema.json")
        zerrs = validate_with_schema(zone, zone_schema)
        zerrs.extend(validate_zone_semantics(zone, profile_index, corridor_index))
        if zerrs:
            print("ZONE FAIL:")
            for e in zerrs:
                print("  -", e)
            return 2

        pid = zone["profile"]["profile_id"]
        prof_path = profile_index.get(pid)
        if not prof_path:
            print(f"ERROR: profile not found for profile_id {pid}")
            return 2
        profile = load_yaml(prof_path)
    else:
        if not getattr(args, "profile", ""):
            print("ERROR: provide either a profile path or --zone")
            return 2
        profile_path = pathlib.Path(args.profile)
        if not profile_path.is_absolute():
            profile_path = REPO_ROOT / profile_path
        if not profile_path.exists():
            print(f"ERROR: profile not found: {profile_path}")
            return 2
        profile = load_yaml(profile_path)

    # Validate profile before building
    profile_schema = schema_validator(REPO_ROOT / "schemas" / "profile.schema.json")
    perrs = validate_with_schema(profile, profile_schema)
    perrs.extend(validate_profile_semantics(profile, module_index, corridor_index))
    if perrs:
        print("PROFILE FAIL:")
        for e in perrs:
            print("  -", e)
        return 2

    # Prepare output
    out_dir = pathlib.Path(args.out)
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    bundle_dir = out_dir / "bundle"
    if bundle_dir.exists():
        shutil.rmtree(bundle_dir)
    bundle_dir.mkdir(parents=True, exist_ok=True)

    # Copy module directories into bundle
    for m in profile.get("modules", []):
        if not isinstance(m, dict):
            print("ERROR: invalid profile.modules entry")
            return 2
        mid = m.get("module_id")
        resolved = resolve_module_by_id(mid, module_index)
        if not resolved:
            print("ERROR: cannot build; missing module", mid)
            return 2
        found, _ = resolved
        target = bundle_dir / found.relative_to(REPO_ROOT)
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copytree(found, target)

    # Apply overlays (zone build only)
    if zone and zone_path:
        for ov in zone.get("overlays", []) or []:
            for patch in ov.get("patches", []) or []:
                pp = (zone_path.parent / patch).resolve()
                if not pp.exists():
                    print(f"ERROR: overlay patch not found: {pp}")
                    return 2
                try:
                    subprocess.run(
                        [
                            "git",
                            "apply",
                            "--unsafe-paths",
                            "--directory",
                            str(bundle_dir),
                            str(pp),
                        ],
                        check=True,
                        stdout=subprocess.PIPE,
                        stderr=subprocess.PIPE,
                        text=True,
                    )
                except subprocess.CalledProcessError as ex:
                    print("ERROR: failed to apply overlay patch:", pp)
                    if ex.stderr:
                        print(ex.stderr.strip())
                    return 2

    # Render templated artifacts (in-place in the bundle)
    strict_render = bool(getattr(args, "strict_render", False))
    if not getattr(args, "no_render", False):
        base_ctx: Dict[str, Any] = {}
        if zone:
            base_ctx.update(
                {
                    "zone_id": zone.get("zone_id"),
                    "zone_name": zone.get("zone_name"),
                    "jurisdiction_id": zone.get("jurisdiction_id"),
                }
            )
        for m in profile.get("modules", []):
            if not isinstance(m, dict):
                continue
            mid = m.get("module_id")
            resolved = resolve_module_by_id(mid, module_index)
            if not resolved:
                continue
            mdir, mdata = resolved
            bundle_mod_dir = bundle_dir / mdir.relative_to(REPO_ROOT)

            # Resolve parameter values: defaults -> profile params -> zone params_overrides
            params: Dict[str, Any] = {}
            for pname, pspec in (mdata.get("parameters") or {}).items():
                if isinstance(pspec, dict) and "default" in pspec:
                    params[pname] = pspec.get("default")
            params.update(m.get("params", {}) or {})
            if zone:
                overrides = (zone.get("params_overrides") or {}).get(mid) or {}
                if isinstance(overrides, dict):
                    params.update(overrides)

            ctx = build_template_context({**base_ctx, **params})
            try:
                render_templates_in_dir(bundle_mod_dir, ctx, strict=strict_render)
            except KeyError as ex:
                print(f"ERROR: templating failed for {mid}: {ex}")
                return 2

    # Write resolved profile + optional lockfile
    (bundle_dir / "profile.resolved.yaml").write_text(
        yaml.safe_dump(profile, sort_keys=False), encoding="utf-8"
    )

    if zone and zone_path:
        # Emit a lockfile alongside the bundle for reproducible deployments
        lock_out = out_dir / "stack.lock"
        try:
            zone_arg = str(zone_path.relative_to(REPO_ROOT))
        except Exception:
            zone_arg = str(zone_path)
        try:
            out_arg = str(lock_out.relative_to(REPO_ROOT))
        except Exception:
            out_arg = str(lock_out)
        rc = cmd_lock(argparse.Namespace(zone=zone_arg, out=out_arg))
        if rc != 0:
            return rc

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
        "stack_spec_version": STACK_SPEC_VERSION,
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
        # best effort: locate corridor module by corridor_id
        trust_hash = ""
        rot_hash = ""
        manifest_hash = ""
        vc_hash = ""
        signers: List[str] = []

        agreement_hashes: List[str] = []
        agreement_signers: List[str] = []
        activated: bool | None = None

        for mdir in find_modules(REPO_ROOT):
            mdata = load_yaml(mdir / "module.yaml")
            if mdata.get("kind") != "corridor":
                continue

            cy = mdir / "corridor.yaml"
            if not cy.exists():
                continue

            c = load_yaml(cy)
            if c.get("corridor_id") != cid:
                continue

            # Digests for security artifacts + corridor manifest
            ta = mdir / (c.get("trust_anchors_path") or "trust-anchors.yaml")
            kr = mdir / (c.get("key_rotation_path") or "key-rotation.yaml")
            trust_hash = sha256_file(ta) if ta.exists() else "MISSING"
            rot_hash = sha256_file(kr) if kr.exists() else "MISSING"
            manifest_hash = sha256_file(cy) if cy.exists() else "MISSING"

            # Corridor Definition VC (required in v0.3+; optional in older stacks)
            vc_rel = (c.get("definition_vc_path") or "").strip()
            vc_path = (mdir / vc_rel) if vc_rel else None
            if vc_path and vc_path.exists():
                vc_hash = sha256_file(vc_path)
                try:
                    vcj = load_json(vc_path)
                    pr = vcj.get("proof")
                    if isinstance(pr, dict):
                        signers = [str(pr.get("verificationMethod") or "")]
                    elif isinstance(pr, list):
                        signers = [str(x.get("verificationMethod") or "") for x in pr if isinstance(x, dict)]
                    signers = sorted({s.split("#", 1)[0] for s in signers if s})
                except Exception:
                    signers = []
            else:
                vc_hash = "MISSING" if vc_rel else ""

            # Corridor Agreement VC(s) (optional)
            for rel in _agreement_paths(c):
                ap = mdir / rel
                if ap.exists():
                    agreement_hashes.append(sha256_file(ap))
                    try:
                        avcj = load_json(ap)
                        pr = avcj.get("proof")
                        if isinstance(pr, dict):
                            agreement_signers.append(str(pr.get("verificationMethod") or ""))
                        elif isinstance(pr, list):
                            agreement_signers.extend([str(x.get("verificationMethod") or "") for x in pr if isinstance(x, dict)])
                    except Exception:
                        pass
                else:
                    agreement_hashes.append("MISSING")

            agreement_signers = sorted({s.split("#", 1)[0] for s in agreement_signers if s})

            # Activation status (thresholds) — when agreement VC(s) exist
            asummary: Dict[str, Any] = {}
            aerrs: List[str] = []
            if agreement_hashes:
                try:
                    aerrs, asummary = corridor_agreement_summary(mdir)
                    activated = bool(asummary.get("activated")) and not aerrs
                    # Prefer signed_parties if available (participant-specific agreement VCs)
                    signed_parties = asummary.get("signed_parties") or []
                    if signed_parties:
                        agreement_signers = list(signed_parties)
                except Exception:
                    activated = False

            break

        entry: Dict[str, Any] = {
            "corridor_id": cid,
            "corridor_manifest_sha256": manifest_hash or "",
            "trust_anchors_sha256": trust_hash or "",
            "key_rotation_sha256": rot_hash or "",
            "corridor_definition_vc_sha256": vc_hash or "",
            "corridor_definition_signers": signers,
        }

        if agreement_hashes:
            entry["corridor_agreement_vc_sha256"] = agreement_hashes
            entry["corridor_agreement_signers"] = agreement_signers
            entry["corridor_activated"] = bool(activated) if activated is not None else False
            # Optional: content-addressed agreement set digest + per-file payload hashes
            aset = (asummary or {}).get("agreement_set_sha256")
            if aset:
                entry["corridor_agreement_set_sha256"] = aset
            ap = (asummary or {}).get("agreement_payload_sha256_by_path")
            if isinstance(ap, dict) and ap:
                entry["corridor_agreement_payload_sha256_by_path"] = ap
            blockers = (asummary or {}).get("blocked_parties") or []
            if isinstance(blockers, list) and blockers:
                entry["corridor_activation_blockers"] = [
                    f"{b.get('id')}:{b.get('commitment')}" for b in blockers if isinstance(b, dict)
                ]

        lock["corridors"].append(entry)


    out_path = pathlib.Path(args.out) if args.out else zone_path.parent / (zone.get("lockfile_path") or "stack.lock")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(yaml.safe_dump(lock, sort_keys=False), encoding="utf-8")
    print("LOCK OK: wrote", out_path)
    return 0

def cmd_check_coverage(args: argparse.Namespace) -> int:
    """Check that all MUST/SHALL clauses in legal Akoma sources are mapped.

    This is the CLI equivalent of tests/test_policy_to_code_completeness.py.
    """
    policy_schema = schema_validator(REPO_ROOT / "schemas" / "policy-to-code.schema.json")
    module_index = build_module_index(REPO_ROOT)

    # Determine scope: all modules, a profile, or a zone.
    module_ids: List[str] = []
    if getattr(args, "zone", ""):
        zone_path = pathlib.Path(args.zone)
        if not zone_path.is_absolute():
            zone_path = REPO_ROOT / zone_path
        zone = load_yaml(zone_path)
        pid = (zone.get("profile") or {}).get("profile_id")
        if not pid:
            print("ERROR: zone missing profile.profile_id")
            return 2
        # resolve profile
        prof_path = build_profile_index(REPO_ROOT).get(pid)
        if not prof_path:
            print("ERROR: profile not found for", pid)
            return 2
        profile = load_yaml(prof_path)
        module_ids = [m.get("module_id") for m in (profile.get("modules") or []) if isinstance(m, dict) and m.get("module_id")]
    elif getattr(args, "profile", ""):
        prof_path = pathlib.Path(args.profile)
        if not prof_path.is_absolute():
            prof_path = REPO_ROOT / prof_path
        profile = load_yaml(prof_path)
        module_ids = [m.get("module_id") for m in (profile.get("modules") or []) if isinstance(m, dict) and m.get("module_id")]
    else:
        module_ids = list(module_index.keys())

    must_re = re.compile(r"\b(MUST|SHALL)\b")

    def extract_must_eids(xml_path: pathlib.Path) -> List[str]:
        doc = etree.parse(str(xml_path))
        eids: List[str] = []
        for el in doc.xpath('//*[@eId]'):
            text = " ".join([t.strip() for t in el.xpath(".//text()") if str(t).strip()])
            if must_re.search(text):
                eids.append(str(el.get("eId")))
        return eids

    def map_covers_eid(map_data: Any, eid: str) -> bool:
        if not isinstance(map_data, list):
            return False
        for entry in map_data:
            if not isinstance(entry, dict):
                continue
            for lr in entry.get("legal_refs", []) or []:
                if isinstance(lr, dict) and lr.get("eId") == eid:
                    return True
                if isinstance(lr, str) and eid in lr:
                    return True
        return False

    ok = True
    for mid in module_ids:
        if mid not in module_index:
            # ignore missing in scope (e.g., stale profile)
            continue
        mdir, _ = module_index[mid]
        akn_dir = mdir / "src" / "akn"
        if not akn_dir.exists():
            continue
        must_eids: List[str] = []
        for xml in akn_dir.rglob("*.xml"):
            must_eids.extend(extract_must_eids(xml))
        if not must_eids:
            continue

        map_path = mdir / "src" / "policy-to-code" / "map.yaml"
        if not map_path.exists():
            ok = False
            print(f"FAIL: {mid} has MUST/SHALL clauses but no policy-to-code map at {map_path.relative_to(mdir)}")
            continue
        map_data = load_yaml(map_path)
        map_errors = validate_with_schema(map_data, policy_schema)
        if map_errors:
            ok = False
            print(f"FAIL: {mid} policy-to-code map schema errors:")
            for e in map_errors:
                print("  -", e)
            continue

        missing = [eid for eid in must_eids if not map_covers_eid(map_data, eid)]
        if missing:
            ok = False
            print(f"FAIL: {mid} missing policy-to-code entries for eIds: {missing}")

    if ok:
        print("OK: policy-to-code coverage checks passed")
        return 0
    return 2

def cmd_diff(args: argparse.Namespace) -> int:
    """Diff two stack.lock files for upgrade impact analysis."""
    a_path = pathlib.Path(args.a)
    b_path = pathlib.Path(args.b)
    if not a_path.is_absolute():
        a_path = REPO_ROOT / a_path
    if not b_path.is_absolute():
        b_path = REPO_ROOT / b_path
    a = load_yaml(a_path)
    b = load_yaml(b_path)

    a_mods = {m["module_id"]: m for m in (a.get("modules") or []) if isinstance(m, dict) and m.get("module_id")}
    b_mods = {m["module_id"]: m for m in (b.get("modules") or []) if isinstance(m, dict) and m.get("module_id")}

    added = sorted(set(b_mods) - set(a_mods))
    removed = sorted(set(a_mods) - set(b_mods))
    common = sorted(set(a_mods) & set(b_mods))

    changed: List[str] = []
    for mid in common:
        am = a_mods[mid]
        bm = b_mods[mid]
        fields = ["version", "variant", "manifest_sha256", "content_sha256"]
        for f in fields:
            if str(am.get(f, "")) != str(bm.get(f, "")):
                changed.append(mid)
                break
        else:
            # params diff
            if (am.get("params") or {}) != (bm.get("params") or {}):
                changed.append(mid)

    print(f"Diff: {a_path} -> {b_path}")
    if added:
        print("\nAdded modules:")
        for mid in added:
            print("  +", mid)
    if removed:
        print("\nRemoved modules:")
        for mid in removed:
            print("  -", mid)
    if changed:
        print("\nChanged modules:")
        for mid in sorted(set(changed)):
            am = a_mods[mid]
            bm = b_mods[mid]
            print(f"  ~ {mid}")
            for f in ["version", "variant", "manifest_sha256", "content_sha256"]:
                av = str(am.get(f, ""))
                bv = str(bm.get(f, ""))
                if av != bv:
                    print(f"      {f}: {av} -> {bv}")
            if (am.get("params") or {}) != (bm.get("params") or {}):
                print("      params: changed")

    # overlays + corridors (best effort)
    if (a.get("overlays") or []) != (b.get("overlays") or []):
        print("\nOverlays: changed")
    if (a.get("corridors") or []) != (b.get("corridors") or []):
        print("\nCorridors: changed")

    return 0

def cmd_publish(args: argparse.Namespace) -> int:
    """Publish rendered artifacts (Akoma -> HTML/PDF) from a repo or bundle directory."""
    root = pathlib.Path(args.path)
    if not root.is_absolute():
        root = REPO_ROOT / root
    if not root.exists():
        print(f"ERROR: path not found: {root}")
        return 2

    out_dir = pathlib.Path(args.out_dir)
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir
    out_dir.mkdir(parents=True, exist_ok=True)

    from tools.akoma.render import render_html, render_pdf_from_html, render_pdf_text

    xslt_path = REPO_ROOT / "tools" / "akoma" / "xslt" / "akn2html.xsl"
    if not xslt_path.exists():
        print(f"ERROR: missing XSLT at {xslt_path}")
        return 2

    xml_files = sorted({p for p in root.glob("**/src/akn/**/*.xml") if p.is_file()})
    if not xml_files:
        print("No Akoma XML files found under", root)
        return 0

    wrote = 0
    for xml in xml_files:
        try:
            rel = xml.relative_to(root)
        except Exception:
            rel = pathlib.Path(xml.name)
        html_out = out_dir / rel.with_suffix(".html")
        html_out.parent.mkdir(parents=True, exist_ok=True)
        try:
            render_html(xml, xslt_path, html_out)
        except Exception as ex:
            print(f"WARN: failed to render {xml}: {ex}")
            continue
        wrote += 1
        if args.pdf:
            pdf_out = out_dir / rel.with_suffix(".pdf")
            pdf_out.parent.mkdir(parents=True, exist_ok=True)
            if not render_pdf_from_html(html_out, pdf_out):
                render_pdf_text(xml, pdf_out)
        
    print(f"PUBLISH OK: rendered {wrote} document(s) to {out_dir}")
    return 0



# ---------------------------------------------------------------------------
# Verifiable Credentials (VC) and corridor cryptographic verification
# ---------------------------------------------------------------------------

def _as_list(x: Any) -> List[Any]:
    if x is None:
        return []
    if isinstance(x, list):
        return x
    return [x]

def verify_corridor_definition_vc(module_dir: pathlib.Path) -> List[str]:
    """
    Verify that a corridor module is cryptographically bound:
    - corridor.yaml validates
    - trust-anchors/key-rotation validate
    - corridor definition VC verifies cryptographically
    - VC subject binds to sha256(corridor.yaml / trust-anchors.yaml / key-rotation.yaml)
    - VC signer(s) are authorized by trust-anchors.yaml for `corridor_definition`
    """
    errs: List[str] = []
    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        return ["Missing corridor.yaml"]

    corridor_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.schema.json")
    c = load_yaml(corridor_path)
    errs.extend([f"corridor.yaml: {e}" for e in validate_with_schema(c, corridor_schema)])

    # Security artifacts
    ta_path = module_dir / (c.get("trust_anchors_path") or "trust-anchors.yaml")
    kr_path = module_dir / (c.get("key_rotation_path") or "key-rotation.yaml")
    if not ta_path.exists():
        errs.append(f"Missing {ta_path.name}")
        return errs
    if not kr_path.exists():
        errs.append(f"Missing {kr_path.name}")
        return errs

    trust_schema = schema_validator(REPO_ROOT / "schemas" / "trust-anchors.schema.json")
    keyrot_schema = schema_validator(REPO_ROOT / "schemas" / "key-rotation.schema.json")
    errs.extend([f"{ta_path.name}: {e}" for e in validate_with_schema(load_yaml(ta_path), trust_schema)])
    errs.extend([f"{kr_path.name}: {e}" for e in validate_with_schema(load_yaml(kr_path), keyrot_schema)])

    # Definition VC
    vc_rel = (c.get("definition_vc_path") or "").strip()
    if not vc_rel:
        errs.append("corridor.yaml: missing definition_vc_path")
        return errs
    vc_path = module_dir / vc_rel
    if not vc_path.exists():
        errs.append(f"Missing {vc_rel}")
        return errs

    vc_schema = schema_validator(REPO_ROOT / "schemas" / "vc.corridor-definition.schema.json")
    vcj = load_json(vc_path)
    errs.extend([f"{vc_rel}: {e}" for e in validate_with_schema(vcj, vc_schema)])

    # Cryptographic verification (supports did:key offline)
    try:
        from tools.vc import verify_credential  # type: ignore
        results = verify_credential(vcj)
        ok_methods = [r.verification_method for r in results if r.ok]
        bad = [r for r in results if not r.ok]
        if not ok_methods:
            errs.append(f"{vc_rel}: no valid proofs")
        for r in bad:
            errs.append(f"{vc_rel}: invalid proof for {r.verification_method}: {r.error}")
    except Exception as ex:
        errs.append(f"{vc_rel}: VC verification error: {ex}")
        ok_methods = []

    # Binding checks: sha256
    subj = (vcj.get("credentialSubject") or {})
    if isinstance(subj, dict):
        art = subj.get("artifacts") or {}
        def get_hash(name: str) -> str:
            try:
                return str(((art.get(name) or {}).get("sha256") or "")).strip()
            except Exception:
                return ""
        expected_manifest = get_hash("corridor_manifest")
        expected_ta = get_hash("trust_anchors")
        expected_kr = get_hash("key_rotation")

        if expected_manifest and expected_manifest != sha256_file(corridor_path):
            errs.append(f"{vc_rel}: corridor_manifest.sha256 mismatch (VC vs file)")
        if expected_ta and expected_ta != sha256_file(ta_path):
            errs.append(f"{vc_rel}: trust_anchors.sha256 mismatch (VC vs file)")
        if expected_kr and expected_kr != sha256_file(kr_path):
            errs.append(f"{vc_rel}: key_rotation.sha256 mismatch (VC vs file)")

        vc_cid = subj.get("corridor_id")
        if vc_cid and vc_cid != c.get("corridor_id"):
            errs.append(f"{vc_rel}: corridor_id mismatch (VC={vc_cid} vs manifest={c.get('corridor_id')})")

    # Authorization checks against trust anchors
    try:
        ta = load_yaml(ta_path)
        anchors = ta.get("trust_anchors") or []
        allowed = set()
        for a in anchors:
            if not isinstance(a, dict):
                continue
            if "corridor_definition" in (a.get("allowed_attestations") or []):
                allowed.add(str(a.get("identifier") or "").split("#", 1)[0])
        for vm in ok_methods:
            did = str(vm).split("#", 1)[0]
            if allowed and did not in allowed:
                errs.append(f"{vc_rel}: signer {did} is not authorized for corridor_definition in trust-anchors.yaml")
    except Exception:
        # best-effort; schema validation already covers shape
        pass

    return errs


def _agreement_paths(c: Any) -> List[str]:
    """Return agreement VC path(s) from corridor.yaml supporting string or list forms."""
    if c is None or not isinstance(c, dict):
        return []
    av = c.get("agreement_vc_path")
    if not av:
        return []
    if isinstance(av, str):
        s = av.strip()
        return [s] if s else []
    if isinstance(av, list):
        out: List[str] = []
        for item in av:
            if isinstance(item, str) and item.strip():
                out.append(item.strip())
        return out
    return []

def corridor_agreement_summary(module_dir: pathlib.Path) -> Tuple[List[str], Dict[str, Any]]:
    """Compute Corridor Agreement VC status.

    Returns (errors, summary) where summary is a machine-readable dict with:
    - corridor_id
    - has_agreement
    - agreement_paths
    - definition_payload_sha256
    - participants (as provided in the VC)
    - agreement_pattern ('party-specific' or 'multi-signer')
    - signed_parties_all (unique DIDs that produced at least one valid proof)
    - signed_parties (unique DIDs counted for threshold evaluation; commitment-aware)
    - party_commitments (participant DID -> commitment verb, when party-specific)
    - blocked_parties (list of parties whose commitment blocks activation)
    - thresholds (per-role evaluation)
    - activated (True when threshold rules are satisfied and no verification errors exist)

    Notes:
    - If no agreement VC is configured, this returns (corridor.yaml schema errors, {{has_agreement: False}}).
    - Participant-specific agreement VCs MAY set credentialSubject.party. When present, validators:
        - require the party.id DID to have signed that VC,
        - require that each party appears at most once across agreement_vc_path (status lock),
        - interpret credentialSubject.commitment as that party's current status.
    - Activation thresholds are evaluated over *affirmative* commitments only, using
      activation.accept_commitments (default: ['agree']).
    """

    errs: List[str] = []
    corridor_path = module_dir / 'corridor.yaml'
    if not corridor_path.exists():
        return (['Missing corridor.yaml'], {'has_agreement': False, 'activated': False})

    corridor_schema = schema_validator(REPO_ROOT / 'schemas' / 'corridor.schema.json')
    c = load_yaml(corridor_path)
    errs.extend([f"corridor.yaml: {e}" for e in validate_with_schema(c, corridor_schema)])

    agreement_paths = _agreement_paths(c)
    summary: Dict[str, Any] = {
        'corridor_id': c.get('corridor_id'),
        'has_agreement': bool(agreement_paths),
        'agreement_paths': agreement_paths,
        'activated': False,
    }

    if not agreement_paths:
        return (errs, summary)

    # Security artifacts (trust anchors) for signer authorization
    ta_path = module_dir / (c.get('trust_anchors_path') or 'trust-anchors.yaml')
    if not ta_path.exists():
        errs.append(f"Missing {ta_path.name}")
        return (errs, summary)

    # Corridor Definition VC binding: hash the canonical payload excluding proof
    def_rel = (c.get('definition_vc_path') or '').strip()
    if not def_rel:
        errs.append('corridor.yaml: missing definition_vc_path')
        return (errs, summary)
    def_path = module_dir / def_rel
    if not def_path.exists():
        errs.append(f"Missing {def_rel}")
        return (errs, summary)

    try:
        def_vcj = load_json(def_path)
        from tools.vc import signing_input  # type: ignore
        def_payload_sha256 = sha256_bytes(signing_input(def_vcj))
        def_vc_id = str(def_vcj.get('id') or '').strip()
        summary['definition_payload_sha256'] = def_payload_sha256
        if def_vc_id:
            summary['definition_vc_id'] = def_vc_id
    except Exception as ex:
        errs.append(f"{def_rel}: unable to load Corridor Definition VC for agreement binding: {ex}")
        return (errs, summary)

    # Load trust anchors for corridor_agreement authorization
    allowed_agreement = set()
    try:
        ta = load_yaml(ta_path)
        for a in (ta.get('trust_anchors') or []):
            if not isinstance(a, dict):
                continue
            if 'corridor_agreement' in (a.get('allowed_attestations') or []):
                ident = str(a.get('identifier') or '').split('#', 1)[0]
                if ident:
                    allowed_agreement.add(ident)
    except Exception:
        pass

    if not allowed_agreement:
        errs.append(
            f"{ta_path.name}: no trust anchors authorize corridor_agreement (allowed_attestations includes 'corridor_agreement')"
        )

    base_subject: Dict[str, Any] | None = None
    agreement_pattern: str | None = None  # 'party-specific' or 'multi-signer'
    all_valid_signers: set[str] = set()
    signed_parties_all: set[str] = set()
    agreement_payload_sha256_by_path: Dict[str, str] = {}

    # Participant-specific state lock + commitments
    seen_party_ids: set[str] = set()
    party_by_vc: Dict[str, str] = {}
    party_role_by_vc: Dict[str, str] = {}
    party_commitment_by_party: Dict[str, str] = {}
    party_path_by_party: Dict[str, str] = {}

    vc_schema = schema_validator(REPO_ROOT / 'schemas' / 'vc.corridor-agreement.schema.json')

    for rel in agreement_paths:
        vc_path = module_dir / rel
        if not vc_path.exists():
            errs.append(f"Missing {rel}")
            continue

        vcj = load_json(vc_path)
        # Deterministic payload hash (excludes proofs)
        try:
            from tools.vc import signing_input  # type: ignore
            agreement_payload_sha256_by_path[rel] = sha256_bytes(signing_input(vcj))
        except Exception as ex:
            errs.append(f"{rel}: unable to compute agreement payload sha256: {ex}")
        errs.extend([f"{rel}: {e}" for e in validate_with_schema(vcj, vc_schema)])

        # Cryptographic verification
        try:
            from tools.vc import verify_credential  # type: ignore
            results = verify_credential(vcj)
            ok_methods = [r.verification_method for r in results if r.ok]
            bad = [r for r in results if not r.ok]

            if not ok_methods:
                errs.append(f"{rel}: no valid proofs")
            for r in bad:
                errs.append(f"{rel}: invalid proof for {r.verification_method}: {r.error}")

            ok_dids = [str(vm).split('#', 1)[0] for vm in ok_methods if vm]
            for did in ok_dids:
                all_valid_signers.add(did)
        except Exception as ex:
            errs.append(f"{rel}: VC verification error: {ex}")
            ok_dids = []

        subj = vcj.get('credentialSubject') or {}
        if not isinstance(subj, dict):
            errs.append(f"{rel}: credentialSubject must be an object")
            continue

        issuer_did = str(vcj.get('issuer') or '').split('#', 1)[0]
        commitment = str(subj.get('commitment') or '').strip() or 'agree'
        commitment_norm = commitment.strip().lower()

        # Corridor id binding
        vc_cid = subj.get('corridor_id')
        if vc_cid and vc_cid != c.get('corridor_id'):
            errs.append(f"{rel}: corridor_id mismatch (VC={vc_cid} vs manifest={c.get('corridor_id')})")

        # Definition VC binding
        expected_hash = str(subj.get('definition_payload_sha256') or '').strip()
        if expected_hash and expected_hash != def_payload_sha256:
            errs.append(f"{rel}: definition_payload_sha256 mismatch (VC vs corridor definition payload)")

        expected_def_id = str(subj.get('definition_vc_id') or '').strip()
        if expected_def_id and def_vc_id and expected_def_id != def_vc_id:
            errs.append(f"{rel}: definition_vc_id mismatch (VC={expected_def_id} vs definition={def_vc_id})")

        # Authorization checks (best-effort; for offline did:key, allowed list should include did:key)
        if allowed_agreement:
            for did in ok_dids:
                if did and did not in allowed_agreement:
                    errs.append(f"{rel}: signer {did} is not authorized for corridor_agreement in {ta_path.name}")

        # Participant-specific party semantics
        party_obj = subj.get('party')
        if party_obj is not None:
            if agreement_pattern == 'multi-signer':
                errs.append(
                    f"agreement: mixed agreement patterns (party-specific + multi-signer) in agreement_vc_path; choose one pattern"
                )
            agreement_pattern = agreement_pattern or 'party-specific'

            if not isinstance(party_obj, dict):
                errs.append(f"{rel}: credentialSubject.party must be an object")
            else:
                party_id = str(party_obj.get('id') or '').split('#', 1)[0]
                party_role = str(party_obj.get('role') or '').strip()

                if not party_id:
                    errs.append(f"{rel}: credentialSubject.party.id is required for participant-specific agreement VCs")
                else:
                    if party_id in seen_party_ids:
                        errs.append(
                            f"agreement: duplicate party {party_id} across agreement_vc_path (status lock violation); include only one current VC per party"
                        )
                    else:
                        seen_party_ids.add(party_id)

                    party_by_vc[rel] = party_id
                    if party_role:
                        party_role_by_vc[rel] = party_role

                    party_commitment_by_party[party_id] = commitment_norm
                    party_path_by_party[party_id] = rel

                    # For participant-specific agreement VCs, issuer SHOULD equal party.id (keeps provenance clear)
                    if issuer_did and issuer_did != party_id:
                        errs.append(
                            f"{rel}: issuer {issuer_did} must match credentialSubject.party.id {party_id} for participant-specific agreement VCs"
                        )

                    if party_id and party_id not in ok_dids:
                        errs.append(f"{rel}: party {party_id} did not sign this agreement VC")
                    if party_id and party_id in ok_dids:
                        signed_parties_all.add(party_id)

        else:
            if agreement_pattern == 'party-specific':
                errs.append(
                    f"agreement: mixed agreement patterns (party-specific + multi-signer) in agreement_vc_path; choose one pattern"
                )
            agreement_pattern = agreement_pattern or 'multi-signer'
            # Legacy/multi-signer agreement: count all signer DIDs.
            for did in ok_dids:
                if did:
                    signed_parties_all.add(did)

        # Ensure multiple agreement VC subjects do not conflict (base fields)
        if base_subject is None:
            base_subject = subj
        else:
            keys = ('corridor_id', 'definition_payload_sha256', 'participants', 'activation', 'terms')
            # In multi-signer mode, commitment is a global property and must be consistent.
            if agreement_pattern == 'multi-signer':
                keys = keys + ('commitment',)
            for k in keys:
                if subj.get(k) != base_subject.get(k):
                    errs.append(f"{rel}: agreement VC subject.{k} does not match other agreement VC(s)")
                    break

            # definition_vc_id is optional; only enforce when both are present
            a_def = str(base_subject.get('definition_vc_id') or '').strip()
            b_def = str(subj.get('definition_vc_id') or '').strip()
            if a_def and b_def and a_def != b_def:
                errs.append(f"{rel}: agreement VC subject.definition_vc_id does not match other agreement VC(s)")

    summary['agreement_pattern'] = agreement_pattern or 'unknown'
    summary['signed_parties_all'] = sorted(signed_parties_all)
    if agreement_payload_sha256_by_path:
        summary['agreement_payload_sha256_by_path'] = {
            k: agreement_payload_sha256_by_path[k] for k in sorted(agreement_payload_sha256_by_path.keys())
        }
        digest_obj = {
            'corridor_id': summary.get('corridor_id'),
            'definition_payload_sha256': summary.get('definition_payload_sha256'),
            'agreement_payload_sha256': sorted(agreement_payload_sha256_by_path.values()),
        }
        digest_bytes = json.dumps(digest_obj, sort_keys=True, separators=(',', ':')).encode('utf-8')
        summary['agreement_set_sha256'] = sha256_bytes(digest_bytes)

    # If we never got a usable subject, stop here
    if base_subject is None:
        return (errs, summary)

    participants = base_subject.get('participants') or []
    summary['participants'] = participants

    role_by_did: Dict[str, str] = {}
    if isinstance(participants, list):
        for p in participants:
            if not isinstance(p, dict):
                continue
            pid = str(p.get('id') or '').split('#', 1)[0]
            role = str(p.get('role') or '').strip()
            if pid and role:
                role_by_did[pid] = role

    # If corridor.yaml participants are specified (non-empty), require exact match with VC participants.
    corridor_participants = c.get('participants') or []
    if isinstance(corridor_participants, list) and corridor_participants:
        cset = {str(x).split('#', 1)[0] for x in corridor_participants if x}
        vset = set(role_by_did.keys())
        if cset != vset:
            missing = sorted(cset - vset)
            extra = sorted(vset - cset)
            if missing:
                errs.append(f"agreement: corridor.yaml participants not present in agreement VC participants: {missing}")
            if extra:
                errs.append(f"agreement: agreement VC participants not present in corridor.yaml participants: {extra}")

    # All valid signers MUST be listed as participants (keeps activation semantics unambiguous)
    for did in sorted(all_valid_signers):
        if role_by_did and did not in role_by_did:
            errs.append(f"agreement: signer {did} is not listed in credentialSubject.participants")

    # If party fields are present, ensure party role matches participant list
    for rel, party_id in party_by_vc.items():
        if not party_id:
            continue
        expected_role = role_by_did.get(party_id)
        declared_role = party_role_by_vc.get(rel, '')
        if expected_role and declared_role and expected_role != declared_role:
            errs.append(
                f"{rel}: party role mismatch for {party_id} (party.role={declared_role} vs participants.role={expected_role})"
            )
        if role_by_did and party_id not in role_by_did:
            errs.append(f"{rel}: party {party_id} is not listed in credentialSubject.participants")

    activation = base_subject.get('activation') or {}
    thresholds = []
    accept_commitments = ['agree']
    if isinstance(activation, dict):
        thresholds = activation.get('thresholds') or []
        ac = activation.get('accept_commitments')
        if isinstance(ac, list):
            norm = [str(x).strip().lower() for x in ac if str(x).strip()]
            if norm:
                accept_commitments = norm

    if not isinstance(thresholds, list) or not thresholds:
        errs.append('agreement: missing activation.thresholds')
        return (errs, summary)

    accept_set = set(accept_commitments)

    # Commitment-aware signer set (what counts toward thresholds)
    signed_parties: set[str] = set()
    blocked_parties: List[Dict[str, Any]] = []

    if (agreement_pattern or '') == 'party-specific':
        # Any non-affirmative party commitment blocks activation (even if thresholds would be satisfied).
        for party_id, comm in sorted(party_commitment_by_party.items()):
            if comm not in accept_set:
                blocked_parties.append(
                    {'id': party_id, 'commitment': comm, 'path': party_path_by_party.get(party_id, '')}
                )
                errs.append(
                    f"agreement: party {party_id} commitment '{comm}' blocks activation (accept_commitments={accept_commitments})"
                )
        # Count only affirmative commitments toward thresholds.
        for did in signed_parties_all:
            comm = party_commitment_by_party.get(did, 'agree')
            if comm in accept_set:
                signed_parties.add(did)

        summary['party_commitments'] = {k: v for k, v in sorted(party_commitment_by_party.items())}
    else:
        # Multi-signer agreement: treat subject.commitment as global (default 'agree').
        global_comm = str(base_subject.get('commitment') or '').strip().lower() or 'agree'
        summary['commitment'] = global_comm
        if global_comm not in accept_set:
            errs.append(
                f"agreement: commitment '{global_comm}' blocks activation (accept_commitments={accept_commitments})"
            )
        else:
            signed_parties = set(signed_parties_all)

    summary['blocked_parties'] = blocked_parties

    # Threshold evaluation
    threshold_summaries: List[Dict[str, Any]] = []
    activated = True

    for t in thresholds:
        if not isinstance(t, dict):
            continue
        role = str(t.get('role') or '').strip()
        required = t.get('required')
        of_val = t.get('of') if 'of' in t else None

        try:
            req_n = int(required)
        except Exception:
            errs.append(f"agreement: invalid threshold.required for role '{role}'")
            activated = False
            continue

        participants_with_role = [d for d, r in role_by_did.items() if r == role]
        signed_with_role = [d for d in signed_parties if role_by_did.get(d) == role]

        if of_val is not None:
            try:
                of_n = int(of_val)
                if of_n != len(participants_with_role):
                    errs.append(
                        f"agreement: threshold 'of' mismatch for role '{role}' (of={of_n} vs participants={len(participants_with_role)})"
                    )
                    activated = False
            except Exception:
                errs.append(f"agreement: invalid threshold.of for role '{role}'")
                activated = False

        if len(signed_with_role) < req_n:
            errs.append(
                f"agreement: activation threshold not met for role '{role}' ({len(signed_with_role)} signed, requires {req_n})"
            )
            activated = False

        threshold_summaries.append(
            {
                'role': role,
                'required': req_n,
                'of': int(of_val) if of_val is not None else None,
                'participants': len(participants_with_role),
                'signed': len(signed_with_role),
                'satisfied': len(signed_with_role) >= req_n,
            }
        )

    summary['signed_parties'] = sorted(signed_parties)
    summary['thresholds'] = threshold_summaries

    signed_by_role: Dict[str, List[str]] = {}
    for did in signed_parties:
        role = role_by_did.get(did)
        if role:
            signed_by_role.setdefault(role, []).append(did)
    summary['signed_by_role'] = {r: sorted(dids) for r, dids in signed_by_role.items()}

    summary['activated'] = bool(agreement_paths) and activated and (len(errs) == 0)

    return (errs, summary)


def verify_corridor_agreement_vc(module_dir: pathlib.Path) -> List[str]:
    """Verify Corridor Agreement VC(s) if configured.

    This is the strict CLI/test-facing validator wrapper around :func:`corridor_agreement_summary`.

    If `agreement_vc_path` is omitted, returns only corridor.yaml schema errors (if any).
    """

    errs, _summary = corridor_agreement_summary(module_dir)
    return errs



def cmd_vc_keygen(args: argparse.Namespace) -> int:
    """Generate an Ed25519 OKP JWK (did:key compatible)."""
    from tools.vc import generate_ed25519_jwk, public_jwk_from_private_jwk, b64url_decode, did_key_from_ed25519_public_key  # type: ignore

    out_path = pathlib.Path(args.out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)

    jwk = generate_ed25519_jwk(kid=getattr(args, "kid", "key-1") or "key-1")
    out_path.write_text(json.dumps(jwk, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # Optional public-only JWK output
    pub_out = getattr(args, "public_out", "") or ""
    if pub_out:
        pub_path = pathlib.Path(pub_out)
        if not pub_path.is_absolute():
            pub_path = REPO_ROOT / pub_path
        pub_path.parent.mkdir(parents=True, exist_ok=True)
        pub_path.write_text(json.dumps(public_jwk_from_private_jwk(jwk), indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # Print derived did:key for convenience
    pub_bytes = b64url_decode(jwk["x"])
    did = did_key_from_ed25519_public_key(pub_bytes)
    print(did)
    return 0

def cmd_vc_sign(args: argparse.Namespace) -> int:
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    key_path = pathlib.Path(args.key)
    if not key_path.is_absolute():
        key_path = REPO_ROOT / key_path

    vcj = load_json(cred_path)
    jwk = load_json(key_path)

    from tools.vc import load_ed25519_private_key_from_jwk, add_ed25519_proof  # type: ignore
    priv, did = load_ed25519_private_key_from_jwk(jwk)

    vm = args.verification_method.strip() if args.verification_method else (did + "#key-1")
    add_ed25519_proof(vcj, priv, vm, proof_purpose=args.purpose)

    out_path = pathlib.Path(args.out) if args.out else cred_path.with_name(cred_path.stem + ".signed.json")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print("Wrote signed VC to", out_path)
    return 0


def cmd_vc_verify(args: argparse.Namespace) -> int:
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    vcj = load_json(cred_path)

    from tools.vc import verify_credential  # type: ignore
    results = verify_credential(vcj)
    ok = True
    for r in results:
        if r.ok:
            print("OK  ", r.verification_method)
        else:
            ok = False
            print("FAIL", r.verification_method, "-", r.error)
    if not results:
        print("FAIL: no proofs present")
        return 2
    return 0 if ok else 2



def cmd_vc_payload_hash(args: argparse.Namespace) -> int:
    """Print sha256 of the canonical VC payload excluding `proof`.

    This is the signing input hash used for Corridor Agreement VC binding to a Corridor Definition VC.
    """
    cred_path = pathlib.Path(args.credential)
    if not cred_path.is_absolute():
        cred_path = REPO_ROOT / cred_path
    vcj = load_json(cred_path)

    from tools.vc import signing_input  # type: ignore
    print(sha256_bytes(signing_input(vcj)))
    return 0


def cmd_corridor_verify(args: argparse.Namespace) -> int:
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent
    errs = verify_corridor_definition_vc(module_dir)
    errs.extend(verify_corridor_agreement_vc(module_dir))
    if errs:
        print("CORRIDOR FAIL:", module_dir)
        for e in errs:
            print("  -", e)
        return 2
    # If an agreement VC is configured, corridor verify implies activation thresholds are met.
    c = load_yaml(module_dir / "corridor.yaml")
    has_agreement = bool(_agreement_paths(c))
    if has_agreement:
        print("OK: corridor verified + activated:", module_dir)
    else:
        print("OK: corridor verified (no agreement VC configured):", module_dir)
    return 0



def cmd_corridor_vc_init_definition(args: argparse.Namespace) -> int:
    """Initialize (or update) a Corridor Definition VC template from corridor.yaml + artifact hashes.

    This is an authoring helper: it produces either an unsigned VC template or a signed VC, depending on --sign.
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk, did_key_from_ed25519_public_key, b64url_decode  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        print(f"Missing corridor.yaml in {module_dir}", file=sys.stderr)
        return 2

    c = load_yaml(corridor_path)
    corridor_id = str(c.get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    issuer = str(getattr(args, "issuer", "") or "").strip()
    if not issuer:
        print("--issuer is required", file=sys.stderr)
        return 2

    ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
    kr_rel = str(c.get("key_rotation_path") or "key-rotation.yaml")
    ta_path = module_dir / ta_rel
    kr_path = module_dir / kr_rel
    if not ta_path.exists():
        print(f"Missing {ta_rel}", file=sys.stderr)
        return 2
    if not kr_path.exists():
        print(f"Missing {kr_rel}", file=sys.stderr)
        return 2

    ruleset = str(getattr(args, "ruleset", "") or c.get("verification_ruleset") or "msez.corridor.verification.v1").strip()
    vc_id = str(getattr(args, "id", "") or "").strip() or f"urn:msez:vc:corridor-definition:{corridor_id}"
    maintainer = str(getattr(args, "maintainer", "") or "").strip()
    version = str(getattr(args, "version", "") or "").strip()

    vcj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorDefinitionCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "corridor_id": corridor_id,
            "ruleset": ruleset,
            "artifacts": {
                "corridor_manifest": {"path": "corridor.yaml", "sha256": sha256_file(corridor_path)},
                "trust_anchors": {"path": ta_rel, "sha256": sha256_file(ta_path)},
                "key_rotation": {"path": kr_rel, "sha256": sha256_file(kr_path)},
            },
        },
    }
    if version:
        vcj["credentialSubject"]["version"] = version
    if maintainer:
        vcj["credentialSubject"]["maintainer"] = maintainer

    # Output path
    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if getattr(args, "sign", False):
            out_path = module_dir / (str(c.get("definition_vc_path") or "corridor.vc.json"))
        else:
            out_path = module_dir / "corridor.vc.unsigned.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)

    # Optional signing
    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)
        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            # If issuer is did:key and matches derived did, use issuer#key-1; else use derived did#key-1
            issuer_base = issuer.split("#", 1)[0]
            vm_did = issuer_base if issuer_base.startswith("did:key:") else did
            vm = f"{vm_did}#key-1"
        add_ed25519_proof(vcj, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))
    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_vc_init_agreement(args: argparse.Namespace) -> int:
    """Initialize a Corridor Agreement VC template from a corridor package.

    Generates participants from trust-anchors.yaml entries that authorize 'corridor_agreement'.
    Defaults to unanimous thresholds per role (safe default); edit the output for alternative governance.
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk, signing_input  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        print(f"Missing corridor.yaml in {module_dir}", file=sys.stderr)
        return 2
    c = load_yaml(corridor_path)
    corridor_id = str(c.get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    party = str(getattr(args, "party", "") or "").strip()
    issuer = str(getattr(args, "issuer", "") or "").strip()
    if not issuer:
        issuer = party
    if not issuer:
        print("--issuer is required (or --party for participant-specific agreement)", file=sys.stderr)
        return 2

    # Resolve Corridor Definition VC and compute payload hash
    def_rel = str(c.get("definition_vc_path") or "corridor.vc.json")
    def_path = module_dir / def_rel
    if not def_path.exists():
        print(f"Missing {def_rel}", file=sys.stderr)
        return 2
    def_vcj = load_json(def_path)
    def_vc_id = str(def_vcj.get("id") or "").strip()
    def_payload_sha256 = sha256_bytes(signing_input(def_vcj))

    # Participants sourced from trust anchors authorized for corridor_agreement
    ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
    ta_path = module_dir / ta_rel
    if not ta_path.exists():
        print(f"Missing {ta_rel}", file=sys.stderr)
        return 2
    ta = load_yaml(ta_path)

    participants: List[Dict[str, Any]] = []
    for a in (ta.get("trust_anchors") or []):
        if not isinstance(a, dict):
            continue
        if "corridor_agreement" not in (a.get("allowed_attestations") or []):
            continue
        did = str(a.get("identifier") or "").split("#", 1)[0]
        if not did:
            continue
        role = str((a.get("metadata") or {}).get("role") or "participant").strip() or "participant"
        name = str(a.get("anchor_id") or "").strip()
        pobj: Dict[str, Any] = {"id": did, "role": role}
        if name:
            pobj["name"] = name
        participants.append(pobj)

    if not participants:
        print(f"{ta_rel}: no trust anchors authorize corridor_agreement; cannot scaffold participants", file=sys.stderr)
        return 2

    # Default activation thresholds: unanimity per role (safe default).
    by_role: Dict[str, List[Dict[str, Any]]] = {}
    for p in participants:
        by_role.setdefault(str(p.get("role") or "participant"), []).append(p)

    thresholds: List[Dict[str, Any]] = []
    for role, plist in sorted(by_role.items()):
        n = len(plist)
        thresholds.append(
            {
                "role": role,
                "required": n,
                "of": n,
                "description": f"all {role} participants must sign ({n}-of-{n})",
            }
        )

    accept_commitments = []
    ac_arg = getattr(args, "accept_commitments", "") or ""
    if ac_arg:
        accept_commitments = [x.strip() for x in ac_arg.split(",") if x.strip()]
    if not accept_commitments:
        accept_commitments = ["agree"]

    terms_ref = str(getattr(args, "terms_ref", "") or "").strip() or "urn:msez:terms:TODO"
    maintainer = str(getattr(args, "maintainer", "") or "").strip()
    version = str(getattr(args, "version", "") or "").strip()
    commitment = str(getattr(args, "commitment", "") or "").strip() or "agree"

    # For participant-specific agreement VCs, role must be provided or derivable.
    party_role = str(getattr(args, "role", "") or "").strip()
    if party and not party_role:
        for p0 in participants:
            if str(p0.get("id") or "") == party:
                party_role = str(p0.get("role") or "").strip()
                break
    if party and not party_role:
        print("--role is required when --party is set (and cannot be inferred from trust anchors)", file=sys.stderr)
        return 2

    # ID / output naming
    vc_id = str(getattr(args, "id", "") or "").strip()
    if not vc_id:
        if party:
            slug = re.sub(r"[^a-zA-Z0-9]+", "-", party).strip("-")[:32] or "party"
            vc_id = f"urn:msez:vc:corridor-agreement:{corridor_id}:{slug}"
        else:
            vc_id = f"urn:msez:vc:corridor-agreement:{corridor_id}"

    subj: Dict[str, Any] = {
        "corridor_id": corridor_id,
        "definition_vc_id": def_vc_id,
        "definition_payload_sha256": def_payload_sha256,
        "participants": participants,
        "activation": {
            "thresholds": thresholds,
            "effectiveFrom": now_rfc3339(),
            "accept_commitments": accept_commitments,
        },
        "terms": {"reference": terms_ref},
    }
    if version:
        subj["version"] = version
    if maintainer:
        subj["maintainer"] = maintainer

    if party:
        subj["party"] = {"id": party, "role": party_role}
        subj["commitment"] = commitment
        subj["party_terms"] = {}

    vcj: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1",
            "https://schemas.momentum-sez.org/contexts/msez/corridor/v1",
        ],
        "id": vc_id,
        "type": ["VerifiableCredential", "MSEZCorridorAgreementCredential"],
        "issuer": issuer,
        "issuanceDate": now_rfc3339(),
        "credentialSubject": subj,
    }

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if party:
            slug = re.sub(r"[^a-zA-Z0-9]+", "-", party).strip("-")[:32] or "party"
            out_name = f"corridor.agreement.{slug}.vc.json" if getattr(args, "sign", False) else f"corridor.agreement.{slug}.unsigned.json"
        else:
            out_name = "corridor.agreement.vc.json" if getattr(args, "sign", False) else "corridor.agreement.unsigned.json"
        out_path = module_dir / out_name

    out_path.parent.mkdir(parents=True, exist_ok=True)

    if getattr(args, "sign", False):
        key_path = str(getattr(args, "key", "") or "").strip()
        if not key_path:
            print("--key is required when --sign is set", file=sys.stderr)
            return 2
        kp = pathlib.Path(key_path)
        if not kp.is_absolute():
            kp = REPO_ROOT / kp
        jwk = load_json(kp)
        priv, did = load_ed25519_private_key_from_jwk(jwk)
        vm = str(getattr(args, "verification_method", "") or "").strip()
        if not vm:
            vm_did = issuer.split("#", 1)[0] if issuer.startswith("did:key:") else did
            vm = f"{vm_did}#key-1"
        add_ed25519_proof(vcj, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out_path.write_text(json.dumps(vcj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0

def cmd_corridor_status(args: argparse.Namespace) -> int:
    '''Show corridor VC and agreement activation status.'''
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    definition_errors = verify_corridor_definition_vc(module_dir)
    agreement_errors, agreement_summary = corridor_agreement_summary(module_dir)

    out = {
        'module_dir': str(module_dir),
        'corridor_id': agreement_summary.get('corridor_id'),
        'definition_ok': len(definition_errors) == 0,
        'definition_errors': definition_errors,
        'agreement': agreement_summary,
        'agreement_errors': agreement_errors,
        'activated': bool(agreement_summary.get('activated')),
    }

    if getattr(args, 'json', False):
        print(json.dumps(out, indent=2, ensure_ascii=False))
    else:
        cid = out.get('corridor_id') or module_dir.name
        print(f"Corridor: {cid}")
        print("  Definition VC:", "OK" if out['definition_ok'] else "FAIL")
        for e in definition_errors:
            print("    -", e)

        if agreement_summary.get('has_agreement'):
            print("  Agreement:", "ACTIVATED" if out['activated'] else "NOT ACTIVE")
            pattern = agreement_summary.get('agreement_pattern') or ''
            if pattern:
                print(f"    Pattern: {pattern}")
            aset = agreement_summary.get('agreement_set_sha256') or ''
            if aset:
                print(f"    Agreement set sha256: {aset}")
            signed = agreement_summary.get('signed_parties') or []
            signed_all = agreement_summary.get('signed_parties_all') or []
            participants = agreement_summary.get('participants') or []
            print(f"    Signed parties (affirmative): {len(signed)}/{len(participants)} (total signed: {len(signed_all)})")
            blocked = agreement_summary.get('blocked_parties') or []
            if blocked:
                print("    Blocked parties:")
                for b in blocked:
                    if not isinstance(b, dict):
                        continue
                    pid = b.get('id')
                    comm = b.get('commitment')
                    path = b.get('path')
                    tail = f" ({path})" if path else ""
                    print(f"      - {pid}: {comm}{tail}")
            for th in (agreement_summary.get('thresholds') or []):
                if not isinstance(th, dict):
                    continue
                role = th.get('role')
                signed_n = th.get('signed')
                req_n = th.get('required')
                sat = th.get('satisfied')
                print(f"    - threshold[{role}]: {signed_n}/{req_n} => {'OK' if sat else 'MISSING'}")
            for e in agreement_errors:
                print("    -", e)
        else:
            print("  Agreement: not configured")

    # Exit non-zero if definition fails OR if agreement configured but not activated.
    if not out['definition_ok']:
        return 2
    if agreement_summary.get('has_agreement') and not out['activated']:
        return 2
    return 0

def main() -> int:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    v = sub.add_parser("validate")
    v.add_argument("profile", nargs="?", default="profiles/digital-financial-center/profile.yaml")
    v.add_argument("--zone", default="", help="Validate a zone.yaml (overrides profile path)")
    v.add_argument("--all-modules", action="store_true")
    v.add_argument("--all-profiles", action="store_true")
    v.add_argument("--all-zones", action="store_true")
    v.set_defaults(func=cmd_validate)

    b = sub.add_parser("build")
    b.add_argument("profile", nargs="?", default="")
    b.add_argument("--zone", default="", help="Build from a zone.yaml (recommended)")
    b.add_argument("--out", default="dist")
    b.add_argument("--strict-render", action="store_true", help="Fail build when template variables are missing")
    b.add_argument("--no-render", action="store_true", help="Skip template rendering step")
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

    vc = sub.add_parser("vc")
    vc_sub = vc.add_subparsers(dest="vc_cmd", required=True)

    vcs = vc_sub.add_parser("sign")
    vcs.add_argument("credential", help="Path to an unsigned VC JSON")
    vcs.add_argument("--key", required=True, help="Private key as OKP/Ed25519 JWK (with d,x)")
    vcs.add_argument("--verification-method", default="", help="Optional verificationMethod override (default: did:key derived from JWK + '#key-1')")
    vcs.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    vcs.add_argument("--out", default="", help="Output path (default: <input>.signed.json)")
    vcs.set_defaults(func=cmd_vc_sign)

    vcv = vc_sub.add_parser("verify")
    vcv.add_argument("credential", help="Path to a signed VC JSON")
    vcv.set_defaults(func=cmd_vc_verify)

    vch = vc_sub.add_parser("payload-hash")
    vch.add_argument("credential", help="Path to a VC JSON (signed or unsigned)")
    vch.set_defaults(func=cmd_vc_payload_hash)

    vck = vc_sub.add_parser("keygen")
    vck.add_argument("--out", required=True, help="Output path for private OKP/Ed25519 JWK")
    vck.add_argument("--public-out", default="", help="Optional output path for public-only JWK (no 'd')")
    vck.add_argument("--kid", default="key-1", help="JWK key id (kid)")
    vck.set_defaults(func=cmd_vc_keygen)

    cor = sub.add_parser("corridor")
    cor_sub = cor.add_subparsers(dest="corridor_cmd", required=True)

    corv = cor_sub.add_parser("verify")
    corv.add_argument("path", help="Corridor module directory or corridor.yaml path")
    corv.set_defaults(func=cmd_corridor_verify)

    cdef = cor_sub.add_parser("vc-init-definition")
    cdef.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cdef.add_argument("--issuer", required=True, help="Issuer DID for the Corridor Definition VC")
    cdef.add_argument("--id", default="", help="Credential id (default: urn:msez:vc:corridor-definition:<corridor_id>)")
    cdef.add_argument("--ruleset", default="", help="Verification ruleset override (default: corridor.yaml verification_ruleset)")
    cdef.add_argument("--version", default="", help="credentialSubject.version")
    cdef.add_argument("--maintainer", default="", help="credentialSubject.maintainer")
    cdef.add_argument("--out", default="", help="Output path (default: corridor.vc.unsigned.json or corridor.vc.json when --sign)")
    cdef.add_argument("--sign", action="store_true", help="Sign the generated VC")
    cdef.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cdef.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cdef.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cdef.set_defaults(func=cmd_corridor_vc_init_definition)

    cagr = cor_sub.add_parser("vc-init-agreement")
    cagr.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cagr.add_argument("--issuer", default="", help="Issuer DID (defaults to --party when provided)")
    cagr.add_argument("--party", default="", help="Party DID for participant-specific agreement VC (optional)")
    cagr.add_argument("--role", default="", help="Party role for --party (required if not inferable from trust-anchors)")
    cagr.add_argument("--commitment", default="agree", help="Party commitment verb (default: agree)")
    cagr.add_argument("--accept-commitments", dest="accept_commitments", default="", help="Comma-separated affirmative commitment verbs (default: agree)")
    cagr.add_argument("--terms-ref", default="", help="Terms reference (default: urn:msez:terms:TODO)")
    cagr.add_argument("--id", default="", help="Credential id override")
    cagr.add_argument("--version", default="", help="credentialSubject.version")
    cagr.add_argument("--maintainer", default="", help="credentialSubject.maintainer")
    cagr.add_argument("--out", default="", help="Output path")
    cagr.add_argument("--sign", action="store_true", help="Sign the generated VC")
    cagr.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cagr.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cagr.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cagr.set_defaults(func=cmd_corridor_vc_init_agreement)

    cors = cor_sub.add_parser("status")
    cors.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cors.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    cors.set_defaults(func=cmd_corridor_status)


    c = sub.add_parser("check-coverage")
    c.add_argument("--profile", default="", help="Restrict check to modules in a profile.yaml")
    c.add_argument("--zone", default="", help="Restrict check to modules in the zone's profile")
    c.set_defaults(func=cmd_check_coverage)

    d = sub.add_parser("diff")
    d.add_argument("a", help="First stack.lock path")
    d.add_argument("b", help="Second stack.lock path")
    d.set_defaults(func=cmd_diff)

    p = sub.add_parser("publish")
    p.add_argument("path", help="Repo, bundle, or module directory")
    p.add_argument("--out-dir", default="dist/publish")
    p.add_argument("--pdf", action="store_true")
    p.set_defaults(func=cmd_publish)

    args = ap.parse_args()
    return args.func(args)

if __name__ == "__main__":
    sys.exit(main())
