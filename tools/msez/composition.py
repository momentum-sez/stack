"""Multi-Jurisdiction Zone Composition Engine.

Enables deployments like:
    "Deploy the civic code of NY with the corporate law of Delaware,
     but with the digital asset clearance, settlement and securities
     laws of ADGM with automated AI arbitration turned on"

Architecture:
    A ZoneComposition is built from multiple JurisdictionLayers,
    each contributing specific legal/regulatory domains:

    ZoneComposition
    ├── civic_layer:      JurisdictionLayer (e.g., NY civic code)
    ├── corporate_layer:  JurisdictionLayer (e.g., Delaware corporate)
    ├── financial_layer:  JurisdictionLayer (e.g., ADGM digital assets)
    ├── arbitration:      ArbitrationConfig (e.g., AI-assisted DIFC-LCIA)
    └── corridors:        List[CorridorConfig]

Each layer specifies:
    - Jurisdiction ID (hierarchical: country-region-zone)
    - Domains to import (civic, corporate, financial, etc.)
    - Lawpacks to pin (immutable legal text)
    - Regpacks to pin (regulatory guidance snapshots)
    - Licensepacks to pin (active license registries)

The composition engine:
    1. Validates layer compatibility (no domain conflicts)
    2. Resolves transitive dependencies
    3. Generates a unified stack.lock
    4. Produces deployment artifacts
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

from tools.msez.core import (
    REPO_ROOT,
    canonical_json_bytes,
    is_valid_sha256,
    load_json,
    load_yaml,
    sha256_bytes,
)


class Domain(Enum):
    """Legal/regulatory domains that can be composed."""

    CIVIC = "civic"
    CORPORATE = "corporate"
    COMMERCIAL = "commercial"
    FINANCIAL = "financial"
    SECURITIES = "securities"
    BANKING = "banking"
    PAYMENTS = "payments"
    CUSTODY = "custody"
    CLEARING = "clearing"
    SETTLEMENT = "settlement"
    DIGITAL_ASSETS = "digital-assets"
    TAX = "tax"
    EMPLOYMENT = "employment"
    IMMIGRATION = "immigration"
    IP = "intellectual-property"
    DATA_PROTECTION = "data-protection"
    AML_CFT = "aml-cft"
    CONSUMER_PROTECTION = "consumer-protection"
    ARBITRATION = "arbitration"
    LICENSING = "licensing"


@dataclass
class LawpackRef:
    """Reference to a pinned lawpack."""

    jurisdiction_id: str
    domain: str
    digest_sha256: str
    version: str = ""
    effective_date: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = {
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain,
            "lawpack_digest_sha256": self.digest_sha256,
        }
        if self.version:
            d["version"] = self.version
        if self.effective_date:
            d["effective_date"] = self.effective_date
        return d


@dataclass
class RegpackRef:
    """Reference to a pinned regpack."""

    jurisdiction_id: str
    domain: str
    digest_sha256: str
    as_of_date: str = ""

    def to_dict(self) -> Dict[str, Any]:
        d = {
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain,
            "regpack_digest_sha256": self.digest_sha256,
        }
        if self.as_of_date:
            d["as_of_date"] = self.as_of_date
        return d


@dataclass
class LicensepackRef:
    """Reference to a pinned licensepack."""

    jurisdiction_id: str
    domain: str
    digest_sha256: str
    snapshot_timestamp: str = ""
    includes: Optional[Dict[str, int]] = None

    def to_dict(self) -> Dict[str, Any]:
        d = {
            "jurisdiction_id": self.jurisdiction_id,
            "domain": self.domain,
            "licensepack_digest_sha256": self.digest_sha256,
        }
        if self.snapshot_timestamp:
            d["snapshot_timestamp"] = self.snapshot_timestamp
        if self.includes:
            d["includes"] = self.includes
        return d


@dataclass
class JurisdictionLayer:
    """A layer contributing specific domains from a jurisdiction.

    Examples:
        # New York civic code
        JurisdictionLayer(
            jurisdiction_id="us-ny",
            domains=[Domain.CIVIC],
            description="New York State civic code",
        )

        # Delaware corporate law
        JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE],
            description="Delaware General Corporation Law",
        )

        # ADGM digital assets
        JurisdictionLayer(
            jurisdiction_id="ae-abudhabi-adgm",
            domains=[Domain.DIGITAL_ASSETS, Domain.SECURITIES, Domain.CLEARING, Domain.SETTLEMENT],
            description="ADGM Financial Services Regulatory Framework",
        )
    """

    jurisdiction_id: str
    domains: List[Domain]
    description: str = ""
    lawpacks: List[LawpackRef] = field(default_factory=list)
    regpacks: List[RegpackRef] = field(default_factory=list)
    licensepacks: List[LicensepackRef] = field(default_factory=list)
    module_overrides: Dict[str, str] = field(default_factory=dict)

    def validate(self) -> List[str]:
        """Validate layer configuration."""
        errors = []

        # Jurisdiction ID format: country[-region[-zone]]
        if not re.match(r"^[a-z]{2}(-[a-z0-9-]+)*$", self.jurisdiction_id):
            errors.append(f"Invalid jurisdiction_id format: {self.jurisdiction_id}")

        if not self.domains:
            errors.append(f"Layer {self.jurisdiction_id} has no domains")

        # Validate lawpack references
        for lp in self.lawpacks:
            if not is_valid_sha256(lp.digest_sha256):
                errors.append(f"Invalid lawpack digest: {lp.digest_sha256}")

        # Validate regpack references
        for rp in self.regpacks:
            if not is_valid_sha256(rp.digest_sha256):
                errors.append(f"Invalid regpack digest: {rp.digest_sha256}")

        # Validate licensepack references
        for lcp in self.licensepacks:
            if not is_valid_sha256(lcp.digest_sha256):
                errors.append(f"Invalid licensepack digest: {lcp.digest_sha256}")

        return errors

    def domain_set(self) -> Set[Domain]:
        """Return set of domains provided by this layer."""
        return set(self.domains)


class ArbitrationMode(Enum):
    """Arbitration modes for dispute resolution."""

    TRADITIONAL = "traditional"
    AI_ASSISTED = "ai-assisted"
    AI_AUTONOMOUS = "ai-autonomous"
    HYBRID = "hybrid"


@dataclass
class ArbitrationConfig:
    """Configuration for zone arbitration system.

    Supports traditional, AI-assisted, and autonomous arbitration modes.
    """

    mode: ArbitrationMode = ArbitrationMode.TRADITIONAL
    institution_id: str = ""
    rules_version: str = ""
    ai_model: str = ""
    human_review_threshold_usd: int = 0
    appeal_allowed: bool = True
    max_claim_usd: int = 0

    def to_dict(self) -> Dict[str, Any]:
        d = {
            "mode": self.mode.value,
            "appeal_allowed": self.appeal_allowed,
        }
        if self.institution_id:
            d["institution_id"] = self.institution_id
        if self.rules_version:
            d["rules_version"] = self.rules_version
        if self.ai_model:
            d["ai_model"] = self.ai_model
        if self.human_review_threshold_usd > 0:
            d["human_review_threshold_usd"] = self.human_review_threshold_usd
        if self.max_claim_usd > 0:
            d["max_claim_usd"] = self.max_claim_usd
        return d


@dataclass
class CorridorConfig:
    """Configuration for a settlement corridor."""

    corridor_id: str
    source_jurisdiction: str
    target_jurisdiction: str
    settlement_currency: str = "USD"
    settlement_mechanism: str = "rtgs"
    max_settlement_usd: int = 0
    finality_seconds: int = 3600

    def to_dict(self) -> Dict[str, Any]:
        return {
            "corridor_id": self.corridor_id,
            "source_jurisdiction": self.source_jurisdiction,
            "target_jurisdiction": self.target_jurisdiction,
            "settlement_currency": self.settlement_currency,
            "settlement_mechanism": self.settlement_mechanism,
            "max_settlement_usd": self.max_settlement_usd,
            "finality_seconds": self.finality_seconds,
        }


@dataclass
class ZoneComposition:
    """A composed zone from multiple jurisdiction layers.

    This is the central abstraction for multi-jurisdiction deployments.
    """

    zone_id: str
    name: str
    description: str = ""
    layers: List[JurisdictionLayer] = field(default_factory=list)
    arbitration: Optional[ArbitrationConfig] = None
    corridors: List[CorridorConfig] = field(default_factory=list)
    profile: str = "digital-financial-center"

    def validate(self) -> List[str]:
        """Validate the composition for conflicts and completeness."""
        errors = []

        # Validate zone_id format
        if not re.match(r"^[a-z][a-z0-9.-]*$", self.zone_id):
            errors.append(f"Invalid zone_id format: {self.zone_id}")

        # Validate each layer
        for layer in self.layers:
            errors.extend(layer.validate())

        # Check for domain conflicts (same domain from multiple layers)
        domain_sources: Dict[Domain, List[str]] = {}
        for layer in self.layers:
            for domain in layer.domains:
                if domain not in domain_sources:
                    domain_sources[domain] = []
                domain_sources[domain].append(layer.jurisdiction_id)

        for domain, sources in domain_sources.items():
            if len(sources) > 1:
                errors.append(
                    f"Domain conflict: {domain.value} provided by multiple layers: "
                    f"{', '.join(sources)}"
                )

        return errors

    def all_domains(self) -> Set[Domain]:
        """Return all domains covered by this composition."""
        result: Set[Domain] = set()
        for layer in self.layers:
            result.update(layer.domains)
        return result

    def domain_coverage_report(self) -> Dict[str, str]:
        """Return mapping of domain -> source jurisdiction."""
        report: Dict[str, str] = {}
        for layer in self.layers:
            for domain in layer.domains:
                report[domain.value] = layer.jurisdiction_id
        return report

    def to_zone_yaml(self) -> Dict[str, Any]:
        """Generate zone.yaml content from this composition."""
        zone = {
            "zone_id": self.zone_id,
            "name": self.name,
            "description": self.description,
            "spec_version": "0.4.44",
            "profile": self.profile,
            "composition": {
                "layers": [
                    {
                        "jurisdiction_id": layer.jurisdiction_id,
                        "domains": [d.value for d in layer.domains],
                        "description": layer.description,
                    }
                    for layer in self.layers
                ],
                "domain_mapping": self.domain_coverage_report(),
            },
        }

        # Aggregate lawpacks
        lawpacks = []
        for layer in self.layers:
            lawpacks.extend(lp.to_dict() for lp in layer.lawpacks)
        if lawpacks:
            zone["lawpacks"] = lawpacks

        # Aggregate regpacks
        regpacks = []
        for layer in self.layers:
            regpacks.extend(rp.to_dict() for rp in layer.regpacks)
        if regpacks:
            zone["regpacks"] = regpacks

        # Aggregate licensepacks
        licensepacks = []
        for layer in self.layers:
            licensepacks.extend(lcp.to_dict() for lcp in layer.licensepacks)
        if licensepacks:
            zone["licensepacks"] = licensepacks

        # Arbitration config
        if self.arbitration:
            zone["arbitration"] = self.arbitration.to_dict()

        # Corridors
        if self.corridors:
            zone["corridors"] = [c.to_dict() for c in self.corridors]

        return zone

    def to_stack_lock(self) -> Dict[str, Any]:
        """Generate stack.lock content from this composition."""
        from tools.msez.core import now_iso8601

        lock = {
            "spec_version": "0.4.44",
            "zone_id": self.zone_id,
            "generated_at": now_iso8601(),
            "composition_digest": self.composition_digest(),
        }

        # Lawpacks
        lawpacks = []
        for layer in self.layers:
            for lp in layer.lawpacks:
                lawpacks.append(lp.to_dict())
        if lawpacks:
            lock["lawpacks"] = lawpacks

        # Regpacks
        regpacks = []
        for layer in self.layers:
            for rp in layer.regpacks:
                regpacks.append(rp.to_dict())
        if regpacks:
            lock["regpacks"] = regpacks

        # Licensepacks
        licensepacks = []
        for layer in self.layers:
            for lcp in layer.licensepacks:
                licensepacks.append(lcp.to_dict())
        if licensepacks:
            lock["licensepacks"] = licensepacks

        return lock

    def composition_digest(self) -> str:
        """Compute canonical digest of the composition."""
        # Sort layers by jurisdiction_id for determinism
        sorted_layers = sorted(self.layers, key=lambda l: l.jurisdiction_id)

        composition = {
            "zone_id": self.zone_id,
            "layers": [
                {
                    "jurisdiction_id": l.jurisdiction_id,
                    "domains": sorted(d.value for d in l.domains),
                }
                for l in sorted_layers
            ],
        }

        return sha256_bytes(canonical_json_bytes(composition))


def compose_zone(
    zone_id: str,
    name: str,
    *,
    civic: Optional[Tuple[str, str]] = None,
    corporate: Optional[Tuple[str, str]] = None,
    financial: Optional[Tuple[str, str]] = None,
    digital_assets: Optional[Tuple[str, str]] = None,
    arbitration_mode: ArbitrationMode = ArbitrationMode.TRADITIONAL,
    ai_arbitration: bool = False,
    description: str = "",
) -> ZoneComposition:
    """Convenience function to compose a zone from jurisdiction specs.

    Args:
        zone_id: Unique identifier for the zone
        name: Human-readable name
        civic: Tuple of (jurisdiction_id, description) for civic law
        corporate: Tuple of (jurisdiction_id, description) for corporate law
        financial: Tuple of (jurisdiction_id, description) for financial services
        digital_assets: Tuple of (jurisdiction_id, description) for digital assets
        arbitration_mode: Arbitration mode
        ai_arbitration: Enable AI-assisted arbitration
        description: Zone description

    Returns:
        A validated ZoneComposition

    Example:
        zone = compose_zone(
            "momentum.demo.hybrid",
            "Hybrid Jurisdiction Demo Zone",
            civic=("us-ny", "New York civic code"),
            corporate=("us-de", "Delaware corporate law"),
            financial=("ae-abudhabi-adgm", "ADGM financial services"),
            digital_assets=("ae-abudhabi-adgm", "ADGM digital assets"),
            ai_arbitration=True,
        )
    """
    layers: List[JurisdictionLayer] = []

    if civic:
        layers.append(JurisdictionLayer(
            jurisdiction_id=civic[0],
            domains=[Domain.CIVIC],
            description=civic[1],
        ))

    if corporate:
        layers.append(JurisdictionLayer(
            jurisdiction_id=corporate[0],
            domains=[Domain.CORPORATE, Domain.COMMERCIAL],
            description=corporate[1],
        ))

    if financial:
        # Check if same as digital_assets to avoid duplicates
        financial_domains = [
            Domain.FINANCIAL,
            Domain.BANKING,
            Domain.PAYMENTS,
            Domain.SETTLEMENT,
        ]

        # If digital_assets is from same jurisdiction, merge
        if digital_assets and digital_assets[0] == financial[0]:
            financial_domains.extend([
                Domain.DIGITAL_ASSETS,
                Domain.SECURITIES,
                Domain.CLEARING,
                Domain.CUSTODY,
            ])
            digital_assets = None  # Don't add separate layer

        layers.append(JurisdictionLayer(
            jurisdiction_id=financial[0],
            domains=financial_domains,
            description=financial[1],
        ))

    if digital_assets:
        layers.append(JurisdictionLayer(
            jurisdiction_id=digital_assets[0],
            domains=[
                Domain.DIGITAL_ASSETS,
                Domain.SECURITIES,
                Domain.CLEARING,
                Domain.CUSTODY,
            ],
            description=digital_assets[1],
        ))

    # Configure arbitration
    arb_config = None
    if ai_arbitration:
        arb_config = ArbitrationConfig(
            mode=ArbitrationMode.AI_ASSISTED,
            ai_model="claude-opus-4-5-20251101",
            human_review_threshold_usd=100000,
            appeal_allowed=True,
        )
    elif arbitration_mode != ArbitrationMode.TRADITIONAL:
        arb_config = ArbitrationConfig(mode=arbitration_mode)

    composition = ZoneComposition(
        zone_id=zone_id,
        name=name,
        description=description,
        layers=layers,
        arbitration=arb_config,
    )

    # Validate
    errors = composition.validate()
    if errors:
        raise ValueError(f"Invalid composition: {'; '.join(errors)}")

    return composition


def load_composition_from_yaml(path: Path) -> ZoneComposition:
    """Load a zone composition from a YAML file."""
    data = load_yaml(path)

    if not isinstance(data, dict):
        raise ValueError(f"Invalid composition file: {path}")

    layers = []
    for layer_data in data.get("layers", []):
        domains = [Domain(d) for d in layer_data.get("domains", [])]

        lawpacks = [
            LawpackRef(
                jurisdiction_id=lp.get("jurisdiction_id", ""),
                domain=lp.get("domain", ""),
                digest_sha256=lp.get("lawpack_digest_sha256", ""),
                version=lp.get("version", ""),
                effective_date=lp.get("effective_date", ""),
            )
            for lp in layer_data.get("lawpacks", [])
        ]

        regpacks = [
            RegpackRef(
                jurisdiction_id=rp.get("jurisdiction_id", ""),
                domain=rp.get("domain", ""),
                digest_sha256=rp.get("regpack_digest_sha256", ""),
                as_of_date=rp.get("as_of_date", ""),
            )
            for rp in layer_data.get("regpacks", [])
        ]

        licensepacks = [
            LicensepackRef(
                jurisdiction_id=lcp.get("jurisdiction_id", ""),
                domain=lcp.get("domain", ""),
                digest_sha256=lcp.get("licensepack_digest_sha256", ""),
                snapshot_timestamp=lcp.get("snapshot_timestamp", ""),
                includes=lcp.get("includes"),
            )
            for lcp in layer_data.get("licensepacks", [])
        ]

        layers.append(JurisdictionLayer(
            jurisdiction_id=layer_data.get("jurisdiction_id", ""),
            domains=domains,
            description=layer_data.get("description", ""),
            lawpacks=lawpacks,
            regpacks=regpacks,
            licensepacks=licensepacks,
            module_overrides=layer_data.get("module_overrides", {}),
        ))

    # Parse arbitration config
    arb_data = data.get("arbitration")
    arb_config = None
    if arb_data:
        arb_config = ArbitrationConfig(
            mode=ArbitrationMode(arb_data.get("mode", "traditional")),
            institution_id=arb_data.get("institution_id", ""),
            rules_version=arb_data.get("rules_version", ""),
            ai_model=arb_data.get("ai_model", ""),
            human_review_threshold_usd=arb_data.get("human_review_threshold_usd", 0),
            appeal_allowed=arb_data.get("appeal_allowed", True),
            max_claim_usd=arb_data.get("max_claim_usd", 0),
        )

    # Parse corridors
    corridors = [
        CorridorConfig(
            corridor_id=c.get("corridor_id", ""),
            source_jurisdiction=c.get("source_jurisdiction", ""),
            target_jurisdiction=c.get("target_jurisdiction", ""),
            settlement_currency=c.get("settlement_currency", "USD"),
            settlement_mechanism=c.get("settlement_mechanism", "rtgs"),
            max_settlement_usd=c.get("max_settlement_usd", 0),
            finality_seconds=c.get("finality_seconds", 3600),
        )
        for c in data.get("corridors", [])
    ]

    return ZoneComposition(
        zone_id=data.get("zone_id", ""),
        name=data.get("name", ""),
        description=data.get("description", ""),
        layers=layers,
        arbitration=arb_config,
        corridors=corridors,
        profile=data.get("profile", "digital-financial-center"),
    )
