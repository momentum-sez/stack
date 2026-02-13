"""Comprehensive tests for the licensepack module.

Tests cover:
- License status and domain enums
- LicenseCondition, LicensePermission, LicenseRestriction dataclasses
- LicenseHolder and License dataclasses
- LicensePack operations (add, verify, compute_digest)
- Delta computation between packs
- Compliance state evaluation
- Edge cases and error handling
"""

import json
import pytest
import tempfile
from decimal import Decimal
from datetime import datetime, timezone, timedelta
from pathlib import Path

from tools.licensepack import (
    LicenseStatus,
    LicenseDomain,
    ComplianceState,
    LicenseCondition,
    LicensePermission,
    LicenseRestriction,
    LicenseHolder,
    License,
    LicenseType,
    Regulator,
    LicensepackMetadata,
    LicensePack,
    compute_licensepack_digest,
    jcs_canonicalize,
)


class TestEnums:
    """Test enum classes."""

    def test_license_status_values(self):
        """All expected status values exist."""
        statuses = {s.value for s in LicenseStatus}
        assert "active" in statuses
        assert "suspended" in statuses
        assert "revoked" in statuses
        assert "expired" in statuses
        assert "pending" in statuses
        assert "surrendered" in statuses

    def test_license_domain_values(self):
        """All expected domain values exist."""
        domains = {d.value for d in LicenseDomain}
        assert "financial" in domains
        assert "corporate" in domains
        assert "professional" in domains
        assert "trade" in domains
        assert "insurance" in domains
        assert "mixed" in domains

    def test_compliance_state_values(self):
        """All compliance states exist."""
        states = {s.value for s in ComplianceState}
        assert "COMPLIANT" in states
        assert "NON_COMPLIANT" in states
        assert "PENDING" in states
        assert "SUSPENDED" in states
        assert "UNKNOWN" in states


class TestLicenseCondition:
    """Test LicenseCondition dataclass."""

    def test_basic_construction(self):
        """Basic condition construction."""
        cond = LicenseCondition(
            condition_id="cap-1",
            condition_type="capital",
            description="Minimum capital requirement",
            metric="regulatory_capital",
            threshold="1000000",
            currency="USD",
            operator=">=",
        )
        assert cond.condition_id == "cap-1"
        assert cond.threshold == "1000000"

    def test_is_active_no_expiry(self):
        """Condition without expiry is active."""
        cond = LicenseCondition(
            condition_id="c1",
            condition_type="operational",
            description="Test",
            status="active",
        )
        assert cond.is_active() is True

    def test_is_active_expired(self):
        """Expired condition is not active."""
        cond = LicenseCondition(
            condition_id="c1",
            condition_type="operational",
            description="Test",
            status="active",
            expiry_date="2020-01-01",
        )
        assert cond.is_active() is False

    def test_is_active_inactive_status(self):
        """Inactive status means not active."""
        cond = LicenseCondition(
            condition_id="c1",
            condition_type="operational",
            description="Test",
            status="waived",
        )
        assert cond.is_active() is False

    def test_to_dict_minimal(self):
        """to_dict with minimal fields."""
        cond = LicenseCondition(
            condition_id="c1",
            condition_type="capital",
            description="Test condition",
        )
        d = cond.to_dict()
        assert d["condition_id"] == "c1"
        assert d["condition_type"] == "capital"
        assert "metric" not in d  # None values excluded

    def test_to_dict_full(self):
        """to_dict with all fields."""
        cond = LicenseCondition(
            condition_id="cap-1",
            condition_type="capital",
            description="Minimum capital",
            metric="regulatory_capital",
            threshold="1000000",
            currency="USD",
            operator=">=",
            frequency="continuous",
            reporting_frequency="quarterly",
            effective_date="2024-01-01",
            expiry_date="2025-12-31",
        )
        d = cond.to_dict()
        assert d["threshold"] == "1000000"
        assert d["reporting_frequency"] == "quarterly"


class TestLicensePermission:
    """Test LicensePermission dataclass."""

    def test_basic_construction(self):
        """Basic permission construction."""
        perm = LicensePermission(
            permission_id="p1",
            activity="deposit_taking",
            scope={"client_types": ["retail", "professional"]},
        )
        assert perm.permission_id == "p1"
        assert perm.activity == "deposit_taking"

    def test_permits_activity_match(self):
        """Activity match check."""
        perm = LicensePermission(
            permission_id="p1",
            activity="custody",
            status="active",
        )
        assert perm.permits_activity("custody") is True
        assert perm.permits_activity("trading") is False

    def test_permits_activity_inactive(self):
        """Inactive permission doesn't permit."""
        perm = LicensePermission(
            permission_id="p1",
            activity="custody",
            status="suspended",
        )
        assert perm.permits_activity("custody") is False

    def test_within_limits_no_limits(self):
        """No limits means all amounts allowed."""
        perm = LicensePermission(
            permission_id="p1",
            activity="transfer",
            limits={},
        )
        assert perm.within_limits(Decimal("1000000000"), "USD") is True

    def test_within_limits_under(self):
        """Amount under limit is allowed."""
        perm = LicensePermission(
            permission_id="p1",
            activity="transfer",
            limits={
                "single_transfer_max": "100000",
                "currency": "USD",
            },
        )
        assert perm.within_limits(Decimal("50000"), "USD") is True

    def test_within_limits_over(self):
        """Amount over limit is blocked."""
        perm = LicensePermission(
            permission_id="p1",
            activity="transfer",
            limits={
                "single_transfer_max": "100000",
                "currency": "USD",
            },
        )
        assert perm.within_limits(Decimal("150000"), "USD") is False


class TestLicenseRestriction:
    """Test LicenseRestriction dataclass."""

    def test_basic_construction(self):
        """Basic restriction construction."""
        rest = LicenseRestriction(
            restriction_id="r1",
            restriction_type="geographic",
            description="No US operations",
            blocked_jurisdictions=["us", "us-*"],
        )
        assert rest.restriction_id == "r1"
        assert "us" in rest.blocked_jurisdictions

    def test_blocks_activity(self):
        """Activity blocking check."""
        rest = LicenseRestriction(
            restriction_id="r1",
            restriction_type="activity",
            description="No CFD trading",
            blocked_activities=["cfd_trading", "binary_options"],
        )
        assert rest.blocks_activity("cfd_trading") is True
        assert rest.blocks_activity("spot_trading") is False

    def test_blocks_jurisdiction_explicit(self):
        """Explicit jurisdiction blocking."""
        rest = LicenseRestriction(
            restriction_id="r1",
            restriction_type="geographic",
            description="No US",
            blocked_jurisdictions=["us"],
        )
        assert rest.blocks_jurisdiction("us") is True
        assert rest.blocks_jurisdiction("uk") is False

    def test_blocks_jurisdiction_wildcard_allow(self):
        """Wildcard block with allow list."""
        rest = LicenseRestriction(
            restriction_id="r1",
            restriction_type="geographic",
            description="Only EU",
            blocked_jurisdictions=["*"],
            allowed_jurisdictions=["de", "fr", "nl"],
        )
        assert rest.blocks_jurisdiction("us") is True
        assert rest.blocks_jurisdiction("de") is False
        assert rest.blocks_jurisdiction("fr") is False
        assert rest.blocks_jurisdiction("jp") is True


class TestLicenseHolder:
    """Test LicenseHolder dataclass."""

    def test_basic_construction(self):
        """Basic holder construction."""
        holder = LicenseHolder(
            holder_id="H001",
            entity_type="company",
            legal_name="Acme Financial Services Ltd",
            registration_number="12345678",
            jurisdiction_of_incorporation="ae-dubai-difc",
        )
        assert holder.holder_id == "H001"
        assert holder.legal_name == "Acme Financial Services Ltd"

    def test_to_dict(self):
        """to_dict serialization."""
        holder = LicenseHolder(
            holder_id="H001",
            entity_type="company",
            legal_name="Test Corp",
            registration_number="REG123",
            jurisdiction_of_incorporation="sg-mas",
            did="did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
        )
        d = holder.to_dict()
        assert d["holder_id"] == "H001"
        assert d["did"] == "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"


class TestLicense:
    """Test License dataclass."""

    def test_basic_construction(self):
        """Basic license construction."""
        lic = License(
            license_id="LIC001",
            license_type_id="banking.category4",
            license_number="BNK-001",
            holder_id="H001",
            holder_legal_name="Acme Financial Services Ltd",
            holder_did="did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            status=LicenseStatus.ACTIVE,
            issued_date="2023-01-01",
            regulator_id="ae-dubai-difc-dfsa",
        )
        assert lic.license_id == "LIC001"
        assert lic.status == LicenseStatus.ACTIVE

    def test_is_active_status(self):
        """Active status check."""
        lic = License(
            license_id="L1",
            license_type_id="banking.cat4",
            license_number=None,
            holder_id="H1",
            holder_legal_name="Test Corp",
            status=LicenseStatus.ACTIVE,
            issued_date="2023-01-01",
            regulator_id="reg1",
        )
        assert lic.is_active() is True

        lic.status = LicenseStatus.SUSPENDED
        assert lic.is_active() is False

    def test_is_expired(self):
        """Expiry date check."""
        lic = License(
            license_id="L1",
            license_type_id="banking.cat4",
            license_number=None,
            holder_id="H1",
            holder_legal_name="Test Corp",
            status=LicenseStatus.ACTIVE,
            issued_date="2020-01-01",
            expiry_date="2020-12-31",  # Expired
            regulator_id="reg1",
        )
        assert lic.is_expired() is True

    def test_has_permission(self):
        """Permission check."""
        lic = License(
            license_id="L1",
            license_type_id="banking.cat4",
            license_number=None,
            holder_id="H1",
            holder_legal_name="Test Corp",
            status=LicenseStatus.ACTIVE,
            issued_date="2023-01-01",
            regulator_id="reg1",
            permissions=[
                LicensePermission(
                    permission_id="p1",
                    activity="deposit_taking",
                ),
                LicensePermission(
                    permission_id="p2",
                    activity="lending",
                ),
            ],
        )
        assert lic.permits_activity("deposit_taking") is True
        assert lic.permits_activity("trading") is False


class TestLicenseType:
    """Test LicenseType dataclass."""

    def test_basic_construction(self):
        """Basic license type construction."""
        lt = LicenseType(
            license_type_id="banking.category4",
            name="Category 4 Banking License",
            description="Category 4 banking license for DIFC",
            regulator_id="ae-dubai-difc-dfsa",
            category="banking",
            permitted_activities=["deposit_taking", "lending", "fx"],
        )
        assert lt.license_type_id == "banking.category4"
        assert lt.category == "banking"


class TestRegulator:
    """Test Regulator dataclass."""

    def test_basic_construction(self):
        """Basic regulator construction."""
        reg = Regulator(
            regulator_id="ae-dubai-difc-dfsa",
            name="Dubai Financial Services Authority",
            jurisdiction_id="ae-dubai-difc",
            registry_url="https://www.dfsa.ae",
        )
        assert reg.regulator_id == "ae-dubai-difc-dfsa"
        assert "dfsa" in reg.registry_url.lower()


class TestLicensepackMetadata:
    """Test LicensepackMetadata dataclass."""

    def test_basic_construction(self):
        """Basic metadata construction."""
        reg = Regulator(
            regulator_id="dfsa",
            name="Dubai Financial Services Authority",
            jurisdiction_id="ae-dubai-difc",
        )
        meta = LicensepackMetadata(
            licensepack_id="licensepack:ae-dubai-difc:financial:2024-01-15T10:30:00Z",
            jurisdiction_id="ae-dubai-difc",
            domain=LicenseDomain.FINANCIAL,
            as_of_date="2024-01-15",
            snapshot_timestamp="2024-01-15T10:30:00Z",
            snapshot_type="on_demand",
            regulator=reg,
            license="CC0-1.0",
        )
        assert meta.jurisdiction_id == "ae-dubai-difc"
        assert meta.domain == LicenseDomain.FINANCIAL


class TestLicensePack:
    """Test LicensePack class."""

    @pytest.fixture
    def sample_pack(self):
        """Create a sample license pack for testing."""
        reg = Regulator(
            regulator_id="dfsa",
            name="Dubai Financial Services Authority",
            jurisdiction_id="ae-dubai-difc",
            registry_url="https://www.dfsa.ae",
        )
        meta = LicensepackMetadata(
            licensepack_id="licensepack:ae-dubai-difc:financial:2024-01-15T10:30:00Z",
            jurisdiction_id="ae-dubai-difc",
            domain=LicenseDomain.FINANCIAL,
            as_of_date="2024-01-15",
            snapshot_timestamp="2024-01-15T10:30:00Z",
            snapshot_type="on_demand",
            regulator=reg,
            license="CC0-1.0",
        )
        pack = LicensePack(metadata=meta)

        # Add a license type
        pack.add_license_type(LicenseType(
            license_type_id="banking.cat4",
            name="Category 4 Banking",
            description="Category 4 banking license",
            regulator_id="dfsa",
            category="banking",
            permitted_activities=["deposit_taking", "lending"],
        ))

        # Add a license
        pack.add_license(License(
            license_id="DFSA-LIC-001",
            license_type_id="banking.cat4",
            license_number=None,
            holder_id="H001",
            holder_legal_name="Acme Financial Services Ltd",
            holder_did="did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK",
            status=LicenseStatus.ACTIVE,
            issued_date="2023-01-01",
            expiry_date="2028-12-31",
            regulator_id="dfsa",
            permissions=[
                LicensePermission(
                    permission_id="p1",
                    activity="deposit_taking",
                    limits={"single_deposit_taking_max": "10000000", "currency": "USD"},
                ),
            ],
        ))

        return pack

    def test_add_license(self, sample_pack):
        """Adding licenses works."""
        assert len(sample_pack.licenses) == 1
        assert "DFSA-LIC-001" in sample_pack.licenses

    def test_get_license(self, sample_pack):
        """Getting licenses works."""
        lic = sample_pack.get_license("DFSA-LIC-001")
        assert lic is not None
        assert lic.license_id == "DFSA-LIC-001"

    def test_get_license_missing(self, sample_pack):
        """Missing license returns None."""
        lic = sample_pack.get_license("nonexistent")
        assert lic is None

    def test_get_licenses_by_holder(self, sample_pack):
        """Getting licenses by holder DID."""
        did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
        licenses = sample_pack.get_license_by_holder_did(did)
        assert len(licenses) == 1
        assert licenses[0].license_id == "DFSA-LIC-001"

    def test_verify_license_compliant(self, sample_pack):
        """Verify license returns compliant for valid holder/activity."""
        did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
        is_valid, state, lic = sample_pack.verify_license(
            holder_did=did,
            activity="deposit_taking",
        )
        assert is_valid is True
        assert state == ComplianceState.COMPLIANT
        assert lic is not None

    def test_verify_license_no_permission(self, sample_pack):
        """Verify license fails for unpermitted activity."""
        did = "did:key:z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
        is_valid, state, lic = sample_pack.verify_license(
            holder_did=did,
            activity="trading",  # Not permitted
        )
        assert is_valid is False
        assert state == ComplianceState.NON_COMPLIANT

    def test_verify_license_unknown_holder(self, sample_pack):
        """Verify license fails for unknown holder."""
        is_valid, state, lic = sample_pack.verify_license(
            holder_did="did:key:unknown",
            activity="deposit_taking",
        )
        assert is_valid is False
        assert state == ComplianceState.NON_COMPLIANT

    def test_compute_digest_deterministic(self, sample_pack):
        """Digest is deterministic."""
        d1 = sample_pack.compute_digest()
        d2 = sample_pack.compute_digest()
        assert d1 == d2
        assert len(d1) == 64  # SHA256 hex

    def test_compute_digest_changes_on_update(self, sample_pack):
        """Digest changes when pack is updated."""
        d1 = sample_pack.compute_digest()

        # Add another license
        sample_pack.add_license(License(
            license_id="DFSA-LIC-002",
            license_type_id="banking.cat4",
            license_number=None,
            holder_id="H002",
            holder_legal_name="Beta Corp",
            status=LicenseStatus.ACTIVE,
            issued_date="2024-01-01",
            regulator_id="dfsa",
        ))

        d2 = sample_pack.compute_digest()
        assert d1 != d2

    def test_pack_counts(self, sample_pack):
        """Pack collection counts."""
        assert len(sample_pack.licenses) == 1
        assert len(sample_pack.license_types) == 1
        active = sample_pack.get_active_licenses()
        assert len(active) == 1

    def test_pack_metadata_access(self, sample_pack):
        """Pack metadata access via properties."""
        assert sample_pack.jurisdiction_id == "ae-dubai-difc"
        assert sample_pack.domain == LicenseDomain.FINANCIAL
        meta_dict = sample_pack.metadata.to_dict()
        assert meta_dict["jurisdiction_id"] == "ae-dubai-difc"


class TestJCSCanonicalize:
    """Test JCS canonicalization function."""

    def test_simple_object(self):
        """Simple object canonicalization."""
        obj = {"b": 2, "a": 1}
        result = jcs_canonicalize(obj)
        assert result == b'{"a":1,"b":2}'

    def test_nested_object(self):
        """Nested object canonicalization."""
        obj = {"outer": {"b": 2, "a": 1}, "z": [3, 1, 2]}
        result = jcs_canonicalize(obj)
        # Keys sorted, lists preserve order
        assert b'"outer":{"a":1,"b":2}' in result
        assert b'"z":[3,1,2]' in result

    def test_unicode(self):
        """Unicode handling."""
        obj = {"name": "日本語"}
        result = jcs_canonicalize(obj)
        assert "日本語".encode("utf-8") in result


class TestComputeLicensepackDigest:
    """Test compute_licensepack_digest function."""

    def _make_pack(self, jurisdiction_id: str) -> LicensePack:
        """Helper to create a minimal LicensePack."""
        reg = Regulator(
            regulator_id="test-reg",
            name="Test Regulator",
            jurisdiction_id=jurisdiction_id,
        )
        meta = LicensepackMetadata(
            licensepack_id=f"licensepack:{jurisdiction_id}:financial:2024-01-01T00:00:00Z",
            jurisdiction_id=jurisdiction_id,
            domain=LicenseDomain.FINANCIAL,
            as_of_date="2024-01-01",
            snapshot_timestamp="2024-01-01T00:00:00Z",
            snapshot_type="on_demand",
            regulator=reg,
            license="CC0-1.0",
        )
        return LicensePack(metadata=meta)

    def test_deterministic(self):
        """Digest computation is deterministic."""
        pack = self._make_pack("test")
        d1 = compute_licensepack_digest(pack)
        d2 = compute_licensepack_digest(pack)
        assert d1 == d2

    def test_different_data_different_digest(self):
        """Different data produces different digest."""
        pack1 = self._make_pack("test1")
        pack2 = self._make_pack("test2")
        d1 = compute_licensepack_digest(pack1)
        d2 = compute_licensepack_digest(pack2)
        assert d1 != d2


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_pack(self):
        """Empty pack operations."""
        reg = Regulator(
            regulator_id="test-reg",
            name="Test Regulator",
            jurisdiction_id="test",
        )
        meta = LicensepackMetadata(
            licensepack_id="licensepack:test:financial:2024-01-01T00:00:00Z",
            jurisdiction_id="test",
            domain=LicenseDomain.FINANCIAL,
            as_of_date="2024-01-01",
            snapshot_timestamp="2024-01-01T00:00:00Z",
            snapshot_type="on_demand",
            regulator=reg,
            license="CC0-1.0",
        )
        pack = LicensePack(metadata=meta)

        assert len(pack.licenses) == 0

        # Digest still works
        digest = pack.compute_digest()
        assert len(digest) == 64

    def test_license_with_all_optional_fields(self):
        """License with all optional fields."""
        lic = License(
            license_id="L1",
            license_type_id="type1",
            license_number="LN-001",
            holder_id="H1",
            holder_legal_name="Test Corp",
            holder_did="did:key:test",
            status=LicenseStatus.ACTIVE,
            issued_date="2024-01-01",
            effective_date="2024-01-15",
            expiry_date="2029-12-31",
            regulator_id="reg1",
            permissions=[
                LicensePermission(
                    permission_id="p1",
                    activity="test",
                    scope={"region": "global"},
                    limits={"max": "1000000"},
                ),
            ],
            conditions=[
                LicenseCondition(
                    condition_id="c1",
                    condition_type="capital",
                    description="Min capital",
                    threshold="500000",
                ),
            ],
            restrictions=[
                LicenseRestriction(
                    restriction_id="r1",
                    restriction_type="geographic",
                    description="No US",
                    blocked_jurisdictions=["us"],
                ),
            ],
        )

        d = lic.to_dict()
        assert d["license_id"] == "L1"
        assert len(lic.permissions) == 1
        assert len(lic.conditions) == 1
        assert len(lic.restrictions) == 1

    def test_permission_with_complex_scope(self):
        """Permission with complex scope definition."""
        perm = LicensePermission(
            permission_id="p1",
            activity="advisory",
            scope={
                "client_types": ["retail", "professional", "institutional"],
                "product_categories": ["equity", "fixed_income", "derivatives"],
                "geographic_scope": ["global"],
                "exceptions": {
                    "excluded_products": ["structured_products"],
                    "excluded_jurisdictions": ["us"],
                },
            },
            limits={
                "max_aum_usd": "10000000000",
                "max_clients": 1000,
            },
        )
        d = perm.to_dict()
        assert "exceptions" in d["scope"]


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
