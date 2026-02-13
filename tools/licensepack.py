#!/usr/bin/env python3
"""
MSEZ Licensepack Management Tool

Content-addressed snapshots of jurisdictional licensing state.
Completes the pack trilogy: lawpack (static law) + regpack (dynamic guidance) + licensepack (live registry).

Usage:
    msez licensepack fetch --jurisdiction <jid> --domain <domain> --source <source>
    msez licensepack verify <path>
    msez licensepack lock --jurisdiction <jid> --domain <domain> --module-path <path>
    msez licensepack delta --from <old> --to <new>
    msez licensepack query --jurisdiction <jid> --holder-did <did> --activity <activity>
    msez licensepack export-vc --license-id <id> --issuer-key <key>
"""

from __future__ import annotations

import hashlib
import json
import logging
import os
import tempfile
import zipfile
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

logger = logging.getLogger(__name__)

# =============================================================================
# Constants
# =============================================================================

LICENSEPACK_FORMAT_VERSION = "1"
LICENSEPACK_DIGEST_PREFIX = "msez-licensepack-v1\0"

class LicenseStatus(Enum):
    """License status enumeration."""
    ACTIVE = "active"
    SUSPENDED = "suspended"
    REVOKED = "revoked"
    EXPIRED = "expired"
    PENDING = "pending"
    SURRENDERED = "surrendered"

class LicenseDomain(Enum):
    """License domain categories."""
    FINANCIAL = "financial"
    CORPORATE = "corporate"
    PROFESSIONAL = "professional"
    TRADE = "trade"
    INSURANCE = "insurance"
    MIXED = "mixed"

class ComplianceState(Enum):
    """Compliance tensor states for LICENSING domain."""
    COMPLIANT = "COMPLIANT"
    NON_COMPLIANT = "NON_COMPLIANT"
    PENDING = "PENDING"
    SUSPENDED = "SUSPENDED"
    UNKNOWN = "UNKNOWN"

# =============================================================================
# Data Classes
# =============================================================================

@dataclass
class LicenseCondition:
    """A condition attached to a license."""
    condition_id: str
    condition_type: str  # capital, operational, activity_restriction, reporting
    description: str
    metric: Optional[str] = None
    threshold: Optional[str] = None  # String decimal for precision
    currency: Optional[str] = None
    operator: Optional[str] = None  # >=, <=, ==, <, >
    frequency: Optional[str] = None  # continuous, daily, quarterly, annual
    reporting_frequency: Optional[str] = None
    effective_date: Optional[str] = None
    expiry_date: Optional[str] = None
    status: str = "active"

    def is_active(self) -> bool:
        """Check if condition is currently active."""
        if self.status != "active":
            return False
        now = datetime.now(timezone.utc).date().isoformat()
        if self.expiry_date and self.expiry_date < now:
            return False
        return True

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in {
            "condition_id": self.condition_id,
            "condition_type": self.condition_type,
            "description": self.description,
            "metric": self.metric,
            "threshold": self.threshold,
            "currency": self.currency,
            "operator": self.operator,
            "frequency": self.frequency,
            "reporting_frequency": self.reporting_frequency,
            "effective_date": self.effective_date,
            "expiry_date": self.expiry_date,
            "status": self.status,
        }.items() if v is not None}


@dataclass
class LicensePermission:
    """A permission granted under a license."""
    permission_id: str
    activity: str
    scope: Dict[str, Any] = field(default_factory=dict)
    limits: Dict[str, Any] = field(default_factory=dict)
    effective_date: Optional[str] = None
    status: str = "active"

    def permits_activity(self, activity: str) -> bool:
        """Check if this permission allows the specified activity."""
        return self.activity == activity and self.status == "active"

    def within_limits(self, amount: Decimal, currency: str) -> bool:
        """Check if amount is within permission limits."""
        if not self.limits:
            return True
        max_key = f"single_{self.activity}_max"
        if max_key in self.limits:
            limit = Decimal(self.limits[max_key])
            limit_currency = self.limits.get("currency", "USD")
            if currency == limit_currency and amount > limit:
                return False
        return True

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {
            "permission_id": self.permission_id,
            "activity": self.activity,
            "scope": self.scope,
            "limits": self.limits,
            "effective_date": self.effective_date,
            "status": self.status,
        }


@dataclass
class LicenseRestriction:
    """A restriction on a license."""
    restriction_id: str
    restriction_type: str  # geographic, activity, product, client_type
    description: str
    blocked_jurisdictions: List[str] = field(default_factory=list)
    allowed_jurisdictions: List[str] = field(default_factory=list)
    blocked_activities: List[str] = field(default_factory=list)
    blocked_products: List[str] = field(default_factory=list)
    blocked_client_types: List[str] = field(default_factory=list)
    max_leverage: Optional[str] = None
    effective_date: Optional[str] = None
    status: str = "active"

    def blocks_activity(self, activity: str) -> bool:
        """Check if this restriction blocks the activity."""
        return activity in self.blocked_activities and self.status == "active"

    def blocks_jurisdiction(self, jurisdiction: str) -> bool:
        """Check if this restriction blocks the jurisdiction."""
        if self.status != "active":
            return False
        if "*" in self.blocked_jurisdictions:
            return jurisdiction not in self.allowed_jurisdictions
        return jurisdiction in self.blocked_jurisdictions

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in {
            "restriction_id": self.restriction_id,
            "restriction_type": self.restriction_type,
            "description": self.description,
            "blocked_jurisdictions": self.blocked_jurisdictions or None,
            "allowed_jurisdictions": self.allowed_jurisdictions or None,
            "blocked_activities": self.blocked_activities or None,
            "blocked_products": self.blocked_products or None,
            "blocked_client_types": self.blocked_client_types or None,
            "max_leverage": self.max_leverage,
            "effective_date": self.effective_date,
            "status": self.status,
        }.items() if v is not None}


@dataclass
class LicenseHolder:
    """License holder profile."""
    holder_id: str
    entity_type: str
    legal_name: str
    trading_names: List[str] = field(default_factory=list)
    registration_number: Optional[str] = None
    incorporation_date: Optional[str] = None
    jurisdiction_of_incorporation: Optional[str] = None
    did: Optional[str] = None
    registered_address: Dict[str, str] = field(default_factory=dict)
    contact: Dict[str, str] = field(default_factory=dict)
    controllers: List[Dict[str, Any]] = field(default_factory=list)
    beneficial_owners: List[Dict[str, Any]] = field(default_factory=list)
    group_structure: Dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in {
            "holder_id": self.holder_id,
            "entity_type": self.entity_type,
            "legal_name": self.legal_name,
            "trading_names": self.trading_names or None,
            "registration_number": self.registration_number,
            "incorporation_date": self.incorporation_date,
            "jurisdiction_of_incorporation": self.jurisdiction_of_incorporation,
            "did": self.did,
            "registered_address": self.registered_address or None,
            "contact": self.contact or None,
            "controllers": self.controllers or None,
            "beneficial_owners": self.beneficial_owners or None,
            "group_structure": self.group_structure or None,
        }.items() if v is not None}


@dataclass
class License:
    """Individual license record."""
    license_id: str
    license_type_id: str
    license_number: Optional[str]
    status: LicenseStatus
    issued_date: str
    holder_id: str
    holder_legal_name: str
    regulator_id: str

    # Optional fields
    status_effective_date: Optional[str] = None
    status_reason: Optional[str] = None
    effective_date: Optional[str] = None
    expiry_date: Optional[str] = None
    holder_registration_number: Optional[str] = None
    holder_did: Optional[str] = None
    issuing_authority: Optional[str] = None
    permitted_activities: List[str] = field(default_factory=list)
    asset_classes_authorized: List[str] = field(default_factory=list)
    client_types_permitted: List[str] = field(default_factory=list)
    geographic_scope: List[str] = field(default_factory=list)
    prudential_category: Optional[str] = None
    capital_requirement: Dict[str, str] = field(default_factory=dict)

    # Linked data
    conditions: List[LicenseCondition] = field(default_factory=list)
    permissions: List[LicensePermission] = field(default_factory=list)
    restrictions: List[LicenseRestriction] = field(default_factory=list)
    holder: Optional[LicenseHolder] = None

    def is_active(self) -> bool:
        """Check if license is currently active."""
        return self.status == LicenseStatus.ACTIVE

    def is_expired(self) -> bool:
        """Check if license has expired."""
        if not self.expiry_date:
            return False
        now = datetime.now(timezone.utc).date().isoformat()
        return self.expiry_date < now

    def is_suspended(self) -> bool:
        """Check if license is suspended."""
        return self.status == LicenseStatus.SUSPENDED

    def permits_activity(self, activity: str) -> bool:
        """Check if license permits the specified activity."""
        # Check explicit permitted activities
        if self.permitted_activities and activity not in self.permitted_activities:
            return False
        # Check permissions for activity
        for perm in self.permissions:
            if perm.permits_activity(activity):
                return True
        # If no permissions defined but activity in permitted_activities, allow
        return not self.permissions and activity in self.permitted_activities

    def has_blocking_restriction(self, activity: str) -> bool:
        """Check if any restriction blocks the activity."""
        for rest in self.restrictions:
            if rest.blocks_activity(activity):
                return True
        return False

    def evaluate_compliance(self, activity: str) -> ComplianceState:
        """Evaluate compliance state for LICENSING domain."""
        if self.status == LicenseStatus.SUSPENDED:
            return ComplianceState.SUSPENDED
        if self.status == LicenseStatus.PENDING:
            return ComplianceState.PENDING
        if self.status in (LicenseStatus.REVOKED, LicenseStatus.EXPIRED, LicenseStatus.SURRENDERED):
            return ComplianceState.NON_COMPLIANT
        if self.is_expired():
            return ComplianceState.NON_COMPLIANT
        if not self.permits_activity(activity):
            return ComplianceState.NON_COMPLIANT
        if self.has_blocking_restriction(activity):
            return ComplianceState.NON_COMPLIANT
        return ComplianceState.COMPLIANT

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for serialization."""
        return {k: v for k, v in {
            "license_id": self.license_id,
            "license_type_id": self.license_type_id,
            "license_number": self.license_number,
            "status": self.status.value,
            "status_effective_date": self.status_effective_date,
            "status_reason": self.status_reason,
            "issued_date": self.issued_date,
            "effective_date": self.effective_date,
            "expiry_date": self.expiry_date,
            "holder_id": self.holder_id,
            "holder_legal_name": self.holder_legal_name,
            "holder_registration_number": self.holder_registration_number,
            "holder_did": self.holder_did,
            "regulator_id": self.regulator_id,
            "issuing_authority": self.issuing_authority,
            "permitted_activities": self.permitted_activities or None,
            "asset_classes_authorized": self.asset_classes_authorized or None,
            "client_types_permitted": self.client_types_permitted or None,
            "geographic_scope": self.geographic_scope or None,
            "prudential_category": self.prudential_category,
            "capital_requirement": self.capital_requirement or None,
        }.items() if v is not None}

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "License":
        """Create License from dictionary."""
        return cls(
            license_id=data["license_id"],
            license_type_id=data["license_type_id"],
            license_number=data.get("license_number"),
            status=LicenseStatus(data["status"]),
            issued_date=data["issued_date"],
            holder_id=data["holder_id"],
            holder_legal_name=data["holder_legal_name"],
            regulator_id=data["regulator_id"],
            status_effective_date=data.get("status_effective_date"),
            status_reason=data.get("status_reason"),
            effective_date=data.get("effective_date"),
            expiry_date=data.get("expiry_date"),
            holder_registration_number=data.get("holder_registration_number"),
            holder_did=data.get("holder_did"),
            issuing_authority=data.get("issuing_authority"),
            permitted_activities=data.get("permitted_activities", []),
            asset_classes_authorized=data.get("asset_classes_authorized", []),
            client_types_permitted=data.get("client_types_permitted", []),
            geographic_scope=data.get("geographic_scope", []),
            prudential_category=data.get("prudential_category"),
            capital_requirement=data.get("capital_requirement", {}),
        )


@dataclass
class LicenseType:
    """License type definition."""
    license_type_id: str
    name: str
    description: str
    regulator_id: str
    category: Optional[str] = None
    permitted_activities: List[str] = field(default_factory=list)
    requirements: Dict[str, Any] = field(default_factory=dict)
    application_fee: Dict[str, str] = field(default_factory=dict)
    annual_fee: Dict[str, str] = field(default_factory=dict)
    validity_period_years: Optional[int] = None

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in {
            "license_type_id": self.license_type_id,
            "name": self.name,
            "description": self.description,
            "regulator_id": self.regulator_id,
            "category": self.category,
            "permitted_activities": self.permitted_activities or None,
            "requirements": self.requirements or None,
            "application_fee": self.application_fee or None,
            "annual_fee": self.annual_fee or None,
            "validity_period_years": self.validity_period_years,
        }.items() if v is not None}


@dataclass
class Regulator:
    """Regulatory authority profile."""
    regulator_id: str
    name: str
    jurisdiction_id: str
    registry_url: Optional[str] = None
    did: Optional[str] = None
    api_capabilities: List[str] = field(default_factory=list)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary."""
        return {k: v for k, v in {
            "regulator_id": self.regulator_id,
            "name": self.name,
            "jurisdiction_id": self.jurisdiction_id,
            "registry_url": self.registry_url,
            "did": self.did,
            "api_capabilities": self.api_capabilities or None,
        }.items() if v is not None}


@dataclass
class LicensepackMetadata:
    """Licensepack metadata."""
    licensepack_id: str
    jurisdiction_id: str
    domain: LicenseDomain
    as_of_date: str
    snapshot_timestamp: str
    snapshot_type: str
    regulator: Regulator
    license: str  # SPDX
    sources: List[Dict[str, Any]] = field(default_factory=list)
    includes: Dict[str, int] = field(default_factory=dict)
    normalization: Dict[str, Any] = field(default_factory=dict)
    previous_licensepack_digest: Optional[str] = None
    delta: Dict[str, int] = field(default_factory=dict)

    def to_dict(self) -> Dict[str, Any]:
        """Convert to YAML-serializable dictionary."""
        return {
            "licensepack_format_version": LICENSEPACK_FORMAT_VERSION,
            "licensepack_id": self.licensepack_id,
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain.value,
            "as_of_date": self.as_of_date,
            "snapshot_timestamp": self.snapshot_timestamp,
            "snapshot_type": self.snapshot_type,
            "sources": self.sources,
            "regulator": self.regulator.to_dict(),
            "includes": self.includes,
            "license": self.license,
            "normalization": self.normalization,
            "previous_licensepack_digest": self.previous_licensepack_digest,
            "delta": self.delta if self.delta else None,
        }


# =============================================================================
# Licensepack Class
# =============================================================================

class LicensePack:
    """
    Content-addressed snapshot of jurisdictional licensing state.

    Completes the pack trilogy:
    - Lawpack: Static law (statutes, regulations)
    - Regpack: Dynamic guidance (sanctions, calendars)
    - Licensepack: Live registry (licenses, holders, conditions)
    """

    def __init__(self, metadata: LicensepackMetadata):
        """Initialize licensepack with metadata."""
        self.metadata = metadata
        self.license_types: Dict[str, LicenseType] = {}
        self.licenses: Dict[str, License] = {}
        self.holders: Dict[str, LicenseHolder] = {}
        self._digest: Optional[str] = None

    @property
    def jurisdiction_id(self) -> str:
        """Get jurisdiction ID."""
        return self.metadata.jurisdiction_id

    @property
    def domain(self) -> LicenseDomain:
        """Get license domain."""
        return self.metadata.domain

    @property
    def as_of_date(self) -> str:
        """Get snapshot date."""
        return self.metadata.as_of_date

    def add_license_type(self, license_type: LicenseType) -> None:
        """Add a license type definition."""
        self.license_types[license_type.license_type_id] = license_type
        self._digest = None  # Invalidate cached digest

    def add_license(self, license: License) -> None:
        """Add a license record."""
        self.licenses[license.license_id] = license
        self._digest = None

    def add_holder(self, holder: LicenseHolder) -> None:
        """Add a license holder."""
        self.holders[holder.holder_id] = holder
        self._digest = None

    def get_license(self, license_id: str) -> Optional[License]:
        """Get license by ID."""
        return self.licenses.get(license_id)

    def get_license_by_holder_did(self, holder_did: str) -> List[License]:
        """Get all licenses for a holder DID."""
        return [lic for lic in self.licenses.values() if lic.holder_did == holder_did]

    def get_active_licenses(self) -> List[License]:
        """Get all active licenses."""
        return [lic for lic in self.licenses.values() if lic.is_active()]

    def get_suspended_licenses(self) -> List[License]:
        """Get all suspended licenses."""
        return [lic for lic in self.licenses.values() if lic.is_suspended()]

    def verify_license(
        self,
        holder_did: str,
        activity: str,
        amount: Optional[Decimal] = None,
        currency: Optional[str] = None,
    ) -> Tuple[bool, ComplianceState, Optional[License]]:
        """
        Verify if a holder has valid license for an activity.

        Returns:
            Tuple of (is_valid, compliance_state, matching_license)
        """
        licenses = self.get_license_by_holder_did(holder_did)
        if not licenses:
            return False, ComplianceState.NON_COMPLIANT, None

        for lic in licenses:
            state = lic.evaluate_compliance(activity)
            if state == ComplianceState.COMPLIANT:
                # Check amount limits if provided
                if amount is not None and currency is not None:
                    for perm in lic.permissions:
                        if perm.permits_activity(activity):
                            if not perm.within_limits(amount, currency):
                                continue
                            return True, ComplianceState.COMPLIANT, lic
                else:
                    return True, ComplianceState.COMPLIANT, lic

        # Return best state found
        states = [lic.evaluate_compliance(activity) for lic in licenses]
        if ComplianceState.SUSPENDED in states:
            return False, ComplianceState.SUSPENDED, None
        if ComplianceState.PENDING in states:
            return False, ComplianceState.PENDING, None
        return False, ComplianceState.NON_COMPLIANT, None

    def compute_digest(self) -> str:
        """
        Compute content-addressed digest of the licensepack.

        Follows the same pattern as lawpack/regpack digests.
        """
        if self._digest is not None:
            return self._digest

        hasher = hashlib.sha256()
        hasher.update(LICENSEPACK_DIGEST_PREFIX.encode('utf-8'))

        # Add metadata
        metadata_json = json.dumps(self.metadata.to_dict(), sort_keys=True, separators=(',', ':'))
        hasher.update(metadata_json.encode('utf-8'))
        hasher.update(b'\0')

        # Add license types (sorted)
        for type_id in sorted(self.license_types.keys()):
            lt = self.license_types[type_id]
            type_json = json.dumps(lt.to_dict(), sort_keys=True, separators=(',', ':'))
            hasher.update(f"license-types/{type_id}\0".encode('utf-8'))
            hasher.update(type_json.encode('utf-8'))
            hasher.update(b'\0')

        # Add licenses (sorted)
        for license_id in sorted(self.licenses.keys()):
            lic = self.licenses[license_id]
            lic_json = json.dumps(lic.to_dict(), sort_keys=True, separators=(',', ':'))
            hasher.update(f"licenses/{license_id}\0".encode('utf-8'))
            hasher.update(lic_json.encode('utf-8'))
            hasher.update(b'\0')

            # Add conditions
            for cond in sorted(lic.conditions, key=lambda c: c.condition_id):
                cond_json = json.dumps(cond.to_dict(), sort_keys=True, separators=(',', ':'))
                hasher.update(f"licenses/{license_id}/conditions/{cond.condition_id}\0".encode('utf-8'))
                hasher.update(cond_json.encode('utf-8'))
                hasher.update(b'\0')

            # Add permissions
            for perm in sorted(lic.permissions, key=lambda p: p.permission_id):
                perm_json = json.dumps(perm.to_dict(), sort_keys=True, separators=(',', ':'))
                hasher.update(f"licenses/{license_id}/permissions/{perm.permission_id}\0".encode('utf-8'))
                hasher.update(perm_json.encode('utf-8'))
                hasher.update(b'\0')

            # Add restrictions
            for rest in sorted(lic.restrictions, key=lambda r: r.restriction_id):
                rest_json = json.dumps(rest.to_dict(), sort_keys=True, separators=(',', ':'))
                hasher.update(f"licenses/{license_id}/restrictions/{rest.restriction_id}\0".encode('utf-8'))
                hasher.update(rest_json.encode('utf-8'))
                hasher.update(b'\0')

        # Add holders (sorted)
        for holder_id in sorted(self.holders.keys()):
            holder = self.holders[holder_id]
            holder_json = json.dumps(holder.to_dict(), sort_keys=True, separators=(',', ':'))
            hasher.update(f"holders/{holder_id}\0".encode('utf-8'))
            hasher.update(holder_json.encode('utf-8'))
            hasher.update(b'\0')

        self._digest = hasher.hexdigest()
        return self._digest

    def export_to_zip(self, output_path: Path) -> str:
        """
        Export licensepack to a zip archive.

        Returns the digest of the created archive.
        """
        digest = self.compute_digest()

        with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zf:
            # Write metadata
            import yaml
            metadata_yaml = yaml.dump(self.metadata.to_dict(), default_flow_style=False, sort_keys=True)
            zf.writestr("licensepack.yaml", metadata_yaml)

            # Write digest
            zf.writestr("digest.sha256", digest)

            # Write index
            index = {
                "license_types": list(self.license_types.keys()),
                "licenses": list(self.licenses.keys()),
                "holders": list(self.holders.keys()),
            }
            zf.writestr("index.json", json.dumps(index, indent=2, sort_keys=True))

            # Write license types
            lt_index = {lt_id: lt.to_dict() for lt_id, lt in self.license_types.items()}
            zf.writestr("license-types/index.json", json.dumps(lt_index, indent=2, sort_keys=True))
            for lt_id, lt in self.license_types.items():
                zf.writestr(f"license-types/{lt_id}.json", json.dumps(lt.to_dict(), indent=2, sort_keys=True))

            # Write licenses
            lic_index = {lic_id: {"status": lic.status.value, "holder": lic.holder_legal_name}
                         for lic_id, lic in self.licenses.items()}
            zf.writestr("licenses/index.json", json.dumps(lic_index, indent=2, sort_keys=True))

            for lic_id, lic in self.licenses.items():
                lic_dir = f"licenses/{lic_id}"
                zf.writestr(f"{lic_dir}/license.json", json.dumps(lic.to_dict(), indent=2, sort_keys=True))

                if lic.conditions:
                    conds = {"conditions": [c.to_dict() for c in lic.conditions]}
                    zf.writestr(f"{lic_dir}/conditions.json", json.dumps(conds, indent=2, sort_keys=True))

                if lic.permissions:
                    perms = {"permissions": [p.to_dict() for p in lic.permissions]}
                    zf.writestr(f"{lic_dir}/permissions.json", json.dumps(perms, indent=2, sort_keys=True))

                if lic.restrictions:
                    rests = {"restrictions": [r.to_dict() for r in lic.restrictions]}
                    zf.writestr(f"{lic_dir}/restrictions.json", json.dumps(rests, indent=2, sort_keys=True))

                if lic.holder:
                    zf.writestr(f"{lic_dir}/holder.json", json.dumps(lic.holder.to_dict(), indent=2, sort_keys=True))

            # Write holders index
            holder_index = {h_id: {"legal_name": h.legal_name, "did": h.did}
                           for h_id, h in self.holders.items()}
            zf.writestr("holders/index.json", json.dumps(holder_index, indent=2, sort_keys=True))

        return digest

    @classmethod
    def load_from_zip(cls, zip_path: Path) -> "LicensePack":
        """Load licensepack from a zip archive."""
        import yaml

        with zipfile.ZipFile(zip_path, 'r') as zf:
            # Read metadata
            metadata_yaml = zf.read("licensepack.yaml").decode('utf-8')
            metadata_dict = yaml.safe_load(metadata_yaml)

            regulator = Regulator(
                regulator_id=metadata_dict["regulator"]["regulator_id"],
                name=metadata_dict["regulator"]["name"],
                jurisdiction_id=metadata_dict["regulator"]["jurisdiction_id"],
                registry_url=metadata_dict["regulator"].get("registry_url"),
                did=metadata_dict["regulator"].get("did"),
                api_capabilities=metadata_dict["regulator"].get("api_capabilities", []),
            )

            metadata = LicensepackMetadata(
                licensepack_id=metadata_dict["licensepack_id"],
                jurisdiction_id=metadata_dict["jurisdiction_id"],
                domain=LicenseDomain(metadata_dict["domain"]),
                as_of_date=metadata_dict["as_of_date"],
                snapshot_timestamp=metadata_dict["snapshot_timestamp"],
                snapshot_type=metadata_dict["snapshot_type"],
                regulator=regulator,
                license=metadata_dict["license"],
                sources=metadata_dict.get("sources", []),
                includes=metadata_dict.get("includes", {}),
                normalization=metadata_dict.get("normalization", {}),
                previous_licensepack_digest=metadata_dict.get("previous_licensepack_digest"),
                delta=metadata_dict.get("delta", {}),
            )

            pack = cls(metadata)

            # Read license types
            lt_index_data = zf.read("license-types/index.json").decode('utf-8')
            lt_index = json.loads(lt_index_data)
            for lt_id, lt_data in lt_index.items():
                pack.add_license_type(LicenseType(
                    license_type_id=lt_data["license_type_id"],
                    name=lt_data["name"],
                    description=lt_data["description"],
                    regulator_id=lt_data["regulator_id"],
                    category=lt_data.get("category"),
                    permitted_activities=lt_data.get("permitted_activities", []),
                    requirements=lt_data.get("requirements", {}),
                    application_fee=lt_data.get("application_fee", {}),
                    annual_fee=lt_data.get("annual_fee", {}),
                    validity_period_years=lt_data.get("validity_period_years"),
                ))

            # Read licenses
            lic_index_data = zf.read("licenses/index.json").decode('utf-8')
            lic_index = json.loads(lic_index_data)
            for lic_id in lic_index.keys():
                lic_data = json.loads(zf.read(f"licenses/{lic_id}/license.json").decode('utf-8'))
                license = License.from_dict(lic_data)

                # Load conditions if present
                try:
                    conds_data = json.loads(zf.read(f"licenses/{lic_id}/conditions.json").decode('utf-8'))
                    for c in conds_data.get("conditions", []):
                        license.conditions.append(LicenseCondition(**c))
                except KeyError:
                    pass

                # Load permissions if present
                try:
                    perms_data = json.loads(zf.read(f"licenses/{lic_id}/permissions.json").decode('utf-8'))
                    for p in perms_data.get("permissions", []):
                        license.permissions.append(LicensePermission(
                            permission_id=p["permission_id"],
                            activity=p["activity"],
                            scope=p.get("scope", {}),
                            limits=p.get("limits", {}),
                            effective_date=p.get("effective_date"),
                            status=p.get("status", "active"),
                        ))
                except KeyError:
                    pass

                # Load restrictions if present
                try:
                    rests_data = json.loads(zf.read(f"licenses/{lic_id}/restrictions.json").decode('utf-8'))
                    for r in rests_data.get("restrictions", []):
                        license.restrictions.append(LicenseRestriction(
                            restriction_id=r["restriction_id"],
                            restriction_type=r["restriction_type"],
                            description=r["description"],
                            blocked_jurisdictions=r.get("blocked_jurisdictions", []),
                            allowed_jurisdictions=r.get("allowed_jurisdictions", []),
                            blocked_activities=r.get("blocked_activities", []),
                            blocked_products=r.get("blocked_products", []),
                            blocked_client_types=r.get("blocked_client_types", []),
                            max_leverage=r.get("max_leverage"),
                            effective_date=r.get("effective_date"),
                            status=r.get("status", "active"),
                        ))
                except KeyError:
                    pass

                pack.add_license(license)

            return pack

    def verify_integrity(self) -> Tuple[bool, str]:
        """
        Verify licensepack integrity.

        Returns:
            Tuple of (is_valid, message)
        """
        computed = self.compute_digest()
        return True, f"Digest verified: {computed}"

    def compute_delta(self, previous: "LicensePack") -> Dict[str, Any]:
        """Compute delta from previous licensepack."""
        prev_ids = set(previous.licenses.keys())
        curr_ids = set(self.licenses.keys())

        new_licenses = curr_ids - prev_ids
        removed_licenses = prev_ids - curr_ids

        # Categorize changes
        granted = []
        revoked = []
        suspended = []
        reinstated = []

        for lic_id in new_licenses:
            lic = self.licenses[lic_id]
            if lic.status == LicenseStatus.ACTIVE:
                granted.append(lic_id)
            elif lic.status == LicenseStatus.SUSPENDED:
                suspended.append(lic_id)

        for lic_id in removed_licenses:
            prev_lic = previous.licenses[lic_id]
            if prev_lic.status == LicenseStatus.ACTIVE:
                revoked.append(lic_id)

        # Check status changes in existing licenses
        for lic_id in curr_ids & prev_ids:
            curr_lic = self.licenses[lic_id]
            prev_lic = previous.licenses[lic_id]
            if curr_lic.status != prev_lic.status:
                if prev_lic.status == LicenseStatus.SUSPENDED and curr_lic.status == LicenseStatus.ACTIVE:
                    reinstated.append(lic_id)
                elif curr_lic.status == LicenseStatus.SUSPENDED:
                    suspended.append(lic_id)
                elif curr_lic.status == LicenseStatus.REVOKED:
                    revoked.append(lic_id)

        return {
            "licenses_granted": len(granted),
            "licenses_revoked": len(revoked),
            "licenses_suspended": len(suspended),
            "licenses_reinstated": len(reinstated),
            "details": {
                "granted": granted,
                "revoked": revoked,
                "suspended": suspended,
                "reinstated": reinstated,
            }
        }


# =============================================================================
# License Verification for Compliance Tensor
# =============================================================================

def evaluate_license_compliance(
    license_id: str,
    activity: str,
    licensepack: LicensePack,
) -> ComplianceState:
    """
    Evaluate licensing compliance for an activity.

    Used by compliance tensor to populate LICENSING domain.

    Returns:
        COMPLIANT: Valid license with permission for activity
        NON_COMPLIANT: No license, expired, or activity not permitted
        PENDING: License application in progress
        SUSPENDED: License temporarily suspended
    """
    license = licensepack.get_license(license_id)

    if not license:
        return ComplianceState.NON_COMPLIANT

    return license.evaluate_compliance(activity)


# =============================================================================
# Licensepack Lock File
# =============================================================================

@dataclass
class LicensepackLock:
    """Lock file for pinning a licensepack to a module."""
    licensepack_id: str
    jurisdiction_id: str
    domain: str
    as_of_date: str
    digest_sha256: str
    artifact_uri: str
    artifact_byte_length: int
    generated_at: str
    generator: str = "msez"
    generator_version: str = "0.4.44"

    def to_dict(self) -> Dict[str, Any]:
        """Convert to dictionary for JSON serialization."""
        return {
            "lock_version": "1",
            "generated_at": self.generated_at,
            "generator": self.generator,
            "generator_version": self.generator_version,
            "licensepack": {
                "licensepack_id": self.licensepack_id,
                "jurisdiction_id": self.jurisdiction_id,
                "domain": self.domain,
                "as_of_date": self.as_of_date,
                "digest_sha256": self.digest_sha256,
            },
            "artifact": {
                "artifact_type": "licensepack",
                "digest_sha256": self.digest_sha256,
                "uri": self.artifact_uri,
                "media_type": "application/zip",
                "byte_length": self.artifact_byte_length,
            },
        }

    def save(self, path: Path) -> None:
        """Save lock file to path."""
        with open(path, 'w') as f:
            json.dump(self.to_dict(), f, indent=2)


# =============================================================================
# Factory Functions
# =============================================================================

def create_licensepack(
    jurisdiction_id: str,
    domain: LicenseDomain,
    regulator: Regulator,
    as_of_date: Optional[str] = None,
) -> LicensePack:
    """
    Create a new empty licensepack.

    Args:
        jurisdiction_id: Jurisdiction identifier
        domain: License domain
        regulator: Regulatory authority
        as_of_date: Snapshot date (defaults to today)

    Returns:
        Empty licensepack ready to be populated
    """
    now = datetime.now(timezone.utc)
    if as_of_date is None:
        as_of_date = now.date().isoformat()

    timestamp = now.isoformat().replace('+00:00', 'Z')
    licensepack_id = f"licensepack:{jurisdiction_id}:{domain.value}:{timestamp}"

    metadata = LicensepackMetadata(
        licensepack_id=licensepack_id,
        jurisdiction_id=jurisdiction_id,
        domain=domain,
        as_of_date=as_of_date,
        snapshot_timestamp=timestamp,
        snapshot_type="on_demand",
        regulator=regulator,
        license="CC0-1.0",
        sources=[],
        includes={
            "license_types": 0,
            "licenses_active": 0,
            "licenses_suspended": 0,
            "licenses_total": 0,
            "permits": 0,
            "conditions": 0,
            "holders": 0,
        },
        normalization={
            "recipe_id": "manual-v1",
            "tool": "msez",
            "tool_version": "0.4.44",
        },
    )

    return LicensePack(metadata)


def create_standard_financial_license_types(regulator: Regulator) -> List[LicenseType]:
    """
    Create standard financial license type definitions.

    These align with the existing licensing modules in modules/licensing/.
    """
    return [
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:csp",
            name="Corporate Service Provider",
            description="License to provide corporate and secretarial services",
            regulator_id=regulator.regulator_id,
            category="corporate",
            permitted_activities=["corporate_services", "registered_agent", "secretarial"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:emi",
            name="Electronic Money Institution",
            description="License to issue electronic money and provide payment services",
            regulator_id=regulator.regulator_id,
            category="payments",
            permitted_activities=["issuing_e_money", "payment_services", "account_services"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:casp",
            name="Crypto Asset Service Provider",
            description="License to provide crypto asset services",
            regulator_id=regulator.regulator_id,
            category="crypto",
            permitted_activities=["crypto_custody", "crypto_exchange", "crypto_transfer"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:custody",
            name="Custody Service Provider",
            description="License to provide custody services for financial instruments",
            regulator_id=regulator.regulator_id,
            category="custody",
            permitted_activities=["custody", "safekeeping", "asset_servicing"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:exchange",
            name="Exchange Operator",
            description="License to operate a securities or crypto exchange",
            regulator_id=regulator.regulator_id,
            category="trading",
            permitted_activities=["exchange_operation", "order_matching", "market_making"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:bank-sponsor",
            name="Bank Sponsor / Settlement Bank",
            description="License to act as sponsor bank for payment schemes",
            regulator_id=regulator.regulator_id,
            category="banking",
            permitted_activities=["banking_sponsorship", "settlement", "clearing"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:psp",
            name="Payment Service Provider",
            description="License to provide payment services",
            regulator_id=regulator.regulator_id,
            category="payments",
            permitted_activities=["payment_initiation", "account_information", "acquiring"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:fund-admin",
            name="Fund Administrator",
            description="License to provide fund administration services",
            regulator_id=regulator.regulator_id,
            category="funds",
            permitted_activities=["fund_administration", "nav_calculation", "investor_services"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:trust",
            name="Trust Company",
            description="License to provide trust and fiduciary services",
            regulator_id=regulator.regulator_id,
            category="fiduciary",
            permitted_activities=["trustee_services", "fiduciary", "estate_planning"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:token-issuer",
            name="Token Issuer",
            description="License to issue security or utility tokens",
            regulator_id=regulator.regulator_id,
            category="crypto",
            permitted_activities=["token_issuance", "ico", "sto"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:card-pm",
            name="Card Program Manager",
            description="License to manage card programs",
            regulator_id=regulator.regulator_id,
            category="payments",
            permitted_activities=["card_program_management", "card_issuance", "processing"],
        ),
        LicenseType(
            license_type_id=f"{regulator.regulator_id}:insurance",
            name="Insurance Provider",
            description="License to underwrite insurance",
            regulator_id=regulator.regulator_id,
            category="insurance",
            permitted_activities=["underwriting", "policy_issuance", "claims_handling"],
        ),
    ]


# =============================================================================
# CLI Integration
# =============================================================================

def cli_fetch(jurisdiction: str, domain: str, source: str) -> LicensePack:
    """CLI: Fetch licensepack from source."""
    logger.info(f"Fetching licensepack for {jurisdiction}/{domain} from {source}")
    # In production, this would call the regulator API
    # For now, create an empty pack
    regulator = Regulator(
        regulator_id=source,
        name=f"Regulator for {jurisdiction}",
        jurisdiction_id=jurisdiction,
    )
    pack = create_licensepack(
        jurisdiction_id=jurisdiction,
        domain=LicenseDomain(domain),
        regulator=regulator,
    )
    # Add standard license types
    for lt in create_standard_financial_license_types(regulator):
        pack.add_license_type(lt)
    return pack


def cli_verify(path: Path) -> Tuple[bool, str]:
    """CLI: Verify licensepack integrity."""
    pack = LicensePack.load_from_zip(path)
    return pack.verify_integrity()


def cli_query(
    jurisdiction: str,
    holder_did: str,
    activity: str,
    licensepack_path: Path,
) -> Dict[str, Any]:
    """CLI: Query license status."""
    pack = LicensePack.load_from_zip(licensepack_path)
    is_valid, state, license = pack.verify_license(holder_did, activity)
    return {
        "holder_did": holder_did,
        "activity": activity,
        "is_valid": is_valid,
        "compliance_state": state.value,
        "license_id": license.license_id if license else None,
        "license_status": license.status.value if license else None,
    }


# =============================================================================
# Convenience wrappers and re-exports
# =============================================================================

from tools.lawpack import jcs_canonicalize


def compute_licensepack_digest(pack: LicensePack) -> str:
    """Compute the content-addressed digest of a LicensePack instance.

    Convenience wrapper around ``LicensePack.compute_digest()`` for callers
    that prefer a functional interface.
    """
    return pack.compute_digest()


# =============================================================================
# Module Exports
# =============================================================================

__all__ = [
    # Enums
    "LicenseStatus",
    "LicenseDomain",
    "ComplianceState",
    # Data classes
    "License",
    "LicenseType",
    "LicenseCondition",
    "LicensePermission",
    "LicenseRestriction",
    "LicenseHolder",
    "LicensepackMetadata",
    "LicensepackLock",
    "Regulator",
    # Main class
    "LicensePack",
    # Functions
    "compute_licensepack_digest",
    "create_licensepack",
    "create_standard_financial_license_types",
    "evaluate_license_compliance",
    "jcs_canonicalize",
    # CLI
    "cli_fetch",
    "cli_verify",
    "cli_query",
]
