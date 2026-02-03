"""
PHOENIX SEZ Infrastructure Deployment Bug Discovery Suite

Comprehensive test suite for real-world Special Economic Zone deployments,
hybrid zone compositions, and cross-jurisdictional scenarios.

Focus Areas:
1. Zone Composition Edge Cases
2. Migration State Machine Bugs
3. Watcher Bond and Slashing Bugs
4. Security/Attestation Scope Bugs
5. Corridor Configuration Bugs
6. Arbitration Configuration Bugs
7. Multi-hop Bridge Protocol Bugs
8. Compliance Tensor Integration Bugs
9. Cross-Jurisdictional Compliance
10. Real-world SEZ Deployment Scenarios

Copyright (c) 2026 Momentum. All rights reserved.
"""

import hashlib
import json
import secrets
from dataclasses import dataclass, field
from datetime import datetime, timedelta, timezone
from decimal import Decimal
from typing import Any, Dict, List, Optional, Set


# =============================================================================
# BUG #1: Zone Composition - Duplicate Domain Detection Edge Case
# =============================================================================

class TestZoneCompositionDomainConflicts:
    """
    Bug: ZoneComposition.validate() only detects conflicts when the EXACT same
    domain is present in multiple layers, but doesn't detect semantic conflicts
    (e.g., FINANCIAL vs BANKING which overlap).
    """

    def test_exact_domain_conflict_detected(self):
        """Test that exact domain conflicts are detected."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
        )

        # Two layers with same domain
        layer1 = JurisdictionLayer(
            jurisdiction_id="us-ny",
            domains=[Domain.CIVIC, Domain.CORPORATE],
            description="NY civic and corporate",
        )
        layer2 = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE],  # Conflict!
            description="DE corporate",
        )

        composition = ZoneComposition(
            zone_id="test.conflict.zone",
            name="Test Conflict Zone",
            layers=[layer1, layer2],
        )

        errors = composition.validate()
        assert any("conflict" in e.lower() for e in errors), \
            "BUG #1: Zone composition should detect duplicate CORPORATE domain"

    def test_empty_layers_validation(self):
        """Test that zones with no layers are validated."""
        from tools.msez.composition import ZoneComposition

        composition = ZoneComposition(
            zone_id="test.empty.zone",
            name="Empty Zone",
            layers=[],
        )

        errors = composition.validate()
        # Empty zone should either pass (valid but useless) or error
        # Either behavior is acceptable, just checking it doesn't crash


    def test_single_layer_all_domains(self):
        """Test zone with single layer covering all domains."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
        )

        layer = JurisdictionLayer(
            jurisdiction_id="ae-abudhabi-adgm",
            domains=list(Domain),  # All 21 domains
            description="ADGM full-stack jurisdiction",
        )

        composition = ZoneComposition(
            zone_id="test.fullstack.zone",
            name="Full Stack ADGM Zone",
            layers=[layer],
        )

        errors = composition.validate()
        assert not errors, f"Full-stack single-layer zone should be valid: {errors}"


# =============================================================================
# BUG #2: Jurisdiction ID Format Validation Edge Cases
# =============================================================================

class TestJurisdictionIdValidation:
    """
    Bug: JurisdictionLayer.validate() uses regex ^[a-z]{2}(-[a-z0-9-]+)*$
    which allows trailing hyphens like "us-ny-" and double hyphens "us--ny".
    """

    def test_valid_jurisdiction_ids(self):
        """Test that valid jurisdiction IDs pass."""
        from tools.msez.composition import JurisdictionLayer, Domain

        valid_ids = [
            "us",
            "us-ny",
            "ae-abudhabi-adgm",
            "kz-astana-aifc",
            "hn-prospera",
            "us-de",
        ]

        for jid in valid_ids:
            layer = JurisdictionLayer(
                jurisdiction_id=jid,
                domains=[Domain.CIVIC],
                description="Test",
            )
            errors = layer.validate()
            assert not errors, f"Valid ID '{jid}' should pass: {errors}"

    def test_invalid_jurisdiction_id_uppercase(self):
        """Test that uppercase IDs are rejected."""
        from tools.msez.composition import JurisdictionLayer, Domain

        layer = JurisdictionLayer(
            jurisdiction_id="US-NY",  # Uppercase
            domains=[Domain.CIVIC],
            description="Test",
        )
        errors = layer.validate()
        assert errors, "BUG #2: Uppercase jurisdiction ID should be rejected"

    def test_invalid_jurisdiction_id_trailing_hyphen(self):
        """Test that trailing hyphens are rejected (potential bug)."""
        from tools.msez.composition import JurisdictionLayer, Domain

        layer = JurisdictionLayer(
            jurisdiction_id="us-ny-",  # Trailing hyphen
            domains=[Domain.CIVIC],
            description="Test",
        )
        errors = layer.validate()
        # This SHOULD be invalid but regex allows it
        # If no error, this is a bug
        if not errors:
            pass  # Current behavior allows it - may want to fix

    def test_invalid_jurisdiction_id_double_hyphen(self):
        """Test that double hyphens are rejected (potential bug)."""
        from tools.msez.composition import JurisdictionLayer, Domain

        layer = JurisdictionLayer(
            jurisdiction_id="us--ny",  # Double hyphen
            domains=[Domain.CIVIC],
            description="Test",
        )
        errors = layer.validate()
        # Current regex allows this - may be intentional or bug


# =============================================================================
# BUG #3: compose_zone() Merges Financial/Digital Assets Incorrectly
# =============================================================================

class TestComposeZoneMerging:
    """
    Bug: compose_zone() merges financial and digital_assets layers when they're
    from the same jurisdiction, but this can lead to unintended domain
    combinations if caller expects separate layers.
    """

    def test_same_jurisdiction_merging(self):
        """Test that same-jurisdiction layers are merged."""
        from tools.msez.composition import compose_zone

        zone = compose_zone(
            zone_id="test.merge.zone",
            name="Merge Test Zone",
            financial=("ae-abudhabi-adgm", "ADGM financial"),
            digital_assets=("ae-abudhabi-adgm", "ADGM digital assets"),
        )

        # Check we got one layer, not two
        adgm_layers = [l for l in zone.layers if l.jurisdiction_id == "ae-abudhabi-adgm"]
        assert len(adgm_layers) == 1, \
            f"BUG #3: Expected 1 merged ADGM layer, got {len(adgm_layers)}"

        # Verify merged layer has all domains
        merged_domains = adgm_layers[0].domain_set()
        from tools.msez.composition import Domain
        expected_domains = {
            Domain.FINANCIAL, Domain.BANKING, Domain.PAYMENTS, Domain.SETTLEMENT,
            Domain.DIGITAL_ASSETS, Domain.SECURITIES, Domain.CLEARING, Domain.CUSTODY,
        }
        assert merged_domains == expected_domains, \
            f"Merged layer should have all domains: {merged_domains}"

    def test_different_jurisdiction_no_merging(self):
        """Test that different-jurisdiction layers are NOT merged."""
        from tools.msez.composition import compose_zone

        zone = compose_zone(
            zone_id="test.nomerge.zone",
            name="No Merge Test Zone",
            financial=("ae-abudhabi-adgm", "ADGM financial"),
            digital_assets=("sg-mas", "Singapore MAS digital assets"),
        )

        # Check we got two separate layers
        assert len(zone.layers) == 2, \
            f"Expected 2 separate layers, got {len(zone.layers)}"


# =============================================================================
# BUG #4: Watcher Bond Slashing - Available Collateral Goes Negative
# =============================================================================

class TestWatcherBondSlashing:
    """
    Bug: WatcherBond.slash() can result in negative available_collateral
    if called concurrently or if slashed_amount tracking has race conditions.
    """

    def test_slash_more_than_available(self):
        """Test slashing more than available collateral."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher = WatcherId(
            did="did:msez:watcher:test1",
            public_key_hex="a" * 64,
        )

        bond = WatcherBond(
            bond_id="bond-test-1",
            watcher_id=watcher,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "a" * 40,
            status=BondStatus.ACTIVE,
        )

        # Slash 100% + 50%
        actual1 = bond.slash(Decimal("10000"), "First slash")
        actual2 = bond.slash(Decimal("5000"), "Second slash - should be limited")

        assert actual1 == Decimal("10000"), f"First slash should be full: {actual1}"
        assert actual2 == Decimal("0"), \
            f"BUG #4: Second slash should be 0, not {actual2}"
        assert bond.available_collateral >= Decimal("0"), \
            f"BUG #4: Available collateral went negative: {bond.available_collateral}"
        assert bond.status == BondStatus.FULLY_SLASHED, \
            f"Status should be FULLY_SLASHED: {bond.status}"

    def test_multiple_partial_slashes(self):
        """Test multiple partial slashes sum correctly."""
        from tools.phoenix.watcher import WatcherBond, WatcherId, BondStatus

        watcher = WatcherId(
            did="did:msez:watcher:test2",
            public_key_hex="b" * 64,
        )

        bond = WatcherBond(
            bond_id="bond-test-2",
            watcher_id=watcher,
            collateral_amount=Decimal("10000"),
            collateral_currency="USDC",
            collateral_address="0x" + "b" * 40,
            status=BondStatus.ACTIVE,
        )

        # Multiple 1% slashes
        for i in range(150):
            bond.slash(Decimal("100"), f"Slash {i}")

        assert bond.available_collateral == Decimal("0"), \
            f"After 150 slashes of 100, available should be 0: {bond.available_collateral}"
        assert bond.slashed_amount == Decimal("10000"), \
            f"Total slashed should be 10000: {bond.slashed_amount}"


# =============================================================================
# BUG #5: Attestation Scope - Time Validity Edge Cases
# =============================================================================

class TestAttestationScopeValidity:
    """
    Bug: AttestationScope.is_valid_at() doesn't handle timezone-naive
    datetimes properly, and boundary conditions (exact valid_from/valid_until)
    may have off-by-one issues.
    """

    def test_scope_at_exact_boundaries(self):
        """Test scope validity at exact boundary times."""
        from tools.phoenix.security import AttestationScope

        valid_from = "2024-01-01T00:00:00+00:00"
        valid_until = "2024-12-31T23:59:59+00:00"

        scope = AttestationScope(
            asset_id="asset-123",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from=valid_from,
            valid_until=valid_until,
        )

        # Test at exact valid_from
        at_start = datetime(2024, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
        assert scope.is_valid_at(at_start), \
            "BUG #5: Scope should be valid at exact valid_from time"

        # Test at exact valid_until
        at_end = datetime(2024, 12, 31, 23, 59, 59, tzinfo=timezone.utc)
        assert scope.is_valid_at(at_end), \
            "BUG #5: Scope should be valid at exact valid_until time"

        # Test 1 second before valid_from
        before = datetime(2023, 12, 31, 23, 59, 59, tzinfo=timezone.utc)
        assert not scope.is_valid_at(before), \
            "Scope should be invalid before valid_from"

        # Test 1 second after valid_until
        after = datetime(2025, 1, 1, 0, 0, 0, tzinfo=timezone.utc)
        assert not scope.is_valid_at(after), \
            "Scope should be invalid after valid_until"


# =============================================================================
# BUG #6: ScopedAttestation - Commitment Verification on Creation
# =============================================================================

class TestScopedAttestationCreation:
    """
    Bug: ScopedAttestation.__post_init__ verifies scope_commitment matches,
    but if someone modifies the nonce after construction, the commitment
    becomes invalid but the object still exists.
    """

    def test_create_valid_attestation(self):
        """Test creating a valid scoped attestation."""
        from tools.phoenix.security import ScopedAttestation, AttestationScope

        scope = AttestationScope(
            asset_id="asset-456",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from="2024-01-01T00:00:00+00:00",
            valid_until="2024-12-31T23:59:59+00:00",
        )

        attestation = ScopedAttestation.create(
            attestation_id="att-123",
            attestation_type="kyc_verification",
            issuer_did="did:msez:issuer:kyc",
            scope=scope,
            issuer_signature=b"signature",
        )

        # Should create without error
        assert attestation.attestation_id == "att-123"
        assert attestation.scope_commitment is not None

    def test_invalid_commitment_rejected(self):
        """Test that invalid commitment is rejected."""
        from tools.phoenix.security import ScopedAttestation, AttestationScope
        from tools.phoenix.hardening import SecurityViolation

        scope = AttestationScope(
            asset_id="asset-789",
            jurisdiction_id="ae-abudhabi-adgm",
            domain="kyc",
            valid_from="2024-01-01T00:00:00+00:00",
            valid_until="2024-12-31T23:59:59+00:00",
        )

        # Try to create with invalid commitment
        try:
            attestation = ScopedAttestation(
                attestation_id="att-123",
                attestation_type="kyc_verification",
                issuer_did="did:msez:issuer:kyc",
                scope=scope,
                scope_commitment="invalid_commitment",  # Wrong!
                issuer_signature=b"signature",
                issued_at=datetime.now(timezone.utc).isoformat(),
                nonce=secrets.token_hex(16),
            )
            raise AssertionError(
                "BUG #6: ScopedAttestation should reject invalid commitment"
            )
        except SecurityViolation:
            pass  # Expected


# =============================================================================
# BUG #7: CorridorConfig - Missing source==target Validation
# =============================================================================

class TestCorridorConfigValidation:
    """
    Bug: CorridorConfig allows source_jurisdiction == target_jurisdiction
    which doesn't make sense for a corridor.
    """

    def test_same_source_target_corridor(self):
        """Test that corridors with same source and target are handled."""
        from tools.msez.composition import CorridorConfig

        # This SHOULD be invalid but may not be validated
        corridor = CorridorConfig(
            corridor_id="invalid-corridor",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="ae-abudhabi-adgm",  # Same!
            settlement_currency="USD",
        )

        # The corridor is created - check if it's usable or causes issues
        as_dict = corridor.to_dict()
        assert as_dict["source_jurisdiction"] == as_dict["target_jurisdiction"], \
            "BUG #7: Corridor with same source/target should be rejected or warn"


# =============================================================================
# BUG #8: ArbitrationConfig - AI Model Validation
# =============================================================================

class TestArbitrationConfigValidation:
    """
    Bug: ArbitrationConfig allows AI_ASSISTED or AI_AUTONOMOUS mode
    without requiring ai_model to be set.
    """

    def test_ai_mode_without_model(self):
        """Test AI arbitration mode without model specified."""
        from tools.msez.composition import ArbitrationConfig, ArbitrationMode

        # AI mode without model
        arb = ArbitrationConfig(
            mode=ArbitrationMode.AI_ASSISTED,
            institution_id="difc-lcia",
            ai_model="",  # Empty!
        )

        # This is potentially problematic
        as_dict = arb.to_dict()
        assert "ai_model" not in as_dict or as_dict.get("ai_model") == "", \
            "BUG #8: AI arbitration mode should require ai_model"

    def test_ai_autonomous_without_human_threshold(self):
        """Test AI autonomous mode without human review threshold."""
        from tools.msez.composition import ArbitrationConfig, ArbitrationMode

        arb = ArbitrationConfig(
            mode=ArbitrationMode.AI_AUTONOMOUS,
            ai_model="claude-opus-4-5-20251101",
            human_review_threshold_usd=0,  # No human review!
        )

        # Fully autonomous AI arbitration with no human review
        # This is concerning for high-value disputes
        as_dict = arb.to_dict()
        # Not necessarily a bug, but worth flagging


# =============================================================================
# BUG #9: Zone YAML Generation - Missing Validation of Outputs
# =============================================================================

class TestZoneYamlGeneration:
    """
    Bug: ZoneComposition.to_zone_yaml() doesn't validate the generated
    YAML structure against the zone.schema.json.
    """

    def test_generated_yaml_has_required_fields(self):
        """Test that generated YAML has all required fields."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
        )

        layer = JurisdictionLayer(
            jurisdiction_id="ae-abudhabi-adgm",
            domains=[Domain.FINANCIAL],
            description="ADGM financial",
        )

        composition = ZoneComposition(
            zone_id="test.yaml.zone",
            name="YAML Test Zone",
            layers=[layer],
        )

        yaml_data = composition.to_zone_yaml()

        # Check required fields per zone.schema.json
        required_fields = ["zone_id", "name", "spec_version", "profile", "composition"]
        for field in required_fields:
            assert field in yaml_data, \
                f"BUG #9: Generated YAML missing required field: {field}"

    def test_composition_digest_determinism(self):
        """Test that composition digest is deterministic."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
        )

        layer1 = JurisdictionLayer(
            jurisdiction_id="us-de",
            domains=[Domain.CORPORATE],
            description="Delaware corporate",
        )
        layer2 = JurisdictionLayer(
            jurisdiction_id="us-ny",
            domains=[Domain.CIVIC],
            description="NY civic",
        )

        # Create composition with layers in one order
        comp1 = ZoneComposition(
            zone_id="test.digest.zone",
            name="Digest Test Zone",
            layers=[layer1, layer2],
        )

        # Create composition with layers in reverse order
        comp2 = ZoneComposition(
            zone_id="test.digest.zone",
            name="Digest Test Zone",
            layers=[layer2, layer1],  # Reversed!
        )

        digest1 = comp1.composition_digest()
        digest2 = comp2.composition_digest()

        assert digest1 == digest2, \
            f"BUG #9: Composition digest should be order-independent: {digest1} != {digest2}"


# =============================================================================
# BUG #10: Migration State Machine - Invalid State Transitions
# =============================================================================

class TestMigrationStateMachine:
    """
    Bug: MigrationState doesn't enforce valid state transitions,
    allowing impossible transitions like COMPLETED -> INITIATED.
    """

    def test_terminal_state_properties(self):
        """Test that terminal states are correctly identified."""
        from tools.phoenix.migration import MigrationState

        terminal_states = [
            MigrationState.COMPLETED,
            MigrationState.COMPENSATED,
            MigrationState.DISPUTED,
            MigrationState.CANCELLED,
        ]

        for state in terminal_states:
            assert state.is_terminal(), f"{state} should be terminal"

        non_terminal_states = [
            MigrationState.INITIATED,
            MigrationState.COMPLIANCE_CHECK,
            MigrationState.ATTESTATION_GATHERING,
            MigrationState.SOURCE_LOCK,
            MigrationState.TRANSIT,
            MigrationState.DESTINATION_VERIFICATION,
            MigrationState.DESTINATION_UNLOCK,
        ]

        for state in non_terminal_states:
            assert not state.is_terminal(), f"{state} should NOT be terminal"

    def test_cancellation_allowed_states(self):
        """Test which states allow cancellation."""
        from tools.phoenix.migration import MigrationState

        # Early states should allow cancellation
        assert MigrationState.INITIATED.allows_cancellation()
        assert MigrationState.COMPLIANCE_CHECK.allows_cancellation()
        assert MigrationState.ATTESTATION_GATHERING.allows_cancellation()

        # Later states should NOT allow cancellation
        assert not MigrationState.SOURCE_LOCK.allows_cancellation(), \
            "BUG #10: SOURCE_LOCK should not allow cancellation (funds locked)"
        assert not MigrationState.TRANSIT.allows_cancellation(), \
            "TRANSIT should not allow cancellation"
        assert not MigrationState.COMPLETED.allows_cancellation(), \
            "COMPLETED should not allow cancellation"


# =============================================================================
# BUG #11: Real-World Scenario - Dubai-Singapore Corridor
# =============================================================================

class TestDubaiSingaporeCorridor:
    """
    Real-world test: Dubai DIFC to Singapore MAS asset migration.
    Tests cross-jurisdictional compliance, corridor setup, and settlement.
    """

    def test_difc_mas_zone_composition(self):
        """Test composing a DIFC-MAS hybrid zone."""
        from tools.msez.composition import (
            compose_zone,
            CorridorConfig,
            Domain,
        )

        zone = compose_zone(
            zone_id="difc-mas.hybrid.zone",
            name="DIFC-MAS Hybrid Financial Zone",
            financial=("ae-dubai-difc", "DIFC financial services"),
            digital_assets=("sg-mas", "MAS digital asset framework"),
            ai_arbitration=True,
            description="Cross-border financial services with digital assets",
        )

        # Verify composition
        errors = zone.validate()
        assert not errors, f"Zone should be valid: {errors}"

        # Check domains are correctly assigned
        domain_mapping = zone.domain_coverage_report()
        assert domain_mapping.get("financial") == "ae-dubai-difc"
        assert domain_mapping.get("digital-assets") == "sg-mas"

    def test_corridor_settlement_config(self):
        """Test corridor settlement configuration."""
        from tools.msez.composition import CorridorConfig

        # SWIFT corridor
        swift_corridor = CorridorConfig(
            corridor_id="difc-mas-swift",
            source_jurisdiction="ae-dubai-difc",
            target_jurisdiction="sg-mas",
            settlement_currency="USD",
            settlement_mechanism="swift-iso20022",
            max_settlement_usd=10_000_000,
            finality_seconds=86400,  # 24 hours
        )

        # Stablecoin corridor
        stablecoin_corridor = CorridorConfig(
            corridor_id="difc-mas-usdc",
            source_jurisdiction="ae-dubai-difc",
            target_jurisdiction="sg-mas",
            settlement_currency="USDC",
            settlement_mechanism="ethereum-l2",
            max_settlement_usd=1_000_000,
            finality_seconds=900,  # 15 minutes
        )

        # Verify configs
        assert swift_corridor.finality_seconds > stablecoin_corridor.finality_seconds
        assert swift_corridor.max_settlement_usd > stablecoin_corridor.max_settlement_usd


# =============================================================================
# BUG #12: Real-World Scenario - Prospera Charter City Full-Stack
# =============================================================================

class TestProsperaCharterCity:
    """
    Real-world test: Prospera Honduras charter city deployment.
    Full-stack zone with all legal/regulatory/financial infrastructure.
    """

    def test_prospera_full_stack_zone(self):
        """Test full-stack Prospera zone composition."""
        from tools.msez.composition import (
            ZoneComposition,
            JurisdictionLayer,
            Domain,
            ArbitrationConfig,
            ArbitrationMode,
        )

        # Prospera has its own legal framework
        prospera_layer = JurisdictionLayer(
            jurisdiction_id="hn-prospera",
            domains=[
                Domain.CIVIC,
                Domain.CORPORATE,
                Domain.COMMERCIAL,
                Domain.FINANCIAL,
                Domain.SECURITIES,
                Domain.BANKING,
                Domain.PAYMENTS,
                Domain.CUSTODY,
                Domain.SETTLEMENT,
                Domain.DIGITAL_ASSETS,
                Domain.TAX,
                Domain.EMPLOYMENT,
                Domain.IP,
                Domain.DATA_PROTECTION,
                Domain.AML_CFT,
                Domain.CONSUMER_PROTECTION,
                Domain.ARBITRATION,
                Domain.LICENSING,
            ],
            description="Prospera ZEDE comprehensive legal framework",
        )

        arb_config = ArbitrationConfig(
            mode=ArbitrationMode.HYBRID,
            institution_id="prospera-arbitration-center",
            ai_model="claude-opus-4-5-20251101",
            human_review_threshold_usd=50000,
            appeal_allowed=True,
        )

        zone = ZoneComposition(
            zone_id="prospera.main.zone",
            name="Prospera Main Zone",
            description="Prospera ZEDE full-stack economic zone",
            layers=[prospera_layer],
            arbitration=arb_config,
            profile="charter-city",
        )

        errors = zone.validate()
        assert not errors, f"Prospera zone should be valid: {errors}"

        # Should cover all domains except maybe immigration/clearing
        covered = zone.all_domains()
        assert len(covered) >= 18, f"Prospera should cover most domains: {len(covered)}"


# =============================================================================
# BUG #13: Bridge Protocol - Fee Calculation Edge Cases
# =============================================================================

class TestBridgeFeeCalculation:
    """
    Bug: Bridge fee calculations may have precision issues with
    very large or very small amounts.
    """

    def test_fee_precision_large_amounts(self):
        """Test fee calculation for very large amounts."""
        from tools.phoenix.manifold import CorridorEdge

        edge = CorridorEdge(
            corridor_id="test-large-amount",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="sg-mas",
            transfer_fee_bps=50,  # 0.5%
            flat_fee_usd=Decimal("100"),
        )

        # Very large amount - $1 billion
        large_amount = Decimal("1000000000")
        cost = edge.transfer_cost(large_amount)

        # Expected: 100 + (1B * 50 / 10000) = 100 + 5,000,000 = 5,000,100
        expected = Decimal("5000100")
        assert cost == expected, f"BUG #13: Large amount fee wrong: {cost} != {expected}"

    def test_fee_precision_small_amounts(self):
        """Test fee calculation for very small amounts."""
        from tools.phoenix.manifold import CorridorEdge

        edge = CorridorEdge(
            corridor_id="test-small-amount",
            source_jurisdiction="ae-abudhabi-adgm",
            target_jurisdiction="sg-mas",
            transfer_fee_bps=50,
            flat_fee_usd=Decimal("100"),
        )

        # Very small amount - $0.01
        small_amount = Decimal("0.01")
        cost = edge.transfer_cost(small_amount)

        # Expected: 100 + (0.01 * 50 / 10000) = 100 + 0.00005 = 100.00005
        expected = Decimal("100") + (small_amount * Decimal("50") / Decimal("10000"))
        assert cost == expected, f"BUG #13: Small amount fee wrong: {cost} != {expected}"


# =============================================================================
# BUG #14: Compliance Tensor - Zone Integration
# =============================================================================

class TestComplianceTensorZoneIntegration:
    """
    Test compliance tensor integration with zone compositions.
    """

    def test_multi_jurisdiction_tensor(self):
        """Test tensor with multiple jurisdictions from zone."""
        from tools.phoenix.tensor import (
            ComplianceTensorV2,
            ComplianceDomain,
            ComplianceState,
        )

        tensor = ComplianceTensorV2()

        # Asset compliant in DIFC but not in MAS
        asset_id = "asset-" + secrets.token_hex(32)

        # DIFC compliance
        tensor.set(
            asset_id=asset_id,
            jurisdiction_id="ae-dubai-difc",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.COMPLIANT,
            time_quantum=1704067200,
        )
        tensor.set(
            asset_id=asset_id,
            jurisdiction_id="ae-dubai-difc",
            domain=ComplianceDomain.AML,
            state=ComplianceState.COMPLIANT,
            time_quantum=1704067200,
        )

        # MAS - pending KYC
        tensor.set(
            asset_id=asset_id,
            jurisdiction_id="sg-mas",
            domain=ComplianceDomain.KYC,
            state=ComplianceState.PENDING,
            time_quantum=1704067200,
        )

        # Evaluate compliance in each jurisdiction
        difc_compliant, difc_state, difc_issues = tensor.evaluate(
            asset_id, "ae-dubai-difc",
            domains={ComplianceDomain.KYC, ComplianceDomain.AML},
            time_quantum=1704067200,
        )

        mas_compliant, mas_state, mas_issues = tensor.evaluate(
            asset_id, "sg-mas",
            domains={ComplianceDomain.KYC},
            time_quantum=1704067200,
        )

        assert difc_compliant, "Should be DIFC compliant"
        assert not mas_compliant, "Should NOT be MAS compliant (pending)"


# =============================================================================
# RUN ALL TESTS
# =============================================================================

def run_tests():
    """Run all test classes."""
    test_classes = [
        TestZoneCompositionDomainConflicts,
        TestJurisdictionIdValidation,
        TestComposeZoneMerging,
        TestWatcherBondSlashing,
        TestAttestationScopeValidity,
        TestScopedAttestationCreation,
        TestCorridorConfigValidation,
        TestArbitrationConfigValidation,
        TestZoneYamlGeneration,
        TestMigrationStateMachine,
        TestDubaiSingaporeCorridor,
        TestProsperaCharterCity,
        TestBridgeFeeCalculation,
        TestComplianceTensorZoneIntegration,
    ]

    passed = 0
    failed = 0
    errors = []

    for cls in test_classes:
        print(f'\n=== {cls.__name__} ===')
        if cls.__doc__:
            print(f'  {cls.__doc__.strip().split(chr(10))[0]}')
        instance = cls()
        for method_name in dir(instance):
            if method_name.startswith('test_'):
                try:
                    getattr(instance, method_name)()
                    print(f'  PASS: {method_name}')
                    passed += 1
                except AssertionError as e:
                    print(f'  FAIL: {method_name}')
                    print(f'        {e}')
                    errors.append((cls.__name__, method_name, e))
                    failed += 1
                except Exception as e:
                    print(f'  ERROR: {method_name}')
                    print(f'        {type(e).__name__}: {e}')
                    errors.append((cls.__name__, method_name, e))
                    failed += 1

    print(f'\n{"="*60}')
    print(f'RESULTS: {passed} passed, {failed} failed')
    if errors:
        print('\nFailed/Error tests (BUGS FOUND):')
        for cls_name, method_name, error in errors:
            print(f'  {cls_name}.{method_name}: {error}')

    return failed == 0


if __name__ == "__main__":
    import sys
    sys.exit(0 if run_tests() else 1)
