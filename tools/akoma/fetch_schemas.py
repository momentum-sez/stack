#!/usr/bin/env python3
"""Fetch Akoma Ntoso schemas for offline validation.

This tool downloads:
- akomantoso30.xsd
- xml.xsd

Default source: docs.oasis-open.org.

In CI, these are fetched at runtime; deployments may vendor them if desired.
"""

from __future__ import annotations
import argparse
import pathlib
import urllib.request

DEFAULT_BASE = "https://docs.oasis-open.org/legaldocml/akn-core/v1.0/os/part2-specs/schemas/"

FILES = ["akomantoso30.xsd", "xml.xsd"]

def fetch(dest: pathlib.Path, base_url: str = DEFAULT_BASE) -> None:
    dest.mkdir(parents=True, exist_ok=True)
    for fn in FILES:
        url = base_url + fn
        out = dest / fn
        print("Downloading", url, "->", out)
        urllib.request.urlretrieve(url, out)

def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dest", default="tools/akoma/schemas", help="destination directory")
    ap.add_argument("--base-url", default=DEFAULT_BASE, help="base URL for schemas")
    args = ap.parse_args()

    fetch(pathlib.Path(args.dest), args.base_url)
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
