#!/usr/bin/env python3
"""MSEZ Stack tool (reference implementation) — v0.4.14

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
- corridor state channels (genesis root + signed receipts + state root verification)

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
from datetime import datetime
from typing import Any, Dict, List, Optional, Tuple

import yaml
from jsonschema import Draft202012Validator
from referencing import Registry, Resource
from referencing.jsonschema import DRAFT202012
from lxml import etree
REPO_ROOT = pathlib.Path(__file__).resolve().parents[1]

# Ensure imports like `tools.akoma.render` work even when this file is executed
# as a script (sys.path[0] becomes the tools/ directory).
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

# Local (repo) imports (after sys.path fix)
from tools import artifacts as artifact_cas
STACK_SPEC_VERSION = "0.4.14"

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

def _coerce_sha256(value: Any) -> str:
    """Coerce either a raw sha256 hex string or an ArtifactRef-like dict into a digest string.

    This enables backward-compatible schemas where a field may be either:
    - a sha256 hex string, or
    - an object containing {digest_sha256: <hex>} (e.g., an ArtifactRef).
    """
    if isinstance(value, dict):
        return str(value.get("digest_sha256") or "").strip().lower()
    if isinstance(value, str):
        return value.strip().lower()
    return ""


def make_artifact_ref(
    artifact_type: str,
    digest_sha256: str,
    *,
    uri: str = "",
    display_name: str = "",
    media_type: str = "",
    byte_length: int | None = None,
) -> Dict[str, Any]:
    """Construct a minimally-populated ArtifactRef object.

    We use ArtifactRefs as the universal typed digest commitment substrate.
    Keeping this helper centralized makes it harder for different commands
    to drift in how they emit typed references.
    """
    d = str(digest_sha256 or "").strip().lower()
    out: Dict[str, Any] = {
        "artifact_type": str(artifact_type or "").strip(),
        "digest_sha256": d,
    }
    u = str(uri or "").strip()
    if u:
        out["uri"] = u
    dn = str(display_name or "").strip()
    if dn:
        out["display_name"] = dn
    mt = str(media_type or "").strip()
    if mt:
        out["media_type"] = mt
    if byte_length is not None:
        try:
            out["byte_length"] = int(byte_length)
        except Exception:
            pass
    return out


def load_authority_registry(
    module_dir: pathlib.Path,
    corridor_cfg: Dict[str, Any],
) -> tuple[Dict[str, set[str]], list[str]]:
    """Load + verify an optional Authority Registry VC referenced by a corridor module.

    This provides an external signer authorization layer intended to mitigate trust-anchor
    circularity. If corridor.yaml includes an `authority_registry_vc_path`, verifiers
    can require that any signer authorized in trust-anchors.yaml is also explicitly
    listed in the registry for the corresponding attestation.

    Returns:
      (allowed_by_attestation, errors)

    Where allowed_by_attestation maps attestation names (e.g. "corridor_definition") to a
    set of base DIDs authorized for that attestation.
    """

    rel = ""
    try:
        rel = str((corridor_cfg or {}).get("authority_registry_vc_path") or "").strip()
    except Exception:
        rel = ""

    if not rel:
        return ({}, [])

    errs: list[str] = []
    vc_path = module_dir / rel
    if not vc_path.exists():
        return ({}, [f"{rel}: authority registry VC not found"])

    try:
        vc = load_json(vc_path)
    except Exception as ex:
        return ({}, [f"{rel}: failed to parse authority registry VC: {ex}"])

    # Schema validation (offline)
    schema = schema_validator(REPO_ROOT / 'schemas' / 'vc.authority-registry.schema.json')
    for e in validate_with_schema(vc, schema):
        errs.append(f"{rel}: {e}")

    # Cryptographic VC verification
    try:
        from tools.vc import base_did, verify_credential

        results = verify_credential(vc)
        if not results or not any(r.ok for r in results):
            errs.append(f"{rel}: authority registry VC has no valid proof")

        allowed: Dict[str, set[str]] = {}
        subj = vc.get("credentialSubject") or {}
        authorities = subj.get("authorities") or []
        if not isinstance(authorities, list):
            authorities = []

        for a in authorities:
            if not isinstance(a, dict):
                continue
            did = base_did(a.get("did") or a.get("id") or "")
            if not did:
                continue

            att = a.get("allowed_attestations")
            if isinstance(att, str):
                att_list = [att]
            elif isinstance(att, list):
                att_list = att
            else:
                # No explicit attestation list means "registry knows about this DID".
                # Callers may treat this as wildcard depending on policy.
                att_list = []

            for name in att_list:
                if not isinstance(name, str):
                    continue
                n = name.strip()
                if not n:
                    continue
                allowed.setdefault(n, set()).add(did)

            # Support wildcard authorization in registries.
            if any(str(x).strip() == "*" for x in att_list):
                allowed.setdefault("*", set()).add(did)

        return (allowed, errs)

    except Exception as ex:
        errs.append(f"{rel}: authority registry VC verification failed: {ex}")
        return ({}, errs)


_SCHEMA_REGISTRY: Optional[Registry] = None


def _schema_registry(repo_root: pathlib.Path = REPO_ROOT) -> Registry:
    """Build an in-memory registry of known schemas keyed by $id.

    This enables offline validation of schemas that use $ref across the stack.
    """
    global _SCHEMA_REGISTRY
    if _SCHEMA_REGISTRY is not None:
        return _SCHEMA_REGISTRY

    reg = Registry()
    schemas_dir = repo_root / 'schemas'
    if schemas_dir.exists():
        for sp in sorted(schemas_dir.glob('*.schema.json')):
            try:
                sj = load_json(sp)
            except Exception:
                continue
            sid = sj.get('$id')
            if not sid or not isinstance(sid, str):
                continue
            try:
                reg = reg.with_resource(sid, Resource.from_contents(sj, default_specification=DRAFT202012))
            except Exception:
                # Fallback: accept unknown metaschemas/annotation-only docs
                try:
                    reg = reg.with_resource(sid, Resource.from_contents(sj))
                except Exception:
                    pass

    _SCHEMA_REGISTRY = reg
    return reg


def schema_validator(schema_path: pathlib.Path) -> Draft202012Validator:
    schema = load_json(schema_path)
    return Draft202012Validator(schema, registry=_schema_registry())

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
    emit_artifactrefs = bool(getattr(args, "emit_artifactrefs", False))

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
        "lawpacks": [],
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

    # lawpacks (jurisdictional legal corpus pins)
    # Zones may optionally declare a jurisdiction_stack (multiple governing layers), and lawpack_domains.
    jurisdiction_stack = zone.get("jurisdiction_stack") or [zone.get("jurisdiction_id")]
    if not isinstance(jurisdiction_stack, list) or not jurisdiction_stack:
        jurisdiction_stack = [zone.get("jurisdiction_id")]
    lawpack_domains = zone.get("lawpack_domains") or ["civil", "financial"]
    if not isinstance(lawpack_domains, list) or not lawpack_domains:
        lawpack_domains = ["civil", "financial"]

    for jid in jurisdiction_stack:
        if not jid:
            continue
        for dom in lawpack_domains:
            if not dom:
                continue
            # expected module location: modules/legal/jurisdictions/<jid segments>/<domain>
            mdir = REPO_ROOT / "modules" / "legal" / "jurisdictions"
            for seg in str(jid).split("-"):
                mdir = mdir / seg
            mdir = mdir / str(dom)
            lp_lock_path = mdir / "lawpack.lock.json"

            entry = {
                "jurisdiction_id": str(jid),
                "domain": str(dom),
                "lawpack_digest_sha256": "MISSING",
                "lawpack_lock_path": str(lp_lock_path.relative_to(REPO_ROOT)),
                "lawpack_lock_sha256": "MISSING",
                "lawpack_artifact_path": "",
                "as_of_date": "",
            }

            if lp_lock_path.exists():
                try:
                    lp_lock = load_json(lp_lock_path)
                    entry["lawpack_digest_sha256"] = str(lp_lock.get("lawpack_digest_sha256") or "MISSING")
                    entry["lawpack_lock_sha256"] = sha256_file(lp_lock_path)
                    entry["lawpack_artifact_path"] = str(lp_lock.get("artifact_path") or "")


                    # Prefer the canonical CAS convention when present:
                    # dist/artifacts/lawpack/<digest>.lawpack.zip
                    dg = str(entry.get("lawpack_digest_sha256") or "").strip().lower()
                    if SHA256_HEX_RE.match(dg):
                        cas_candidate = REPO_ROOT / "dist" / "artifacts" / "lawpack" / f"{dg}.lawpack.zip"
                        if cas_candidate.exists():
                            try:
                                entry["lawpack_artifact_path"] = str(cas_candidate.relative_to(REPO_ROOT))
                            except Exception:
                                entry["lawpack_artifact_path"] = str(cas_candidate)
                    entry["as_of_date"] = str(lp_lock.get("as_of_date") or "")
                except Exception:
                    pass

            # Optional v0.4.14+ emission: use ArtifactRef as the default digest substrate.
            dg_final = str(entry.get("lawpack_digest_sha256") or "").strip().lower()
            if emit_artifactrefs and SHA256_HEX_RE.match(dg_final):
                # Resolution hint: prefer CAS path if known, otherwise point to lawpack.lock.json.
                hint = str(entry.get("lawpack_artifact_path") or "").strip() or str(entry.get("lawpack_lock_path") or "").strip()
                entry["lawpack_digest_sha256"] = make_artifact_ref(
                    "lawpack",
                    dg_final,
                    uri=hint,
                )

            lock["lawpacks"].append(entry)

    # corridors
    for cid in zone.get("corridors", []) or []:
        # best effort: locate corridor module by corridor_id
        trust_hash = ""
        rot_hash = ""
        manifest_hash = ""
        vc_hash = ""
        trust_uri = ""
        rot_uri = ""
        manifest_uri = ""
        vc_uri = ""
        signers: List[str] = []

        agreement_hashes: List[str] = []
        agreement_uris: List[str] = []
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
            ta_rel = str(c.get("trust_anchors_path") or "trust-anchors.yaml")
            kr_rel = str(c.get("key_rotation_path") or "key-rotation.yaml")
            ta = mdir / ta_rel
            kr = mdir / kr_rel
            trust_hash = sha256_file(ta) if ta.exists() else "MISSING"
            rot_hash = sha256_file(kr) if kr.exists() else "MISSING"
            manifest_hash = sha256_file(cy) if cy.exists() else "MISSING"

            # URI hints (repo-relative) for typed ArtifactRef emission
            try:
                trust_uri = str(ta.relative_to(REPO_ROOT))
            except Exception:
                trust_uri = str(ta)
            try:
                rot_uri = str(kr.relative_to(REPO_ROOT))
            except Exception:
                rot_uri = str(kr)
            try:
                manifest_uri = str(cy.relative_to(REPO_ROOT))
            except Exception:
                manifest_uri = str(cy)

            # Corridor Definition VC (required in v0.3+; optional in older stacks)
            vc_rel = (c.get("definition_vc_path") or "").strip()
            vc_path = (mdir / vc_rel) if vc_rel else None
            if vc_path and vc_path.exists():
                vc_hash = sha256_file(vc_path)
                try:
                    vc_uri = str(vc_path.relative_to(REPO_ROOT))
                except Exception:
                    vc_uri = str(vc_path)
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
                        agreement_uris.append(str(ap.relative_to(REPO_ROOT)))
                    except Exception:
                        agreement_uris.append(str(ap))
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
                    agreement_uris.append(str(rel))

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

        # Optional v0.4.14+ emission: use ArtifactRef as the default digest substrate.
        if emit_artifactrefs:
            mh = str(entry.get("corridor_manifest_sha256") or "").strip().lower()
            th = str(entry.get("trust_anchors_sha256") or "").strip().lower()
            rh = str(entry.get("key_rotation_sha256") or "").strip().lower()
            vh = str(entry.get("corridor_definition_vc_sha256") or "").strip().lower()
            if SHA256_HEX_RE.match(mh):
                entry["corridor_manifest_sha256"] = make_artifact_ref("blob", mh, uri=manifest_uri)
            if SHA256_HEX_RE.match(th):
                entry["trust_anchors_sha256"] = make_artifact_ref("blob", th, uri=trust_uri)
            if SHA256_HEX_RE.match(rh):
                entry["key_rotation_sha256"] = make_artifact_ref("blob", rh, uri=rot_uri)
            if SHA256_HEX_RE.match(vh):
                entry["corridor_definition_vc_sha256"] = make_artifact_ref("blob", vh, uri=vc_uri)

        if agreement_hashes:
            if emit_artifactrefs:
                refs: List[Any] = []
                for i, dg in enumerate(agreement_hashes):
                    dgn = str(dg or "").strip().lower()
                    if SHA256_HEX_RE.match(dgn):
                        uri = agreement_uris[i] if i < len(agreement_uris) else ""
                        refs.append(make_artifact_ref("blob", dgn, uri=uri))
                    else:
                        refs.append(dg)
                entry["corridor_agreement_vc_sha256"] = refs
            else:
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


def cmd_law_list(args: argparse.Namespace) -> int:
    """List jurisdictional legal corpus modules (placeholders or populated)."""
    out: List[Dict[str, Any]] = []
    for mp in REPO_ROOT.glob("modules/legal/jurisdictions/**/module.yaml"):
        mod_dir = mp.parent
        rel = mod_dir.relative_to(REPO_ROOT)
        parts = rel.parts
        # modules/legal/jurisdictions/<jid_path...>/<domain>
        if len(parts) < 5:
            continue
        domain = parts[-1]
        jid_parts = parts[3:-1]
        jurisdiction_id = "-".join(jid_parts)
        if args.jurisdiction and args.jurisdiction != jurisdiction_id:
            continue
        if args.domain and args.domain != domain:
            continue
        manifest = load_yaml(mp)
        out.append({
            "jurisdiction_id": jurisdiction_id,
            "domain": domain,
            "module_id": manifest.get("module_id"),
            "path": str(rel),
            "version": manifest.get("version"),
            "license": manifest.get("license"),
        })

    out = sorted(out, key=lambda r: (r["jurisdiction_id"], r["domain"]))
    if args.json:
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        for r in out:
            print(f"{r['jurisdiction_id']:>14}  {r['domain']:<10}  {r['module_id']}  ({r['path']})")
    return 0


def cmd_law_coverage(args: argparse.Namespace) -> int:
    """Report whether the scaffolded corpus modules exist for each registry jurisdiction."""
    reg_path = REPO_ROOT / "registries" / "jurisdictions.yaml"
    if not reg_path.exists():
        print("registries/jurisdictions.yaml not found")
        return 2
    reg = load_yaml(reg_path) or []
    domains = ["civil", "financial"]
    rows: List[Dict[str, Any]] = []
    for e in reg:
        if not isinstance(e, dict):
            continue
        jid = e.get("jurisdiction_id", "")
        name = e.get("name", "")
        if not jid:
            continue
        base = REPO_ROOT / "modules" / "legal" / "jurisdictions"
        for seg in jid.split("-"):
            base = base / seg
        row = {"jurisdiction_id": jid, "name": name}
        for d in domains:
            row[d] = (base / d / "module.yaml").exists()
        rows.append(row)

    rows = sorted(rows, key=lambda r: r["jurisdiction_id"])
    if args.json:
        print(json.dumps(rows, indent=2, sort_keys=True))
    else:
        print(f"{'jurisdiction_id':<18}  {'civil':<5}  {'financial':<9}  name")
        print("-" * 88)
        for r in rows:
            civ = "yes" if r['civil'] else "no"
            fin = "yes" if r['financial'] else "no"
            print(f"{r['jurisdiction_id']:<18}  {civ:<5}  {fin:<9}  {r['name']}")
    return 0


def cmd_law_ingest(args: argparse.Namespace) -> int:
    """Ingest a jurisdiction corpus module into a content-addressed lawpack.zip.

    This builds:
      - dist/lawpacks/<jurisdiction_id>/<domain>/<digest>.lawpack.zip (implementation output)
      - dist/artifacts/lawpack/<digest>.lawpack.zip (canonical CAS copy; see spec/97-artifacts.md)
      - <module_dir>/lawpack.lock.json

    The lock entry is later pinned into stack.lock via `msez lock`.
    """
    from tools.lawpack import ingest_lawpack  # type: ignore

    module_dir = pathlib.Path(args.module)
    if not module_dir.is_absolute():
        module_dir = REPO_ROOT / module_dir

    out_dir = pathlib.Path(args.out_dir) if args.out_dir else (REPO_ROOT / "dist" / "lawpacks")
    if not out_dir.is_absolute():
        out_dir = REPO_ROOT / out_dir

    as_of_date = str(args.as_of_date or "").strip()
    if not as_of_date:
        print("ERROR: --as-of-date is required (YYYY-MM-DD)", file=sys.stderr)
        return 2

    try:
        lock_obj = ingest_lawpack(
            module_dir=module_dir,
            out_dir=out_dir,
            as_of_date=as_of_date,
            repo_root=REPO_ROOT,
            fetch=bool(getattr(args, "fetch", False)),
            include_raw=bool(getattr(args, "include_raw", False)),
            tool_version=STACK_SPEC_VERSION,
        )
    except Exception as ex:
        print(f"ERROR: law ingest failed: {ex}", file=sys.stderr)
        return 2

    # v0.4.7+: store a canonical CAS copy: dist/artifacts/lawpack/<digest>.lawpack.zip
    cas_artifact_path: str = ""
    try:
        digest = str(lock_obj.get("lawpack_digest_sha256") or "").strip().lower()
        apath = str(lock_obj.get("artifact_path") or "").strip()
        if digest and apath:
            src = pathlib.Path(apath)
            if not src.is_absolute():
                src = REPO_ROOT / src
            if src.exists():
                cas_path = artifact_cas.store_artifact_file(
                    "lawpack",
                    digest,
                    src,
                    repo_root=REPO_ROOT,
                    store_root=None,
                    dest_name=None,
                    overwrite=False,
                )
                try:
                    cas_artifact_path = str(cas_path.relative_to(REPO_ROOT))
                except Exception:
                    cas_artifact_path = str(cas_path)
    except Exception as ex:
        print(f"WARN: unable to store lawpack artifact in dist/artifacts: {ex}", file=sys.stderr)
        cas_artifact_path = ""

    if args.json:
        out = dict(lock_obj)
        if cas_artifact_path:
            out["cas_artifact_path"] = cas_artifact_path
        print(json.dumps(out, indent=2, sort_keys=True))
    else:
        print("LAWPACK OK:")
        print("  jurisdiction_id:", lock_obj.get("jurisdiction_id"))
        print("  domain:", lock_obj.get("domain"))
        print("  as_of_date:", lock_obj.get("as_of_date"))
        print("  digest:", lock_obj.get("lawpack_digest_sha256"))
        print("  artifact_path:", lock_obj.get("artifact_path"))
        if cas_artifact_path:
            print("  cas_artifact_path:", cas_artifact_path)
        print("  lock_path:", (module_dir / "lawpack.lock.json").relative_to(REPO_ROOT))
    return 0


def cmd_law_attest_init(args: argparse.Namespace) -> int:
    """Initialize a Lawpack Validity Attestation VC skeleton.

    This does not add a proof; use `msez vc sign` to sign the generated VC.
    """

    jurisdiction_id = str(getattr(args, "jurisdiction_id", "") or "").strip()
    domain = str(getattr(args, "domain", "") or "").strip()
    as_of_date = str(getattr(args, "as_of_date", "") or "").strip()
    issuer = str(getattr(args, "issuer", "") or "").strip() or "did:key:REPLACE_ME"

    dg = _coerce_sha256(getattr(args, "lawpack_digest", ""))
    if not SHA256_HEX_RE.match(dg):
        print("ERROR: --lawpack-digest must be a 64-hex sha256 digest", file=sys.stderr)
        return 2

    vc: Dict[str, Any] = {
        "@context": [
            "https://www.w3.org/2018/credentials/v1",
            "https://schemas.momentum-sez.org/contexts/msez/v1.jsonld",
            "https://schemas.momentum-sez.org/contexts/msez/lawpack/v1.jsonld",
        ],
        "id": f"urn:uuid:{uuid.uuid4()}",
        "type": ["VerifiableCredential", "MSEZLawpackAttestationCredential"],
        "issuer": {"id": issuer},
        "issuanceDate": now_rfc3339(),
        "credentialSubject": {
            "jurisdiction_id": jurisdiction_id,
            "domain": domain,
            "as_of_date": as_of_date,
            "lawpack": make_artifact_ref("lawpack", dg),
            "assertion": {
                "status": "asserted_valid",
                "statement": (
                    "I attest that the referenced lawpack digest corresponds to a legally valid body of law "
                    "for the stated jurisdiction and domain as-of the specified date."
                ),
                "sources": [],
                "evidence": [],
            },
        },
    }

    out_path = pathlib.Path(str(getattr(args, "out", "") or "") or f"lawpack-attestation.{dg[:8]}.vc.json")
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(vc, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(str(out_path.relative_to(REPO_ROOT)))
    return 0


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

    # Binding checks: artifact sha256 pins
    subj = (vcj.get("credentialSubject") or {})
    if isinstance(subj, dict):
        art = subj.get("artifacts") or {}
        if isinstance(art, dict):
            for name, spec in art.items():
                if not isinstance(spec, dict):
                    continue

                # Legacy form: {path, sha256}
                if ("sha256" in spec) or ("path" in spec):
                    rel = str(spec.get("path") or "").strip()
                    expected = str(spec.get("sha256") or "").strip().lower()
                # ArtifactRef form: {artifact_type, digest_sha256, uri?, ...}
                else:
                    rel = str(spec.get("uri") or spec.get("path") or "").strip()
                    expected = str(spec.get("digest_sha256") or "").strip().lower()
                    if not rel:
                        # Name-based defaults for the standard corridor package layout.
                        if name == "corridor_manifest":
                            rel = "corridor.yaml"
                        elif name == "trust_anchors":
                            rel = str(ta_path.name)
                        elif name == "key_rotation":
                            rel = str(kr_path.name)

                if not rel or not expected:
                    continue

                # If the hint is a non-local URI, we can't validate on-disk binding here.
                if "://" in rel or rel.startswith("ipfs:") or rel.startswith("urn:"):
                    continue

                fpath = module_dir / rel
                if not fpath.exists():
                    errs.append(f"{vc_rel}: artifacts.{name}.path missing: {rel}")
                    continue
                actual = sha256_file(fpath)
                if actual != expected:
                    errs.append(f"{vc_rel}: artifacts.{name}.sha256 mismatch (VC vs file)")

        vc_cid = subj.get("corridor_id")
        if vc_cid and vc_cid != c.get("corridor_id"):
            errs.append(f"{vc_rel}: corridor_id mismatch (VC={vc_cid} vs manifest={c.get('corridor_id')})")

    # Optional external authority registry constraint (mitigates trust-anchor circularity).
    reg_allowed, reg_errs = load_authority_registry(module_dir, c)
    if reg_errs:
        errs.extend(reg_errs)
    reg_def_allowed = set(reg_allowed.get("corridor_definition", set())) | set(reg_allowed.get("*", set()))

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

        # If a registry is present, the corridor module trust-anchor set must be a subset of it.
        if reg_def_allowed:
            for did in allowed:
                if did and did not in reg_def_allowed:
                    errs.append(f"{vc_rel}: trust anchor {did} is not authorized by authority-registry for corridor_definition")

            # Enforce that corridor_definition signers are drawn from the registry-constrained set.
            allowed = allowed.intersection(reg_def_allowed)

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
        def_lawpack_compat = ((def_vcj.get('credentialSubject') or {}).get('lawpack_compatibility'))
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

    # Optional external authority registry constraint (mitigates trust-anchor circularity).
    reg_allowed, reg_errs = load_authority_registry(module_dir, c)
    if reg_errs:
        errs.extend(reg_errs)
    reg_ag_allowed = set(reg_allowed.get("corridor_agreement", set())) | set(reg_allowed.get("*", set()))
    if reg_ag_allowed:
        for did in sorted(list(allowed_agreement)):
            if did and did not in reg_ag_allowed:
                errs.append(
                    f"{ta_path.name}: trust anchor {did} is not authorized by authority-registry for corridor_agreement"
                )
        allowed_agreement = allowed_agreement.intersection(reg_ag_allowed)

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
    pinned_lawpacks_by_party: Dict[str, List[Dict[str, Any]]] = {}

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

                    # Lawpack pins (v0.4.1+): participant attests to the exact legal corpus digests in force.
                    pinned = subj.get("pinned_lawpacks")
                    if pinned is None:
                        pinned_lawpacks_by_party[party_id] = []
                    elif isinstance(pinned, list):
                        pinned_lawpacks_by_party[party_id] = pinned
                    else:
                        errs.append(f"{rel}: credentialSubject.pinned_lawpacks must be an array when present")

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

    # Expose the participant role mapping for downstream verifiers (e.g. receipt threshold enforcement).
    summary['role_by_did'] = {k: role_by_did[k] for k in sorted(role_by_did.keys())}

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

    # v0.4.14+: receipt signing thresholds (fork resistance)
    #
    # Receipt signing policy lives in credentialSubject.state_channel.receipt_signing.
    # If absent, we fall back to activation.thresholds (backward compatible) but verifiers
    # SHOULD prefer unanimity for bilateral corridors (2-of-2) to prevent valid forks.
    summary['activation_thresholds'] = thresholds
    rs_obj = None
    if isinstance(base_subject.get('state_channel'), dict):
        rs_obj = (base_subject.get('state_channel') or {}).get('receipt_signing')
    if rs_obj is None:
        rs_obj = base_subject.get('receipt_signing')

    rs_thresholds: List[Dict[str, Any]] = []
    if isinstance(rs_obj, dict):
        t = rs_obj.get('thresholds')
        if isinstance(t, list):
            rs_thresholds = [x for x in t if isinstance(x, dict)]
    if not rs_thresholds:
        rs_thresholds = [x for x in thresholds if isinstance(x, dict)]
    summary['receipt_signing_thresholds'] = rs_thresholds

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

    summary['pinned_lawpacks_by_party'] = pinned_lawpacks_by_party

    # Lawpack compatibility enforcement (v0.4.1+)
    if isinstance(def_lawpack_compat, dict):
        req_domains = def_lawpack_compat.get("required_domains") or []
        allowed_list = def_lawpack_compat.get("allowed") or []
        allowed_map: Dict[Tuple[str, str], set] = {}

        if isinstance(allowed_list, list):
            for ent in allowed_list:
                if not isinstance(ent, dict):
                    continue
                jid = str(ent.get("jurisdiction_id") or "").strip()
                dom = str(ent.get("domain") or "").strip()
                digests = ent.get("digests_sha256") or []
                if jid and dom and isinstance(digests, list):
                    allowed_map[(jid, dom)] = set(str(d).strip() for d in digests if isinstance(d, str) and d.strip())

        # Required domain coverage (only meaningful for party-specific agreements)
        if isinstance(req_domains, list) and req_domains and agreement_pattern == "party-specific":
            for pid in sorted(role_by_did.keys()):
                pinned = pinned_lawpacks_by_party.get(pid, [])
                present = set()
                for lp in pinned:
                    if isinstance(lp, dict):
                        present.add(str(lp.get("domain") or "").strip())
                for dom in req_domains:
                    if str(dom) not in present:
                        errs.append(f"lawpacks: party {pid} missing pinned lawpack for required domain '{dom}'")

        # Allowlist enforcement (only if provided)
        if allowed_map:
            for pid, pinned in pinned_lawpacks_by_party.items():
                for lp in pinned:
                    if not isinstance(lp, dict):
                        continue
                    jid = str(lp.get("jurisdiction_id") or "").strip()
                    dom = str(lp.get("domain") or "").strip()
                    digest = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if not (jid and dom and digest):
                        continue
                    allowed = allowed_map.get((jid, dom))
                    if allowed is None:
                        errs.append(f"lawpacks: {pid} pinned {jid}/{dom} digest {digest} but definition has no allowlist entry for that jurisdiction/domain")
                    elif digest not in allowed:
                        errs.append(f"lawpacks: {pid} pinned {jid}/{dom} digest {digest} which is not in definition allowlist")


    summary['activated'] = bool(agreement_paths) and activated and (len(errs) == 0)

    return (errs, summary)


def verify_corridor_agreement_vc(module_dir: pathlib.Path) -> List[str]:
    """Verify Corridor Agreement VC(s) if configured.

    This is the strict CLI/test-facing validator wrapper around :func:`corridor_agreement_summary`.

    If `agreement_vc_path` is omitted, returns only corridor.yaml schema errors (if any).
    """

    errs, _summary = corridor_agreement_summary(module_dir)
    return errs



# --- Corridor state channels (v0.4.3+) ----------------------------------------

SHA256_HEX_RE = re.compile(r"^[a-f0-9]{64}$")


def _normalize_digest_set(values):
    """Normalize a digest list into a sorted unique list.

    This is used to ensure digest sets are stable (order-independent) prior to hashing.
    """
    out: List[str] = []
    seen = set()
    for v in (values or []):
        # Backwards compatible: accept either raw digest strings or ArtifactRef-like
        # objects carrying {digest_sha256: <hex>}.
        s = _coerce_sha256(v)
        if not s:
            continue
        if not SHA256_HEX_RE.match(s):
            raise ValueError(f"invalid sha256 digest: {v}")
        if s in seen:
            continue
        seen.add(s)
        out.append(s)
    out.sort()
    return out


def _load_rulesets_registry() -> dict:
    """Load registries/rulesets.yaml mapping ruleset_id -> descriptor path."""
    rp = REPO_ROOT / "registries" / "rulesets.yaml"
    if not rp.exists():
        return {}
    reg = load_yaml(rp) or {}
    mapping = {}
    for ent in (reg.get("rulesets") or []):
        if not isinstance(ent, dict):
            continue
        rid = str(ent.get("ruleset_id") or "").strip()
        path = str(ent.get("path") or "").strip()
        if rid and path:
            mapping[rid] = path
    return mapping


def ruleset_descriptor_digest_sha256(ruleset_id: str) -> str:
    """Compute the sha256 digest of a ruleset descriptor (content-addressed)."""
    rid = str(ruleset_id or "").strip()
    if not rid:
        raise ValueError("ruleset_id is required")

    reg = _load_rulesets_registry()
    rel = reg.get(rid)
    if not rel:
        raise ValueError(f"ruleset_id not found in registries/rulesets.yaml: {rid}")

    path = pathlib.Path(rel)
    if not path.is_absolute():
        path = REPO_ROOT / path
    if not path.exists():
        raise FileNotFoundError(f"ruleset descriptor missing: {rel}")

    data = load_json(path)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(data))


def corridor_expected_ruleset_digest_set(module_dir: pathlib.Path) -> List[str]:
    """Return the ruleset digest set that MUST govern corridor state receipts.

    By default this includes:
    - the corridor verification ruleset (corridor.yaml verification_ruleset)
    - the corridor state transition ruleset (corridor.yaml state_channel.transition_ruleset or default)
    """
    c = load_yaml(module_dir / "corridor.yaml")
    verification_ruleset = str(c.get("verification_ruleset") or "msez.corridor.verification.v1").strip()
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}
    transition_ruleset = str(state_cfg.get("transition_ruleset") or "msez.corridor.state-transition.v2").strip()

    digests = [
        ruleset_descriptor_digest_sha256(verification_ruleset),
        ruleset_descriptor_digest_sha256(transition_ruleset),
    ]
    return _normalize_digest_set(digests)


def corridor_expected_lawpack_digest_set(module_dir: pathlib.Path) -> List[str]:
    """Return the union of pinned lawpack digests from the activated agreement set.

    This is best-effort when no agreement is configured.
    """
    try:
        errs, summary = corridor_agreement_summary(module_dir)
        # If agreements exist but are invalid, keep deterministic behavior by still using what we can parse.
        pinned_by_party = (summary or {}).get("pinned_lawpacks_by_party") or {}
        digests = []
        if isinstance(pinned_by_party, dict):
            for _party, pinned in pinned_by_party.items():
                if not isinstance(pinned, list):
                    continue
                for lp in pinned:
                    if not isinstance(lp, dict):
                        continue
                    d = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if d:
                        digests.append(d)
        return _normalize_digest_set(digests)
    except Exception:
        return []


def _jcs_sha256_of_json_file(path: pathlib.Path) -> str:
    """Compute SHA256(JCS(json)) for a JSON file.

    Used for content-addressed digests where insignificant whitespace/ordering should not matter.
    """
    data = load_json(path)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(data))


def _resolve_path_repo_or_module(module_dir: pathlib.Path, rel: str) -> pathlib.Path:
    """Resolve a path that may be repo-relative or module-relative.

    Resolution order (v0.4.7+):
    1) absolute paths
    2) module-relative (to support overlays/bundles overriding shared names)
    3) repo-relative (shared artifacts like schemas/rulesets)

    This bias toward module-relative paths is intentional: it prevents accidental capture
    of a repo-root file when verifying a built bundle directory.
    """
    p = pathlib.Path(str(rel))
    if p.is_absolute():
        return p
    p_mod = module_dir / p
    if p_mod.exists():
        return p_mod
    p_repo = REPO_ROOT / p
    if p_repo.exists():
        return p_repo
    return p_mod


def _build_transition_type_registry_mapping(base_dir: pathlib.Path, reg_obj: Dict[str, Any], *, label: str) -> Dict[str, Dict[str, Any]]:
    """Build a `kind -> entry` mapping from a transition type registry object.

    Best-effort fills missing digest fields from referenced artifacts.

    Parameters:
      base_dir: directory to resolve module-relative paths (fallback when repo-relative does not exist)
      reg_obj: loaded registry object (already schema-validated)
      label: human label used in error messages
    """
    mapping: Dict[str, Dict[str, Any]] = {}
    for ent in (reg_obj.get("transition_types") or []):
        if not isinstance(ent, dict):
            continue
        kind = str(ent.get("kind") or "").strip()
        if not kind:
            continue
        if kind in mapping:
            raise ValueError(f"duplicate transition kind in registry ({label}): {kind}")

        # Copy and best-effort fill missing digests from referenced artifacts.
        e = dict(ent)

        # Schema digest: compute from schema_path when digest omitted.
        if not _coerce_sha256(e.get("schema_digest_sha256")):
            sp = str(e.get("schema_path") or "").strip()
            if sp:
                try:
                    s_path = _resolve_path_repo_or_module(base_dir, sp)
                    if s_path.exists():
                        e["schema_digest_sha256"] = _jcs_sha256_of_json_file(s_path)
                except Exception:
                    pass

        # Ruleset digest: prefer ruleset_id lookup; fall back to ruleset_path.
        if not _coerce_sha256(e.get("ruleset_digest_sha256")):
            rid = str(e.get("ruleset_id") or "").strip()
            if rid:
                try:
                    e["ruleset_digest_sha256"] = ruleset_descriptor_digest_sha256(rid)
                except Exception:
                    pass
            rp = str(e.get("ruleset_path") or "").strip()
            if rp and not _coerce_sha256(e.get("ruleset_digest_sha256")):
                try:
                    r_path = _resolve_path_repo_or_module(base_dir, rp)
                    if r_path.exists():
                        e["ruleset_digest_sha256"] = _jcs_sha256_of_json_file(r_path)
                except Exception:
                    pass

        # ZK circuit digest is not auto-computed unless a local path is provided (future-proof).

        mapping[kind] = e

    return mapping


def corridor_transition_type_registry(module_dir: pathlib.Path) -> Tuple[Optional[pathlib.Path], Dict[str, Any], Dict[str, Dict[str, Any]]]:
    """Load the corridor's transition type registry (if configured).

    The registry maps `transition.kind` -> optional digest references:
    - schema_digest_sha256 (payload format)
    - ruleset_digest_sha256 (validation semantics)
    - zk_circuit_digest_sha256 (proof-carrying transitions)

    The registry path is taken from corridor.yaml:
      state_channel.transition_type_registry_path

    Returns (registry_path, registry_object, mapping).
    """
    c = load_yaml(module_dir / "corridor.yaml")
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}
    rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
    if not rel:
        return None, {}, {}

    reg_path = _resolve_path_repo_or_module(module_dir, rel)
    if not reg_path.exists():
        raise FileNotFoundError(f"transition type registry not found: {rel}")

    reg_obj = load_yaml(reg_path) or {}
    reg_schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.registry.schema.json")
    verrs = validate_with_schema(reg_obj, reg_schema)
    if verrs:
        raise ValueError(f"transition type registry invalid ({rel}): {verrs[0]}")

    mapping = _build_transition_type_registry_mapping(module_dir, reg_obj, label=rel)
    return reg_path, reg_obj, mapping


def fill_transition_envelope_from_registry(env: Dict[str, Any], entry: Dict[str, Any]) -> Dict[str, Any]:
    """Fill a transition envelope's digest references from a registry entry.

    If the envelope already specifies a digest reference that disagrees with the registry entry,
    this function raises ValueError.
    """
    out = dict(env)
    for field in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
        expected = _coerce_sha256(entry.get(field))
        if not expected:
            continue
        actual = _coerce_sha256(out.get(field))
        if actual and actual != expected:
            raise ValueError(f"transition.{field} mismatch for kind '{out.get('kind')}'")
        out[field] = expected
    return out


def verify_transition_envelope_against_registry(
    env: Dict[str, Any],
    entry: Dict[str, Any],
    *,
    enforce: bool = False,
    allow_overrides: bool = False,
) -> List[str]:
    """Verify that a transition envelope's digest references are consistent with a registry entry.

    By default this is *non-strict*: if a registry defines a digest but the envelope omits it,
    verification passes. Set enforce=True to require presence.
    """
    errs: List[str] = []
    kind = str(env.get("kind") or "").strip()
    for field in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
        expected = _coerce_sha256(entry.get(field))
        actual = _coerce_sha256(env.get(field))
        if expected:
            if actual and actual != expected:
                if not allow_overrides:
                    errs.append(f"transition.{field} mismatch for kind '{kind}'")
            elif (not actual) and enforce:
                errs.append(f"transition.{field} missing for kind '{kind}' (required by registry)")
    return errs


TRANSITION_TYPES_SNAPSHOT_TAG = "msez.transition-types.registry.snapshot.v1"


def transition_type_registry_snapshot(registry_version: int, reg_map: Dict[str, Dict[str, Any]]) -> Dict[str, Any]:
    """Build the canonical transition-type registry snapshot used for content addressing.

    The snapshot intentionally commits only to semantics-relevant fields:
    - kind
    - schema_digest_sha256
    - ruleset_digest_sha256
    - zk_circuit_digest_sha256

    Any descriptive/provenance metadata should live outside the snapshot.
    """
    items: List[Dict[str, Any]] = []
    for kind in sorted(reg_map.keys()):
        e = reg_map[kind] or {}
        item: Dict[str, Any] = {"kind": kind}
        for f in ["schema_digest_sha256", "ruleset_digest_sha256", "zk_circuit_digest_sha256"]:
            v = str(e.get(f) or "").strip().lower()
            if v:
                item[f] = v
        items.append(item)

    return {
        "tag": TRANSITION_TYPES_SNAPSHOT_TAG,
        "registry_version": int(registry_version or 1),
        "transition_types": items,
    }


def transition_type_registry_snapshot_digest(snapshot: Dict[str, Any]) -> str:
    """Compute SHA256(JCS(snapshot))."""
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(snapshot))


def build_transition_type_registry_lock(
    *,
    reg_path: pathlib.Path,
    reg_obj: Dict[str, Any],
    reg_map: Dict[str, Dict[str, Any]],
) -> Dict[str, Any]:
    """Build a transition type registry lock object.

    The lock contains:
    - provenance (source registry path + sha256 of canonicalized YAML)
    - snapshot (canonical semantics)
    - snapshot_digest_sha256 (content address)
    """
    from tools.lawpack import canonicalize_yaml  # type: ignore

    registry_version = int((reg_obj or {}).get("version") or 1)
    snapshot = transition_type_registry_snapshot(registry_version, reg_map)
    digest = transition_type_registry_snapshot_digest(snapshot)

    # Prefer a repo-relative path when possible (helps portability).
    try:
        rel = os.path.relpath(reg_path, REPO_ROOT)
    except Exception:
        rel = str(reg_path)

    lock_obj: Dict[str, Any] = {
        "transition_types_lock_version": 1,
        "generated_at": datetime.utcnow().replace(microsecond=0).isoformat() + "Z",
        "source": {
            "registry_path": rel,
            "registry_sha256": sha256_bytes(canonicalize_yaml(reg_path)),
        },
        "snapshot": snapshot,
        "snapshot_digest_sha256": digest,
    }
    return lock_obj


def load_transition_type_registry_lock(lock_path: pathlib.Path) -> Tuple[Dict[str, Any], Dict[str, Dict[str, Any]], str]:
    """Load and validate a transition type registry lock.

    Returns (lock_object, mapping, snapshot_digest_sha256).
    """
    lock_obj = load_json(lock_path) or {}
    schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.lock.schema.json")
    errs = validate_with_schema(lock_obj, schema)
    if errs:
        raise ValueError(f"transition type registry lock invalid ({lock_path}): {errs[0]}")

    snap = lock_obj.get("snapshot")
    if not isinstance(snap, dict):
        raise ValueError("transition type registry lock missing snapshot")
    if str(snap.get("tag") or "") != TRANSITION_TYPES_SNAPSHOT_TAG:
        raise ValueError("transition type registry lock snapshot.tag mismatch")

    digest = transition_type_registry_snapshot_digest(snap)
    expected = str(lock_obj.get("snapshot_digest_sha256") or "").strip().lower()
    if expected and digest != expected:
        raise ValueError("transition type registry lock snapshot_digest_sha256 mismatch")
    lock_obj["snapshot_digest_sha256"] = digest

    mapping: Dict[str, Dict[str, Any]] = {}
    for ent in (snap.get("transition_types") or []):
        if not isinstance(ent, dict):
            continue
        kind = str(ent.get("kind") or "").strip()
        if not kind:
            continue
        if kind in mapping:
            raise ValueError(f"duplicate transition kind in lock snapshot: {kind}")
        mapping[kind] = dict(ent)
    return lock_obj, mapping, digest



TRANSITION_TYPES_LOCK_CAS_SUFFIX = ".transition-types.lock.json"


def transition_types_lock_store_dirs(repo_root: pathlib.Path = REPO_ROOT) -> List[pathlib.Path]:
    """Return directories to search for content-addressed Transition Type Registry lock snapshots.

    Canonical CAS location (v0.4.7+):
      - dist/artifacts/transition-types

    The generic artifact store roots may be extended via:
      - MSEZ_ARTIFACT_STORE_DIRS (os.pathsep-separated)

    For backwards compatibility, this function also honors:
      - MSEZ_TRANSITION_TYPES_STORE_DIRS (type directories; legacy)

    Each store directory is expected to contain files named:

      <digest>.transition-types.lock.json

    where <digest> is the lock's snapshot_digest_sha256.
    """
    repo_root = repo_root.resolve()

    dirs: List[pathlib.Path] = []

    # Legacy override: directories that already point at transition-type lock files.
    legacy_env = (os.environ.get("MSEZ_TRANSITION_TYPES_STORE_DIRS") or "").strip()
    if legacy_env:
        for part in legacy_env.split(os.pathsep):
            p = part.strip()
            if not p:
                continue
            pp = pathlib.Path(p)
            if not pp.is_absolute():
                pp = repo_root / pp
            dirs.append(pp)

    # New generic store roots (dist/artifacts/*)
    for root in artifact_cas.artifact_store_roots(repo_root):
        # artifact_store_roots returns absolute paths.
        dirs.append(pathlib.Path(root) / "transition-types")

    # Legacy default (pre-v0.4.7)
    dirs.append(repo_root / "dist" / "registries" / "transition-types")

    # Deduplicate while preserving order.
    out: List[pathlib.Path] = []
    seen = set()
    for d in dirs:
        try:
            key = str(d.resolve())
        except Exception:
            key = str(d)
        if key in seen:
            continue
        seen.add(key)
        out.append(d)
    return out


def transition_types_lock_cas_path(
    digest_sha256: str,
    *,
    store_dir: Optional[pathlib.Path] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> pathlib.Path:
    """Return the canonical content-addressed path for a transition type registry lock snapshot.

    Canonical location (v0.4.7+):
      dist/artifacts/transition-types/<digest>.transition-types.lock.json

    ``store_dir`` may be used to override the *type directory*.
    """
    d = str(digest_sha256 or "").strip().lower()
    if not SHA256_HEX_RE.match(d):
        raise ValueError("digest must be 64 lowercase hex chars")

    base = store_dir or (repo_root / "dist" / "artifacts" / "transition-types")
    if not base.is_absolute():
        base = repo_root / base

    return base / f"{d}{TRANSITION_TYPES_LOCK_CAS_SUFFIX}"


def store_transition_type_registry_lock_to_cas(
    lock_path: pathlib.Path,
    *,
    store_dir: Optional[pathlib.Path] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> Tuple[pathlib.Path, str]:
    """Store a Transition Type Registry lock file into the content-addressed registry store.

    Returns (cas_path, snapshot_digest_sha256).
    """
    if not lock_path.exists():
        raise FileNotFoundError(f"lock not found: {lock_path}")

    lock_obj, _mapping, digest = load_transition_type_registry_lock(lock_path)

    cas_path = transition_types_lock_cas_path(digest, store_dir=store_dir, repo_root=repo_root)
    cas_path.parent.mkdir(parents=True, exist_ok=True)

    # Write a normalized JSON representation (stable formatting; digest is over snapshot, not file bytes).
    cas_path.write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return cas_path, digest


def resolve_transition_type_registry_lock_by_digest(
    digest_sha256: str,
    *,
    module_dir: Optional[pathlib.Path] = None,
    store_dirs: Optional[List[pathlib.Path]] = None,
    repo_root: pathlib.Path = REPO_ROOT,
) -> pathlib.Path:
    """Resolve a Transition Type Registry lock snapshot by digest.

    Search order:
    1) content-addressed store(s) (default: dist/artifacts/transition-types)
    2) module-local conventional names (best-effort)
    3) repo fallback (registries/transition-types.lock.json) when it matches the digest
    """
    d = str(digest_sha256 or "").strip().lower()
    if not SHA256_HEX_RE.match(d):
        raise ValueError("digest must be 64 lowercase hex chars")

    dirs = store_dirs or transition_types_lock_store_dirs(repo_root)

    # 1) CAS store(s)
    for sd in dirs:
        sdp = sd
        if not sdp.is_absolute():
            sdp = repo_root / sdp
        cand = sdp / f"{d}{TRANSITION_TYPES_LOCK_CAS_SUFFIX}"
        if cand.exists():
            return cand

    # 2) module-local conventional names
    if module_dir is not None:
        for rel in ["transition-types.lock.json", "registries/transition-types.lock.json"]:
            cand = module_dir / rel
            if cand.exists():
                try:
                    _lock_obj, _mapping, dig = load_transition_type_registry_lock(cand)
                    if dig == d:
                        return cand
                except Exception:
                    pass

    # 3) repo fallback
    fallback = repo_root / "registries" / "transition-types.lock.json"
    if fallback.exists():
        try:
            _lock_obj, _mapping, dig = load_transition_type_registry_lock(fallback)
            if dig == d:
                return fallback
        except Exception:
            pass

    raise FileNotFoundError(f"transition type registry lock not found for digest {d}")


def corridor_transition_type_registry_snapshot(module_dir: pathlib.Path) -> Tuple[Optional[str], Dict[str, Dict[str, Any]]]:
    """Return (transition_type_registry_digest_sha256, mapping) best-effort.

    Preference order:
    1) transition_type_registry_lock_path (JSON lock)
    2) transition_type_registry_path (YAML registry -> derived snapshot)
    """
    c = load_yaml(module_dir / "corridor.yaml")
    state_cfg = c.get("state_channel") if isinstance(c, dict) else None
    if not isinstance(state_cfg, dict):
        state_cfg = {}

    reg_rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
    lock_rel = str(state_cfg.get("transition_type_registry_lock_path") or "").strip()

    # Derive a default lock name when a registry is configured.
    if not lock_rel and reg_rel:
        rp = pathlib.Path(reg_rel)
        if rp.suffix.lower() in {".yaml", ".yml"}:
            lock_rel = str(rp.with_suffix(".lock.json"))
        else:
            lock_rel = reg_rel + ".lock.json"

    if lock_rel:
        lp = _resolve_path_repo_or_module(module_dir, lock_rel)
        if lp.exists():
            lock_obj, mapping, digest = load_transition_type_registry_lock(lp)
            return digest, mapping

    if reg_rel:
        _rp, reg_obj, mapping = corridor_transition_type_registry(module_dir)
        if mapping:
            registry_version = int((reg_obj or {}).get("version") or 1)
            snap = transition_type_registry_snapshot(registry_version, mapping)
            digest = transition_type_registry_snapshot_digest(snap)
            return digest, mapping

    return None, {}


def cmd_registry_transition_types_lock(args: argparse.Namespace) -> int:
    """Generate a content-addressed Transition Type Registry lock file.

    The lock captures a deterministic snapshot (kind -> digests) and emits:
    - snapshot (canonical JSON)
    - snapshot_digest_sha256 = sha256(JCS(snapshot))
    """
    reg_arg = str(getattr(args, "registry", "") or "").strip()
    if not reg_arg:
        print("registry path is required", file=sys.stderr)
        return 2

    reg_path = pathlib.Path(reg_arg)
    if not reg_path.is_absolute():
        reg_path = REPO_ROOT / reg_path
    if not reg_path.exists():
        print(f"Registry not found: {reg_path}", file=sys.stderr)
        return 2

    try:
        reg_obj = load_yaml(reg_path)
    except Exception as ex:
        print(f"ERROR: invalid YAML: {ex}", file=sys.stderr)
        return 2

    schema = schema_validator(REPO_ROOT / "schemas" / "transition-types.registry.schema.json")
    errs = validate_with_schema(reg_obj, schema)
    if errs:
        for e in errs:
            print("  -", e, file=sys.stderr)
        return 2

    try:
        reg_map = _build_transition_type_registry_mapping(reg_path.parent, reg_obj, label=str(reg_path))
        lock_obj = build_transition_type_registry_lock(reg_path=reg_path, reg_obj=reg_obj, reg_map=reg_map)
    except Exception as ex:
        print(f"ERROR: unable to build transition type registry lock: {ex}", file=sys.stderr)
        return 2

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        if reg_path.suffix.lower() in {".yaml", ".yml"}:
            out_path = reg_path.with_suffix(".lock.json")
        else:
            out_path = pathlib.Path(str(reg_path) + ".lock.json")

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(lock_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    # v0.4.6+: store lock snapshots; v0.4.7+: canonical store is dist/artifacts/transition-types
    cas_path: Optional[pathlib.Path] = None
    if not bool(getattr(args, 'no_store', False)):
        try:
            sd_arg = str(getattr(args, 'store_dir', '') or '').strip()
            sd_path: Optional[pathlib.Path] = pathlib.Path(sd_arg) if sd_arg else None
            if sd_path is not None and not sd_path.is_absolute():
                sd_path = REPO_ROOT / sd_path
            cas_path, _dig = store_transition_type_registry_lock_to_cas(out_path, store_dir=sd_path, repo_root=REPO_ROOT)
        except Exception as ex:
            print(f"WARN: unable to store registry lock snapshot in content-addressed store: {ex}", file=sys.stderr)
            cas_path = None

    if getattr(args, "json", False):
        print(
            json.dumps(
                {
                    "path": str(out_path),
                    "snapshot_digest_sha256": str(lock_obj.get("snapshot_digest_sha256") or ""),
                    "stored_path": str(cas_path) if cas_path is not None else "",
                },
                indent=2,
            )
        )
    else:
        print(str(out_path))
    return 0



def cmd_registry_transition_types_store(args: argparse.Namespace) -> int:
    """Store a transition type registry lock snapshot in the content-addressed registry store."""
    lock_arg = str(getattr(args, "lock", "") or "").strip()
    if not lock_arg:
        print("lock path is required", file=sys.stderr)
        return 2

    lock_path = pathlib.Path(lock_arg)
    if not lock_path.is_absolute():
        lock_path = REPO_ROOT / lock_path
    if not lock_path.exists():
        print(f"Lock not found: {lock_path}", file=sys.stderr)
        return 2

    sd_arg = str(getattr(args, "store_dir", "") or "").strip()
    sd_path: Optional[pathlib.Path] = pathlib.Path(sd_arg) if sd_arg else None
    if sd_path is not None and not sd_path.is_absolute():
        sd_path = REPO_ROOT / sd_path

    try:
        cas_path, digest = store_transition_type_registry_lock_to_cas(lock_path, store_dir=sd_path, repo_root=REPO_ROOT)
    except Exception as ex:
        print(f"ERROR: unable to store lock snapshot: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "json", False):
        print(json.dumps({"snapshot_digest_sha256": digest, "path": str(cas_path)}, indent=2))
    else:
        print(str(cas_path))
    return 0


def cmd_registry_transition_types_resolve(args: argparse.Namespace) -> int:
    """Resolve a transition type registry lock snapshot by digest."""
    dig = str(getattr(args, "digest", "") or "").strip().lower()
    if not dig:
        print("digest is required", file=sys.stderr)
        return 2

    # Start from default dirs (env + dist/...)
    dirs = transition_types_lock_store_dirs(REPO_ROOT)

    # Optional additional search dirs (searched first)
    extra = getattr(args, "store_dir", None) or []
    for sd in reversed(list(extra)):
        s = str(sd or "").strip()
        if not s:
            continue
        pth = pathlib.Path(s)
        if not pth.is_absolute():
            pth = REPO_ROOT / pth
        if pth not in dirs:
            dirs.insert(0, pth)

    try:
        lp = resolve_transition_type_registry_lock_by_digest(dig, module_dir=None, store_dirs=dirs, repo_root=REPO_ROOT)
    except Exception as ex:
        print(f"ERROR: unable to resolve lock snapshot: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "show", False):
        try:
            obj = load_json(lp)
            print(json.dumps(obj, indent=2, ensure_ascii=False))
        except Exception as ex:
            print(f"ERROR: unable to read lock at {lp}: {ex}", file=sys.stderr)
            return 2
        return 0

    if getattr(args, "json", False):
        # Validate by loading (also recomputes digest)
        try:
            _lock_obj, _mapping, digest = load_transition_type_registry_lock(lp)
        except Exception as ex:
            print(f"ERROR: invalid lock snapshot at {lp}: {ex}", file=sys.stderr)
            return 2
        print(json.dumps({"snapshot_digest_sha256": digest, "path": str(lp)}, indent=2))
    else:
        print(str(lp))
    return 0


def corridor_state_genesis_root(module_dir: pathlib.Path) -> str:
    """Compute the corridor genesis root binding the state channel to corridor substrate.

    genesis_root = SHA256(JCS({
      tag, corridor_id, definition_payload_sha256, agreement_set_sha256,
      lawpack_digest_set, ruleset_digest_set
    }))
    """
    corridor_path = module_dir / "corridor.yaml"
    if not corridor_path.exists():
        raise FileNotFoundError("Missing corridor.yaml")
    c = load_yaml(corridor_path)
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        raise ValueError("corridor.yaml missing corridor_id")

    def_rel = str((c or {}).get("definition_vc_path") or "").strip()
    if not def_rel:
        raise ValueError("corridor.yaml missing definition_vc_path")
    def_path = module_dir / def_rel
    if not def_path.exists():
        raise FileNotFoundError(f"Missing Corridor Definition VC: {def_rel}")

    from tools.vc import signing_input  # type: ignore
    def_vcj = load_json(def_path)
    definition_payload_sha256 = sha256_bytes(signing_input(def_vcj))

    # Agreement set digest is optional (corridors may be single-operator). When no agreement is configured,
    # agreement_set_sha256 is set to "" and the genesis still binds to definition+digest-sets.
    agreement_set_sha256 = ""
    try:
        _errs, summary = corridor_agreement_summary(module_dir)
        agreement_set_sha256 = str((summary or {}).get("agreement_set_sha256") or "").strip()
    except Exception:
        agreement_set_sha256 = ""

    lawpack_digest_set = corridor_expected_lawpack_digest_set(module_dir)
    ruleset_digest_set = corridor_expected_ruleset_digest_set(module_dir)

    from tools.lawpack import jcs_canonicalize  # type: ignore
    payload = {
        "tag": "msez.corridor.state.genesis.v1",
        "corridor_id": corridor_id,
        "definition_payload_sha256": definition_payload_sha256,
        "agreement_set_sha256": agreement_set_sha256,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
    }
    return sha256_bytes(jcs_canonicalize(payload))


def corridor_state_next_root(receipt: Dict[str, Any]) -> str:
    """Compute next_root for a corridor receipt (excludes proof + next_root)."""
    if not isinstance(receipt, dict):
        raise ValueError("receipt must be an object")
    tmp = dict(receipt)
    tmp.pop("proof", None)
    tmp.pop("next_root", None)
    from tools.lawpack import jcs_canonicalize  # type: ignore
    return sha256_bytes(jcs_canonicalize(tmp))


def cmd_corridor_state_genesis_root(args: argparse.Namespace) -> int:
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    try:
        root = corridor_state_genesis_root(module_dir)
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    if getattr(args, "json", False):
        out = {
            "corridor": str(module_dir),
            "genesis_root": root,
        }
        print(json.dumps(out, indent=2))
    else:
        print(root)
    return 0


def _load_transition_envelope(path: str) -> Dict[str, Any]:
    """Load a transition input and coerce it into a typed transition envelope.

    v0.4.4+ RECOMMENDS a typed envelope of the form:

      {
        "type": "MSEZTransitionEnvelope",
        "kind": "...",
        "schema": "...",                    # optional (URI)
        "schema_digest_sha256": "<sha256>", # optional (payload schema)
        "ruleset_digest_sha256": "<sha256>",# optional (transition validation semantics)
        "zk_circuit_digest_sha256": "<sha256>", # optional (proof-carrying transitions)
        "payload": { ... },            # optional (may be omitted for privacy)
        "payload_sha256": "<sha256>",  # required when payload omitted
        "attachments": [...],          # optional
        "meta": {...}                  # optional
      }

    Backward compatible input forms:
    - empty path => noop envelope
    - legacy object with top-level 'kind' => treated as {kind, payload=<remaining fields>}
    - any other JSON value => treated as payload with kind='generic'
    """
    from tools.lawpack import jcs_canonicalize  # type: ignore

    if not path:
        payload: Any = {}
        payload_sha256 = sha256_bytes(jcs_canonicalize(payload))
        return {
            "type": "MSEZTransitionEnvelope",
            "kind": "noop",
            "payload": payload,
            "payload_sha256": payload_sha256,
        }

    pp = pathlib.Path(path)
    if not pp.is_absolute():
        pp = REPO_ROOT / pp
    if not pp.exists():
        raise FileNotFoundError(f"transition file not found: {path}")

    data = load_json(pp)

    # If already an envelope, normalize/complete it.
    if isinstance(data, dict) and str(data.get("type") or "") == "MSEZTransitionEnvelope":
        env = dict(data)
    elif isinstance(data, dict) and "kind" in data and "payload" in data:
        env = {"type": "MSEZTransitionEnvelope", **data}
    elif isinstance(data, dict) and "kind" in data:
        kind = str(data.get("kind") or "generic").strip() or "generic"
        payload = {k: v for (k, v) in data.items() if k != "kind"}
        env = {"type": "MSEZTransitionEnvelope", "kind": kind, "payload": payload}
    else:
        env = {"type": "MSEZTransitionEnvelope", "kind": "generic", "payload": data}

    kind = str(env.get("kind") or "").strip()
    if not kind:
        raise ValueError("transition envelope missing kind")

    # If payload is present, compute and (re)write payload_sha256.
    if "payload" in env:
        payload_sha256 = sha256_bytes(jcs_canonicalize(env.get("payload")))
        existing = str(env.get("payload_sha256") or "").strip().lower()
        if existing and existing != payload_sha256:
            raise ValueError("transition envelope payload_sha256 does not match computed SHA256(JCS(payload))")
        env["payload_sha256"] = payload_sha256
    else:
        # No payload embedded: require payload_sha256.
        ps = str(env.get("payload_sha256") or "").strip().lower()
        if not SHA256_HEX_RE.match(ps):
            raise ValueError("transition envelope must include payload_sha256 when payload is omitted")
        env["payload_sha256"] = ps

    env["type"] = "MSEZTransitionEnvelope"
    env["kind"] = kind
    return env


def cmd_corridor_state_receipt_init(args: argparse.Namespace) -> int:
    """Create a corridor state receipt and optionally sign it.

    By default:
    - prev_root is the corridor genesis_root
    - lawpack_digest_set is derived from the activated agreement-set (if present)
    - ruleset_digest_set is derived from the corridor verification + state-transition rulesets
    """
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()
    if not corridor_id:
        print("corridor.yaml missing corridor_id", file=sys.stderr)
        return 2

    # digest sets
    try:
        lp_set = getattr(args, "lawpack_digest", None) or []
        if lp_set:
            lawpack_digest_set = _normalize_digest_set(lp_set)
        else:
            lawpack_digest_set = corridor_expected_lawpack_digest_set(module_dir)

        rs_set = getattr(args, "ruleset_digest", None) or []
        if rs_set:
            ruleset_digest_set = _normalize_digest_set(rs_set)
        else:
            ruleset_digest_set = corridor_expected_ruleset_digest_set(module_dir)
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    # prev root
    prev_root_arg = str(getattr(args, "prev_root", "") or "").strip().lower()
    try:
        if not prev_root_arg or prev_root_arg in {"genesis", "genesis_root"}:
            prev_root = corridor_state_genesis_root(module_dir)
        else:
            if not SHA256_HEX_RE.match(prev_root_arg):
                raise ValueError("--prev-root must be 64 lowercase hex chars or 'genesis'")
            prev_root = prev_root_arg
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    try:
        seq = int(getattr(args, "sequence", 0))
        if seq < 0:
            raise ValueError("sequence must be >= 0")
    except Exception:
        print("--sequence must be a non-negative integer", file=sys.stderr)
        return 2

    ts = str(getattr(args, "timestamp", "") or "").strip()
    if not ts:
        ts = now_rfc3339()

    try:
        transition = _load_transition_envelope(str(getattr(args, "transition", "") or "").strip())
    except Exception as ex:
        print(f"ERROR: {ex}", file=sys.stderr)
        return 2

    # Optional Transition Type Registry (v0.4.4+) and Registry Lock (v0.4.5+):
    # If corridor.yaml configures a registry and/or lock, receipts MAY commit to the registry snapshot digest
    # (transition_type_registry_digest_sha256). When this digest is present, receipts can avoid repeating
    # per-transition digest references.
    fill_transition_digests = bool(getattr(args, "fill_transition_digests", False))
    ttr_digest: str = ""
    ttr_map: Dict[str, Dict[str, Any]] = {}
    try:
        _d, _m = corridor_transition_type_registry_snapshot(module_dir)
        ttr_digest = str(_d or "").strip().lower()
        ttr_map = _m or {}
        if ttr_map:
            kind = str(transition.get("kind") or "").strip()
            if kind and kind in ttr_map and fill_transition_digests:
                transition = fill_transition_envelope_from_registry(transition, ttr_map[kind])
                # If the transition references a ruleset digest, ensure it is included in the receipt-level ruleset_digest_set.
                trd = _coerce_sha256(transition.get("ruleset_digest_sha256"))
                if trd:
                    ruleset_digest_set = _normalize_digest_set(list(ruleset_digest_set) + [trd])
    except Exception as ex:
        print(f"ERROR: transition type registry: {ex}", file=sys.stderr)
        return 2

    receipt: Dict[str, Any] = {
        "type": "MSEZCorridorStateReceipt",
        "corridor_id": corridor_id,
        "sequence": seq,
        "timestamp": ts,
        "prev_root": prev_root,
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "transition": transition,
    }

    # Bind the receipt to the transition type registry snapshot (single commitment) when configured.
    if ttr_digest:
        receipt["transition_type_registry_digest_sha256"] = ttr_digest

    # Compute next_root deterministically and attach
    try:
        receipt["next_root"] = corridor_state_next_root(receipt)
    except Exception as ex:
        print(f"ERROR: unable to compute next_root: {ex}", file=sys.stderr)
        return 2

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
            vm = f"{did}#key-1"
        add_ed25519_proof(receipt, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out = str(getattr(args, "out", "") or "").strip()
    if out:
        out_path = pathlib.Path(out)
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path
    else:
        out_path = pathlib.Path(f"corridor-receipt.{seq}.json")
        if not out_path.is_absolute():
            out_path = REPO_ROOT / out_path

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(receipt, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def _collect_receipt_paths(path: pathlib.Path) -> List[pathlib.Path]:
    if path.is_dir():
        return sorted([p for p in path.glob("*.json") if p.is_file()])
    return [path]


def cmd_corridor_state_verify(args: argparse.Namespace) -> int:
    """Verify a corridor state receipt chain.

    Validates:
    - schema
    - signature(s)
    - next_root recomputation
    - digest-set bindings (best-effort)
    - root continuity (prev_root chaining, monotonic sequence)
    """
    from tools.vc import verify_credential  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath
    if not rpath.exists():
        print(f"Receipts path not found: {rpath}", file=sys.stderr)
        return 2

    receipt_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.receipt.schema.json")

    # Expected substrate
    try:
        expected_genesis = corridor_state_genesis_root(module_dir)
    except Exception as ex:
        print(f"ERROR: unable to compute genesis_root: {ex}", file=sys.stderr)
        return 2

    expected_ruleset_set = []
    try:
        expected_ruleset_set = corridor_expected_ruleset_digest_set(module_dir)
    except Exception:
        expected_ruleset_set = []

    expected_lawpack_set = []
    try:
        expected_lawpack_set = corridor_expected_lawpack_digest_set(module_dir)
    except Exception:
        expected_lawpack_set = []

    # Transition-type enforcement (v0.4.4+) and registry snapshot binding (v0.4.5+).
    #
    # v0.4.6+: receipts may commit to historical registry lock snapshots by digest, even if the corridor module later updates.
    enforce_transition_types = bool(getattr(args, "enforce_transition_types", False))


    # Commitment completeness (v0.4.8+): optionally fail verification if any digest commitments
    # in receipts cannot be resolved via the artifact CAS.
    require_artifacts = bool(getattr(args, "require_artifacts", False))
    _artifact_store_roots = artifact_cas.artifact_store_roots(REPO_ROOT)
    _artifact_checked: set[tuple[str, str]] = set()

    def _require_artifact(atype: str, digest: str, where: str) -> None:
        if not require_artifacts:
            return
        d = str(digest or "").strip().lower()
        if not d:
            return
        key = (atype, d)
        if key in _artifact_checked:
            return
        _artifact_checked.add(key)
        try:
            artifact_cas.resolve_artifact_by_digest(atype, d, repo_root=REPO_ROOT, store_roots=_artifact_store_roots)
        except Exception as ex:
            errors.append(f"{where}: missing artifact {atype}:{d} ({ex})")

    corridor_ttr_digest: Optional[str] = None
    corridor_ttr_map: Dict[str, Dict[str, Any]] = {}
    try:
        corridor_ttr_digest, corridor_ttr_map = corridor_transition_type_registry_snapshot(module_dir)
    except Exception:
        corridor_ttr_digest = None
        corridor_ttr_map = {}

    # Cache for resolved registry snapshots (digest -> mapping)
    _ttr_cache: Dict[str, Dict[str, Dict[str, Any]]] = {}
    if corridor_ttr_digest and corridor_ttr_map:
        _ttr_cache[corridor_ttr_digest] = corridor_ttr_map

    _ttr_store_dirs = transition_types_lock_store_dirs(REPO_ROOT)

    # Optional trust-anchor enforcement
    enforce_trust = bool(getattr(args, "enforce_trust_anchors", False))

    # Optional receipt threshold enforcement (v0.4.14+)
    #
    # This is the primary fork-resistance mechanism for corridors operating as verifiable state channels:
    # if N-of-M required parties sign every receipt, two conflicting receipts cannot both be valid
    # unless a signer double-signs (which is cryptographically provable).
    enforce_receipt_threshold = bool(getattr(args, "enforce_receipt_threshold", False))
    receipt_thresholds: List[Dict[str, Any]] = []
    receipt_role_by_did: Dict[str, str] = {}
    allowed_receipt_signers = set()
    if enforce_trust:
        try:
            c = load_yaml(module_dir / "corridor.yaml")
            ta_rel = str((c or {}).get("trust_anchors_path") or "trust-anchors.yaml")
            ta_path = module_dir / ta_rel
            ta = load_yaml(ta_path)
            for a in (ta.get("trust_anchors") or []):
                if not isinstance(a, dict):
                    continue
                if "corridor_receipt" in (a.get("allowed_attestations") or []):
                    ident = str(a.get("identifier") or "").split("#", 1)[0]
                    if ident:
                        allowed_receipt_signers.add(ident)
        except Exception:
            allowed_receipt_signers = set()

    # Load receipts
    paths = _collect_receipt_paths(rpath)
    receipts = []
    errors = []

    # Load receipt signing policy from the corridor agreement set (if requested).
    #
    # This uses the participant roles declared in the agreement VC(s), and the receipt_signing_thresholds
    # negotiated by the parties. If the agreement does not specify a receipt signing policy, we fall
    # back to activation thresholds (backward compatible).
    if enforce_receipt_threshold:
        try:
            aerrs, asummary = corridor_agreement_summary(module_dir)
            if aerrs:
                errors.extend([f"agreement: {e}" for e in aerrs])
            receipt_thresholds = asummary.get("receipt_signing_thresholds") or []
            receipt_role_by_did = asummary.get("role_by_did") or {}
            if not isinstance(receipt_thresholds, list) or not receipt_thresholds:
                errors.append("agreement: missing receipt_signing_thresholds (cannot enforce receipt thresholds)")
        except Exception as ex:
            errors.append(f"agreement: unable to load agreement receipt signing policy: {ex}")

    for rp in paths:
        try:
            receipt = load_json(rp)
        except Exception as ex:
            errors.append(f"{rp}: invalid JSON: {ex}")
            continue

        # Schema
        serrs = validate_with_schema(receipt, receipt_schema)
        if serrs:
            errors.extend([f"{rp}: {e}" for e in serrs])
            continue

        # Signature(s)
        ok_dids: set[str] = set()
        try:
            results = verify_credential(receipt)
            if not results:
                errors.append(f"{rp}: missing proof(s)")
                continue
            bad = [r for r in results if not r.ok]
            if bad:
                for b in bad:
                    errors.append(f"{rp}: invalid proof for {b.verification_method}: {b.error}")
            ok_methods = [r.verification_method for r in results if r.ok]
            ok_dids = {str(vm).split('#', 1)[0] for vm in ok_methods if vm}
            if enforce_trust and allowed_receipt_signers:
                if not (ok_dids & allowed_receipt_signers):
                    errors.append(f"{rp}: no valid receipt proof from an allowed trust anchor")
        except Exception as ex:
            errors.append(f"{rp}: proof verification error: {ex}")

        # Receipt signing threshold enforcement (v0.4.14+)
        if enforce_receipt_threshold and receipt_thresholds:
            # Count only signers who are declared participants in the activated agreement set.
            counts_by_role: Dict[str, int] = {}
            for did in ok_dids:
                role = receipt_role_by_did.get(did)
                if role:
                    counts_by_role[role] = counts_by_role.get(role, 0) + 1

            for thr in receipt_thresholds:
                if not isinstance(thr, dict):
                    continue
                role = str(thr.get("role") or "").strip()
                required = int(thr.get("required") or 0)
                if not role or required <= 0:
                    continue
                have = counts_by_role.get(role, 0)
                if have < required:
                    errors.append(
                        f"{rp}: receipt signing threshold not met for role '{role}': have {have}, need {required}"
                    )

        # next_root recomputation
        try:
            computed = corridor_state_next_root(receipt)
            if computed != str(receipt.get("next_root") or ""):
                errors.append(f"{rp}: next_root mismatch (computed {computed})")
        except Exception as ex:
            errors.append(f"{rp}: unable to compute next_root: {ex}")

        # digest-set bindings
        try:
            r_rules = _normalize_digest_set(receipt.get("ruleset_digest_set") or [])
            if expected_ruleset_set:
                missing = [d for d in expected_ruleset_set if d not in r_rules]
                if missing:
                    errors.append(f"{rp}: ruleset_digest_set missing expected digest(s)")
        except Exception as ex:
            errors.append(f"{rp}: invalid ruleset_digest_set: {ex}")

        try:
            r_law = _normalize_digest_set(receipt.get("lawpack_digest_set") or [])
            if expected_lawpack_set and r_law != expected_lawpack_set:
                errors.append(f"{rp}: lawpack_digest_set mismatch")
        except Exception as ex:
            errors.append(f"{rp}: invalid lawpack_digest_set: {ex}")


        # Commitment completeness (optional): require that any committed digest can be resolved via CAS.
        if require_artifacts:
            try:
                for d in _normalize_digest_set(receipt.get("lawpack_digest_set") or []):
                    _require_artifact("lawpack", d, f"{rp}: lawpack_digest_set")
            except Exception:
                pass
            try:
                for d in _normalize_digest_set(receipt.get("ruleset_digest_set") or []):
                    _require_artifact("ruleset", d, f"{rp}: ruleset_digest_set")
            except Exception:
                pass

        # Transition Type Registry checks (v0.4.4+) with snapshot binding (v0.4.5+) and
        # content-addressed lock resolution (v0.4.6+).
        try:
            r_ttr = _coerce_sha256(receipt.get("transition_type_registry_digest_sha256"))


            # Commitment completeness: ensure the committed transition registry snapshot can be resolved.
            if require_artifacts and r_ttr:
                _require_artifact("transition-types", r_ttr, f"{rp}: transition_type_registry_digest_sha256")

            # Determine effective registry mapping for this receipt.
            effective_registry: Dict[str, Dict[str, Any]] = corridor_ttr_map
            if r_ttr:
                if r_ttr in _ttr_cache:
                    effective_registry = _ttr_cache[r_ttr]
                else:
                    lp = resolve_transition_type_registry_lock_by_digest(
                        r_ttr,
                        module_dir=module_dir,
                        store_dirs=_ttr_store_dirs,
                        repo_root=REPO_ROOT,
                    )
                    _lock_obj, mapping, digest = load_transition_type_registry_lock(lp)
                    _ttr_cache[digest] = mapping
                    effective_registry = mapping

                if enforce_transition_types and not effective_registry:
                    errors.append(
                        f"{rp}: transition_type_registry_digest_sha256 present but resolved registry snapshot is empty"
                    )

            t = receipt.get("transition")
            if isinstance(t, dict) and str(t.get("type") or "") == "MSEZTransitionEnvelope":
                kind = str(t.get("kind") or "").strip()
                entry = effective_registry.get(kind) if (effective_registry and kind) else None

                # Commitment completeness: resolve any schema/ruleset/circuit digests referenced
                # by the transition envelope or its registry entry (if present).
                if require_artifacts:
                    # Explicit per-receipt overrides (when present) are direct commitments.
                    sd = _coerce_sha256(t.get("schema_digest_sha256"))
                    if sd:
                        _require_artifact("schema", sd, f"{rp}: transition.schema_digest_sha256")
                    rd = _coerce_sha256(t.get("ruleset_digest_sha256"))
                    if rd:
                        _require_artifact("ruleset", rd, f"{rp}: transition.ruleset_digest_sha256")
                    cd = _coerce_sha256(t.get("zk_circuit_digest_sha256"))
                    if cd:
                        _require_artifact("circuit", cd, f"{rp}: transition.zk_circuit_digest_sha256")

                    # Attachments are typed artifact references (v0.4.10+), but legacy receipts
                    # may omit artifact_type; those default to blob.
                    atts = t.get("attachments")
                    if isinstance(atts, list):
                        for a in atts:
                            if not isinstance(a, dict):
                                continue
                            ad = str(a.get("digest_sha256") or "").strip().lower()
                            if not ad:
                                continue
                            at = str(a.get("artifact_type") or "").strip().lower()
                            if not at:
                                at = "blob"
                            _require_artifact(at, ad, f"{rp}: transition.attachments[]")

                    # If the receipt binds to a registry snapshot digest, that digest commits to the
                    # registry entry's digests. In require mode, we require those to be resolvable too.
                    if entry and kind:
                        esd = _coerce_sha256(entry.get("schema_digest_sha256"))
                        if esd:
                            _require_artifact("schema", esd, f"{rp}: registry[{kind}].schema_digest_sha256")
                        erd = _coerce_sha256(entry.get("ruleset_digest_sha256"))
                        if erd:
                            _require_artifact("ruleset", erd, f"{rp}: registry[{kind}].ruleset_digest_sha256")
                        ecd = _coerce_sha256(entry.get("zk_circuit_digest_sha256"))
                        if ecd:
                            _require_artifact("circuit", ecd, f"{rp}: registry[{kind}].zk_circuit_digest_sha256")

                if effective_registry and kind:
                    if not entry:
                        if enforce_transition_types:
                            errors.append(f"{rp}: transition.kind '{kind}' not found in transition type registry snapshot")
                    else:
                        allow_overrides = bool(t.get("registry_override") or False)
                        enforce_fields = bool(enforce_transition_types and not r_ttr)
                        terrs = verify_transition_envelope_against_registry(
                            t,
                            entry,
                            enforce=enforce_fields,
                            allow_overrides=allow_overrides,
                        )
                        for te in terrs:
                            errors.append(f"{rp}: {te}")

                else:
                    # No registry mapping available. In enforce mode, require explicit digest references.
                    if enforce_transition_types and not r_ttr:
                        has_any = bool(
                            _coerce_sha256(t.get("schema_digest_sha256"))
                            or _coerce_sha256(t.get("ruleset_digest_sha256"))
                            or _coerce_sha256(t.get("zk_circuit_digest_sha256"))
                        )
                        if kind and not has_any:
                            errors.append(f"{rp}: transition has no registry snapshot digest and no explicit digest references")

                # If transition references a ruleset digest, it SHOULD be present in receipt.ruleset_digest_set.
                trd = _coerce_sha256(t.get("ruleset_digest_sha256"))
                if trd:
                    try:
                        rr = _normalize_digest_set(receipt.get("ruleset_digest_set") or [])
                        if trd not in rr:
                            errors.append(f"{rp}: transition.ruleset_digest_sha256 not included in ruleset_digest_set")
                    except Exception:
                        pass

                # If both envelope and receipt include a circuit digest, they MUST match.
                tcd = _coerce_sha256(t.get("zk_circuit_digest_sha256"))
                rzk = receipt.get("zk")
                if tcd and isinstance(rzk, dict):
                    rcd = _coerce_sha256(rzk.get("circuit_digest_sha256"))
                    if rcd and rcd != tcd:
                        errors.append(f"{rp}: zk.circuit_digest_sha256 mismatch (receipt vs transition)")


            # Commitment completeness: resolve ZK proof artifacts when referenced.
            if require_artifacts:
                zk = receipt.get("zk")
                if isinstance(zk, dict):
                    zcd = _coerce_sha256(zk.get("circuit_digest_sha256"))
                    if zcd:
                        _require_artifact("circuit", zcd, f"{rp}: zk.circuit_digest_sha256")
                    zvk = _coerce_sha256(zk.get("verifier_key_digest_sha256"))
                    if zvk:
                        _require_artifact("proof-key", zvk, f"{rp}: zk.verifier_key_digest_sha256")
                    zph = _coerce_sha256(zk.get("proof_sha256"))
                    if zph:
                        _require_artifact("blob", zph, f"{rp}: zk.proof_sha256")
        except Exception as ex:
            errors.append(f"{rp}: transition type registry verification error: {ex}")

        receipts.append((rp, receipt))

    if errors:
        print("STATE FAIL:")
        for e in errors:
            print("  -", e)
        return 2

    # Sort by sequence and verify continuity
    receipts.sort(key=lambda x: int(x[1].get("sequence") or 0))
    if not receipts:
        print("STATE FAIL: no receipts", file=sys.stderr)
        return 2

    expected_prev = expected_genesis
    expected_seq = int(receipts[0][1].get("sequence") or 0)

    # First receipt MUST chain from genesis_root
    first_prev = str(receipts[0][1].get("prev_root") or "")
    if first_prev != expected_prev:
        print(f"STATE FAIL: first receipt prev_root does not equal genesis_root ({first_prev} != {expected_prev})", file=sys.stderr)
        return 2

    last_next = ""
    for rp, receipt in receipts:
        seq = int(receipt.get("sequence") or 0)
        prev = str(receipt.get("prev_root") or "")
        nxt = str(receipt.get("next_root") or "")
        if seq != expected_seq:
            print(f"STATE FAIL: non-contiguous sequence at {rp} (got {seq}, expected {expected_seq})", file=sys.stderr)
            return 2
        if prev != expected_prev:
            print(f"STATE FAIL: prev_root mismatch at {rp} (got {prev}, expected {expected_prev})", file=sys.stderr)
            return 2
        expected_prev = nxt
        expected_seq += 1
        last_next = nxt

    if getattr(args, "json", False):
        out = {
            "corridor": str(module_dir),
            "genesis_root": expected_genesis,
            "receipt_count": len(receipts),
            "final_root": last_next,
        }
        print(json.dumps(out, indent=2))
    else:
        print(last_next)
    return 0




# --- Receipt accumulator (MMR) + inclusion proofs (v0.4.3+) -----------------


def _corridor_state_load_verified_receipts(module_dir: pathlib.Path, receipts_path: pathlib.Path, enforce_trust: bool = False):
    """Load and verify receipts, returning (genesis_root, receipts_sorted, final_root).

    This is a shared helper for checkpoint/proof commands.
    """
    from tools.vc import verify_credential  # type: ignore

    receipt_schema = schema_validator(REPO_ROOT / "schemas" / "corridor.receipt.schema.json")

    # Expected substrate
    expected_genesis = corridor_state_genesis_root(module_dir)

    expected_ruleset_set = []
    try:
        expected_ruleset_set = corridor_expected_ruleset_digest_set(module_dir)
    except Exception:
        expected_ruleset_set = []

    expected_lawpack_set = []
    try:
        expected_lawpack_set = corridor_expected_lawpack_digest_set(module_dir)
    except Exception:
        expected_lawpack_set = []

    # Transition Type Registry checks (v0.4.4+) with snapshot binding (v0.4.5+) and
    # content-addressed lock resolution (v0.4.6+): best-effort checks
    enforce_transition_types = False

    corridor_ttr_digest: Optional[str] = None
    corridor_ttr_map: Dict[str, Dict[str, Any]] = {}
    try:
        corridor_ttr_digest, corridor_ttr_map = corridor_transition_type_registry_snapshot(module_dir)
    except Exception:
        corridor_ttr_digest = None
        corridor_ttr_map = {}

    _ttr_cache: Dict[str, Dict[str, Dict[str, Any]]] = {}
    if corridor_ttr_digest and corridor_ttr_map:
        _ttr_cache[corridor_ttr_digest] = corridor_ttr_map
    _ttr_store_dirs = transition_types_lock_store_dirs(REPO_ROOT)

    # Optional trust-anchor enforcement (same semantics as `corridor state verify`)
    allowed_receipt_signers = set()
    if enforce_trust:
        try:
            c = load_yaml(module_dir / "corridor.yaml")
            ta_rel = str((c or {}).get("trust_anchors_path") or "trust-anchors.yaml")
            ta_path = module_dir / ta_rel
            ta = load_yaml(ta_path)
            for a in (ta.get("trust_anchors") or []):
                if not isinstance(a, dict):
                    continue
                if "corridor_receipt" in (a.get("allowed_attestations") or []):
                    ident = str(a.get("identifier") or "").split("#", 1)[0]
                    if ident:
                        allowed_receipt_signers.add(ident)
        except Exception:
            allowed_receipt_signers = set()

    # Load receipts
    paths = _collect_receipt_paths(receipts_path)
    receipts = []
    errors = []

    for rp in paths:
        try:
            receipt = load_json(rp)
        except Exception as ex:
            errors.append(f"{rp}: invalid JSON: {ex}")
            continue

        serrs = validate_with_schema(receipt, receipt_schema)
        if serrs:
            errors.extend([f"{rp}: {e}" for e in serrs])
            continue

        # Signature(s)
        try:
            results = verify_credential(receipt)
            if not results:
                errors.append(f"{rp}: missing proof(s)")
                continue
            bad = [r for r in results if not r.ok]
            if bad:
                for b in bad:
                    errors.append(f"{rp}: invalid proof for {b.verification_method}: {b.error}")
            ok_methods = [r.verification_method for r in results if r.ok]
            ok_dids = {str(vm).split('#', 1)[0] for vm in ok_methods if vm}
            if enforce_trust and allowed_receipt_signers:
                if not (ok_dids & allowed_receipt_signers):
                    errors.append(f"{rp}: no valid receipt proof from an allowed trust anchor")
        except Exception as ex:
            errors.append(f"{rp}: proof verification error: {ex}")

        # next_root recomputation
        try:
            computed = corridor_state_next_root(receipt)
            if computed != str(receipt.get("next_root") or ""):
                errors.append(f"{rp}: next_root mismatch (computed {computed})")
        except Exception as ex:
            errors.append(f"{rp}: unable to compute next_root: {ex}")

        # digest-set bindings
        try:
            r_rules = _normalize_digest_set(receipt.get("ruleset_digest_set") or [])
            if expected_ruleset_set:
                missing = [d for d in expected_ruleset_set if d not in r_rules]
                if missing:
                    errors.append(f"{rp}: ruleset_digest_set missing expected digest(s)")
        except Exception as ex:
            errors.append(f"{rp}: invalid ruleset_digest_set: {ex}")

        try:
            r_law = _normalize_digest_set(receipt.get("lawpack_digest_set") or [])
            if expected_lawpack_set and r_law != expected_lawpack_set:
                errors.append(f"{rp}: lawpack_digest_set mismatch")
        except Exception as ex:
            errors.append(f"{rp}: invalid lawpack_digest_set: {ex}")

        # Optional Transition Type Registry checks (v0.4.4+) with snapshot binding (v0.4.5+) and
        # content-addressed lock resolution (v0.4.6+)
        try:
            r_ttr = _coerce_sha256(receipt.get("transition_type_registry_digest_sha256"))

            effective_registry: Dict[str, Dict[str, Any]] = corridor_ttr_map
            if r_ttr:
                if r_ttr in _ttr_cache:
                    effective_registry = _ttr_cache[r_ttr]
                else:
                    lp = resolve_transition_type_registry_lock_by_digest(
                        r_ttr,
                        module_dir=module_dir,
                        store_dirs=_ttr_store_dirs,
                        repo_root=REPO_ROOT,
                    )
                    _lock_obj, mapping, digest = load_transition_type_registry_lock(lp)
                    _ttr_cache[digest] = mapping
                    effective_registry = mapping

            tr = receipt.get("transition")
            if isinstance(tr, dict) and str(tr.get("type") or "") == "MSEZTransitionEnvelope" and effective_registry:
                kind = str(tr.get("kind") or "").strip()
                entry = effective_registry.get(kind)
                if entry:
                    allow_overrides = bool(tr.get("registry_override") or False)
                    enforce_fields = bool(enforce_transition_types and not r_ttr)
                    terrs = verify_transition_envelope_against_registry(
                        tr,
                        entry,
                        enforce=enforce_fields,
                        allow_overrides=allow_overrides,
                    )
                    errors.extend([f"{rp}: {e}" for e in terrs])

                    # If a transition references a ruleset digest, it SHOULD be included in the receipt's ruleset_digest_set.
                    t_ruleset = _coerce_sha256(tr.get("ruleset_digest_sha256"))
                    if t_ruleset:
                        rr = _normalize_digest_set(receipt.get("ruleset_digest_set") or [])
                        if t_ruleset not in rr:
                            errors.append(f"{rp}: transition.ruleset_digest_sha256 not present in ruleset_digest_set")

                    # If both transition and receipt carry circuit digests, they must agree.
                    t_circuit = _coerce_sha256(tr.get("zk_circuit_digest_sha256"))
                    zk = receipt.get("zk")
                    if t_circuit and isinstance(zk, dict):
                        r_circuit = _coerce_sha256(zk.get("circuit_digest_sha256"))
                        if r_circuit and r_circuit != t_circuit:
                            errors.append(f"{rp}: receipt.zk.circuit_digest_sha256 does not match transition.zk_circuit_digest_sha256")
        except Exception as ex:
            errors.append(f"{rp}: transition type registry check error: {ex}")

        receipts.append((rp, receipt))

    if errors:
        raise ValueError("; ".join(errors[:10]) + (" ..." if len(errors) > 10 else ""))

    receipts.sort(key=lambda x: int(x[1].get("sequence") or 0))
    if not receipts:
        raise ValueError("no receipts")

    # Verify continuity
    expected_prev = expected_genesis
    first_seq = int(receipts[0][1].get("sequence") or 0)
    if first_seq != 0:
        raise ValueError(f"first receipt sequence must be 0 for MMR indexing (got {first_seq})")
    expected_seq = 0

    first_prev = str(receipts[0][1].get("prev_root") or "")
    if first_prev != expected_prev:
        raise ValueError(f"first receipt prev_root does not equal genesis_root ({first_prev} != {expected_prev})")

    last_next = ""
    for rp, receipt in receipts:
        seq = int(receipt.get("sequence") or 0)
        prev = str(receipt.get("prev_root") or "")
        nxt = str(receipt.get("next_root") or "")
        if seq != expected_seq:
            raise ValueError(f"non-contiguous sequence at {rp} (got {seq}, expected {expected_seq})")
        if prev != expected_prev:
            raise ValueError(f"prev_root mismatch at {rp} (got {prev}, expected {expected_prev})")
        expected_prev = nxt
        expected_seq += 1
        last_next = nxt

    return expected_genesis, [r for (_p, r) in receipts], last_next


def cmd_corridor_state_checkpoint(args: argparse.Namespace) -> int:
    """Create a signed checkpoint committing to the corridor state head and receipt MMR root."""
    from tools.vc import now_rfc3339, add_ed25519_proof, load_ed25519_private_key_from_jwk  # type: ignore
    from tools.mmr import mmr_root_from_next_roots  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath
    if not rpath.exists():
        print(f"Receipts path not found: {rpath}", file=sys.stderr)
        return 2

    try:
        genesis_root, receipts, final_root = _corridor_state_load_verified_receipts(
            module_dir, rpath, enforce_trust=bool(getattr(args, "enforce_trust_anchors", False))
        )
    except Exception as ex:
        print(f"STATE FAIL: {ex}", file=sys.stderr)
        return 2

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    # Prefer the digest sets as actually carried in receipts (after normalization).
    lawpack_digest_set = _normalize_digest_set((receipts[0].get("lawpack_digest_set") or []))
    ruleset_digest_set = _normalize_digest_set((receipts[0].get("ruleset_digest_set") or []))

    next_roots = [str(r.get("next_root") or "").strip().lower() for r in receipts]
    mmr_info = mmr_root_from_next_roots(next_roots)

    checkpoint: Dict[str, Any] = {
        "type": "MSEZCorridorStateCheckpoint",
        "corridor_id": corridor_id,
        "timestamp": now_rfc3339(),
        "genesis_root": genesis_root,
        "final_state_root": final_root,
        "receipt_count": len(receipts),
        "lawpack_digest_set": lawpack_digest_set,
        "ruleset_digest_set": ruleset_digest_set,
        "mmr": {
            "type": "MSEZReceiptMMR",
            "algorithm": "sha256",
            "size": mmr_info["size"],
            "root": mmr_info["root"],
            "peaks": mmr_info.get("peaks", []),
        },
    }

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
            vm = f"{did}#key-1"
        add_ed25519_proof(checkpoint, priv, vm, proof_purpose=str(getattr(args, "purpose", "assertionMethod")))

    out = str(getattr(args, "out", "") or "").strip() or "corridor.checkpoint.json"
    out_path = pathlib.Path(out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(checkpoint, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_state_inclusion_proof(args: argparse.Namespace) -> int:
    """Generate an MMR inclusion proof for a receipt sequence (leaf index)."""
    from tools.vc import now_rfc3339  # type: ignore
    from tools.mmr import build_inclusion_proof  # type: ignore
    from tools.lawpack import jcs_canonicalize  # type: ignore

    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    receipts_arg = str(getattr(args, "receipts", "") or "").strip()
    if not receipts_arg:
        print("--receipts is required", file=sys.stderr)
        return 2
    rpath = pathlib.Path(receipts_arg)
    if not rpath.is_absolute():
        rpath = REPO_ROOT / rpath
    if not rpath.exists():
        print(f"Receipts path not found: {rpath}", file=sys.stderr)
        return 2

    try:
        _genesis_root, receipts, _final_root = _corridor_state_load_verified_receipts(
            module_dir, rpath, enforce_trust=bool(getattr(args, "enforce_trust_anchors", False))
        )
    except Exception as ex:
        print(f"STATE FAIL: {ex}", file=sys.stderr)
        return 2

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    seq_arg = getattr(args, "sequence", None)
    if seq_arg is None:
        print("--sequence is required", file=sys.stderr)
        return 2
    try:
        seq = int(seq_arg)
        if seq < 0:
            raise ValueError()
    except Exception:
        print("--sequence must be a non-negative integer", file=sys.stderr)
        return 2

    next_roots = [str(r.get("next_root") or "").strip().lower() for r in receipts]
    if seq >= len(next_roots):
        print(f"--sequence out of range (have {len(next_roots)} receipts)", file=sys.stderr)
        return 2

    base = build_inclusion_proof(next_roots, seq)

    proof_obj: Dict[str, Any] = {
        "type": "MSEZCorridorReceiptInclusionProof",
        "corridor_id": corridor_id,
        "generated_at": now_rfc3339(),
        "mmr": {
            "type": "MSEZReceiptMMR",
            "algorithm": "sha256",
            "size": base["size"],
            "root": base["root"],
        },
        "leaf_index": base["leaf_index"],
        "receipt_next_root": base["receipt_next_root"],
        "leaf_hash": base["leaf_hash"],
        "peak_index": base["peak_index"],
        "peak_height": base["peak_height"],
        "path": base["path"],
        "peaks": base["peaks"],
        "computed_peak_root": base.get("computed_peak_root", ""),
    }

    # Optional checkpoint binding: include digest of a checkpoint payload (excluding proof)
    cp_path = str(getattr(args, "checkpoint", "") or "").strip()
    if cp_path:
        cpp = pathlib.Path(cp_path)
        if not cpp.is_absolute():
            cpp = REPO_ROOT / cpp
        if not cpp.exists():
            print(f"Checkpoint not found: {cpp}", file=sys.stderr)
            return 2
        cp = load_json(cpp)
        # Verify it matches the MMR root/size.
        try:
            cp_mmr = (cp or {}).get("mmr") or {}
            if str(cp_mmr.get("root") or "").strip().lower() != str(base["root"]).strip().lower():
                print("Checkpoint mmr.root does not match computed proof root", file=sys.stderr)
                return 2
            if int(cp_mmr.get("size") or 0) != int(base["size"]):
                print("Checkpoint mmr.size does not match computed proof size", file=sys.stderr)
                return 2
        except Exception:
            print("Invalid checkpoint format", file=sys.stderr)
            return 2

        tmp = dict(cp)
        tmp.pop("proof", None)
        checkpoint_digest = sha256_bytes(jcs_canonicalize(tmp))
        proof_obj["checkpoint_digest_sha256"] = checkpoint_digest
        proof_obj["checkpoint_path"] = str(cpp)
        proof_obj["checkpoint_mmr_root"] = str((cp_mmr or {}).get("root") or "")
        proof_obj["checkpoint_receipt_count"] = int((cp_mmr or {}).get("size") or 0)
        proof_obj["checkpoint_final_state_root"] = str((cp or {}).get("final_state_root") or "")
        proof_obj["checkpoint_ref"] = {
            "artifact_type": "checkpoint",
            "digest_sha256": checkpoint_digest,
            "uri": str(cpp),
            "media_type": "application/json",
        }

    out = str(getattr(args, "out", "") or "").strip() or f"corridor.inclusion-proof.{seq}.json"
    out_path = pathlib.Path(out)
    if not out_path.is_absolute():
        out_path = REPO_ROOT / out_path
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(proof_obj, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    print(str(out_path))
    return 0


def cmd_corridor_state_verify_inclusion(args: argparse.Namespace) -> int:
    """Verify an inclusion proof against a signed checkpoint and a receipt.

    Validates:
    - Receipt signature(s)
    - Checkpoint signature(s)
    - Receipt next_root recomputation
    - MMR inclusion proof: receipt.next_root included in checkpoint.mmr.root

    If --enforce-trust-anchors is set, requires at least one valid signature from a trust anchor
    authorized for:
    - corridor_receipt (receipt)
    - corridor_checkpoint (checkpoint) OR corridor_receipt (fallback)
    """
    from tools.vc import verify_credential  # type: ignore
    from tools.mmr import verify_inclusion_proof  # type: ignore

    # Resolve corridor module
    p = pathlib.Path(args.path)
    if not p.is_absolute():
        p = REPO_ROOT / p
    module_dir = p if p.is_dir() else p.parent

    c = load_yaml(module_dir / "corridor.yaml")
    corridor_id = str((c or {}).get("corridor_id") or "").strip()

    enforce_trust = bool(getattr(args, "enforce_trust_anchors", False))
    allowed_receipt_signers = set()
    allowed_checkpoint_signers = set()

    if enforce_trust:
        try:
            ta_rel = str((c or {}).get("trust_anchors_path") or "trust-anchors.yaml")
            ta = load_yaml(module_dir / ta_rel)
            for a in (ta.get("trust_anchors") or []):
                if not isinstance(a, dict):
                    continue
                ident = str(a.get("identifier") or "").split("#", 1)[0]
                if not ident:
                    continue
                allowed = set(a.get("allowed_attestations") or [])
                if "corridor_receipt" in allowed:
                    allowed_receipt_signers.add(ident)
                # corridor_checkpoint is preferred; corridor_receipt is accepted for backwards compatibility.
                if "corridor_checkpoint" in allowed or "corridor_receipt" in allowed:
                    allowed_checkpoint_signers.add(ident)
        except Exception:
            allowed_receipt_signers = set()
            allowed_checkpoint_signers = set()

    # Inputs
    receipt_path = pathlib.Path(str(getattr(args, "receipt", "") or "").strip())
    proof_path = pathlib.Path(str(getattr(args, "proof", "") or "").strip())
    checkpoint_path = pathlib.Path(str(getattr(args, "checkpoint", "") or "").strip())

    for name, pp in [("--receipt", receipt_path), ("--proof", proof_path), ("--checkpoint", checkpoint_path)]:
        if not str(pp):
            print(f"{name} is required", file=sys.stderr)
            return 2
        if not pp.is_absolute():
            pp2 = REPO_ROOT / pp
        else:
            pp2 = pp
        if not pp2.exists():
            print(f"{name} not found: {pp2}", file=sys.stderr)
            return 2

    # Normalize absolute paths
    if not receipt_path.is_absolute():
        receipt_path = REPO_ROOT / receipt_path
    if not proof_path.is_absolute():
        proof_path = REPO_ROOT / proof_path
    if not checkpoint_path.is_absolute():
        checkpoint_path = REPO_ROOT / checkpoint_path

    receipt = load_json(receipt_path)
    proof = load_json(proof_path)
    checkpoint = load_json(checkpoint_path)

    # Corridor id binding (best-effort)
    if corridor_id:
        for obj_name, obj in [("receipt", receipt), ("checkpoint", checkpoint), ("proof", proof)]:
            obj_cid = str((obj or {}).get("corridor_id") or "").strip()
            if obj_cid and obj_cid != corridor_id:
                print(f"FAIL: {obj_name}.corridor_id does not match corridor.yaml", file=sys.stderr)
                return 2

    # Verify receipt proof(s)
    rres = verify_credential(receipt)
    if not rres or any(not r.ok for r in rres):
        print("FAIL: receipt signature invalid", file=sys.stderr)
        return 2
    if enforce_trust and allowed_receipt_signers:
        ok_dids = {str(r.verification_method).split('#', 1)[0] for r in rres if r.ok and r.verification_method}
        if not (ok_dids & allowed_receipt_signers):
            print("FAIL: receipt not signed by an allowed trust anchor", file=sys.stderr)
            return 2

    # Verify checkpoint proof(s)
    cres = verify_credential(checkpoint)
    if not cres or any(not r.ok for r in cres):
        print("FAIL: checkpoint signature invalid", file=sys.stderr)
        return 2
    if enforce_trust and allowed_checkpoint_signers:
        ok_dids = {str(r.verification_method).split('#', 1)[0] for r in cres if r.ok and r.verification_method}
        if not (ok_dids & allowed_checkpoint_signers):
            print("FAIL: checkpoint not signed by an allowed trust anchor", file=sys.stderr)
            return 2

    # Verify receipt next_root
    computed = corridor_state_next_root(receipt)
    if computed != str(receipt.get("next_root") or ""):
        print("FAIL: receipt next_root mismatch", file=sys.stderr)
        return 2

    # Ensure proof binds to this receipt digest
    if str(proof.get("receipt_next_root") or "").strip().lower() != str(receipt.get("next_root") or "").strip().lower():
        print("FAIL: proof receipt_next_root does not match receipt", file=sys.stderr)
        return 2

    # Optional: sequence binding
    try:
        if int(proof.get("leaf_index") or -1) != int(receipt.get("sequence") or -2):
            print("FAIL: proof.leaf_index does not match receipt.sequence", file=sys.stderr)
            return 2
    except Exception:
        pass

    # Ensure proof binds to checkpoint root
    cp_mmr = (checkpoint or {}).get("mmr") or {}
    pr_mmr = (proof or {}).get("mmr") or {}
    if str(cp_mmr.get("root") or "").strip().lower() != str(pr_mmr.get("root") or "").strip().lower():
        print("FAIL: proof mmr.root does not match checkpoint", file=sys.stderr)
        return 2
    if int(cp_mmr.get("size") or 0) != int(pr_mmr.get("size") or 0):
        print("FAIL: proof mmr.size does not match checkpoint", file=sys.stderr)
        return 2
    try:
        if int(checkpoint.get("receipt_count") or 0) and int(checkpoint.get("receipt_count") or 0) != int(cp_mmr.get("size") or 0):
            print("FAIL: checkpoint receipt_count does not match checkpoint.mmr.size", file=sys.stderr)
            return 2
    except Exception:
        pass

    # Optional proof binding: checkpoint digest commitment (v0.4.11+)
    expected_cp_digest = ""
    if isinstance((proof or {}).get("checkpoint_ref"), dict):
        expected_cp_digest = _coerce_sha256((proof or {}).get("checkpoint_ref"))
    if not expected_cp_digest:
        expected_cp_digest = str((proof or {}).get("checkpoint_digest_sha256") or "").strip().lower()
    if expected_cp_digest:
        try:
            from tools.lawpack import jcs_canonicalize  # type: ignore
            tmp = dict(checkpoint)
            tmp.pop("proof", None)
            computed_cp_digest = sha256_bytes(jcs_canonicalize(tmp))
            if computed_cp_digest != expected_cp_digest:
                print("FAIL: checkpoint digest does not match proof commitment", file=sys.stderr)
                return 2
        except Exception:
            pass

    # Optional informational bindings
    try:
        if (proof or {}).get("checkpoint_final_state_root") and (checkpoint or {}).get("final_state_root"):
            if str((proof or {}).get("checkpoint_final_state_root") or "").strip().lower() != str((checkpoint or {}).get("final_state_root") or "").strip().lower():
                print("FAIL: proof checkpoint_final_state_root does not match checkpoint", file=sys.stderr)
                return 2
    except Exception:
        pass

    # Verify MMR inclusion proof
    mmr_proof = {
        "size": int(pr_mmr.get("size") or 0),
        "root": str(pr_mmr.get("root") or "").strip().lower(),
        "leaf_index": int(proof.get("leaf_index") or 0),
        "receipt_next_root": str(proof.get("receipt_next_root") or "").strip().lower(),
        "leaf_hash": str(proof.get("leaf_hash") or "").strip().lower(),
        "peak_index": int(proof.get("peak_index") or 0),
        "peak_height": int(proof.get("peak_height") or 0),
        "path": proof.get("path"),
        "peaks": proof.get("peaks"),
    }

    if not verify_inclusion_proof(mmr_proof):
        print("FAIL: invalid inclusion proof", file=sys.stderr)
        return 2

    print("OK")
    return 0


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

    # Optional transition type registry artifact pins (v0.4.4+) and lock pin (v0.4.5+)
    try:
        state_cfg = c.get("state_channel") if isinstance(c, dict) else None
        if not isinstance(state_cfg, dict):
            state_cfg = {}

        ttr_rel = str(state_cfg.get("transition_type_registry_path") or "").strip()
        ttl_rel = str(state_cfg.get("transition_type_registry_lock_path") or "").strip()

        # Derive a default lock name when a registry is configured.
        if not ttl_rel and ttr_rel:
            rp = pathlib.Path(ttr_rel)
            if rp.suffix.lower() in {".yaml", ".yml"}:
                ttl_rel = str(rp.with_suffix(".lock.json"))
            else:
                ttl_rel = ttr_rel + ".lock.json"

        # Pin the lock artifact first (preferred).
        if ttl_rel:
            ttl_path = _resolve_path_repo_or_module(module_dir, ttl_rel)
            if not ttl_path.exists():
                print(f"Missing {ttl_rel}", file=sys.stderr)
                return 2
            vcj["credentialSubject"]["artifacts"]["transition_type_registry_lock"] = {
                "path": ttl_rel,
                "sha256": sha256_file(ttl_path),
            }

        # Optionally also pin the human-authored registry YAML.
        if ttr_rel:
            ttr_path = _resolve_path_repo_or_module(module_dir, ttr_rel)
            if not ttr_path.exists():
                print(f"Missing {ttr_rel}", file=sys.stderr)
                return 2
            vcj["credentialSubject"]["artifacts"]["transition_type_registry"] = {
                "path": ttr_rel,
                "sha256": sha256_file(ttr_path),
            }
    except Exception as ex:
        print(f"ERROR: unable to pin transition type registry artifacts: {ex}", file=sys.stderr)
        return 2

    # Ruleset digest pins (v0.4.3+)
    # Provides content-addressed identifiers for the verifier logic governing this corridor.
    try:
        state_cfg = c.get("state_channel") if isinstance(c, dict) else None
        if not isinstance(state_cfg, dict):
            state_cfg = {}
        transition_ruleset = str(state_cfg.get("transition_ruleset") or "msez.corridor.state-transition.v2").strip()
        vcj["credentialSubject"]["ruleset_digest_set"] = _normalize_digest_set([
            ruleset_descriptor_digest_sha256(ruleset),
            ruleset_descriptor_digest_sha256(transition_ruleset),
        ])
    except Exception:
        # Best-effort: allow authoring even if ruleset registry is missing.
        pass

    # Lawpack compatibility scaffold (v0.4.1+)
    # By default we require each participant to pin (and sign) civil + financial lawpacks.
    # To additionally *constrain* which digests are compatible, pass --allow-lawpack one or more times.
    from tools.lawpack import parse_lawpack_ref  # type: ignore
    req_domains = getattr(args, "require_domain", None) or ["civil", "financial"]
    allow_refs = getattr(args, "allow_lawpack", None) or []
    lawpack_compat: Dict[str, Any] = {"required_domains": req_domains}

    if allow_refs:
        allowed_map: Dict[Tuple[str, str], set] = {}
        for ref in allow_refs:
            lr = parse_lawpack_ref(ref)
            key = (lr["jurisdiction_id"], lr["domain"])
            allowed_map.setdefault(key, set()).add(lr["lawpack_digest_sha256"])
        allowed_list: List[Dict[str, Any]] = []
        for (jid, dom), digests in sorted(allowed_map.items(), key=lambda x: (x[0][0], x[0][1])):
            allowed_list.append({"jurisdiction_id": jid, "domain": dom, "digests_sha256": sorted(digests)})
        lawpack_compat["allowed"] = allowed_list

    vcj["credentialSubject"]["lawpack_compatibility"] = lawpack_compat

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
        # v0.4.14+: state-channel receipt signing policy.
        # Default is unanimity per role (safe against forks unless a signer equivocates).
        "state_channel": {
            "receipt_signing": {
                "thresholds": thresholds,
                "effectiveFrom": now_rfc3339(),
                "description": "receipt signing thresholds for corridor state transitions",
            }
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

        # Participant-specific lawpack pins (v0.4.1+)
        from tools.lawpack import parse_lawpack_ref  # type: ignore
        pinned: List[Dict[str, Any]] = []

        # 1) pull from a zone's stack.lock (recommended)
        zl = str(getattr(args, "from_zone_lock", "") or "").strip()
        if zl:
            zl_path = pathlib.Path(zl)
            if not zl_path.is_absolute():
                zl_path = REPO_ROOT / zl_path
            try:
                zl_obj = load_yaml(zl_path)
                for lp in (zl_obj.get("lawpacks") or []):
                    if not isinstance(lp, dict):
                        continue
                    digest = _coerce_sha256(lp.get("lawpack_digest_sha256"))
                    if not digest or digest == "MISSING":
                        continue
                    pinned.append({
                        "jurisdiction_id": str(lp.get("jurisdiction_id") or ""),
                        "domain": str(lp.get("domain") or ""),
                        "lawpack_digest_sha256": digest,
                        "lawpack_lock_sha256": str(lp.get("lawpack_lock_sha256") or ""),
                        "as_of_date": str(lp.get("as_of_date") or ""),
                    })
            except Exception as ex:
                print(f"WARNING: could not load zone lock {zl_path}: {ex}", file=sys.stderr)

        # 2) explicit refs
        for ref in (getattr(args, "pinned_lawpack", None) or []):
            try:
                pinned.append(parse_lawpack_ref(ref))
            except Exception as ex:
                print(f"WARNING: ignoring invalid --pinned-lawpack '{ref}': {ex}", file=sys.stderr)

        # de-dup (jurisdiction_id, domain, digest)
        seen = set()
        dedup: List[Dict[str, Any]] = []
        for lp in pinned:
            key = (str(lp.get("jurisdiction_id") or ""), str(lp.get("domain") or ""), _coerce_sha256(lp.get("lawpack_digest_sha256")))
            if key in seen:
                continue
            seen.add(key)
            dedup.append(lp)
        subj["pinned_lawpacks"] = dedup

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



# --- Generic artifact CAS (v0.4.7+) ------------------------------------------

ARTIFACT_TYPES_KNOWN = [
    "lawpack",
    "ruleset",
    "transition-types",
    "circuit",
    "schema",
    "vc",
    "checkpoint",
    "proof-key",
    "blob",
]


def resolve_artifact_commitment(
    artifact_type: str,
    digest_sha256: str,
    *,
    repo_root: pathlib.Path = REPO_ROOT,
    store_roots: Optional[List[pathlib.Path]] = None,
    module_dir: Optional[pathlib.Path] = None,
) -> pathlib.Path:
    """Resolve an artifact commitment (type + sha256 digest) to a concrete path.

    Primary resolution uses the generic CAS convention:

      dist/artifacts/<type>/<digest>.*

    For backwards compatibility, this function provides best-effort legacy fallbacks:
    - transition-types: dist/artifacts/transition-types + (legacy fallbacks)
    - lawpack: dist/lawpacks/**/<digest>.lawpack.zip
    - ruleset: scan rulesets declared in registries/rulesets.yaml and compare digests

    The intent is that any digest commitment appearing in receipts/VCs has an obvious
    resolution path.
    """

    repo_root = repo_root.resolve()
    at = artifact_cas.normalize_artifact_type(artifact_type)
    dg = artifact_cas.normalize_digest(digest_sha256)

    roots = store_roots or artifact_cas.artifact_store_roots(repo_root)

    # 1) Generic CAS lookup.
    try:
        return artifact_cas.resolve_artifact_by_digest(at, dg, repo_root=repo_root, store_roots=roots)
    except FileNotFoundError:
        pass

    # 2) Legacy fallbacks (type-specific).
    if at == "transition-types":
        return resolve_transition_type_registry_lock_by_digest(dg, module_dir=module_dir, repo_root=repo_root)

    if at == "lawpack":
        legacy_root = repo_root / "dist" / "lawpacks"
        if legacy_root.exists():
            for cand in sorted(legacy_root.rglob(f"{dg}.lawpack.zip")):
                if cand.is_file():
                    return cand
        raise FileNotFoundError(f"lawpack artifact not found for digest {dg}")

    if at == "ruleset":
        # Ruleset digests are SHA256(JCS(ruleset_descriptor_json)).
        reg = _load_rulesets_registry()
        for rid, rel in reg.items():
            try:
                dig = ruleset_descriptor_digest_sha256(rid)
            except Exception:
                continue
            if dig != dg:
                continue
            p = pathlib.Path(rel)
            if not p.is_absolute():
                p = repo_root / p
            if p.exists():
                return p
        raise FileNotFoundError(f"ruleset descriptor not found for digest {dg}")

    raise FileNotFoundError(f"artifact not found for type '{at}' digest {dg}")


def cmd_artifact_store(args: argparse.Namespace) -> int:
    at = str(getattr(args, "type", "") or "").strip()
    digest = str(getattr(args, "digest", "") or "").strip()
    src = pathlib.Path(str(getattr(args, "path", "") or "").strip())
    if not (at and digest and str(src)):
        print("type, digest and path are required", file=sys.stderr)
        return 2

    store_root = str(getattr(args, "store_root", "") or "").strip()
    name = str(getattr(args, "name", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    dest = artifact_cas.store_artifact_file(
        at,
        digest,
        src,
        repo_root=REPO_ROOT,
        store_root=pathlib.Path(store_root) if store_root else None,
        dest_name=name or None,
        overwrite=overwrite,
    )

    if getattr(args, "json", False):
        print(json.dumps({"stored": dest.as_posix(), "type": artifact_cas.normalize_artifact_type(at), "digest": artifact_cas.normalize_digest(digest)}, indent=2))
    else:
        print(dest.as_posix())
    return 0


def cmd_artifact_resolve(args: argparse.Namespace) -> int:
    at = str(getattr(args, "type", "") or "").strip()
    digest = str(getattr(args, "digest", "") or "").strip()
    if not (at and digest):
        print("type and digest are required", file=sys.stderr)
        return 2

    extra_roots = getattr(args, "store_root", None) or []
    roots = []
    for r in extra_roots:
        rr = str(r or "").strip()
        if not rr:
            continue
        roots.append(pathlib.Path(rr))

    p = resolve_artifact_commitment(
        at,
        digest,
        repo_root=REPO_ROOT,
        store_roots=roots if roots else None,
    )

    if getattr(args, "show", False):
        # Only show text-like artifacts.
        if p.suffix.lower() in {".json", ".yaml", ".yml", ".md", ".txt"}:
            print(p.read_text(encoding="utf-8"))
        else:
            print(p.as_posix())
    else:
        if getattr(args, "json", False):
            print(json.dumps({"path": p.as_posix(), "type": artifact_cas.normalize_artifact_type(at), "digest": artifact_cas.normalize_digest(digest)}, indent=2))
        else:
            print(p.as_posix())
    return 0


def cmd_artifact_index_rulesets(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/ruleset with content-addressed copies of declared rulesets."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    reg = _load_rulesets_registry()
    if not reg:
        print("No rulesets found in registries/rulesets.yaml", file=sys.stderr)
        return 2

    stored = []
    for rid, rel in sorted(reg.items()):
        try:
            digest = ruleset_descriptor_digest_sha256(rid)
        except Exception as ex:
            print(f"WARN: could not digest ruleset {rid}: {ex}", file=sys.stderr)
            continue
        src = pathlib.Path(rel)
        if not src.is_absolute():
            src = REPO_ROOT / src
        if not src.exists():
            print(f"WARN: missing ruleset file for {rid}: {src}", file=sys.stderr)
            continue
        dest = artifact_cas.store_artifact_file(
            "ruleset",
            digest,
            src,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({"ruleset_id": rid, "digest": digest, "path": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} ruleset artifacts")
    return 0


def cmd_artifact_index_lawpacks(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/lawpack by copying any local dist/lawpacks/**/*.lawpack.zip files."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    legacy_root = REPO_ROOT / "dist" / "lawpacks"
    if not legacy_root.exists():
        print("dist/lawpacks not found", file=sys.stderr)
        return 2

    stored = []
    for lp in sorted(legacy_root.rglob("*.lawpack.zip")):
        name = lp.name
        digest = name.split(".", 1)[0].strip().lower()
        if not SHA256_HEX_RE.match(digest):
            continue
        dest = artifact_cas.store_artifact_file(
            "lawpack",
            digest,
            lp,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({"digest": digest, "src": lp.as_posix(), "dest": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} lawpack artifacts")
    return 0


def cmd_artifact_index_schemas(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/schema with content-addressed copies of JSON Schemas."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    schema_dir = REPO_ROOT / "schemas"
    if not schema_dir.exists():
        print("schemas/ not found", file=sys.stderr)
        return 2

    stored = []
    for sp in sorted(schema_dir.rglob("*.schema.json")):
        if not sp.is_file():
            continue
        try:
            digest = _jcs_sha256_of_json_file(sp)
        except Exception as ex:
            print(f"WARN: could not digest schema {sp}: {ex}", file=sys.stderr)
            continue

        dest = artifact_cas.store_artifact_file(
            "schema",
            digest,
            sp,
            repo_root=REPO_ROOT,
            store_root=pathlib.Path(store_root) if store_root else None,
            overwrite=overwrite,
        )
        stored.append({
            "schema": sp.relative_to(REPO_ROOT).as_posix(),
            "digest": digest,
            "path": dest.as_posix(),
        })

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} schema artifacts")
    return 0


def cmd_artifact_index_vcs(args: argparse.Namespace) -> int:
    """Populate dist/artifacts/vc with content-addressed copies of VCs (payload digest)."""
    store_root = str(getattr(args, "store_root", "") or "").strip()
    overwrite = bool(getattr(args, "overwrite", False))

    from tools.vc import signing_input  # type: ignore

    search_roots = [
        REPO_ROOT / "modules",
        REPO_ROOT / "docs" / "examples" / "vc",
        REPO_ROOT / "tests" / "fixtures",
    ]

    stored = []
    seen_paths = set()

    for root in search_roots:
        if not root.exists():
            continue
        for vp in sorted(root.rglob("*.vc.json")):
            if not vp.is_file():
                continue
            try:
                rel = vp.relative_to(REPO_ROOT).as_posix()
            except Exception:
                rel = vp.as_posix()
            if rel in seen_paths:
                continue
            seen_paths.add(rel)

            try:
                vcj = load_json(vp)
                digest = sha256_bytes(signing_input(vcj))
            except Exception as ex:
                print(f"WARN: could not digest VC {vp}: {ex}", file=sys.stderr)
                continue

            dest = artifact_cas.store_artifact_file(
                "vc",
                digest,
                vp,
                repo_root=REPO_ROOT,
                store_root=pathlib.Path(store_root) if store_root else None,
                overwrite=overwrite,
            )
            stored.append({"vc": rel, "digest": digest, "path": dest.as_posix()})

    if getattr(args, "json", False):
        print(json.dumps({"stored": stored, "count": len(stored)}, indent=2))
    else:
        print(f"Stored {len(stored)} VC artifacts")
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
    l.add_argument(
        "--emit-artifactrefs",
        action="store_true",
        help=(
            "Emit typed ArtifactRef objects for digest-bearing fields (lawpacks + corridor artifacts) "
            "instead of legacy raw sha256 strings."
        ),
    )
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
    cdef.add_argument("--require-domain", dest="require_domain", action="append", default=[], help="Require a lawpack domain for corridor participants (repeatable; default: civil+financial)")
    cdef.add_argument("--allow-lawpack", dest="allow_lawpack", action="append", default=[], help="Allow a specific lawpack digest (<jurisdiction_id>:<domain>:<sha256>) (repeatable)")
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
    cagr.add_argument("--from-zone-lock", dest="from_zone_lock", default="", help="Populate pinned_lawpacks from a zone stack.lock")
    cagr.add_argument("--pinned-lawpack", dest="pinned_lawpack", action="append", default=[], help="Pinned lawpack ref (<jurisdiction_id>:<domain>:<sha256>) (repeatable)")
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



    # Corridor state channels (verifiable receipts)
    cstate = cor_sub.add_parser("state")
    cstate_sub = cstate.add_subparsers(dest="corridor_state_cmd", required=True)

    cg = cstate_sub.add_parser("genesis-root", help="Compute corridor genesis_root")
    cg.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cg.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    cg.set_defaults(func=cmd_corridor_state_genesis_root)

    cri = cstate_sub.add_parser("receipt-init", help="Create a corridor state receipt and optionally sign it")
    cri.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cri.add_argument("--sequence", type=int, default=0, help="Receipt sequence number (default: 0)")
    cri.add_argument("--prev-root", default="genesis", help="prev_root (default: genesis). Use 64-hex or 'genesis'")
    cri.add_argument("--timestamp", default="", help="RFC3339 timestamp override")
    cri.add_argument("--transition", default="", help="Path to a JSON transition object (default: noop)")
    cri.add_argument(
        "--fill-transition-digests",
        action="store_true",
        help="Populate transition envelope digest references from the configured transition type registry (default: omit and rely on the registry snapshot digest commitment)",
    )
    cri.add_argument("--lawpack-digest", dest="lawpack_digest", action="append", default=[], help="Lawpack digest sha256 to include (repeatable; default: derive from agreement-set)")
    cri.add_argument("--ruleset-digest", dest="ruleset_digest", action="append", default=[], help="Ruleset digest sha256 to include (repeatable; default: derive from corridor rulesets)")
    cri.add_argument("--out", default="", help="Output path (default: corridor-receipt.<seq>.json)")
    cri.add_argument("--sign", action="store_true", help="Sign the receipt")
    cri.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    cri.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    cri.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    cri.set_defaults(func=cmd_corridor_state_receipt_init)

    csv = cstate_sub.add_parser("verify", help="Verify a corridor receipt chain and print the final root")
    csv.add_argument("path", help="Corridor module directory or corridor.yaml path")
    csv.add_argument("--receipts", required=True, help="Receipt file or directory containing receipt JSON files")
    csv.add_argument("--enforce-trust-anchors", action="store_true", help="Require at least one valid proof from a trust anchor authorized for corridor_receipt")
    csv.add_argument(
        "--enforce-receipt-threshold",
        action="store_true",
        help=(
            "Enforce receipt signing thresholds derived from Corridor Agreement VC(s) (recommended for fork resistance). "
            "Defaults to the agreement's state_channel.receipt_signing thresholds when present; otherwise falls back to activation thresholds."
        ),
    )
    csv.add_argument(
        "--enforce-transition-types",
        action="store_true",
        help="Require that receipt transitions resolve against the corridor's transition type registry; when a receipt does not bind to a registry snapshot digest, require explicit per-transition digest references",
    )
    csv.add_argument(
        "--require-artifacts",
        action="store_true",
        help="Fail verification if any digest commitment in receipts cannot be resolved via the artifact CAS (dist/artifacts/<type>/<digest>.* or configured store roots)",
    )
    csv.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    csv.set_defaults(func=cmd_corridor_state_verify)


    ccp = cstate_sub.add_parser("checkpoint", help="Create a corridor state checkpoint (MMR root) and optionally sign it")
    ccp.add_argument("path", help="Corridor module directory or corridor.yaml path")
    ccp.add_argument("--receipts", required=True, help="Receipt file or directory containing receipt JSON files")
    ccp.add_argument("--enforce-trust-anchors", action="store_true", help="Require at least one valid proof from a trust anchor authorized for corridor_receipt")
    ccp.add_argument("--out", default="", help="Output path (default: corridor.checkpoint.json)")
    ccp.add_argument("--sign", action="store_true", help="Sign the checkpoint")
    ccp.add_argument("--key", default="", help="Private OKP/Ed25519 JWK for signing (required with --sign)")
    ccp.add_argument("--verification-method", default="", help="verificationMethod override (default: did:key derived from JWK + '#key-1')")
    ccp.add_argument("--purpose", default="assertionMethod", help="proofPurpose (default: assertionMethod)")
    ccp.set_defaults(func=cmd_corridor_state_checkpoint)

    cpr = cstate_sub.add_parser("proof", help="Generate an MMR inclusion proof for a receipt sequence")
    cpr.add_argument("path", help="Corridor module directory or corridor.yaml path")
    cpr.add_argument("--receipts", required=True, help="Receipt directory containing receipt JSON files")
    cpr.add_argument("--sequence", required=True, type=int, help="Receipt sequence number (leaf index) to prove")
    cpr.add_argument("--checkpoint", default="", help="Optional checkpoint path; when provided, the proof is checked against it")
    cpr.add_argument("--enforce-trust-anchors", action="store_true", help="Verify receipt signatures and require trust anchors")
    cpr.add_argument("--out", default="", help="Output path (default: corridor.inclusion-proof.<seq>.json)")
    cpr.set_defaults(func=cmd_corridor_state_inclusion_proof)

    civ = cstate_sub.add_parser("verify-inclusion", help="Verify an inclusion proof against a signed checkpoint")
    civ.add_argument("path", help="Corridor module directory or corridor.yaml path")
    civ.add_argument("--receipt", required=True, help="Receipt JSON file")
    civ.add_argument("--proof", required=True, help="Inclusion proof JSON file")
    civ.add_argument("--checkpoint", required=True, help="Signed checkpoint JSON file")
    civ.add_argument("--enforce-trust-anchors", action="store_true", help="Require receipt/checkpoint signatures from trust anchors")
    civ.set_defaults(func=cmd_corridor_state_verify_inclusion)


    c = sub.add_parser("check-coverage")
    c.add_argument("--profile", default="", help="Restrict check to modules in a profile.yaml")
    c.add_argument("--zone", default="", help="Restrict check to modules in the zone's profile")
    c.set_defaults(func=cmd_check_coverage)


    law = sub.add_parser("law")
    law_sub = law.add_subparsers(dest="law_cmd", required=True)

    ll = law_sub.add_parser("list")
    ll.add_argument("--jurisdiction", default="", help="Filter by jurisdiction_id (e.g., us-ca)")
    ll.add_argument("--domain", default="", help="Filter by domain (civil|financial)")
    ll.add_argument("--json", action="store_true")
    ll.set_defaults(func=cmd_law_list)

    lc = law_sub.add_parser("coverage")
    lc.add_argument("--json", action="store_true")
    lc.set_defaults(func=cmd_law_coverage)


    li = law_sub.add_parser("ingest")
    li.add_argument("module", help="Path to a jurisdiction corpus module directory (e.g., modules/legal/jurisdictions/us/ca/civil)")
    li.add_argument("--as-of-date", dest="as_of_date", required=True, help="Snapshot date (YYYY-MM-DD)")
    li.add_argument("--out-dir", dest="out_dir", default="dist/lawpacks", help="Output directory for lawpack artifacts")
    li.add_argument("--fetch", action="store_true", help="Fetch declared sources into src/raw (best-effort)")
    li.add_argument("--include-raw", dest="include_raw", action="store_true", help="Include src/raw files inside the lawpack.zip (license permitting)")
    li.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    li.set_defaults(func=cmd_law_ingest)

    latt = law_sub.add_parser("attest-init", help="Initialize a Lawpack Validity Attestation VC skeleton")
    latt.add_argument("--jurisdiction-id", required=True)
    latt.add_argument("--domain", required=True)
    latt.add_argument("--lawpack-digest", required=True)
    latt.add_argument("--as-of-date", required=True, help="Snapshot date (YYYY-MM-DD)")
    latt.add_argument("--issuer", default="", help="Issuer DID (e.g., did:key:z6Mk...)")
    latt.add_argument("--out", default="", help="Output path for the VC JSON")
    latt.set_defaults(func=cmd_law_attest_init)



    art = sub.add_parser("artifact")
    art_sub = art.add_subparsers(dest="artifact_cmd", required=True)

    astore = art_sub.add_parser("store", help="Store an artifact into the content-addressed CAS")
    astore.add_argument("type", help="Artifact type (e.g., lawpack|ruleset|transition-types|circuit|schema|vc|checkpoint|proof-key|blob)")
    astore.add_argument("digest", help="sha256 digest (64-hex)")
    astore.add_argument("path", help="Source file path")
    astore.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    astore.add_argument("--name", default="", help="Optional destination filename override")
    astore.add_argument("--overwrite", action="store_true", help="Overwrite existing file if present")
    astore.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    astore.set_defaults(func=cmd_artifact_store)

    ares = art_sub.add_parser("resolve", help="Resolve an artifact by (type,digest) via CAS")
    ares.add_argument("type", help="Artifact type")
    ares.add_argument("digest", help="sha256 digest (64-hex)")
    ares.add_argument("--store-root", dest="store_root", action="append", default=[], help="Additional store root to search (repeatable)")
    ares.add_argument("--show", action="store_true", help="Print text artifacts instead of just the path")
    ares.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ares.set_defaults(func=cmd_artifact_resolve)

    air = art_sub.add_parser("index-rulesets", help="Populate dist/artifacts/ruleset with content-addressed ruleset descriptors")
    air.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    air.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    air.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    air.set_defaults(func=cmd_artifact_index_rulesets)

    ail = art_sub.add_parser("index-lawpacks", help="Populate dist/artifacts/lawpack by copying local dist/lawpacks/**/*.lawpack.zip")
    ail.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    ail.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    ail.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ail.set_defaults(func=cmd_artifact_index_lawpacks)


    ais = art_sub.add_parser("index-schemas", help="Populate dist/artifacts/schema with content-addressed JSON Schemas")
    ais.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    ais.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    ais.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    ais.set_defaults(func=cmd_artifact_index_schemas)

    aiv = art_sub.add_parser("index-vcs", help="Populate dist/artifacts/vc with content-addressed VCs (payload digests)")
    aiv.add_argument("--store-root", dest="store_root", default="", help="Store root (default: dist/artifacts)")
    aiv.add_argument("--overwrite", action="store_true", help="Overwrite existing files")
    aiv.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    aiv.set_defaults(func=cmd_artifact_index_vcs)

    reg = sub.add_parser("registry")
    reg_sub = reg.add_subparsers(dest="registry_cmd", required=True)

    rtl = reg_sub.add_parser("transition-types-lock", help="Generate a transition type registry lock (transition-types.lock.json)")
    rtl.add_argument("registry", help="Path to a transition type registry YAML (transition-types.yaml)")
    rtl.add_argument("--out", default="", help="Output path (default: <registry>.lock.json)")
    rtl.add_argument("--no-store", action="store_true", help="Do not write a content-addressed copy into dist/artifacts/transition-types")
    rtl.add_argument("--store-dir", default="", help="Content-addressed store directory (default: dist/artifacts/transition-types)")
    rtl.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rtl.set_defaults(func=cmd_registry_transition_types_lock)

    rts = reg_sub.add_parser("transition-types-store", help="Store a transition type registry lock snapshot by digest")
    rts.add_argument("lock", help="Path to a transition-types.lock.json")
    rts.add_argument("--store-dir", default="", help="Store directory (default: dist/artifacts/transition-types)")
    rts.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rts.set_defaults(func=cmd_registry_transition_types_store)

    rtr = reg_sub.add_parser("transition-types-resolve", help="Resolve a transition type registry lock snapshot by digest")
    rtr.add_argument("digest", help="snapshot_digest_sha256 (64-hex)")
    rtr.add_argument("--store-dir", action="append", default=[], help="Additional search directory (repeatable)")
    rtr.add_argument("--show", action="store_true", help="Print the lock JSON instead of the path")
    rtr.add_argument("--json", action="store_true", help="Output machine-readable JSON")
    rtr.set_defaults(func=cmd_registry_transition_types_resolve)

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
