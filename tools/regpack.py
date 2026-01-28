#!/usr/bin/env python3
"""RegPack Module: Dynamic Regulatory State Management.

This module implements the RegPack system for capturing dynamic regulatory state
that Lawpacks cannot represent:
- Sanctions lists (OFAC, EU, UN) - changes daily
- License registries - changes monthly
- Reporting deadlines - changes quarterly
- Regulatory guidance - changes weekly/monthly
- Enforcement priorities - changes daily/weekly

Usage:
    from tools.regpack import RegPackManager, SanctionsChecker

    manager = RegPackManager(jurisdiction_id="uae-adgm", domain="financial")
    regpack = manager.create_snapshot(as_of_date="2026-01-15")
    
    checker = SanctionsChecker(regpack)
    result = checker.check_entity("Acme Corp")
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import uuid
from dataclasses import dataclass, field
from datetime import date, datetime, timezone
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

# ─────────────────────────────────────────────────────────────────────────────
# Constants
# ─────────────────────────────────────────────────────────────────────────────

STACK_SPEC_VERSION = "0.4.42"
REGPACK_VERSION = "1.0"

NAMESPACE_REGPACK = uuid.UUID("6ba7b810-9dad-11d1-80b4-00c04fd430c9")

SANCTIONS_SOURCES = {
    "ofac_sdn": {
        "name": "OFAC SDN List",
        "url": "https://www.treasury.gov/ofac/downloads/sdn.xml",
        "format": "xml",
    },
    "ofac_sectoral": {
        "name": "OFAC Sectoral Sanctions",
        "url": "https://www.treasury.gov/ofac/downloads/consolidated/consolidated.xml",
        "format": "xml",
    },
    "eu_consolidated": {
        "name": "EU Consolidated Sanctions",
        "url": "https://webgate.ec.europa.eu/fsd/fsf/public/files/xmlFullSanctionsList_1_1/content",
        "format": "xml",
    },
    "un_consolidated": {
        "name": "UN Security Council Consolidated List",
        "url": "https://scsanctions.un.org/resources/xml/en/consolidated.xml",
        "format": "xml",
    },
    "uk_consolidated": {
        "name": "UK Sanctions List",
        "url": "https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/uk_sanctions_list.xml",
        "format": "xml",
    },
}


# ─────────────────────────────────────────────────────────────────────────────
# Data Types
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class SanctionsEntry:
    """A single entry in a sanctions list."""
    entry_id: str
    entry_type: str  # individual, entity, vessel, aircraft
    source_lists: List[str]
    primary_name: str
    aliases: List[Dict[str, str]] = field(default_factory=list)
    identifiers: List[Dict[str, str]] = field(default_factory=list)
    addresses: List[Dict[str, str]] = field(default_factory=list)
    nationalities: List[str] = field(default_factory=list)
    date_of_birth: Optional[str] = None
    programs: List[str] = field(default_factory=list)
    listing_date: Optional[str] = None
    remarks: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "entry_id": self.entry_id,
            "entry_type": self.entry_type,
            "source_lists": self.source_lists,
            "primary_name": self.primary_name,
        }
        if self.aliases:
            d["aliases"] = self.aliases
        if self.identifiers:
            d["identifiers"] = self.identifiers
        if self.addresses:
            d["addresses"] = self.addresses
        if self.nationalities:
            d["nationalities"] = self.nationalities
        if self.date_of_birth:
            d["date_of_birth"] = self.date_of_birth
        if self.programs:
            d["programs"] = self.programs
        if self.listing_date:
            d["listing_date"] = self.listing_date
        if self.remarks:
            d["remarks"] = self.remarks
        return d


@dataclass
class SanctionsSnapshot:
    """A point-in-time snapshot of consolidated sanctions."""
    snapshot_id: str
    snapshot_timestamp: str
    sources: Dict[str, Dict[str, Any]]
    entries: List[SanctionsEntry]
    consolidated_counts: Dict[str, int] = field(default_factory=dict)
    delta_from_previous: Optional[Dict[str, Any]] = None
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "type": "MSEZSanctionsSnapshot",
            "stack_spec_version": STACK_SPEC_VERSION,
            "snapshot_id": self.snapshot_id,
            "snapshot_timestamp": self.snapshot_timestamp,
            "sources": self.sources,
            "consolidated": self.consolidated_counts,
            "delta_from_previous": self.delta_from_previous,
        }


@dataclass
class RegulatorProfile:
    """Profile of a regulatory authority."""
    regulator_id: str
    name: str
    jurisdiction_id: str
    parent_authority: Optional[str] = None
    scope: Dict[str, List[str]] = field(default_factory=dict)
    contact: Dict[str, str] = field(default_factory=dict)
    api_capabilities: Dict[str, bool] = field(default_factory=dict)
    timezone: str = "UTC"
    business_days: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "regulator_id": self.regulator_id,
            "name": self.name,
            "jurisdiction_id": self.jurisdiction_id,
            "parent_authority": self.parent_authority,
            "scope": self.scope,
            "contact": self.contact,
            "api_capabilities": self.api_capabilities,
            "timezone": self.timezone,
            "business_days": self.business_days,
        }


@dataclass
class LicenseType:
    """A type of regulatory license."""
    license_type_id: str
    name: str
    regulator_id: str
    requirements: Dict[str, Any] = field(default_factory=dict)
    application: Dict[str, Any] = field(default_factory=dict)
    ongoing_obligations: Dict[str, Any] = field(default_factory=dict)
    validity_period_years: int = 1
    renewal_lead_time_days: int = 90
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "license_type_id": self.license_type_id,
            "name": self.name,
            "regulator_id": self.regulator_id,
            "requirements": self.requirements,
            "application": self.application,
            "ongoing_obligations": self.ongoing_obligations,
            "validity_period_years": self.validity_period_years,
            "renewal_lead_time_days": self.renewal_lead_time_days,
        }


@dataclass
class ReportingRequirement:
    """A regulatory reporting requirement."""
    report_type_id: str
    name: str
    regulator_id: str
    applicable_to: List[str]
    frequency: str  # daily, weekly, monthly, quarterly, annual
    deadlines: Dict[str, Dict[str, str]] = field(default_factory=dict)
    submission: Dict[str, Any] = field(default_factory=dict)
    late_penalty: Dict[str, Any] = field(default_factory=dict)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "report_type_id": self.report_type_id,
            "name": self.name,
            "regulator_id": self.regulator_id,
            "applicable_to": self.applicable_to,
            "frequency": self.frequency,
            "deadlines": self.deadlines,
            "submission": self.submission,
            "late_penalty": self.late_penalty,
        }


@dataclass
class ComplianceDeadline:
    """An upcoming compliance deadline."""
    deadline_id: str
    regulator_id: str
    deadline_type: str  # report, filing, renewal, payment
    description: str
    due_date: str
    grace_period_days: int = 0
    applicable_license_types: List[str] = field(default_factory=list)
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "deadline_id": self.deadline_id,
            "regulator_id": self.regulator_id,
            "deadline_type": self.deadline_type,
            "description": self.description,
            "due_date": self.due_date,
            "grace_period_days": self.grace_period_days,
            "applicable_license_types": self.applicable_license_types,
        }


@dataclass
class RegPackMetadata:
    """Metadata for a RegPack."""
    regpack_id: str
    jurisdiction_id: str
    domain: str
    as_of_date: str
    snapshot_type: str
    sources: List[Dict[str, Any]] = field(default_factory=list)
    includes: Dict[str, Any] = field(default_factory=dict)
    previous_regpack_digest: Optional[str] = None
    created_at: Optional[str] = None
    expires_at: Optional[str] = None
    digest_sha256: Optional[str] = None
    
    def to_dict(self) -> Dict[str, Any]:
        d = {
            "type": "MSEZRegPackMetadata",
            "stack_spec_version": STACK_SPEC_VERSION,
            "regpack_version": REGPACK_VERSION,
            "regpack_id": self.regpack_id,
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain,
            "as_of_date": self.as_of_date,
            "snapshot_type": self.snapshot_type,
            "sources": self.sources,
            "includes": self.includes,
        }
        if self.previous_regpack_digest:
            d["previous_regpack_digest"] = self.previous_regpack_digest
        if self.created_at:
            d["created_at"] = self.created_at
        if self.expires_at:
            d["expires_at"] = self.expires_at
        if self.digest_sha256:
            d["digest_sha256"] = self.digest_sha256
        return d


# ─────────────────────────────────────────────────────────────────────────────
# Sanctions Checker
# ─────────────────────────────────────────────────────────────────────────────

@dataclass
class SanctionsCheckResult:
    """Result of a sanctions check."""
    query: str
    checked_at: str
    snapshot_id: str
    matched: bool
    matches: List[Dict[str, Any]] = field(default_factory=list)
    match_score: float = 0.0
    
    def to_dict(self) -> Dict[str, Any]:
        return {
            "query": self.query,
            "checked_at": self.checked_at,
            "snapshot_id": self.snapshot_id,
            "matched": self.matched,
            "matches": self.matches,
            "match_score": self.match_score,
        }


class SanctionsChecker:
    """Check entities against consolidated sanctions lists."""
    
    def __init__(self, entries: List[SanctionsEntry], snapshot_id: str):
        self.entries = entries
        self.snapshot_id = snapshot_id
        self._build_index()
    
    def _build_index(self) -> None:
        """Build search indices for fast lookup."""
        self._name_index: Dict[str, List[SanctionsEntry]] = {}
        self._id_index: Dict[str, List[SanctionsEntry]] = {}
        
        for entry in self.entries:
            # Index by normalized name
            norm_name = self._normalize(entry.primary_name)
            if norm_name not in self._name_index:
                self._name_index[norm_name] = []
            self._name_index[norm_name].append(entry)
            
            # Index aliases
            for alias in entry.aliases:
                norm_alias = self._normalize(alias.get("alias", ""))
                if norm_alias and norm_alias not in self._name_index:
                    self._name_index[norm_alias] = []
                if norm_alias:
                    self._name_index[norm_alias].append(entry)
            
            # Index identifiers
            for ident in entry.identifiers:
                id_val = ident.get("value", "").upper().strip()
                if id_val:
                    if id_val not in self._id_index:
                        self._id_index[id_val] = []
                    self._id_index[id_val].append(entry)
    
    def _normalize(self, s: str) -> str:
        """Normalize a string for matching."""
        s = s.lower().strip()
        s = re.sub(r"[^\w\s]", "", s)
        s = re.sub(r"\s+", " ", s)
        return s
    
    def _fuzzy_score(self, query: str, target: str) -> float:
        """Compute fuzzy match score (0.0 - 1.0)."""
        query = self._normalize(query)
        target = self._normalize(target)
        
        if query == target:
            return 1.0
        
        if query in target or target in query:
            return 0.9
        
        # Simple token overlap
        q_tokens = set(query.split())
        t_tokens = set(target.split())
        
        if not q_tokens or not t_tokens:
            return 0.0
        
        overlap = len(q_tokens & t_tokens)
        total = len(q_tokens | t_tokens)
        
        return overlap / total if total > 0 else 0.0
    
    def check_entity(
        self,
        name: str,
        identifiers: Optional[List[Dict[str, str]]] = None,
        threshold: float = 0.7,
    ) -> SanctionsCheckResult:
        """Check if an entity matches any sanctions entry."""
        now = datetime.now(timezone.utc).isoformat()
        matches: List[Dict[str, Any]] = []
        max_score = 0.0
        
        # Check name
        norm_name = self._normalize(name)
        
        # Exact match
        if norm_name in self._name_index:
            for entry in self._name_index[norm_name]:
                matches.append({
                    "entry": entry.to_dict(),
                    "match_type": "exact_name",
                    "score": 1.0,
                })
                max_score = 1.0
        
        # Fuzzy match
        if max_score < 1.0:
            for norm_target, entries in self._name_index.items():
                score = self._fuzzy_score(name, norm_target)
                if score >= threshold and score > max_score:
                    for entry in entries:
                        matches.append({
                            "entry": entry.to_dict(),
                            "match_type": "fuzzy_name",
                            "score": score,
                        })
                    max_score = max(max_score, score)
        
        # Check identifiers
        if identifiers:
            for ident in identifiers:
                id_val = ident.get("value", "").upper().strip()
                if id_val in self._id_index:
                    for entry in self._id_index[id_val]:
                        matches.append({
                            "entry": entry.to_dict(),
                            "match_type": "identifier",
                            "identifier_type": ident.get("type"),
                            "score": 1.0,
                        })
                        max_score = 1.0
        
        # Deduplicate matches
        seen_ids = set()
        unique_matches = []
        for m in matches:
            entry_id = m["entry"]["entry_id"]
            if entry_id not in seen_ids:
                seen_ids.add(entry_id)
                unique_matches.append(m)
        
        return SanctionsCheckResult(
            query=name,
            checked_at=now,
            snapshot_id=self.snapshot_id,
            matched=len(unique_matches) > 0,
            matches=unique_matches,
            match_score=max_score,
        )


# ─────────────────────────────────────────────────────────────────────────────
# RegPack Manager
# ─────────────────────────────────────────────────────────────────────────────

class RegPackManager:
    """Manage RegPack creation, verification, and querying."""
    
    def __init__(
        self,
        jurisdiction_id: str,
        domain: str,
        store_root: Optional[Path] = None,
    ):
        self.jurisdiction_id = jurisdiction_id
        self.domain = domain
        self.store_root = store_root or Path("regpacks")
    
    def _deterministic_timestamp(self, offset: int = 0) -> str:
        """Generate deterministic timestamp from SOURCE_DATE_EPOCH."""
        epoch = int(os.environ.get("SOURCE_DATE_EPOCH", "0"))
        if epoch == 0:
            dt = datetime.now(timezone.utc)
        else:
            dt = datetime.fromtimestamp(epoch + offset, tz=timezone.utc)
        return dt.strftime("%Y-%m-%dT%H:%M:%SZ")
    
    def _compute_digest(self, content: bytes) -> str:
        """Compute SHA256 digest."""
        return hashlib.sha256(content).hexdigest()
    
    def _canonical_json(self, obj: Any) -> bytes:
        """Produce canonical JSON bytes (JCS)."""
        return json.dumps(
            obj,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
        ).encode("utf-8")
    
    def create_sanctions_snapshot(
        self,
        as_of_date: str,
        entries: List[SanctionsEntry],
        sources: Dict[str, Dict[str, Any]],
        previous_digest: Optional[str] = None,
    ) -> SanctionsSnapshot:
        """Create a new sanctions snapshot."""
        snapshot_id = f"sanctions-{as_of_date}"
        timestamp = self._deterministic_timestamp()
        
        # Count by type
        counts = {
            "total_records": len(entries),
            "individuals": sum(1 for e in entries if e.entry_type == "individual"),
            "entities": sum(1 for e in entries if e.entry_type == "entity"),
            "vessels": sum(1 for e in entries if e.entry_type == "vessel"),
            "aircraft": sum(1 for e in entries if e.entry_type == "aircraft"),
        }
        
        delta = None
        if previous_digest:
            # In production, would compute actual delta
            delta = {
                "previous_snapshot_digest": previous_digest,
                "additions": 0,
                "removals": 0,
                "modifications": 0,
            }
        
        return SanctionsSnapshot(
            snapshot_id=snapshot_id,
            snapshot_timestamp=timestamp,
            sources=sources,
            entries=entries,
            consolidated_counts=counts,
            delta_from_previous=delta,
        )
    
    def create_regpack_metadata(
        self,
        as_of_date: str,
        snapshot_type: str = "quarterly",
        regulators: List[str] = None,
        sanctions_lists: List[str] = None,
        license_types: int = 0,
        report_types: int = 0,
        guidance_documents: int = 0,
        previous_digest: Optional[str] = None,
    ) -> RegPackMetadata:
        """Create RegPack metadata."""
        regpack_id = f"regpack:{self.jurisdiction_id}:{self.domain}:{as_of_date.replace('-', '')[:6]}"
        
        sources = []
        if sanctions_lists:
            for sl in sanctions_lists:
                if sl in SANCTIONS_SOURCES:
                    sources.append({
                        "type": "sanctions_feed",
                        "provider": sl,
                        "endpoint": SANCTIONS_SOURCES[sl]["url"],
                        "fetched_at": self._deterministic_timestamp(),
                    })
        
        includes = {
            "regulators": regulators or [],
            "sanctions_lists": sanctions_lists or [],
            "license_types": license_types,
            "report_types": report_types,
            "guidance_documents": guidance_documents,
        }
        
        return RegPackMetadata(
            regpack_id=regpack_id,
            jurisdiction_id=self.jurisdiction_id,
            domain=self.domain,
            as_of_date=as_of_date,
            snapshot_type=snapshot_type,
            sources=sources,
            includes=includes,
            previous_regpack_digest=previous_digest,
            created_at=self._deterministic_timestamp(),
        )
    
    def compute_regpack_digest(
        self,
        metadata: RegPackMetadata,
        sanctions: Optional[SanctionsSnapshot] = None,
        regulators: List[RegulatorProfile] = None,
        deadlines: List[ComplianceDeadline] = None,
    ) -> str:
        """Compute the canonical digest for a regpack."""
        components = [b"msez-regpack-v1\0"]
        
        # Add metadata
        components.append(self._canonical_json(metadata.to_dict()))
        
        # Add sanctions snapshot metadata (not full entries)
        if sanctions:
            components.append(self._canonical_json(sanctions.to_dict()))
        
        # Add regulators (sorted by ID)
        if regulators:
            reg_index = {"regulators": sorted([r.regulator_id for r in regulators])}
            components.append(self._canonical_json(reg_index))
        
        # Add deadlines
        if deadlines:
            dl_data = {"deadlines": [d.to_dict() for d in sorted(deadlines, key=lambda x: x.deadline_id)]}
            components.append(self._canonical_json(dl_data))
        
        combined = b"".join(components)
        return self._compute_digest(combined)


# ─────────────────────────────────────────────────────────────────────────────
# Exports
# ─────────────────────────────────────────────────────────────────────────────

__all__ = [
    "STACK_SPEC_VERSION",
    "REGPACK_VERSION",
    "SANCTIONS_SOURCES",
    "SanctionsEntry",
    "SanctionsSnapshot",
    "SanctionsCheckResult",
    "SanctionsChecker",
    "RegulatorProfile",
    "LicenseType",
    "ReportingRequirement",
    "ComplianceDeadline",
    "RegPackMetadata",
    "RegPackManager",
]
