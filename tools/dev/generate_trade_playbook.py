#!/usr/bin/env python3
"""Deterministic generator + checker for the trade corridor playbook (Part 2e).

Combined Option 1 + Option 2 with Option 2 as the gate.

- mode=generate: writes canonical-bytes outputs under docs/examples/trade/dist/
- mode=check: validates the existing bytes match exactly (no writes).

The intent is that CI runs `--mode check` and fails on any drift.

Part 2e implements the FULL artifact graph:
- Zone locks (exporter/importer with deterministic timestamps)
- Corridor agreements (export/import corridors)
- Corridor receipts + checkpoint chains (obligation + settlement)
- Settlement plan/anchor linking obligation→settlement
- Proof bindings with evidence refs
- Closure root manifest with full artifact graph
- Dashboard JSON summary (cards + tables for UI rendering)

Determinism constraints:
- Fixed epoch: 2026-01-01T00:00:00Z (SOURCE_DATE_EPOCH=1735689600)
- Stable IDs: uuid5/URNs only (no uuid4)
- Canonical JSON (JCS) for all artifacts
- Byte-for-byte reproducibility verification
"""

from __future__ import annotations

import argparse
import hashlib
import os
import sys
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


REPO_ROOT = Path(__file__).resolve().parents[2]

# Ensure `import tools.*` works when invoked as a script.
sys.path.insert(0, str(REPO_ROOT))

from tools.msez import canonical_json_bytes, write_canonical_json_file, sha256_bytes  # type: ignore

# ─────────────────────────────────────────────────────────────────────────────
# Constants & Determinism
# ─────────────────────────────────────────────────────────────────────────────

STACK_SPEC_VERSION = "0.4.41"

# Fixed epoch for deterministic timestamps (2025-01-01T00:00:00Z)
# Matches the test and CI configuration: SOURCE_DATE_EPOCH=1735689600
SOURCE_DATE_EPOCH = 1735689600

# UUID5 namespace for deterministic IDs (MSEZ trade playbook namespace)
NAMESPACE_TRADE_PLAYBOOK = uuid.UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c8")


def deterministic_timestamp(offset_seconds: int = 0) -> str:
    """Return RFC3339 timestamp based on SOURCE_DATE_EPOCH + offset."""
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH", SOURCE_DATE_EPOCH))
    dt = datetime.fromtimestamp(epoch + offset_seconds, tz=timezone.utc)
    return dt.strftime("%Y-%m-%dT%H:%M:%SZ")


def deterministic_uuid5(name: str) -> str:
    """Generate deterministic UUID5 from name."""
    return str(uuid.uuid5(NAMESPACE_TRADE_PLAYBOOK, name))


def deterministic_urn(kind: str, name: str) -> str:
    """Generate deterministic URN: urn:msez:{kind}:{uuid5}."""
    return f"urn:msez:{kind}:{deterministic_uuid5(name)}"


def artifact_digest(obj: Any) -> str:
    """Compute sha256 digest of canonical JSON bytes."""
    return sha256_bytes(canonical_json_bytes(obj))


# ─────────────────────────────────────────────────────────────────────────────
# Configuration
# ─────────────────────────────────────────────────────────────────────────────

@dataclass(frozen=True)
class Config:
    mode: str
    docs_root: Path
    dist_root: Path
    src_root: Path
    store_root: Path


@dataclass
class ArtifactRegistry:
    """Tracks all generated artifacts for closure audit."""
    artifacts: Dict[str, Dict[str, Any]] = field(default_factory=dict)
    
    def register(self, artifact_id: str, artifact_type: str, path: str, digest: str, obj: Any) -> None:
        self.artifacts[artifact_id] = {
            "artifact_id": artifact_id,
            "artifact_type": artifact_type,
            "path": path,
            "digest_sha256": digest,
            "object": obj,
        }
    
    def get_ref(self, artifact_id: str) -> Dict[str, Any]:
        """Return ArtifactRef for the given artifact."""
        art = self.artifacts[artifact_id]
        return {
            "artifact_type": art["artifact_type"],
            "artifact_id": artifact_id,
            "digest_sha256": art["digest_sha256"],
            "uri": f"file://{art['path']}",
        }


# ─────────────────────────────────────────────────────────────────────────────
# File I/O Helpers
# ─────────────────────────────────────────────────────────────────────────────

def _read_bytes(path: Path) -> bytes:
    return path.read_bytes()


def _canonical_bytes_match(path: Path, obj: Any) -> bool:
    expected = canonical_json_bytes(obj)
    actual = _read_bytes(path)
    if actual == expected:
        return True
    if actual == expected + b"\n":
        return True
    return False


def _ensure_dir(path: Path, mode: str) -> None:
    if path.exists():
        if not path.is_dir():
            raise SystemExit(f"Expected directory, found file: {path}")
        return
    if mode == "check":
        raise SystemExit(f"Missing directory in check mode: {path}")
    path.mkdir(parents=True, exist_ok=True)


def _write_or_check_json(path: Path, obj: Any, mode: str) -> None:
    if path.exists():
        if not path.is_file():
            raise SystemExit(f"Expected file, found directory: {path}")
        if _canonical_bytes_match(path, obj):
            return
        if mode == "check":
            raise SystemExit(f"File is not canonical or content differs: {path}")
        # generate mode: overwrite with canonical bytes
        write_canonical_json_file(path, obj)
        return

    if mode == "check":
        raise SystemExit(f"Missing file in check mode: {path}")
    path.parent.mkdir(parents=True, exist_ok=True)
    write_canonical_json_file(path, obj)


# ─────────────────────────────────────────────────────────────────────────────
# Zone Lock Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_zone_lock(zone_id: str, jurisdiction_id: str, jurisdiction_stack: List[str],
                    corridors: List[str], lawpack_digests: List[str], offset: int = 0) -> Dict[str, Any]:
    """Build a deterministic zone lock artifact."""
    return {
        "type": "MSEZZoneLock",
        "stack_spec_version": STACK_SPEC_VERSION,
        "zone_id": zone_id,
        "jurisdiction_id": jurisdiction_id,
        "jurisdiction_stack": jurisdiction_stack,
        "locked_at": deterministic_timestamp(offset),
        "profile": {
            "profile_id": "org.momentum.msez.profile.trade-playbook",
            "version": STACK_SPEC_VERSION,
        },
        "lawpack_domains": ["civil", "financial"],
        "lawpack_digest_set": sorted(lawpack_digests),
        "corridors": corridors,
        "key_rotation_policy": {
            "default": {
                "rotation_days": 90,
                "grace_days": 14,
            }
        },
    }


# ─────────────────────────────────────────────────────────────────────────────
# Corridor Agreement Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_corridor_agreement(
    corridor_id: str,
    corridor_name: str,
    corridor_type: str,
    exporter_zone_ref: Dict[str, Any],
    importer_zone_ref: Dict[str, Any],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic corridor agreement."""
    return {
        "type": "MSEZCorridorAgreement",
        "stack_spec_version": STACK_SPEC_VERSION,
        "corridor_id": corridor_id,
        "corridor_name": corridor_name,
        "corridor_type": corridor_type,
        "created_at": deterministic_timestamp(offset),
        "parties": [
            {
                "role": "exporter",
                "zone_ref": exporter_zone_ref,
            },
            {
                "role": "importer",
                "zone_ref": importer_zone_ref,
            },
        ],
        "governance": {
            "dispute_resolution": "arbitration",
            "governing_law": "ae-dubai-difc",
        },
        "settlement_corridors": [
            "org.momentum.msez.corridor.swift.iso20022-cross-border",
            "org.momentum.msez.corridor.stablecoin.regulated-stablecoin",
        ],
    }


# ─────────────────────────────────────────────────────────────────────────────
# Corridor Receipt & Checkpoint Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_corridor_checkpoint(
    corridor_id: str,
    sequence: int,
    checkpoint_type: str,
    prev_digest: str,
    payload: Dict[str, Any],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic corridor checkpoint."""
    checkpoint = {
        "type": "MSEZCorridorCheckpoint",
        "stack_spec_version": STACK_SPEC_VERSION,
        "corridor_id": corridor_id,
        "sequence": sequence,
        "checkpoint_type": checkpoint_type,
        "timestamp": deterministic_timestamp(offset),
        "prev_checkpoint_digest": prev_digest,
        "payload": payload,
    }
    # Compute self-referential digest (without proof)
    checkpoint["checkpoint_digest"] = artifact_digest(checkpoint)
    return checkpoint


def build_corridor_receipt(
    corridor_id: str,
    sequence: int,
    prev_root: str,
    lawpack_digests: List[str],
    ruleset_digests: List[str],
    transition_payload: Dict[str, Any],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic corridor receipt."""
    receipt_base = {
        "type": "MSEZCorridorStateReceipt",
        "stack_spec_version": STACK_SPEC_VERSION,
        "corridor_id": corridor_id,
        "sequence": sequence,
        "timestamp": deterministic_timestamp(offset),
        "prev_root": prev_root,
        "lawpack_digest_set": sorted(lawpack_digests),
        "ruleset_digest_set": sorted(ruleset_digests),
        "transition": transition_payload,
    }
    # Compute next_root as sha256(JCS(receipt_without_proof_and_next_root))
    next_root = artifact_digest(receipt_base)
    receipt_base["next_root"] = next_root
    # Add placeholder proof (would be signed in production)
    receipt_base["proof"] = {
        "type": "DataIntegrityProof",
        "cryptosuite": "eddsa-jcs-2022",
        "created": deterministic_timestamp(offset),
        "verificationMethod": "did:key:z6MkExample#key-1",
        "proofPurpose": "assertionMethod",
        "proofValue": "z" + "0" * 86,  # Placeholder deterministic signature
    }
    return receipt_base


# ─────────────────────────────────────────────────────────────────────────────
# Settlement Plan & Anchor Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_settlement_plan(
    plan_id: str,
    obligation_refs: List[Dict[str, Any]],
    settlement_legs: List[Dict[str, Any]],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic settlement plan."""
    return {
        "type": "MSEZCorridorSettlementPlan",
        "stack_spec_version": STACK_SPEC_VERSION,
        "plan_id": plan_id,
        "created_at": deterministic_timestamp(offset),
        "netting_method": "single-currency-greedy",
        "obligations": obligation_refs,
        "netting": [
            {
                "party": "did:key:z6MkExporter",
                "currency": "USD",
                "net_amount": 50000,
            },
            {
                "party": "did:key:z6MkImporter",
                "currency": "USD",
                "net_amount": -50000,
            },
        ],
        "settlement_legs": settlement_legs,
    }


def build_settlement_anchor(
    anchor_id: str,
    plan_ref: Dict[str, Any],
    settlement_corridor_id: str,
    settlement_receipt_ref: Dict[str, Any],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic settlement anchor."""
    return {
        "type": "MSEZCorridorSettlementAnchor",
        "stack_spec_version": STACK_SPEC_VERSION,
        "anchor_id": anchor_id,
        "created_at": deterministic_timestamp(offset),
        "plan_ref": plan_ref,
        "settlement_corridor_id": settlement_corridor_id,
        "settlement_receipt_ref": settlement_receipt_ref,
        "finality_status": {
            "level": "confirmed",
            "confirmed_at": deterministic_timestamp(offset + 60),
        },
    }


# ─────────────────────────────────────────────────────────────────────────────
# Proof Binding Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_proof_binding(
    purpose: str,
    proof_ref: Dict[str, Any],
    commitments: List[Dict[str, Any]],
    offset: int = 0,
) -> Dict[str, Any]:
    """Build a deterministic proof binding."""
    return {
        "type": "MSEZProofBinding",
        "stack_spec_version": STACK_SPEC_VERSION,
        "issued_at": deterministic_timestamp(offset),
        "issuer": "did:key:z6MkAuditor",
        "purpose": purpose,
        "proof_ref": proof_ref,
        "commitments": commitments,
    }


# ─────────────────────────────────────────────────────────────────────────────
# Dashboard JSON Generation
# ─────────────────────────────────────────────────────────────────────────────

def build_dashboard_json(
    registry: ArtifactRegistry,
    cfg: Config,
) -> Dict[str, Any]:
    """Build dashboard JSON for UI rendering (cards + tables)."""
    artifacts = list(registry.artifacts.values())
    
    # Group by type
    by_type: Dict[str, List[Dict[str, Any]]] = {}
    for art in artifacts:
        t = art["artifact_type"]
        if t not in by_type:
            by_type[t] = []
        by_type[t].append(art)
    
    # Build cards (summary stats)
    cards = [
        {
            "card_id": "total-artifacts",
            "title": "Total Artifacts",
            "value": len(artifacts),
            "icon": "file-check",
        },
        {
            "card_id": "zone-locks",
            "title": "Zone Locks",
            "value": len(by_type.get("zone-lock", [])),
            "icon": "lock",
        },
        {
            "card_id": "corridor-agreements",
            "title": "Corridor Agreements",
            "value": len(by_type.get("corridor-agreement", [])),
            "icon": "handshake",
        },
        {
            "card_id": "receipts",
            "title": "Corridor Receipts",
            "value": len(by_type.get("corridor-receipt", [])),
            "icon": "receipt",
        },
        {
            "card_id": "checkpoints",
            "title": "Checkpoints",
            "value": len(by_type.get("corridor-checkpoint", [])),
            "icon": "flag",
        },
        {
            "card_id": "settlement-plans",
            "title": "Settlement Plans",
            "value": len(by_type.get("settlement-plan", [])),
            "icon": "calculator",
        },
        {
            "card_id": "proof-bindings",
            "title": "Proof Bindings",
            "value": len(by_type.get("proof-binding", [])),
            "icon": "shield-check",
        },
    ]
    
    # Build tables
    tables = {
        "artifacts": {
            "columns": ["artifact_id", "artifact_type", "path", "digest_sha256"],
            "rows": [
                {
                    "artifact_id": art["artifact_id"],
                    "artifact_type": art["artifact_type"],
                    "path": art["path"],
                    "digest_sha256": art["digest_sha256"][:16] + "...",
                }
                for art in artifacts
            ],
        },
    }
    
    return {
        "type": "MSEZTradePlaybookDashboard",
        "stack_spec_version": STACK_SPEC_VERSION,
        "generated_at": deterministic_timestamp(),
        "cards": cards,
        "tables": tables,
    }


# ─────────────────────────────────────────────────────────────────────────────
# Closure Root Manifest
# ─────────────────────────────────────────────────────────────────────────────

def build_closure_root(
    registry: ArtifactRegistry,
    cfg: Config,
) -> Dict[str, Any]:
    """Build closure root manifest with full artifact graph."""
    artifact_refs = [
        {
            "artifact_id": art["artifact_id"],
            "artifact_type": art["artifact_type"],
            "digest_sha256": art["digest_sha256"],
            "path": art["path"],
        }
        for art in registry.artifacts.values()
    ]
    
    # Sort for determinism
    artifact_refs.sort(key=lambda x: x["artifact_id"])
    
    return {
        "type": "MSEZTradePlaybookClosureRoot",
        "stack_spec_version": STACK_SPEC_VERSION,
        "scenario_id": "trade:example:baseline",
        "generated_at": deterministic_timestamp(),
        "zones": [
            {
                "name": "exporter",
                "zone_yaml": "src/zones/exporter/zone.yaml",
            },
            {
                "name": "importer",
                "zone_yaml": "src/zones/importer/zone.yaml",
            },
        ],
        "artifact_count": len(artifact_refs),
        "artifacts": artifact_refs,
        "dist": {
            "store_root": "dist/artifacts",
        },
    }


# ─────────────────────────────────────────────────────────────────────────────
# Main Generation Pipeline
# ─────────────────────────────────────────────────────────────────────────────

def run(cfg: Config) -> None:
    """Execute the full artifact generation pipeline."""
    _ensure_dir(cfg.docs_root, cfg.mode)
    _ensure_dir(cfg.src_root, cfg.mode)
    _ensure_dir(cfg.dist_root, cfg.mode)
    _ensure_dir(cfg.store_root, cfg.mode)
    
    registry = ArtifactRegistry()
    
    # ─────────────────────────────────────────────────────────────────────────
    # 1. Zone Locks
    # ─────────────────────────────────────────────────────────────────────────
    
    # Placeholder lawpack digests (deterministic)
    lawpack_civil = sha256_bytes(b"lawpack:civil:v1")
    lawpack_financial = sha256_bytes(b"lawpack:financial:v1")
    lawpack_digests = [lawpack_civil, lawpack_financial]
    
    corridors = [
        "org.momentum.msez.corridor.swift.iso20022-cross-border",
        "org.momentum.msez.corridor.stablecoin.regulated-stablecoin",
    ]
    
    # Exporter zone lock
    exporter_lock = build_zone_lock(
        zone_id="org.momentum.msez.zone.trade-playbook.exporter",
        jurisdiction_id="ae-dubai-difc",
        jurisdiction_stack=["ae", "ae-dubai", "ae-dubai-difc"],
        corridors=corridors,
        lawpack_digests=lawpack_digests,
        offset=0,
    )
    exporter_lock_path = cfg.store_root / "zone-locks" / "exporter.lock.json"
    exporter_lock_digest = artifact_digest(exporter_lock)
    _ensure_dir(exporter_lock_path.parent, cfg.mode)
    _write_or_check_json(exporter_lock_path, exporter_lock, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("zone-lock", "exporter"),
        artifact_type="zone-lock",
        path=str(exporter_lock_path.relative_to(cfg.docs_root)),
        digest=exporter_lock_digest,
        obj=exporter_lock,
    )
    
    # Importer zone lock
    importer_lock = build_zone_lock(
        zone_id="org.momentum.msez.zone.trade-playbook.importer",
        jurisdiction_id="us-ca",
        jurisdiction_stack=["us", "us-ca"],
        corridors=corridors,
        lawpack_digests=lawpack_digests,
        offset=10,
    )
    importer_lock_path = cfg.store_root / "zone-locks" / "importer.lock.json"
    importer_lock_digest = artifact_digest(importer_lock)
    _write_or_check_json(importer_lock_path, importer_lock, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("zone-lock", "importer"),
        artifact_type="zone-lock",
        path=str(importer_lock_path.relative_to(cfg.docs_root)),
        digest=importer_lock_digest,
        obj=importer_lock,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 2. Corridor Agreement
    # ─────────────────────────────────────────────────────────────────────────
    
    corridor_agreement = build_corridor_agreement(
        corridor_id="org.momentum.msez.corridor.trade-playbook.obligation",
        corridor_name="Trade Playbook Obligation Corridor",
        corridor_type="obligation",
        exporter_zone_ref=registry.get_ref(deterministic_urn("zone-lock", "exporter")),
        importer_zone_ref=registry.get_ref(deterministic_urn("zone-lock", "importer")),
        offset=100,
    )
    agreement_path = cfg.store_root / "agreements" / "obligation-corridor.agreement.json"
    agreement_digest = artifact_digest(corridor_agreement)
    _ensure_dir(agreement_path.parent, cfg.mode)
    _write_or_check_json(agreement_path, corridor_agreement, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-agreement", "obligation"),
        artifact_type="corridor-agreement",
        path=str(agreement_path.relative_to(cfg.docs_root)),
        digest=agreement_digest,
        obj=corridor_agreement,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 3. Obligation Corridor Receipts (Invoice → BOL → LC)
    # ─────────────────────────────────────────────────────────────────────────
    
    _ensure_dir(cfg.store_root / "receipts" / "obligation", cfg.mode)
    
    # Genesis root (deterministic)
    genesis_root = sha256_bytes(b"genesis:obligation:corridor")
    
    # Placeholder ruleset digests
    ruleset_invoice = sha256_bytes(b"ruleset:invoice:v1")
    ruleset_bol = sha256_bytes(b"ruleset:bol:v1")
    ruleset_lc = sha256_bytes(b"ruleset:lc:v1")
    
    # Receipt 0: Invoice issued
    receipt_0 = build_corridor_receipt(
        corridor_id="org.momentum.msez.corridor.trade-playbook.obligation",
        sequence=0,
        prev_root=genesis_root,
        lawpack_digests=lawpack_digests,
        ruleset_digests=[ruleset_invoice],
        transition_payload={
            "transition_type": "invoice.issued",
            "invoice_id": deterministic_urn("invoice", "INV-001"),
            "amount": {"currency": "USD", "value": 50000},
            "exporter_did": "did:key:z6MkExporter",
            "importer_did": "did:key:z6MkImporter",
        },
        offset=200,
    )
    receipt_0_path = cfg.store_root / "receipts" / "obligation" / "receipt-0.json"
    receipt_0_digest = artifact_digest(receipt_0)
    _write_or_check_json(receipt_0_path, receipt_0, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-receipt", "obligation-0"),
        artifact_type="corridor-receipt",
        path=str(receipt_0_path.relative_to(cfg.docs_root)),
        digest=receipt_0_digest,
        obj=receipt_0,
    )
    
    # Receipt 1: Bill of Lading issued
    receipt_1 = build_corridor_receipt(
        corridor_id="org.momentum.msez.corridor.trade-playbook.obligation",
        sequence=1,
        prev_root=receipt_0["next_root"],
        lawpack_digests=lawpack_digests,
        ruleset_digests=[ruleset_invoice, ruleset_bol],
        transition_payload={
            "transition_type": "bol.issued",
            "bol_id": deterministic_urn("bol", "BOL-001"),
            "invoice_ref": deterministic_urn("invoice", "INV-001"),
            "carrier": "Maersk Line",
            "vessel": "MSC Oscar",
        },
        offset=300,
    )
    receipt_1_path = cfg.store_root / "receipts" / "obligation" / "receipt-1.json"
    receipt_1_digest = artifact_digest(receipt_1)
    _write_or_check_json(receipt_1_path, receipt_1, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-receipt", "obligation-1"),
        artifact_type="corridor-receipt",
        path=str(receipt_1_path.relative_to(cfg.docs_root)),
        digest=receipt_1_digest,
        obj=receipt_1,
    )
    
    # Receipt 2: Letter of Credit issued
    receipt_2 = build_corridor_receipt(
        corridor_id="org.momentum.msez.corridor.trade-playbook.obligation",
        sequence=2,
        prev_root=receipt_1["next_root"],
        lawpack_digests=lawpack_digests,
        ruleset_digests=[ruleset_invoice, ruleset_bol, ruleset_lc],
        transition_payload={
            "transition_type": "lc.issued",
            "lc_id": deterministic_urn("lc", "LC-001"),
            "invoice_ref": deterministic_urn("invoice", "INV-001"),
            "issuing_bank": "Emirates NBD",
            "beneficiary_bank": "Wells Fargo",
        },
        offset=400,
    )
    receipt_2_path = cfg.store_root / "receipts" / "obligation" / "receipt-2.json"
    receipt_2_digest = artifact_digest(receipt_2)
    _write_or_check_json(receipt_2_path, receipt_2, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-receipt", "obligation-2"),
        artifact_type="corridor-receipt",
        path=str(receipt_2_path.relative_to(cfg.docs_root)),
        digest=receipt_2_digest,
        obj=receipt_2,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 4. Obligation Corridor Checkpoints
    # ─────────────────────────────────────────────────────────────────────────
    
    _ensure_dir(cfg.store_root / "checkpoints" / "obligation", cfg.mode)
    
    checkpoint_0 = build_corridor_checkpoint(
        corridor_id="org.momentum.msez.corridor.trade-playbook.obligation",
        sequence=0,
        checkpoint_type="milestone",
        prev_digest=genesis_root,
        payload={
            "milestone": "documents_complete",
            "receipt_refs": [
                registry.get_ref(deterministic_urn("corridor-receipt", "obligation-0")),
                registry.get_ref(deterministic_urn("corridor-receipt", "obligation-1")),
                registry.get_ref(deterministic_urn("corridor-receipt", "obligation-2")),
            ],
        },
        offset=500,
    )
    checkpoint_0_path = cfg.store_root / "checkpoints" / "obligation" / "checkpoint-0.json"
    checkpoint_0_digest = artifact_digest(checkpoint_0)
    _write_or_check_json(checkpoint_0_path, checkpoint_0, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-checkpoint", "obligation-0"),
        artifact_type="corridor-checkpoint",
        path=str(checkpoint_0_path.relative_to(cfg.docs_root)),
        digest=checkpoint_0_digest,
        obj=checkpoint_0,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 5. Settlement Corridor Receipts (SWIFT pacs.008)
    # ─────────────────────────────────────────────────────────────────────────
    
    _ensure_dir(cfg.store_root / "receipts" / "settlement", cfg.mode)
    
    settlement_genesis = sha256_bytes(b"genesis:settlement:corridor")
    ruleset_swift = sha256_bytes(b"ruleset:swift:pacs008:v1")
    
    settlement_receipt_0 = build_corridor_receipt(
        corridor_id="org.momentum.msez.corridor.swift.iso20022-cross-border",
        sequence=0,
        prev_root=settlement_genesis,
        lawpack_digests=lawpack_digests,
        ruleset_digests=[ruleset_swift],
        transition_payload={
            "transition_type": "swift.pacs008.initiated",
            "message_id": deterministic_urn("swift", "PACS008-001"),
            "amount": {"currency": "USD", "value": 50000},
            "debtor_bic": "ABORAEADXXX",
            "creditor_bic": "WFBIUS6SXXX",
        },
        offset=600,
    )
    settlement_receipt_0_path = cfg.store_root / "receipts" / "settlement" / "receipt-0.json"
    settlement_receipt_0_digest = artifact_digest(settlement_receipt_0)
    _write_or_check_json(settlement_receipt_0_path, settlement_receipt_0, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-receipt", "settlement-0"),
        artifact_type="corridor-receipt",
        path=str(settlement_receipt_0_path.relative_to(cfg.docs_root)),
        digest=settlement_receipt_0_digest,
        obj=settlement_receipt_0,
    )
    
    settlement_receipt_1 = build_corridor_receipt(
        corridor_id="org.momentum.msez.corridor.swift.iso20022-cross-border",
        sequence=1,
        prev_root=settlement_receipt_0["next_root"],
        lawpack_digests=lawpack_digests,
        ruleset_digests=[ruleset_swift],
        transition_payload={
            "transition_type": "swift.pacs008.confirmed",
            "message_id": deterministic_urn("swift", "PACS008-001"),
            "confirmation_ref": "SWIFT-CONF-12345",
        },
        offset=700,
    )
    settlement_receipt_1_path = cfg.store_root / "receipts" / "settlement" / "receipt-1.json"
    settlement_receipt_1_digest = artifact_digest(settlement_receipt_1)
    _write_or_check_json(settlement_receipt_1_path, settlement_receipt_1, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("corridor-receipt", "settlement-1"),
        artifact_type="corridor-receipt",
        path=str(settlement_receipt_1_path.relative_to(cfg.docs_root)),
        digest=settlement_receipt_1_digest,
        obj=settlement_receipt_1,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 6. Settlement Plan
    # ─────────────────────────────────────────────────────────────────────────
    
    _ensure_dir(cfg.store_root / "settlement", cfg.mode)
    
    settlement_plan = build_settlement_plan(
        plan_id=deterministic_urn("settlement-plan", "PLAN-001"),
        obligation_refs=[
            {
                "obligation_id": deterministic_urn("invoice", "INV-001"),
                "amount": {"currency": "USD", "value": 50000},
                "debtor": "did:key:z6MkImporter",
                "creditor": "did:key:z6MkExporter",
            },
        ],
        settlement_legs=[
            {
                "leg_id": deterministic_urn("settlement-leg", "LEG-001"),
                "corridor_id": "org.momentum.msez.corridor.swift.iso20022-cross-border",
                "amount": {"currency": "USD", "value": 50000},
                "payer": "did:key:z6MkImporter",
                "payee": "did:key:z6MkExporter",
            },
        ],
        offset=800,
    )
    settlement_plan_path = cfg.store_root / "settlement" / "plan.json"
    settlement_plan_digest = artifact_digest(settlement_plan)
    _write_or_check_json(settlement_plan_path, settlement_plan, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("settlement-plan", "PLAN-001"),
        artifact_type="settlement-plan",
        path=str(settlement_plan_path.relative_to(cfg.docs_root)),
        digest=settlement_plan_digest,
        obj=settlement_plan,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 7. Settlement Anchor
    # ─────────────────────────────────────────────────────────────────────────
    
    settlement_anchor = build_settlement_anchor(
        anchor_id=deterministic_urn("settlement-anchor", "ANCHOR-001"),
        plan_ref=registry.get_ref(deterministic_urn("settlement-plan", "PLAN-001")),
        settlement_corridor_id="org.momentum.msez.corridor.swift.iso20022-cross-border",
        settlement_receipt_ref=registry.get_ref(deterministic_urn("corridor-receipt", "settlement-1")),
        offset=900,
    )
    settlement_anchor_path = cfg.store_root / "settlement" / "anchor.json"
    settlement_anchor_digest = artifact_digest(settlement_anchor)
    _write_or_check_json(settlement_anchor_path, settlement_anchor, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("settlement-anchor", "ANCHOR-001"),
        artifact_type="settlement-anchor",
        path=str(settlement_anchor_path.relative_to(cfg.docs_root)),
        digest=settlement_anchor_digest,
        obj=settlement_anchor,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 8. Proof Bindings
    # ─────────────────────────────────────────────────────────────────────────
    
    _ensure_dir(cfg.store_root / "proof-bindings", cfg.mode)
    
    # Sanctions screening proof binding
    sanctions_binding = build_proof_binding(
        purpose="sanctions.screening.v1",
        proof_ref={
            "artifact_type": "external-attestation",
            "uri": "urn:sanctions:ofac:screening:2026-01-01",
            "digest_sha256": sha256_bytes(b"sanctions:ofac:clear"),
        },
        commitments=[
            {
                "kind": "corridor.receipt",
                "digest_sha256": receipt_0_digest,
                "corridor_id": "org.momentum.msez.corridor.trade-playbook.obligation",
                "sequence": 0,
            },
        ],
        offset=1000,
    )
    sanctions_binding_path = cfg.store_root / "proof-bindings" / "sanctions.json"
    sanctions_binding_digest = artifact_digest(sanctions_binding)
    _write_or_check_json(sanctions_binding_path, sanctions_binding, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("proof-binding", "sanctions"),
        artifact_type="proof-binding",
        path=str(sanctions_binding_path.relative_to(cfg.docs_root)),
        digest=sanctions_binding_digest,
        obj=sanctions_binding,
    )
    
    # Carrier event proof binding
    carrier_binding = build_proof_binding(
        purpose="carrier.event.v1",
        proof_ref={
            "artifact_type": "external-attestation",
            "uri": "urn:carrier:maersk:bol:verified",
            "digest_sha256": sha256_bytes(b"carrier:maersk:bol:verified"),
        },
        commitments=[
            {
                "kind": "corridor.receipt",
                "digest_sha256": receipt_1_digest,
                "corridor_id": "org.momentum.msez.corridor.trade-playbook.obligation",
                "sequence": 1,
            },
        ],
        offset=1100,
    )
    carrier_binding_path = cfg.store_root / "proof-bindings" / "carrier.json"
    carrier_binding_digest = artifact_digest(carrier_binding)
    _write_or_check_json(carrier_binding_path, carrier_binding, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("proof-binding", "carrier"),
        artifact_type="proof-binding",
        path=str(carrier_binding_path.relative_to(cfg.docs_root)),
        digest=carrier_binding_digest,
        obj=carrier_binding,
    )
    
    # Payment rails proof binding
    payment_binding = build_proof_binding(
        purpose="payment.confirmation.v1",
        proof_ref={
            "artifact_type": "external-attestation",
            "uri": "urn:swift:pacs008:confirmed",
            "digest_sha256": sha256_bytes(b"swift:pacs008:confirmed"),
        },
        commitments=[
            {
                "kind": "settlement-anchor",
                "digest_sha256": settlement_anchor_digest,
            },
        ],
        offset=1200,
    )
    payment_binding_path = cfg.store_root / "proof-bindings" / "payment.json"
    payment_binding_digest = artifact_digest(payment_binding)
    _write_or_check_json(payment_binding_path, payment_binding, cfg.mode)
    registry.register(
        artifact_id=deterministic_urn("proof-binding", "payment"),
        artifact_type="proof-binding",
        path=str(payment_binding_path.relative_to(cfg.docs_root)),
        digest=payment_binding_digest,
        obj=payment_binding,
    )
    
    # ─────────────────────────────────────────────────────────────────────────
    # 9. Dashboard JSON
    # ─────────────────────────────────────────────────────────────────────────
    
    dashboard = build_dashboard_json(registry, cfg)
    dashboard_path = cfg.dist_root / "dashboard.json"
    _write_or_check_json(dashboard_path, dashboard, cfg.mode)
    
    # ─────────────────────────────────────────────────────────────────────────
    # 10. Closure Root Manifest
    # ─────────────────────────────────────────────────────────────────────────
    
    closure_root = build_closure_root(registry, cfg)
    closure_root_path = cfg.dist_root / "manifest.playbook.root.json"
    _write_or_check_json(closure_root_path, closure_root, cfg.mode)
    
    # ─────────────────────────────────────────────────────────────────────────
    # 11. CAS Index (manifest of manifests)
    # ─────────────────────────────────────────────────────────────────────────
    
    cas_index = {
        "type": "MSEZCASIndex",
        "stack_spec_version": STACK_SPEC_VERSION,
        "generated_at": deterministic_timestamp(),
        "store_root": "dist/artifacts",
        "artifact_count": len(registry.artifacts),
        "digest_algorithm": "sha256",
        "manifests": [
            {
                "manifest_type": "closure-root",
                "path": "dist/manifest.playbook.root.json",
                "digest_sha256": artifact_digest(closure_root),
            },
            {
                "manifest_type": "dashboard",
                "path": "dist/dashboard.json",
                "digest_sha256": artifact_digest(dashboard),
            },
        ],
    }
    cas_index_path = cfg.store_root / "cas-index.json"
    _write_or_check_json(cas_index_path, cas_index, cfg.mode)
    
    if cfg.mode == "generate":
        print(f"Generated {len(registry.artifacts)} artifacts to {cfg.store_root}")
    else:
        print(f"Verified {len(registry.artifacts)} artifacts (byte-for-byte match)")


def parse_args(argv: list[str]) -> Config:
    p = argparse.ArgumentParser(
        description="Deterministic trade playbook generator (Part 2e)",
        epilog="CI runs with --mode check to verify byte-for-byte reproducibility.",
    )
    p.add_argument("--mode", choices=["generate", "check"], required=True,
                   help="generate: write artifacts; check: verify without writes")
    p.add_argument(
        "--docs-root",
        default=str(REPO_ROOT / "docs/examples/trade"),
        help="docs/examples/trade root",
    )
    args = p.parse_args(argv)

    docs_root = Path(args.docs_root).resolve()
    return Config(
        mode=args.mode,
        docs_root=docs_root,
        src_root=docs_root / "src",
        dist_root=docs_root / "dist",
        store_root=docs_root / "dist/artifacts",
    )


def main() -> None:
    cfg = parse_args(sys.argv[1:])
    run(cfg)


if __name__ == "__main__":
    main()
