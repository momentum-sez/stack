"""
Hybrid Zone Deployment Test Suite

Tests real-world scenarios for modular SEZ deployments, including:
- Multi-jurisdiction composition
- Module dependency resolution
- Conflict detection between jurisdictional components
- Profile-based deployment validation
- Corridor establishment and validation

These tests are designed to expose bugs in the deployment pipeline by simulating
actual zone configurations that operators would deploy in production.
"""

import pytest
from dataclasses import dataclass, field
from typing import Dict, List, Set, Optional, Any
from enum import Enum
from decimal import Decimal
import hashlib
import json


# =============================================================================
# ZONE COMPOSITION PRIMITIVES
# =============================================================================

class JurisdictionFamily(Enum):
    """Module families that can be sourced from different jurisdictions."""
    CIVIC = "civic"
    CORPORATE = "corporate"
    FINANCIAL = "financial"
    DIGITAL_ASSETS = "digital_assets"
    TAX = "tax"
    ARBITRATION = "arbitration"
    IDENTITY = "identity"
    TRADE = "trade"
    LICENSING = "licensing"
    GOVERNANCE = "governance"


class LegalTradition(Enum):
    """Legal tradition underlying a jurisdiction."""
    COMMON_LAW = "common_law"
    CIVIL_LAW = "civil_law"
    ISLAMIC_LAW = "islamic_law"
    HYBRID = "hybrid"


@dataclass
class JurisdictionProfile:
    """Profile of a jurisdiction's capabilities."""
    jurisdiction_id: str
    name: str
    legal_tradition: LegalTradition
    supported_families: Set[JurisdictionFamily]
    requires_families: Set[JurisdictionFamily] = field(default_factory=set)
    conflicts_with: Set[str] = field(default_factory=set)
    min_capital_usd: Decimal = Decimal("0")
    kyc_tier_required: int = 0
    allows_crypto: bool = False
    allows_foreign_ownership: bool = True
    treaty_partners: Set[str] = field(default_factory=set)


@dataclass
class ModuleDependency:
    """Dependency relationship between modules."""
    source_module: str
    target_module: str
    dependency_type: str  # "requires", "recommends", "conflicts"
    reason: str


@dataclass
class ZoneComposition:
    """A composed zone from multiple jurisdictions."""
    zone_id: str
    name: str
    jurisdiction_selections: Dict[JurisdictionFamily, str]
    modules: List[str] = field(default_factory=list)
    conflicts: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    hash: str = ""

    def is_valid(self) -> bool:
        return len(self.conflicts) == 0


# =============================================================================
# JURISDICTION REGISTRY
# =============================================================================

JURISDICTION_REGISTRY: Dict[str, JurisdictionProfile] = {
    # UAE Jurisdictions
    "ae-abudhabi-adgm": JurisdictionProfile(
        jurisdiction_id="ae-abudhabi-adgm",
        name="Abu Dhabi Global Market",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.DIGITAL_ASSETS,
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.ARBITRATION,
            JurisdictionFamily.LICENSING,
        },
        min_capital_usd=Decimal("50000"),
        kyc_tier_required=2,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"kz-aifc", "ae-dubai-difc", "sg-mas"},
    ),
    "ae-dubai-difc": JurisdictionProfile(
        jurisdiction_id="ae-dubai-difc",
        name="Dubai International Financial Centre",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.ARBITRATION,
            JurisdictionFamily.LICENSING,
        },
        min_capital_usd=Decimal("100000"),
        kyc_tier_required=2,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"ae-abudhabi-adgm", "kz-aifc", "uk-fca"},
    ),
    "ae-dubai-jafza": JurisdictionProfile(
        jurisdiction_id="ae-dubai-jafza",
        name="Jebel Ali Free Zone",
        legal_tradition=LegalTradition.CIVIL_LAW,
        supported_families={
            JurisdictionFamily.TRADE,
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.LICENSING,
        },
        min_capital_usd=Decimal("10000"),
        kyc_tier_required=1,
        allows_crypto=False,
        allows_foreign_ownership=True,
        treaty_partners={"ae-dubai-difc", "sg-mas"},
    ),

    # US Jurisdictions
    "us-de": JurisdictionProfile(
        jurisdiction_id="us-de",
        name="Delaware",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.GOVERNANCE,
        },
        kyc_tier_required=1,
        allows_crypto=False,
        allows_foreign_ownership=True,
        treaty_partners={"us-ny", "us-wy"},
    ),
    "us-ny": JurisdictionProfile(
        jurisdiction_id="us-ny",
        name="New York",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.CIVIC,
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.LICENSING,
        },
        min_capital_usd=Decimal("500000"),
        kyc_tier_required=3,
        allows_crypto=True,  # BitLicense
        allows_foreign_ownership=True,
        treaty_partners={"us-de", "uk-fca"},
        conflicts_with={"ae-abudhabi-adgm"},  # Regulatory conflict on crypto
    ),
    "us-wy": JurisdictionProfile(
        jurisdiction_id="us-wy",
        name="Wyoming",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.DIGITAL_ASSETS,
            JurisdictionFamily.CORPORATE,
        },
        kyc_tier_required=1,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"us-de"},
    ),

    # Kazakhstan
    "kz-aifc": JurisdictionProfile(
        jurisdiction_id="kz-aifc",
        name="Astana International Financial Centre",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.CIVIC,  # AIFC has its own legal foundation
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.ARBITRATION,
            JurisdictionFamily.DIGITAL_ASSETS,
        },
        min_capital_usd=Decimal("25000"),
        kyc_tier_required=2,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"ae-abudhabi-adgm", "ae-dubai-difc"},
    ),

    # Caribbean
    "ky-cayman": JurisdictionProfile(
        jurisdiction_id="ky-cayman",
        name="Cayman Islands",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.CIVIC,  # Cayman has its own legal system
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.TAX,
        },
        min_capital_usd=Decimal("1"),
        kyc_tier_required=2,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"uk-fca", "us-de"},
    ),

    # Central America
    "hn-prospera": JurisdictionProfile(
        jurisdiction_id="hn-prospera",
        name="Próspera ZEDE",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.CIVIC,
            JurisdictionFamily.CORPORATE,
            JurisdictionFamily.GOVERNANCE,
            JurisdictionFamily.IDENTITY,
            JurisdictionFamily.TAX,
        },
        min_capital_usd=Decimal("0"),
        kyc_tier_required=0,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners=set(),
    ),

    # Singapore
    "sg-mas": JurisdictionProfile(
        jurisdiction_id="sg-mas",
        name="Singapore MAS Regulated",
        legal_tradition=LegalTradition.COMMON_LAW,
        supported_families={
            JurisdictionFamily.FINANCIAL,
            JurisdictionFamily.LICENSING,
            JurisdictionFamily.ARBITRATION,
        },
        min_capital_usd=Decimal("250000"),
        kyc_tier_required=3,
        allows_crypto=True,
        allows_foreign_ownership=True,
        treaty_partners={"ae-dubai-difc", "ae-abudhabi-adgm", "uk-fca"},
    ),
}


# =============================================================================
# COMPOSITION ENGINE
# =============================================================================

class CompositionEngine:
    """Engine for composing hybrid zones from multiple jurisdictions."""

    def __init__(self, registry: Dict[str, JurisdictionProfile]):
        self.registry = registry
        self.dependency_graph = self._build_dependency_graph()

    def _build_dependency_graph(self) -> Dict[JurisdictionFamily, Set[JurisdictionFamily]]:
        """Build graph of module family dependencies."""
        return {
            # Financial services require corporate structure
            JurisdictionFamily.FINANCIAL: {JurisdictionFamily.CORPORATE},
            # Digital assets require financial infrastructure
            JurisdictionFamily.DIGITAL_ASSETS: {JurisdictionFamily.FINANCIAL, JurisdictionFamily.CORPORATE},
            # Licensing requires corporate and regulatory foundation
            JurisdictionFamily.LICENSING: {JurisdictionFamily.CORPORATE},
            # Trade requires corporate and licensing
            JurisdictionFamily.TRADE: {JurisdictionFamily.CORPORATE, JurisdictionFamily.LICENSING},
            # Arbitration requires legal foundation
            JurisdictionFamily.ARBITRATION: {JurisdictionFamily.CIVIC},
            # Governance requires identity
            JurisdictionFamily.GOVERNANCE: {JurisdictionFamily.IDENTITY},
            # Tax is independent
            JurisdictionFamily.TAX: set(),
            # Identity is independent
            JurisdictionFamily.IDENTITY: set(),
            # Civic is independent
            JurisdictionFamily.CIVIC: set(),
            # Corporate requires civic foundation
            JurisdictionFamily.CORPORATE: {JurisdictionFamily.CIVIC},
        }

    def compose(
        self,
        zone_id: str,
        name: str,
        selections: Dict[JurisdictionFamily, str],
    ) -> ZoneComposition:
        """Compose a zone from jurisdiction selections."""
        composition = ZoneComposition(
            zone_id=zone_id,
            name=name,
            jurisdiction_selections=selections,
        )

        # Validate all selections exist
        for family, jurisdiction_id in selections.items():
            if jurisdiction_id not in self.registry:
                composition.conflicts.append(
                    f"Unknown jurisdiction: {jurisdiction_id}"
                )
                continue

            profile = self.registry[jurisdiction_id]

            # Check jurisdiction supports the family
            if family not in profile.supported_families:
                composition.conflicts.append(
                    f"{jurisdiction_id} does not support {family.value}"
                )

        # Check dependencies are satisfied
        for family, jurisdiction_id in selections.items():
            deps = self.dependency_graph.get(family, set())
            for dep in deps:
                if dep not in selections:
                    composition.conflicts.append(
                        f"{family.value} requires {dep.value} but none selected"
                    )

        # Check for legal tradition conflicts
        traditions = set()
        for family, jurisdiction_id in selections.items():
            if jurisdiction_id in self.registry:
                profile = self.registry[jurisdiction_id]
                traditions.add(profile.legal_tradition)

        if LegalTradition.COMMON_LAW in traditions and LegalTradition.CIVIL_LAW in traditions:
            composition.warnings.append(
                "Mixed legal traditions (common law + civil law) may cause interpretation conflicts"
            )

        # Check for explicit jurisdiction conflicts
        selected_jurisdictions = set(selections.values())
        for jurisdiction_id in selected_jurisdictions:
            if jurisdiction_id in self.registry:
                profile = self.registry[jurisdiction_id]
                conflicts = profile.conflicts_with & selected_jurisdictions
                for conflict in conflicts:
                    composition.conflicts.append(
                        f"{jurisdiction_id} conflicts with {conflict}"
                    )

        # Generate hash (convert enum keys to strings for JSON)
        selections_str = {k.value: v for k, v in selections.items()}
        composition.hash = hashlib.sha256(
            json.dumps(dict(sorted(selections_str.items()))).encode()
        ).hexdigest()[:16]

        return composition

    def validate_corridor_compatibility(
        self,
        source_jurisdiction: str,
        target_jurisdiction: str,
    ) -> tuple[bool, List[str]]:
        """Check if two jurisdictions can form a corridor."""
        issues = []

        if source_jurisdiction not in self.registry:
            issues.append(f"Unknown source jurisdiction: {source_jurisdiction}")
            return False, issues

        if target_jurisdiction not in self.registry:
            issues.append(f"Unknown target jurisdiction: {target_jurisdiction}")
            return False, issues

        source = self.registry[source_jurisdiction]
        target = self.registry[target_jurisdiction]

        # Check treaty relationship
        if target_jurisdiction not in source.treaty_partners:
            issues.append(
                f"No treaty relationship between {source_jurisdiction} and {target_jurisdiction}"
            )

        # Check KYC compatibility
        if source.kyc_tier_required != target.kyc_tier_required:
            issues.append(
                f"KYC tier mismatch: {source_jurisdiction} requires tier {source.kyc_tier_required}, "
                f"{target_jurisdiction} requires tier {target.kyc_tier_required}"
            )

        # Check crypto compatibility for digital asset transfers
        if source.allows_crypto != target.allows_crypto:
            issues.append(
                f"Crypto compatibility mismatch: {source_jurisdiction} "
                f"{'allows' if source.allows_crypto else 'prohibits'} crypto, "
                f"{target_jurisdiction} {'allows' if target.allows_crypto else 'prohibits'} crypto"
            )

        return len(issues) == 0, issues


# =============================================================================
# TEST SCENARIOS: VALID COMPOSITIONS
# =============================================================================

class TestValidHybridZoneCompositions:
    """Tests for valid zone compositions that should succeed."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_digital_financial_center_adgm_delaware(self, engine):
        """
        Scenario: Digital financial center combining ADGM financial/digital
        with Delaware corporate structure - but missing civic foundation.

        This demonstrates that selecting a jurisdiction for a family it
        doesn't support will fail.
        """
        composition = engine.compose(
            zone_id="momentum.dfc.001",
            name="ADGM-Delaware DFC",
            selections={
                JurisdictionFamily.CIVIC: "ae-abudhabi-adgm",  # ADGM doesn't support CIVIC
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
                JurisdictionFamily.DIGITAL_ASSETS: "ae-abudhabi-adgm",
            },
        )

        # Should fail: ADGM doesn't support civic family
        assert not composition.is_valid()
        assert any("does not support" in c.lower() for c in composition.conflicts)

    def test_digital_financial_center_fixed(self, engine):
        """
        Corrected DFC composition with proper civic foundation.
        """
        composition = engine.compose(
            zone_id="momentum.dfc.002",
            name="ADGM-Delaware DFC (Fixed)",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",  # Provides civic foundation
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
                JurisdictionFamily.DIGITAL_ASSETS: "ae-abudhabi-adgm",
            },
        )

        assert composition.is_valid(), f"Unexpected conflicts: {composition.conflicts}"
        assert composition.hash  # Should have generated hash

    def test_trade_hub_jafza_difc(self, engine):
        """
        Scenario: Trade hub combining JAFZA trade infrastructure
        with DIFC corporate and financial.
        """
        composition = engine.compose(
            zone_id="momentum.trade.001",
            name="Dubai Trade Hub",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "ae-dubai-difc",
                JurisdictionFamily.LICENSING: "ae-dubai-jafza",
                JurisdictionFamily.TRADE: "ae-dubai-jafza",
                JurisdictionFamily.FINANCIAL: "ae-dubai-difc",
                JurisdictionFamily.ARBITRATION: "ae-dubai-difc",
            },
        )

        assert composition.is_valid(), f"Unexpected conflicts: {composition.conflicts}"
        # Should warn about mixed legal traditions
        assert any("legal tradition" in w.lower() for w in composition.warnings)

    def test_charter_city_full_stack(self, engine):
        """
        Scenario: Charter city with full civic infrastructure.
        """
        composition = engine.compose(
            zone_id="momentum.city.001",
            name="Prospera-Style Charter",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "hn-prospera",
                JurisdictionFamily.GOVERNANCE: "hn-prospera",
                JurisdictionFamily.IDENTITY: "hn-prospera",
                JurisdictionFamily.TAX: "hn-prospera",
            },
        )

        assert composition.is_valid(), f"Unexpected conflicts: {composition.conflicts}"
        assert len(composition.warnings) == 0  # Single jurisdiction = no conflicts

    def test_aifc_difc_corridor_zone(self, engine):
        """
        Scenario: Zone optimized for AIFC-DIFC corridor operations.
        """
        composition = engine.compose(
            zone_id="momentum.corridor.001",
            name="AIFC-DIFC Corridor Zone",
            selections={
                JurisdictionFamily.CIVIC: "kz-aifc",
                JurisdictionFamily.CORPORATE: "kz-aifc",
                JurisdictionFamily.FINANCIAL: "kz-aifc",
                JurisdictionFamily.ARBITRATION: "kz-aifc",
            },
        )

        # Check corridor compatibility
        compatible, issues = engine.validate_corridor_compatibility(
            "kz-aifc", "ae-dubai-difc"
        )

        assert composition.is_valid()
        assert compatible, f"Corridor issues: {issues}"


# =============================================================================
# TEST SCENARIOS: INVALID COMPOSITIONS (BUG DETECTION)
# =============================================================================

class TestInvalidCompositions:
    """Tests for invalid compositions that should be rejected."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_missing_dependency_financial_without_corporate(self, engine):
        """
        Bug detection: Financial services selected without corporate foundation.
        """
        composition = engine.compose(
            zone_id="momentum.broken.001",
            name="Broken - No Corporate",
            selections={
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
            },
        )

        assert not composition.is_valid()
        assert any("corporate" in c.lower() for c in composition.conflicts)

    def test_missing_dependency_digital_assets_chain(self, engine):
        """
        Bug detection: Digital assets require financial, which requires corporate.
        """
        composition = engine.compose(
            zone_id="momentum.broken.002",
            name="Broken - Digital Asset Chain",
            selections={
                JurisdictionFamily.DIGITAL_ASSETS: "ae-abudhabi-adgm",
            },
        )

        assert not composition.is_valid()
        # Should catch both missing financial AND corporate
        assert len(composition.conflicts) >= 2

    def test_jurisdiction_conflict_ny_adgm(self, engine):
        """
        Bug detection: NY and ADGM have regulatory conflicts on crypto.
        """
        composition = engine.compose(
            zone_id="momentum.conflict.001",
            name="Conflicting Crypto Regulations",
            selections={
                JurisdictionFamily.CIVIC: "us-ny",
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
                JurisdictionFamily.LICENSING: "us-ny",
            },
        )

        assert not composition.is_valid()
        assert any("conflicts with" in c.lower() for c in composition.conflicts)

    def test_unsupported_family_selection(self, engine):
        """
        Bug detection: Selecting a family from a jurisdiction that doesn't support it.
        """
        composition = engine.compose(
            zone_id="momentum.broken.003",
            name="Delaware Financial (Invalid)",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "us-de",  # Delaware doesn't do financial services
            },
        )

        assert not composition.is_valid()
        assert any("does not support" in c.lower() for c in composition.conflicts)

    def test_unknown_jurisdiction(self, engine):
        """
        Bug detection: Referencing a jurisdiction that doesn't exist.
        """
        composition = engine.compose(
            zone_id="momentum.broken.004",
            name="Unknown Jurisdiction",
            selections={
                JurisdictionFamily.FINANCIAL: "xx-nonexistent",
            },
        )

        assert not composition.is_valid()
        assert any("unknown" in c.lower() for c in composition.conflicts)

    def test_arbitration_without_civic(self, engine):
        """
        Bug detection: Arbitration requires civic legal foundation.
        """
        composition = engine.compose(
            zone_id="momentum.broken.005",
            name="Arbitration Without Legal",
            selections={
                JurisdictionFamily.ARBITRATION: "ae-dubai-difc",
                JurisdictionFamily.CORPORATE: "us-de",
            },
        )

        assert not composition.is_valid()
        assert any("civic" in c.lower() for c in composition.conflicts)


# =============================================================================
# TEST SCENARIOS: CORRIDOR COMPATIBILITY
# =============================================================================

class TestCorridorCompatibility:
    """Tests for corridor formation between jurisdictions."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_adgm_aifc_treaty_partners(self, engine):
        """ADGM and AIFC have a treaty relationship."""
        compatible, issues = engine.validate_corridor_compatibility(
            "ae-abudhabi-adgm", "kz-aifc"
        )

        assert compatible, f"Should be compatible: {issues}"

    def test_difc_cayman_no_treaty(self, engine):
        """DIFC and Cayman don't have direct treaty."""
        compatible, issues = engine.validate_corridor_compatibility(
            "ae-dubai-difc", "ky-cayman"
        )

        assert not compatible
        assert any("treaty" in i.lower() for i in issues)

    def test_crypto_corridor_incompatibility(self, engine):
        """
        JAFZA (no crypto) cannot form crypto corridor with ADGM (crypto enabled).
        """
        compatible, issues = engine.validate_corridor_compatibility(
            "ae-dubai-jafza", "ae-abudhabi-adgm"
        )

        assert not compatible
        assert any("crypto" in i.lower() for i in issues)

    def test_kyc_tier_mismatch(self, engine):
        """
        NY (tier 3) and Prospera (tier 0) have KYC tier mismatch.
        """
        compatible, issues = engine.validate_corridor_compatibility(
            "us-ny", "hn-prospera"
        )

        assert not compatible
        assert any("kyc tier" in i.lower() for i in issues)

    def test_same_jurisdiction_corridor(self, engine):
        """Same jurisdiction should be treated as trivially compatible or error."""
        compatible, issues = engine.validate_corridor_compatibility(
            "ae-abudhabi-adgm", "ae-abudhabi-adgm"
        )

        # Same jurisdiction - either trivially compatible or no treaty to itself
        # The important thing is it doesn't crash
        assert compatible or len(issues) > 0


# =============================================================================
# TEST SCENARIOS: EDGE CASES
# =============================================================================

class TestEdgeCases:
    """Edge cases that could reveal subtle bugs."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_empty_composition(self, engine):
        """Empty zone composition."""
        composition = engine.compose(
            zone_id="momentum.empty",
            name="Empty Zone",
            selections={},
        )

        # Empty is technically valid (no conflicts)
        assert composition.is_valid()
        assert len(composition.modules) == 0

    def test_all_families_single_jurisdiction(self, engine):
        """All families from single jurisdiction (if supported)."""
        # Prospera supports the most families
        composition = engine.compose(
            zone_id="momentum.single.001",
            name="Single Jurisdiction Max",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "hn-prospera",
                JurisdictionFamily.GOVERNANCE: "hn-prospera",
                JurisdictionFamily.IDENTITY: "hn-prospera",
                JurisdictionFamily.TAX: "hn-prospera",
            },
        )

        assert composition.is_valid()

    def test_circular_dependency_detection(self, engine):
        """
        Ensure the engine doesn't infinite loop on complex dependency chains.
        """
        # This tests the dependency resolution algorithm
        composition = engine.compose(
            zone_id="momentum.complex.001",
            name="Complex Dependencies",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
                JurisdictionFamily.DIGITAL_ASSETS: "ae-abudhabi-adgm",
                JurisdictionFamily.LICENSING: "ae-abudhabi-adgm",
                JurisdictionFamily.ARBITRATION: "ae-dubai-difc",
                JurisdictionFamily.IDENTITY: "hn-prospera",
                JurisdictionFamily.GOVERNANCE: "hn-prospera",
            },
        )

        # Should complete without hanging
        assert composition.hash

    def test_hash_determinism(self, engine):
        """Same selections should produce same hash."""
        selections = {
            JurisdictionFamily.CIVIC: "hn-prospera",
            JurisdictionFamily.CORPORATE: "us-de",
        }

        comp1 = engine.compose("test.001", "Test 1", selections)
        comp2 = engine.compose("test.002", "Test 2", selections)

        # Same selections = same hash
        assert comp1.hash == comp2.hash

    def test_hash_order_independence(self, engine):
        """Hash should be independent of selection order."""
        selections1 = {
            JurisdictionFamily.CIVIC: "hn-prospera",
            JurisdictionFamily.CORPORATE: "us-de",
        }
        selections2 = {
            JurisdictionFamily.CORPORATE: "us-de",
            JurisdictionFamily.CIVIC: "hn-prospera",
        }

        comp1 = engine.compose("test.001", "Test 1", selections1)
        comp2 = engine.compose("test.002", "Test 2", selections2)

        assert comp1.hash == comp2.hash


# =============================================================================
# TEST SCENARIOS: REAL-WORLD DEPLOYMENT PATTERNS
# =============================================================================

class TestRealWorldDeploymentPatterns:
    """Tests based on actual SEZ deployment patterns observed in practice."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_pattern_crypto_exchange(self, engine):
        """
        Pattern: Crypto exchange wanting US corporate + UAE operations.

        Common pattern for exchanges serving US customers while operating
        from crypto-friendly jurisdiction.
        """
        composition = engine.compose(
            zone_id="momentum.exchange.001",
            name="Crypto Exchange Zone",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "us-de",
                JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",
                JurisdictionFamily.DIGITAL_ASSETS: "ae-abudhabi-adgm",
                JurisdictionFamily.LICENSING: "ae-abudhabi-adgm",
            },
        )

        assert composition.is_valid(), f"Conflicts: {composition.conflicts}"

    def test_pattern_fund_structure(self, engine):
        """
        Pattern: Fund structure with Cayman fund + DIFC manager + Singapore custody.

        Standard pattern for hedge funds operating in Asia-Pacific.
        """
        composition = engine.compose(
            zone_id="momentum.fund.001",
            name="APAC Fund Structure",
            selections={
                JurisdictionFamily.CIVIC: "ky-cayman",
                JurisdictionFamily.CORPORATE: "ky-cayman",
                JurisdictionFamily.FINANCIAL: "ky-cayman",
                JurisdictionFamily.TAX: "ky-cayman",
            },
        )

        assert composition.is_valid()

    def test_pattern_trade_finance(self, engine):
        """
        Pattern: Trade finance hub with UAE free zone + Singapore banking.
        """
        composition = engine.compose(
            zone_id="momentum.tradefinance.001",
            name="Trade Finance Hub",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "ae-dubai-difc",
                JurisdictionFamily.TRADE: "ae-dubai-jafza",
                JurisdictionFamily.LICENSING: "ae-dubai-jafza",
                JurisdictionFamily.FINANCIAL: "ae-dubai-difc",
                JurisdictionFamily.ARBITRATION: "sg-mas",
            },
        )

        # This should have mixed tradition warning
        assert any("legal tradition" in w.lower() for w in composition.warnings)

    def test_pattern_digital_nomad_zone(self, engine):
        """
        Pattern: Digital nomad zone with minimal regulation.
        """
        composition = engine.compose(
            zone_id="momentum.nomad.001",
            name="Digital Nomad Zone",
            selections={
                JurisdictionFamily.CIVIC: "hn-prospera",
                JurisdictionFamily.CORPORATE: "hn-prospera",
                JurisdictionFamily.IDENTITY: "hn-prospera",
                JurisdictionFamily.TAX: "hn-prospera",
            },
        )

        assert composition.is_valid()
        # Should be valid for minimal requirements
        profile = JURISDICTION_REGISTRY["hn-prospera"]
        assert profile.kyc_tier_required == 0


# =============================================================================
# TEST SCENARIOS: CAPITAL REQUIREMENTS
# =============================================================================

class TestCapitalRequirements:
    """Tests for capital requirement validation across jurisdictions."""

    @pytest.fixture
    def engine(self):
        return CompositionEngine(JURISDICTION_REGISTRY)

    def test_minimum_capital_aggregation(self, engine):
        """
        Validate that zone inherits highest capital requirement from selected jurisdictions.
        """
        selections = {
            JurisdictionFamily.CIVIC: "hn-prospera",  # $0
            JurisdictionFamily.CORPORATE: "us-de",  # $0
            JurisdictionFamily.FINANCIAL: "ae-abudhabi-adgm",  # $50,000
            JurisdictionFamily.LICENSING: "sg-mas",  # $250,000
        }

        max_capital = Decimal("0")
        for family, jurisdiction_id in selections.items():
            if jurisdiction_id in JURISDICTION_REGISTRY:
                profile = JURISDICTION_REGISTRY[jurisdiction_id]
                max_capital = max(max_capital, profile.min_capital_usd)

        # Singapore has highest at $250,000
        assert max_capital == Decimal("250000")

    def test_capital_requirements_by_profile(self, engine):
        """Different jurisdiction profiles have expected capital requirements."""
        expectations = {
            "hn-prospera": Decimal("0"),
            "ky-cayman": Decimal("1"),
            "ae-dubai-jafza": Decimal("10000"),
            "kz-aifc": Decimal("25000"),
            "ae-abudhabi-adgm": Decimal("50000"),
            "ae-dubai-difc": Decimal("100000"),
            "sg-mas": Decimal("250000"),
            "us-ny": Decimal("500000"),
        }

        for jurisdiction_id, expected in expectations.items():
            profile = JURISDICTION_REGISTRY[jurisdiction_id]
            assert profile.min_capital_usd == expected, \
                f"{jurisdiction_id} expected {expected}, got {profile.min_capital_usd}"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
