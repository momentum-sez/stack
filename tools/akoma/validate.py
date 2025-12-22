#!/usr/bin/env python3
"""Validate Akoma Ntoso documents against XSD schemas."""

from __future__ import annotations
import argparse
import pathlib
from lxml import etree

DEFAULT_SCHEMA_DIR = pathlib.Path(__file__).resolve().parent / "schemas"

def load_schema(schema_dir: pathlib.Path) -> etree.XMLSchema:
    main_xsd = schema_dir / "akomantoso30.xsd"
    if not main_xsd.exists():
        raise FileNotFoundError(f"Missing schema file: {main_xsd}. Run tools/akoma/fetch_schemas.py first.")
    xmlschema_doc = etree.parse(str(main_xsd))
    return etree.XMLSchema(xmlschema_doc)

def validate_dir(target_dir: pathlib.Path, schema: etree.XMLSchema) -> list[str]:
    errors: list[str] = []
    for xml_file in target_dir.rglob("*.xml"):
        try:
            doc = etree.parse(str(xml_file))
            if not schema.validate(doc):
                for e in schema.error_log:
                    errors.append(f"{xml_file}: {e.message} (line {e.line})")
        except Exception as ex:
            errors.append(f"{xml_file}: {ex}")
    return errors

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("path", nargs="?", default="modules", help="directory to scan for Akoma XML")
    ap.add_argument("--schema-dir", default=str(DEFAULT_SCHEMA_DIR), help="directory containing akomantoso30.xsd")
    args = ap.parse_args()

    schema_dir = pathlib.Path(args.schema_dir)
    schema = load_schema(schema_dir)

    target = pathlib.Path(args.path)
    errs = validate_dir(target, schema)
    if errs:
        print("AKOMA VALIDATION FAILED")
        for e in errs[:200]:
            print(" -", e)
        if len(errs) > 200:
            print(f"... {len(errs)-200} more")
        return 2
    print("AKOMA VALIDATION OK")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
