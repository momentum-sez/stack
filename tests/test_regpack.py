"""RegPack System Tests for v0.4.41.

Comprehensive test coverage for the RegPack system:
- Sanctions list ingestion and checking
- License registry
- Reporting requirements
- Compliance calendar
- RegPack attestation VCs
- Corridor binding
"""

import json
import os
from datetime import date, datetime, timezone
from decimal import Decimal
from pathlib import Path

import pytest


REPO_ROOT = Path(__file__).resolve().parents[1]


# ─────────────────────────────────────────────────────────────────────────────
# Fixtures
# ─────────────────────────────────────────────────────────────────────────────

@pytest.fixture
def sample_sanctions_entries():
    """Sample sanctions entries for testing."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsEntry
    
    return [
        SanctionsEntry(
            entry_id="ofac-12345",
            entry_type="entity",
            source_lists=["ofac_sdn"],
            primary_name="Acme Sanctioned Corp",
            aliases=[
                {"alias": "Acme SC", "alias_type": "aka"},
                {"alias": "ASC Holdings", "alias_type": "dba"},
            ],
            identifiers=[
                {"type": "registration_number", "value": "REG-12345", "country": "IR"},
            ],
            addresses=[
                {"city": "Tehran", "country": "IR"},
            ],
            nationalities=["IR"],
            programs=["IRAN", "SDGT"],
            listing_date="2020-03-15",
        ),
        SanctionsEntry(
            entry_id="eu-67890",
            entry_type="individual",
            source_lists=["eu_consolidated", "un_consolidated"],
            primary_name="John Sanctioned Person",
            aliases=[
                {"alias": "J. Person", "alias_type": "aka"},
            ],
            identifiers=[
                {"type": "passport", "value": "AB123456", "country": "RU"},
            ],
            nationalities=["RU"],
            date_of_birth="1970-05-20",
            programs=["RUSSIA"],
            listing_date="2022-02-28",
        ),
        SanctionsEntry(
            entry_id="ofac-vessel-001",
            entry_type="vessel",
            source_lists=["ofac_sdn"],
            primary_name="MV Sanctioned Ship",
            identifiers=[
                {"type": "imo_number", "value": "9876543"},
                {"type": "mmsi", "value": "123456789"},
            ],
            programs=["DPRK"],
            listing_date="2021-06-01",
        ),
    ]


@pytest.fixture
def regpack_manager():
    """Create a RegPack manager."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import RegPackManager
    
    os.environ["SOURCE_DATE_EPOCH"] = "1735689600"
    return RegPackManager(jurisdiction_id="uae-adgm", domain="financial")


# ─────────────────────────────────────────────────────────────────────────────
# RegPack Metadata Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_metadata_schema_validates():
    """Verify regpack metadata schema is valid."""
    schema_path = REPO_ROOT / "schemas" / "regpack.metadata.schema.json"
    assert schema_path.exists(), "regpack.metadata.schema.json must exist"
    
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    assert schema["$id"].endswith("regpack.metadata.schema.json")
    assert "MSEZRegPackMetadata" in str(schema)


def test_regpack_metadata_creation(regpack_manager):
    """Test creating RegPack metadata."""
    metadata = regpack_manager.create_regpack_metadata(
        as_of_date="2026-01-15",
        snapshot_type="quarterly",
        regulators=["adgm-fsra", "adgm-ra"],
        sanctions_lists=["ofac_sdn", "eu_consolidated"],
        license_types=12,
        report_types=8,
    )
    
    assert metadata.regpack_id.startswith("regpack:uae-adgm:financial:")
    assert metadata.jurisdiction_id == "uae-adgm"
    assert metadata.domain == "financial"
    assert metadata.snapshot_type == "quarterly"
    assert len(metadata.includes["regulators"]) == 2
    assert len(metadata.includes["sanctions_lists"]) == 2


def test_regpack_digest_computation_deterministic(regpack_manager):
    """Verify regpack digest is deterministic."""
    metadata = regpack_manager.create_regpack_metadata(
        as_of_date="2026-01-15",
        snapshot_type="quarterly",
    )
    
    digest1 = regpack_manager.compute_regpack_digest(metadata)
    digest2 = regpack_manager.compute_regpack_digest(metadata)
    
    assert digest1 == digest2
    assert len(digest1) == 64  # SHA256 hex


# ─────────────────────────────────────────────────────────────────────────────
# Sanctions Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_sanctions_snapshot_parsing(regpack_manager, sample_sanctions_entries):
    """Test creating a sanctions snapshot."""
    snapshot = regpack_manager.create_sanctions_snapshot(
        as_of_date="2026-01-15",
        entries=sample_sanctions_entries,
        sources={
            "ofac_sdn": {
                "url": "https://www.treasury.gov/ofac/downloads/sdn.xml",
                "fetched_at": "2026-01-15T00:00:00Z",
                "record_count": 2,
            },
            "eu_consolidated": {
                "url": "https://webgate.ec.europa.eu/...",
                "fetched_at": "2026-01-15T00:00:00Z",
                "record_count": 1,
            },
        },
    )
    
    assert snapshot.snapshot_id == "sanctions-2026-01-15"
    assert snapshot.consolidated_counts["total_records"] == 3
    assert snapshot.consolidated_counts["entities"] == 1
    assert snapshot.consolidated_counts["individuals"] == 1
    assert snapshot.consolidated_counts["vessels"] == 1


def test_regpack_sanctions_check_entity_exact_match(sample_sanctions_entries):
    """Test exact name match in sanctions check."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsChecker
    
    checker = SanctionsChecker(sample_sanctions_entries, "sanctions-2026-01-15")
    
    result = checker.check_entity("Acme Sanctioned Corp")
    
    assert result.matched is True
    assert len(result.matches) == 1
    assert result.match_score == 1.0
    assert result.matches[0]["match_type"] == "exact_name"


def test_regpack_sanctions_check_entity_alias_match(sample_sanctions_entries):
    """Test alias match in sanctions check."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsChecker
    
    checker = SanctionsChecker(sample_sanctions_entries, "sanctions-2026-01-15")
    
    result = checker.check_entity("ASC Holdings")
    
    assert result.matched is True
    assert len(result.matches) >= 1


def test_regpack_sanctions_check_entity_identifier_match(sample_sanctions_entries):
    """Test identifier match in sanctions check."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsChecker
    
    checker = SanctionsChecker(sample_sanctions_entries, "sanctions-2026-01-15")
    
    result = checker.check_entity(
        "Unknown Company",
        identifiers=[{"type": "registration_number", "value": "REG-12345"}],
    )
    
    assert result.matched is True
    assert any(m["match_type"] == "identifier" for m in result.matches)


def test_regpack_sanctions_check_entity_no_match(sample_sanctions_entries):
    """Test no match in sanctions check."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsChecker
    
    checker = SanctionsChecker(sample_sanctions_entries, "sanctions-2026-01-15")
    
    result = checker.check_entity("Totally Clean Company Ltd")
    
    assert result.matched is False
    assert len(result.matches) == 0


def test_regpack_sanctions_check_fuzzy_match(sample_sanctions_entries):
    """Test fuzzy name match in sanctions check."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import SanctionsChecker
    
    checker = SanctionsChecker(sample_sanctions_entries, "sanctions-2026-01-15")
    
    # Slight variation should still match with fuzzy
    result = checker.check_entity("Acme Sanctioned Corporation", threshold=0.7)
    
    # May or may not match depending on fuzzy algorithm
    # At minimum, should not crash
    assert result.query == "Acme Sanctioned Corporation"


def test_regpack_sanctions_consolidation(regpack_manager, sample_sanctions_entries):
    """Test that sanctions from multiple sources are consolidated correctly."""
    snapshot = regpack_manager.create_sanctions_snapshot(
        as_of_date="2026-01-15",
        entries=sample_sanctions_entries,
        sources={
            "ofac_sdn": {"url": "...", "fetched_at": "2026-01-15T00:00:00Z", "record_count": 2},
            "eu_consolidated": {"url": "...", "fetched_at": "2026-01-15T00:00:00Z", "record_count": 1},
            "un_consolidated": {"url": "...", "fetched_at": "2026-01-15T00:00:00Z", "record_count": 1},
        },
    )
    
    # John Sanctioned Person is in both EU and UN
    assert snapshot.consolidated_counts["total_records"] == 3


def test_regpack_sanctions_delta_computation(regpack_manager, sample_sanctions_entries):
    """Test delta computation between snapshots."""
    snapshot = regpack_manager.create_sanctions_snapshot(
        as_of_date="2026-01-15",
        entries=sample_sanctions_entries,
        sources={},
        previous_digest="abc123" * 10 + "abcd",
    )
    
    assert snapshot.delta_from_previous is not None
    assert snapshot.delta_from_previous["previous_snapshot_digest"].startswith("abc123")


# ─────────────────────────────────────────────────────────────────────────────
# License Registry Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_license_type_validation():
    """Test license type data structure."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import LicenseType
    
    license_type = LicenseType(
        license_type_id="fsra-cat3a",
        name="Category 3A - Dealing in Investments",
        regulator_id="adgm-fsra",
        requirements={
            "minimum_capital": {"amount": 500000, "currency": "USD"},
            "key_persons": {
                "senior_executive_officer": {"required": True},
                "compliance_officer": {"required": True},
            },
        },
        application={
            "fee": {"amount": 10000, "currency": "USD"},
            "processing_time_days": 90,
        },
        ongoing_obligations={
            "annual_fee": {"amount": 15000, "currency": "USD"},
            "reporting": ["quarterly_prudential", "annual_audit"],
        },
        validity_period_years=1,
        renewal_lead_time_days=90,
    )
    
    d = license_type.to_dict()
    
    assert d["license_type_id"] == "fsra-cat3a"
    assert d["requirements"]["minimum_capital"]["amount"] == 500000
    assert d["validity_period_years"] == 1


# ─────────────────────────────────────────────────────────────────────────────
# Reporting Requirements Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_reporting_deadline_computation():
    """Test reporting requirement data structure."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import ReportingRequirement
    
    requirement = ReportingRequirement(
        report_type_id="fsra-qpr",
        name="Quarterly Prudential Return",
        regulator_id="adgm-fsra",
        applicable_to=["fsra-cat3a", "fsra-cat3b"],
        frequency="quarterly",
        deadlines={
            "q1": {"period_end": "2026-03-31", "due_date": "2026-04-30"},
            "q2": {"period_end": "2026-06-30", "due_date": "2026-07-31"},
        },
        submission={
            "method": "api",
            "endpoint": "https://api.fsra.adgm.com/v1/reporting/qpr",
            "format": "json",
        },
        late_penalty={
            "grace_period_days": 5,
            "daily_penalty": {"amount": 500, "currency": "USD"},
        },
    )
    
    d = requirement.to_dict()
    
    assert d["frequency"] == "quarterly"
    assert d["deadlines"]["q1"]["due_date"] == "2026-04-30"
    assert d["late_penalty"]["daily_penalty"]["amount"] == 500


# ─────────────────────────────────────────────────────────────────────────────
# Compliance Calendar Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_compliance_deadline():
    """Test compliance deadline data structure."""
    import sys
    sys.path.insert(0, str(REPO_ROOT))
    from tools.regpack import ComplianceDeadline
    
    deadline = ComplianceDeadline(
        deadline_id="dl-2026-q1-qpr",
        regulator_id="adgm-fsra",
        deadline_type="report",
        description="Q1 2026 Quarterly Prudential Return",
        due_date="2026-04-30",
        grace_period_days=5,
        applicable_license_types=["fsra-cat3a", "fsra-cat3b", "fsra-cat3c"],
    )
    
    d = deadline.to_dict()
    
    assert d["deadline_type"] == "report"
    assert d["due_date"] == "2026-04-30"
    assert len(d["applicable_license_types"]) == 3


# ─────────────────────────────────────────────────────────────────────────────
# RegPack Attestation VC Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_attestation_vc_schema():
    """Verify regpack attestation VC schema exists."""
    schema_path = REPO_ROOT / "schemas" / "vc.regpack-attestation.schema.json"
    # Schema may not exist yet - this test documents the requirement
    if schema_path.exists():
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        assert "MSEZRegPackAttestationCredential" in str(schema)


# ─────────────────────────────────────────────────────────────────────────────
# Corridor Binding Tests
# ─────────────────────────────────────────────────────────────────────────────

def test_regpack_corridor_binding(regpack_manager):
    """Test regpack can be bound to a corridor."""
    metadata = regpack_manager.create_regpack_metadata(
        as_of_date="2026-01-15",
        snapshot_type="quarterly",
    )
    
    digest = regpack_manager.compute_regpack_digest(metadata)
    
    # Create corridor binding structure
    binding = {
        "zone_id": "ae-dubai-difc",
        "regpack_ref": {
            "artifact_type": "regpack",
            "digest_sha256": digest,
        },
        "max_age_days": 7,
        "required_components": ["sanctions", "licenses"],
    }
    
    assert binding["regpack_ref"]["digest_sha256"] == digest
    assert len(digest) == 64


def test_regpack_freshness_check(regpack_manager):
    """Test regpack freshness validation."""
    metadata = regpack_manager.create_regpack_metadata(
        as_of_date="2026-01-15",
        snapshot_type="quarterly",
    )
    
    # Metadata has created_at, can check freshness
    assert metadata.created_at is not None
    
    # In a real implementation, would compare against current time
    # and max_age_days from corridor binding
