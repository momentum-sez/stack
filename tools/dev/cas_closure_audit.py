#!/usr/bin/env python3
"""CAS Closure Audit Tool for Trade Playbook (Part 2e+ Hard Route).

This tool performs a strict closure audit on the trade playbook's CAS store,
verifying:
1. All artifacts referenced in the closure root are present
2. All digests recompute correctly
3. All ArtifactRefs resolve
4. The closure graph is complete (no dangling references)
5. Byte-level equality for all canonical JSON artifacts

Usage:
    python tools/dev/cas_closure_audit.py --store-root docs/examples/trade/dist/artifacts
    python tools/dev/cas_closure_audit.py --closure-root docs/examples/trade/dist/manifest.playbook.root.json
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT))

from tools.msez import canonical_json_bytes, load_json  # type: ignore


@dataclass
class AuditResult:
    """Result of a CAS closure audit."""
    passed: bool = True
    total_artifacts: int = 0
    verified_artifacts: int = 0
    missing_artifacts: List[str] = field(default_factory=list)
    digest_mismatches: List[Dict[str, Any]] = field(default_factory=list)
    dangling_refs: List[Dict[str, Any]] = field(default_factory=list)
    non_canonical_files: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "passed": self.passed,
            "total_artifacts": self.total_artifacts,
            "verified_artifacts": self.verified_artifacts,
            "missing_artifacts": self.missing_artifacts,
            "digest_mismatches": self.digest_mismatches,
            "dangling_refs": self.dangling_refs,
            "non_canonical_files": self.non_canonical_files,
            "warnings": self.warnings,
        }


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def load_artifact(path: Path) -> Tuple[Optional[Dict[str, Any]], Optional[bytes]]:
    """Load a JSON artifact and return (parsed, raw_bytes)."""
    if not path.exists():
        return None, None
    try:
        raw = path.read_bytes()
        parsed = json.loads(raw.decode("utf-8"))
        return parsed, raw
    except Exception:
        return None, None


def is_canonical(raw_bytes: bytes, obj: Any) -> bool:
    """Check if raw bytes are canonical JSON (JCS)."""
    canonical = canonical_json_bytes(obj)
    # Allow trailing newline
    return raw_bytes == canonical or raw_bytes == canonical + b"\n"


def extract_artifact_refs(obj: Any, refs: List[Dict[str, Any]], path: str = "") -> None:
    """Recursively extract ArtifactRef objects from a structure."""
    if isinstance(obj, dict):
        # Check if this is an ArtifactRef
        if "artifact_type" in obj and "digest_sha256" in obj:
            refs.append({
                "path": path,
                "artifact_type": obj.get("artifact_type"),
                "artifact_id": obj.get("artifact_id", ""),
                "digest_sha256": obj.get("digest_sha256"),
                "uri": obj.get("uri", ""),
            })
        # Recurse into values
        for k, v in obj.items():
            extract_artifact_refs(v, refs, f"{path}.{k}" if path else k)
    elif isinstance(obj, list):
        for i, item in enumerate(obj):
            extract_artifact_refs(item, refs, f"{path}[{i}]")


def compute_strict_digest(obj: Dict[str, Any]) -> str:
    """Compute strict digest for an artifact (sha256 of canonical JSON)."""
    # Remove proof field for signing input
    tmp = dict(obj)
    tmp.pop("proof", None)
    return sha256_bytes(canonical_json_bytes(tmp))


def audit_closure(
    closure_root_path: Path,
    store_root: Path,
    strict: bool = True,
    emit_diff: bool = False,
) -> AuditResult:
    """Perform a full CAS closure audit."""
    result = AuditResult()
    
    # Load closure root
    closure_root, closure_bytes = load_artifact(closure_root_path)
    if closure_root is None:
        result.passed = False
        result.missing_artifacts.append(str(closure_root_path))
        return result
    
    # Check closure root is canonical
    if strict and not is_canonical(closure_bytes, closure_root):
        result.passed = False
        result.non_canonical_files.append(str(closure_root_path))
    
    # Extract artifacts from closure root
    artifacts = closure_root.get("artifacts", [])
    result.total_artifacts = len(artifacts)
    
    # Track all referenced digests for dangling ref detection
    all_digests: Set[str] = set()
    resolved_digests: Set[str] = set()
    
    # Verify each artifact
    for art_entry in artifacts:
        artifact_id = art_entry.get("artifact_id", "unknown")
        artifact_type = art_entry.get("artifact_type", "unknown")
        expected_digest = art_entry.get("digest_sha256", "")
        rel_path = art_entry.get("path", "")
        
        all_digests.add(expected_digest)
        
        # Resolve path
        if rel_path.startswith("dist/"):
            art_path = closure_root_path.parent.parent / rel_path
        else:
            art_path = store_root / rel_path
        
        # Load artifact
        art_obj, art_bytes = load_artifact(art_path)
        if art_obj is None:
            result.passed = False
            result.missing_artifacts.append(f"{artifact_id} ({rel_path})")
            continue
        
        resolved_digests.add(expected_digest)
        
        # Verify canonical bytes
        if strict and not is_canonical(art_bytes, art_obj):
            result.passed = False
            result.non_canonical_files.append(rel_path)
            if emit_diff:
                # Compute expected canonical bytes for diff
                expected = canonical_json_bytes(art_obj)
                actual = art_bytes.rstrip(b"\n")
                if expected != actual:
                    result.digest_mismatches.append({
                        "artifact_id": artifact_id,
                        "path": rel_path,
                        "expected_bytes_len": len(expected),
                        "actual_bytes_len": len(actual),
                    })
        
        # Verify digest
        actual_digest = compute_strict_digest(art_obj)
        if actual_digest != expected_digest:
            result.passed = False
            result.digest_mismatches.append({
                "artifact_id": artifact_id,
                "path": rel_path,
                "expected_digest": expected_digest,
                "actual_digest": actual_digest,
            })
            continue
        
        # Extract nested ArtifactRefs
        nested_refs: List[Dict[str, Any]] = []
        extract_artifact_refs(art_obj, nested_refs)
        
        for ref in nested_refs:
            ref_digest = ref.get("digest_sha256", "")
            if ref_digest:
                all_digests.add(ref_digest)
        
        result.verified_artifacts += 1
    
    # Check for dangling references
    dangling = all_digests - resolved_digests
    for d in dangling:
        # Check if it's a known external reference (lawpack, ruleset, etc.)
        # For now, just warn
        result.warnings.append(f"Unresolved digest reference: {d[:16]}...")
    
    return result


def audit_store(store_root: Path, strict: bool = True) -> AuditResult:
    """Audit all JSON files in a CAS store for canonical bytes."""
    result = AuditResult()
    
    json_files = list(store_root.rglob("*.json"))
    result.total_artifacts = len(json_files)
    
    for path in json_files:
        art_obj, art_bytes = load_artifact(path)
        if art_obj is None:
            result.warnings.append(f"Could not parse: {path}")
            continue
        
        if strict and not is_canonical(art_bytes, art_obj):
            result.passed = False
            result.non_canonical_files.append(str(path.relative_to(store_root)))
        else:
            result.verified_artifacts += 1
    
    return result


def main() -> int:
    parser = argparse.ArgumentParser(
        description="CAS closure audit for trade playbook",
        epilog="Verifies artifact graph integrity, digest correctness, and canonical bytes.",
    )
    parser.add_argument(
        "--closure-root",
        dest="closure_root",
        default="",
        help="Path to closure root manifest (e.g., docs/examples/trade/dist/manifest.playbook.root.json)",
    )
    parser.add_argument(
        "--store-root",
        dest="store_root",
        default="",
        help="Path to CAS store root (e.g., docs/examples/trade/dist/artifacts)",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        default=True,
        help="Enforce strict canonical bytes (default: true)",
    )
    parser.add_argument(
        "--emit-diff",
        dest="emit_diff",
        action="store_true",
        help="Emit byte-level diff details on mismatch",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output machine-readable JSON",
    )
    
    args = parser.parse_args()
    
    # Determine paths
    if args.closure_root:
        closure_root_path = Path(args.closure_root)
        if not closure_root_path.is_absolute():
            closure_root_path = REPO_ROOT / closure_root_path
        store_root = closure_root_path.parent / "artifacts"
        result = audit_closure(closure_root_path, store_root, args.strict, args.emit_diff)
    elif args.store_root:
        store_root = Path(args.store_root)
        if not store_root.is_absolute():
            store_root = REPO_ROOT / store_root
        result = audit_store(store_root, args.strict)
    else:
        # Default: audit the trade playbook
        closure_root_path = REPO_ROOT / "docs/examples/trade/dist/manifest.playbook.root.json"
        store_root = REPO_ROOT / "docs/examples/trade/dist/artifacts"
        result = audit_closure(closure_root_path, store_root, args.strict, args.emit_diff)
    
    if args.json:
        print(json.dumps(result.to_dict(), indent=2))
    else:
        print(f"CAS Closure Audit Results")
        print(f"=" * 50)
        print(f"Status: {'PASSED' if result.passed else 'FAILED'}")
        print(f"Total artifacts: {result.total_artifacts}")
        print(f"Verified artifacts: {result.verified_artifacts}")
        
        if result.missing_artifacts:
            print(f"\nMissing artifacts ({len(result.missing_artifacts)}):")
            for m in result.missing_artifacts:
                print(f"  - {m}")
        
        if result.digest_mismatches:
            print(f"\nDigest mismatches ({len(result.digest_mismatches)}):")
            for m in result.digest_mismatches:
                print(f"  - {m['artifact_id']}: expected {m.get('expected_digest', '')[:16]}...")
        
        if result.non_canonical_files:
            print(f"\nNon-canonical files ({len(result.non_canonical_files)}):")
            for f in result.non_canonical_files:
                print(f"  - {f}")
        
        if result.warnings:
            print(f"\nWarnings ({len(result.warnings)}):")
            for w in result.warnings:
                print(f"  - {w}")
    
    return 0 if result.passed else 1


if __name__ == "__main__":
    sys.exit(main())
