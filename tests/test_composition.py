"""Comprehensive tests for the multi-jurisdiction composition engine.

Tests cover:
- Domain enumeration and layer construction
- Composition validation and conflict detection
- Zone YAML generation
- Stack lock generation
- Digest computation
- Complex multi-layer compositions
- Error handling and edge cases
"""

import json
import pytest
from pathlib import Path

from tools.msez.composition import (
    ArbitrationConfig,
    ArbitrationMode,
    CorridorConfig,
    Domain,
    JurisdictionLayer,
    LawpackRef,
    LicensepackRef,
    RegpackRef,
    ZoneComposition,
    compose_zone,
    load_composition_from_yaml,
)


class TestDomain:
    """Test Domain enumeration."""

    def test_domain_values(self):
        """All expected domains exist."""
        expected = {
            "civic", "corporate", "commercial", "financial", "securities",
            "banking", "payments", "custody", "clearing", "settlement",
            "digital-assets", "tax", "employment", "immigration",
            "intellectual-property", "data-protection", "aml-cft",
            "consumer-protection", "arbitration", "licensing",
        }
        actual = {d.value for d in Domain}
        assert actual == expected

    def test_domain_from_string(self):
        """Can construct Domain from string value."""
        assert Domain("civic") == Domain.CIVIC
        assert Domain("digital-assets") == Domain.DIGITAL_ASSETS

    def test_invalid_domain_raises(self):
        """Invalid domain string raises ValueError."""
        with pytest.raises(ValueError):
            Domain("invalid-domain")


class TestLawpackRef:
    """Test LawpackRef dataclass."""

    def test_basic_construction(self):
        """Basic construction works."""
        ref = LawpackRef(
            jurisdiction_id="us-de",
            domain="corporate",
            digest_sha256="a" * 64,
        )
        assert ref.jurisdiction_id == "us-de"
        assert ref.domain == "corporate"
        assert ref.digest_sha256 == "a" * 64

    def test_to_dict_minimal(self):
        """to_dict with minimal fields."""
        ref = LawpackRef(
            jurisdiction_id="us-ny",
            domain="civic",
            digest_sha256="b" * 64,
        )
        d = ref.to_dict()
        assert d == {
            "jurisdiction_id": "us-ny",
            "domain": "civic",
            "lawpack_digest_sha256": "b" * 64,
        }

    def test_to_dict_full(self):
        """to_dict with all fields."""
        ref = LawpackRef(
            jurisdiction_id="us-de",
            domain="corporate",
            digest_sha256="c" * 64,
            version="2024.01",
            effective_date="2024-01-01",
        )
        d = ref.to_dict()
        assert d["version"] == "2024.01"
        assert d["effective_date"] == "2024-01-01"


class TestJurisdictionLayer:
    """Test JurisdictionLayer dataclass."""

    def test_basic_construction(self):
        """Basic layer construction."""
        layer = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE, Domain.COMMERCIAL],
            description="Delaware corporate law",
        )
        assert layer.jurisdiction_id == "us-de"
        assert Domain.CORPORATE in layer.domains
        assert len(layer.lawpacks) == 0

    def test_validate_valid_layer(self):
        """Valid layer passes validation."""
        layer = JurisdictionLayer(
            jurisdiction_id="ae-abudhabi-adgm",
            domains=[Domain.FINANCIAL, Domain.DIGITAL_ASSETS],
        )
        errors = layer.validate()
        assert len(errors) == 0

    def test_validate_invalid_jurisdiction_id(self):
        """Invalid jurisdiction_id format detected."""
        layer = JurisdictionLayer(
            jurisdiction_id="Invalid-ID",
            domains=[Domain.CIVIC],
        )
        errors = layer.validate()
        assert any("jurisdiction_id" in e.lower() for e in errors)

    def test_validate_empty_domains(self):
        """Empty domains list detected."""
        layer = JurisdictionLayer(
            jurisdiction_id="us-ny",
            domains=[],
        )
        errors = layer.validate()
        assert any("no domains" in e.lower() for e in errors)

    def test_validate_invalid_lawpack_digest(self):
        """Invalid lawpack digest detected."""
        layer = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE],
            lawpacks=[
                LawpackRef(
                    jurisdiction_id="us-de",
                    domain="corporate",
                    digest_sha256="invalid",
                )
            ],
        )
        errors = layer.validate()
        assert any("lawpack" in e.lower() and "digest" in e.lower() for e in errors)

    def test_domain_set(self):
        """domain_set returns correct set."""
        layer = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE, Domain.COMMERCIAL, Domain.CORPORATE],  # duplicate
        )
        ds = layer.domain_set()
        assert ds == {Domain.CORPORATE, Domain.COMMERCIAL}


class TestArbitrationConfig:
    """Test ArbitrationConfig dataclass."""

    def test_default_construction(self):
        """Default construction is traditional mode."""
        config = ArbitrationConfig()
        assert config.mode == ArbitrationMode.TRADITIONAL
        assert config.appeal_allowed is True

    def test_ai_assisted_mode(self):
        """AI-assisted mode configuration."""
        config = ArbitrationConfig(
            mode=ArbitrationMode.AI_ASSISTED,
            ai_model="claude-opus-4-5-20251101",
            human_review_threshold_usd=100000,
        )
        assert config.mode == ArbitrationMode.AI_ASSISTED
        assert config.ai_model == "claude-opus-4-5-20251101"

    def test_to_dict(self):
        """to_dict serializes correctly."""
        config = ArbitrationConfig(
            mode=ArbitrationMode.AI_ASSISTED,
            institution_id="difc-lcia",
            ai_model="claude-opus-4-5-20251101",
            human_review_threshold_usd=50000,
            max_claim_usd=1000000,
        )
        d = config.to_dict()
        assert d["mode"] == "ai-assisted"
        assert d["institution_id"] == "difc-lcia"
        assert d["ai_model"] == "claude-opus-4-5-20251101"
        assert d["human_review_threshold_usd"] == 50000
        assert d["max_claim_usd"] == 1000000


class TestZoneComposition:
    """Test ZoneComposition dataclass."""

    def test_basic_construction(self):
        """Basic composition construction."""
        comp = ZoneComposition(
            zone_id="test.zone.1",
            name="Test Zone",
        )
        assert comp.zone_id == "test.zone.1"
        assert comp.name == "Test Zone"
        assert len(comp.layers) == 0

    def test_validate_valid_composition(self):
        """Valid composition passes validation."""
        comp = ZoneComposition(
            zone_id="momentum.test.zone",
            name="Test Zone",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-ny",
                    domains=[Domain.CIVIC],
                ),
                JurisdictionLayer(
                    jurisdiction_id="us-de",
                    domains=[Domain.CORPORATE],
                ),
            ],
        )
        errors = comp.validate()
        assert len(errors) == 0

    def test_validate_domain_conflict(self):
        """Domain conflict detected."""
        comp = ZoneComposition(
            zone_id="test.conflict",
            name="Conflict Zone",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-ny",
                    domains=[Domain.CORPORATE],  # NY corporate
                ),
                JurisdictionLayer(
                    jurisdiction_id="us-de",
                    domains=[Domain.CORPORATE],  # DE corporate - conflict!
                ),
            ],
        )
        errors = comp.validate()
        assert any("conflict" in e.lower() for e in errors)

    def test_validate_invalid_zone_id(self):
        """Invalid zone_id detected."""
        comp = ZoneComposition(
            zone_id="Invalid Zone ID!",
            name="Test",
        )
        errors = comp.validate()
        assert any("zone_id" in e.lower() for e in errors)

    def test_all_domains(self):
        """all_domains aggregates correctly."""
        comp = ZoneComposition(
            zone_id="test.zone",
            name="Test",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-ny",
                    domains=[Domain.CIVIC, Domain.TAX],
                ),
                JurisdictionLayer(
                    jurisdiction_id="us-de",
                    domains=[Domain.CORPORATE],
                ),
            ],
        )
        domains = comp.all_domains()
        assert domains == {Domain.CIVIC, Domain.TAX, Domain.CORPORATE}

    def test_domain_coverage_report(self):
        """domain_coverage_report maps correctly."""
        comp = ZoneComposition(
            zone_id="test.zone",
            name="Test",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-ny",
                    domains=[Domain.CIVIC],
                ),
                JurisdictionLayer(
                    jurisdiction_id="us-de",
                    domains=[Domain.CORPORATE],
                ),
            ],
        )
        report = comp.domain_coverage_report()
        assert report["civic"] == "us-ny"
        assert report["corporate"] == "us-de"

    def test_to_zone_yaml(self):
        """to_zone_yaml generates correct structure."""
        comp = ZoneComposition(
            zone_id="momentum.hybrid.demo",
            name="Hybrid Demo Zone",
            description="Test zone",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-ny",
                    domains=[Domain.CIVIC],
                    description="NY civic",
                ),
                JurisdictionLayer(
                    jurisdiction_id="ae-abudhabi-adgm",
                    domains=[Domain.FINANCIAL, Domain.DIGITAL_ASSETS],
                    description="ADGM financial",
                ),
            ],
            arbitration=ArbitrationConfig(
                mode=ArbitrationMode.AI_ASSISTED,
                ai_model="claude-opus-4-5-20251101",
            ),
        )
        yaml_data = comp.to_zone_yaml()

        assert yaml_data["zone_id"] == "momentum.hybrid.demo"
        assert yaml_data["spec_version"] == "0.4.44"
        assert "composition" in yaml_data
        assert len(yaml_data["composition"]["layers"]) == 2
        assert "arbitration" in yaml_data
        assert yaml_data["arbitration"]["mode"] == "ai-assisted"

    def test_composition_digest_deterministic(self):
        """composition_digest is deterministic."""
        comp1 = ZoneComposition(
            zone_id="test.zone",
            name="Test",
            layers=[
                JurisdictionLayer(jurisdiction_id="us-de", domains=[Domain.CORPORATE]),
                JurisdictionLayer(jurisdiction_id="us-ny", domains=[Domain.CIVIC]),
            ],
        )
        comp2 = ZoneComposition(
            zone_id="test.zone",
            name="Different Name",  # Name doesn't affect digest
            layers=[
                # Different order - should produce same digest after sorting
                JurisdictionLayer(jurisdiction_id="us-ny", domains=[Domain.CIVIC]),
                JurisdictionLayer(jurisdiction_id="us-de", domains=[Domain.CORPORATE]),
            ],
        )
        assert comp1.composition_digest() == comp2.composition_digest()

    def test_to_stack_lock(self):
        """to_stack_lock generates correct structure."""
        comp = ZoneComposition(
            zone_id="test.zone",
            name="Test",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="us-de",
                    domains=[Domain.CORPORATE],
                    lawpacks=[
                        LawpackRef(
                            jurisdiction_id="us-de",
                            domain="corporate",
                            digest_sha256="a" * 64,
                        )
                    ],
                ),
            ],
        )
        lock = comp.to_stack_lock()

        assert lock["zone_id"] == "test.zone"
        assert lock["spec_version"] == "0.4.44"
        assert "lawpacks" in lock
        assert len(lock["lawpacks"]) == 1
        assert "composition_digest" in lock


class TestComposeZone:
    """Test compose_zone convenience function."""

    def test_simple_composition(self):
        """Simple composition with civic and corporate layers."""
        zone = compose_zone(
            "test.simple",
            "Simple Test Zone",
            civic=("us-ny", "New York civic code"),
            corporate=("us-de", "Delaware corporate law"),
        )
        assert zone.zone_id == "test.simple"
        assert len(zone.layers) == 2
        assert Domain.CIVIC in zone.all_domains()
        assert Domain.CORPORATE in zone.all_domains()

    def test_financial_layer(self):
        """Composition with financial layer."""
        zone = compose_zone(
            "test.financial",
            "Financial Zone",
            financial=("ae-abudhabi-adgm", "ADGM financial services"),
        )
        domains = zone.all_domains()
        assert Domain.FINANCIAL in domains
        assert Domain.BANKING in domains
        assert Domain.PAYMENTS in domains
        assert Domain.SETTLEMENT in domains

    def test_merged_financial_and_digital_assets(self):
        """Same jurisdiction for financial and digital assets merges."""
        zone = compose_zone(
            "test.merged",
            "Merged Zone",
            financial=("ae-abudhabi-adgm", "ADGM financial"),
            digital_assets=("ae-abudhabi-adgm", "ADGM digital assets"),
        )
        # Should be single layer since same jurisdiction
        assert len(zone.layers) == 1
        domains = zone.all_domains()
        assert Domain.FINANCIAL in domains
        assert Domain.DIGITAL_ASSETS in domains

    def test_separate_financial_and_digital_assets(self):
        """Different jurisdictions create separate layers."""
        zone = compose_zone(
            "test.separate",
            "Separate Zone",
            financial=("sg-mas", "Singapore financial"),
            digital_assets=("ae-abudhabi-adgm", "ADGM digital assets"),
        )
        assert len(zone.layers) == 2

    def test_ai_arbitration(self):
        """AI arbitration configuration."""
        zone = compose_zone(
            "test.ai.arb",
            "AI Arbitration Zone",
            civic=("us-ny", "NY"),
            ai_arbitration=True,
        )
        assert zone.arbitration is not None
        assert zone.arbitration.mode == ArbitrationMode.AI_ASSISTED
        assert zone.arbitration.ai_model == "claude-opus-4-5-20251101"

    def test_full_hybrid_composition(self):
        """Full hybrid composition like the example."""
        zone = compose_zone(
            "momentum.hybrid.nyc-de-adgm",
            "NYC-Delaware-ADGM Hybrid Zone",
            civic=("us-ny", "New York State civic code"),
            corporate=("us-de", "Delaware General Corporation Law"),
            financial=("ae-abudhabi-adgm", "ADGM Financial Services Framework"),
            digital_assets=("ae-abudhabi-adgm", "ADGM digital asset regulations"),
            ai_arbitration=True,
            description="Hybrid zone: NY civic + DE corporate + ADGM financial/digital",
        )

        # Validate structure
        errors = zone.validate()
        assert len(errors) == 0

        # Check layers
        assert len(zone.layers) == 3  # NY, DE, ADGM (merged)

        # Check domains
        domains = zone.all_domains()
        assert Domain.CIVIC in domains
        assert Domain.CORPORATE in domains
        assert Domain.COMMERCIAL in domains
        assert Domain.FINANCIAL in domains
        assert Domain.DIGITAL_ASSETS in domains

        # Check domain sources
        report = zone.domain_coverage_report()
        assert report["civic"] == "us-ny"
        assert report["corporate"] == "us-de"
        assert report["financial"] == "ae-abudhabi-adgm"
        assert report["digital-assets"] == "ae-abudhabi-adgm"

        # Check arbitration
        assert zone.arbitration is not None
        assert zone.arbitration.mode == ArbitrationMode.AI_ASSISTED


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_zone_id_invalid(self):
        """Empty zone_id is invalid."""
        comp = ZoneComposition(zone_id="", name="Test")
        errors = comp.validate()
        assert len(errors) > 0

    def test_zone_id_special_chars_invalid(self):
        """Zone ID with special chars is invalid."""
        comp = ZoneComposition(zone_id="test zone!", name="Test")
        errors = comp.validate()
        assert any("zone_id" in e.lower() for e in errors)

    def test_jurisdiction_uppercase_invalid(self):
        """Uppercase jurisdiction ID is invalid."""
        layer = JurisdictionLayer(
            jurisdiction_id="US-NY",  # Should be lowercase
            domains=[Domain.CIVIC],
        )
        errors = layer.validate()
        assert len(errors) > 0

    def test_many_layers_composition(self):
        """Composition with many layers."""
        layers = [
            JurisdictionLayer(
                jurisdiction_id=f"xx-region-{i}",
                domains=[list(Domain)[i % len(Domain)]],
            )
            for i in range(10)
        ]
        comp = ZoneComposition(
            zone_id="test.many.layers",
            name="Many Layers",
            layers=layers,
        )
        # May have conflicts depending on domain distribution
        _ = comp.validate()

    def test_composition_with_corridors(self):
        """Composition with settlement corridors."""
        comp = ZoneComposition(
            zone_id="test.corridors",
            name="Corridor Zone",
            layers=[
                JurisdictionLayer(
                    jurisdiction_id="ae-abudhabi-adgm",
                    domains=[Domain.FINANCIAL, Domain.SETTLEMENT],
                ),
            ],
            corridors=[
                CorridorConfig(
                    corridor_id="adgm-sg-mas",
                    source_jurisdiction="ae-abudhabi-adgm",
                    target_jurisdiction="sg-mas",
                    settlement_currency="USD",
                    finality_seconds=3600,
                ),
            ],
        )
        yaml_data = comp.to_zone_yaml()
        assert "corridors" in yaml_data
        assert len(yaml_data["corridors"]) == 1
        assert yaml_data["corridors"][0]["corridor_id"] == "adgm-sg-mas"


class TestCorridorConfig:
    """Test CorridorConfig dataclass."""

    def test_basic_construction(self):
        """Basic corridor construction."""
        corridor = CorridorConfig(
            corridor_id="test-corridor",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="sg-mas",
        )
        assert corridor.corridor_id == "test-corridor"
        assert corridor.settlement_currency == "USD"
        assert corridor.finality_seconds == 3600

    def test_to_dict(self):
        """to_dict serializes all fields."""
        corridor = CorridorConfig(
            corridor_id="adgm-to-hk",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="hk-hkma",
            settlement_currency="HKD",
            settlement_mechanism="rtgs",
            max_settlement_usd=10000000,
            finality_seconds=1800,
        )
        d = corridor.to_dict()
        assert d["corridor_id"] == "adgm-to-hk"
        assert d["settlement_currency"] == "HKD"
        assert d["max_settlement_usd"] == 10000000


class TestRegpackRef:
    """Test RegpackRef dataclass."""

    def test_basic_construction(self):
        """Basic regpack reference."""
        ref = RegpackRef(
            jurisdiction_id="ae-abudhabi-adgm",
            domain="financial",
            digest_sha256="d" * 64,
        )
        assert ref.jurisdiction_id == "ae-abudhabi-adgm"

    def test_to_dict(self):
        """to_dict serializes correctly."""
        ref = RegpackRef(
            jurisdiction_id="sg-mas",
            domain="aml-cft",
            digest_sha256="e" * 64,
            as_of_date="2024-01-15",
        )
        d = ref.to_dict()
        assert d["domain"] == "aml-cft"
        assert d["as_of_date"] == "2024-01-15"


class TestLicensepackRef:
    """Test LicensepackRef dataclass."""

    def test_basic_construction(self):
        """Basic licensepack reference."""
        ref = LicensepackRef(
            jurisdiction_id="ae-dubai-difc",
            domain="financial",
            digest_sha256="f" * 64,
        )
        assert ref.jurisdiction_id == "ae-dubai-difc"

    def test_to_dict_with_includes(self):
        """to_dict with includes metadata."""
        ref = LicensepackRef(
            jurisdiction_id="ae-dubai-difc",
            domain="financial",
            digest_sha256="f" * 64,
            snapshot_timestamp="2024-01-15T10:30:00Z",
            includes={
                "licenses_active": 150,
                "licenses_total": 200,
                "license_types": 12,
            },
        )
        d = ref.to_dict()
        assert d["snapshot_timestamp"] == "2024-01-15T10:30:00Z"
        assert d["includes"]["licenses_active"] == 150


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
